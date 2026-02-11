use dhruv_core::{Body, Frame, Observer, Query, StateVector};
use dhruv_frames::{
    SphericalCoords, SphericalState, cartesian_state_to_spherical_state, cartesian_to_spherical,
};
use dhruv_time::Epoch;

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
