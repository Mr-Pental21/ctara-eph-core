//! Conjunction/opposition/aspect search engine.
//!
//! Finds when two bodies reach a target ecliptic longitude difference.
//! Uses coarse scan + bisection on the angular difference function.
//!
//! Algorithm: pure numerical bisection on f(t) = normalize(lon1(t) - lon2(t) - target).
//! The normalize function wraps to [-180, +180] so zero-crossings correspond to
//! the target separation. Standard numerical root-finding; no external code referenced.

use dhruv_core::{Body, Engine, Frame, Observer, Query};
use dhruv_frames::{
    DEFAULT_PRECESSION_MODEL, PrecessionModel, ReferencePlane, cartesian_to_spherical,
    icrf_to_ecliptic, icrf_to_invariable, precess_ecliptic_j2000_to_date_with_model,
};

use crate::conjunction_types::{ConjunctionConfig, ConjunctionEvent, SearchDirection};
use crate::error::SearchError;
use crate::search_util::{is_genuine_crossing, normalize_to_pm180};

/// Maximum scan range in days (~800 days covers all synodic periods).
const MAX_SCAN_DAYS: f64 = 800.0;

/// Query a body's ecliptic-of-date longitude and latitude in degrees.
///
/// Queries ICRF/J2000, rotates to J2000 ecliptic, then applies the selected
/// 3D precession model to yield ecliptic-of-date coordinates. This is the primary
/// choke-point for all graha tropical longitudes.
pub fn body_ecliptic_lon_lat(
    engine: &Engine,
    body: Body,
    jd_tdb: f64,
) -> Result<(f64, f64), SearchError> {
    body_ecliptic_lon_lat_with_model(engine, body, jd_tdb, DEFAULT_PRECESSION_MODEL)
}

/// Model-aware variant of [`body_ecliptic_lon_lat`].
pub fn body_ecliptic_lon_lat_with_model(
    engine: &Engine,
    body: Body,
    jd_tdb: f64,
    precession_model: PrecessionModel,
) -> Result<(f64, f64), SearchError> {
    let query = Query {
        target: body,
        observer: Observer::Body(Body::Earth),
        frame: Frame::IcrfJ2000,
        epoch_tdb_jd: jd_tdb,
    };
    let state = engine.query(query)?;
    let ecl_j2000 = icrf_to_ecliptic(&state.position_km);
    let t = (jd_tdb - 2_451_545.0) / 36525.0;
    let ecl_date = precess_ecliptic_j2000_to_date_with_model(&ecl_j2000, t, precession_model);
    let sph = cartesian_to_spherical(&ecl_date);
    Ok((sph.lon_deg.rem_euclid(360.0), sph.lat_deg))
}

/// Query a body's longitude and latitude on the specified reference plane.
///
/// - `Ecliptic`: ICRF → ecliptic J2000 → precess to date → spherical (existing path).
/// - `Invariable`: ICRF → invariable plane → spherical (no precession needed).
pub fn body_lon_lat_on_plane(
    engine: &Engine,
    body: Body,
    jd_tdb: f64,
    precession_model: PrecessionModel,
    plane: ReferencePlane,
) -> Result<(f64, f64), SearchError> {
    match plane {
        ReferencePlane::Ecliptic => {
            body_ecliptic_lon_lat_with_model(engine, body, jd_tdb, precession_model)
        }
        ReferencePlane::Invariable => {
            let query = Query {
                target: body,
                observer: Observer::Body(Body::Earth),
                frame: Frame::IcrfJ2000,
                epoch_tdb_jd: jd_tdb,
            };
            let state = engine.query(query)?;
            let inv = icrf_to_invariable(&state.position_km);
            let sph = cartesian_to_spherical(&inv);
            Ok((sph.lon_deg.rem_euclid(360.0), sph.lat_deg))
        }
    }
}

/// Query a body's ecliptic-of-date longitude, latitude, and longitude speed.
///
/// Returns `(lon_deg, lat_deg, lon_speed_deg_per_day)`.
///
/// `lon_speed` is computed by finite-differencing fully-precessed of-date
/// longitudes at t±1 min. This correctly captures the full dP/dt·r term
/// (including dΠ_A/dt, dπ_A/dt, and latitude coupling) that a simple
/// `P·v_j2000 + scalar prec_rate` approximation would miss.
/// Requires 3 engine queries; acceptable since this path is not the hot path.
pub(crate) fn body_ecliptic_state(
    engine: &Engine,
    body: Body,
    jd_tdb: f64,
) -> Result<(f64, f64, f64), SearchError> {
    // Position at query epoch via full precession.
    let (lon, lat) = body_ecliptic_lon_lat(engine, body, jd_tdb)?;

    // lon_speed: finite-difference of of-date longitudes to capture all Ṗ·r terms.
    const DT: f64 = 1.0 / 1440.0; // 1-minute step in JD days
    let (lon_plus, _) = body_ecliptic_lon_lat(engine, body, jd_tdb + DT)?;
    let (lon_minus, _) = body_ecliptic_lon_lat(engine, body, jd_tdb - DT)?;
    let lon_speed = normalize_to_pm180(lon_plus - lon_minus) / (2.0 * DT);

    Ok((lon, lat, lon_speed))
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
#[allow(clippy::too_many_arguments)]
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
    find_event(
        engine,
        body1,
        body2,
        jd_tdb,
        SearchDirection::Forward,
        config,
    )
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
        return Err(SearchError::InvalidConfig("jd_end must be after jd_start"));
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
