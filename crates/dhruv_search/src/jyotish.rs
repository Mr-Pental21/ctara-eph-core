//! Vedic jyotish orchestration: queries engine for graha positions.
//!
//! Provides the bridge between the ephemeris engine and the pure-math
//! Vedic calculation modules. Queries all 9 graha positions at a given
//! epoch and converts to sidereal longitudes.

use dhruv_core::{Body, Engine};
use dhruv_time::{EopKernel, UtcTime};
use dhruv_vedic_base::arudha::all_arudha_padas;
use dhruv_vedic_base::riseset::compute_rise_set;
use dhruv_vedic_base::riseset_types::{GeoLocation, RiseSetConfig, RiseSetEvent, RiseSetResult};
use dhruv_vedic_base::special_lagna::all_special_lagnas;
use dhruv_vedic_base::upagraha::TIME_BASED_UPAGRAHAS;
use dhruv_vedic_base::vaar::vaar_from_jd;
use dhruv_vedic_base::{
    ALL_GRAHAS, Amsha, AmshaRequest, AmshaVariation, AllSpecialLagnas, AllUpagrahas, ArudhaResult,
    AshtakavargaResult, AyanamshaSystem, BhavaConfig, BhavaResult, DrishtiEntry, Graha, LunarNode,
    NodeMode, Upagraha, amsha_longitude, ayanamsha_deg, bhrigu_bindu,
    calculate_ashtakavarga, compute_bhavas, ghati_lagna, ghatikas_since_sunrise,
    graha_drishti, graha_drishti_matrix, hora_lagna, jd_tdb_to_centuries, lagna_longitude_rad,
    lunar_node_deg, nakshatra_from_longitude, normalize_360, nth_rashi_from, pranapada_lagna,
    rashi_from_longitude, rashi_lord_by_index, sree_lagna, sun_based_upagrahas, time_upagraha_jd,
};

use crate::conjunction::body_ecliptic_lon_lat;
use crate::error::SearchError;
use crate::jyotish_types::{
    AmshaChart, AmshaChartScope, AmshaEntry, AmshaResult, AmshaSelectionConfig, BindusConfig,
    BindusResult, DrishtiConfig, DrishtiResult, FullKundaliConfig, FullKundaliResult, GrahaEntry,
    GrahaLongitudes, GrahaPositions, GrahaPositionsConfig, MAX_AMSHA_REQUESTS,
};
use crate::panchang::vedic_day_sunrises;
use crate::sankranti_types::SankrantiConfig;

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

/// One-shot, function-local cache for shared intermediates.
///
/// This is intentionally not exposed and not persisted across calls.
#[derive(Debug, Clone)]
struct JyotishContext {
    jd_tdb: f64,
    jd_utc: f64,
    ayanamsha: f64,
    graha_lons: Option<GrahaLongitudes>,
    lagna_sid: Option<f64>,
    bhava_result: Option<BhavaResult>,
    sunrise_pair: Option<(f64, f64)>,
    sunset_jd: Option<f64>,
}

impl JyotishContext {
    fn new(engine: &Engine, utc: &UtcTime, aya_config: &SankrantiConfig) -> Self {
        let jd_tdb = utc.to_jd_tdb(engine.lsk());
        let jd_utc = utc_to_jd_utc(utc);
        let t = jd_tdb_to_centuries(jd_tdb);
        let ayanamsha = ayanamsha_deg(aya_config.ayanamsha_system, t, aya_config.use_nutation);
        Self {
            jd_tdb,
            jd_utc,
            ayanamsha,
            graha_lons: None,
            lagna_sid: None,
            bhava_result: None,
            sunrise_pair: None,
            sunset_jd: None,
        }
    }

    fn graha_lons<'a>(
        &'a mut self,
        engine: &Engine,
        aya_config: &SankrantiConfig,
    ) -> Result<&'a GrahaLongitudes, SearchError> {
        if self.graha_lons.is_none() {
            let lons = graha_sidereal_longitudes(
                engine,
                self.jd_tdb,
                aya_config.ayanamsha_system,
                aya_config.use_nutation,
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
        let lagna_sid = normalize(lagna_rad.to_degrees() - self.ayanamsha);
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
            self.jd_utc.floor() + 0.5,
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
}

/// Query all 9 graha sidereal longitudes at a given TDB epoch.
///
/// For the 7 physical planets, queries the engine for tropical ecliptic
/// longitude and subtracts ayanamsha. For Rahu/Ketu, uses the mean/true
/// node mathematical formulas.
pub fn graha_sidereal_longitudes(
    engine: &Engine,
    jd_tdb: f64,
    system: AyanamshaSystem,
    use_nutation: bool,
) -> Result<GrahaLongitudes, SearchError> {
    let t = jd_tdb_to_centuries(jd_tdb);
    let aya = ayanamsha_deg(system, t, use_nutation);

    let mut longitudes = [0.0f64; 9];

    for graha in ALL_GRAHAS {
        let idx = graha.index() as usize;
        match graha {
            Graha::Rahu => {
                let rahu_tropical = lunar_node_deg(LunarNode::Rahu, t, NodeMode::True);
                longitudes[idx] = normalize(rahu_tropical - aya);
            }
            Graha::Ketu => {
                let ketu_tropical = lunar_node_deg(LunarNode::Ketu, t, NodeMode::True);
                longitudes[idx] = normalize(ketu_tropical - aya);
            }
            _ => {
                let body = graha_to_body(graha).expect("sapta graha has body");
                let (lon_tropical, _lat) = body_ecliptic_lon_lat(engine, body, jd_tdb)?;
                longitudes[idx] = normalize(lon_tropical - aya);
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
    let mut ctx = JyotishContext::new(engine, utc, aya_config);
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
    let mut ctx = JyotishContext::new(engine, utc, aya_config);
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
    let bhava_result = ctx.bhava_result(engine, eop, location, bhava_config)?;

    let mut cusp_sid = [0.0f64; 12];
    for i in 0..12 {
        cusp_sid[i] = normalize(bhava_result.bhavas[i].cusp_deg - aya);
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
    let mut ctx = JyotishContext::new(engine, utc, aya_config);
    all_upagrahas_for_date_with_ctx(
        engine,
        eop,
        utc,
        location,
        riseset_config,
        aya_config,
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
    ctx: &mut JyotishContext,
) -> Result<AllUpagrahas, SearchError> {
    let jd_tdb = ctx.jd_tdb;
    let aya = ctx.ayanamsha;

    let (jd_sunrise, jd_next_sunrise) =
        ctx.sunrise_pair(engine, eop, utc, location, riseset_config)?;
    let jd_sunset = ctx.sunset_jd(engine, eop, location, riseset_config)?;

    // Determine if birth time is during day (sunrise to sunset) or night.
    let is_day = jd_tdb >= jd_sunrise && jd_tdb < jd_sunset;
    let weekday = vaar_from_jd(jd_sunrise).index();

    // Compute time-based upagrahas (lagna at portion start/end).
    let mut time_lons = [0.0f64; 6]; // Gulika, Maandi, Kaala, Mrityu, ArthaPrahara, YamaGhantaka
    for (i, &upa) in TIME_BASED_UPAGRAHAS.iter().enumerate() {
        let target_jd =
            time_upagraha_jd(upa, weekday, is_day, jd_sunrise, jd_sunset, jd_next_sunrise);
        let lagna_rad = lagna_longitude_rad(engine.lsk(), eop, location, target_jd)?;
        let lagna_tropical = lagna_rad.to_degrees();
        time_lons[i] = normalize_360(lagna_tropical - aya);
    }

    // Compute sun-based upagrahas from sidereal Sun longitude.
    let sun_sid = ctx.graha_lons(engine, aya_config)?.longitude(Graha::Surya);
    let sun_up = sun_based_upagrahas(sun_sid);

    Ok(AllUpagrahas {
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
    })
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
    let mut ctx = JyotishContext::new(engine, utc, aya_config);
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
    let graha_lons = *ctx.graha_lons(engine, aya_config)?;

    let lagna_sid = if config.include_lagna {
        Some(ctx.lagna_sid(engine, eop, location)?)
    } else {
        None
    };

    let bhava_result = if config.include_bhava {
        Some(ctx.bhava_result(engine, eop, location, bhava_config)?)
    } else {
        None
    };

    // Build GrahaEntry for each of the 9 grahas.
    let mut grahas = [GrahaEntry::sentinel(); 9];
    for graha in ALL_GRAHAS {
        let idx = graha.index() as usize;
        let sid_lon = graha_lons.longitude(graha);
        grahas[idx] = make_graha_entry(sid_lon, config, bhava_result, aya);
    }

    let lagna = if config.include_lagna {
        make_graha_entry(
            lagna_sid.expect("include_lagna implies lagna_sid"),
            config,
            bhava_result,
            aya,
        )
    } else {
        GrahaEntry::sentinel()
    };

    let outer_planets = if config.include_outer_planets {
        let outer_bodies = [Body::Uranus, Body::Neptune, Body::Pluto];
        let mut entries = [GrahaEntry::sentinel(); 3];
        for (i, &body) in outer_bodies.iter().enumerate() {
            let (lon_tropical, _lat) = body_ecliptic_lon_lat(engine, body, jd_tdb)?;
            let sid_lon = normalize(lon_tropical - aya);
            entries[i] = make_graha_entry(sid_lon, config, bhava_result, aya);
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
    aya: f64,
) -> GrahaEntry {
    let rashi_info = rashi_from_longitude(sid_lon);
    let (nakshatra, nakshatra_index, pada) = if config.include_nakshatra {
        let nak = nakshatra_from_longitude(sid_lon);
        (nak.nakshatra, nak.nakshatra_index, nak.pada)
    } else {
        (dhruv_vedic_base::Nakshatra::Ashwini, 255, 0)
    };
    let bhava_number = if let Some(result) = bhava_result {
        // Determine which bhava this longitude falls in (use tropical for matching bhava cusps)
        let tropical_lon = normalize(sid_lon + aya);
        find_bhava_number(tropical_lon, result)
    } else {
        0
    };
    GrahaEntry {
        sidereal_longitude: sid_lon,
        rashi: rashi_info.rashi,
        rashi_index: rashi_info.rashi_index,
        nakshatra,
        nakshatra_index,
        pada,
        bhava_number,
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
    let mut ctx = JyotishContext::new(engine, utc, aya_config);
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
    let mut ctx = JyotishContext::new(engine, utc, aya_config);
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
    let jd_sunset = ctx.sunset_jd(engine, eop, location, riseset_config)?;

    let is_day = ctx.jd_tdb >= jd_sunrise && ctx.jd_tdb < jd_sunset;
    let weekday = vaar_from_jd(jd_sunrise).index();

    let gulika_jd = time_upagraha_jd(
        Upagraha::Gulika,
        weekday,
        is_day,
        jd_sunrise,
        jd_sunset,
        jd_next_sunrise,
    );
    let gulika_rad = lagna_longitude_rad(engine.lsk(), eop, location, gulika_jd)?;
    let gulika_sid = normalize_360(gulika_rad.to_degrees() - aya);

    let maandi_jd = time_upagraha_jd(
        Upagraha::Maandi,
        weekday,
        is_day,
        jd_sunrise,
        jd_sunset,
        jd_next_sunrise,
    );
    let maandi_rad = lagna_longitude_rad(engine.lsk(), eop, location, maandi_jd)?;
    let maandi_sid = normalize_360(maandi_rad.to_degrees() - aya);

    let bb_lon = bhrigu_bindu(rahu_sid, moon_sid);
    let pp_lon = pranapada_lagna(sun_sid, ghatikas);
    let hl_lon = hora_lagna(sun_sid, ghatikas);
    let gl_lon = ghati_lagna(sun_sid, ghatikas);
    let sl_lon = sree_lagna(moon_sid, lagna_sid);

    let bhava_result = ctx.bhava_result(engine, eop, location, bhava_config)?;
    let bhava_opt = if config.include_bhava {
        Some(bhava_result)
    } else {
        None
    };

    let mut cusp_sid = [0.0f64; 12];
    for (i, cusp) in cusp_sid.iter_mut().enumerate() {
        *cusp = normalize(bhava_result.bhavas[i].cusp_deg - aya);
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
        arudha_padas[i] = make_graha_entry(arudha_raw[i].longitude_deg, &gp_config, bhava_opt, aya);
    }

    Ok(BindusResult {
        arudha_padas,
        bhrigu_bindu: make_graha_entry(bb_lon, &gp_config, bhava_opt, aya),
        pranapada_lagna: make_graha_entry(pp_lon, &gp_config, bhava_opt, aya),
        gulika: make_graha_entry(gulika_sid, &gp_config, bhava_opt, aya),
        maandi: make_graha_entry(maandi_sid, &gp_config, bhava_opt, aya),
        hora_lagna: make_graha_entry(hl_lon, &gp_config, bhava_opt, aya),
        ghati_lagna: make_graha_entry(gl_lon, &gp_config, bhava_opt, aya),
        sree_lagna: make_graha_entry(sl_lon, &gp_config, bhava_opt, aya),
    })
}

/// Compute graha drishti (planetary aspects) with optional extensions.
///
/// Always computes the 9×9 graha-to-graha matrix. Optionally extends to:
/// - graha-to-bhava-cusp (12 cusps) if `config.include_bhava`
/// - graha-to-lagna if `config.include_lagna`
/// - graha-to-core-bindus (19 points) if `config.include_bindus`
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
    let mut ctx = JyotishContext::new(engine, utc, aya_config);
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
    let graha_lons = *ctx.graha_lons(engine, aya_config)?;
    let graha_to_graha = graha_drishti_matrix(&graha_lons.longitudes);

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
        let bhava_result = ctx.bhava_result(engine, eop, location, bhava_config)?;
        let mut cusp_sid = [0.0f64; 12];
        for (i, cusp) in cusp_sid.iter_mut().enumerate() {
            *cusp = normalize(bhava_result.bhavas[i].cusp_deg - aya);
        }
        let mut entries = [[DrishtiEntry::zero(); 12]; 9];
        for g in ALL_GRAHAS {
            let gi = g.index() as usize;
            for ci in 0..12 {
                entries[gi][ci] = graha_drishti(g, graha_lons.longitudes[gi], cusp_sid[ci]);
            }
        }
        entries
    } else {
        [[DrishtiEntry::zero(); 12]; 9]
    };

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
        graha_to_lagna,
        graha_to_bindus,
    })
}

/// Compute a full kundali in one shot, sharing intermediates across sections.
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
    let mut ctx = JyotishContext::new(engine, utc, aya_config);

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
            &mut ctx,
        )?)
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
            config, &graha_positions, &bindus, &upagrahas, &special_lagnas, &mut ctx,
        )?)
    } else {
        None
    };

    Ok(FullKundaliResult {
        graha_positions,
        bindus,
        drishti,
        ashtakavarga,
        upagrahas,
        special_lagnas,
        amshas,
    })
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

/// Normalize longitude to [0, 360).
fn normalize(deg: f64) -> f64 {
    let r = deg % 360.0;
    if r < 0.0 { r + 360.0 } else { r }
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
fn transform_to_amsha_entry(
    sidereal_lon: f64,
    amsha: Amsha,
    variation: Option<AmshaVariation>,
) -> AmshaEntry {
    let amsha_lon = amsha_longitude(sidereal_lon, amsha, variation);
    make_amsha_entry(amsha_lon)
}

/// Validate an AmshaRequest slice.
fn validate_amsha_requests(requests: &[AmshaRequest]) -> Result<(), SearchError> {
    if requests.len() > MAX_AMSHA_REQUESTS {
        return Err(SearchError::InvalidConfig("amsha count exceeds maximum"));
    }
    for req in requests {
        let v = req.effective_variation();
        if !v.is_applicable_to(req.amsha) {
            return Err(SearchError::InvalidConfig(
                "variation not applicable to amsha",
            ));
        }
    }
    Ok(())
}

/// Convert AmshaSelectionConfig to a Vec of AmshaRequest.
fn selection_to_requests(
    sel: &AmshaSelectionConfig,
) -> Result<Vec<AmshaRequest>, SearchError> {
    if sel.count as usize > MAX_AMSHA_REQUESTS {
        return Err(SearchError::InvalidConfig("amsha count exceeds maximum"));
    }
    let mut requests = Vec::with_capacity(sel.count as usize);
    for i in 0..sel.count as usize {
        let amsha = Amsha::from_code(sel.codes[i]).ok_or_else(|| {
            SearchError::InvalidConfig("unknown amsha code")
        })?;
        let variation = if sel.variations[i] == 0 {
            None
        } else {
            let v = AmshaVariation::from_code(sel.variations[i]).ok_or_else(|| {
                SearchError::InvalidConfig("unknown variation code")
            })?;
            if !v.is_applicable_to(amsha) {
                return Err(SearchError::InvalidConfig(
                    "variation not applicable to amsha",
                ));
            }
            Some(v)
        };
        requests.push(AmshaRequest { amsha, variation });
    }
    Ok(requests)
}

/// Build an AmshaChart for one amsha request given pre-computed D1 longitudes.
fn build_amsha_chart(
    req: &AmshaRequest,
    graha_lons: &[f64; 9],
    lagna_sid: f64,
    scope: &AmshaChartScope,
    bhava_cusps_sid: Option<&[f64; 12]>,
    arudha_lons: Option<&[f64; 12]>,
    upagraha_lons: Option<&[f64; 11]>,
    sphuta_lons: Option<&[f64; 16]>,
    special_lagna_lons: Option<&[f64; 8]>,
) -> AmshaChart {
    let amsha = req.amsha;
    let variation = req.variation;
    let effective_variation = req.effective_variation();

    let mut grahas = [make_amsha_entry(0.0); 9];
    for i in 0..9 {
        grahas[i] = transform_to_amsha_entry(graha_lons[i], amsha, variation);
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
        variation: effective_variation,
        grahas,
        lagna,
        bhava_cusps,
        arudha_padas,
        upagrahas,
        sphutas,
        special_lagnas,
    }
}

/// Compute amsha charts for all entities at a given date.
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
    let mut ctx = JyotishContext::new(engine, utc, aya_config);

    // Get D1 graha longitudes
    let graha_lons = *ctx.graha_lons(engine, aya_config)?;
    let lagna_sid = ctx.lagna_sid(engine, eop, location)?;
    let aya = ctx.ayanamsha;

    // Bhava cusps (sidereal)
    let bhava_cusps_sid = if scope.include_bhava_cusps {
        let bhava_result = ctx.bhava_result(engine, eop, location, bhava_config)?;
        let mut cusps = [0.0f64; 12];
        for i in 0..12 {
            cusps[i] = normalize(bhava_result.bhavas[i].cusp_deg - aya);
        }
        Some(cusps)
    } else {
        None
    };

    // Arudha padas
    let arudha_lons = if scope.include_arudha_padas {
        let bhava_result = ctx.bhava_result(engine, eop, location, bhava_config)?;
        let mut cusp_sid = [0.0f64; 12];
        for i in 0..12 {
            cusp_sid[i] = normalize(bhava_result.bhavas[i].cusp_deg - aya);
        }
        let mut lord_lons = [0.0f64; 12];
        for i in 0..12 {
            let cusp_rashi_idx = (cusp_sid[i] / 30.0) as u8;
            let lord = rashi_lord_by_index(cusp_rashi_idx).unwrap_or(Graha::Surya);
            lord_lons[i] = graha_lons.longitude(lord);
        }
        let raw = all_arudha_padas(&cusp_sid, &lord_lons);
        let mut lons = [0.0f64; 12];
        for i in 0..12 {
            lons[i] = raw[i].longitude_deg;
        }
        Some(lons)
    } else {
        None
    };

    // Upagrahas
    let upagraha_lons = if scope.include_upagrahas {
        let upa = all_upagrahas_for_date_with_ctx(
            engine, eop, utc, location, riseset_config, aya_config, &mut ctx,
        )?;
        Some(all_upagraha_lons(&upa))
    } else {
        None
    };

    // Sphutas
    let sphuta_lons = if scope.include_sphutas {
        let gl = *ctx.graha_lons(engine, aya_config)?;
        let sun_sid = gl.longitude(Graha::Surya);
        let moon_sid = gl.longitude(Graha::Chandra);
        let rahu_sid = gl.longitude(Graha::Rahu);
        let mars_sid = gl.longitude(Graha::Mangal);
        let jupiter_sid = gl.longitude(Graha::Guru);
        let venus_sid = gl.longitude(Graha::Shukra);
        let lagna_sid_v = ctx.lagna_sid(engine, eop, location)?;

        let (jd_sunrise, jd_next_sunrise) =
            ctx.sunrise_pair(engine, eop, utc, location, riseset_config)?;
        let jd_sunset = ctx.sunset_jd(engine, eop, location, riseset_config)?;
        let is_day = ctx.jd_tdb >= jd_sunrise && ctx.jd_tdb < jd_sunset;
        let weekday = vaar_from_jd(jd_sunrise).index();

        let gulika_jd = time_upagraha_jd(
            Upagraha::Gulika, weekday, is_day, jd_sunrise, jd_sunset, jd_next_sunrise,
        );
        let gulika_rad = lagna_longitude_rad(engine.lsk(), eop, location, gulika_jd)?;
        let gulika_sid = normalize_360(gulika_rad.to_degrees() - aya);

        // 8th lord: lord of the rashi containing the 8th cusp (lagna + 210 deg approx)
        let eighth_cusp_sid = normalize(lagna_sid_v + 210.0);
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
            lagna: lagna_sid_v,
            eighth_lord: eighth_lord_lon,
            gulika: gulika_sid,
        };
        let all = dhruv_vedic_base::all_sphutas(&inputs);
        let mut lons = [0.0f64; 16];
        for (i, (_sphuta, lon)) in all.iter().enumerate() {
            lons[i] = *lon;
        }
        Some(lons)
    } else {
        None
    };

    // Special lagnas
    let special_lagna_lons = if scope.include_special_lagnas {
        let sl = special_lagnas_for_date_with_ctx(
            engine, eop, utc, location, riseset_config, aya_config, &mut ctx,
        )?;
        Some(all_special_lagna_lons(&sl))
    } else {
        None
    };

    let charts = requests
        .iter()
        .map(|req| {
            build_amsha_chart(
                req,
                &graha_lons.longitudes,
                lagna_sid,
                scope,
                bhava_cusps_sid.as_ref(),
                arudha_lons.as_ref(),
                upagraha_lons.as_ref(),
                sphuta_lons.as_ref(),
                special_lagna_lons.as_ref(),
            )
        })
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

    // Extract arudha padas if available
    let arudha_lons = if scope.include_arudha_padas {
        kundali.bindus.as_ref().map(|b| {
            let mut lons = [0.0f64; 12];
            for i in 0..12 {
                lons[i] = b.arudha_padas[i].sidereal_longitude;
            }
            lons
        })
    } else {
        None
    };

    // Extract upagrahas if available
    let upagraha_lons = if scope.include_upagrahas {
        kundali.upagrahas.as_ref().map(|u| all_upagraha_lons(u))
    } else {
        None
    };

    // Extract special lagnas if available
    let special_lagna_lons = if scope.include_special_lagnas {
        kundali
            .special_lagnas
            .as_ref()
            .map(|s| all_special_lagna_lons(s))
    } else {
        None
    };

    // Sphutas: not available from FullKundaliResult directly
    let sphuta_lons: Option<[f64; 16]> = None;

    let charts = requests
        .iter()
        .map(|req| {
            build_amsha_chart(
                req,
                &gp.grahas.map(|g| g.sidereal_longitude),
                lagna_sid,
                scope,
                bhava_cusps_sid,
                arudha_lons.as_ref(),
                upagraha_lons.as_ref(),
                sphuta_lons.as_ref(),
                special_lagna_lons.as_ref(),
            )
        })
        .collect();

    Ok(AmshaResult { charts })
}

/// Internal: compute amsha charts from within full_kundali_for_date.
fn amsha_charts_from_kundali_with_ctx(
    config: &FullKundaliConfig,
    graha_positions: &Option<GrahaPositions>,
    bindus: &Option<BindusResult>,
    upagrahas: &Option<AllUpagrahas>,
    special_lagnas: &Option<AllSpecialLagnas>,
    ctx: &mut JyotishContext,
) -> Result<AmshaResult, SearchError> {
    let requests = selection_to_requests(&config.amsha_selection)?;
    validate_amsha_requests(&requests)?;

    let gp = graha_positions
        .as_ref()
        .ok_or(SearchError::InvalidConfig(
            "graha_positions required for amsha charts",
        ))?;

    let lagna_sid = gp.lagna.sidereal_longitude;
    let scope = &config.amsha_scope;

    // Bhava cusps (sidereal) from context
    let bhava_cusps_sid = if scope.include_bhava_cusps {
        if let Some(ref bhava_result) = ctx.bhava_result {
            let mut cusps = [0.0f64; 12];
            for i in 0..12 {
                cusps[i] = normalize(bhava_result.bhavas[i].cusp_deg - ctx.ayanamsha);
            }
            Some(cusps)
        } else {
            None
        }
    } else {
        None
    };

    let arudha_lons = if scope.include_arudha_padas {
        bindus.as_ref().map(|b| {
            let mut lons = [0.0f64; 12];
            for i in 0..12 {
                lons[i] = b.arudha_padas[i].sidereal_longitude;
            }
            lons
        })
    } else {
        None
    };

    let upagraha_lons = if scope.include_upagrahas {
        upagrahas.as_ref().map(|u| all_upagraha_lons(u))
    } else {
        None
    };

    let special_lagna_lons = if scope.include_special_lagnas {
        special_lagnas
            .as_ref()
            .map(|s| all_special_lagna_lons(s))
    } else {
        None
    };

    let sphuta_lons: Option<[f64; 16]> = None;

    let charts = requests
        .iter()
        .map(|req| {
            build_amsha_chart(
                req,
                &gp.grahas.map(|g| g.sidereal_longitude),
                lagna_sid,
                scope,
                bhava_cusps_sid.as_ref(),
                arudha_lons.as_ref(),
                upagraha_lons.as_ref(),
                sphuta_lons.as_ref(),
                special_lagna_lons.as_ref(),
            )
        })
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
}
