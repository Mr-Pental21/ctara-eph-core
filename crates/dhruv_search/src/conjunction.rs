//! Conjunction/opposition/aspect search engine.
//!
//! Finds when two bodies reach a target ecliptic longitude difference.
//! Uses coarse scan + bisection on the angular difference function.
//!
//! Algorithm: pure numerical bisection on f(t) = normalize(lon1(t) - lon2(t) - target).
//! The normalize function wraps to [-180, +180] so zero-crossings correspond to
//! the target separation. Standard numerical root-finding; no external code referenced.

use dhruv_core::{Body, Engine, Frame, Observer, Query};
use dhruv_frames::{cartesian_state_to_spherical_state, cartesian_to_spherical, icrf_to_ecliptic};

use crate::conjunction_types::{ConjunctionConfig, ConjunctionEvent, SearchDirection};
use crate::error::SearchError;

/// Maximum scan range in days (~800 days covers all synodic periods).
const MAX_SCAN_DAYS: f64 = 800.0;

/// Query a body's ecliptic longitude and latitude in degrees.
pub(crate) fn body_ecliptic_lon_lat(
    engine: &Engine,
    body: Body,
    jd_tdb: f64,
) -> Result<(f64, f64), SearchError> {
    let query = Query {
        target: body,
        observer: Observer::Body(Body::Earth),
        frame: Frame::IcrfJ2000,
        epoch_tdb_jd: jd_tdb,
    };
    let state = engine.query(query)?;
    let ecl = icrf_to_ecliptic(&state.position_km);
    let sph = cartesian_to_spherical(&ecl);
    Ok((sph.lon_deg, sph.lat_deg))
}

/// Query a body's ecliptic longitude, latitude, and longitude speed.
///
/// Returns `(lon_deg, lat_deg, lon_speed_deg_per_day)`.
/// Uses `Frame::EclipticJ2000` so the engine rotates both position and velocity.
pub(crate) fn body_ecliptic_state(
    engine: &Engine,
    body: Body,
    jd_tdb: f64,
) -> Result<(f64, f64, f64), SearchError> {
    let query = Query {
        target: body,
        observer: Observer::Body(Body::Earth),
        frame: Frame::EclipticJ2000,
        epoch_tdb_jd: jd_tdb,
    };
    let state = engine.query(query)?;
    let sph = cartesian_state_to_spherical_state(&state.position_km, &state.velocity_km_s);
    Ok((sph.lon_deg.rem_euclid(360.0), sph.lat_deg, sph.lon_speed))
}

/// Normalize an angle to [-180, +180].
pub(crate) fn normalize_to_pm180(deg: f64) -> f64 {
    let mut d = deg % 360.0;
    if d > 180.0 {
        d -= 360.0;
    } else if d <= -180.0 {
        d += 360.0;
    }
    d
}

/// Compute the separation function f(t) = normalize(lon1 - lon2 - target).
/// Returns (f_val, lon1, lon2, lat1, lat2).
fn separation_function(
    engine: &Engine,
    body1: Body,
    body2: Body,
    target_deg: f64,
    jd_tdb: f64,
) -> Result<(f64, f64, f64, f64, f64), SearchError> {
    let (lon1, lat1) = body_ecliptic_lon_lat(engine, body1, jd_tdb)?;
    let (lon2, lat2) = body_ecliptic_lon_lat(engine, body2, jd_tdb)?;
    let f = normalize_to_pm180(lon1 - lon2 - target_deg);
    Ok((f, lon1, lon2, lat1, lat2))
}

/// Check if a sign change is a genuine zero crossing vs a wrap-around discontinuity.
///
/// When the normalized function jumps from ~+180 to ~-180 (or vice versa),
/// the product is negative but it's not a real zero crossing. A genuine
/// crossing has both values relatively small in magnitude.
fn is_genuine_crossing(f_a: f64, f_b: f64) -> bool {
    f_a * f_b < 0.0 && (f_a - f_b).abs() < 270.0
}

/// Compute actual_separation_deg closest to the target.
///
/// Returns the raw (lon1 - lon2) mod 360 value closest to `target_deg`.
/// This avoids the 0°/360° ambiguity: for target=0°, a result of -0.001°
/// is reported as ~0° rather than ~360°.
fn compute_actual_separation(lon1: f64, lon2: f64, target_deg: f64) -> f64 {
    let raw = ((lon1 - lon2) % 360.0 + 360.0) % 360.0; // [0, 360)
    let delta = normalize_to_pm180(raw - target_deg);
    target_deg + delta
}

/// Bisect to refine the zero crossing between t_a and t_b.
fn bisect_refinement(
    engine: &Engine,
    body1: Body,
    body2: Body,
    target_deg: f64,
    mut t_a: f64,
    mut f_a: f64,
    mut t_b: f64,
    _f_b: f64,
    config: &ConjunctionConfig,
) -> Result<ConjunctionEvent, SearchError> {
    let mut lon1 = 0.0;
    let mut lon2 = 0.0;
    let mut lat1 = 0.0;
    let mut lat2 = 0.0;

    for _ in 0..config.max_iterations {
        let t_mid = 0.5 * (t_a + t_b);
        let (f_mid, l1, l2, la1, la2) =
            separation_function(engine, body1, body2, target_deg, t_mid)?;

        lon1 = l1;
        lon2 = l2;
        lat1 = la1;
        lat2 = la2;

        if f_a * f_mid <= 0.0 {
            t_b = t_mid;
        } else {
            t_a = t_mid;
            f_a = f_mid;
        }

        if (t_b - t_a).abs() < config.convergence_days {
            break;
        }
    }

    let t_final = 0.5 * (t_a + t_b);
    let actual_sep = compute_actual_separation(lon1, lon2, target_deg);

    Ok(ConjunctionEvent {
        jd_tdb: t_final,
        actual_separation_deg: actual_sep,
        body1_longitude_deg: lon1,
        body2_longitude_deg: lon2,
        body1_latitude_deg: lat1,
        body2_latitude_deg: lat2,
        body1,
        body2,
    })
}

/// Find the next or previous conjunction/aspect event.
fn find_event(
    engine: &Engine,
    body1: Body,
    body2: Body,
    jd_start: f64,
    direction: SearchDirection,
    config: &ConjunctionConfig,
) -> Result<Option<ConjunctionEvent>, SearchError> {
    config.validate().map_err(SearchError::InvalidConfig)?;

    let step = match direction {
        SearchDirection::Forward => config.step_size_days,
        SearchDirection::Backward => -config.step_size_days,
    };

    let max_steps = (MAX_SCAN_DAYS / config.step_size_days).ceil() as usize;

    let (mut f_prev, _, _, _, _) =
        separation_function(engine, body1, body2, config.target_separation_deg, jd_start)?;
    let mut t_prev = jd_start;

    for _ in 0..max_steps {
        let t_curr = t_prev + step;
        let (f_curr, _, _, _, _) =
            separation_function(engine, body1, body2, config.target_separation_deg, t_curr)?;

        // Check for genuine zero crossing (not a wrap-around discontinuity)
        if is_genuine_crossing(f_prev, f_curr) {
            // Ensure t_a < t_b for bisection
            let (t_a, f_a, t_b, f_b) = if t_prev < t_curr {
                (t_prev, f_prev, t_curr, f_curr)
            } else {
                (t_curr, f_curr, t_prev, f_prev)
            };
            let event = bisect_refinement(
                engine,
                body1,
                body2,
                config.target_separation_deg,
                t_a,
                f_a,
                t_b,
                f_b,
                config,
            )?;
            return Ok(Some(event));
        }

        t_prev = t_curr;
        f_prev = f_curr;
    }

    Ok(None)
}

/// Find the next conjunction/aspect event after `jd_tdb`.
pub fn next_conjunction(
    engine: &Engine,
    body1: Body,
    body2: Body,
    jd_tdb: f64,
    config: &ConjunctionConfig,
) -> Result<Option<ConjunctionEvent>, SearchError> {
    find_event(engine, body1, body2, jd_tdb, SearchDirection::Forward, config)
}

/// Find the previous conjunction/aspect event before `jd_tdb`.
pub fn prev_conjunction(
    engine: &Engine,
    body1: Body,
    body2: Body,
    jd_tdb: f64,
    config: &ConjunctionConfig,
) -> Result<Option<ConjunctionEvent>, SearchError> {
    find_event(
        engine,
        body1,
        body2,
        jd_tdb,
        SearchDirection::Backward,
        config,
    )
}

/// Search for all conjunction/aspect events in a time range.
pub fn search_conjunctions(
    engine: &Engine,
    body1: Body,
    body2: Body,
    jd_start: f64,
    jd_end: f64,
    config: &ConjunctionConfig,
) -> Result<Vec<ConjunctionEvent>, SearchError> {
    config.validate().map_err(SearchError::InvalidConfig)?;

    if jd_end <= jd_start {
        return Err(SearchError::InvalidConfig(
            "jd_end must be after jd_start",
        ));
    }

    let mut events = Vec::new();
    let step = config.step_size_days;

    let (mut f_prev, _, _, _, _) =
        separation_function(engine, body1, body2, config.target_separation_deg, jd_start)?;
    let mut t_prev = jd_start;

    loop {
        let t_curr = (t_prev + step).min(jd_end);
        let (f_curr, _, _, _, _) =
            separation_function(engine, body1, body2, config.target_separation_deg, t_curr)?;

        if is_genuine_crossing(f_prev, f_curr) {
            let event = bisect_refinement(
                engine,
                body1,
                body2,
                config.target_separation_deg,
                t_prev,
                f_prev,
                t_curr,
                f_curr,
                config,
            )?;
            if event.jd_tdb >= jd_start && event.jd_tdb <= jd_end {
                events.push(event);
            }
        }

        if t_curr >= jd_end {
            break;
        }

        t_prev = t_curr;
        f_prev = f_curr;
    }

    Ok(events)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_basic() {
        assert!((normalize_to_pm180(0.0) - 0.0).abs() < 1e-10);
        assert!((normalize_to_pm180(180.0) - 180.0).abs() < 1e-10);
        assert!((normalize_to_pm180(-180.0) - 180.0).abs() < 1e-10);
        assert!((normalize_to_pm180(270.0) - (-90.0)).abs() < 1e-10);
        assert!((normalize_to_pm180(-270.0) - 90.0).abs() < 1e-10);
        assert!((normalize_to_pm180(360.0) - 0.0).abs() < 1e-10);
        assert!((normalize_to_pm180(450.0) - 90.0).abs() < 1e-10);
    }

    #[test]
    fn genuine_crossing_positive() {
        assert!(is_genuine_crossing(5.0, -3.0));
        assert!(is_genuine_crossing(-10.0, 10.0));
    }

    #[test]
    fn wraparound_rejected() {
        // +170 to -170 is a 340° jump — wrap-around, not crossing
        assert!(!is_genuine_crossing(170.0, -170.0));
        assert!(!is_genuine_crossing(-170.0, 170.0));
    }

    #[test]
    fn actual_sep_near_zero() {
        // lon1 slightly less than lon2 → raw ≈ 359.999°, target=0 → report ~0
        let sep = compute_actual_separation(100.0, 100.0001, 0.0);
        assert!(sep < 0.01, "sep = {sep}");
    }

    #[test]
    fn actual_sep_opposition() {
        let sep = compute_actual_separation(280.0, 100.0, 180.0);
        assert!((sep - 180.0).abs() < 0.01, "sep = {sep}");
    }

    #[test]
    fn config_conjunction_defaults() {
        let c = ConjunctionConfig::conjunction(0.5);
        assert!((c.target_separation_deg - 0.0).abs() < 1e-10);
        assert!((c.step_size_days - 0.5).abs() < 1e-10);
        assert_eq!(c.max_iterations, 50);
        assert!(c.validate().is_ok());
    }

    #[test]
    fn config_opposition_defaults() {
        let c = ConjunctionConfig::opposition(1.0);
        assert!((c.target_separation_deg - 180.0).abs() < 1e-10);
        assert!(c.validate().is_ok());
    }

    #[test]
    fn config_aspect_90() {
        let c = ConjunctionConfig::aspect(90.0, 1.0);
        assert!((c.target_separation_deg - 90.0).abs() < 1e-10);
        assert!(c.validate().is_ok());
    }

    #[test]
    fn config_rejects_negative_target() {
        let mut c = ConjunctionConfig::conjunction(1.0);
        c.target_separation_deg = -10.0;
        assert!(c.validate().is_err());
    }

    #[test]
    fn config_rejects_360_target() {
        let mut c = ConjunctionConfig::conjunction(1.0);
        c.target_separation_deg = 360.0;
        assert!(c.validate().is_err());
    }

    #[test]
    fn config_rejects_zero_step() {
        let c = ConjunctionConfig::conjunction(0.0);
        assert!(c.validate().is_err());
    }

    #[test]
    fn config_rejects_zero_iterations() {
        let mut c = ConjunctionConfig::conjunction(1.0);
        c.max_iterations = 0;
        assert!(c.validate().is_err());
    }
}
