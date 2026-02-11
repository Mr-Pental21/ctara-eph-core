use dhruv_core::{Body, Frame, Observer, Query, StateVector};
use dhruv_frames::{
    SphericalCoords, SphericalState, cartesian_state_to_spherical_state, cartesian_to_spherical,
};
use dhruv_search::panchang_types::{
    AyanaInfo, GhatikaInfo, HoraInfo, KaranaInfo, MasaInfo, PanchangInfo, TithiInfo, VaarInfo,
    VarshaInfo, YogaInfo,
};
use dhruv_search::sankranti_types::{SankrantiConfig, SankrantiEvent};
use dhruv_search::{LunarPhaseEvent, SearchError};
use dhruv_time::{EopKernel, Epoch, UtcTime};
use dhruv_vedic_base::{
    AyanamshaSystem, Nakshatra28Info, NakshatraInfo, RashiInfo, ayanamsha_deg,
    jd_tdb_to_centuries, nakshatra28_from_longitude, nakshatra_from_longitude,
    rashi_from_longitude,
};
use dhruv_vedic_base::riseset_types::{GeoLocation, RiseSetConfig};

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
pub fn longitude(
    target: Body,
    observer: Observer,
    date: UtcDate,
) -> Result<f64, DhruvError> {
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
    Ok(results.into_iter().map(|r| r.map_err(DhruvError::from)).collect())
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
    let tropical = longitude(target, observer, date)?;
    let jd = utc_to_jd_tdb(date)?;
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
    dhruv_search::next_purnima(eng, &utc)?
        .ok_or(DhruvError::Search(SearchError::NoConvergence(
            "could not find next purnima",
        )))
}

/// Find the previous Purnima (full moon) before the given date.
pub fn prev_purnima(date: UtcDate) -> Result<LunarPhaseEvent, DhruvError> {
    let eng = engine()?;
    let utc: UtcTime = date.into();
    dhruv_search::prev_purnima(eng, &utc)?
        .ok_or(DhruvError::Search(SearchError::NoConvergence(
            "could not find previous purnima",
        )))
}

/// Find the next Amavasya (new moon) after the given date.
pub fn next_amavasya(date: UtcDate) -> Result<LunarPhaseEvent, DhruvError> {
    let eng = engine()?;
    let utc: UtcTime = date.into();
    dhruv_search::next_amavasya(eng, &utc)?
        .ok_or(DhruvError::Search(SearchError::NoConvergence(
            "could not find next amavasya",
        )))
}

/// Find the previous Amavasya (new moon) before the given date.
pub fn prev_amavasya(date: UtcDate) -> Result<LunarPhaseEvent, DhruvError> {
    let eng = engine()?;
    let utc: UtcTime = date.into();
    dhruv_search::prev_amavasya(eng, &utc)?
        .ok_or(DhruvError::Search(SearchError::NoConvergence(
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
    dhruv_search::next_sankranti(eng, &utc, &config)?
        .ok_or(DhruvError::Search(SearchError::NoConvergence(
            "could not find next sankranti",
        )))
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
    dhruv_search::prev_sankranti(eng, &utc, &config)?
        .ok_or(DhruvError::Search(SearchError::NoConvergence(
            "could not find previous sankranti",
        )))
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
    Ok(dhruv_search::vaar_for_date(eng, eop, &utc, location, &rs_config)?)
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
    Ok(dhruv_search::hora_for_date(eng, eop, &utc, location, &rs_config)?)
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
    Ok(dhruv_search::ghatika_for_date(eng, eop, &utc, location, &rs_config)?)
}

/// Compute all six daily panchang elements for a single moment.
///
/// Returns tithi, karana, yoga, vaar, hora, and ghatika efficiently
/// by sharing intermediate computations (body longitudes, sunrises).
pub fn panchang(
    date: UtcDate,
    eop: &EopKernel,
    location: &GeoLocation,
    system: AyanamshaSystem,
    use_nutation: bool,
) -> Result<PanchangInfo, DhruvError> {
    let eng = engine()?;
    let utc: UtcTime = date.into();
    let rs_config = RiseSetConfig::default();
    let config = SankrantiConfig::new(system, use_nutation);
    Ok(dhruv_search::panchang_for_date(eng, eop, &utc, location, &rs_config, &config)?)
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
    Ok(dhruv_search::graha_sidereal_longitudes(eng, jd, system, use_nutation)?)
}

/// Compute all 16 sphutas for the given inputs.
///
/// This is a thin wrapper over [`dhruv_vedic_base::all_sphutas`].
pub fn sphutas(
    inputs: &dhruv_vedic_base::SphutalInputs,
) -> [(dhruv_vedic_base::Sphuta, f64); 16] {
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
    Ok(dhruv_search::special_lagnas_for_date(eng, eop, &utc, location, &rs_config, &config)?)
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
    Ok(dhruv_search::arudha_padas_for_date(eng, eop, &utc, location, &bhava_config, &config)?)
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
    Ok(dhruv_search::ashtakavarga_for_date(eng, eop, &utc, location, &config)?)
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
    Ok(dhruv_search::all_upagrahas_for_date(eng, eop, &utc, location, &rs_config, &config)?)
}
