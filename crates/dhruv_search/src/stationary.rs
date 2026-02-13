//! Stationary point and max-speed search engine.
//!
//! **Stationary search**: finds when a planet's ecliptic longitude velocity
//! crosses zero (station retrograde / station direct).
//!
//! **Max speed search**: finds when a planet's ecliptic longitude velocity
//! reaches a local extremum (acceleration crosses zero).
//!
//! Both use the same coarse-scan + bisection pattern as the conjunction engine.
//!
//! Algorithm: pure numerical bisection on f(t) = lon_speed(t) for stationary,
//! and g(t) = (lon_speed(t+h) - lon_speed(t-h)) / (2h) for max speed.
//! Standard numerical root-finding; no external code referenced.
//! See docs/clean_room_stationary.md for provenance.

use dhruv_core::{Body, Engine};

use crate::conjunction::body_ecliptic_state;
use crate::conjunction_types::SearchDirection;
use crate::error::SearchError;
use crate::stationary_types::{
    MaxSpeedEvent, MaxSpeedType, StationType, StationaryConfig, StationaryEvent,
};

/// Maximum scan range in days (~800 days covers all synodic periods).
const MAX_SCAN_DAYS: f64 = 800.0;

// ---------------------------------------------------------------------------
// Body validation
// ---------------------------------------------------------------------------

/// Bodies that cannot have stationary points (they don't go retrograde geocentrically).
/// Sun: always moves eastward along the ecliptic.
/// Moon: always moves eastward (geocentrically).
/// Earth: we observe from Earth, so its geocentric longitude is undefined.
fn validate_stationary_body(body: Body) -> Result<(), SearchError> {
    match body {
        Body::Sun | Body::Moon | Body::Earth => Err(SearchError::InvalidConfig(
            "Sun, Moon, and Earth do not have stationary points",
        )),
        _ => Ok(()),
    }
}

/// Bodies invalid for max-speed search.
/// Earth: we observe from Earth.
fn validate_max_speed_body(body: Body) -> Result<(), SearchError> {
    if body == Body::Earth {
        Err(SearchError::InvalidConfig(
            "Earth cannot be searched from Earth observer",
        ))
    } else {
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Generic bisection solver
// ---------------------------------------------------------------------------

/// Bisect to find the zero crossing of a scalar function between t_a and t_b.
///
/// `f_at` evaluates the function at a given time.
/// Returns the refined time of the zero crossing.
fn bisect_zero<F>(
    mut t_a: f64,
    mut f_a: f64,
    mut t_b: f64,
    _f_b: f64,
    max_iter: u32,
    convergence_days: f64,
    f_at: &F,
) -> Result<f64, SearchError>
where
    F: Fn(f64) -> Result<f64, SearchError>,
{
    for _ in 0..max_iter {
        let t_mid = 0.5 * (t_a + t_b);
        let f_mid = f_at(t_mid)?;

        if f_a * f_mid <= 0.0 {
            t_b = t_mid;
        } else {
            t_a = t_mid;
            f_a = f_mid;
        }

        if (t_b - t_a).abs() < convergence_days {
            break;
        }
    }

    Ok(0.5 * (t_a + t_b))
}

// ---------------------------------------------------------------------------
// Stationary point search (velocity = 0)
// ---------------------------------------------------------------------------

/// Find a single stationary event by coarse scan for speed sign change, then bisect.
fn find_stationary_event(
    engine: &Engine,
    body: Body,
    jd_start: f64,
    direction: SearchDirection,
    config: &StationaryConfig,
) -> Result<Option<StationaryEvent>, SearchError> {
    config.validate().map_err(SearchError::InvalidConfig)?;
    validate_stationary_body(body)?;

    let step = match direction {
        SearchDirection::Forward => config.step_size_days,
        SearchDirection::Backward => -config.step_size_days,
    };

    let max_steps = (MAX_SCAN_DAYS / config.step_size_days).ceil() as usize;

    let (_, _, mut v_prev) = body_ecliptic_state(engine, body, jd_start)?;
    let mut t_prev = jd_start;

    for _ in 0..max_steps {
        let t_curr = t_prev + step;
        let (_, _, v_curr) = body_ecliptic_state(engine, body, t_curr)?;

        // Check for sign change in velocity
        if v_prev * v_curr < 0.0 {
            // Ensure t_a < t_b for bisection
            let (t_a, v_a, t_b, v_b) = if t_prev < t_curr {
                (t_prev, v_prev, t_curr, v_curr)
            } else {
                (t_curr, v_curr, t_prev, v_prev)
            };

            let speed_at = |t: f64| -> Result<f64, SearchError> {
                let (_, _, v) = body_ecliptic_state(engine, body, t)?;
                Ok(v)
            };

            let t_station = bisect_zero(
                t_a,
                v_a,
                t_b,
                v_b,
                config.max_iterations,
                config.convergence_days,
                &speed_at,
            )?;

            let (lon, lat, _) = body_ecliptic_state(engine, body, t_station)?;

            // Classify: positive→negative = StationRetrograde, negative→positive = StationDirect
            let station_type = if v_a > 0.0 {
                StationType::StationRetrograde
            } else {
                StationType::StationDirect
            };

            return Ok(Some(StationaryEvent {
                jd_tdb: t_station,
                body,
                longitude_deg: lon,
                latitude_deg: lat,
                station_type,
            }));
        }

        t_prev = t_curr;
        v_prev = v_curr;
    }

    Ok(None)
}

/// Find the next stationary point after `jd_tdb`.
pub fn next_stationary(
    engine: &Engine,
    body: Body,
    jd_tdb: f64,
    config: &StationaryConfig,
) -> Result<Option<StationaryEvent>, SearchError> {
    find_stationary_event(engine, body, jd_tdb, SearchDirection::Forward, config)
}

/// Find the previous stationary point before `jd_tdb`.
pub fn prev_stationary(
    engine: &Engine,
    body: Body,
    jd_tdb: f64,
    config: &StationaryConfig,
) -> Result<Option<StationaryEvent>, SearchError> {
    find_stationary_event(engine, body, jd_tdb, SearchDirection::Backward, config)
}

/// Search for all stationary points in a time range.
pub fn search_stationary(
    engine: &Engine,
    body: Body,
    jd_start: f64,
    jd_end: f64,
    config: &StationaryConfig,
) -> Result<Vec<StationaryEvent>, SearchError> {
    config.validate().map_err(SearchError::InvalidConfig)?;
    validate_stationary_body(body)?;

    if jd_end <= jd_start {
        return Err(SearchError::InvalidConfig("jd_end must be after jd_start"));
    }

    let mut events = Vec::new();
    let step = config.step_size_days;

    let (_, _, mut v_prev) = body_ecliptic_state(engine, body, jd_start)?;
    let mut t_prev = jd_start;

    loop {
        let t_curr = (t_prev + step).min(jd_end);
        let (_, _, v_curr) = body_ecliptic_state(engine, body, t_curr)?;

        if v_prev * v_curr < 0.0 {
            let speed_at = |t: f64| -> Result<f64, SearchError> {
                let (_, _, v) = body_ecliptic_state(engine, body, t)?;
                Ok(v)
            };

            let t_station = bisect_zero(
                t_prev,
                v_prev,
                t_curr,
                v_curr,
                config.max_iterations,
                config.convergence_days,
                &speed_at,
            )?;

            if t_station >= jd_start && t_station <= jd_end {
                let (lon, lat, _) = body_ecliptic_state(engine, body, t_station)?;

                let station_type = if v_prev > 0.0 {
                    StationType::StationRetrograde
                } else {
                    StationType::StationDirect
                };

                events.push(StationaryEvent {
                    jd_tdb: t_station,
                    body,
                    longitude_deg: lon,
                    latitude_deg: lat,
                    station_type,
                });
            }
        }

        if t_curr >= jd_end {
            break;
        }

        t_prev = t_curr;
        v_prev = v_curr;
    }

    Ok(events)
}

// ---------------------------------------------------------------------------
// Max speed search (acceleration = 0, i.e. velocity extremum)
// ---------------------------------------------------------------------------

/// Numerical acceleration via central difference: (v(t+h) - v(t-h)) / (2h).
fn numerical_acceleration(engine: &Engine, body: Body, t: f64, h: f64) -> Result<f64, SearchError> {
    let (_, _, v_plus) = body_ecliptic_state(engine, body, t + h)?;
    let (_, _, v_minus) = body_ecliptic_state(engine, body, t - h)?;
    Ok((v_plus - v_minus) / (2.0 * h))
}

/// Find a single max-speed event by coarse scan for acceleration sign change, then bisect.
fn find_max_speed_event(
    engine: &Engine,
    body: Body,
    jd_start: f64,
    direction: SearchDirection,
    config: &StationaryConfig,
) -> Result<Option<MaxSpeedEvent>, SearchError> {
    config.validate().map_err(SearchError::InvalidConfig)?;
    validate_max_speed_body(body)?;

    let step = match direction {
        SearchDirection::Forward => config.step_size_days,
        SearchDirection::Backward => -config.step_size_days,
    };

    let h = config.numerical_step_days;
    let max_steps = (MAX_SCAN_DAYS / config.step_size_days).ceil() as usize;

    let mut a_prev = numerical_acceleration(engine, body, jd_start, h)?;
    let mut t_prev = jd_start;

    for _ in 0..max_steps {
        let t_curr = t_prev + step;
        let a_curr = numerical_acceleration(engine, body, t_curr, h)?;

        if a_prev * a_curr < 0.0 {
            let (t_a, a_a, t_b, a_b) = if t_prev < t_curr {
                (t_prev, a_prev, t_curr, a_curr)
            } else {
                (t_curr, a_curr, t_prev, a_prev)
            };

            let accel_at =
                |t: f64| -> Result<f64, SearchError> { numerical_acceleration(engine, body, t, h) };

            let t_peak = bisect_zero(
                t_a,
                a_a,
                t_b,
                a_b,
                config.max_iterations,
                config.convergence_days,
                &accel_at,
            )?;

            let (lon, lat, speed) = body_ecliptic_state(engine, body, t_peak)?;

            let speed_type = if speed >= 0.0 {
                MaxSpeedType::MaxDirect
            } else {
                MaxSpeedType::MaxRetrograde
            };

            return Ok(Some(MaxSpeedEvent {
                jd_tdb: t_peak,
                body,
                longitude_deg: lon,
                latitude_deg: lat,
                speed_deg_per_day: speed,
                speed_type,
            }));
        }

        t_prev = t_curr;
        a_prev = a_curr;
    }

    Ok(None)
}

/// Find the next max-speed event after `jd_tdb`.
pub fn next_max_speed(
    engine: &Engine,
    body: Body,
    jd_tdb: f64,
    config: &StationaryConfig,
) -> Result<Option<MaxSpeedEvent>, SearchError> {
    find_max_speed_event(engine, body, jd_tdb, SearchDirection::Forward, config)
}

/// Find the previous max-speed event before `jd_tdb`.
pub fn prev_max_speed(
    engine: &Engine,
    body: Body,
    jd_tdb: f64,
    config: &StationaryConfig,
) -> Result<Option<MaxSpeedEvent>, SearchError> {
    find_max_speed_event(engine, body, jd_tdb, SearchDirection::Backward, config)
}

/// Search for all max-speed events in a time range.
pub fn search_max_speed(
    engine: &Engine,
    body: Body,
    jd_start: f64,
    jd_end: f64,
    config: &StationaryConfig,
) -> Result<Vec<MaxSpeedEvent>, SearchError> {
    config.validate().map_err(SearchError::InvalidConfig)?;
    validate_max_speed_body(body)?;

    if jd_end <= jd_start {
        return Err(SearchError::InvalidConfig("jd_end must be after jd_start"));
    }

    let mut events = Vec::new();
    let step = config.step_size_days;
    let h = config.numerical_step_days;

    let mut a_prev = numerical_acceleration(engine, body, jd_start, h)?;
    let mut t_prev = jd_start;

    loop {
        let t_curr = (t_prev + step).min(jd_end);
        let a_curr = numerical_acceleration(engine, body, t_curr, h)?;

        if a_prev * a_curr < 0.0 {
            let accel_at =
                |t: f64| -> Result<f64, SearchError> { numerical_acceleration(engine, body, t, h) };

            let t_peak = bisect_zero(
                t_prev,
                a_prev,
                t_curr,
                a_curr,
                config.max_iterations,
                config.convergence_days,
                &accel_at,
            )?;

            if t_peak >= jd_start && t_peak <= jd_end {
                let (lon, lat, speed) = body_ecliptic_state(engine, body, t_peak)?;

                let speed_type = if speed >= 0.0 {
                    MaxSpeedType::MaxDirect
                } else {
                    MaxSpeedType::MaxRetrograde
                };

                events.push(MaxSpeedEvent {
                    jd_tdb: t_peak,
                    body,
                    longitude_deg: lon,
                    latitude_deg: lat,
                    speed_deg_per_day: speed,
                    speed_type,
                });
            }
        }

        if t_curr >= jd_end {
            break;
        }

        t_prev = t_curr;
        a_prev = a_curr;
    }

    Ok(events)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sun_rejected_for_stationary() {
        assert!(validate_stationary_body(Body::Sun).is_err());
    }

    #[test]
    fn moon_rejected_for_stationary() {
        assert!(validate_stationary_body(Body::Moon).is_err());
    }

    #[test]
    fn earth_rejected_for_stationary() {
        assert!(validate_stationary_body(Body::Earth).is_err());
    }

    #[test]
    fn mercury_allowed_for_stationary() {
        assert!(validate_stationary_body(Body::Mercury).is_ok());
    }

    #[test]
    fn mars_allowed_for_stationary() {
        assert!(validate_stationary_body(Body::Mars).is_ok());
    }

    #[test]
    fn earth_rejected_for_max_speed() {
        assert!(validate_max_speed_body(Body::Earth).is_err());
    }

    #[test]
    fn sun_allowed_for_max_speed() {
        assert!(validate_max_speed_body(Body::Sun).is_ok());
    }

    #[test]
    fn moon_allowed_for_max_speed() {
        assert!(validate_max_speed_body(Body::Moon).is_ok());
    }

    #[test]
    fn mercury_allowed_for_max_speed() {
        assert!(validate_max_speed_body(Body::Mercury).is_ok());
    }
}
