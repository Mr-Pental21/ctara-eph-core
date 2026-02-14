use dhruv_core::{Body, Frame, Observer, Query, StateVector};
use dhruv_frames::{
    SphericalCoords, SphericalState, cartesian_state_to_spherical_state, cartesian_to_spherical,
    nutation_iau2000b,
};
use dhruv_search::conjunction_types::{ConjunctionConfig, ConjunctionEvent};
use dhruv_search::grahan_types::{ChandraGrahan, GrahanConfig, SuryaGrahan};
use dhruv_search::panchang_types::{
    AyanaInfo, GhatikaInfo, HoraInfo, KaranaInfo, MasaInfo, PanchangInfo, PanchangNakshatraInfo,
    TithiInfo, VaarInfo, VarshaInfo, YogaInfo,
};
use dhruv_search::sankranti_types::{SankrantiConfig, SankrantiEvent};
use dhruv_search::stationary_types::{MaxSpeedEvent, StationaryConfig, StationaryEvent};
use dhruv_search::{LunarPhaseEvent, SearchError};
use dhruv_time::{EopKernel, Epoch, UtcTime, calendar_to_jd};
use dhruv_vedic_base::riseset_types::{GeoLocation, RiseSetConfig, RiseSetEvent, RiseSetResult};
use dhruv_vedic_base::{
    AshtakavargaResult, AyanamshaSystem, BhavaConfig, BhavaResult, BhinnaAshtakavarga,
    DrishtiEntry, GrahaDrishtiMatrix, LunarNode, Nakshatra28Info, NakshatraInfo, NodeMode, Rashi,
    RashiInfo, SarvaAshtakavarga, ayanamsha_deg, jd_tdb_to_centuries, lunar_node_deg,
    nakshatra_from_longitude, nakshatra28_from_longitude, rashi_from_longitude,
};

use crate::date::UtcDate;
use crate::error::DhruvError;
use crate::global::engine;

/// Convert a UTC date to a TDB Julian Date using the global engine's LSK.
fn utc_to_jd_tdb(date: UtcDate) -> Result<f64, DhruvError> {
    let eng = engine()?;
    let epoch = Epoch::from_utc(
        date.year,
        date.month,
        date.day,
        date.hour,
        date.min,
        date.sec,
        eng.lsk(),
    );
    Ok(epoch.as_jd_tdb())
}

/// Convert a UTC date to a Julian Date (UTC scale, no TDB conversion).
fn utc_to_jd_utc(date: UtcDate) -> f64 {
    let day_frac =
        date.day as f64 + date.hour as f64 / 24.0 + date.min as f64 / 1440.0 + date.sec / 86_400.0;
    calendar_to_jd(date.year, date.month, day_frac)
}

/// Query the global engine for spherical coordinates (lon, lat, distance).
///
/// Uses ecliptic J2000 frame and converts Cartesian output to spherical.
pub fn position(
    target: Body,
    observer: Observer,
    date: UtcDate,
) -> Result<SphericalCoords, DhruvError> {
    let eng = engine()?;
    let jd = utc_to_jd_tdb(date)?;
    let query = Query {
        target,
        observer,
        frame: Frame::EclipticJ2000,
        epoch_tdb_jd: jd,
    };
    let state = eng.query(query)?;
    Ok(cartesian_to_spherical(&state.position_km))
}

/// Query the global engine for full spherical state (position + angular velocities).
///
/// Uses ecliptic J2000 frame and converts Cartesian state to spherical state.
pub fn position_full(
    target: Body,
    observer: Observer,
    date: UtcDate,
) -> Result<SphericalState, DhruvError> {
    let eng = engine()?;
    let jd = utc_to_jd_tdb(date)?;
    let query = Query {
        target,
        observer,
        frame: Frame::EclipticJ2000,
        epoch_tdb_jd: jd,
    };
    let state = eng.query(query)?;
    Ok(cartesian_state_to_spherical_state(
        &state.position_km,
        &state.velocity_km_s,
    ))
}

/// Query the global engine for ecliptic longitude in degrees.
///
/// Shorthand for `position(target, observer, date)?.lon_deg`.
pub fn longitude(target: Body, observer: Observer, date: UtcDate) -> Result<f64, DhruvError> {
    Ok(position(target, observer, date)?.lon_deg)
}

/// Query the global engine for a Cartesian state vector in any frame.
pub fn query(
    target: Body,
    observer: Observer,
    frame: Frame,
    date: UtcDate,
) -> Result<StateVector, DhruvError> {
    let eng = engine()?;
    let jd = utc_to_jd_tdb(date)?;
    let q = Query {
        target,
        observer,
        frame,
        epoch_tdb_jd: jd,
    };
    Ok(eng.query(q)?)
}

/// Batch query the global engine. Each request specifies its own target,
/// observer, frame, and date. Returns one `Result` per request.
///
/// Shares memoization across queries at the same epoch, just like
/// [`dhruv_core::Engine::query_batch`].
pub fn query_batch(
    requests: &[(Body, Observer, Frame, UtcDate)],
) -> Result<Vec<Result<StateVector, DhruvError>>, DhruvError> {
    let eng = engine()?;

    let queries: Vec<Query> = requests
        .iter()
        .map(|(target, observer, frame, date)| {
            let epoch = Epoch::from_utc(
                date.year,
                date.month,
                date.day,
                date.hour,
                date.min,
                date.sec,
                eng.lsk(),
            );
            Query {
                target: *target,
                observer: *observer,
                frame: *frame,
                epoch_tdb_jd: epoch.as_jd_tdb(),
            }
        })
        .collect();

    let results = eng.query_batch(&queries);
    Ok(results
        .into_iter()
        .map(|r| r.map_err(DhruvError::from))
        .collect())
}

// ---------------------------------------------------------------------------
// Sidereal / Rashi / Nakshatra convenience
// ---------------------------------------------------------------------------

/// Compute sidereal longitude by subtracting ayanamsha from tropical longitude.
///
/// Queries the global engine for tropical ecliptic longitude, then subtracts
/// the specified ayanamsha. Result is in degrees [0, 360).
pub fn sidereal_longitude(
    target: Body,
    observer: Observer,
    date: UtcDate,
    system: AyanamshaSystem,
    use_nutation: bool,
) -> Result<f64, DhruvError> {
    let eng = engine()?;
    let epoch = Epoch::from_utc(
        date.year,
        date.month,
        date.day,
        date.hour,
        date.min,
        date.sec,
        eng.lsk(),
    );
    let jd = epoch.as_jd_tdb();
    let state = eng.query(Query {
        target,
        observer,
        frame: Frame::EclipticJ2000,
        epoch_tdb_jd: jd,
    })?;
    let tropical = cartesian_to_spherical(&state.position_km).lon_deg;
    let t = jd_tdb_to_centuries(jd);
    let aya = ayanamsha_deg(system, t, use_nutation);
    let sid = (tropical - aya) % 360.0;
    Ok(if sid < 0.0 { sid + 360.0 } else { sid })
}

/// Determine the rashi (zodiac sign) of a body at a given date.
///
/// Queries tropical longitude, subtracts ayanamsha, and returns the rashi
/// with DMS position within the sign.
pub fn rashi(
    target: Body,
    observer: Observer,
    date: UtcDate,
    system: AyanamshaSystem,
    use_nutation: bool,
) -> Result<RashiInfo, DhruvError> {
    let sid = sidereal_longitude(target, observer, date, system, use_nutation)?;
    Ok(rashi_from_longitude(sid))
}

/// Determine the nakshatra (27-scheme) of a body at a given date.
///
/// Returns nakshatra, pada (1-4), and position within the nakshatra.
pub fn nakshatra(
    target: Body,
    observer: Observer,
    date: UtcDate,
    system: AyanamshaSystem,
    use_nutation: bool,
) -> Result<NakshatraInfo, DhruvError> {
    let sid = sidereal_longitude(target, observer, date, system, use_nutation)?;
    Ok(nakshatra_from_longitude(sid))
}

/// Determine the nakshatra (28-scheme with Abhijit) of a body at a given date.
pub fn nakshatra28(
    target: Body,
    observer: Observer,
    date: UtcDate,
    system: AyanamshaSystem,
    use_nutation: bool,
) -> Result<Nakshatra28Info, DhruvError> {
    let sid = sidereal_longitude(target, observer, date, system, use_nutation)?;
    Ok(nakshatra28_from_longitude(sid))
}

// ---------------------------------------------------------------------------
// Panchang convenience functions
// ---------------------------------------------------------------------------

/// Find the next Purnima (full moon) after the given date.
pub fn next_purnima(date: UtcDate) -> Result<LunarPhaseEvent, DhruvError> {
    let eng = engine()?;
    let utc: UtcTime = date.into();
    dhruv_search::next_purnima(eng, &utc)?.ok_or(DhruvError::Search(SearchError::NoConvergence(
        "could not find next purnima",
    )))
}

/// Find the previous Purnima (full moon) before the given date.
pub fn prev_purnima(date: UtcDate) -> Result<LunarPhaseEvent, DhruvError> {
    let eng = engine()?;
    let utc: UtcTime = date.into();
    dhruv_search::prev_purnima(eng, &utc)?.ok_or(DhruvError::Search(SearchError::NoConvergence(
        "could not find previous purnima",
    )))
}

/// Find the next Amavasya (new moon) after the given date.
pub fn next_amavasya(date: UtcDate) -> Result<LunarPhaseEvent, DhruvError> {
    let eng = engine()?;
    let utc: UtcTime = date.into();
    dhruv_search::next_amavasya(eng, &utc)?.ok_or(DhruvError::Search(SearchError::NoConvergence(
        "could not find next amavasya",
    )))
}

/// Find the previous Amavasya (new moon) before the given date.
pub fn prev_amavasya(date: UtcDate) -> Result<LunarPhaseEvent, DhruvError> {
    let eng = engine()?;
    let utc: UtcTime = date.into();
    dhruv_search::prev_amavasya(eng, &utc)?.ok_or(DhruvError::Search(SearchError::NoConvergence(
        "could not find previous amavasya",
    )))
}

/// Find the next Sankranti (Sun entering a rashi) after the given date.
pub fn next_sankranti(
    date: UtcDate,
    system: AyanamshaSystem,
    use_nutation: bool,
) -> Result<SankrantiEvent, DhruvError> {
    let eng = engine()?;
    let utc: UtcTime = date.into();
    let config = SankrantiConfig::new(system, use_nutation);
    dhruv_search::next_sankranti(eng, &utc, &config)?.ok_or(DhruvError::Search(
        SearchError::NoConvergence("could not find next sankranti"),
    ))
}

/// Find the previous Sankranti (Sun entering a rashi) before the given date.
pub fn prev_sankranti(
    date: UtcDate,
    system: AyanamshaSystem,
    use_nutation: bool,
) -> Result<SankrantiEvent, DhruvError> {
    let eng = engine()?;
    let utc: UtcTime = date.into();
    let config = SankrantiConfig::new(system, use_nutation);
    dhruv_search::prev_sankranti(eng, &utc, &config)?.ok_or(DhruvError::Search(
        SearchError::NoConvergence("could not find previous sankranti"),
    ))
}

/// Determine the Masa (lunar month, Amanta system) for the given date.
pub fn masa(
    date: UtcDate,
    system: AyanamshaSystem,
    use_nutation: bool,
) -> Result<MasaInfo, DhruvError> {
    let eng = engine()?;
    let utc: UtcTime = date.into();
    let config = SankrantiConfig::new(system, use_nutation);
    Ok(dhruv_search::masa_for_date(eng, &utc, &config)?)
}

/// Determine the Ayana (Uttarayana/Dakshinayana) for the given date.
pub fn ayana(
    date: UtcDate,
    system: AyanamshaSystem,
    use_nutation: bool,
) -> Result<AyanaInfo, DhruvError> {
    let eng = engine()?;
    let utc: UtcTime = date.into();
    let config = SankrantiConfig::new(system, use_nutation);
    Ok(dhruv_search::ayana_for_date(eng, &utc, &config)?)
}

/// Determine the Varsha (60-year samvatsara cycle position) for the given date.
pub fn varsha(
    date: UtcDate,
    system: AyanamshaSystem,
    use_nutation: bool,
) -> Result<VarshaInfo, DhruvError> {
    let eng = engine()?;
    let utc: UtcTime = date.into();
    let config = SankrantiConfig::new(system, use_nutation);
    Ok(dhruv_search::varsha_for_date(eng, &utc, &config)?)
}

// ---------------------------------------------------------------------------
// Tithi / Karana / Yoga / Vaar / Hora / Ghatika
// ---------------------------------------------------------------------------

/// Determine the Tithi (lunar day) for the given date.
///
/// Returns the tithi with its start/end UTC times.
pub fn tithi(date: UtcDate) -> Result<TithiInfo, DhruvError> {
    let eng = engine()?;
    let utc: UtcTime = date.into();
    Ok(dhruv_search::tithi_for_date(eng, &utc)?)
}

/// Determine the Karana (half-tithi) for the given date.
///
/// Returns the karana with its start/end UTC times.
pub fn karana(date: UtcDate) -> Result<KaranaInfo, DhruvError> {
    let eng = engine()?;
    let utc: UtcTime = date.into();
    Ok(dhruv_search::karana_for_date(eng, &utc)?)
}

/// Determine the Yoga (luni-solar yoga) for the given date.
///
/// Requires ayanamsha system because the sum (Moon+Sun) does not cancel ayanamsha.
pub fn yoga(
    date: UtcDate,
    system: AyanamshaSystem,
    use_nutation: bool,
) -> Result<YogaInfo, DhruvError> {
    let eng = engine()?;
    let utc: UtcTime = date.into();
    let config = SankrantiConfig::new(system, use_nutation);
    Ok(dhruv_search::yoga_for_date(eng, &utc, &config)?)
}

/// Determine the Moon's Nakshatra (27-scheme) with start/end times for the given date.
///
/// Unlike [`nakshatra`] which gives any body's nakshatra as a static lookup,
/// this returns the Moon's current nakshatra with boundary times via bisection.
pub fn moon_nakshatra(
    date: UtcDate,
    system: AyanamshaSystem,
    use_nutation: bool,
) -> Result<PanchangNakshatraInfo, DhruvError> {
    let eng = engine()?;
    let utc: UtcTime = date.into();
    let config = SankrantiConfig::new(system, use_nutation);
    Ok(dhruv_search::nakshatra_for_date(eng, &utc, &config)?)
}

/// Determine the Vaar (Vedic weekday) for the given date and location.
///
/// The Vedic day runs from sunrise to next sunrise. Uses default RiseSetConfig
/// (upper limb, with refraction, no altitude correction).
pub fn vaar(
    date: UtcDate,
    eop: &EopKernel,
    location: &GeoLocation,
) -> Result<VaarInfo, DhruvError> {
    let eng = engine()?;
    let utc: UtcTime = date.into();
    let rs_config = RiseSetConfig::default();
    Ok(dhruv_search::vaar_for_date(
        eng, eop, &utc, location, &rs_config,
    )?)
}

/// Determine the Hora (planetary hour) for the given date and location.
///
/// Uses the Chaldean planetary hour sequence offset by the Vaar day lord.
pub fn hora(
    date: UtcDate,
    eop: &EopKernel,
    location: &GeoLocation,
) -> Result<HoraInfo, DhruvError> {
    let eng = engine()?;
    let utc: UtcTime = date.into();
    let rs_config = RiseSetConfig::default();
    Ok(dhruv_search::hora_for_date(
        eng, eop, &utc, location, &rs_config,
    )?)
}

/// Determine the Ghatika (1-60, each ~24 min) for the given date and location.
///
/// Ghatikas divide the Vedic day (sunrise to sunrise) into 60 equal parts.
pub fn ghatika(
    date: UtcDate,
    eop: &EopKernel,
    location: &GeoLocation,
) -> Result<GhatikaInfo, DhruvError> {
    let eng = engine()?;
    let utc: UtcTime = date.into();
    let rs_config = RiseSetConfig::default();
    Ok(dhruv_search::ghatika_for_date(
        eng, eop, &utc, location, &rs_config,
    )?)
}

/// Compute daily panchang elements for a single moment.
///
/// Returns tithi, karana, yoga, vaar, hora, and ghatika efficiently
/// by sharing intermediate computations (body longitudes, sunrises).
///
/// When `include_calendar` is true, also computes masa, ayana, and varsha.
pub fn panchang(
    date: UtcDate,
    eop: &EopKernel,
    location: &GeoLocation,
    system: AyanamshaSystem,
    use_nutation: bool,
    include_calendar: bool,
) -> Result<PanchangInfo, DhruvError> {
    let eng = engine()?;
    let utc: UtcTime = date.into();
    let rs_config = RiseSetConfig::default();
    let config = SankrantiConfig::new(system, use_nutation);
    Ok(dhruv_search::panchang_for_date(
        eng,
        eop,
        &utc,
        location,
        &rs_config,
        &config,
        include_calendar,
    )?)
}

// ---------------------------------------------------------------------------
// Graha / Sphuta convenience
// ---------------------------------------------------------------------------

/// Query sidereal longitudes of all 9 grahas at the given date.
pub fn graha_longitudes(
    date: UtcDate,
    system: AyanamshaSystem,
    use_nutation: bool,
) -> Result<dhruv_search::GrahaLongitudes, DhruvError> {
    let eng = engine()?;
    let jd = utc_to_jd_tdb(date)?;
    Ok(dhruv_search::graha_sidereal_longitudes(
        eng,
        jd,
        system,
        use_nutation,
    )?)
}

/// Compute all 16 sphutas for the given inputs.
///
/// This is a thin wrapper over [`dhruv_vedic_base::all_sphutas`].
pub fn sphutas(inputs: &dhruv_vedic_base::SphutalInputs) -> [(dhruv_vedic_base::Sphuta, f64); 16] {
    dhruv_vedic_base::all_sphutas(inputs)
}

/// Compute all 8 special lagnas for a given date and location.
///
/// Requires EOP kernel for sidereal time and sunrise computation.
pub fn special_lagnas(
    date: UtcDate,
    eop: &EopKernel,
    location: &GeoLocation,
    system: AyanamshaSystem,
    use_nutation: bool,
) -> Result<dhruv_vedic_base::AllSpecialLagnas, DhruvError> {
    let eng = engine()?;
    let utc: UtcTime = date.into();
    let rs_config = RiseSetConfig::default();
    let config = SankrantiConfig::new(system, use_nutation);
    Ok(dhruv_search::special_lagnas_for_date(
        eng, eop, &utc, location, &rs_config, &config,
    )?)
}

/// Compute all 12 arudha padas for a given date and location.
///
/// Requires EOP kernel for bhava cusp computation.
pub fn arudha_padas(
    date: UtcDate,
    eop: &EopKernel,
    location: &GeoLocation,
    system: AyanamshaSystem,
    use_nutation: bool,
) -> Result<[dhruv_vedic_base::ArudhaResult; 12], DhruvError> {
    let eng = engine()?;
    let utc: UtcTime = date.into();
    let bhava_config = dhruv_vedic_base::BhavaConfig::default();
    let config = SankrantiConfig::new(system, use_nutation);
    Ok(dhruv_search::arudha_padas_for_date(
        eng,
        eop,
        &utc,
        location,
        &bhava_config,
        &config,
    )?)
}

/// Compute complete Ashtakavarga (BAV + SAV + Sodhana) for a given date and location.
///
/// Returns all 7 Bhinna Ashtakavargas plus the Sarvashtakavarga with
/// Trikona and Ekadhipatya Sodhana applied.
pub fn ashtakavarga(
    date: UtcDate,
    eop: &EopKernel,
    location: &GeoLocation,
    system: AyanamshaSystem,
    use_nutation: bool,
) -> Result<dhruv_vedic_base::AshtakavargaResult, DhruvError> {
    let eng = engine()?;
    let utc: UtcTime = date.into();
    let config = SankrantiConfig::new(system, use_nutation);
    Ok(dhruv_search::ashtakavarga_for_date(
        eng, eop, &utc, location, &config,
    )?)
}

/// Compute comprehensive graha positions with optional features.
///
/// Set config flags to include nakshatra/pada, lagna, outer planets, and bhava placement.
pub fn graha_positions(
    date: UtcDate,
    eop: &EopKernel,
    location: &GeoLocation,
    system: AyanamshaSystem,
    use_nutation: bool,
    config: &dhruv_search::GrahaPositionsConfig,
) -> Result<dhruv_search::GrahaPositions, DhruvError> {
    let eng = engine()?;
    let utc: UtcTime = date.into();
    let bhava_config = dhruv_vedic_base::BhavaConfig::default();
    let aya_config = SankrantiConfig::new(system, use_nutation);
    Ok(dhruv_search::graha_positions(
        eng,
        eop,
        &utc,
        location,
        &bhava_config,
        &aya_config,
        config,
    )?)
}

/// Compute curated sensitive points (bindus) with optional nakshatra/bhava enrichment.
///
/// Collects 19 key Vedic points: 12 arudha padas, bhrigu bindu,
/// pranapada, gulika, maandi, hora lagna, ghati lagna, sree lagna.
pub fn core_bindus(
    date: UtcDate,
    eop: &EopKernel,
    location: &GeoLocation,
    system: AyanamshaSystem,
    use_nutation: bool,
    config: &dhruv_search::BindusConfig,
) -> Result<dhruv_search::BindusResult, DhruvError> {
    let eng = engine()?;
    let utc: UtcTime = date.into();
    let bhava_config = dhruv_vedic_base::BhavaConfig::default();
    let rs_config = RiseSetConfig::default();
    let aya_config = SankrantiConfig::new(system, use_nutation);
    Ok(dhruv_search::core_bindus(
        eng,
        eop,
        &utc,
        location,
        &bhava_config,
        &rs_config,
        &aya_config,
        config,
    )?)
}

/// Compute graha drishti (planetary aspects) with optional extensions.
///
/// Computes the 9×9 graha-to-graha virupa matrix, optionally extending to
/// graha-to-bhava-cusps, graha-to-lagna, and graha-to-core-bindus.
pub fn drishti(
    date: UtcDate,
    eop: &EopKernel,
    location: &GeoLocation,
    system: AyanamshaSystem,
    use_nutation: bool,
    config: &dhruv_search::DrishtiConfig,
) -> Result<dhruv_search::DrishtiResult, DhruvError> {
    let eng = engine()?;
    let utc: UtcTime = date.into();
    let bhava_config = dhruv_vedic_base::BhavaConfig::default();
    let rs_config = RiseSetConfig::default();
    let aya_config = SankrantiConfig::new(system, use_nutation);
    Ok(dhruv_search::drishti_for_date(
        eng,
        eop,
        &utc,
        location,
        &bhava_config,
        &rs_config,
        &aya_config,
        config,
    )?)
}

/// Compute a full kundali in one shot, sharing intermediates across sections.
pub fn full_kundali(
    date: UtcDate,
    eop: &EopKernel,
    location: &GeoLocation,
    system: AyanamshaSystem,
    use_nutation: bool,
    config: &dhruv_search::FullKundaliConfig,
) -> Result<dhruv_search::FullKundaliResult, DhruvError> {
    let eng = engine()?;
    let utc: UtcTime = date.into();
    let bhava_config = dhruv_vedic_base::BhavaConfig::default();
    let rs_config = RiseSetConfig::default();
    let aya_config = SankrantiConfig::new(system, use_nutation);
    Ok(dhruv_search::full_kundali_for_date(
        eng,
        eop,
        &utc,
        location,
        &bhava_config,
        &rs_config,
        &aya_config,
        config,
    )?)
}

/// Compute amsha (divisional) charts for a given date and location.
pub fn amsha_charts(
    date: UtcDate,
    eop: &EopKernel,
    location: &GeoLocation,
    system: AyanamshaSystem,
    use_nutation: bool,
    requests: &[dhruv_vedic_base::AmshaRequest],
    scope: &dhruv_search::AmshaChartScope,
) -> Result<dhruv_search::AmshaResult, DhruvError> {
    let eng = engine()?;
    let utc: UtcTime = date.into();
    let bhava_config = dhruv_vedic_base::BhavaConfig::default();
    let rs_config = RiseSetConfig::default();
    let aya_config = SankrantiConfig::new(system, use_nutation);
    Ok(dhruv_search::amsha_charts_for_date(
        eng,
        eop,
        &utc,
        location,
        &bhava_config,
        &rs_config,
        &aya_config,
        requests,
        scope,
    )?)
}

// ---------------------------------------------------------------------------
// Shadbala / Vimsopaka
// ---------------------------------------------------------------------------

/// Compute Shadbala (six-fold planetary strength) for all 7 sapta grahas.
pub fn shadbala(
    date: UtcDate,
    eop: &EopKernel,
    location: &GeoLocation,
    system: AyanamshaSystem,
    use_nutation: bool,
) -> Result<dhruv_search::ShadbalaResult, DhruvError> {
    let eng = engine()?;
    let utc: UtcTime = date.into();
    let bhava_config = dhruv_vedic_base::BhavaConfig::default();
    let rs_config = RiseSetConfig::default();
    let aya_config = SankrantiConfig::new(system, use_nutation);
    Ok(dhruv_search::shadbala_for_date(
        eng, eop, &utc, location, &bhava_config, &rs_config, &aya_config,
    )?)
}

/// Compute Shadbala for a single graha (sapta grahas only; rejects Rahu/Ketu).
pub fn shadbala_for_graha(
    date: UtcDate,
    eop: &EopKernel,
    location: &GeoLocation,
    system: AyanamshaSystem,
    use_nutation: bool,
    graha: dhruv_vedic_base::Graha,
) -> Result<dhruv_search::ShadbalaEntry, DhruvError> {
    let eng = engine()?;
    let utc: UtcTime = date.into();
    let bhava_config = dhruv_vedic_base::BhavaConfig::default();
    let rs_config = RiseSetConfig::default();
    let aya_config = SankrantiConfig::new(system, use_nutation);
    Ok(dhruv_search::shadbala_for_graha(
        eng, eop, &utc, location, &bhava_config, &rs_config, &aya_config, graha,
    )?)
}

/// Compute Vimsopaka Bala for all 9 navagrahas.
pub fn vimsopaka(
    date: UtcDate,
    eop: &EopKernel,
    location: &GeoLocation,
    system: AyanamshaSystem,
    use_nutation: bool,
    node_policy: dhruv_vedic_base::NodeDignityPolicy,
) -> Result<dhruv_search::VimsopakaResult, DhruvError> {
    let eng = engine()?;
    let utc: UtcTime = date.into();
    let aya_config = SankrantiConfig::new(system, use_nutation);
    Ok(dhruv_search::vimsopaka_for_date(
        eng, eop, &utc, location, &aya_config, node_policy,
    )?)
}

/// Compute Vimsopaka Bala for a single graha (all 9 including Rahu/Ketu).
pub fn vimsopaka_for_graha(
    date: UtcDate,
    eop: &EopKernel,
    location: &GeoLocation,
    system: AyanamshaSystem,
    use_nutation: bool,
    node_policy: dhruv_vedic_base::NodeDignityPolicy,
    graha: dhruv_vedic_base::Graha,
) -> Result<dhruv_search::VimsopakaEntry, DhruvError> {
    let eng = engine()?;
    let utc: UtcTime = date.into();
    let aya_config = SankrantiConfig::new(system, use_nutation);
    Ok(dhruv_search::vimsopaka_for_graha(
        eng, eop, &utc, location, &aya_config, node_policy, graha,
    )?)
}

/// Compute all 11 upagrahas for a given date and location.
pub fn upagrahas(
    date: UtcDate,
    eop: &EopKernel,
    location: &GeoLocation,
    system: AyanamshaSystem,
    use_nutation: bool,
) -> Result<dhruv_vedic_base::AllUpagrahas, DhruvError> {
    let eng = engine()?;
    let utc: UtcTime = date.into();
    let rs_config = RiseSetConfig::default();
    let config = SankrantiConfig::new(system, use_nutation);
    Ok(dhruv_search::all_upagrahas_for_date(
        eng, eop, &utc, location, &rs_config, &config,
    )?)
}

// ---------------------------------------------------------------------------
// Ayanamsha / Nutation
// ---------------------------------------------------------------------------

/// Compute the ayanamsha value in degrees for the given date.
///
/// Returns the precession offset used to convert tropical to sidereal longitudes.
pub fn ayanamsha(
    date: UtcDate,
    system: AyanamshaSystem,
    use_nutation: bool,
) -> Result<f64, DhruvError> {
    let jd = utc_to_jd_tdb(date)?;
    let t = jd_tdb_to_centuries(jd);
    Ok(ayanamsha_deg(system, t, use_nutation))
}

/// Compute IAU 2000B nutation values for the given date.
///
/// Returns `(delta_psi_arcsec, delta_epsilon_arcsec)` — nutation in
/// longitude and obliquity respectively, both in arcseconds.
pub fn nutation(date: UtcDate) -> Result<(f64, f64), DhruvError> {
    let jd = utc_to_jd_tdb(date)?;
    let t = jd_tdb_to_centuries(jd);
    Ok(nutation_iau2000b(t))
}

// ---------------------------------------------------------------------------
// Lunar Node
// ---------------------------------------------------------------------------

/// Compute the ecliptic longitude of a lunar node (Rahu or Ketu).
///
/// Returns longitude in degrees [0, 360).
pub fn lunar_node(node: LunarNode, date: UtcDate, mode: NodeMode) -> Result<f64, DhruvError> {
    let jd = utc_to_jd_tdb(date)?;
    let t = jd_tdb_to_centuries(jd);
    Ok(lunar_node_deg(node, t, mode))
}

// ---------------------------------------------------------------------------
// Rise / Set
// ---------------------------------------------------------------------------

/// Compute sunrise for the given date and location.
///
/// Uses default RiseSetConfig (upper limb, with refraction).
pub fn sunrise(
    date: UtcDate,
    eop: &EopKernel,
    location: &GeoLocation,
) -> Result<RiseSetResult, DhruvError> {
    let eng = engine()?;
    let jd_utc = utc_to_jd_utc(date);
    Ok(dhruv_vedic_base::compute_rise_set(
        eng,
        eng.lsk(),
        eop,
        location,
        RiseSetEvent::Sunrise,
        jd_utc,
        &RiseSetConfig::default(),
    )?)
}

/// Compute sunset for the given date and location.
///
/// Uses default RiseSetConfig (upper limb, with refraction).
pub fn sunset(
    date: UtcDate,
    eop: &EopKernel,
    location: &GeoLocation,
) -> Result<RiseSetResult, DhruvError> {
    let eng = engine()?;
    let jd_utc = utc_to_jd_utc(date);
    Ok(dhruv_vedic_base::compute_rise_set(
        eng,
        eng.lsk(),
        eop,
        location,
        RiseSetEvent::Sunset,
        jd_utc,
        &RiseSetConfig::default(),
    )?)
}

/// Compute all 8 rise/set/twilight events for the given date and location.
///
/// Returns results for: Sunrise, Sunset, CivilDawn, CivilDusk,
/// NauticalDawn, NauticalDusk, AstronomicalDawn, AstronomicalDusk.
pub fn all_rise_set_events(
    date: UtcDate,
    eop: &EopKernel,
    location: &GeoLocation,
) -> Result<Vec<RiseSetResult>, DhruvError> {
    let eng = engine()?;
    let jd_utc = utc_to_jd_utc(date);
    Ok(dhruv_vedic_base::compute_all_events(
        eng,
        eng.lsk(),
        eop,
        location,
        jd_utc,
        &RiseSetConfig::default(),
    )?)
}

// ---------------------------------------------------------------------------
// Bhava (House) Computation
// ---------------------------------------------------------------------------

/// Compute 12 bhava (house) cusps for the given date and location.
pub fn bhavas(
    date: UtcDate,
    eop: &EopKernel,
    location: &GeoLocation,
    config: &BhavaConfig,
) -> Result<BhavaResult, DhruvError> {
    let eng = engine()?;
    let jd_utc = utc_to_jd_utc(date);
    Ok(dhruv_vedic_base::compute_bhavas(
        eng,
        eng.lsk(),
        eop,
        location,
        jd_utc,
        config,
    )?)
}

// ---------------------------------------------------------------------------
// Lagna / MC / RAMC
// ---------------------------------------------------------------------------

/// Compute the lagna (ascendant) ecliptic longitude in degrees.
pub fn lagna(date: UtcDate, eop: &EopKernel, location: &GeoLocation) -> Result<f64, DhruvError> {
    let eng = engine()?;
    let jd_utc = utc_to_jd_utc(date);
    let rad = dhruv_vedic_base::lagna_longitude_rad(eng.lsk(), eop, location, jd_utc)?;
    Ok(rad.to_degrees().rem_euclid(360.0))
}

/// Compute the MC (Midheaven) ecliptic longitude in degrees.
pub fn mc(date: UtcDate, eop: &EopKernel, location: &GeoLocation) -> Result<f64, DhruvError> {
    let eng = engine()?;
    let jd_utc = utc_to_jd_utc(date);
    let rad = dhruv_vedic_base::mc_longitude_rad(eng.lsk(), eop, location, jd_utc)?;
    Ok(rad.to_degrees().rem_euclid(360.0))
}

/// Compute RAMC (Right Ascension of MC / local sidereal time) in degrees.
pub fn ramc(date: UtcDate, eop: &EopKernel, location: &GeoLocation) -> Result<f64, DhruvError> {
    let eng = engine()?;
    let jd_utc = utc_to_jd_utc(date);
    let rad = dhruv_vedic_base::ramc_rad(eng.lsk(), eop, location, jd_utc)?;
    Ok(rad.to_degrees().rem_euclid(360.0))
}

// ---------------------------------------------------------------------------
// Conjunction Search
// ---------------------------------------------------------------------------

/// Find the next conjunction/aspect event between two bodies after the given date.
pub fn next_conjunction(
    body1: Body,
    body2: Body,
    date: UtcDate,
    config: &ConjunctionConfig,
) -> Result<Option<ConjunctionEvent>, DhruvError> {
    let eng = engine()?;
    let jd = utc_to_jd_tdb(date)?;
    Ok(dhruv_search::next_conjunction(
        eng, body1, body2, jd, config,
    )?)
}

/// Find the previous conjunction/aspect event between two bodies before the given date.
pub fn prev_conjunction(
    body1: Body,
    body2: Body,
    date: UtcDate,
    config: &ConjunctionConfig,
) -> Result<Option<ConjunctionEvent>, DhruvError> {
    let eng = engine()?;
    let jd = utc_to_jd_tdb(date)?;
    Ok(dhruv_search::prev_conjunction(
        eng, body1, body2, jd, config,
    )?)
}

/// Search for all conjunction/aspect events between two bodies in a date range.
pub fn search_conjunctions(
    body1: Body,
    body2: Body,
    start: UtcDate,
    end: UtcDate,
    config: &ConjunctionConfig,
) -> Result<Vec<ConjunctionEvent>, DhruvError> {
    let eng = engine()?;
    let jd_start = utc_to_jd_tdb(start)?;
    let jd_end = utc_to_jd_tdb(end)?;
    Ok(dhruv_search::search_conjunctions(
        eng, body1, body2, jd_start, jd_end, config,
    )?)
}

// ---------------------------------------------------------------------------
// Eclipse (Grahan) Search
// ---------------------------------------------------------------------------

/// Find the next lunar eclipse (Chandra Grahan) after the given date.
pub fn next_chandra_grahan(date: UtcDate) -> Result<Option<ChandraGrahan>, DhruvError> {
    let eng = engine()?;
    let jd = utc_to_jd_tdb(date)?;
    Ok(dhruv_search::next_chandra_grahan(
        eng,
        jd,
        &GrahanConfig::default(),
    )?)
}

/// Find the previous lunar eclipse (Chandra Grahan) before the given date.
pub fn prev_chandra_grahan(date: UtcDate) -> Result<Option<ChandraGrahan>, DhruvError> {
    let eng = engine()?;
    let jd = utc_to_jd_tdb(date)?;
    Ok(dhruv_search::prev_chandra_grahan(
        eng,
        jd,
        &GrahanConfig::default(),
    )?)
}

/// Search for all lunar eclipses in a date range.
pub fn search_chandra_grahan(
    start: UtcDate,
    end: UtcDate,
) -> Result<Vec<ChandraGrahan>, DhruvError> {
    let eng = engine()?;
    let jd_start = utc_to_jd_tdb(start)?;
    let jd_end = utc_to_jd_tdb(end)?;
    Ok(dhruv_search::search_chandra_grahan(
        eng,
        jd_start,
        jd_end,
        &GrahanConfig::default(),
    )?)
}

/// Find the next solar eclipse (Surya Grahan) after the given date.
pub fn next_surya_grahan(date: UtcDate) -> Result<Option<SuryaGrahan>, DhruvError> {
    let eng = engine()?;
    let jd = utc_to_jd_tdb(date)?;
    Ok(dhruv_search::next_surya_grahan(
        eng,
        jd,
        &GrahanConfig::default(),
    )?)
}

/// Find the previous solar eclipse (Surya Grahan) before the given date.
pub fn prev_surya_grahan(date: UtcDate) -> Result<Option<SuryaGrahan>, DhruvError> {
    let eng = engine()?;
    let jd = utc_to_jd_tdb(date)?;
    Ok(dhruv_search::prev_surya_grahan(
        eng,
        jd,
        &GrahanConfig::default(),
    )?)
}

/// Search for all solar eclipses in a date range.
pub fn search_surya_grahan(start: UtcDate, end: UtcDate) -> Result<Vec<SuryaGrahan>, DhruvError> {
    let eng = engine()?;
    let jd_start = utc_to_jd_tdb(start)?;
    let jd_end = utc_to_jd_tdb(end)?;
    Ok(dhruv_search::search_surya_grahan(
        eng,
        jd_start,
        jd_end,
        &GrahanConfig::default(),
    )?)
}

// ---------------------------------------------------------------------------
// Stationary / Max-Speed Search
// ---------------------------------------------------------------------------

/// Find the next stationary point (retrograde or direct station) for a body.
pub fn next_stationary(
    body: Body,
    date: UtcDate,
    config: &StationaryConfig,
) -> Result<Option<StationaryEvent>, DhruvError> {
    let eng = engine()?;
    let jd = utc_to_jd_tdb(date)?;
    Ok(dhruv_search::next_stationary(eng, body, jd, config)?)
}

/// Find the previous stationary point for a body.
pub fn prev_stationary(
    body: Body,
    date: UtcDate,
    config: &StationaryConfig,
) -> Result<Option<StationaryEvent>, DhruvError> {
    let eng = engine()?;
    let jd = utc_to_jd_tdb(date)?;
    Ok(dhruv_search::prev_stationary(eng, body, jd, config)?)
}

/// Search for all stationary points of a body in a date range.
pub fn search_stationary(
    body: Body,
    start: UtcDate,
    end: UtcDate,
    config: &StationaryConfig,
) -> Result<Vec<StationaryEvent>, DhruvError> {
    let eng = engine()?;
    let jd_start = utc_to_jd_tdb(start)?;
    let jd_end = utc_to_jd_tdb(end)?;
    Ok(dhruv_search::search_stationary(
        eng, body, jd_start, jd_end, config,
    )?)
}

/// Find the next maximum-speed event for a body.
pub fn next_max_speed(
    body: Body,
    date: UtcDate,
    config: &StationaryConfig,
) -> Result<Option<MaxSpeedEvent>, DhruvError> {
    let eng = engine()?;
    let jd = utc_to_jd_tdb(date)?;
    Ok(dhruv_search::next_max_speed(eng, body, jd, config)?)
}

/// Find the previous maximum-speed event for a body.
pub fn prev_max_speed(
    body: Body,
    date: UtcDate,
    config: &StationaryConfig,
) -> Result<Option<MaxSpeedEvent>, DhruvError> {
    let eng = engine()?;
    let jd = utc_to_jd_tdb(date)?;
    Ok(dhruv_search::prev_max_speed(eng, body, jd, config)?)
}

/// Search for all maximum-speed events of a body in a date range.
pub fn search_max_speed(
    body: Body,
    start: UtcDate,
    end: UtcDate,
    config: &StationaryConfig,
) -> Result<Vec<MaxSpeedEvent>, DhruvError> {
    let eng = engine()?;
    let jd_start = utc_to_jd_tdb(start)?;
    let jd_end = utc_to_jd_tdb(end)?;
    Ok(dhruv_search::search_max_speed(
        eng, body, jd_start, jd_end, config,
    )?)
}

// ---------------------------------------------------------------------------
// Panchang range-search functions
// ---------------------------------------------------------------------------

/// Search for all Purnimas (full moons) in a date range.
pub fn search_purnimas(start: UtcDate, end: UtcDate) -> Result<Vec<LunarPhaseEvent>, DhruvError> {
    let eng = engine()?;
    let utc_start: UtcTime = start.into();
    let utc_end: UtcTime = end.into();
    Ok(dhruv_search::search_purnimas(eng, &utc_start, &utc_end)?)
}

/// Search for all Amavasyas (new moons) in a date range.
pub fn search_amavasyas(start: UtcDate, end: UtcDate) -> Result<Vec<LunarPhaseEvent>, DhruvError> {
    let eng = engine()?;
    let utc_start: UtcTime = start.into();
    let utc_end: UtcTime = end.into();
    Ok(dhruv_search::search_amavasyas(eng, &utc_start, &utc_end)?)
}

/// Search for all Sankrantis (Sun entering a rashi) in a date range.
pub fn search_sankrantis(
    start: UtcDate,
    end: UtcDate,
    system: AyanamshaSystem,
    use_nutation: bool,
) -> Result<Vec<SankrantiEvent>, DhruvError> {
    let eng = engine()?;
    let utc_start: UtcTime = start.into();
    let utc_end: UtcTime = end.into();
    let config = SankrantiConfig::new(system, use_nutation);
    Ok(dhruv_search::search_sankrantis(
        eng, &utc_start, &utc_end, &config,
    )?)
}

/// Find the next Sankranti for a specific rashi (e.g. Mesha Sankranti).
pub fn next_specific_sankranti(
    date: UtcDate,
    rashi: Rashi,
    system: AyanamshaSystem,
    use_nutation: bool,
) -> Result<Option<SankrantiEvent>, DhruvError> {
    let eng = engine()?;
    let utc: UtcTime = date.into();
    let config = SankrantiConfig::new(system, use_nutation);
    Ok(dhruv_search::next_specific_sankranti(
        eng, &utc, rashi, &config,
    )?)
}

/// Find the previous Sankranti for a specific rashi.
pub fn prev_specific_sankranti(
    date: UtcDate,
    rashi: Rashi,
    system: AyanamshaSystem,
    use_nutation: bool,
) -> Result<Option<SankrantiEvent>, DhruvError> {
    let eng = engine()?;
    let utc: UtcTime = date.into();
    let config = SankrantiConfig::new(system, use_nutation);
    Ok(dhruv_search::prev_specific_sankranti(
        eng, &utc, rashi, &config,
    )?)
}

// ---------------------------------------------------------------------------
// Individual Sphuta Formulas (pure math, no engine required)
// ---------------------------------------------------------------------------

/// Bhrigu Bindu = midpoint of Rahu and Moon.
pub fn bhrigu_bindu(rahu: f64, moon: f64) -> f64 {
    dhruv_vedic_base::bhrigu_bindu(rahu, moon)
}

/// Prana Sphuta = Lagna + Moon (mod 360).
pub fn prana_sphuta(lagna: f64, moon: f64) -> f64 {
    dhruv_vedic_base::prana_sphuta(lagna, moon)
}

/// Deha Sphuta = Moon + Lagna (mod 360) — alias with reversed parameter order.
pub fn deha_sphuta(moon: f64, lagna: f64) -> f64 {
    dhruv_vedic_base::deha_sphuta(moon, lagna)
}

/// Mrityu Sphuta = 8th lord + Lagna (mod 360).
pub fn mrityu_sphuta(eighth_lord: f64, lagna: f64) -> f64 {
    dhruv_vedic_base::mrityu_sphuta(eighth_lord, lagna)
}

/// Tithi Sphuta = (Moon - Sun) mod 360 + Lagna (mod 360).
pub fn tithi_sphuta(moon: f64, sun: f64, lagna: f64) -> f64 {
    dhruv_vedic_base::tithi_sphuta(moon, sun, lagna)
}

/// Yoga Sphuta = Sun + Moon (mod 360).
pub fn yoga_sphuta(sun: f64, moon: f64) -> f64 {
    dhruv_vedic_base::yoga_sphuta(sun, moon)
}

/// Yoga Sphuta Normalized = (Sun + Moon) mod 360.
pub fn yoga_sphuta_normalized(sun: f64, moon: f64) -> f64 {
    dhruv_vedic_base::yoga_sphuta_normalized(sun, moon)
}

/// Rahu Tithi Sphuta = (Rahu - Sun) mod 360 + Lagna (mod 360).
pub fn rahu_tithi_sphuta(rahu: f64, sun: f64, lagna: f64) -> f64 {
    dhruv_vedic_base::rahu_tithi_sphuta(rahu, sun, lagna)
}

/// Kshetra Sphuta from Venus, Moon, Mars, Jupiter, and Lagna.
pub fn kshetra_sphuta(venus: f64, moon: f64, mars: f64, jupiter: f64, lagna: f64) -> f64 {
    dhruv_vedic_base::kshetra_sphuta(venus, moon, mars, jupiter, lagna)
}

/// Beeja Sphuta from Sun, Venus, and Jupiter.
pub fn beeja_sphuta(sun: f64, venus: f64, jupiter: f64) -> f64 {
    dhruv_vedic_base::beeja_sphuta(sun, venus, jupiter)
}

/// TriSphuta = Lagna + Moon + Gulika (mod 360).
pub fn trisphuta(lagna: f64, moon: f64, gulika: f64) -> f64 {
    dhruv_vedic_base::trisphuta(lagna, moon, gulika)
}

/// ChatusSphuta = TriSphuta + Sun (mod 360).
pub fn chatussphuta(trisphuta_val: f64, sun: f64) -> f64 {
    dhruv_vedic_base::chatussphuta(trisphuta_val, sun)
}

/// PanchaSphuta = ChatusSphuta + Rahu (mod 360).
pub fn panchasphuta(chatussphuta_val: f64, rahu: f64) -> f64 {
    dhruv_vedic_base::panchasphuta(chatussphuta_val, rahu)
}

/// Sookshma TriSphuta = Lagna + Moon + Gulika + Sun (mod 360).
pub fn sookshma_trisphuta(lagna: f64, moon: f64, gulika: f64, sun: f64) -> f64 {
    dhruv_vedic_base::sookshma_trisphuta(lagna, moon, gulika, sun)
}

/// Avayoga Sphuta = Sun + Moon inverted.
pub fn avayoga_sphuta(sun: f64, moon: f64) -> f64 {
    dhruv_vedic_base::avayoga_sphuta(sun, moon)
}

/// Kunda = Lagna + Moon + Mars (mod 360).
pub fn kunda(lagna: f64, moon: f64, mars: f64) -> f64 {
    dhruv_vedic_base::kunda(lagna, moon, mars)
}

// ---------------------------------------------------------------------------
// Individual Special Lagna Formulas (pure math, no engine required)
// ---------------------------------------------------------------------------

/// Bhava Lagna from Sun longitude and ghatikas since sunrise.
pub fn bhava_lagna(sun_lon: f64, ghatikas: f64) -> f64 {
    dhruv_vedic_base::bhava_lagna(sun_lon, ghatikas)
}

/// Hora Lagna from Sun longitude and ghatikas since sunrise.
pub fn hora_lagna(sun_lon: f64, ghatikas: f64) -> f64 {
    dhruv_vedic_base::hora_lagna(sun_lon, ghatikas)
}

/// Ghati Lagna from Sun longitude and ghatikas since sunrise.
pub fn ghati_lagna(sun_lon: f64, ghatikas: f64) -> f64 {
    dhruv_vedic_base::ghati_lagna(sun_lon, ghatikas)
}

/// Vighati Lagna from lagna longitude and vighatikas elapsed.
pub fn vighati_lagna(lagna_lon: f64, vighatikas: f64) -> f64 {
    dhruv_vedic_base::vighati_lagna(lagna_lon, vighatikas)
}

/// Varnada Lagna from lagna and hora lagna longitudes.
pub fn varnada_lagna(lagna_lon: f64, hora_lagna_lon: f64) -> f64 {
    dhruv_vedic_base::varnada_lagna(lagna_lon, hora_lagna_lon)
}

/// Sree Lagna from Moon and lagna longitudes.
pub fn sree_lagna(moon_lon: f64, lagna_lon: f64) -> f64 {
    dhruv_vedic_base::sree_lagna(moon_lon, lagna_lon)
}

/// Pranapada Lagna from Sun longitude and ghatikas since sunrise.
pub fn pranapada_lagna(sun_lon: f64, ghatikas: f64) -> f64 {
    dhruv_vedic_base::pranapada_lagna(sun_lon, ghatikas)
}

/// Indu Lagna from Moon longitude and the two graha lords.
pub fn indu_lagna(
    moon_lon: f64,
    lagna_lord: dhruv_vedic_base::Graha,
    moon_9th_lord: dhruv_vedic_base::Graha,
) -> f64 {
    dhruv_vedic_base::indu_lagna(moon_lon, lagna_lord, moon_9th_lord)
}

/// Compute ghatikas elapsed since sunrise for a given UTC moment.
pub fn ghatikas_since_sunrise(
    date: UtcDate,
    eop: &EopKernel,
    location: &GeoLocation,
) -> Result<f64, DhruvError> {
    let eng = engine()?;
    let utc: UtcTime = date.into();
    let rs_config = RiseSetConfig::default();
    let (sunrise_jd, next_sunrise_jd) =
        dhruv_search::vedic_day_sunrises(eng, eop, &utc, location, &rs_config)?;
    let jd_tdb = utc_to_jd_tdb(date)?;
    Ok(dhruv_vedic_base::ghatikas_since_sunrise(
        jd_tdb,
        sunrise_jd,
        next_sunrise_jd,
    ))
}

// ---------------------------------------------------------------------------
// Utility Primitives (pure math or simple lookups)
// ---------------------------------------------------------------------------

/// Determine tithi position from Moon-Sun elongation in degrees.
pub fn tithi_from_elongation(elongation_deg: f64) -> dhruv_vedic_base::TithiPosition {
    dhruv_vedic_base::tithi_from_elongation(elongation_deg)
}

/// Determine karana position from Moon-Sun elongation in degrees.
pub fn karana_from_elongation(elongation_deg: f64) -> dhruv_vedic_base::KaranaPosition {
    dhruv_vedic_base::karana_from_elongation(elongation_deg)
}

/// Determine yoga position from sidereal Sun+Moon sum in degrees.
pub fn yoga_from_sum(sum_deg: f64) -> dhruv_vedic_base::YogaPosition {
    dhruv_vedic_base::yoga_from_sum(sum_deg)
}

/// Determine the Vaar (weekday) from a Julian Date.
pub fn vaar_from_jd(jd: f64) -> dhruv_vedic_base::Vaar {
    dhruv_vedic_base::vaar_from_jd(jd)
}

/// Determine the Masa (lunar month name) from a rashi index (0-11).
pub fn masa_from_rashi_index(rashi_index: u8) -> dhruv_vedic_base::Masa {
    dhruv_vedic_base::masa_from_rashi_index(rashi_index)
}

/// Determine the Ayana from a sidereal longitude in degrees.
pub fn ayana_from_sidereal_longitude(lon_deg: f64) -> dhruv_vedic_base::Ayana {
    dhruv_vedic_base::ayana_from_sidereal_longitude(lon_deg)
}

/// Determine the Samvatsara (60-year cycle) from a year.
///
/// Returns `(samvatsara, order)` where `order` is 1-60.
pub fn samvatsara_from_year(year: i32) -> (dhruv_vedic_base::Samvatsara, u8) {
    dhruv_vedic_base::samvatsara_from_year(year)
}

/// Compute the rashi index that is `offset` signs from `rashi_index`.
pub fn nth_rashi_from(rashi_index: u8, offset: u8) -> u8 {
    dhruv_vedic_base::nth_rashi_from(rashi_index, offset)
}

/// Determine the rashi lord for a given rashi index (0-11).
pub fn rashi_lord(rashi_index: u8) -> Option<dhruv_vedic_base::Graha> {
    dhruv_vedic_base::rashi_lord_by_index(rashi_index)
}

/// Compute the Hora lord at a specific (vaar, hora_index) combination.
pub fn hora_at(vaar: dhruv_vedic_base::Vaar, hora_index: u8) -> dhruv_vedic_base::Hora {
    dhruv_vedic_base::hora_at(vaar, hora_index)
}

/// Compute ghatika position from elapsed time since sunrise.
///
/// `seconds_since_sunrise` and `vedic_day_duration_seconds` define the position.
pub fn ghatika_from_elapsed(
    seconds_since_sunrise: f64,
    vedic_day_duration_seconds: f64,
) -> dhruv_vedic_base::GhatikaPosition {
    dhruv_vedic_base::ghatika_from_elapsed(seconds_since_sunrise, vedic_day_duration_seconds)
}

/// Normalize an angle to the range [0, 360).
pub fn normalize_360(deg: f64) -> f64 {
    dhruv_vedic_base::normalize_360(deg)
}

/// Approximate Julian Date of local noon from a midnight JD and longitude.
pub fn approximate_local_noon_jd(jd_ut_midnight: f64, longitude_deg: f64) -> f64 {
    dhruv_vedic_base::approximate_local_noon_jd(jd_ut_midnight, longitude_deg)
}

/// Compute a single arudha pada from bhava cusp longitude and its lord longitude.
///
/// Returns `(longitude_deg, rashi_index)`.
pub fn arudha_pada(bhava_cusp_lon: f64, lord_lon: f64) -> (f64, u8) {
    dhruv_vedic_base::arudha_pada(bhava_cusp_lon, lord_lon)
}

/// Compute the 5 sun-based upagrahas from the Sun's sidereal longitude.
pub fn sun_based_upagrahas(sun_sidereal_lon: f64) -> dhruv_vedic_base::SunBasedUpagrahas {
    dhruv_vedic_base::sun_based_upagrahas(sun_sidereal_lon)
}

// ---------------------------------------------------------------------------
// Panchang Intermediates (require engine access)
// ---------------------------------------------------------------------------

/// Compute Moon-Sun elongation (Moon_lon - Sun_lon) mod 360 at a given date.
pub fn elongation_at(date: UtcDate) -> Result<f64, DhruvError> {
    let eng = engine()?;
    let jd = utc_to_jd_tdb(date)?;
    Ok(dhruv_search::elongation_at(eng, jd)?)
}

/// Compute sidereal sum (Moon_sid + Sun_sid) mod 360 at a given date.
pub fn sidereal_sum_at(
    date: UtcDate,
    system: AyanamshaSystem,
    use_nutation: bool,
) -> Result<f64, DhruvError> {
    let eng = engine()?;
    let jd = utc_to_jd_tdb(date)?;
    let config = SankrantiConfig::new(system, use_nutation);
    Ok(dhruv_search::sidereal_sum_at(eng, jd, &config)?)
}

/// Compute the Vedic day sunrise bracket (today's and next sunrise JD TDB).
pub fn vedic_day_sunrises(
    date: UtcDate,
    eop: &EopKernel,
    location: &GeoLocation,
) -> Result<(f64, f64), DhruvError> {
    let eng = engine()?;
    let utc: UtcTime = date.into();
    let rs_config = RiseSetConfig::default();
    Ok(dhruv_search::vedic_day_sunrises(
        eng, eop, &utc, location, &rs_config,
    )?)
}

/// Query a body's ecliptic longitude and latitude in degrees.
pub fn body_ecliptic_lon_lat(body: Body, date: UtcDate) -> Result<(f64, f64), DhruvError> {
    let eng = engine()?;
    let jd = utc_to_jd_tdb(date)?;
    Ok(dhruv_search::body_ecliptic_lon_lat(eng, body, jd)?)
}

/// Determine the Tithi from a pre-computed elongation at a given date.
pub fn tithi_at(date: UtcDate, elongation_deg: f64) -> Result<TithiInfo, DhruvError> {
    let eng = engine()?;
    let jd = utc_to_jd_tdb(date)?;
    Ok(dhruv_search::tithi_at(eng, jd, elongation_deg)?)
}

/// Determine the Karana from a pre-computed elongation at a given date.
pub fn karana_at(date: UtcDate, elongation_deg: f64) -> Result<KaranaInfo, DhruvError> {
    let eng = engine()?;
    let jd = utc_to_jd_tdb(date)?;
    Ok(dhruv_search::karana_at(eng, jd, elongation_deg)?)
}

/// Determine the Yoga from a pre-computed sidereal sum at a given date.
pub fn yoga_at(
    date: UtcDate,
    sidereal_sum_deg: f64,
    system: AyanamshaSystem,
    use_nutation: bool,
) -> Result<YogaInfo, DhruvError> {
    let eng = engine()?;
    let jd = utc_to_jd_tdb(date)?;
    let config = SankrantiConfig::new(system, use_nutation);
    Ok(dhruv_search::yoga_at(eng, jd, sidereal_sum_deg, &config)?)
}

/// Determine the Moon's nakshatra from a pre-computed sidereal longitude.
pub fn nakshatra_at(
    date: UtcDate,
    moon_sidereal_deg: f64,
    system: AyanamshaSystem,
    use_nutation: bool,
) -> Result<PanchangNakshatraInfo, DhruvError> {
    let eng = engine()?;
    let jd = utc_to_jd_tdb(date)?;
    let config = SankrantiConfig::new(system, use_nutation);
    Ok(dhruv_search::nakshatra_at(
        eng,
        jd,
        moon_sidereal_deg,
        &config,
    )?)
}

// ---------------------------------------------------------------------------
// Low-level Ashtakavarga / Drishti (pure computation)
// ---------------------------------------------------------------------------

/// Compute full Ashtakavarga from rashi positions (BAV + SAV + Sodhana).
///
/// `graha_rashis` contains rashi indices (0-11) for Sun through Saturn (7 planets).
/// `lagna_rashi` is the lagna's rashi index (0-11).
pub fn calculate_ashtakavarga(graha_rashis: &[u8; 7], lagna_rashi: u8) -> AshtakavargaResult {
    dhruv_vedic_base::calculate_ashtakavarga(graha_rashis, lagna_rashi)
}

/// Compute a single Bhinna Ashtakavarga (BAV) for one graha.
///
/// `graha_index` is 0-6 (Sun through Saturn).
pub fn calculate_bav(
    graha_index: u8,
    graha_rashis: &[u8; 7],
    lagna_rashi: u8,
) -> BhinnaAshtakavarga {
    dhruv_vedic_base::calculate_bav(graha_index, graha_rashis, lagna_rashi)
}

/// Compute all 7 Bhinna Ashtakavargas.
pub fn calculate_all_bav(graha_rashis: &[u8; 7], lagna_rashi: u8) -> [BhinnaAshtakavarga; 7] {
    dhruv_vedic_base::calculate_all_bav(graha_rashis, lagna_rashi)
}

/// Compute Sarva Ashtakavarga (SAV) from all 7 BAVs.
pub fn calculate_sav(bavs: &[BhinnaAshtakavarga; 7]) -> SarvaAshtakavarga {
    dhruv_vedic_base::calculate_sav(bavs)
}

/// Apply Trikona Sodhana reduction to SAV totals.
pub fn trikona_sodhana(totals: &[u8; 12]) -> [u8; 12] {
    dhruv_vedic_base::trikona_sodhana(totals)
}

/// Apply Ekadhipatya Sodhana reduction to post-Trikona totals.
pub fn ekadhipatya_sodhana(after_trikona: &[u8; 12]) -> [u8; 12] {
    dhruv_vedic_base::ekadhipatya_sodhana(after_trikona)
}

/// Compute graha drishti (planetary aspect) between a single source and target.
pub fn graha_drishti(
    graha: dhruv_vedic_base::Graha,
    source_lon: f64,
    target_lon: f64,
) -> DrishtiEntry {
    dhruv_vedic_base::graha_drishti(graha, source_lon, target_lon)
}

/// Compute the full 9×9 graha drishti matrix from sidereal longitudes.
pub fn graha_drishti_matrix(longitudes: &[f64; 9]) -> GrahaDrishtiMatrix {
    dhruv_vedic_base::graha_drishti_matrix(longitudes)
}
