//! Sankranti (Sun entering a new rashi) search engine.
//!
//! Finds when the Sun's sidereal longitude crosses a rashi boundary (multiples of 30 deg).
//! Uses coarse scan + bisection on the sidereal longitude difference function.
//!
//! Algorithm: numerical root-finding on f(t) = normalize(sun_sid(t) - boundary).
//! Clean-room implementation from standard astronomical conventions.

use dhruv_core::{Body, Engine};
use dhruv_time::UtcTime;
use dhruv_vedic_base::{ALL_RASHIS, Rashi, jd_tdb_to_centuries};

use crate::conjunction::{body_ecliptic_lon_lat, body_lon_lat_on_plane};
use crate::error::SearchError;
use crate::sankranti_types::{SankrantiConfig, SankrantiEvent};
use crate::search_util::{find_zero_crossing, normalize_to_pm180};

/// Maximum scan range in days (~400 days covers more than a full year).
const MAX_SCAN_DAYS: f64 = 400.0;

/// Get Sun's sidereal longitude at a given JD TDB.
///
/// Uses the reference plane configured in `config` for both the body longitude
/// and the ayanamsha, ensuring frame consistency.
fn sun_sidereal_longitude(
    engine: &Engine,
    jd_tdb: f64,
    config: &SankrantiConfig,
) -> Result<f64, SearchError> {
    let (lon, _lat) = body_lon_lat_on_plane(
        engine,
        Body::Sun,
        jd_tdb,
        config.precession_model,
        config.reference_plane,
    )?;
    let t = jd_tdb_to_centuries(jd_tdb);
    let aya = config.ayanamsha_deg_at_centuries(t);
    let sid = (lon - aya).rem_euclid(360.0);
    Ok(sid)
}

/// Find the next boundary (rashi cusp) that the Sun will cross.
///
/// Returns the boundary in degrees (a multiple of 30).
fn next_boundary(sidereal_lon: f64) -> f64 {
    let current_rashi = (sidereal_lon / 30.0).floor();
    ((current_rashi + 1.0) * 30.0) % 360.0
}

/// Find the previous boundary (rashi cusp) that the Sun last crossed.
///
/// Returns the boundary in degrees (a multiple of 30).
fn prev_boundary(sidereal_lon: f64) -> f64 {
    let current_rashi = (sidereal_lon / 30.0).floor();
    (current_rashi * 30.0) % 360.0
}

fn build_event(
    engine: &Engine,
    jd_tdb: f64,
    boundary_deg: f64,
    config: &SankrantiConfig,
) -> Result<SankrantiEvent, SearchError> {
    // sun_tropical_longitude_deg is ALWAYS ecliptic tropical (existing semantics).
    let (tropical_lon, _lat) = body_ecliptic_lon_lat(engine, Body::Sun, jd_tdb)?;
    // Sidereal via reference plane (may be invariable for Jagganatha).
    let (lon_on_plane, _) = body_lon_lat_on_plane(
        engine,
        Body::Sun,
        jd_tdb,
        config.precession_model,
        config.reference_plane,
    )?;
    let t = jd_tdb_to_centuries(jd_tdb);
    let aya = config.ayanamsha_deg_at_centuries(t);
    let sid = (lon_on_plane - aya).rem_euclid(360.0);

    // Rashi being entered = boundary / 30
    let rashi_index = ((boundary_deg / 30.0).round() as u8) % 12;
    let rashi = ALL_RASHIS[rashi_index as usize];

    Ok(SankrantiEvent {
        utc: UtcTime::from_jd_tdb(jd_tdb, engine.lsk()),
        rashi,
        rashi_index,
        sun_sidereal_longitude_deg: sid,
        sun_tropical_longitude_deg: tropical_lon,
    })
}

/// Find the next Sankranti (Sun entering any rashi) after the given UTC time.
pub fn next_sankranti(
    engine: &Engine,
    utc: &UtcTime,
    config: &SankrantiConfig,
) -> Result<Option<SankrantiEvent>, SearchError> {
    config.validate().map_err(SearchError::InvalidConfig)?;

    let jd = utc.to_jd_tdb(engine.lsk());
    let sid_lon = sun_sidereal_longitude(engine, jd, config)?;
    let boundary = next_boundary(sid_lon);

    // Sun moves ~0.986 deg/day in sidereal
    let deg_to_go = (boundary - sid_lon).rem_euclid(360.0);
    let estimate_days = deg_to_go / 0.986;
    // Start search a bit before the estimate
    let search_start = jd + estimate_days.max(0.5) - 2.0;

    let max_steps = (MAX_SCAN_DAYS / config.step_size_days).ceil() as usize;

    let f = |t: f64| -> Result<f64, SearchError> {
        let sid = sun_sidereal_longitude(engine, t, config)?;
        Ok(normalize_to_pm180(sid - boundary))
    };

    let result = find_zero_crossing(
        &f,
        search_start,
        config.step_size_days,
        max_steps,
        config.max_iterations,
        config.convergence_days,
    )?;

    match result {
        Some(t) if t >= jd => Ok(Some(build_event(engine, t, boundary, config)?)),
        // If we found a crossing before jd (shouldn't happen often), retry
        Some(_) => {
            let result2 = find_zero_crossing(
                &f,
                jd,
                config.step_size_days,
                max_steps,
                config.max_iterations,
                config.convergence_days,
            )?;
            match result2 {
                Some(t2) => Ok(Some(build_event(engine, t2, boundary, config)?)),
                None => Ok(None),
            }
        }
        None => Ok(None),
    }
}

/// Find the previous Sankranti (Sun entering any rashi) before the given UTC time.
pub fn prev_sankranti(
    engine: &Engine,
    utc: &UtcTime,
    config: &SankrantiConfig,
) -> Result<Option<SankrantiEvent>, SearchError> {
    config.validate().map_err(SearchError::InvalidConfig)?;

    let jd = utc.to_jd_tdb(engine.lsk());
    let sid_lon = sun_sidereal_longitude(engine, jd, config)?;
    let boundary = prev_boundary(sid_lon);

    // Sun moves ~0.986 deg/day backwards in time
    let deg_since = (sid_lon - boundary).rem_euclid(360.0);
    let estimate_days = deg_since / 0.986;
    let search_start = jd - estimate_days.max(0.5) + 2.0;

    let max_steps = (MAX_SCAN_DAYS / config.step_size_days).ceil() as usize;

    let f = |t: f64| -> Result<f64, SearchError> {
        let sid = sun_sidereal_longitude(engine, t, config)?;
        Ok(normalize_to_pm180(sid - boundary))
    };

    let result = find_zero_crossing(
        &f,
        search_start,
        -config.step_size_days,
        max_steps,
        config.max_iterations,
        config.convergence_days,
    )?;

    match result {
        Some(t) if t <= jd => Ok(Some(build_event(engine, t, boundary, config)?)),
        Some(_) => {
            let result2 = find_zero_crossing(
                &f,
                jd,
                -config.step_size_days,
                max_steps,
                config.max_iterations,
                config.convergence_days,
            )?;
            match result2 {
                Some(t2) => Ok(Some(build_event(engine, t2, boundary, config)?)),
                None => Ok(None),
            }
        }
        None => Ok(None),
    }
}

/// Search for all Sankrantis in a UTC time range.
pub fn search_sankrantis(
    engine: &Engine,
    start: &UtcTime,
    end: &UtcTime,
    config: &SankrantiConfig,
) -> Result<Vec<SankrantiEvent>, SearchError> {
    config.validate().map_err(SearchError::InvalidConfig)?;

    let jd_start = start.to_jd_tdb(engine.lsk());
    let jd_end = end.to_jd_tdb(engine.lsk());

    if jd_end <= jd_start {
        return Err(SearchError::InvalidConfig("end must be after start"));
    }

    let mut events = Vec::new();
    let mut cursor = *start;

    // Find sankrantis iteratively
    while let Some(event) = next_sankranti(engine, &cursor, config)? {
        let event_jd = event.utc.to_jd_tdb(engine.lsk());
        if event_jd > jd_end {
            break;
        }
        // Advance cursor slightly past this event
        cursor = UtcTime::from_jd_tdb(event_jd + 0.01, engine.lsk());
        events.push(event);
    }

    Ok(events)
}

/// Find the next time the Sun enters a specific rashi.
pub fn next_specific_sankranti(
    engine: &Engine,
    utc: &UtcTime,
    rashi: Rashi,
    config: &SankrantiConfig,
) -> Result<Option<SankrantiEvent>, SearchError> {
    config.validate().map_err(SearchError::InvalidConfig)?;

    let jd = utc.to_jd_tdb(engine.lsk());
    let boundary = rashi.index() as f64 * 30.0;

    let max_steps = (MAX_SCAN_DAYS / config.step_size_days).ceil() as usize;

    let f = |t: f64| -> Result<f64, SearchError> {
        let sid = sun_sidereal_longitude(engine, t, config)?;
        Ok(normalize_to_pm180(sid - boundary))
    };

    let result = find_zero_crossing(
        &f,
        jd,
        config.step_size_days,
        max_steps,
        config.max_iterations,
        config.convergence_days,
    )?;

    match result {
        Some(t) => Ok(Some(build_event(engine, t, boundary, config)?)),
        None => Ok(None),
    }
}

/// Find the previous time the Sun entered a specific rashi.
pub fn prev_specific_sankranti(
    engine: &Engine,
    utc: &UtcTime,
    rashi: Rashi,
    config: &SankrantiConfig,
) -> Result<Option<SankrantiEvent>, SearchError> {
    config.validate().map_err(SearchError::InvalidConfig)?;

    let jd = utc.to_jd_tdb(engine.lsk());
    let boundary = rashi.index() as f64 * 30.0;

    let max_steps = (MAX_SCAN_DAYS / config.step_size_days).ceil() as usize;

    let f = |t: f64| -> Result<f64, SearchError> {
        let sid = sun_sidereal_longitude(engine, t, config)?;
        Ok(normalize_to_pm180(sid - boundary))
    };

    let result = find_zero_crossing(
        &f,
        jd,
        -config.step_size_days,
        max_steps,
        config.max_iterations,
        config.convergence_days,
    )?;

    match result {
        Some(t) => Ok(Some(build_event(engine, t, boundary, config)?)),
        None => Ok(None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dhruv_frames::DEFAULT_PRECESSION_MODEL;
    use dhruv_vedic_base::AyanamshaSystem;

    #[test]
    fn next_boundary_basic() {
        assert!((next_boundary(10.0) - 30.0).abs() < 1e-10);
        assert!((next_boundary(45.0) - 60.0).abs() < 1e-10);
        assert!((next_boundary(350.0) - 0.0).abs() < 1e-10);
    }

    #[test]
    fn prev_boundary_basic() {
        assert!((prev_boundary(10.0) - 0.0).abs() < 1e-10);
        assert!((prev_boundary(45.0) - 30.0).abs() < 1e-10);
        assert!((prev_boundary(350.0) - 330.0).abs() < 1e-10);
    }

    #[test]
    fn config_validates() {
        let c = SankrantiConfig::default_lahiri();
        assert!(c.validate().is_ok());
    }

    #[test]
    fn config_rejects_zero_step() {
        let mut c = SankrantiConfig::default_lahiri();
        c.step_size_days = 0.0;
        assert!(c.validate().is_err());
    }

    #[test]
    fn config_rejects_zero_iterations() {
        let mut c = SankrantiConfig::default_lahiri();
        c.max_iterations = 0;
        assert!(c.validate().is_err());
    }

    #[test]
    fn default_lahiri_config() {
        let c = SankrantiConfig::default_lahiri();
        assert_eq!(c.ayanamsha_system, AyanamshaSystem::Lahiri);
        assert!(!c.use_nutation);
        assert_eq!(c.precession_model, DEFAULT_PRECESSION_MODEL);
    }
}
