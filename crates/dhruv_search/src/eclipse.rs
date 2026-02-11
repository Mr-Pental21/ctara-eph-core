//! Eclipse computation: lunar (penumbral/partial/total) and solar (geocentric).
//!
//! Builds on the conjunction engine to find new/full moons, then applies
//! shadow geometry for classification, magnitude, and contact times.
//!
//! Lunar eclipse algorithm:
//!   1. Find full moons (Sun-Moon opposition, 180 deg separation)
//!   2. Filter by ecliptic latitude threshold
//!   3. Compute Earth shadow radii using Danjon augmented method
//!   4. Classify by comparing Moon's angular distance to shadow radii
//!   5. Find contact times by bisection
//!
//! Solar eclipse algorithm (geocentric):
//!   1. Find new moons (Sun-Moon conjunction, 0 deg separation)
//!   2. Filter by ecliptic latitude threshold
//!   3. Compute apparent Sun and Moon angular radii from distances
//!   4. Classify by comparing radii and minimum separation
//!   5. Find contact times by bisection
//!
//! Sources: standard spherical astronomy (Meeus Ch. 54 for shadow geometry,
//! IAU 2015 nominal radii). See docs/clean_room_eclipse.md.

use dhruv_core::{Body, Engine, Frame, Observer, Query};
use dhruv_frames::{cartesian_to_spherical, icrf_to_ecliptic};

use crate::conjunction::{search_conjunctions, next_conjunction, prev_conjunction};
use crate::conjunction_types::ConjunctionConfig;
use crate::eclipse_types::{
    EclipseConfig, LunarEclipse, LunarEclipseType, SolarEclipse, SolarEclipseType,
};
use crate::error::SearchError;

// ---------------------------------------------------------------------------
// Constants (IAU 2015 nominal values)
// ---------------------------------------------------------------------------

/// Earth equatorial radius in km (IAU 2015 Resolution B3).
const EARTH_RADIUS_KM: f64 = 6378.137;

/// Sun nominal radius in km (IAU 2015 Resolution B3).
const SUN_RADIUS_KM: f64 = 696_000.0;

/// Moon mean radius in km (IAU 2015).
const MOON_RADIUS_KM: f64 = 1737.4;

/// Danjon atmospheric enlargement factor for Earth's shadow.
/// The Earth's atmosphere causes the geometrical shadow to appear ~2% larger.
/// Published in Meeus, "Astronomical Algorithms", Ch. 54.
const DANJON_ENLARGEMENT: f64 = 1.02;

/// Ecliptic latitude threshold for eclipse candidacy (degrees).
/// Generous threshold; exact geometry filters afterward.
const ECLIPSE_LAT_THRESHOLD_DEG: f64 = 2.0;

/// Step size for new/full moon scan (days). Moon synodic period ~29.5 days,
/// so 0.5 day step safely brackets all crossings.
const MOON_STEP_DAYS: f64 = 0.5;

/// Bisection convergence for contact times (days). ~0.86 ms precision.
const CONTACT_CONVERGENCE_DAYS: f64 = 1e-8;

/// Maximum bisection iterations for contact times.
const CONTACT_MAX_ITER: u32 = 50;

// ---------------------------------------------------------------------------
// Internal geometry helpers
// ---------------------------------------------------------------------------

/// Query Moon's ecliptic longitude, latitude (deg), and distance from Earth (km).
fn moon_ecliptic(engine: &Engine, jd_tdb: f64) -> Result<(f64, f64, f64), SearchError> {
    let query = Query {
        target: Body::Moon,
        observer: Observer::Body(Body::Earth),
        frame: Frame::IcrfJ2000,
        epoch_tdb_jd: jd_tdb,
    };
    let state = engine.query(query)?;
    let ecl = icrf_to_ecliptic(&state.position_km);
    let sph = cartesian_to_spherical(&ecl);
    Ok((sph.lon_deg, sph.lat_deg, sph.distance_km))
}

/// Query Sun's distance from Earth in km.
fn sun_distance(engine: &Engine, jd_tdb: f64) -> Result<f64, SearchError> {
    let query = Query {
        target: Body::Sun,
        observer: Observer::Body(Body::Earth),
        frame: Frame::IcrfJ2000,
        epoch_tdb_jd: jd_tdb,
    };
    let state = engine.query(query)?;
    let r = (state.position_km[0].powi(2)
        + state.position_km[1].powi(2)
        + state.position_km[2].powi(2))
    .sqrt();
    Ok(r)
}

/// Angular separation between Sun and Moon centers (degrees) at a given epoch.
/// Computed from their ICRF positions relative to Earth.
fn sun_moon_angular_separation(engine: &Engine, jd_tdb: f64) -> Result<f64, SearchError> {
    let sun_q = Query {
        target: Body::Sun,
        observer: Observer::Body(Body::Earth),
        frame: Frame::IcrfJ2000,
        epoch_tdb_jd: jd_tdb,
    };
    let moon_q = Query {
        target: Body::Moon,
        observer: Observer::Body(Body::Earth),
        frame: Frame::IcrfJ2000,
        epoch_tdb_jd: jd_tdb,
    };
    let sun_state = engine.query(sun_q)?;
    let moon_state = engine.query(moon_q)?;

    // Unit vectors
    let s = &sun_state.position_km;
    let m = &moon_state.position_km;
    let r_s = (s[0] * s[0] + s[1] * s[1] + s[2] * s[2]).sqrt();
    let r_m = (m[0] * m[0] + m[1] * m[1] + m[2] * m[2]).sqrt();

    if r_s < 1e-10 || r_m < 1e-10 {
        return Ok(0.0);
    }

    let dot = (s[0] * m[0] + s[1] * m[1] + s[2] * m[2]) / (r_s * r_m);
    let angle_rad = dot.clamp(-1.0, 1.0).acos();
    Ok(angle_rad.to_degrees())
}

/// Compute Earth shadow radii at Moon's distance using the Danjon method.
///
/// Returns (penumbral_radius_deg, umbral_radius_deg) as angular radii
/// on the sky at the Moon's distance.
///
/// The Danjon method enlarges the geometrical shadow by 2% to account
/// for Earth's atmosphere.
fn shadow_radii_deg(sun_dist_km: f64, moon_dist_km: f64) -> (f64, f64) {
    // Parallax of Sun and Moon
    let pi_sun = (EARTH_RADIUS_KM / sun_dist_km).asin();
    let pi_moon = (EARTH_RADIUS_KM / moon_dist_km).asin();

    // Angular semidiameter of the Sun as seen from Earth
    let s_sun = (SUN_RADIUS_KM / sun_dist_km).asin();

    // Penumbral shadow radius (projected at Moon's distance)
    let penumbral_rad = DANJON_ENLARGEMENT * (pi_moon + pi_sun + s_sun);
    // Umbral shadow radius
    let umbral_rad = DANJON_ENLARGEMENT * (pi_moon + pi_sun - s_sun);

    (penumbral_rad.to_degrees(), umbral_rad.to_degrees())
}

/// Moon's angular semidiameter in degrees.
fn moon_angular_radius_deg(moon_dist_km: f64) -> f64 {
    (MOON_RADIUS_KM / moon_dist_km).asin().to_degrees()
}

/// Sun's angular semidiameter in degrees.
fn sun_angular_radius_deg(sun_dist_km: f64) -> f64 {
    (SUN_RADIUS_KM / sun_dist_km).asin().to_degrees()
}

/// Angular distance of the Moon's center from the anti-solar point (shadow axis).
/// At full moon, this is approximately 180 - (Sun-Moon separation),
/// which gives the angular offset from the center of Earth's shadow.
fn moon_shadow_offset_deg(engine: &Engine, jd_tdb: f64) -> Result<f64, SearchError> {
    let sep = sun_moon_angular_separation(engine, jd_tdb)?;
    // At exact opposition sep = 180°. Shadow offset = 180° - sep.
    // The Moon's ecliptic latitude drives this offset.
    Ok((180.0 - sep).abs())
}

// ---------------------------------------------------------------------------
// Lunar eclipses
// ---------------------------------------------------------------------------

/// Classify a lunar eclipse based on geometry.
fn classify_lunar(
    shadow_offset_deg: f64,
    moon_radius_deg: f64,
    umbral_radius_deg: f64,
    penumbral_radius_deg: f64,
) -> Option<LunarEclipseType> {
    let moon_near_edge = shadow_offset_deg - moon_radius_deg;
    let moon_far_edge = shadow_offset_deg + moon_radius_deg;

    if moon_near_edge >= penumbral_radius_deg {
        // Moon entirely outside penumbra — no eclipse
        None
    } else if moon_far_edge <= umbral_radius_deg {
        // Moon entirely inside umbra — total
        Some(LunarEclipseType::Total)
    } else if moon_near_edge < umbral_radius_deg {
        // Moon partially inside umbra — partial
        Some(LunarEclipseType::Partial)
    } else {
        // Moon in penumbra only
        Some(LunarEclipseType::Penumbral)
    }
}

/// Find a contact time by bisecting when Moon's limb crosses a shadow boundary.
///
/// `boundary_radius_deg` is the shadow radius (umbral or penumbral).
/// `limb_sign`: -1.0 for near limb (inner), +1.0 for far limb (outer).
/// Searches between `t_a` and `t_b`.
fn find_lunar_contact(
    engine: &Engine,
    t_a: f64,
    t_b: f64,
    boundary_radius_deg: f64,
    limb_sign: f64,
) -> Result<f64, SearchError> {
    // f(t) = (shadow_offset + limb_sign * moon_radius) - boundary_radius
    // We look for f(t) = 0

    let f = |jd: f64| -> Result<f64, SearchError> {
        let offset = moon_shadow_offset_deg(engine, jd)?;
        let (_, _, moon_dist) = moon_ecliptic(engine, jd)?;
        let moon_r = moon_angular_radius_deg(moon_dist);
        Ok(offset + limb_sign * moon_r - boundary_radius_deg)
    };

    let mut ta = t_a;
    let mut tb = t_b;
    let mut fa = f(ta)?;

    for _ in 0..CONTACT_MAX_ITER {
        let tm = 0.5 * (ta + tb);
        let fm = f(tm)?;

        if fa * fm <= 0.0 {
            tb = tm;
        } else {
            ta = tm;
            fa = fm;
        }

        if (tb - ta).abs() < CONTACT_CONVERGENCE_DAYS {
            break;
        }
    }

    Ok(0.5 * (ta + tb))
}

/// Compute a single lunar eclipse from a full moon event.
fn compute_lunar_eclipse(
    engine: &Engine,
    full_moon_jd: f64,
    config: &EclipseConfig,
) -> Result<Option<LunarEclipse>, SearchError> {
    // Get Moon's ecliptic latitude at full moon
    let (_, moon_lat, moon_dist) = moon_ecliptic(engine, full_moon_jd)?;

    // Quick filter
    if moon_lat.abs() > ECLIPSE_LAT_THRESHOLD_DEG {
        return Ok(None);
    }

    let sun_dist = sun_distance(engine, full_moon_jd)?;
    let (penumbral_radius, umbral_radius) = shadow_radii_deg(sun_dist, moon_dist);
    let moon_radius = moon_angular_radius_deg(moon_dist);
    let shadow_offset = moon_shadow_offset_deg(engine, full_moon_jd)?;

    let eclipse_type = match classify_lunar(shadow_offset, moon_radius, umbral_radius, penumbral_radius) {
        Some(t) => t,
        None => return Ok(None),
    };

    if !config.include_penumbral && eclipse_type == LunarEclipseType::Penumbral {
        return Ok(None);
    }

    // Compute magnitudes
    let umbral_magnitude = (umbral_radius - shadow_offset + moon_radius) / (2.0 * moon_radius);
    let penumbral_magnitude =
        (penumbral_radius - shadow_offset + moon_radius) / (2.0 * moon_radius);

    // Contact times — search window: ~6 hours around greatest eclipse
    let half_window = 0.25; // 6 hours in days

    // P1: near limb enters penumbra (outer limb, going in)
    let p1_jd = find_lunar_contact(
        engine,
        full_moon_jd - half_window,
        full_moon_jd,
        penumbral_radius,
        1.0, // far limb crosses penumbra boundary
    )?;

    // P4: far limb exits penumbra
    let p4_jd = find_lunar_contact(
        engine,
        full_moon_jd,
        full_moon_jd + half_window,
        penumbral_radius,
        1.0,
    )?;

    // U1/U4: umbral contacts (only if partial or total)
    let (u1_jd, u4_jd) = if eclipse_type != LunarEclipseType::Penumbral {
        let u1 = find_lunar_contact(
            engine,
            full_moon_jd - half_window,
            full_moon_jd,
            umbral_radius,
            1.0,
        )?;
        let u4 = find_lunar_contact(
            engine,
            full_moon_jd,
            full_moon_jd + half_window,
            umbral_radius,
            1.0,
        )?;
        (Some(u1), Some(u4))
    } else {
        (None, None)
    };

    // U2/U3: totality contacts (only if total)
    let (u2_jd, u3_jd) = if eclipse_type == LunarEclipseType::Total {
        let u2 = find_lunar_contact(
            engine,
            full_moon_jd - half_window,
            full_moon_jd,
            umbral_radius,
            -1.0, // near limb crosses umbra boundary
        )?;
        let u3 = find_lunar_contact(
            engine,
            full_moon_jd,
            full_moon_jd + half_window,
            umbral_radius,
            -1.0,
        )?;
        (Some(u2), Some(u3))
    } else {
        (None, None)
    };

    let angular_sep = sun_moon_angular_separation(engine, full_moon_jd)?;

    Ok(Some(LunarEclipse {
        eclipse_type,
        magnitude: umbral_magnitude,
        penumbral_magnitude,
        greatest_eclipse_jd: full_moon_jd,
        p1_jd,
        u1_jd,
        u2_jd,
        u3_jd,
        u4_jd,
        p4_jd,
        moon_ecliptic_lat_deg: moon_lat,
        angular_separation_deg: angular_sep,
    }))
}

/// Find the next lunar eclipse after `jd_tdb`.
pub fn next_lunar_eclipse(
    engine: &Engine,
    jd_tdb: f64,
    config: &EclipseConfig,
) -> Result<Option<LunarEclipse>, SearchError> {
    let moon_config = ConjunctionConfig::opposition(MOON_STEP_DAYS);
    let mut search_jd = jd_tdb;

    // Search up to ~2 years (enough for at least 2 eclipse seasons)
    for _ in 0..50 {
        let full_moon = next_conjunction(engine, Body::Sun, Body::Moon, search_jd, &moon_config)?;
        let Some(fm) = full_moon else {
            return Ok(None);
        };

        if let Some(eclipse) = compute_lunar_eclipse(engine, fm.jd_tdb, config)? {
            return Ok(Some(eclipse));
        }

        // Advance past this full moon
        search_jd = fm.jd_tdb + 1.0;
    }

    Ok(None)
}

/// Find the previous lunar eclipse before `jd_tdb`.
pub fn prev_lunar_eclipse(
    engine: &Engine,
    jd_tdb: f64,
    config: &EclipseConfig,
) -> Result<Option<LunarEclipse>, SearchError> {
    let moon_config = ConjunctionConfig::opposition(MOON_STEP_DAYS);
    let mut search_jd = jd_tdb;

    for _ in 0..50 {
        let full_moon = prev_conjunction(engine, Body::Sun, Body::Moon, search_jd, &moon_config)?;
        let Some(fm) = full_moon else {
            return Ok(None);
        };

        if let Some(eclipse) = compute_lunar_eclipse(engine, fm.jd_tdb, config)? {
            return Ok(Some(eclipse));
        }

        search_jd = fm.jd_tdb - 1.0;
    }

    Ok(None)
}

/// Search for all lunar eclipses in a time range.
pub fn search_lunar_eclipses(
    engine: &Engine,
    jd_start: f64,
    jd_end: f64,
    config: &EclipseConfig,
) -> Result<Vec<LunarEclipse>, SearchError> {
    if jd_end <= jd_start {
        return Err(SearchError::InvalidConfig("jd_end must be after jd_start"));
    }

    let moon_config = ConjunctionConfig::opposition(MOON_STEP_DAYS);
    let full_moons = search_conjunctions(engine, Body::Sun, Body::Moon, jd_start, jd_end, &moon_config)?;

    let mut eclipses = Vec::new();
    for fm in &full_moons {
        if let Some(eclipse) = compute_lunar_eclipse(engine, fm.jd_tdb, config)? {
            eclipses.push(eclipse);
        }
    }

    Ok(eclipses)
}

// ---------------------------------------------------------------------------
// Solar eclipses (geocentric)
// ---------------------------------------------------------------------------

/// Classify a geocentric solar eclipse.
fn classify_solar(
    sun_radius_deg: f64,
    moon_radius_deg: f64,
    min_separation_deg: f64,
) -> Option<SolarEclipseType> {
    let sum = sun_radius_deg + moon_radius_deg;

    if min_separation_deg >= sum {
        // No overlap — no eclipse
        return None;
    }

    if min_separation_deg < (moon_radius_deg - sun_radius_deg).abs() {
        // Complete overlap
        if moon_radius_deg >= sun_radius_deg {
            Some(SolarEclipseType::Total)
        } else {
            Some(SolarEclipseType::Annular)
        }
    } else {
        // Partial overlap only
        Some(SolarEclipseType::Partial)
    }
}

/// Find a solar eclipse contact time by bisecting when disk edges touch.
///
/// `target_sep_deg`: the separation at which contact occurs
/// (sun_r + moon_r for external, |sun_r - moon_r| for internal).
fn find_solar_contact(
    engine: &Engine,
    t_a: f64,
    t_b: f64,
    target_sep_deg: f64,
) -> Result<f64, SearchError> {
    let f = |jd: f64| -> Result<f64, SearchError> {
        let sep = sun_moon_angular_separation(engine, jd)?;
        Ok(sep - target_sep_deg)
    };

    let mut ta = t_a;
    let mut tb = t_b;
    let mut fa = f(ta)?;

    for _ in 0..CONTACT_MAX_ITER {
        let tm = 0.5 * (ta + tb);
        let fm = f(tm)?;

        if fa * fm <= 0.0 {
            tb = tm;
        } else {
            ta = tm;
            fa = fm;
        }

        if (tb - ta).abs() < CONTACT_CONVERGENCE_DAYS {
            break;
        }
    }

    Ok(0.5 * (ta + tb))
}

/// Compute a single geocentric solar eclipse from a new moon event.
fn compute_solar_eclipse(
    engine: &Engine,
    new_moon_jd: f64,
    _config: &EclipseConfig,
) -> Result<Option<SolarEclipse>, SearchError> {
    // Get Moon's ecliptic latitude at new moon
    let (_, moon_lat, moon_dist) = moon_ecliptic(engine, new_moon_jd)?;

    if moon_lat.abs() > ECLIPSE_LAT_THRESHOLD_DEG {
        return Ok(None);
    }

    let sun_dist = sun_distance(engine, new_moon_jd)?;
    let sun_r = sun_angular_radius_deg(sun_dist);
    let moon_r = moon_angular_radius_deg(moon_dist);
    let min_sep = sun_moon_angular_separation(engine, new_moon_jd)?;

    let eclipse_type = match classify_solar(sun_r, moon_r, min_sep) {
        Some(t) => t,
        None => return Ok(None),
    };

    let magnitude = moon_r / sun_r;

    // Contact times — search window: ~4 hours around greatest eclipse
    let half_window = 4.0 / 24.0;
    let external_sep = sun_r + moon_r;
    let internal_sep = (sun_r - moon_r).abs();

    // C1: first external contact (disks start touching)
    let c1_jd = find_solar_contact(engine, new_moon_jd - half_window, new_moon_jd, external_sep)
        .ok();

    // C4: last external contact (disks stop touching)
    let c4_jd = find_solar_contact(engine, new_moon_jd, new_moon_jd + half_window, external_sep)
        .ok();

    // C2/C3: internal contacts (only for total/annular)
    let (c2_jd, c3_jd) =
        if eclipse_type == SolarEclipseType::Total || eclipse_type == SolarEclipseType::Annular {
            let c2 = find_solar_contact(
                engine,
                new_moon_jd - half_window,
                new_moon_jd,
                internal_sep,
            )
            .ok();
            let c3 = find_solar_contact(
                engine,
                new_moon_jd,
                new_moon_jd + half_window,
                internal_sep,
            )
            .ok();
            (c2, c3)
        } else {
            (None, None)
        };

    Ok(Some(SolarEclipse {
        eclipse_type,
        magnitude,
        greatest_eclipse_jd: new_moon_jd,
        c1_jd,
        c2_jd,
        c3_jd,
        c4_jd,
        moon_ecliptic_lat_deg: moon_lat,
        angular_separation_deg: min_sep,
    }))
}

/// Find the next geocentric solar eclipse after `jd_tdb`.
pub fn next_solar_eclipse(
    engine: &Engine,
    jd_tdb: f64,
    config: &EclipseConfig,
) -> Result<Option<SolarEclipse>, SearchError> {
    let moon_config = ConjunctionConfig::conjunction(MOON_STEP_DAYS);
    let mut search_jd = jd_tdb;

    for _ in 0..50 {
        let new_moon = next_conjunction(engine, Body::Sun, Body::Moon, search_jd, &moon_config)?;
        let Some(nm) = new_moon else {
            return Ok(None);
        };

        if let Some(eclipse) = compute_solar_eclipse(engine, nm.jd_tdb, config)? {
            return Ok(Some(eclipse));
        }

        search_jd = nm.jd_tdb + 1.0;
    }

    Ok(None)
}

/// Find the previous geocentric solar eclipse before `jd_tdb`.
pub fn prev_solar_eclipse(
    engine: &Engine,
    jd_tdb: f64,
    config: &EclipseConfig,
) -> Result<Option<SolarEclipse>, SearchError> {
    let moon_config = ConjunctionConfig::conjunction(MOON_STEP_DAYS);
    let mut search_jd = jd_tdb;

    for _ in 0..50 {
        let new_moon = prev_conjunction(engine, Body::Sun, Body::Moon, search_jd, &moon_config)?;
        let Some(nm) = new_moon else {
            return Ok(None);
        };

        if let Some(eclipse) = compute_solar_eclipse(engine, nm.jd_tdb, config)? {
            return Ok(Some(eclipse));
        }

        search_jd = nm.jd_tdb - 1.0;
    }

    Ok(None)
}

/// Search for all geocentric solar eclipses in a time range.
pub fn search_solar_eclipses(
    engine: &Engine,
    jd_start: f64,
    jd_end: f64,
    config: &EclipseConfig,
) -> Result<Vec<SolarEclipse>, SearchError> {
    if jd_end <= jd_start {
        return Err(SearchError::InvalidConfig("jd_end must be after jd_start"));
    }

    let moon_config = ConjunctionConfig::conjunction(MOON_STEP_DAYS);
    let new_moons = search_conjunctions(engine, Body::Sun, Body::Moon, jd_start, jd_end, &moon_config)?;

    let mut eclipses = Vec::new();
    for nm in &new_moons {
        if let Some(eclipse) = compute_solar_eclipse(engine, nm.jd_tdb, config)? {
            eclipses.push(eclipse);
        }
    }

    Ok(eclipses)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shadow_radii_reasonable() {
        // Sun at ~1 AU, Moon at ~384400 km
        let (pen, umb) = shadow_radii_deg(149_597_870.7, 384_400.0);
        // Penumbral radius ~1.2-1.3 deg (pi_moon ~0.95 deg dominates)
        assert!(pen > 1.1 && pen < 1.4, "penumbral = {pen}");
        // Umbral radius ~0.65-0.75 deg (pi_moon - s_sun, Danjon enlarged)
        assert!(umb > 0.6 && umb < 0.8, "umbral = {umb}");
    }

    #[test]
    fn moon_angular_radius_typical() {
        let r = moon_angular_radius_deg(384_400.0);
        // ~0.26 deg
        assert!(r > 0.24 && r < 0.28, "moon angular radius = {r}");
    }

    #[test]
    fn sun_angular_radius_typical() {
        let r = sun_angular_radius_deg(149_597_870.7);
        // ~0.266 deg
        assert!(r > 0.25 && r < 0.28, "sun angular radius = {r}");
    }

    #[test]
    fn classify_lunar_total() {
        // Moon center very close to shadow axis, small offset
        // near_edge = 0.1 - 0.26 = -0.16, far_edge = 0.1 + 0.26 = 0.36 < 0.70
        let result = classify_lunar(0.1, 0.26, 0.70, 1.25);
        assert_eq!(result, Some(LunarEclipseType::Total));
    }

    #[test]
    fn classify_lunar_partial() {
        // Moon center near umbra boundary
        // near_edge = 0.55 - 0.26 = 0.29 < 0.70, but far_edge = 0.55 + 0.26 = 0.81 > 0.70
        let result = classify_lunar(0.55, 0.26, 0.70, 1.25);
        assert_eq!(result, Some(LunarEclipseType::Partial));
    }

    #[test]
    fn classify_lunar_penumbral() {
        // Moon center outside umbra but inside penumbra
        // near_edge = 1.05 - 0.26 = 0.79 >= 0.70 (outside umbra)
        // far_edge = 1.05 + 0.26 = 1.31 > 1.25 but near_edge < 1.25 (inside penumbra)
        let result = classify_lunar(1.05, 0.26, 0.70, 1.25);
        assert_eq!(result, Some(LunarEclipseType::Penumbral));
    }

    #[test]
    fn classify_lunar_none() {
        // Moon outside penumbra entirely (near edge > penumbral radius)
        let result = classify_lunar(1.6, 0.26, 0.70, 1.25);
        assert_eq!(result, None);
    }

    #[test]
    fn classify_solar_total() {
        // Moon larger than Sun, separation very small
        let result = classify_solar(0.266, 0.270, 0.002);
        assert_eq!(result, Some(SolarEclipseType::Total));
    }

    #[test]
    fn classify_solar_annular() {
        // Moon smaller than Sun, separation very small
        let result = classify_solar(0.266, 0.250, 0.002);
        assert_eq!(result, Some(SolarEclipseType::Annular));
    }

    #[test]
    fn classify_solar_partial() {
        // Disks overlap but neither fully covers the other
        let result = classify_solar(0.266, 0.260, 0.30);
        assert_eq!(result, Some(SolarEclipseType::Partial));
    }

    #[test]
    fn classify_solar_none() {
        // No overlap
        let result = classify_solar(0.266, 0.260, 0.6);
        assert_eq!(result, None);
    }

    #[test]
    fn eclipse_config_defaults() {
        let c = EclipseConfig::default();
        assert!(c.include_penumbral);
        assert!(c.include_peak_details);
    }

    #[test]
    fn danjon_enlargement_is_1_02() {
        assert!((DANJON_ENLARGEMENT - 1.02).abs() < 1e-10);
    }
}
