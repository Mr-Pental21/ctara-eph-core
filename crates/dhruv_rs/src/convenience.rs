use dhruv_core::{Body, Frame, Observer, Query, StateVector};
use dhruv_frames::{
    SphericalCoords, SphericalState, cartesian_state_to_spherical_state, cartesian_to_spherical,
};
use dhruv_time::Epoch;
use dhruv_vedic_base::{
    AyanamshaSystem, Nakshatra28Info, NakshatraInfo, RashiInfo, ayanamsha_deg,
    jd_tdb_to_centuries, nakshatra28_from_longitude, nakshatra_from_longitude,
    rashi_from_longitude,
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
