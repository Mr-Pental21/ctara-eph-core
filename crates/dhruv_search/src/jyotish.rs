//! Vedic jyotish orchestration: queries engine for graha positions.
//!
//! Provides the bridge between the ephemeris engine and the pure-math
//! Vedic calculation modules. Queries all 9 graha positions at a given
//! epoch and converts to sidereal longitudes.

use dhruv_core::{Body, Engine, Frame, Observer, Query};
use dhruv_frames::{
    ReferencePlane, cartesian_to_spherical, ecliptic_lon_to_invariable_lon, icrf_to_ecliptic,
    icrf_to_invariable, invariable_lon_to_ecliptic_lon, mean_obliquity_of_date_rad,
    precess_ecliptic_j2000_to_date_with_model,
};
use dhruv_time::{EopKernel, UtcTime, jd_to_tdb_seconds, tdb_seconds_to_jd};
use dhruv_vedic_base::arudha::all_arudha_padas;
use dhruv_vedic_base::riseset::compute_rise_set;
use dhruv_vedic_base::riseset_types::{GeoLocation, RiseSetConfig, RiseSetEvent, RiseSetResult};
use dhruv_vedic_base::special_lagna::all_special_lagnas;
use dhruv_vedic_base::upagraha::TIME_BASED_UPAGRAHAS;
use dhruv_vedic_base::vaar::vaar_from_jd;
use dhruv_vedic_base::vimsopaka::{
    DASHAVARGA as VIMSOPAKA_DASHAVARGA, SAPTAVARGA as VIMSOPAKA_SAPTAVARGA,
    SHADVARGA as VIMSOPAKA_SHADVARGA, SHODASAVARGA as VIMSOPAKA_SHODASAVARGA, VargaWeight,
    vimsopaka_dignity_points,
};
use dhruv_vedic_base::{
    ALL_GRAHAS, AllGrahaAvasthas, AllSpecialLagnas, AllUpagrahas, Amsha, AmshaRequest,
    ArudhaResult, AshtakavargaResult, AvasthaInputs, Bhava, BhavaBalaBirthPeriod, BhavaBalaInputs,
    BhavaBalaResult, BhavaConfig, BhavaResult, CharakarakaResult, CharakarakaScheme,
    DIG_BALA_BHAVA, Dignity, DrishtiEntry, Graha, GrahaAvasthas, GrahaDrishtiMatrix,
    KalaBalaInputs, LajjitadiInputs, LunarNode, NodeDignityPolicy, NodeMode, SAPTA_GRAHAS,
    SayanadiInputs, SayanadiResult, ShadbalaInputs, TimeUpagrahaConfig, all_avasthas,
    all_combustion_status, all_shadbalas_from_inputs, all_sphutas, amsha_longitude, baladi_avastha,
    bhava_bala_entry, bhrigu_bindu, calculate_ashtakavarga, calculate_bhava_bala,
    charakarakas_from_longitudes, compound_dignity_in_rashi, compute_bhavas, deeptadi_avastha,
    default_amsha_variation, dignity_in_rashi_with_positions, ghati_lagna, ghatikas_since_sunrise,
    graha_drishti, graha_drishti_matrix, hora_lagna, hora_lord as graha_hora_lord,
    is_valid_amsha_variation, jagradadi_avastha, jd_tdb_to_centuries, lagna_longitude_rad,
    lajjitadi_avastha, lost_planetary_war, lunar_node_deg_for_epoch_on_plane,
    nakshatra_from_longitude, node_dignity_in_rashi, node_dignity_in_rashi_with_temporal_context,
    normalize_360, nth_rashi_from, own_signs, pranapada_lagna, rashi_from_longitude,
    rashi_lord_by_index, sayanadi_all_sub_states, sayanadi_avastha, shadbala_from_inputs,
    sree_lagna, sun_based_upagrahas, time_upagraha_jd_with_config, vaar_lord as graha_vaar_lord,
};

use crate::conjunction::{body_ecliptic_lon_lat, body_ecliptic_state, body_lon_lat_on_plane};
use crate::dasha::{
    DashaInputs, dasha_hierarchy_with_inputs, dasha_snapshot_with_inputs, is_rashi_system,
    needs_moon_lon, needs_sunrise_sunset,
};
use crate::error::SearchError;
use crate::jyotish_types::{
    AmshaChart, AmshaChartScope, AmshaEntry, AmshaResult, AmshaSelectionConfig, BalaBundleResult,
    BhavaResultSet, BindusConfig, BindusResult, DashaSelectionConfig, DashaSnapshotTime,
    DrishtiConfig, DrishtiResult, FullKundaliConfig, FullKundaliResult, GrahaEntry,
    GrahaLongitudeKind, GrahaLongitudes, GrahaLongitudesConfig, GrahaPositions,
    GrahaPositionsConfig, MAX_AMSHA_REQUESTS, MovingOsculatingApogeeEntry, MovingOsculatingApogees,
    ShadbalaEntry, ShadbalaResult, SphutalResult, VimsopakaEntry, VimsopakaResult,
};
use crate::panchang::{
    hora_from_sunrises, masa_for_date_with_eop, panchang_for_date, vaar_for_date,
    varsha_for_date_with_eop, vedic_day_sunrises,
};
use crate::panchang_types::{MasaInfo, VarshaInfo};
use crate::sankranti_types::SankrantiConfig;

const BHAVABALA_TWILIGHT_HALF_DAYS: f64 = 5.0 / 60.0;
/// IAU 2015 nominal solar gravitational parameter, km^3/s^2.
///
/// Provenance is recorded in `docs/clean_room_osculating_apogee.md`.
const SOLAR_GM_KM3_S2: f64 = 132_712_440_000.0;
const SHADBALA_REQUIRED_AMSHAS: [Amsha; 7] = [
    Amsha::D1,
    Amsha::D2,
    Amsha::D3,
    Amsha::D7,
    Amsha::D9,
    Amsha::D12,
    Amsha::D30,
];
const VIMSOPAKA_REQUIRED_AMSHAS: [Amsha; 16] = [
    Amsha::D1,
    Amsha::D2,
    Amsha::D3,
    Amsha::D4,
    Amsha::D7,
    Amsha::D9,
    Amsha::D10,
    Amsha::D12,
    Amsha::D16,
    Amsha::D20,
    Amsha::D24,
    Amsha::D27,
    Amsha::D30,
    Amsha::D40,
    Amsha::D45,
    Amsha::D60,
];
const AVASTHA_REQUIRED_AMSHAS: [Amsha; 1] = [Amsha::D9];

#[derive(Debug, Clone, Copy)]
struct CachedAmshaGrahaData {
    amsha: Amsha,
    variation_code: u8,
    longitudes: [f64; 9],
    rashi_indices: [u8; 9],
    division_indices: [u16; 9],
}

#[derive(Debug, Clone, Default)]
struct AmshaGrahaCache {
    entries: Vec<CachedAmshaGrahaData>,
}

impl AmshaGrahaCache {
    fn get(&self, request: AmshaRequest) -> Option<&CachedAmshaGrahaData> {
        let variation_code = request.effective_variation();
        self.entries
            .iter()
            .find(|cached| cached.amsha == request.amsha && cached.variation_code == variation_code)
    }

    fn get_or_compute<F>(
        &mut self,
        request: AmshaRequest,
        compute: F,
    ) -> Result<&CachedAmshaGrahaData, SearchError>
    where
        F: FnOnce() -> Result<CachedAmshaGrahaData, SearchError>,
    {
        let variation_code = request.effective_variation();
        if let Some(index) = self.entries.iter().position(|cached| {
            cached.amsha == request.amsha && cached.variation_code == variation_code
        }) {
            return Ok(&self.entries[index]);
        }
        self.entries.push(compute()?);
        Ok(self.entries.last().expect("amsha cache entry just pushed"))
    }

    #[cfg(test)]
    fn len(&self) -> usize {
        self.entries.len()
    }
}

#[derive(Debug, Clone, Copy)]
struct CachedUpagrahaData {
    config: TimeUpagrahaConfig,
    upagrahas: AllUpagrahas,
}

#[derive(Debug, Clone, Default)]
struct UpagrahaCache {
    entries: Vec<CachedUpagrahaData>,
}

impl UpagrahaCache {
    #[cfg(test)]
    fn len(&self) -> usize {
        self.entries.len()
    }
}

#[derive(Debug, Clone, Copy)]
struct CachedVimsopakaVargaData {
    request: AmshaRequest,
    node_policy: NodeDignityPolicy,
    dignities: [Dignity; 9],
    points: [f64; 9],
}

#[derive(Debug, Clone, Default)]
struct VimsopakaVargaCache {
    entries: Vec<CachedVimsopakaVargaData>,
}

impl VimsopakaVargaCache {
    #[cfg(test)]
    fn len(&self) -> usize {
        self.entries.len()
    }
}

#[derive(Debug, Clone)]
struct ResolvedAmshaPlan {
    requests: Vec<AmshaRequest>,
}

impl ResolvedAmshaPlan {
    fn is_empty(&self) -> bool {
        self.requests.is_empty()
    }

    fn requests(&self) -> &[AmshaRequest] {
        &self.requests
    }

    fn request_for(&self, amsha: Amsha) -> AmshaRequest {
        self.requests
            .iter()
            .copied()
            .find(|request| request.amsha == amsha)
            .unwrap_or_else(|| AmshaRequest::new(amsha))
    }
}

/// Map a Graha to its dhruv_core::Body for engine queries.
fn graha_to_body(graha: Graha) -> Option<Body> {
    match graha {
        Graha::Surya => Some(Body::Sun),
        Graha::Chandra => Some(Body::Moon),
        Graha::Mangal => Some(Body::Mars),
        Graha::Buddh => Some(Body::Mercury),
        Graha::Guru => Some(Body::Jupiter),
        Graha::Shukra => Some(Body::Venus),
        Graha::Shani => Some(Body::Saturn),
        Graha::Rahu | Graha::Ketu => None,
    }
}

/// Convert an ecliptic longitude (e.g. bhava cusp, gulika) to sidereal
/// on the specified reference plane.
fn ecliptic_to_sidereal(ecl_lon_deg: f64, aya: f64, plane: ReferencePlane) -> f64 {
    let on_plane = match plane {
        ReferencePlane::Ecliptic => ecl_lon_deg,
        ReferencePlane::Invariable => ecliptic_lon_to_invariable_lon(ecl_lon_deg),
    };
    normalize(on_plane - aya)
}

fn jd_tdb_to_jd_utc(engine: &Engine, jd_tdb: f64) -> f64 {
    let tdb_s = jd_to_tdb_seconds(jd_tdb);
    let utc_s = engine.lsk().tdb_to_utc(tdb_s);
    tdb_seconds_to_jd(utc_s)
}

/// Project a tropical ecliptic longitude onto the configured sidereal zodiac.
pub fn tropical_to_sidereal_longitude(
    tropical_lon_deg: f64,
    ayanamsha_deg: f64,
    reference_plane: ReferencePlane,
) -> f64 {
    ecliptic_to_sidereal(tropical_lon_deg, ayanamsha_deg, reference_plane)
}

/// Project tropical bhava cusp / lagna / MC output into the configured sidereal zodiac.
pub fn siderealize_bhava_result(
    result: &BhavaResult,
    ayanamsha_deg: f64,
    reference_plane: ReferencePlane,
) -> BhavaResult {
    let mut bhavas = result.bhavas;
    for bhava in &mut bhavas {
        bhava.cusp_deg =
            tropical_to_sidereal_longitude(bhava.cusp_deg, ayanamsha_deg, reference_plane);
        bhava.start_deg =
            tropical_to_sidereal_longitude(bhava.start_deg, ayanamsha_deg, reference_plane);
        bhava.end_deg =
            tropical_to_sidereal_longitude(bhava.end_deg, ayanamsha_deg, reference_plane);
    }
    BhavaResult {
        bhavas,
        lagna_deg: tropical_to_sidereal_longitude(result.lagna_deg, ayanamsha_deg, reference_plane),
        mc_deg: tropical_to_sidereal_longitude(result.mc_deg, ayanamsha_deg, reference_plane),
    }
}

/// Recover ecliptic tropical longitude from a sidereal longitude on the given
/// plane.  Used for bhava matching: bhava cusps are ecliptic, so the sidereal
/// value must be converted back to ecliptic tropical before comparing.
fn sidereal_to_ecliptic_tropical(sid_lon: f64, aya: f64, plane: ReferencePlane) -> f64 {
    let tropical_on_plane = normalize(sid_lon + aya);
    match plane {
        ReferencePlane::Ecliptic => tropical_on_plane,
        ReferencePlane::Invariable => invariable_lon_to_ecliptic_lon(tropical_on_plane),
    }
}

fn rashi_bhava_number_from_lagna(lagna_sid: f64, sid_lon: f64) -> u8 {
    let lagna_rashi = (normalize(lagna_sid) / 30.0).floor().min(11.0) as u8;
    let body_rashi = (normalize(sid_lon) / 30.0).floor().min(11.0) as u8;
    ((body_rashi + 12 - lagna_rashi) % 12) + 1
}

fn rashi_bhava_result_from_lagna(lagna_sid: f64) -> BhavaResult {
    let lagna_sid = normalize(lagna_sid);
    let lagna_rashi = (lagna_sid / 30.0).floor().min(11.0) as u8;
    let degrees_in_rashi = lagna_sid - f64::from(lagna_rashi) * 30.0;
    let mut bhavas = [Bhava {
        number: 1,
        cusp_deg: 0.0,
        start_deg: 0.0,
        end_deg: 0.0,
    }; 12];
    for (i, bhava) in bhavas.iter_mut().enumerate() {
        let rashi = (lagna_rashi + i as u8) % 12;
        let cusp = normalize(f64::from(rashi) * 30.0 + degrees_in_rashi);
        let next_rashi = (rashi + 1) % 12;
        let end = normalize(f64::from(next_rashi) * 30.0 + degrees_in_rashi);
        *bhava = Bhava {
            number: i as u8 + 1,
            cusp_deg: cusp,
            start_deg: cusp,
            end_deg: end,
        };
    }
    BhavaResult {
        bhavas,
        lagna_deg: lagna_sid,
        mc_deg: bhavas[9].cusp_deg,
    }
}

#[derive(Debug, Clone, Copy)]
struct BalaBhavaBasis {
    bhava_numbers: [u8; 9],
    cusp_sidereal_lons: [f64; 12],
    ascendant_sidereal_lon: f64,
    meridian_sidereal_lon: f64,
}

#[doc(hidden)]
#[derive(Debug, Clone, Copy)]
pub struct CheshtaMotionEntry {
    pub graha: Graha,
    pub sphuta_longitude: f64,
    pub madhyama_longitude: f64,
    pub chaloccha_longitude: f64,
    pub mean_sun_longitude: f64,
    pub graha_heliocentric_mean_longitude: f64,
    pub graha_heliocentric_aphelion_longitude: f64,
}

#[derive(Debug, Clone, Copy)]
struct CachedCheshtaMotion {
    graha: Graha,
    config: GrahaLongitudesConfig,
    sphuta_longitude: f64,
    entry: CheshtaMotionEntry,
}

#[derive(Debug, Clone, Copy)]
struct CachedMeanSun {
    config: GrahaLongitudesConfig,
    longitude: f64,
}

/// One-shot, function-local cache for shared intermediates.
///
/// This is intentionally not exposed and not persisted across calls.
#[derive(Debug, Clone)]
struct JyotishContext {
    jd_tdb: f64,
    jd_utc: f64,
    ayanamsha: f64,
    reference_plane: ReferencePlane,
    graha_lons: Option<GrahaLongitudes>,
    lagna_sid: Option<f64>,
    bhava_result: Option<BhavaResult>,
    sidereal_bhava_cusps: Option<[f64; 12]>,
    graha_bhava_numbers: Option<[u8; 9]>,
    rashi_bhava_result: Option<BhavaResult>,
    rashi_bhava_cusps: Option<[f64; 12]>,
    graha_rashi_bhava_numbers: Option<[u8; 9]>,
    sunrise_pair: Option<(f64, f64)>,
    sunset_jd: Option<f64>,
    graha_drishti_matrix: Option<GrahaDrishtiMatrix>,
    masa_info: Option<MasaInfo>,
    varsha_info: Option<VarshaInfo>,
    /// Ecliptic longitude speeds (deg/day) for sapta grahas (indices 0-6).
    graha_speeds: Option<[f64; 7]>,
    /// Ecliptic declinations (deg) for sapta grahas (indices 0-6).
    graha_declinations: Option<[f64; 7]>,
    mean_sun_longitudes: Vec<CachedMeanSun>,
    cheshta_motion: Vec<CachedCheshtaMotion>,
    amsha_graha_cache: AmshaGrahaCache,
    upagraha_cache: UpagrahaCache,
    vimsopaka_varga_cache: VimsopakaVargaCache,
}

impl JyotishContext {
    fn new(
        engine: &Engine,
        eop: Option<&EopKernel>,
        utc: &UtcTime,
        aya_config: &SankrantiConfig,
    ) -> Self {
        let jd_tdb = crate::search_util::utc_to_jd_tdb_with_eop(engine, eop, utc);
        let jd_utc = utc_to_jd_utc(utc);
        let t = jd_tdb_to_centuries(jd_tdb);
        let ayanamsha = aya_config.ayanamsha_deg_at_centuries(t);
        Self {
            jd_tdb,
            jd_utc,
            ayanamsha,
            reference_plane: aya_config.reference_plane,
            graha_lons: None,
            lagna_sid: None,
            bhava_result: None,
            sidereal_bhava_cusps: None,
            graha_bhava_numbers: None,
            rashi_bhava_result: None,
            rashi_bhava_cusps: None,
            graha_rashi_bhava_numbers: None,
            sunrise_pair: None,
            sunset_jd: None,
            graha_drishti_matrix: None,
            masa_info: None,
            varsha_info: None,
            graha_speeds: None,
            graha_declinations: None,
            mean_sun_longitudes: Vec::new(),
            cheshta_motion: Vec::new(),
            amsha_graha_cache: AmshaGrahaCache::default(),
            upagraha_cache: UpagrahaCache::default(),
            vimsopaka_varga_cache: VimsopakaVargaCache::default(),
        }
    }

    fn graha_lons<'a>(
        &'a mut self,
        engine: &Engine,
        aya_config: &SankrantiConfig,
    ) -> Result<&'a GrahaLongitudes, SearchError> {
        if self.graha_lons.is_none() {
            let lons = graha_longitudes(
                engine,
                self.jd_tdb,
                &GrahaLongitudesConfig::sidereal_with_model(
                    aya_config.ayanamsha_system,
                    aya_config.use_nutation,
                    aya_config.precession_model,
                    aya_config.reference_plane,
                ),
            )?;
            self.graha_lons = Some(lons);
        }
        Ok(self.graha_lons.as_ref().expect("graha longitudes set"))
    }

    fn bhava_result<'a>(
        &'a mut self,
        engine: &Engine,
        eop: &EopKernel,
        location: &GeoLocation,
        bhava_config: &BhavaConfig,
    ) -> Result<&'a BhavaResult, SearchError> {
        if self.bhava_result.is_none() {
            let result = compute_bhavas(
                engine,
                engine.lsk(),
                eop,
                location,
                self.jd_utc,
                bhava_config,
            )?;
            self.bhava_result = Some(result);
        }
        Ok(self.bhava_result.as_ref().expect("bhava result set"))
    }

    fn sidereal_bhava_cusps(
        &mut self,
        engine: &Engine,
        eop: &EopKernel,
        location: &GeoLocation,
        bhava_config: &BhavaConfig,
    ) -> Result<[f64; 12], SearchError> {
        if let Some(cusps) = self.sidereal_bhava_cusps {
            return Ok(cusps);
        }
        let ayanamsha = self.ayanamsha;
        let reference_plane = self.reference_plane;
        let bhava_result = self.bhava_result(engine, eop, location, bhava_config)?;
        let mut cusps = [0.0f64; 12];
        for (i, cusp) in cusps.iter_mut().enumerate() {
            *cusp =
                ecliptic_to_sidereal(bhava_result.bhavas[i].cusp_deg, ayanamsha, reference_plane);
        }
        self.sidereal_bhava_cusps = Some(cusps);
        Ok(cusps)
    }

    fn graha_bhava_numbers(
        &mut self,
        engine: &Engine,
        eop: &EopKernel,
        location: &GeoLocation,
        bhava_config: &BhavaConfig,
        aya_config: &SankrantiConfig,
    ) -> Result<[u8; 9], SearchError> {
        if let Some(numbers) = self.graha_bhava_numbers {
            return Ok(numbers);
        }
        let graha_lons = *self.graha_lons(engine, aya_config)?;
        let bhava_result = *self.bhava_result(engine, eop, location, bhava_config)?;
        let mut numbers = [0u8; 9];
        for graha in ALL_GRAHAS {
            let idx = graha.index() as usize;
            numbers[idx] = find_bhava_number(
                sidereal_to_ecliptic_tropical(
                    graha_lons.longitudes[idx],
                    self.ayanamsha,
                    self.reference_plane,
                ),
                &bhava_result,
            );
        }
        self.graha_bhava_numbers = Some(numbers);
        Ok(numbers)
    }

    fn rashi_bhava_result(
        &mut self,
        engine: &Engine,
        eop: &EopKernel,
        location: &GeoLocation,
    ) -> Result<BhavaResult, SearchError> {
        if let Some(result) = self.rashi_bhava_result {
            return Ok(result);
        }
        let lagna_sid = self.lagna_sid(engine, eop, location)?;
        let result = rashi_bhava_result_from_lagna(lagna_sid);
        self.rashi_bhava_result = Some(result);
        Ok(result)
    }

    fn rashi_bhava_cusps(
        &mut self,
        engine: &Engine,
        eop: &EopKernel,
        location: &GeoLocation,
    ) -> Result<[f64; 12], SearchError> {
        if let Some(cusps) = self.rashi_bhava_cusps {
            return Ok(cusps);
        }
        let result = self.rashi_bhava_result(engine, eop, location)?;
        let mut cusps = [0.0f64; 12];
        for (i, cusp) in cusps.iter_mut().enumerate() {
            *cusp = result.bhavas[i].cusp_deg;
        }
        self.rashi_bhava_cusps = Some(cusps);
        Ok(cusps)
    }

    fn graha_rashi_bhava_numbers(
        &mut self,
        engine: &Engine,
        eop: &EopKernel,
        location: &GeoLocation,
        aya_config: &SankrantiConfig,
    ) -> Result<[u8; 9], SearchError> {
        if let Some(numbers) = self.graha_rashi_bhava_numbers {
            return Ok(numbers);
        }
        let graha_lons = *self.graha_lons(engine, aya_config)?;
        let lagna_sid = self.lagna_sid(engine, eop, location)?;
        let mut numbers = [0u8; 9];
        for graha in ALL_GRAHAS {
            let idx = graha.index() as usize;
            numbers[idx] = rashi_bhava_number_from_lagna(lagna_sid, graha_lons.longitudes[idx]);
        }
        self.graha_rashi_bhava_numbers = Some(numbers);
        Ok(numbers)
    }

    fn bala_bhava_basis(
        &mut self,
        engine: &Engine,
        eop: &EopKernel,
        location: &GeoLocation,
        bhava_config: &BhavaConfig,
        aya_config: &SankrantiConfig,
    ) -> Result<BalaBhavaBasis, SearchError> {
        if bhava_config.use_rashi_bhava_for_bala_avastha {
            let bhava_result = self.rashi_bhava_result(engine, eop, location)?;
            Ok(BalaBhavaBasis {
                bhava_numbers: self.graha_rashi_bhava_numbers(engine, eop, location, aya_config)?,
                cusp_sidereal_lons: self.rashi_bhava_cusps(engine, eop, location)?,
                ascendant_sidereal_lon: bhava_result.lagna_deg,
                meridian_sidereal_lon: bhava_result.mc_deg,
            })
        } else {
            let bhava_result = *self.bhava_result(engine, eop, location, bhava_config)?;
            Ok(BalaBhavaBasis {
                bhava_numbers: self.graha_bhava_numbers(
                    engine,
                    eop,
                    location,
                    bhava_config,
                    aya_config,
                )?,
                cusp_sidereal_lons: self.sidereal_bhava_cusps(
                    engine,
                    eop,
                    location,
                    bhava_config,
                )?,
                ascendant_sidereal_lon: self.lagna_sid(engine, eop, location)?,
                meridian_sidereal_lon: ecliptic_to_sidereal(
                    bhava_result.mc_deg,
                    self.ayanamsha,
                    self.reference_plane,
                ),
            })
        }
    }

    fn lagna_sid(
        &mut self,
        engine: &Engine,
        eop: &EopKernel,
        location: &GeoLocation,
    ) -> Result<f64, SearchError> {
        if let Some(lagna_sid) = self.lagna_sid {
            return Ok(lagna_sid);
        }
        let lagna_rad = lagna_longitude_rad(engine.lsk(), eop, location, self.jd_utc)?;
        let lagna_ecl_deg = lagna_rad.to_degrees();
        // Project lagna to reference plane before subtracting ayanamsha
        let lagna_on_plane = match self.reference_plane {
            ReferencePlane::Ecliptic => lagna_ecl_deg,
            ReferencePlane::Invariable => ecliptic_lon_to_invariable_lon(lagna_ecl_deg),
        };
        let lagna_sid = normalize(lagna_on_plane - self.ayanamsha);
        self.lagna_sid = Some(lagna_sid);
        Ok(lagna_sid)
    }

    fn sunrise_pair(
        &mut self,
        engine: &Engine,
        eop: &EopKernel,
        utc: &UtcTime,
        location: &GeoLocation,
        riseset_config: &RiseSetConfig,
    ) -> Result<(f64, f64), SearchError> {
        if let Some(pair) = self.sunrise_pair {
            return Ok(pair);
        }
        let pair = vedic_day_sunrises(engine, eop, utc, location, riseset_config)?;
        self.sunrise_pair = Some(pair);
        Ok(pair)
    }

    fn sunset_jd(
        &mut self,
        engine: &Engine,
        eop: &EopKernel,
        location: &GeoLocation,
        riseset_config: &RiseSetConfig,
    ) -> Result<f64, SearchError> {
        if let Some(jd) = self.sunset_jd {
            return Ok(jd);
        }
        let noon_jd = dhruv_vedic_base::approximate_local_noon_jd(
            dhruv_vedic_base::utc_day_start_jd(self.jd_utc),
            location.longitude_deg,
        );
        let sunset_result = compute_rise_set(
            engine,
            engine.lsk(),
            eop,
            location,
            RiseSetEvent::Sunset,
            noon_jd,
            riseset_config,
        )
        .map_err(|_| SearchError::NoConvergence("sunset computation failed"))?;
        let jd = match sunset_result {
            RiseSetResult::Event { jd_tdb, .. } => jd_tdb,
            _ => {
                return Err(SearchError::NoConvergence(
                    "sun never sets at this location",
                ));
            }
        };
        self.sunset_jd = Some(jd);
        Ok(jd)
    }

    fn graha_to_graha_drishti(
        &mut self,
        engine: &Engine,
        aya_config: &SankrantiConfig,
    ) -> Result<GrahaDrishtiMatrix, SearchError> {
        if let Some(matrix) = self.graha_drishti_matrix {
            return Ok(matrix);
        }
        let matrix = graha_drishti_matrix(&self.graha_lons(engine, aya_config)?.longitudes);
        self.graha_drishti_matrix = Some(matrix);
        Ok(matrix)
    }

    fn masa_info(
        &mut self,
        engine: &Engine,
        eop: &EopKernel,
        utc: &UtcTime,
        aya_config: &SankrantiConfig,
    ) -> Result<MasaInfo, SearchError> {
        if let Some(masa) = self.masa_info {
            return Ok(masa);
        }
        let masa = masa_for_date_with_eop(engine, Some(eop), utc, aya_config)?;
        self.masa_info = Some(masa);
        Ok(masa)
    }

    fn varsha_info(
        &mut self,
        engine: &Engine,
        eop: &EopKernel,
        utc: &UtcTime,
        aya_config: &SankrantiConfig,
    ) -> Result<VarshaInfo, SearchError> {
        if let Some(varsha) = self.varsha_info {
            return Ok(varsha);
        }
        let varsha = varsha_for_date_with_eop(engine, Some(eop), utc, aya_config)?;
        self.varsha_info = Some(varsha);
        Ok(varsha)
    }

    /// Get ecliptic longitude speeds (deg/day) for sapta grahas, computing on first call.
    fn graha_speeds(&mut self, engine: &Engine) -> Result<[f64; 7], SearchError> {
        if let Some(speeds) = self.graha_speeds {
            return Ok(speeds);
        }
        let speeds = query_sapta_graha_speeds(engine, self.jd_tdb)?;
        self.graha_speeds = Some(speeds);
        Ok(speeds)
    }

    /// Get ecliptic declinations (deg) for sapta grahas, computing on first call.
    fn graha_declinations(&mut self, engine: &Engine) -> Result<[f64; 7], SearchError> {
        if let Some(decls) = self.graha_declinations {
            return Ok(decls);
        }
        let decls = query_sapta_graha_declinations(engine, self.jd_tdb)?;
        self.graha_declinations = Some(decls);
        Ok(decls)
    }

    fn cheshta_motion(
        &mut self,
        engine: &Engine,
        graha: Graha,
        sphuta_longitude: f64,
        config: GrahaLongitudesConfig,
    ) -> Result<CheshtaMotionEntry, SearchError> {
        if let Some(entry) = self
            .cheshta_motion
            .iter()
            .find(|cached| {
                cached.graha == graha
                    && cached.config == config
                    && cached.sphuta_longitude.to_bits() == sphuta_longitude.to_bits()
            })
            .map(|cached| cached.entry)
        {
            return Ok(entry);
        }
        let mean_sun_longitude = self.mean_sun_longitude(engine, config)?;
        let entry = cheshta_motion_entry_with_mean_sun(
            engine,
            self.jd_tdb,
            graha,
            sphuta_longitude,
            &config,
            mean_sun_longitude,
        )?;
        self.cheshta_motion.push(CachedCheshtaMotion {
            graha,
            config,
            sphuta_longitude,
            entry,
        });
        Ok(entry)
    }

    fn mean_sun_longitude(
        &mut self,
        engine: &Engine,
        config: GrahaLongitudesConfig,
    ) -> Result<f64, SearchError> {
        if let Some(cached) = self
            .mean_sun_longitudes
            .iter()
            .find(|cached| cached.config == config)
        {
            return Ok(cached.longitude);
        }
        let longitude = mean_sun_sidereal_longitude(engine, self.jd_tdb, &config)?;
        self.mean_sun_longitudes
            .push(CachedMeanSun { config, longitude });
        Ok(longitude)
    }

    fn amsha_graha_data<'a>(
        &'a mut self,
        engine: &Engine,
        aya_config: &SankrantiConfig,
        request: AmshaRequest,
    ) -> Result<&'a CachedAmshaGrahaData, SearchError> {
        let graha_lons = *self.graha_lons(engine, aya_config)?;
        let divisions = request.amsha.divisions() as f64;
        let division_span = 30.0 / divisions;
        let variation_code = request.effective_variation();
        self.amsha_graha_cache.get_or_compute(request, || {
            let mut longitudes = [0.0f64; 9];
            let mut rashi_indices = [0u8; 9];
            let mut division_indices = [0u16; 9];
            for i in 0..9 {
                let sidereal_lon = graha_lons.longitudes[i];
                let normalized = normalize_360(sidereal_lon);
                let degrees_in_sign = normalized % 30.0;
                let division_index = ((degrees_in_sign / division_span).floor() as u16)
                    .min(request.amsha.divisions().saturating_sub(1));
                let amsha_lon = amsha_longitude(sidereal_lon, request.amsha, Some(variation_code));
                longitudes[i] = amsha_lon;
                rashi_indices[i] = ((normalize_360(amsha_lon) / 30.0).floor() as u8).min(11);
                division_indices[i] = division_index;
            }
            Ok(CachedAmshaGrahaData {
                amsha: request.amsha,
                variation_code,
                longitudes,
                rashi_indices,
                division_indices,
            })
        })
    }

    fn cached_amsha_graha_data(&self, request: AmshaRequest) -> Option<&CachedAmshaGrahaData> {
        self.amsha_graha_cache.get(request)
    }

    fn prime_amsha_graha_data(
        &mut self,
        engine: &Engine,
        aya_config: &SankrantiConfig,
        requests: &[AmshaRequest],
    ) -> Result<(), SearchError> {
        for request in requests {
            let _ = self.amsha_graha_data(engine, aya_config, *request)?;
        }
        Ok(())
    }

    fn upagrahas<'a>(
        &'a mut self,
        engine: &Engine,
        eop: &EopKernel,
        utc: &UtcTime,
        location: &GeoLocation,
        riseset_config: &RiseSetConfig,
        aya_config: &SankrantiConfig,
        upagraha_config: &TimeUpagrahaConfig,
    ) -> Result<&'a AllUpagrahas, SearchError> {
        let config = *upagraha_config;
        if let Some(index) = self
            .upagraha_cache
            .entries
            .iter()
            .position(|cached| cached.config == config)
        {
            return Ok(&self.upagraha_cache.entries[index].upagrahas);
        }
        let jd_tdb = self.jd_tdb;
        let aya = self.ayanamsha;
        let plane = self.reference_plane;
        let (jd_sunrise, jd_next_sunrise) =
            self.sunrise_pair(engine, eop, utc, location, riseset_config)?;
        let jd_sunset = self.sunset_jd(engine, eop, location, riseset_config)?;
        let is_day = jd_tdb >= jd_sunrise && jd_tdb < jd_sunset;
        let weekday = vaar_from_jd(jd_sunrise).index();

        let mut time_lons = [0.0f64; 6];
        for (i, &upa) in TIME_BASED_UPAGRAHAS.iter().enumerate() {
            let target_jd = time_upagraha_jd_with_config(
                upa,
                weekday,
                is_day,
                jd_sunrise,
                jd_sunset,
                jd_next_sunrise,
                upagraha_config,
            );
            let lagna_rad = lagna_longitude_rad(
                engine.lsk(),
                eop,
                location,
                jd_tdb_to_jd_utc(engine, target_jd),
            )?;
            let lagna_ecl = lagna_rad.to_degrees();
            let lagna_on_plane = match plane {
                ReferencePlane::Ecliptic => lagna_ecl,
                ReferencePlane::Invariable => ecliptic_lon_to_invariable_lon(lagna_ecl),
            };
            time_lons[i] = normalize_360(lagna_on_plane - aya);
        }

        let sun_sid = self.graha_lons(engine, aya_config)?.longitude(Graha::Surya);
        let sun_up = sun_based_upagrahas(sun_sid);
        self.upagraha_cache.entries.push(CachedUpagrahaData {
            config,
            upagrahas: AllUpagrahas {
                gulika: time_lons[0],
                maandi: time_lons[1],
                kaala: time_lons[2],
                mrityu: time_lons[3],
                artha_prahara: time_lons[4],
                yama_ghantaka: time_lons[5],
                dhooma: sun_up.dhooma,
                vyatipata: sun_up.vyatipata,
                parivesha: sun_up.parivesha,
                indra_chapa: sun_up.indra_chapa,
                upaketu: sun_up.upaketu,
            },
        });
        Ok(&self
            .upagraha_cache
            .entries
            .last()
            .expect("upagraha cache entry just pushed")
            .upagrahas)
    }

    fn vimsopaka_varga_data<'a>(
        &'a mut self,
        engine: &Engine,
        aya_config: &SankrantiConfig,
        request: AmshaRequest,
        node_policy: NodeDignityPolicy,
    ) -> Result<&'a CachedVimsopakaVargaData, SearchError> {
        if let Some(index) = self
            .vimsopaka_varga_cache
            .entries
            .iter()
            .position(|cached| {
                cached.request.amsha == request.amsha
                    && cached.request.effective_variation() == request.effective_variation()
                    && cached.node_policy == node_policy
            })
        {
            return Ok(&self.vimsopaka_varga_cache.entries[index]);
        }
        let cached_amsha = *self.amsha_graha_data(engine, aya_config, request)?;
        let varga_rashi_indices = cached_amsha.rashi_indices;
        let d1_rashi_indices = self
            .amsha_graha_data(engine, aya_config, AmshaRequest::new(Amsha::D1))?
            .rashi_indices;
        let mut varga_sapta_rashi = [0u8; 7];
        varga_sapta_rashi.copy_from_slice(&varga_rashi_indices[..7]);
        let mut dignities = [Dignity::Sama; 9];
        let mut points = [0.0f64; 9];
        for graha in ALL_GRAHAS {
            let gi = graha.index() as usize;
            let rashi_idx = varga_rashi_indices[gi];
            let dignity = if matches!(graha, Graha::Rahu | Graha::Ketu) {
                node_dignity_in_rashi_with_temporal_context(
                    graha,
                    rashi_idx,
                    &d1_rashi_indices,
                    &varga_rashi_indices,
                    node_policy,
                )
            } else {
                if own_signs(graha).contains(&rashi_idx) {
                    Dignity::OwnSign
                } else {
                    compound_dignity_in_rashi(graha, rashi_idx, &varga_sapta_rashi)
                }
            };
            dignities[gi] = dignity;
            points[gi] = vimsopaka_dignity_points(dignity);
        }
        self.vimsopaka_varga_cache
            .entries
            .push(CachedVimsopakaVargaData {
                request,
                node_policy,
                dignities,
                points,
            });
        Ok(self
            .vimsopaka_varga_cache
            .entries
            .last()
            .expect("vimsopaka varga cache entry just pushed"))
    }
}

/// Query ecliptic-of-date longitude speed (deg/day) for all 7 sapta grahas.
///
/// Uses `body_ecliptic_state` which finite-differences fully-precessed
/// of-date longitudes to capture the complete velocity frame correction.
fn query_sapta_graha_speeds(engine: &Engine, jd_tdb: f64) -> Result<[f64; 7], SearchError> {
    let mut speeds = [0.0f64; 7];
    for graha in SAPTA_GRAHAS {
        let body = graha_to_body(graha).expect("sapta graha has body");
        let (_, _, lon_speed) = body_ecliptic_state(engine, body, jd_tdb)?;
        speeds[graha.index() as usize] = lon_speed;
    }
    Ok(speeds)
}

/// Query ecliptic declination (deg) for all 7 sapta grahas.
///
/// Declination = arcsin(sin(lat)*cos(eps) + cos(lat)*sin(eps)*sin(lon))
/// where lon, lat are ecliptic-of-date coordinates and eps is the
/// mean obliquity of date (IAU 2006).
fn query_sapta_graha_declinations(engine: &Engine, jd_tdb: f64) -> Result<[f64; 7], SearchError> {
    let t = (jd_tdb - 2_451_545.0) / 36525.0;
    let eps = mean_obliquity_of_date_rad(t);
    let sin_eps = eps.sin();
    let cos_eps = eps.cos();
    let mut decls = [0.0f64; 7];
    for graha in SAPTA_GRAHAS {
        let body = graha_to_body(graha).expect("sapta graha has body");
        let (lon_deg, lat_deg) = body_ecliptic_lon_lat(engine, body, jd_tdb)?;
        let lon_rad = lon_deg.to_radians();
        let lat_rad = lat_deg.to_radians();
        let sin_dec = lat_rad.sin() * cos_eps + lat_rad.cos() * sin_eps * lon_rad.sin();
        decls[graha.index() as usize] = sin_dec.clamp(-1.0, 1.0).asin().to_degrees();
    }
    Ok(decls)
}

fn apogee_supported_body(graha: Graha) -> Option<Body> {
    match graha {
        Graha::Mangal => Some(Body::Mars),
        Graha::Buddh => Some(Body::Mercury),
        Graha::Guru => Some(Body::Jupiter),
        Graha::Shukra => Some(Body::Venus),
        Graha::Shani => Some(Body::Saturn),
        _ => None,
    }
}

fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn norm(v: [f64; 3]) -> f64 {
    (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt()
}

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn reference_plane_vector(
    icrf_vector: &[f64; 3],
    jd_tdb: f64,
    config: &GrahaLongitudesConfig,
) -> [f64; 3] {
    match config.reference_plane {
        ReferencePlane::Ecliptic => {
            let ecl_j2000 = icrf_to_ecliptic(icrf_vector);
            let t = jd_tdb_to_centuries(jd_tdb);
            precess_ecliptic_j2000_to_date_with_model(&ecl_j2000, t, config.precession_model)
        }
        ReferencePlane::Invariable => icrf_to_invariable(icrf_vector),
    }
}

#[derive(Debug, Clone, Copy)]
struct OsculatingLongitudes {
    mean_sidereal_longitude: f64,
    aphelion_sidereal_longitude: f64,
    aphelion_reference_plane_longitude: f64,
    ayanamsha_deg: f64,
}

fn osculating_longitudes_for_body(
    engine: &Engine,
    jd_tdb: f64,
    body: Body,
    config: &GrahaLongitudesConfig,
) -> Result<OsculatingLongitudes, SearchError> {
    let state = engine.query(Query {
        target: body,
        observer: Observer::Body(Body::Sun),
        frame: Frame::IcrfJ2000,
        epoch_tdb_jd: jd_tdb,
    })?;
    osculating_longitudes_from_heliocentric_state(
        state.position_km,
        state.velocity_km_s,
        jd_tdb,
        config,
    )
}

fn osculating_longitudes_from_heliocentric_state(
    r: [f64; 3],
    v: [f64; 3],
    jd_tdb: f64,
    config: &GrahaLongitudesConfig,
) -> Result<OsculatingLongitudes, SearchError> {
    let r_norm = norm(r);
    let v_norm = norm(v);
    if r_norm <= f64::EPSILON || v_norm <= f64::EPSILON {
        return Err(SearchError::NoConvergence(
            "heliocentric osculating state vector is degenerate",
        ));
    }

    let h = cross(r, v);
    let h_norm = norm(h);
    if h_norm <= f64::EPSILON {
        return Err(SearchError::NoConvergence(
            "heliocentric osculating angular momentum vector is zero",
        ));
    }

    let vxh = cross(v, h);
    let eccentricity = [
        vxh[0] / SOLAR_GM_KM3_S2 - r[0] / r_norm,
        vxh[1] / SOLAR_GM_KM3_S2 - r[1] / r_norm,
        vxh[2] / SOLAR_GM_KM3_S2 - r[2] / r_norm,
    ];
    let e_norm = norm(eccentricity);
    if e_norm <= f64::EPSILON || e_norm >= 1.0 {
        return Err(SearchError::NoConvergence(
            "heliocentric osculating eccentricity is unsupported",
        ));
    }

    let cos_true_anomaly = (dot(eccentricity, r) / (e_norm * r_norm)).clamp(-1.0, 1.0);
    let sin_true_anomaly =
        (dot(cross(eccentricity, r), h) / (e_norm * r_norm * h_norm)).clamp(-1.0, 1.0);
    let eccentric_anomaly =
        ((1.0 - e_norm * e_norm).sqrt() * sin_true_anomaly).atan2(e_norm + cos_true_anomaly);
    let mean_anomaly_deg =
        normalize_360((eccentric_anomaly - e_norm * eccentric_anomaly.sin()).to_degrees());

    let periapsis_on_plane = reference_plane_vector(&eccentricity, jd_tdb, config);
    let periapsis_reference_plane_longitude =
        normalize_360(cartesian_to_spherical(&periapsis_on_plane).lon_deg);
    let aphelion_reference_plane_longitude =
        normalize_360(periapsis_reference_plane_longitude + 180.0);
    let t = jd_tdb_to_centuries(jd_tdb);
    let ayanamsha_deg = dhruv_vedic_base::ayanamsha_deg_on_plane(
        config.ayanamsha_system,
        t,
        config.use_nutation,
        config.precession_model,
        config.reference_plane,
    );

    Ok(OsculatingLongitudes {
        mean_sidereal_longitude: normalize_360(
            periapsis_reference_plane_longitude + mean_anomaly_deg - ayanamsha_deg,
        ),
        aphelion_sidereal_longitude: normalize_360(
            aphelion_reference_plane_longitude - ayanamsha_deg,
        ),
        aphelion_reference_plane_longitude,
        ayanamsha_deg,
    })
}

fn moving_osculating_apogee_entry(
    engine: &Engine,
    jd_tdb: f64,
    graha: Graha,
    config: &GrahaLongitudesConfig,
) -> Result<MovingOsculatingApogeeEntry, SearchError> {
    if config.kind != GrahaLongitudeKind::Sidereal {
        return Err(SearchError::InvalidConfig(
            "moving osculating apogee requires sidereal longitude config",
        ));
    }

    let body = apogee_supported_body(graha).ok_or(SearchError::InvalidConfig(
        "moving osculating apogee supports only Mangal, Buddh, Guru, Shukra, and Shani",
    ))?;
    let longitudes = osculating_longitudes_for_body(engine, jd_tdb, body, config)?;

    Ok(MovingOsculatingApogeeEntry {
        graha,
        reference_plane_longitude: longitudes.aphelion_reference_plane_longitude,
        ayanamsha_deg: longitudes.ayanamsha_deg,
        sidereal_longitude: longitudes.aphelion_sidereal_longitude,
    })
}

fn mean_sun_sidereal_longitude(
    engine: &Engine,
    jd_tdb: f64,
    config: &GrahaLongitudesConfig,
) -> Result<f64, SearchError> {
    if config.kind != GrahaLongitudeKind::Sidereal {
        return Err(SearchError::InvalidConfig(
            "mean Sun correction longitude requires sidereal longitude config",
        ));
    }
    let earth = osculating_longitudes_for_body(engine, jd_tdb, Body::Earth, config)?;
    Ok(normalize_360(earth.mean_sidereal_longitude + 180.0))
}

fn cheshta_motion_entry_with_mean_sun(
    engine: &Engine,
    jd_tdb: f64,
    graha: Graha,
    sphuta_longitude: f64,
    config: &GrahaLongitudesConfig,
    mean_sun_longitude: f64,
) -> Result<CheshtaMotionEntry, SearchError> {
    let body = apogee_supported_body(graha).ok_or(SearchError::InvalidConfig(
        "cheshta motion supports only Mangal, Buddh, Guru, Shukra, and Shani",
    ))?;
    let graha_motion = osculating_longitudes_for_body(engine, jd_tdb, body, config)?;
    let graha_mean = graha_motion.mean_sidereal_longitude;
    let (madhyama_longitude, chaloccha_longitude) = match graha {
        Graha::Mangal | Graha::Guru | Graha::Shani => (graha_mean, mean_sun_longitude),
        Graha::Buddh | Graha::Shukra => (mean_sun_longitude, graha_mean),
        _ => unreachable!("unsupported graha rejected above"),
    };

    Ok(CheshtaMotionEntry {
        graha,
        sphuta_longitude,
        madhyama_longitude,
        chaloccha_longitude,
        mean_sun_longitude,
        graha_heliocentric_mean_longitude: graha_mean,
        graha_heliocentric_aphelion_longitude: graha_motion.aphelion_sidereal_longitude,
    })
}

#[doc(hidden)]
pub fn cheshta_motion_entries(
    engine: &Engine,
    jd_tdb: f64,
    config: &GrahaLongitudesConfig,
    sphuta_longitudes: &[f64; 9],
) -> Result<[Option<CheshtaMotionEntry>; 7], SearchError> {
    let mut entries = [None; 7];
    let mean_sun_longitude = mean_sun_sidereal_longitude(engine, jd_tdb, config)?;
    for graha in [
        Graha::Mangal,
        Graha::Buddh,
        Graha::Guru,
        Graha::Shukra,
        Graha::Shani,
    ] {
        let idx = graha.index() as usize;
        entries[idx] = Some(cheshta_motion_entry_with_mean_sun(
            engine,
            jd_tdb,
            graha,
            sphuta_longitudes[idx],
            config,
            mean_sun_longitude,
        )?);
    }
    Ok(entries)
}

/// Compute moving osculating apogees at a TDB epoch.
///
/// Entries are returned in caller request order. Duplicate graha requests are
/// fanned out from one per-call computation.
pub fn moving_osculating_apogees(
    engine: &Engine,
    jd_tdb: f64,
    config: &GrahaLongitudesConfig,
    grahas: &[Graha],
) -> Result<MovingOsculatingApogees, SearchError> {
    let mut cache: Vec<(Graha, MovingOsculatingApogeeEntry)> = Vec::new();
    let mut entries = Vec::with_capacity(grahas.len());
    for &graha in grahas {
        let entry = if let Some((_, entry)) = cache.iter().find(|(g, _)| *g == graha) {
            *entry
        } else {
            let entry = moving_osculating_apogee_entry(engine, jd_tdb, graha, config)?;
            cache.push((graha, entry));
            entry
        };
        entries.push(entry);
    }
    Ok(MovingOsculatingApogees { entries })
}

/// Compute moving osculating apogees for a UTC date.
pub fn moving_osculating_apogees_for_date(
    engine: &Engine,
    eop: Option<&EopKernel>,
    utc: &UtcTime,
    config: &GrahaLongitudesConfig,
    grahas: &[Graha],
) -> Result<MovingOsculatingApogees, SearchError> {
    let jd_tdb = crate::search_util::utc_to_jd_tdb_with_eop(engine, eop, utc);
    moving_osculating_apogees(engine, jd_tdb, config, grahas)
}

/// Query all 9 graha longitudes at a given TDB epoch.
pub fn graha_longitudes(
    engine: &Engine,
    jd_tdb: f64,
    config: &GrahaLongitudesConfig,
) -> Result<GrahaLongitudes, SearchError> {
    match config.kind {
        GrahaLongitudeKind::Sidereal => {
            graha_sidereal_longitudes_for_config(engine, jd_tdb, config)
        }
        GrahaLongitudeKind::Tropical => graha_reference_plane_longitudes(engine, jd_tdb, config),
    }
}

fn graha_sidereal_longitudes_for_config(
    engine: &Engine,
    jd_tdb: f64,
    config: &GrahaLongitudesConfig,
) -> Result<GrahaLongitudes, SearchError> {
    let t = jd_tdb_to_centuries(jd_tdb);
    let aya = dhruv_vedic_base::ayanamsha_deg_on_plane(
        config.ayanamsha_system,
        t,
        config.use_nutation,
        config.precession_model,
        config.reference_plane,
    );
    let rahu_on_plane = lunar_node_deg_for_epoch_on_plane(
        engine,
        LunarNode::Rahu,
        jd_tdb,
        NodeMode::True,
        config.precession_model,
        config.reference_plane,
    )?;
    let ketu_on_plane = normalize(rahu_on_plane + 180.0);

    let mut longitudes = [0.0f64; 9];

    for graha in ALL_GRAHAS {
        let idx = graha.index() as usize;
        match graha {
            Graha::Rahu => {
                longitudes[idx] = normalize(rahu_on_plane - aya);
            }
            Graha::Ketu => {
                longitudes[idx] = normalize(ketu_on_plane - aya);
            }
            _ => {
                let body = graha_to_body(graha).expect("sapta graha has body");
                let (lon, _lat) = body_lon_lat_on_plane(
                    engine,
                    body,
                    jd_tdb,
                    config.precession_model,
                    config.reference_plane,
                )?;
                longitudes[idx] = normalize(lon - aya);
            }
        }
    }

    Ok(GrahaLongitudes { longitudes })
}

fn graha_reference_plane_longitudes(
    engine: &Engine,
    jd_tdb: f64,
    config: &GrahaLongitudesConfig,
) -> Result<GrahaLongitudes, SearchError> {
    let dpsi_deg = if config.use_nutation && config.reference_plane == ReferencePlane::Ecliptic {
        let t = jd_tdb_to_centuries(jd_tdb);
        let (dpsi_arcsec, _deps_arcsec) = dhruv_frames::nutation_iau2000b(t);
        dpsi_arcsec / 3600.0
    } else {
        0.0
    };

    let rahu_tropical = lunar_node_deg_for_epoch_on_plane(
        engine,
        LunarNode::Rahu,
        jd_tdb,
        NodeMode::True,
        config.precession_model,
        config.reference_plane,
    )?;
    let ketu_tropical = normalize(rahu_tropical + 180.0);

    let mut longitudes = [0.0f64; 9];
    for graha in ALL_GRAHAS {
        let idx = graha.index() as usize;
        match graha {
            Graha::Rahu => longitudes[idx] = normalize(rahu_tropical + dpsi_deg),
            Graha::Ketu => longitudes[idx] = normalize(ketu_tropical + dpsi_deg),
            _ => {
                let body = graha_to_body(graha).expect("sapta graha has body");
                let (lon, _lat) = body_lon_lat_on_plane(
                    engine,
                    body,
                    jd_tdb,
                    config.precession_model,
                    config.reference_plane,
                )?;
                longitudes[idx] = normalize(lon + dpsi_deg);
            }
        }
    }
    Ok(GrahaLongitudes { longitudes })
}

/// Compute all 8 special lagnas for a given moment and location.
///
/// Orchestrates engine queries for Sun/Moon, Lagna computation, sunrise
/// determination, and delegates to the pure-math `all_special_lagnas()`.
pub fn special_lagnas_for_date(
    engine: &Engine,
    eop: &EopKernel,
    utc: &UtcTime,
    location: &GeoLocation,
    riseset_config: &RiseSetConfig,
    aya_config: &SankrantiConfig,
) -> Result<AllSpecialLagnas, SearchError> {
    let mut ctx = JyotishContext::new(engine, Some(eop), utc, aya_config);
    special_lagnas_for_date_with_ctx(
        engine,
        eop,
        utc,
        location,
        riseset_config,
        aya_config,
        &mut ctx,
    )
}

/// Compute bhava cusps and return them on the configured sidereal zodiac.
pub fn sidereal_bhavas_for_date(
    engine: &Engine,
    eop: &EopKernel,
    utc: &UtcTime,
    location: &GeoLocation,
    bhava_config: &BhavaConfig,
    aya_config: &SankrantiConfig,
) -> Result<BhavaResult, SearchError> {
    let mut ctx = JyotishContext::new(engine, Some(eop), utc, aya_config);
    let tropical = *ctx.bhava_result(engine, eop, location, bhava_config)?;
    Ok(siderealize_bhava_result(
        &tropical,
        ctx.ayanamsha,
        ctx.reference_plane,
    ))
}

/// Compute configured sidereal bhava cusps plus optional rashi-bhava siblings.
pub fn sidereal_bhava_results_for_date(
    engine: &Engine,
    eop: &EopKernel,
    utc: &UtcTime,
    location: &GeoLocation,
    bhava_config: &BhavaConfig,
    aya_config: &SankrantiConfig,
) -> Result<BhavaResultSet, SearchError> {
    let bhava_cusps =
        sidereal_bhavas_for_date(engine, eop, utc, location, bhava_config, aya_config)?;
    let rashi_bhava_cusps = bhava_config
        .include_rashi_bhava_results
        .then(|| rashi_bhava_result_from_lagna(bhava_cusps.lagna_deg));
    Ok(BhavaResultSet {
        bhava_cusps,
        rashi_bhava_cusps,
    })
}

/// Compute lagna (ascendant) on the configured sidereal zodiac.
pub fn sidereal_lagna_for_date(
    engine: &Engine,
    eop: &EopKernel,
    utc: &UtcTime,
    location: &GeoLocation,
    aya_config: &SankrantiConfig,
) -> Result<f64, SearchError> {
    let mut ctx = JyotishContext::new(engine, Some(eop), utc, aya_config);
    ctx.lagna_sid(engine, eop, location)
}

/// Compute MC (midheaven) on the configured sidereal zodiac.
pub fn sidereal_mc_for_date(
    engine: &Engine,
    eop: &EopKernel,
    utc: &UtcTime,
    location: &GeoLocation,
    bhava_config: &BhavaConfig,
    aya_config: &SankrantiConfig,
) -> Result<f64, SearchError> {
    let mut ctx = JyotishContext::new(engine, Some(eop), utc, aya_config);
    let tropical = *ctx.bhava_result(engine, eop, location, bhava_config)?;
    Ok(tropical_to_sidereal_longitude(
        tropical.mc_deg,
        ctx.ayanamsha,
        ctx.reference_plane,
    ))
}

fn special_lagnas_for_date_with_ctx(
    engine: &Engine,
    eop: &EopKernel,
    utc: &UtcTime,
    location: &GeoLocation,
    riseset_config: &RiseSetConfig,
    aya_config: &SankrantiConfig,
    ctx: &mut JyotishContext,
) -> Result<AllSpecialLagnas, SearchError> {
    let graha_lons = *ctx.graha_lons(engine, aya_config)?;
    let sun_sid = graha_lons.longitude(Graha::Surya);
    let moon_sid = graha_lons.longitude(Graha::Chandra);
    let lagna_sid = ctx.lagna_sid(engine, eop, location)?;
    let (jd_sunrise, jd_next_sunrise) =
        ctx.sunrise_pair(engine, eop, utc, location, riseset_config)?;
    let ghatikas = ghatikas_since_sunrise(ctx.jd_tdb, jd_sunrise, jd_next_sunrise);

    let lagna_rashi_idx = (lagna_sid / 30.0) as u8;
    let lagna_lord = rashi_lord_by_index(lagna_rashi_idx).unwrap_or(Graha::Surya);

    let moon_rashi_idx = (moon_sid / 30.0) as u8;
    let moon_9th_rashi_idx = nth_rashi_from(moon_rashi_idx, 9);
    let moon_9th_lord = rashi_lord_by_index(moon_9th_rashi_idx).unwrap_or(Graha::Surya);

    Ok(all_special_lagnas(
        sun_sid,
        moon_sid,
        lagna_sid,
        ghatikas,
        lagna_lord,
        moon_9th_lord,
    ))
}

/// Compute all 12 arudha padas for a given date and location.
///
/// Orchestrates bhava cusp computation, graha sidereal positions, resolves
/// lord longitudes for each house, and delegates to `all_arudha_padas()`.
pub fn arudha_padas_for_date(
    engine: &Engine,
    eop: &EopKernel,
    utc: &UtcTime,
    location: &GeoLocation,
    bhava_config: &BhavaConfig,
    aya_config: &SankrantiConfig,
) -> Result<[ArudhaResult; 12], SearchError> {
    let mut ctx = JyotishContext::new(engine, Some(eop), utc, aya_config);
    arudha_padas_for_date_with_ctx(engine, eop, location, bhava_config, aya_config, &mut ctx)
}

fn arudha_padas_for_date_with_ctx(
    engine: &Engine,
    eop: &EopKernel,
    location: &GeoLocation,
    bhava_config: &BhavaConfig,
    aya_config: &SankrantiConfig,
    ctx: &mut JyotishContext,
) -> Result<[ArudhaResult; 12], SearchError> {
    let graha_lons = *ctx.graha_lons(engine, aya_config)?;
    let aya = ctx.ayanamsha;
    let plane = ctx.reference_plane;
    let bhava_result = ctx.bhava_result(engine, eop, location, bhava_config)?;

    let mut cusp_sid = [0.0f64; 12];
    for (i, cusp) in cusp_sid.iter_mut().enumerate() {
        *cusp = ecliptic_to_sidereal(bhava_result.bhavas[i].cusp_deg, aya, plane);
    }

    // Resolve lord longitude for each house
    let mut lord_lons = [0.0f64; 12];
    for i in 0..12 {
        let cusp_rashi_idx = (cusp_sid[i] / 30.0) as u8;
        let lord = rashi_lord_by_index(cusp_rashi_idx).unwrap_or(Graha::Surya);
        lord_lons[i] = graha_lons.longitude(lord);
    }

    Ok(all_arudha_padas(&cusp_sid, &lord_lons))
}

/// Compute all 11 upagrahas for a given date and location.
///
/// Orchestrates sunrise/sunset computation, portion index determination,
/// lagna computation at portion times, and sun-based chain calculation.
pub fn all_upagrahas_for_date(
    engine: &Engine,
    eop: &EopKernel,
    utc: &UtcTime,
    location: &GeoLocation,
    riseset_config: &RiseSetConfig,
    aya_config: &SankrantiConfig,
) -> Result<AllUpagrahas, SearchError> {
    all_upagrahas_for_date_with_config(
        engine,
        eop,
        utc,
        location,
        riseset_config,
        aya_config,
        &TimeUpagrahaConfig::default(),
    )
}

/// Compute all 11 upagrahas for a given date and location using a custom
/// time-based upagraha period configuration.
pub fn all_upagrahas_for_date_with_config(
    engine: &Engine,
    eop: &EopKernel,
    utc: &UtcTime,
    location: &GeoLocation,
    riseset_config: &RiseSetConfig,
    aya_config: &SankrantiConfig,
    upagraha_config: &TimeUpagrahaConfig,
) -> Result<AllUpagrahas, SearchError> {
    let mut ctx = JyotishContext::new(engine, Some(eop), utc, aya_config);
    all_upagrahas_for_date_with_ctx(
        engine,
        eop,
        utc,
        location,
        riseset_config,
        aya_config,
        upagraha_config,
        &mut ctx,
    )
}

fn all_upagrahas_for_date_with_ctx(
    engine: &Engine,
    eop: &EopKernel,
    utc: &UtcTime,
    location: &GeoLocation,
    riseset_config: &RiseSetConfig,
    aya_config: &SankrantiConfig,
    upagraha_config: &TimeUpagrahaConfig,
    ctx: &mut JyotishContext,
) -> Result<AllUpagrahas, SearchError> {
    Ok(*ctx.upagrahas(
        engine,
        eop,
        utc,
        location,
        riseset_config,
        aya_config,
        upagraha_config,
    )?)
}

/// Compute comprehensive graha positions with optional nakshatra, lagna, outer planets, bhava.
///
/// Central orchestration function: computes sidereal longitudes for all 9 grahas,
/// optionally adding nakshatra/pada, lagna, outer planets (Uranus/Neptune/Pluto),
/// and bhava placement.
pub fn graha_positions(
    engine: &Engine,
    eop: &EopKernel,
    utc: &UtcTime,
    location: &GeoLocation,
    bhava_config: &BhavaConfig,
    aya_config: &SankrantiConfig,
    config: &GrahaPositionsConfig,
) -> Result<GrahaPositions, SearchError> {
    let mut ctx = JyotishContext::new(engine, Some(eop), utc, aya_config);
    graha_positions_with_ctx(
        engine,
        eop,
        location,
        bhava_config,
        aya_config,
        config,
        &mut ctx,
    )
}

fn graha_positions_with_ctx(
    engine: &Engine,
    eop: &EopKernel,
    location: &GeoLocation,
    bhava_config: &BhavaConfig,
    aya_config: &SankrantiConfig,
    config: &GrahaPositionsConfig,
    ctx: &mut JyotishContext,
) -> Result<GrahaPositions, SearchError> {
    let aya = ctx.ayanamsha;
    let jd_tdb = ctx.jd_tdb;
    let plane = ctx.reference_plane;
    let graha_lons = *ctx.graha_lons(engine, aya_config)?;

    let lagna_sid = if config.include_lagna {
        Some(ctx.lagna_sid(engine, eop, location)?)
    } else {
        None
    };

    let bhava_result = if config.include_bhava {
        Some(*ctx.bhava_result(engine, eop, location, bhava_config)?)
    } else {
        None
    };
    let rashi_bhava_lagna_sid = if config.include_bhava && bhava_config.include_rashi_bhava_results
    {
        Some(ctx.lagna_sid(engine, eop, location)?)
    } else {
        None
    };

    // Build GrahaEntry for each of the 9 grahas.
    let mut grahas = [GrahaEntry::sentinel(); 9];
    for graha in ALL_GRAHAS {
        let idx = graha.index() as usize;
        let sid_lon = graha_lons.longitude(graha);
        grahas[idx] = make_graha_entry(
            sid_lon,
            config,
            bhava_result.as_ref(),
            rashi_bhava_lagna_sid,
            aya,
            plane,
        );
    }

    let lagna = if config.include_lagna {
        make_graha_entry(
            lagna_sid.expect("include_lagna implies lagna_sid"),
            config,
            bhava_result.as_ref(),
            rashi_bhava_lagna_sid,
            aya,
            plane,
        )
    } else {
        GrahaEntry::sentinel()
    };

    let outer_planets = if config.include_outer_planets {
        let outer_bodies = [Body::Uranus, Body::Neptune, Body::Pluto];
        let mut entries = [GrahaEntry::sentinel(); 3];
        for (i, &body) in outer_bodies.iter().enumerate() {
            let (lon, _lat) =
                body_lon_lat_on_plane(engine, body, jd_tdb, aya_config.precession_model, plane)?;
            let sid_lon = normalize(lon - aya);
            entries[i] = make_graha_entry(
                sid_lon,
                config,
                bhava_result.as_ref(),
                rashi_bhava_lagna_sid,
                aya,
                plane,
            );
        }
        entries
    } else {
        [GrahaEntry::sentinel(); 3]
    };

    Ok(GrahaPositions {
        grahas,
        lagna,
        outer_planets,
    })
}

/// Build a GrahaEntry from a sidereal longitude, applying optional computations.
fn make_graha_entry(
    sid_lon: f64,
    config: &GrahaPositionsConfig,
    bhava_result: Option<&dhruv_vedic_base::BhavaResult>,
    rashi_bhava_lagna_sid: Option<f64>,
    aya: f64,
    plane: ReferencePlane,
) -> GrahaEntry {
    let rashi_info = rashi_from_longitude(sid_lon);
    let (nakshatra, nakshatra_index, pada) = if config.include_nakshatra {
        let nak = nakshatra_from_longitude(sid_lon);
        (nak.nakshatra, nak.nakshatra_index, nak.pada)
    } else {
        (dhruv_vedic_base::Nakshatra::Ashwini, 255, 0)
    };
    let bhava_number = if let Some(result) = bhava_result {
        // Reconstruct ecliptic tropical longitude for bhava matching.
        // Bhava cusps are ecliptic; when sidereal is on the invariable plane,
        // we must reverse-project to ecliptic before matching.
        find_bhava_number(sidereal_to_ecliptic_tropical(sid_lon, aya, plane), result)
    } else {
        0
    };
    let rashi_bhava_number = rashi_bhava_lagna_sid
        .map(|lagna_sid| rashi_bhava_number_from_lagna(lagna_sid, sid_lon))
        .unwrap_or(0);
    GrahaEntry {
        sidereal_longitude: sid_lon,
        rashi: rashi_info.rashi,
        rashi_index: rashi_info.rashi_index,
        nakshatra,
        nakshatra_index,
        pada,
        bhava_number,
        rashi_bhava_number,
    }
}

/// Find which bhava (1-12) a tropical ecliptic longitude falls in.
fn find_bhava_number(tropical_deg: f64, result: &dhruv_vedic_base::BhavaResult) -> u8 {
    for bhava in &result.bhavas {
        let start = bhava.start_deg;
        let end = bhava.end_deg;
        if start < end {
            if tropical_deg >= start && tropical_deg < end {
                return bhava.number;
            }
        } else {
            // Wraps around 360/0 boundary
            if tropical_deg >= start || tropical_deg < end {
                return bhava.number;
            }
        }
    }
    // Fallback: should not happen, but assign to bhava 1
    1
}

/// Compute complete Ashtakavarga (BAV + SAV + Sodhana) for a given date and location.
///
/// Uses `graha_positions()` to compute graha + lagna sidereal longitudes,
/// then delegates to the pure-math `calculate_ashtakavarga()`.
pub fn ashtakavarga_for_date(
    engine: &Engine,
    eop: &EopKernel,
    utc: &UtcTime,
    location: &GeoLocation,
    aya_config: &SankrantiConfig,
) -> Result<AshtakavargaResult, SearchError> {
    let mut ctx = JyotishContext::new(engine, Some(eop), utc, aya_config);
    ashtakavarga_with_ctx(engine, eop, location, aya_config, &mut ctx)
}

fn ashtakavarga_with_ctx(
    engine: &Engine,
    eop: &EopKernel,
    location: &GeoLocation,
    aya_config: &SankrantiConfig,
    ctx: &mut JyotishContext,
) -> Result<AshtakavargaResult, SearchError> {
    let config = GrahaPositionsConfig {
        include_nakshatra: false,
        include_lagna: true,
        include_outer_planets: false,
        include_bhava: false,
    };
    let bhava_config = BhavaConfig::default();
    let positions = graha_positions_with_ctx(
        engine,
        eop,
        location,
        &bhava_config,
        aya_config,
        &config,
        ctx,
    )?;

    Ok(calculate_ashtakavarga_from_positions(&positions))
}

fn calculate_ashtakavarga_from_positions(positions: &GrahaPositions) -> AshtakavargaResult {
    // Extract rashi indices for 7 sapta grahas (Sun..Saturn only)
    let sapta = [
        Graha::Surya,
        Graha::Chandra,
        Graha::Mangal,
        Graha::Buddh,
        Graha::Guru,
        Graha::Shukra,
        Graha::Shani,
    ];
    let mut graha_rashis = [0u8; 7];
    for (i, &graha) in sapta.iter().enumerate() {
        graha_rashis[i] = positions.grahas[graha.index() as usize].rashi_index;
    }
    let lagna_rashi = positions.lagna.rashi_index;

    calculate_ashtakavarga(&graha_rashis, lagna_rashi)
}

/// Compute curated sensitive points (bindus) with optional nakshatra/bhava enrichment.
///
/// Collects 19 key Vedic sensitive points:
/// - 12 arudha padas (A1-A12)
/// - Bhrigu Bindu, Pranapada Lagna, Gulika, Maandi
/// - Hora Lagna, Ghati Lagna, Sree Lagna
///
/// Each point is wrapped in a `GrahaEntry` with rashi always populated,
/// and nakshatra/bhava optionally populated based on config flags.
#[allow(clippy::too_many_arguments)]
pub fn core_bindus(
    engine: &Engine,
    eop: &EopKernel,
    utc: &UtcTime,
    location: &GeoLocation,
    bhava_config: &BhavaConfig,
    riseset_config: &RiseSetConfig,
    aya_config: &SankrantiConfig,
    config: &BindusConfig,
) -> Result<BindusResult, SearchError> {
    let mut ctx = JyotishContext::new(engine, Some(eop), utc, aya_config);
    core_bindus_with_ctx(
        engine,
        eop,
        utc,
        location,
        bhava_config,
        riseset_config,
        aya_config,
        config,
        &mut ctx,
    )
}

#[allow(clippy::too_many_arguments)]
fn core_bindus_with_ctx(
    engine: &Engine,
    eop: &EopKernel,
    utc: &UtcTime,
    location: &GeoLocation,
    bhava_config: &BhavaConfig,
    riseset_config: &RiseSetConfig,
    aya_config: &SankrantiConfig,
    config: &BindusConfig,
    ctx: &mut JyotishContext,
) -> Result<BindusResult, SearchError> {
    let aya = ctx.ayanamsha;
    let plane = ctx.reference_plane;
    let gp_config = GrahaPositionsConfig {
        include_nakshatra: config.include_nakshatra,
        include_lagna: false,
        include_outer_planets: false,
        include_bhava: config.include_bhava,
    };

    let graha_lons = *ctx.graha_lons(engine, aya_config)?;
    let sun_sid = graha_lons.longitude(Graha::Surya);
    let moon_sid = graha_lons.longitude(Graha::Chandra);
    let rahu_sid = graha_lons.longitude(Graha::Rahu);

    let lagna_sid = ctx.lagna_sid(engine, eop, location)?;

    let (jd_sunrise, jd_next_sunrise) =
        ctx.sunrise_pair(engine, eop, utc, location, riseset_config)?;
    let ghatikas = ghatikas_since_sunrise(ctx.jd_tdb, jd_sunrise, jd_next_sunrise);
    let upagrahas = *ctx.upagrahas(
        engine,
        eop,
        utc,
        location,
        riseset_config,
        aya_config,
        &config.upagraha_config,
    )?;

    let bb_lon = bhrigu_bindu(rahu_sid, moon_sid);
    let pp_lon = pranapada_lagna(sun_sid, ghatikas);
    let hl_lon = hora_lagna(sun_sid, ghatikas);
    let gl_lon = ghati_lagna(sun_sid, ghatikas);
    let sl_lon = sree_lagna(moon_sid, lagna_sid);

    let bhava_result = *ctx.bhava_result(engine, eop, location, bhava_config)?;
    let bhava_opt = if config.include_bhava {
        Some(bhava_result)
    } else {
        None
    };
    let rashi_bhava_lagna_sid = if config.include_bhava && bhava_config.include_rashi_bhava_results
    {
        Some(lagna_sid)
    } else {
        None
    };

    let mut cusp_sid = [0.0f64; 12];
    for (i, cusp) in cusp_sid.iter_mut().enumerate() {
        *cusp = ecliptic_to_sidereal(bhava_result.bhavas[i].cusp_deg, aya, plane);
    }
    let mut lord_lons = [0.0f64; 12];
    for i in 0..12 {
        let cusp_rashi_idx = (cusp_sid[i] / 30.0) as u8;
        let lord = rashi_lord_by_index(cusp_rashi_idx).unwrap_or(Graha::Surya);
        lord_lons[i] = graha_lons.longitude(lord);
    }
    let arudha_raw = all_arudha_padas(&cusp_sid, &lord_lons);
    let mut arudha_padas = [GrahaEntry::sentinel(); 12];
    for i in 0..12 {
        arudha_padas[i] = make_graha_entry(
            arudha_raw[i].longitude_deg,
            &gp_config,
            bhava_opt.as_ref(),
            rashi_bhava_lagna_sid,
            aya,
            plane,
        );
    }

    let rashi_bhava_arudha_padas = if bhava_config.include_rashi_bhava_results {
        let rashi_cusps = ctx.rashi_bhava_cusps(engine, eop, location)?;
        let mut rashi_lord_lons = [0.0f64; 12];
        for i in 0..12 {
            let cusp_rashi_idx = (rashi_cusps[i] / 30.0).floor().min(11.0) as u8;
            let lord = rashi_lord_by_index(cusp_rashi_idx).unwrap_or(Graha::Surya);
            rashi_lord_lons[i] = graha_lons.longitude(lord);
        }
        let raw = all_arudha_padas(&rashi_cusps, &rashi_lord_lons);
        let mut entries = [GrahaEntry::sentinel(); 12];
        for i in 0..12 {
            entries[i] = make_graha_entry(
                raw[i].longitude_deg,
                &gp_config,
                bhava_opt.as_ref(),
                rashi_bhava_lagna_sid,
                aya,
                plane,
            );
        }
        Some(entries)
    } else {
        None
    };

    Ok(BindusResult {
        arudha_padas,
        rashi_bhava_arudha_padas,
        bhrigu_bindu: make_graha_entry(
            bb_lon,
            &gp_config,
            bhava_opt.as_ref(),
            rashi_bhava_lagna_sid,
            aya,
            plane,
        ),
        pranapada_lagna: make_graha_entry(
            pp_lon,
            &gp_config,
            bhava_opt.as_ref(),
            rashi_bhava_lagna_sid,
            aya,
            plane,
        ),
        gulika: make_graha_entry(
            upagrahas.gulika,
            &gp_config,
            bhava_opt.as_ref(),
            rashi_bhava_lagna_sid,
            aya,
            plane,
        ),
        maandi: make_graha_entry(
            upagrahas.maandi,
            &gp_config,
            bhava_opt.as_ref(),
            rashi_bhava_lagna_sid,
            aya,
            plane,
        ),
        hora_lagna: make_graha_entry(
            hl_lon,
            &gp_config,
            bhava_opt.as_ref(),
            rashi_bhava_lagna_sid,
            aya,
            plane,
        ),
        ghati_lagna: make_graha_entry(
            gl_lon,
            &gp_config,
            bhava_opt.as_ref(),
            rashi_bhava_lagna_sid,
            aya,
            plane,
        ),
        sree_lagna: make_graha_entry(
            sl_lon,
            &gp_config,
            bhava_opt.as_ref(),
            rashi_bhava_lagna_sid,
            aya,
            plane,
        ),
    })
}

/// Compute graha drishti (planetary aspects) with optional extensions.
///
/// Always computes the 9×9 graha-to-graha matrix. Optionally extends to:
/// - graha-to-bhava-cusp (12 cusps) if `config.include_bhava`
/// - graha-to-lagna if `config.include_lagna`
/// - graha-to-core-bindus (19 points) if `config.include_bindus`
#[allow(clippy::too_many_arguments)]
pub fn drishti_for_date(
    engine: &Engine,
    eop: &EopKernel,
    utc: &UtcTime,
    location: &GeoLocation,
    bhava_config: &BhavaConfig,
    riseset_config: &RiseSetConfig,
    aya_config: &SankrantiConfig,
    config: &DrishtiConfig,
) -> Result<DrishtiResult, SearchError> {
    let mut ctx = JyotishContext::new(engine, Some(eop), utc, aya_config);
    drishti_for_date_with_ctx(
        engine,
        eop,
        utc,
        location,
        bhava_config,
        riseset_config,
        aya_config,
        config,
        &mut ctx,
        None,
    )
}

#[allow(clippy::too_many_arguments)]
fn drishti_for_date_with_ctx(
    engine: &Engine,
    eop: &EopKernel,
    utc: &UtcTime,
    location: &GeoLocation,
    bhava_config: &BhavaConfig,
    riseset_config: &RiseSetConfig,
    aya_config: &SankrantiConfig,
    config: &DrishtiConfig,
    ctx: &mut JyotishContext,
    precomputed_bindus: Option<&BindusResult>,
) -> Result<DrishtiResult, SearchError> {
    let aya = ctx.ayanamsha;
    let plane = ctx.reference_plane;
    let graha_lons = *ctx.graha_lons(engine, aya_config)?;
    let graha_to_graha = ctx.graha_to_graha_drishti(engine, aya_config)?;

    let graha_to_lagna = if config.include_lagna {
        let lagna_sid = ctx.lagna_sid(engine, eop, location)?;
        let mut entries = [DrishtiEntry::zero(); 9];
        for g in ALL_GRAHAS {
            let i = g.index() as usize;
            entries[i] = graha_drishti(g, graha_lons.longitudes[i], lagna_sid);
        }
        entries
    } else {
        [DrishtiEntry::zero(); 9]
    };

    let graha_to_bhava = if config.include_bhava {
        let cusp_sid = ctx.sidereal_bhava_cusps(engine, eop, location, bhava_config)?;
        let mut entries = [[DrishtiEntry::zero(); 12]; 9];
        for g in ALL_GRAHAS {
            let gi = g.index() as usize;
            for (ci, &cusp) in cusp_sid.iter().enumerate() {
                entries[gi][ci] = graha_drishti(g, graha_lons.longitudes[gi], cusp);
            }
        }
        entries
    } else {
        [[DrishtiEntry::zero(); 12]; 9]
    };
    let graha_to_rashi_bhava = if config.include_bhava && bhava_config.include_rashi_bhava_results {
        let cusp_sid = ctx.rashi_bhava_cusps(engine, eop, location)?;
        let mut entries = [[DrishtiEntry::zero(); 12]; 9];
        for g in ALL_GRAHAS {
            let gi = g.index() as usize;
            for (ci, &cusp) in cusp_sid.iter().enumerate() {
                entries[gi][ci] = graha_drishti(g, graha_lons.longitudes[gi], cusp);
            }
        }
        entries
    } else {
        [[DrishtiEntry::zero(); 12]; 9]
    };
    let _ = (aya, plane);

    let graha_to_bindus = if config.include_bindus {
        let local_bindus;
        let bindus_ref = if let Some(b) = precomputed_bindus {
            b
        } else {
            local_bindus = core_bindus_with_ctx(
                engine,
                eop,
                utc,
                location,
                bhava_config,
                riseset_config,
                aya_config,
                &BindusConfig::default(),
                ctx,
            )?;
            &local_bindus
        };

        let mut bindu_lons = [0.0f64; 19];
        for (i, slot) in bindu_lons.iter_mut().enumerate().take(12) {
            *slot = bindus_ref.arudha_padas[i].sidereal_longitude;
        }
        bindu_lons[12] = bindus_ref.bhrigu_bindu.sidereal_longitude;
        bindu_lons[13] = bindus_ref.pranapada_lagna.sidereal_longitude;
        bindu_lons[14] = bindus_ref.gulika.sidereal_longitude;
        bindu_lons[15] = bindus_ref.maandi.sidereal_longitude;
        bindu_lons[16] = bindus_ref.hora_lagna.sidereal_longitude;
        bindu_lons[17] = bindus_ref.ghati_lagna.sidereal_longitude;
        bindu_lons[18] = bindus_ref.sree_lagna.sidereal_longitude;

        let mut entries = [[DrishtiEntry::zero(); 19]; 9];
        for g in ALL_GRAHAS {
            let gi = g.index() as usize;
            for (bi, lon) in bindu_lons.iter().enumerate() {
                entries[gi][bi] = graha_drishti(g, graha_lons.longitudes[gi], *lon);
            }
        }
        entries
    } else {
        [[DrishtiEntry::zero(); 19]; 9]
    };

    Ok(DrishtiResult {
        graha_to_graha,
        graha_to_bhava,
        graha_to_rashi_bhava,
        graha_to_lagna,
        graha_to_bindus,
    })
}

/// Compute Chara Karakas for a given date.
pub fn charakaraka_for_date(
    engine: &Engine,
    eop: &EopKernel,
    utc: &UtcTime,
    aya_config: &SankrantiConfig,
    scheme: CharakarakaScheme,
) -> Result<CharakarakaResult, SearchError> {
    let mut ctx = JyotishContext::new(engine, Some(eop), utc, aya_config);
    let lons = ctx.graha_lons(engine, aya_config)?;
    Ok(charakarakas_from_longitudes(&lons.longitudes, scheme))
}

/// Compute a full kundali in one shot, sharing intermediates across sections.
#[allow(clippy::too_many_arguments)]
pub fn full_kundali_for_date(
    engine: &Engine,
    eop: &EopKernel,
    utc: &UtcTime,
    location: &GeoLocation,
    bhava_config: &BhavaConfig,
    riseset_config: &RiseSetConfig,
    aya_config: &SankrantiConfig,
    config: &FullKundaliConfig,
) -> Result<FullKundaliResult, SearchError> {
    let mut ctx = JyotishContext::new(engine, Some(eop), utc, aya_config);
    let amsha_plan = resolve_amsha_plan(
        &config.amsha_selection,
        config.include_shadbala,
        config.include_vimsopaka,
        config.include_avastha,
    )?;
    if !amsha_plan.is_empty() {
        ctx.prime_amsha_graha_data(engine, aya_config, amsha_plan.requests())?;
    }

    // Ayanamsha — always computed, used for bhava cusp display.
    let ayanamsha = ctx.ayanamsha;

    // Bhava cusps — only computed when requested.
    let bhava_cusps = if config.include_bhava_cusps {
        let tropical = *ctx.bhava_result(engine, eop, location, bhava_config)?;
        Some(siderealize_bhava_result(
            &tropical,
            ayanamsha,
            ctx.reference_plane,
        ))
    } else {
        None
    };
    let rashi_bhava_cusps =
        if config.include_bhava_cusps && bhava_config.include_rashi_bhava_results {
            Some(ctx.rashi_bhava_result(engine, eop, location)?)
        } else {
            None
        };

    let graha_positions = if config.include_graha_positions {
        Some(graha_positions_with_ctx(
            engine,
            eop,
            location,
            bhava_config,
            aya_config,
            &config.graha_positions_config,
            &mut ctx,
        )?)
    } else {
        None
    };

    let bindus = if config.include_bindus {
        Some(core_bindus_with_ctx(
            engine,
            eop,
            utc,
            location,
            bhava_config,
            riseset_config,
            aya_config,
            &config.bindus_config,
            &mut ctx,
        )?)
    } else {
        None
    };

    let drishti = if config.include_drishti {
        Some(drishti_for_date_with_ctx(
            engine,
            eop,
            utc,
            location,
            bhava_config,
            riseset_config,
            aya_config,
            &config.drishti_config,
            &mut ctx,
            bindus.as_ref(),
        )?)
    } else {
        None
    };

    let ashtakavarga = if config.include_ashtakavarga {
        if let Some(positions) = graha_positions.as_ref() {
            if config.graha_positions_config.include_lagna {
                Some(calculate_ashtakavarga_from_positions(positions))
            } else {
                Some(ashtakavarga_with_ctx(
                    engine, eop, location, aya_config, &mut ctx,
                )?)
            }
        } else {
            Some(ashtakavarga_with_ctx(
                engine, eop, location, aya_config, &mut ctx,
            )?)
        }
    } else {
        None
    };

    let upagrahas = if config.include_upagrahas {
        Some(all_upagrahas_for_date_with_ctx(
            engine,
            eop,
            utc,
            location,
            riseset_config,
            aya_config,
            &config.upagraha_config,
            &mut ctx,
        )?)
    } else {
        None
    };

    let sphutas = if config.include_sphutas {
        Some(SphutalResult {
            longitudes: all_sphuta_lons_with_ctx(
                engine,
                eop,
                utc,
                location,
                riseset_config,
                aya_config,
                &config.upagraha_config,
                &mut ctx,
            )?,
        })
    } else {
        None
    };

    let special_lagnas = if config.include_special_lagnas {
        Some(special_lagnas_for_date_with_ctx(
            engine,
            eop,
            utc,
            location,
            riseset_config,
            aya_config,
            &mut ctx,
        )?)
    } else {
        None
    };

    // Amsha charts: computed if requested, after all D1 positions are resolved.
    let amshas = if config.include_amshas {
        Some(amsha_charts_from_kundali_with_ctx(
            amsha_plan.requests(),
            &config.amsha_scope,
            &graha_positions,
            &rashi_bhava_cusps,
            &bindus,
            &upagrahas,
            &sphutas,
            &special_lagnas,
            &mut ctx,
        )?)
    } else {
        None
    };

    let panchang = if config.include_panchang || config.include_calendar {
        let info = panchang_for_date(
            engine,
            eop,
            utc,
            location,
            riseset_config,
            aya_config,
            config.include_calendar,
        )?;
        if let Some(masa) = info.masa {
            ctx.masa_info = Some(masa);
        }
        if let Some(varsha) = info.varsha {
            ctx.varsha_info = Some(varsha);
        }
        Some(info)
    } else {
        None
    };

    let shadbala = if config.include_shadbala {
        Some(shadbala_for_date_with_ctx(
            engine,
            eop,
            utc,
            location,
            bhava_config,
            riseset_config,
            aya_config,
            &amsha_plan,
            &mut ctx,
        )?)
    } else {
        None
    };

    let bhavabala = if config.include_bhavabala {
        Some(bhavabala_for_date_with_ctx(
            engine,
            eop,
            utc,
            location,
            bhava_config,
            riseset_config,
            aya_config,
            shadbala.as_ref(),
            &mut ctx,
        )?)
    } else {
        None
    };

    let vimsopaka = if config.include_vimsopaka {
        Some(vimsopaka_for_date_with_ctx(
            engine,
            eop,
            location,
            aya_config,
            config.node_dignity_policy,
            &amsha_plan,
            &mut ctx,
        )?)
    } else {
        None
    };

    let avastha = if config.include_avastha {
        Some(avastha_for_date_with_ctx(
            engine,
            eop,
            location,
            utc,
            bhava_config,
            riseset_config,
            aya_config,
            config.node_dignity_policy,
            &amsha_plan,
            &mut ctx,
        )?)
    } else {
        None
    };

    let charakaraka = if config.include_charakaraka {
        let graha_lons = *ctx.graha_lons(engine, aya_config)?;
        Some(charakarakas_from_longitudes(
            &graha_lons.longitudes,
            config.charakaraka_scheme,
        ))
    } else {
        None
    };

    let (dasha, dasha_snapshots) = if config.include_dasha && config.dasha_config.count > 0 {
        compute_kundali_dashas(
            engine,
            eop,
            utc,
            location,
            riseset_config,
            aya_config,
            &config.dasha_config,
            &mut ctx,
        )?
    } else {
        (None, None)
    };

    Ok(FullKundaliResult {
        ayanamsha_deg: ayanamsha,
        bhava_cusps,
        rashi_bhava_cusps,
        graha_positions,
        bindus,
        drishti,
        ashtakavarga,
        upagrahas,
        sphutas,
        special_lagnas,
        amshas,
        shadbala,
        bhavabala,
        vimsopaka,
        avastha,
        charakaraka,
        panchang,
        dasha,
        dasha_snapshots,
    })
}

/// Compute Bhava Bala for all 12 houses at a given date and location.
pub fn bhavabala_for_date(
    engine: &Engine,
    eop: &EopKernel,
    utc: &UtcTime,
    location: &GeoLocation,
    bhava_config: &BhavaConfig,
    riseset_config: &RiseSetConfig,
    aya_config: &SankrantiConfig,
) -> Result<BhavaBalaResult, SearchError> {
    let mut ctx = JyotishContext::new(engine, Some(eop), utc, aya_config);
    bhavabala_for_date_with_ctx(
        engine,
        eop,
        utc,
        location,
        bhava_config,
        riseset_config,
        aya_config,
        None,
        &mut ctx,
    )
}

#[allow(clippy::too_many_arguments)]
fn bhavabala_for_date_with_ctx(
    engine: &Engine,
    eop: &EopKernel,
    utc: &UtcTime,
    location: &GeoLocation,
    bhava_config: &BhavaConfig,
    riseset_config: &RiseSetConfig,
    aya_config: &SankrantiConfig,
    precomputed_shadbala: Option<&ShadbalaResult>,
    ctx: &mut JyotishContext,
) -> Result<BhavaBalaResult, SearchError> {
    let owned_shadbala;
    let shadbala = if let Some(result) = precomputed_shadbala {
        result
    } else {
        let default_amsha_plan =
            resolve_amsha_plan(&AmshaSelectionConfig::default(), true, false, false)?;
        ctx.prime_amsha_graha_data(engine, aya_config, default_amsha_plan.requests())?;
        owned_shadbala = shadbala_for_date_with_ctx(
            engine,
            eop,
            utc,
            location,
            bhava_config,
            riseset_config,
            aya_config,
            &default_amsha_plan,
            ctx,
        )?;
        &owned_shadbala
    };

    let inputs = assemble_bhavabala_inputs(
        engine,
        eop,
        utc,
        location,
        bhava_config,
        riseset_config,
        aya_config,
        shadbala,
        ctx,
    )?;
    Ok(calculate_bhava_bala(&inputs))
}

/// Compute Bhava Bala for a single bhava (1-12).
#[allow(clippy::too_many_arguments)]
pub fn bhavabala_for_bhava(
    engine: &Engine,
    eop: &EopKernel,
    utc: &UtcTime,
    location: &GeoLocation,
    bhava_config: &BhavaConfig,
    riseset_config: &RiseSetConfig,
    aya_config: &SankrantiConfig,
    bhava_number: u8,
) -> Result<dhruv_vedic_base::BhavaBalaEntry, SearchError> {
    if bhava_number == 0 || bhava_number > 12 {
        return Err(SearchError::InvalidConfig(
            "bhavabala bhava_number must be in 1..=12",
        ));
    }
    let mut ctx = JyotishContext::new(engine, Some(eop), utc, aya_config);
    let default_amsha_plan =
        resolve_amsha_plan(&AmshaSelectionConfig::default(), true, false, false)?;
    ctx.prime_amsha_graha_data(engine, aya_config, default_amsha_plan.requests())?;
    let shadbala = shadbala_for_date_with_ctx(
        engine,
        eop,
        utc,
        location,
        bhava_config,
        riseset_config,
        aya_config,
        &default_amsha_plan,
        &mut ctx,
    )?;
    let inputs = assemble_bhavabala_inputs(
        engine,
        eop,
        utc,
        location,
        bhava_config,
        riseset_config,
        aya_config,
        &shadbala,
        &mut ctx,
    )?;
    Ok(bhava_bala_entry(&inputs, bhava_number as usize - 1))
}

/// Compute the bundled bala surfaces for one chart in a shared-context pass.
#[allow(clippy::too_many_arguments)]
pub fn balas_for_date(
    engine: &Engine,
    eop: &EopKernel,
    utc: &UtcTime,
    location: &GeoLocation,
    bhava_config: &BhavaConfig,
    riseset_config: &RiseSetConfig,
    aya_config: &SankrantiConfig,
    node_policy: NodeDignityPolicy,
    amsha_selection: &AmshaSelectionConfig,
) -> Result<BalaBundleResult, SearchError> {
    let mut ctx = JyotishContext::new(engine, Some(eop), utc, aya_config);
    let amsha_plan = resolve_amsha_plan(amsha_selection, true, true, false)?;
    ctx.prime_amsha_graha_data(engine, aya_config, amsha_plan.requests())?;
    let shadbala = shadbala_for_date_with_ctx(
        engine,
        eop,
        utc,
        location,
        bhava_config,
        riseset_config,
        aya_config,
        &amsha_plan,
        &mut ctx,
    )?;
    let vimsopaka = vimsopaka_for_date_with_ctx(
        engine,
        eop,
        location,
        aya_config,
        node_policy,
        &amsha_plan,
        &mut ctx,
    )?;
    let ashtakavarga = ashtakavarga_with_ctx(engine, eop, location, aya_config, &mut ctx)?;
    let bhavabala = bhavabala_for_date_with_ctx(
        engine,
        eop,
        utc,
        location,
        bhava_config,
        riseset_config,
        aya_config,
        Some(&shadbala),
        &mut ctx,
    )?;
    Ok(BalaBundleResult {
        shadbala,
        vimsopaka,
        ashtakavarga,
        bhavabala,
    })
}

// ---------------------------------------------------------------------------
// Dasha context-sharing helpers for FullKundali integration
// ---------------------------------------------------------------------------

/// Build DashaInputs from JyotishContext for a given system.
#[allow(clippy::too_many_arguments)]
fn build_dasha_inputs_from_ctx<'a>(
    engine: &Engine,
    eop: &EopKernel,
    utc: &UtcTime,
    location: &GeoLocation,
    riseset_config: &RiseSetConfig,
    aya_config: &SankrantiConfig,
    system: dhruv_vedic_base::dasha::DashaSystem,
    rashi_inputs_storage: &'a mut Option<dhruv_vedic_base::dasha::RashiDashaInputs>,
    ctx: &mut JyotishContext,
) -> Result<DashaInputs<'a>, SearchError> {
    let moon_sid_lon = if needs_moon_lon(system) {
        Some(ctx.graha_lons(engine, aya_config)?.longitudes[1])
    } else {
        None
    };

    if is_rashi_system(system) {
        let graha_lons = *ctx.graha_lons(engine, aya_config)?;
        let lagna_sid = ctx.lagna_sid(engine, eop, location)?;
        *rashi_inputs_storage = Some(dhruv_vedic_base::dasha::RashiDashaInputs::new(
            graha_lons.longitudes,
            lagna_sid,
        ));
    }

    let sunrise_sunset = if needs_sunrise_sunset(system) {
        let (sunrise, _) = ctx.sunrise_pair(engine, eop, utc, location, riseset_config)?;
        let sunset = ctx.sunset_jd(engine, eop, location, riseset_config)?;
        Some((sunrise, sunset))
    } else {
        None
    };

    Ok(DashaInputs {
        moon_sid_lon,
        rashi_inputs: rashi_inputs_storage.as_ref(),
        sunrise_sunset,
    })
}

/// Compute dasha hierarchies and optional snapshots for FullKundali.
///
/// Implements the fallback contract: per-system failures are skipped (logged),
/// dense output with preserved ordering, None for all-fail.
type KundaliDashaResults = (
    Option<Vec<dhruv_vedic_base::DashaHierarchy>>,
    Option<Vec<dhruv_vedic_base::DashaSnapshot>>,
);

#[allow(clippy::too_many_arguments)]
fn all_sphuta_lons_with_ctx(
    engine: &Engine,
    eop: &EopKernel,
    utc: &UtcTime,
    location: &GeoLocation,
    riseset_config: &RiseSetConfig,
    aya_config: &SankrantiConfig,
    upagraha_config: &TimeUpagrahaConfig,
    ctx: &mut JyotishContext,
) -> Result<[f64; 16], SearchError> {
    let gl = *ctx.graha_lons(engine, aya_config)?;
    let sun_sid = gl.longitude(Graha::Surya);
    let moon_sid = gl.longitude(Graha::Chandra);
    let rahu_sid = gl.longitude(Graha::Rahu);
    let mars_sid = gl.longitude(Graha::Mangal);
    let jupiter_sid = gl.longitude(Graha::Guru);
    let venus_sid = gl.longitude(Graha::Shukra);
    let lagna_sid = ctx.lagna_sid(engine, eop, location)?;

    let upagrahas = *ctx.upagrahas(
        engine,
        eop,
        utc,
        location,
        riseset_config,
        aya_config,
        upagraha_config,
    )?;

    let eighth_cusp_sid = normalize(lagna_sid + 210.0);
    let eighth_rashi_idx = (eighth_cusp_sid / 30.0).floor().min(11.0) as u8;
    let eighth_lord = rashi_lord_by_index(eighth_rashi_idx).unwrap_or(Graha::Surya);
    let eighth_lord_lon = gl.longitude(eighth_lord);

    let inputs = dhruv_vedic_base::SphutalInputs {
        sun: sun_sid,
        moon: moon_sid,
        mars: mars_sid,
        jupiter: jupiter_sid,
        venus: venus_sid,
        rahu: rahu_sid,
        lagna: lagna_sid,
        eighth_lord: eighth_lord_lon,
        gulika: upagrahas.gulika,
    };
    let all = all_sphutas(&inputs);
    let mut lons = [0.0f64; 16];
    for (i, (_sphuta, lon)) in all.iter().enumerate() {
        lons[i] = *lon;
    }
    Ok(lons)
}

#[allow(clippy::too_many_arguments)]
fn compute_kundali_dashas(
    engine: &Engine,
    eop: &EopKernel,
    utc: &UtcTime,
    location: &GeoLocation,
    riseset_config: &RiseSetConfig,
    aya_config: &SankrantiConfig,
    dasha_config: &DashaSelectionConfig,
    ctx: &mut JyotishContext,
) -> Result<KundaliDashaResults, SearchError> {
    // sanitize() MUST run before validate() — see plan Phase C note
    let mut config = *dasha_config;
    config.sanitize();
    config.validate().map_err(SearchError::from)?;

    let birth_jd = utc_to_jd_utc(utc);
    let variation = config.to_variation_config();

    let mut hierarchies = Vec::new();
    let mut snapshots = Vec::new();

    for i in 0..config.count as usize {
        let system_code = config.systems[i];
        let system = match dhruv_vedic_base::dasha::DashaSystem::from_u8(system_code) {
            Some(s) => s,
            None => continue,
        };

        // Build inputs from ctx (may fail for systems needing sunrise at polar locations)
        let mut rashi_storage = None;
        let inputs = match build_dasha_inputs_from_ctx(
            engine,
            eop,
            utc,
            location,
            riseset_config,
            aya_config,
            system,
            &mut rashi_storage,
            ctx,
        ) {
            Ok(i) => i,
            Err(e) => {
                eprintln!("dasha: skipping {system:?}: {e}");
                continue;
            }
        };

        let max_level = config.effective_max_level(i);
        match dasha_hierarchy_with_inputs(birth_jd, system, max_level, &variation, &inputs) {
            Ok(hierarchy) => {
                hierarchies.push(hierarchy);
                // Attempt snapshot only if hierarchy succeeded and snapshot_time is set
                if let Some(snapshot_time) = config.snapshot_time {
                    let query_jd = dasha_snapshot_time_to_jd_utc(snapshot_time);
                    match dasha_snapshot_with_inputs(
                        birth_jd, query_jd, system, max_level, &variation, &inputs,
                    ) {
                        Ok(snap) => snapshots.push(snap),
                        Err(e) => {
                            eprintln!("dasha snapshot: skipping {system:?}: {e}");
                        }
                    }
                }
            }
            Err(e) => {
                eprintln!("dasha: skipping {system:?}: {e}");
            }
        }
    }

    let dasha = if hierarchies.is_empty() {
        None
    } else {
        Some(hierarchies)
    };
    let dasha_snapshots = if config.snapshot_time.is_some() && !snapshots.is_empty() {
        Some(snapshots)
    } else {
        None
    };
    Ok((dasha, dasha_snapshots))
}

/// Convert UtcTime to JD UTC (calendar only, no TDB conversion).
fn utc_to_jd_utc(utc: &UtcTime) -> f64 {
    // Julian Day from calendar date (Meeus algorithm)
    let y = utc.year as f64;
    let m = utc.month as f64;
    let d =
        utc.day as f64 + utc.hour as f64 / 24.0 + utc.minute as f64 / 1440.0 + utc.second / 86400.0;

    let (y2, m2) = if m <= 2.0 {
        (y - 1.0, m + 12.0)
    } else {
        (y, m)
    };
    let a = (y2 / 100.0).floor();
    let b = 2.0 - a + (a / 4.0).floor();

    (365.25 * (y2 + 4716.0)).floor() + (30.6001 * (m2 + 1.0)).floor() + d + b - 1524.5
}

fn dasha_snapshot_time_to_jd_utc(snapshot_time: DashaSnapshotTime) -> f64 {
    match snapshot_time {
        DashaSnapshotTime::Utc(utc) => utc_to_jd_utc(&utc),
        DashaSnapshotTime::JdUtc(jd_utc) => jd_utc,
    }
}

/// Normalize longitude to [0, 360).
fn normalize(deg: f64) -> f64 {
    let r = deg % 360.0;
    if r < 0.0 { r + 360.0 } else { r }
}

// ---------------------------------------------------------------------------
// Shadbala & Vimsopaka orchestration
// ---------------------------------------------------------------------------

/// Compute Shadbala for all 7 sapta grahas at a given date and location.
pub fn shadbala_for_date(
    engine: &Engine,
    eop: &EopKernel,
    utc: &UtcTime,
    location: &GeoLocation,
    bhava_config: &BhavaConfig,
    riseset_config: &RiseSetConfig,
    aya_config: &SankrantiConfig,
    amsha_selection: &AmshaSelectionConfig,
) -> Result<ShadbalaResult, SearchError> {
    let mut ctx = JyotishContext::new(engine, Some(eop), utc, aya_config);
    let amsha_plan = resolve_amsha_plan(amsha_selection, true, false, false)?;
    ctx.prime_amsha_graha_data(engine, aya_config, amsha_plan.requests())?;
    shadbala_for_date_with_ctx(
        engine,
        eop,
        utc,
        location,
        bhava_config,
        riseset_config,
        aya_config,
        &amsha_plan,
        &mut ctx,
    )
}

#[allow(clippy::too_many_arguments)]
fn shadbala_for_date_with_ctx(
    engine: &Engine,
    eop: &EopKernel,
    utc: &UtcTime,
    location: &GeoLocation,
    bhava_config: &BhavaConfig,
    riseset_config: &RiseSetConfig,
    aya_config: &SankrantiConfig,
    amsha_plan: &ResolvedAmshaPlan,
    ctx: &mut JyotishContext,
) -> Result<ShadbalaResult, SearchError> {
    let inputs = assemble_shadbala_inputs(
        engine,
        eop,
        utc,
        location,
        bhava_config,
        riseset_config,
        aya_config,
        amsha_plan,
        ctx,
    )?;
    let breakdowns = all_shadbalas_from_inputs(&inputs);
    let mut entries = [ShadbalaEntry::from_breakdown(Graha::Surya, &breakdowns[0]); 7];
    for (i, graha) in SAPTA_GRAHAS.iter().enumerate() {
        entries[i] = ShadbalaEntry::from_breakdown(*graha, &breakdowns[i]);
    }
    Ok(ShadbalaResult { entries })
}

/// Compute Shadbala for a single sapta graha. Returns error for Rahu/Ketu.
#[allow(clippy::too_many_arguments)]
pub fn shadbala_for_graha(
    engine: &Engine,
    eop: &EopKernel,
    utc: &UtcTime,
    location: &GeoLocation,
    bhava_config: &BhavaConfig,
    riseset_config: &RiseSetConfig,
    aya_config: &SankrantiConfig,
    amsha_selection: &AmshaSelectionConfig,
    graha: Graha,
) -> Result<ShadbalaEntry, SearchError> {
    if graha.index() >= 7 {
        return Err(SearchError::InvalidConfig(
            "shadbala is defined for sapta grahas only (Sun..Saturn)",
        ));
    }
    let mut ctx = JyotishContext::new(engine, Some(eop), utc, aya_config);
    let amsha_plan = resolve_amsha_plan(amsha_selection, true, false, false)?;
    ctx.prime_amsha_graha_data(engine, aya_config, amsha_plan.requests())?;
    let inputs = assemble_shadbala_inputs(
        engine,
        eop,
        utc,
        location,
        bhava_config,
        riseset_config,
        aya_config,
        &amsha_plan,
        &mut ctx,
    )?;
    Ok(ShadbalaEntry::from_breakdown(
        graha,
        &shadbala_from_inputs(graha, &inputs),
    ))
}

/// Compute Vimsopaka Bala for all 9 navagrahas at a given date and location.
pub fn vimsopaka_for_date(
    engine: &Engine,
    eop: &EopKernel,
    utc: &UtcTime,
    location: &GeoLocation,
    aya_config: &SankrantiConfig,
    node_policy: NodeDignityPolicy,
    amsha_selection: &AmshaSelectionConfig,
) -> Result<VimsopakaResult, SearchError> {
    let mut ctx = JyotishContext::new(engine, Some(eop), utc, aya_config);
    let amsha_plan = resolve_amsha_plan(amsha_selection, false, true, false)?;
    ctx.prime_amsha_graha_data(engine, aya_config, amsha_plan.requests())?;
    vimsopaka_for_date_with_ctx(
        engine,
        eop,
        location,
        aya_config,
        node_policy,
        &amsha_plan,
        &mut ctx,
    )
}

fn vimsopaka_for_date_with_ctx(
    engine: &Engine,
    eop: &EopKernel,
    location: &GeoLocation,
    aya_config: &SankrantiConfig,
    node_policy: NodeDignityPolicy,
    amsha_plan: &ResolvedAmshaPlan,
    ctx: &mut JyotishContext,
) -> Result<VimsopakaResult, SearchError> {
    let shad = all_vimsopaka_balas_from_cache(
        engine,
        aya_config,
        amsha_plan,
        ctx,
        &VIMSOPAKA_SHADVARGA,
        node_policy,
    )?;
    let sapt = all_vimsopaka_balas_from_cache(
        engine,
        aya_config,
        amsha_plan,
        ctx,
        &VIMSOPAKA_SAPTAVARGA,
        node_policy,
    )?;
    let dash = all_vimsopaka_balas_from_cache(
        engine,
        aya_config,
        amsha_plan,
        ctx,
        &VIMSOPAKA_DASHAVARGA,
        node_policy,
    )?;
    let shod = all_vimsopaka_balas_from_cache(
        engine,
        aya_config,
        amsha_plan,
        ctx,
        &VIMSOPAKA_SHODASAVARGA,
        node_policy,
    )?;

    let mut entries = [VimsopakaEntry {
        graha: Graha::Surya,
        shadvarga: 0.0,
        saptavarga: 0.0,
        dashavarga: 0.0,
        shodasavarga: 0.0,
    }; 9];
    for (i, graha) in ALL_GRAHAS.iter().enumerate() {
        entries[i] = VimsopakaEntry {
            graha: *graha,
            shadvarga: shad[i].score,
            saptavarga: sapt[i].score,
            dashavarga: dash[i].score,
            shodasavarga: shod[i].score,
        };
    }
    // Suppress unused variable warnings — eop and location are part of the
    // context-sharing pattern even though vimsopaka only needs graha longitudes.
    let _ = (eop, location);
    Ok(VimsopakaResult { entries })
}

fn all_vimsopaka_balas_from_cache(
    engine: &Engine,
    aya_config: &SankrantiConfig,
    amsha_plan: &ResolvedAmshaPlan,
    ctx: &mut JyotishContext,
    vargas: &[VargaWeight],
    node_policy: NodeDignityPolicy,
) -> Result<[dhruv_vedic_base::vimsopaka::VimsopakaBala; 9], SearchError> {
    let mut results = std::array::from_fn(|_| dhruv_vedic_base::vimsopaka::VimsopakaBala {
        score: 0.0,
        entries: Vec::new(),
    });
    let mut weighted_sums = [0.0f64; 9];
    let mut total_weights = [0.0f64; 9];
    for varga in vargas {
        let cached = ctx.vimsopaka_varga_data(
            engine,
            aya_config,
            amsha_plan.request_for(varga.amsha),
            node_policy,
        )?;
        for graha in ALL_GRAHAS {
            let gi = graha.index() as usize;
            let points = cached.points[gi];
            results[gi]
                .entries
                .push(dhruv_vedic_base::vimsopaka::VargaDignityEntry {
                    amsha: varga.amsha,
                    dignity: cached.dignities[gi],
                    points,
                    weight: varga.weight,
                });
            weighted_sums[gi] += points * varga.weight;
            total_weights[gi] += varga.weight;
        }
    }
    for graha in ALL_GRAHAS {
        let gi = graha.index() as usize;
        results[gi].score = if total_weights[gi] > 0.0 {
            weighted_sums[gi] / total_weights[gi]
        } else {
            0.0
        };
    }
    Ok(results)
}

fn vimsopaka_entry_from_cache(
    engine: &Engine,
    aya_config: &SankrantiConfig,
    amsha_plan: &ResolvedAmshaPlan,
    ctx: &mut JyotishContext,
    graha: Graha,
    node_policy: NodeDignityPolicy,
) -> Result<VimsopakaEntry, SearchError> {
    let gi = graha.index() as usize;
    let group_score =
        |vargas: &[VargaWeight], ctx: &mut JyotishContext| -> Result<f64, SearchError> {
            let mut weighted_sum = 0.0;
            let mut total_weight = 0.0;
            for varga in vargas {
                let cached = ctx.vimsopaka_varga_data(
                    engine,
                    aya_config,
                    amsha_plan.request_for(varga.amsha),
                    node_policy,
                )?;
                weighted_sum += cached.points[gi] * varga.weight;
                total_weight += varga.weight;
            }
            Ok(if total_weight > 0.0 {
                weighted_sum / total_weight
            } else {
                0.0
            })
        };

    Ok(VimsopakaEntry {
        graha,
        shadvarga: group_score(&VIMSOPAKA_SHADVARGA, ctx)?,
        saptavarga: group_score(&VIMSOPAKA_SAPTAVARGA, ctx)?,
        dashavarga: group_score(&VIMSOPAKA_DASHAVARGA, ctx)?,
        shodasavarga: group_score(&VIMSOPAKA_SHODASAVARGA, ctx)?,
    })
}

/// Compute Vimsopaka Bala for a single graha. Accepts all 9 navagrahas.
pub fn vimsopaka_for_graha(
    engine: &Engine,
    eop: &EopKernel,
    utc: &UtcTime,
    location: &GeoLocation,
    aya_config: &SankrantiConfig,
    node_policy: NodeDignityPolicy,
    amsha_selection: &AmshaSelectionConfig,
    graha: Graha,
) -> Result<VimsopakaEntry, SearchError> {
    let mut ctx = JyotishContext::new(engine, Some(eop), utc, aya_config);
    let amsha_plan = resolve_amsha_plan(amsha_selection, false, true, false)?;
    ctx.prime_amsha_graha_data(engine, aya_config, amsha_plan.requests())?;
    let _ = location;
    vimsopaka_entry_from_cache(
        engine,
        aya_config,
        &amsha_plan,
        &mut ctx,
        graha,
        node_policy,
    )
}

/// Assemble ShadbalaInputs from engine queries and context.
#[allow(clippy::too_many_arguments)]
fn assemble_shadbala_inputs(
    engine: &Engine,
    eop: &EopKernel,
    utc: &UtcTime,
    location: &GeoLocation,
    bhava_config: &BhavaConfig,
    riseset_config: &RiseSetConfig,
    aya_config: &SankrantiConfig,
    amsha_plan: &ResolvedAmshaPlan,
    ctx: &mut JyotishContext,
) -> Result<ShadbalaInputs, SearchError> {
    // 1. Sidereal longitudes (all 9, needed for drik bala)
    let graha_lons = *ctx.graha_lons(engine, aya_config)?;
    let sidereal_lons = graha_lons.longitudes;

    // 2. Bhava numbers for sapta grahas
    let bhava_basis = ctx.bala_bhava_basis(engine, eop, location, bhava_config, aya_config)?;
    let mut bhava_numbers = [0u8; 7];
    let mut dig_bala_max_cusp_lons = [0.0f64; 7];
    for graha in SAPTA_GRAHAS {
        let idx = graha.index() as usize;
        bhava_numbers[idx] = bhava_basis.bhava_numbers[idx];
        let max_bhava = DIG_BALA_BHAVA[idx] as usize;
        dig_bala_max_cusp_lons[idx] = bhava_basis.cusp_sidereal_lons[max_bhava - 1];
    }

    // 3. Correction-model motion inputs for Cheshta Bala (Surya/Chandra remain 0).
    let cheshta_config = GrahaLongitudesConfig::sidereal_with_model(
        aya_config.ayanamsha_system,
        aya_config.use_nutation,
        aya_config.precession_model,
        aya_config.reference_plane,
    );
    let mut cheshta_madhyama_lons = [0.0f64; 7];
    let mut cheshta_chaloccha_lons = [0.0f64; 7];
    for graha in [
        Graha::Mangal,
        Graha::Buddh,
        Graha::Guru,
        Graha::Shukra,
        Graha::Shani,
    ] {
        let idx = graha.index() as usize;
        let motion = ctx.cheshta_motion(engine, graha, sidereal_lons[idx], cheshta_config)?;
        cheshta_madhyama_lons[idx] = motion.madhyama_longitude;
        cheshta_chaloccha_lons[idx] = motion.chaloccha_longitude;
    }

    // 4. Varga data: 7 vargas × 7 grahas for saptavargaja bala
    let (varga_rashi_indices, varga_longitudes) =
        assemble_varga_data(engine, aya_config, ctx, amsha_plan)?;

    // 5. Kala Bala inputs
    let (jd_sunrise, jd_next_sunrise) =
        ctx.sunrise_pair(engine, eop, utc, location, riseset_config)?;
    let jd_sunset = ctx.sunset_jd(engine, eop, location, riseset_config)?;

    let is_daytime = ctx.jd_tdb >= jd_sunrise && ctx.jd_tdb < jd_sunset;
    let local_day_fraction = (ctx.jd_tdb + 0.5 + location.longitude_deg / 360.0).rem_euclid(1.0);

    // Day/night fraction: position within the day or night portion (0-1)
    let day_night_fraction = if is_daytime {
        let day_len = jd_sunset - jd_sunrise;
        if day_len > 0.0 {
            ((ctx.jd_tdb - jd_sunrise) / day_len).clamp(0.0, 1.0)
        } else {
            0.5
        }
    } else {
        let night_len = jd_next_sunrise - jd_sunset;
        if night_len > 0.0 {
            let elapsed = if ctx.jd_tdb >= jd_sunset {
                ctx.jd_tdb - jd_sunset
            } else {
                // Before sunset but after previous sunrise — shouldn't happen
                // if is_daytime is false, but handle gracefully
                0.0
            };
            (elapsed / night_len).clamp(0.0, 1.0)
        } else {
            0.5
        }
    };

    // Moon-Sun elongation for paksha/nathonnatha
    let sun_sid = sidereal_lons[Graha::Surya.index() as usize];
    let moon_sid = sidereal_lons[Graha::Chandra.index() as usize];
    let moon_sun_elongation = normalize(moon_sid - sun_sid);

    // Lord resolution
    let (year_lord, month_lord, weekday_lord, hora_lord_graha) =
        resolve_kala_lords(engine, eop, utc, location, riseset_config, aya_config, ctx)?;

    // Declinations for sapta grahas
    let graha_declinations = ctx.graha_declinations(engine)?;

    Ok(ShadbalaInputs {
        sidereal_lons,
        bhava_numbers,
        dig_bala_max_cusp_lons,
        cheshta_madhyama_lons,
        cheshta_chaloccha_lons,
        kala: KalaBalaInputs {
            is_daytime,
            day_night_fraction,
            local_day_fraction,
            moon_sun_elongation,
            year_lord,
            month_lord,
            weekday_lord,
            hora_lord: hora_lord_graha,
            graha_declinations,
            sidereal_lons: {
                let mut s7 = [0.0f64; 7];
                s7.copy_from_slice(&sidereal_lons[..7]);
                s7
            },
        },
        include_node_aspects_for_drik_bala: bhava_config.include_node_aspects_for_drik_bala,
        divide_guru_buddh_drishti_by_4_for_drik_bala: bhava_config
            .divide_guru_buddh_drishti_by_4_for_drik_bala,
        chandra_benefic_rule: bhava_config.chandra_benefic_rule,
        varga_rashi_indices,
        varga_longitudes,
    })
}

#[allow(clippy::too_many_arguments)]
fn assemble_bhavabala_inputs(
    engine: &Engine,
    eop: &EopKernel,
    utc: &UtcTime,
    location: &GeoLocation,
    bhava_config: &BhavaConfig,
    riseset_config: &RiseSetConfig,
    aya_config: &SankrantiConfig,
    shadbala: &ShadbalaResult,
    ctx: &mut JyotishContext,
) -> Result<BhavaBalaInputs, SearchError> {
    let graha_lons = *ctx.graha_lons(engine, aya_config)?;
    let bhava_basis = ctx.bala_bhava_basis(engine, eop, location, bhava_config, aya_config)?;
    let cusp_sidereal_lons = bhava_basis.cusp_sidereal_lons;
    let mut house_lord_strengths = [0.0; 12];
    for i in 0..12 {
        let rashi_index = rashi_from_longitude(cusp_sidereal_lons[i]).rashi_index;
        let lord = rashi_lord_by_index(rashi_index).ok_or(SearchError::InvalidConfig(
            "invalid bhava rashi for bhavabala",
        ))?;
        if lord.index() >= 7 {
            return Err(SearchError::InvalidConfig(
                "bhava lord must be a sapta graha for bhavabala",
            ));
        }
        house_lord_strengths[i] = shadbala.entries[lord.index() as usize].total_shashtiamsas;
    }

    let mut aspect_virupas = [[0.0; 12]; 9];
    for graha in ALL_GRAHAS {
        let gi = graha.index() as usize;
        for (bi, &cusp_sid) in cusp_sidereal_lons.iter().enumerate() {
            aspect_virupas[gi][bi] =
                graha_drishti(graha, graha_lons.longitudes[gi], cusp_sid).total_virupa;
        }
    }

    let (vedic_sunrise, _next_sunrise) =
        ctx.sunrise_pair(engine, eop, utc, location, riseset_config)?;
    let vedic_sunset = ctx.sunset_jd(engine, eop, location, riseset_config)?;

    let birth_period = classify_bhavabala_birth_period(ctx.jd_tdb, vedic_sunrise, vedic_sunset);
    Ok(BhavaBalaInputs {
        cusp_sidereal_lons,
        ascendant_sidereal_lon: bhava_basis.ascendant_sidereal_lon,
        meridian_sidereal_lon: bhava_basis.meridian_sidereal_lon,
        graha_bhava_numbers: bhava_basis.bhava_numbers,
        house_lord_strengths,
        aspect_virupas,
        birth_period,
    })
}

fn classify_bhavabala_birth_period(
    jd_tdb: f64,
    vedic_sunrise: f64,
    vedic_sunset: f64,
) -> BhavaBalaBirthPeriod {
    let sunrise_start = vedic_sunrise - BHAVABALA_TWILIGHT_HALF_DAYS;
    let sunrise_end = vedic_sunrise + BHAVABALA_TWILIGHT_HALF_DAYS;
    let sunset_start = vedic_sunset - BHAVABALA_TWILIGHT_HALF_DAYS;
    let sunset_end = vedic_sunset + BHAVABALA_TWILIGHT_HALF_DAYS;

    if (jd_tdb >= sunrise_start && jd_tdb < sunrise_end)
        || (jd_tdb >= sunset_start && jd_tdb < sunset_end)
    {
        BhavaBalaBirthPeriod::Twilight
    } else if jd_tdb >= sunrise_end && jd_tdb < sunset_start {
        BhavaBalaBirthPeriod::Day
    } else {
        BhavaBalaBirthPeriod::Night
    }
}

/// Compute varga longitudes and rashi indices for the 7 saptavargaja vargas × 7 sapta grahas.
///
/// The 7 vargas are: D1, D2, D3, D7, D9, D12, D30.
fn assemble_varga_data(
    engine: &Engine,
    aya_config: &SankrantiConfig,
    ctx: &mut JyotishContext,
    plan: &ResolvedAmshaPlan,
) -> Result<([[u8; 7]; 7], [[f64; 7]; 7]), SearchError> {
    let mut rashi_indices = [[0u8; 7]; 7];
    let mut longitudes = [[0.0f64; 7]; 7];
    for (vi, amsha) in SHADBALA_REQUIRED_AMSHAS.iter().enumerate() {
        let cached = ctx.amsha_graha_data(engine, aya_config, plan.request_for(*amsha))?;
        rashi_indices[vi].copy_from_slice(&cached.rashi_indices[..7]);
        longitudes[vi].copy_from_slice(&cached.longitudes[..7]);
    }
    Ok((rashi_indices, longitudes))
}

/// Resolve the four Kala Bala lords: year, month, weekday, hora.
fn resolve_kala_lords(
    engine: &Engine,
    eop: &EopKernel,
    utc: &UtcTime,
    location: &GeoLocation,
    riseset_config: &RiseSetConfig,
    aya_config: &SankrantiConfig,
    ctx: &mut JyotishContext,
) -> Result<(Graha, Graha, Graha, Graha), SearchError> {
    let varsha = ctx.varsha_info(engine, eop, utc, aya_config)?;
    let year_lord =
        graha_vaar_lord(vaar_for_date(engine, eop, &varsha.start, location, riseset_config)?.vaar);

    let masa = ctx.masa_info(engine, eop, utc, aya_config)?;
    let month_lord =
        graha_vaar_lord(vaar_for_date(engine, eop, &masa.start, location, riseset_config)?.vaar);

    // Weekday lord
    let (jd_sunrise, jd_next_sunrise) =
        ctx.sunrise_pair(engine, eop, utc, location, riseset_config)?;
    let vaar = vaar_from_jd(jd_sunrise);
    let weekday_lord = graha_vaar_lord(vaar);

    // Hora lord
    let hora_info = hora_from_sunrises(ctx.jd_tdb, jd_sunrise, jd_next_sunrise, engine.lsk());
    let hora_lord_graha = graha_hora_lord(vaar, hora_info.hora_index);

    Ok((year_lord, month_lord, weekday_lord, hora_lord_graha))
}

// ---------------------------------------------------------------------------
// Avastha orchestration
// ---------------------------------------------------------------------------

/// Compute avasthas for all 9 grahas at a given UTC date and location.
#[allow(clippy::too_many_arguments)]
pub fn avastha_for_date(
    engine: &Engine,
    eop: &EopKernel,
    location: &GeoLocation,
    utc: &UtcTime,
    bhava_config: &BhavaConfig,
    riseset_config: &RiseSetConfig,
    aya_config: &SankrantiConfig,
    node_policy: NodeDignityPolicy,
    amsha_selection: &AmshaSelectionConfig,
) -> Result<AllGrahaAvasthas, SearchError> {
    let mut ctx = JyotishContext::new(engine, Some(eop), utc, aya_config);
    let amsha_plan = resolve_amsha_plan(amsha_selection, false, false, true)?;
    ctx.prime_amsha_graha_data(engine, aya_config, amsha_plan.requests())?;
    avastha_for_date_with_ctx(
        engine,
        eop,
        location,
        utc,
        bhava_config,
        riseset_config,
        aya_config,
        node_policy,
        &amsha_plan,
        &mut ctx,
    )
}

#[allow(clippy::too_many_arguments)]
fn avastha_for_date_with_ctx(
    engine: &Engine,
    eop: &EopKernel,
    location: &GeoLocation,
    utc: &UtcTime,
    bhava_config: &BhavaConfig,
    riseset_config: &RiseSetConfig,
    aya_config: &SankrantiConfig,
    node_policy: NodeDignityPolicy,
    amsha_plan: &ResolvedAmshaPlan,
    ctx: &mut JyotishContext,
) -> Result<AllGrahaAvasthas, SearchError> {
    let inputs = assemble_avastha_inputs(
        engine,
        eop,
        utc,
        location,
        bhava_config,
        riseset_config,
        aya_config,
        node_policy,
        amsha_plan,
        ctx,
    )?;
    Ok(all_avasthas(&inputs))
}

fn graha_avasthas_from_inputs(inputs: &AvasthaInputs, graha: Graha) -> GrahaAvasthas {
    let index = graha.index() as usize;
    let same_rashi = ALL_GRAHAS
        .iter()
        .copied()
        .filter(|other| {
            let other_index = other.index() as usize;
            other_index != index && inputs.rashi_indices[other_index] == inputs.rashi_indices[index]
        })
        .collect::<Vec<_>>();
    let aspecting = ALL_GRAHAS
        .iter()
        .copied()
        .filter(|other| {
            let other_index = other.index() as usize;
            other_index != index
                && inputs.lajjitadi.drishti_matrix.entries[other_index][index].total_virupa >= 45.0
        })
        .collect::<Vec<_>>();
    let sayanadi = sayanadi_avastha(
        graha,
        inputs.sayanadi.nakshatra_indices[index],
        inputs.sayanadi.navamsa_numbers[index],
        inputs.sayanadi.janma_nakshatra,
        inputs.sayanadi.birth_ghatikas,
        inputs.sayanadi.lagna_rashi_number,
    );
    GrahaAvasthas {
        baladi: baladi_avastha(inputs.sidereal_lons[index], inputs.rashi_indices[index]),
        jagradadi: jagradadi_avastha(inputs.dignities[index]),
        deeptadi: deeptadi_avastha(
            inputs.dignities[index],
            inputs.is_combust[index],
            inputs.is_retrograde[index],
            inputs.lost_war[index],
        ),
        lajjitadi: lajjitadi_avastha(
            graha,
            inputs.bhava_numbers[index],
            inputs.rashi_indices[index],
            inputs.dignities[index],
            &same_rashi,
            &aspecting,
        ),
        sayanadi: SayanadiResult {
            avastha: sayanadi,
            sub_states: sayanadi_all_sub_states(sayanadi, graha),
        },
    }
}

/// Compute avasthas for a single graha.
#[allow(clippy::too_many_arguments)]
pub fn avastha_for_graha(
    engine: &Engine,
    eop: &EopKernel,
    location: &GeoLocation,
    utc: &UtcTime,
    bhava_config: &BhavaConfig,
    riseset_config: &RiseSetConfig,
    aya_config: &SankrantiConfig,
    node_policy: NodeDignityPolicy,
    amsha_selection: &AmshaSelectionConfig,
    graha: Graha,
) -> Result<GrahaAvasthas, SearchError> {
    let mut ctx = JyotishContext::new(engine, Some(eop), utc, aya_config);
    let amsha_plan = resolve_amsha_plan(amsha_selection, false, false, true)?;
    ctx.prime_amsha_graha_data(engine, aya_config, amsha_plan.requests())?;
    let inputs = assemble_avastha_inputs(
        engine,
        eop,
        utc,
        location,
        bhava_config,
        riseset_config,
        aya_config,
        node_policy,
        &amsha_plan,
        &mut ctx,
    )?;
    Ok(graha_avasthas_from_inputs(&inputs, graha))
}

/// Assemble AvasthaInputs from engine queries and JyotishContext cache.
#[allow(clippy::too_many_arguments)]
fn assemble_avastha_inputs(
    engine: &Engine,
    eop: &EopKernel,
    utc: &UtcTime,
    location: &GeoLocation,
    bhava_config: &BhavaConfig,
    riseset_config: &RiseSetConfig,
    aya_config: &SankrantiConfig,
    node_policy: NodeDignityPolicy,
    amsha_plan: &ResolvedAmshaPlan,
    ctx: &mut JyotishContext,
) -> Result<AvasthaInputs, SearchError> {
    // 1. Graha longitudes & rashi indices
    let graha_lons = *ctx.graha_lons(engine, aya_config)?;
    let sidereal_lons = graha_lons.longitudes;
    let rashi_indices = graha_lons.all_rashi_indices();

    // 2. Bhava numbers for all 9 grahas
    let bhava_numbers = ctx
        .bala_bhava_basis(engine, eop, location, bhava_config, aya_config)?
        .bhava_numbers;

    // 3. Speeds → retrograde detection (sapta grahas only; Rahu/Ketu always false)
    let speeds = ctx.graha_speeds(engine)?;
    let mut is_retrograde = [false; 9];
    for i in 0..7 {
        is_retrograde[i] = speeds[i] < 0.0;
    }

    // 4. Declinations for war detection
    let declinations = ctx.graha_declinations(engine)?;

    // 5. Dignities: sapta grahas via compound friendship, Rahu/Ketu via node policy
    let mut sapta_rashi_indices = [0u8; 7];
    sapta_rashi_indices.copy_from_slice(&rashi_indices[..7]);
    let mut dignities = [Dignity::Sama; 9];
    for graha in SAPTA_GRAHAS {
        let idx = graha.index() as usize;
        dignities[idx] = dignity_in_rashi_with_positions(
            graha,
            sidereal_lons[idx],
            rashi_indices[idx],
            &sapta_rashi_indices,
        );
    }
    // Rahu/Ketu via node_dignity_in_rashi
    for &graha in &[Graha::Rahu, Graha::Ketu] {
        let idx = graha.index() as usize;
        dignities[idx] =
            node_dignity_in_rashi(graha, rashi_indices[idx], &rashi_indices, node_policy);
    }

    // 6. Combustion
    let is_combust = all_combustion_status(&sidereal_lons, &is_retrograde);

    // 7. War detection (indices 2-6 only)
    let mut lost_war = [false; 9];
    let mut sapta_lons = [0.0f64; 7];
    sapta_lons.copy_from_slice(&sidereal_lons[..7]);
    for (i, lost) in lost_war.iter_mut().enumerate().take(7).skip(2) {
        *lost = lost_planetary_war(i, &sapta_lons, &declinations);
    }

    // 8. Drishti matrix
    let drishti_matrix = ctx.graha_to_graha_drishti(engine, aya_config)?;

    // 9. Nakshatra indices from sidereal longitudes
    let mut nakshatra_indices = [0u8; 9];
    for i in 0..9 {
        nakshatra_indices[i] = ((sidereal_lons[i] / (360.0 / 27.0)).floor() as u8).min(26);
    }

    // 10. Navamsa numbers
    let navamsa_data =
        ctx.amsha_graha_data(engine, aya_config, amsha_plan.request_for(Amsha::D9))?;
    let mut navamsa_numbers = [0u8; 9];
    for (i, navamsa_number) in navamsa_numbers.iter_mut().enumerate() {
        *navamsa_number = navamsa_data.division_indices[i].min(8) as u8 + 1;
    }

    // 11. Janma nakshatra = Moon's nakshatra index
    let janma_nakshatra = nakshatra_indices[Graha::Chandra.index() as usize];

    // 12. Birth ghatikas (explicit floor)
    let (jd_sunrise, jd_next_sunrise) =
        ctx.sunrise_pair(engine, eop, utc, location, riseset_config)?;
    let ghatikas_f = ghatikas_since_sunrise(ctx.jd_tdb, jd_sunrise, jd_next_sunrise);
    let birth_ghatikas = ghatikas_f.floor() as u16;

    // 13. Lagna rashi number (1-12)
    let lagna_sid = ctx.lagna_sid(engine, eop, location)?;
    let lagna_rashi_number = ((lagna_sid / 30.0).floor() as u8).min(11) + 1;

    Ok(AvasthaInputs {
        sidereal_lons,
        rashi_indices,
        bhava_numbers,
        dignities,
        is_combust,
        is_retrograde,
        lost_war,
        lajjitadi: LajjitadiInputs {
            rashi_indices,
            bhava_numbers,
            dignities,
            drishti_matrix,
        },
        sayanadi: SayanadiInputs {
            nakshatra_indices,
            navamsa_numbers,
            janma_nakshatra,
            birth_ghatikas,
            lagna_rashi_number,
        },
    })
}

// ---------------------------------------------------------------------------
// Amsha (divisional chart) orchestration
// ---------------------------------------------------------------------------

/// Convert a sidereal longitude to an AmshaEntry.
fn make_amsha_entry(sidereal_lon: f64) -> AmshaEntry {
    let info = rashi_from_longitude(sidereal_lon);
    AmshaEntry {
        sidereal_longitude: sidereal_lon,
        rashi: info.rashi,
        rashi_index: info.rashi_index,
        dms: info.dms,
        degrees_in_rashi: info.degrees_in_rashi,
    }
}

/// Transform a sidereal longitude through an amsha and return an AmshaEntry.
fn transform_to_amsha_entry(sidereal_lon: f64, amsha: Amsha, variation: Option<u8>) -> AmshaEntry {
    let amsha_lon = amsha_longitude(sidereal_lon, amsha, variation);
    make_amsha_entry(amsha_lon)
}

/// Validate an AmshaRequest slice.
fn validate_amsha_requests(requests: &[AmshaRequest]) -> Result<(), SearchError> {
    if requests.len() > MAX_AMSHA_REQUESTS {
        return Err(SearchError::InvalidConfig("amsha count exceeds maximum"));
    }
    for req in requests {
        if !is_valid_amsha_variation(req.amsha, req.effective_variation()) {
            return Err(SearchError::InvalidConfig(
                "variation not applicable to amsha",
            ));
        }
    }
    Ok(())
}

fn decode_amsha_selection(sel: &AmshaSelectionConfig) -> Result<Vec<AmshaRequest>, SearchError> {
    if sel.count as usize > MAX_AMSHA_REQUESTS {
        return Err(SearchError::InvalidConfig("amsha count exceeds maximum"));
    }
    let mut requests: Vec<AmshaRequest> = Vec::with_capacity(sel.count as usize);
    for i in 0..sel.count as usize {
        let amsha = Amsha::from_code(sel.codes[i])
            .ok_or(SearchError::InvalidConfig("unknown amsha code"))?;
        let default_variation = default_amsha_variation(amsha);
        let variation = if sel.variations[i] == default_variation {
            None
        } else {
            let v = sel.variations[i];
            if !is_valid_amsha_variation(amsha, v) {
                return Err(SearchError::InvalidConfig(
                    "unknown variation code for amsha",
                ));
            }
            Some(v)
        };
        if let Some(existing) = requests.iter().find(|request| request.amsha == amsha) {
            if existing.effective_variation() != variation.unwrap_or(default_variation) {
                return Err(SearchError::InvalidConfig(
                    "conflicting amsha variation for amsha code",
                ));
            }
            continue;
        }
        requests.push(AmshaRequest { amsha, variation });
    }
    Ok(requests)
}

fn append_required_amshas(requests: &mut Vec<AmshaRequest>, required: &[Amsha]) {
    let mut missing = required
        .iter()
        .copied()
        .filter(|amsha| !requests.iter().any(|request| request.amsha == *amsha))
        .collect::<Vec<_>>();
    missing.sort_by_key(|amsha| amsha.code());
    for amsha in missing {
        requests.push(AmshaRequest::new(amsha));
    }
}

fn resolve_amsha_plan(
    selection: &AmshaSelectionConfig,
    include_shadbala: bool,
    include_vimsopaka: bool,
    include_avastha: bool,
) -> Result<ResolvedAmshaPlan, SearchError> {
    let mut requests = decode_amsha_selection(selection)?;
    if include_shadbala {
        append_required_amshas(&mut requests, &SHADBALA_REQUIRED_AMSHAS);
    }
    if include_vimsopaka {
        append_required_amshas(&mut requests, &VIMSOPAKA_REQUIRED_AMSHAS);
    }
    if include_avastha {
        append_required_amshas(&mut requests, &AVASTHA_REQUIRED_AMSHAS);
    }
    validate_amsha_requests(&requests)?;
    Ok(ResolvedAmshaPlan { requests })
}

fn unique_amsha_requests_for_compute(requests: &[AmshaRequest]) -> (Vec<AmshaRequest>, Vec<usize>) {
    let mut unique = Vec::with_capacity(requests.len());
    let mut positions = Vec::with_capacity(requests.len());
    for request in requests {
        if let Some(index) = unique.iter().position(|candidate: &AmshaRequest| {
            candidate.amsha == request.amsha
                && candidate.effective_variation() == request.effective_variation()
        }) {
            positions.push(index);
        } else {
            unique.push(*request);
            positions.push(unique.len() - 1);
        }
    }
    (unique, positions)
}

/// Build an AmshaChart for one amsha request given pre-computed D1 longitudes.
#[allow(clippy::too_many_arguments)]
fn build_amsha_chart(
    req: &AmshaRequest,
    graha_lons: &[f64; 9],
    graha_cached: Option<&CachedAmshaGrahaData>,
    lagna_sid: f64,
    scope: &AmshaChartScope,
    bhava_cusps_sid: Option<&[f64; 12]>,
    rashi_bhava_cusps_sid: Option<&[f64; 12]>,
    arudha_lons: Option<&[f64; 12]>,
    rashi_bhava_arudha_lons: Option<&[f64; 12]>,
    upagraha_lons: Option<&[f64; 11]>,
    sphuta_lons: Option<&[f64; 16]>,
    special_lagna_lons: Option<&[f64; 8]>,
) -> AmshaChart {
    let amsha = req.amsha;
    let variation = req.variation;
    let effective_variation = req.effective_variation();

    let mut grahas = [make_amsha_entry(0.0); 9];
    if let Some(cached) = graha_cached {
        for i in 0..9 {
            grahas[i] = make_amsha_entry(cached.longitudes[i]);
        }
    } else {
        for i in 0..9 {
            grahas[i] = transform_to_amsha_entry(graha_lons[i], amsha, variation);
        }
    }

    let lagna = transform_to_amsha_entry(lagna_sid, amsha, variation);

    let bhava_cusps = if scope.include_bhava_cusps {
        bhava_cusps_sid.map(|cusps| {
            let mut entries = [make_amsha_entry(0.0); 12];
            for i in 0..12 {
                entries[i] = transform_to_amsha_entry(cusps[i], amsha, variation);
            }
            entries
        })
    } else {
        None
    };
    let rashi_bhava_cusps = if scope.include_bhava_cusps {
        rashi_bhava_cusps_sid.map(|cusps| {
            let mut entries = [make_amsha_entry(0.0); 12];
            for i in 0..12 {
                entries[i] = transform_to_amsha_entry(cusps[i], amsha, variation);
            }
            entries
        })
    } else {
        None
    };

    let arudha_padas = if scope.include_arudha_padas {
        arudha_lons.map(|lons| {
            let mut entries = [make_amsha_entry(0.0); 12];
            for i in 0..12 {
                entries[i] = transform_to_amsha_entry(lons[i], amsha, variation);
            }
            entries
        })
    } else {
        None
    };
    let rashi_bhava_arudha_padas = if scope.include_arudha_padas {
        rashi_bhava_arudha_lons.map(|lons| {
            let mut entries = [make_amsha_entry(0.0); 12];
            for i in 0..12 {
                entries[i] = transform_to_amsha_entry(lons[i], amsha, variation);
            }
            entries
        })
    } else {
        None
    };

    let upagrahas = if scope.include_upagrahas {
        upagraha_lons.map(|lons| {
            let mut entries = [make_amsha_entry(0.0); 11];
            for i in 0..11 {
                entries[i] = transform_to_amsha_entry(lons[i], amsha, variation);
            }
            entries
        })
    } else {
        None
    };

    let sphutas = if scope.include_sphutas {
        sphuta_lons.map(|lons| {
            let mut entries = [make_amsha_entry(0.0); 16];
            for i in 0..16 {
                entries[i] = transform_to_amsha_entry(lons[i], amsha, variation);
            }
            entries
        })
    } else {
        None
    };

    let special_lagnas = if scope.include_special_lagnas {
        special_lagna_lons.map(|lons| {
            let mut entries = [make_amsha_entry(0.0); 8];
            for i in 0..8 {
                entries[i] = transform_to_amsha_entry(lons[i], amsha, variation);
            }
            entries
        })
    } else {
        None
    };

    AmshaChart {
        amsha,
        variation_code: effective_variation,
        grahas,
        lagna,
        bhava_cusps,
        rashi_bhava_cusps,
        arudha_padas,
        rashi_bhava_arudha_padas,
        upagrahas,
        sphutas,
        special_lagnas,
    }
}

/// Compute amsha charts for all entities at a given date.
#[allow(clippy::too_many_arguments)]
pub fn amsha_charts_for_date(
    engine: &Engine,
    eop: &EopKernel,
    utc: &UtcTime,
    location: &GeoLocation,
    bhava_config: &BhavaConfig,
    riseset_config: &RiseSetConfig,
    aya_config: &SankrantiConfig,
    requests: &[AmshaRequest],
    scope: &AmshaChartScope,
) -> Result<AmshaResult, SearchError> {
    validate_amsha_requests(requests)?;
    let mut ctx = JyotishContext::new(engine, Some(eop), utc, aya_config);
    let (unique_requests, unique_positions) = unique_amsha_requests_for_compute(requests);
    ctx.prime_amsha_graha_data(engine, aya_config, &unique_requests)?;

    // Get D1 graha longitudes
    let graha_lons = *ctx.graha_lons(engine, aya_config)?;
    let lagna_sid = ctx.lagna_sid(engine, eop, location)?;

    // Bhava cusps (sidereal)
    let bhava_cusps_sid = if scope.include_bhava_cusps || scope.include_arudha_padas {
        Some(ctx.sidereal_bhava_cusps(engine, eop, location, bhava_config)?)
    } else {
        None
    };
    let rashi_bhava_cusps_sid = if bhava_config.include_rashi_bhava_results
        && (scope.include_bhava_cusps || scope.include_arudha_padas)
    {
        Some(ctx.rashi_bhava_cusps(engine, eop, location)?)
    } else {
        None
    };

    // Arudha padas
    let arudha_lons = if scope.include_arudha_padas {
        let cusp_sid = bhava_cusps_sid.expect("arudha requires sidereal cusp cache");
        let mut lord_lons = [0.0f64; 12];
        for i in 0..12 {
            let cusp_rashi_idx = (cusp_sid[i] / 30.0) as u8;
            let lord = rashi_lord_by_index(cusp_rashi_idx).unwrap_or(Graha::Surya);
            lord_lons[i] = graha_lons.longitude(lord);
        }
        let raw = all_arudha_padas(&cusp_sid, &lord_lons);
        let mut lons = [0.0f64; 12];
        for (i, lon) in lons.iter_mut().enumerate() {
            *lon = raw[i].longitude_deg;
        }
        Some(lons)
    } else {
        None
    };
    let rashi_bhava_arudha_lons =
        if scope.include_arudha_padas && bhava_config.include_rashi_bhava_results {
            let cusp_sid = rashi_bhava_cusps_sid.expect("rashi arudha requires rashi cusp cache");
            let mut lord_lons = [0.0f64; 12];
            for i in 0..12 {
                let cusp_rashi_idx = (cusp_sid[i] / 30.0).floor().min(11.0) as u8;
                let lord = rashi_lord_by_index(cusp_rashi_idx).unwrap_or(Graha::Surya);
                lord_lons[i] = graha_lons.longitude(lord);
            }
            let raw = all_arudha_padas(&cusp_sid, &lord_lons);
            let mut lons = [0.0f64; 12];
            for (i, lon) in lons.iter_mut().enumerate() {
                *lon = raw[i].longitude_deg;
            }
            Some(lons)
        } else {
            None
        };

    // Upagrahas
    let upagraha_lons = if scope.include_upagrahas {
        let upa = all_upagrahas_for_date_with_ctx(
            engine,
            eop,
            utc,
            location,
            riseset_config,
            aya_config,
            &TimeUpagrahaConfig::default(),
            &mut ctx,
        )?;
        Some(all_upagraha_lons(&upa))
    } else {
        None
    };

    // Sphutas
    let sphuta_lons = if scope.include_sphutas {
        Some(all_sphuta_lons_with_ctx(
            engine,
            eop,
            utc,
            location,
            riseset_config,
            aya_config,
            &TimeUpagrahaConfig::default(),
            &mut ctx,
        )?)
    } else {
        None
    };

    // Special lagnas
    let special_lagna_lons = if scope.include_special_lagnas {
        let sl = special_lagnas_for_date_with_ctx(
            engine,
            eop,
            utc,
            location,
            riseset_config,
            aya_config,
            &mut ctx,
        )?;
        Some(all_special_lagna_lons(&sl))
    } else {
        None
    };

    let unique_charts = unique_requests
        .iter()
        .map(|req| {
            let graha_cached = ctx.cached_amsha_graha_data(*req);
            build_amsha_chart(
                req,
                &graha_lons.longitudes,
                graha_cached,
                lagna_sid,
                scope,
                bhava_cusps_sid.as_ref(),
                rashi_bhava_cusps_sid.as_ref(),
                arudha_lons.as_ref(),
                rashi_bhava_arudha_lons.as_ref(),
                upagraha_lons.as_ref(),
                sphuta_lons.as_ref(),
                special_lagna_lons.as_ref(),
            )
        })
        .collect::<Vec<_>>();
    let charts = unique_positions
        .into_iter()
        .map(|index| unique_charts[index].clone())
        .collect();

    Ok(AmshaResult { charts })
}

/// Pure-math transform: compute amsha charts from pre-computed D1 kundali data.
///
/// Requires graha_positions (with lagna) to be present.
/// If scope.include_bhava_cusps is true, bhava_cusps_sid must be Some.
pub fn amsha_charts_from_kundali(
    kundali: &FullKundaliResult,
    bhava_cusps_sid: Option<&[f64; 12]>,
    requests: &[AmshaRequest],
    scope: &AmshaChartScope,
) -> Result<AmshaResult, SearchError> {
    validate_amsha_requests(requests)?;
    let (unique_requests, unique_positions) = unique_amsha_requests_for_compute(requests);

    let gp = kundali
        .graha_positions
        .as_ref()
        .ok_or(SearchError::InvalidConfig(
            "graha_positions required for amsha charts",
        ))?;

    let lagna_sid = gp.lagna.sidereal_longitude;

    if scope.include_bhava_cusps && bhava_cusps_sid.is_none() {
        return Err(SearchError::InvalidConfig(
            "bhava_cusps_sid required when include_bhava_cusps is true",
        ));
    }
    let rashi_bhava_cusps_sid = if scope.include_bhava_cusps {
        kundali.rashi_bhava_cusps.as_ref().map(|result| {
            let mut cusps = [0.0f64; 12];
            for (i, cusp) in cusps.iter_mut().enumerate() {
                *cusp = result.bhavas[i].cusp_deg;
            }
            cusps
        })
    } else {
        None
    };

    // Extract arudha padas if available
    let arudha_lons = if scope.include_arudha_padas {
        kundali.bindus.as_ref().map(|b| {
            let mut lons = [0.0f64; 12];
            for (i, lon) in lons.iter_mut().enumerate() {
                *lon = b.arudha_padas[i].sidereal_longitude;
            }
            lons
        })
    } else {
        None
    };
    let rashi_bhava_arudha_lons = if scope.include_arudha_padas {
        kundali.bindus.as_ref().and_then(|b| {
            b.rashi_bhava_arudha_padas.map(|entries| {
                let mut lons = [0.0f64; 12];
                for (i, lon) in lons.iter_mut().enumerate() {
                    *lon = entries[i].sidereal_longitude;
                }
                lons
            })
        })
    } else {
        None
    };

    // Extract upagrahas if available
    let upagraha_lons = if scope.include_upagrahas {
        kundali.upagrahas.as_ref().map(all_upagraha_lons)
    } else {
        None
    };

    // Extract special lagnas if available
    let special_lagna_lons = if scope.include_special_lagnas {
        kundali.special_lagnas.as_ref().map(all_special_lagna_lons)
    } else {
        None
    };

    let sphuta_lons = if scope.include_sphutas {
        kundali.sphutas.as_ref().map(|s| s.longitudes)
    } else {
        None
    };

    let unique_charts = unique_requests
        .iter()
        .map(|req| {
            build_amsha_chart(
                req,
                &gp.grahas.map(|g| g.sidereal_longitude),
                None,
                lagna_sid,
                scope,
                bhava_cusps_sid,
                rashi_bhava_cusps_sid.as_ref(),
                arudha_lons.as_ref(),
                rashi_bhava_arudha_lons.as_ref(),
                upagraha_lons.as_ref(),
                sphuta_lons.as_ref(),
                special_lagna_lons.as_ref(),
            )
        })
        .collect::<Vec<_>>();
    let charts = unique_positions
        .into_iter()
        .map(|index| unique_charts[index].clone())
        .collect();

    Ok(AmshaResult { charts })
}

/// Internal: compute amsha charts from within full_kundali_for_date.
fn amsha_charts_from_kundali_with_ctx(
    requests: &[AmshaRequest],
    scope: &AmshaChartScope,
    graha_positions: &Option<GrahaPositions>,
    rashi_bhava_cusps: &Option<BhavaResult>,
    bindus: &Option<BindusResult>,
    upagrahas: &Option<AllUpagrahas>,
    sphutas: &Option<SphutalResult>,
    special_lagnas: &Option<AllSpecialLagnas>,
    ctx: &mut JyotishContext,
) -> Result<AmshaResult, SearchError> {
    validate_amsha_requests(&requests)?;
    let (unique_requests, unique_positions) = unique_amsha_requests_for_compute(requests);

    let gp = graha_positions.as_ref().ok_or(SearchError::InvalidConfig(
        "graha_positions required for amsha charts",
    ))?;

    let lagna_sid = gp.lagna.sidereal_longitude;

    // Bhava cusps (sidereal) from context
    let bhava_cusps_sid = if scope.include_bhava_cusps {
        if let Some(ref bhava_result) = ctx.bhava_result {
            let aya = ctx.ayanamsha;
            let plane = ctx.reference_plane;
            let mut cusps = [0.0f64; 12];
            for (i, cusp) in cusps.iter_mut().enumerate() {
                *cusp = ecliptic_to_sidereal(bhava_result.bhavas[i].cusp_deg, aya, plane);
            }
            Some(cusps)
        } else {
            None
        }
    } else {
        None
    };
    let rashi_bhava_cusps_sid = if scope.include_bhava_cusps {
        rashi_bhava_cusps.as_ref().map(|result| {
            let mut cusps = [0.0f64; 12];
            for (i, cusp) in cusps.iter_mut().enumerate() {
                *cusp = result.bhavas[i].cusp_deg;
            }
            cusps
        })
    } else {
        None
    };

    let arudha_lons = if scope.include_arudha_padas {
        bindus.as_ref().map(|b| {
            let mut lons = [0.0f64; 12];
            for (i, lon) in lons.iter_mut().enumerate() {
                *lon = b.arudha_padas[i].sidereal_longitude;
            }
            lons
        })
    } else {
        None
    };
    let rashi_bhava_arudha_lons = if scope.include_arudha_padas {
        bindus.as_ref().and_then(|b| {
            b.rashi_bhava_arudha_padas.map(|entries| {
                let mut lons = [0.0f64; 12];
                for (i, lon) in lons.iter_mut().enumerate() {
                    *lon = entries[i].sidereal_longitude;
                }
                lons
            })
        })
    } else {
        None
    };

    let upagraha_lons = if scope.include_upagrahas {
        upagrahas.as_ref().map(all_upagraha_lons)
    } else {
        None
    };

    let special_lagna_lons = if scope.include_special_lagnas {
        special_lagnas.as_ref().map(all_special_lagna_lons)
    } else {
        None
    };

    let sphuta_lons = if scope.include_sphutas {
        sphutas.as_ref().map(|s| s.longitudes)
    } else {
        None
    };

    let unique_charts = unique_requests
        .iter()
        .map(|req| {
            let graha_cached = ctx.cached_amsha_graha_data(*req);
            build_amsha_chart(
                req,
                &gp.grahas.map(|g| g.sidereal_longitude),
                graha_cached,
                lagna_sid,
                scope,
                bhava_cusps_sid.as_ref(),
                rashi_bhava_cusps_sid.as_ref(),
                arudha_lons.as_ref(),
                rashi_bhava_arudha_lons.as_ref(),
                upagraha_lons.as_ref(),
                sphuta_lons.as_ref(),
                special_lagna_lons.as_ref(),
            )
        })
        .collect::<Vec<_>>();
    let charts = unique_positions
        .into_iter()
        .map(|index| unique_charts[index].clone())
        .collect();

    Ok(AmshaResult { charts })
}

/// Extract all 11 upagraha longitudes into a fixed array.
fn all_upagraha_lons(u: &AllUpagrahas) -> [f64; 11] {
    [
        u.gulika,
        u.maandi,
        u.kaala,
        u.mrityu,
        u.artha_prahara,
        u.yama_ghantaka,
        u.dhooma,
        u.vyatipata,
        u.parivesha,
        u.indra_chapa,
        u.upaketu,
    ]
}

/// Extract all 8 special lagna longitudes into a fixed array.
fn all_special_lagna_lons(s: &AllSpecialLagnas) -> [f64; 8] {
    [
        s.bhava_lagna,
        s.hora_lagna,
        s.ghati_lagna,
        s.vighati_lagna,
        s.varnada_lagna,
        s.sree_lagna,
        s.pranapada_lagna,
        s.indu_lagna,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use dhruv_vedic_base::DEFAULT_AMSHA_VARIATION_CODE;

    #[test]
    fn graha_to_body_mapping() {
        assert_eq!(graha_to_body(Graha::Surya), Some(Body::Sun));
        assert_eq!(graha_to_body(Graha::Chandra), Some(Body::Moon));
        assert_eq!(graha_to_body(Graha::Mangal), Some(Body::Mars));
        assert_eq!(graha_to_body(Graha::Buddh), Some(Body::Mercury));
        assert_eq!(graha_to_body(Graha::Guru), Some(Body::Jupiter));
        assert_eq!(graha_to_body(Graha::Shukra), Some(Body::Venus));
        assert_eq!(graha_to_body(Graha::Shani), Some(Body::Saturn));
        assert_eq!(graha_to_body(Graha::Rahu), None);
        assert_eq!(graha_to_body(Graha::Ketu), None);
    }

    #[test]
    fn decode_amsha_selection_dedupes_exact_duplicates() {
        let mut selection = AmshaSelectionConfig {
            count: 2,
            ..AmshaSelectionConfig::default()
        };
        selection.codes[0] = Amsha::D9.code();
        selection.codes[1] = Amsha::D9.code();

        let decoded = decode_amsha_selection(&selection).expect("selection should decode");
        assert_eq!(decoded.len(), 1);
        assert_eq!(decoded[0].amsha, Amsha::D9);
        assert_eq!(
            decoded[0].effective_variation(),
            DEFAULT_AMSHA_VARIATION_CODE
        );
    }

    #[test]
    fn decode_amsha_selection_rejects_conflicting_duplicates() {
        let mut selection = AmshaSelectionConfig {
            count: 2,
            ..AmshaSelectionConfig::default()
        };
        selection.codes[0] = Amsha::D2.code();
        selection.variations[0] = 0;
        selection.codes[1] = Amsha::D2.code();
        selection.variations[1] = 1;

        let error = decode_amsha_selection(&selection).expect_err("conflicting selection");
        assert!(
            matches!(error, SearchError::InvalidConfig(message) if message.contains("conflicting"))
        );
    }

    #[test]
    fn resolve_amsha_plan_uses_explicit_variations_and_appends_required_union() {
        let mut selection = AmshaSelectionConfig {
            count: 2,
            ..AmshaSelectionConfig::default()
        };
        selection.codes[0] = Amsha::D2.code();
        selection.variations[0] = 1;
        selection.codes[1] = Amsha::D9.code();

        let plan = resolve_amsha_plan(&selection, true, true, true).expect("plan should resolve");
        assert_eq!(plan.requests()[0].amsha, Amsha::D2);
        assert_eq!(plan.requests()[0].effective_variation(), 1);
        assert_eq!(plan.requests()[1].amsha, Amsha::D9);
        assert_eq!(plan.requests().len(), 16);
        assert!(
            plan.requests()
                .iter()
                .any(|request| request.amsha == Amsha::D60)
        );
    }

    #[test]
    fn resolve_amsha_plan_defaults_required_variations_when_not_selected() {
        let selection = AmshaSelectionConfig::default();
        let plan = resolve_amsha_plan(&selection, true, false, false).expect("plan should resolve");

        assert_eq!(plan.requests().len(), SHADBALA_REQUIRED_AMSHAS.len());
        assert_eq!(
            plan.request_for(Amsha::D2).effective_variation(),
            DEFAULT_AMSHA_VARIATION_CODE
        );
    }

    #[test]
    fn amsha_graha_cache_computes_once_per_request() {
        let request = AmshaRequest::with_variation(Amsha::D2, 1);
        let mut cache = AmshaGrahaCache::default();
        let mut compute_calls = 0usize;

        let first = cache
            .get_or_compute(request, || {
                compute_calls += 1;
                Ok(CachedAmshaGrahaData {
                    amsha: Amsha::D2,
                    variation_code: 1,
                    longitudes: [0.0; 9],
                    rashi_indices: [0; 9],
                    division_indices: [0; 9],
                })
            })
            .expect("first cache fill should succeed");
        assert_eq!(first.amsha, Amsha::D2);
        assert_eq!(cache.len(), 1);
        assert_eq!(compute_calls, 1);

        let second = cache
            .get_or_compute(request, || {
                compute_calls += 1;
                Err(SearchError::InvalidConfig("cache should not recompute"))
            })
            .expect("cached lookup should succeed");
        assert_eq!(second.amsha, Amsha::D2);
        assert_eq!(cache.len(), 1);
        assert_eq!(compute_calls, 1);
    }

    #[test]
    fn amsha_graha_cache_keys_by_variation() {
        let mut cache = AmshaGrahaCache::default();

        cache
            .get_or_compute(AmshaRequest::new(Amsha::D2), || {
                Ok(CachedAmshaGrahaData {
                    amsha: Amsha::D2,
                    variation_code: DEFAULT_AMSHA_VARIATION_CODE,
                    longitudes: [0.0; 9],
                    rashi_indices: [0; 9],
                    division_indices: [0; 9],
                })
            })
            .expect("default variation should cache");

        cache
            .get_or_compute(AmshaRequest::with_variation(Amsha::D2, 1), || {
                Ok(CachedAmshaGrahaData {
                    amsha: Amsha::D2,
                    variation_code: 1,
                    longitudes: [0.0; 9],
                    rashi_indices: [0; 9],
                    division_indices: [0; 9],
                })
            })
            .expect("alternate variation should cache independently");

        assert_eq!(cache.len(), 2);
    }

    #[test]
    fn unique_amsha_requests_dedupe_by_effective_variation_and_preserve_positions() {
        let requests = [
            AmshaRequest::new(Amsha::D9),
            AmshaRequest::new(Amsha::D9),
            AmshaRequest::new(Amsha::D2),
            AmshaRequest::with_variation(Amsha::D2, 1),
            AmshaRequest::with_variation(Amsha::D2, 1),
        ];

        let (unique, positions) = unique_amsha_requests_for_compute(&requests);

        assert_eq!(unique.len(), 3);
        assert_eq!(positions, vec![0, 0, 1, 2, 2]);
        assert_eq!(unique[0].amsha, Amsha::D9);
        assert_eq!(
            unique[1].effective_variation(),
            DEFAULT_AMSHA_VARIATION_CODE
        );
        assert_eq!(unique[2].effective_variation(), 1);
    }

    #[test]
    fn upagraha_cache_len_tracks_unique_configs() {
        let mut cache = UpagrahaCache::default();
        cache.entries.push(CachedUpagrahaData {
            config: TimeUpagrahaConfig::default(),
            upagrahas: AllUpagrahas {
                gulika: 0.0,
                maandi: 0.0,
                kaala: 0.0,
                mrityu: 0.0,
                artha_prahara: 0.0,
                yama_ghantaka: 0.0,
                dhooma: 0.0,
                vyatipata: 0.0,
                parivesha: 0.0,
                indra_chapa: 0.0,
                upaketu: 0.0,
            },
        });
        cache.entries.push(CachedUpagrahaData {
            config: TimeUpagrahaConfig {
                gulika_point: dhruv_vedic_base::TimeUpagrahaPoint::End,
                ..TimeUpagrahaConfig::default()
            },
            upagrahas: AllUpagrahas {
                gulika: 1.0,
                maandi: 1.0,
                kaala: 1.0,
                mrityu: 1.0,
                artha_prahara: 1.0,
                yama_ghantaka: 1.0,
                dhooma: 1.0,
                vyatipata: 1.0,
                parivesha: 1.0,
                indra_chapa: 1.0,
                upaketu: 1.0,
            },
        });

        assert_eq!(cache.len(), 2);
    }

    #[test]
    fn vimsopaka_varga_cache_len_tracks_variation_and_policy() {
        let mut cache = VimsopakaVargaCache::default();
        cache.entries.push(CachedVimsopakaVargaData {
            request: AmshaRequest::new(Amsha::D2),
            node_policy: NodeDignityPolicy::SignLordBased,
            dignities: [Dignity::Sama; 9],
            points: [0.0; 9],
        });
        cache.entries.push(CachedVimsopakaVargaData {
            request: AmshaRequest::with_variation(Amsha::D2, 1),
            node_policy: NodeDignityPolicy::SignLordBased,
            dignities: [Dignity::Sama; 9],
            points: [1.0; 9],
        });
        cache.entries.push(CachedVimsopakaVargaData {
            request: AmshaRequest::new(Amsha::D2),
            node_policy: NodeDignityPolicy::AlwaysSama,
            dignities: [Dignity::Sama; 9],
            points: [2.0; 9],
        });

        assert_eq!(cache.len(), 3);
    }

    #[test]
    fn rashi_bhava_basis_cusps_follow_lagna_degree_and_rashi_sequence() {
        let result = rashi_bhava_result_from_lagna(48.25);

        assert_eq!(result.bhavas[0].number, 1);
        assert!((result.bhavas[0].cusp_deg - 48.25).abs() < 1e-12);
        assert!((result.bhavas[1].cusp_deg - 78.25).abs() < 1e-12);
        assert!((result.bhavas[11].cusp_deg - 18.25).abs() < 1e-12);
        assert!((result.mc_deg - result.bhavas[9].cusp_deg).abs() < 1e-12);
    }

    #[test]
    fn rashi_bhava_numbers_are_whole_sign_offsets_from_lagna() {
        let lagna = 48.25;

        assert_eq!(rashi_bhava_number_from_lagna(lagna, 49.0), 1);
        assert_eq!(rashi_bhava_number_from_lagna(lagna, 78.0), 2);
        assert_eq!(rashi_bhava_number_from_lagna(lagna, 17.0), 12);
    }
}
