//! Sunrise/sunset computation with twilight variants.
//!
//! Iterative algorithm based on standard spherical astronomy formulas.
//! Computes the time when the Sun's geocentric altitude equals a target
//! depression angle, for a given observer location and date.
//!
//! Sources: standard astronomical spherical trigonometry (Meeus, USNO,
//! Montenbruck & Pfleger). Original implementation from the fundamental
//! formulas. See `docs/clean_room_riseset.md`.

use std::f64::consts::TAU;

use dhruv_core::{Body, Engine, Frame, Observer, Query};
use dhruv_frames::cartesian_to_spherical;
use dhruv_time::{
    EopKernel, LeapSecondKernel,
    gmst_rad, local_sidereal_time_rad,
    jd_to_tdb_seconds, tdb_seconds_to_jd,
};

use crate::error::VedicError;
use crate::riseset_types::{GeoLocation, RiseSetConfig, RiseSetEvent, RiseSetResult};

/// Maximum iterations for the rise/set refinement loop.
const MAX_ITERATIONS: usize = 5;

/// Convergence threshold in days (~0.086 seconds).
const CONVERGENCE_DAYS: f64 = 1.0e-6;

/// IAU 2015 nominal solar radius in km (Resolution B3).
const SUN_RADIUS_KM: f64 = 696_000.0;

/// Approximate local solar noon JD from 0h UT JD and longitude.
///
/// `JD_noon = JD_0h + 0.5 - longitude_deg / 360`
///
/// This gives the approximate time when the Sun crosses the local meridian.
pub fn approximate_local_noon_jd(jd_ut_midnight: f64, longitude_deg: f64) -> f64 {
    jd_ut_midnight + 0.5 - longitude_deg / 360.0
}

/// Compute the Sun's geocentric equatorial RA, Dec, and distance at a given JD TDB.
///
/// Queries the ephemeris engine for the Sun's position relative to Earth
/// in ICRF/J2000, then converts to spherical coordinates.
///
/// Returns `(ra_rad, dec_rad, distance_km)` where RA is in [0, 2pi),
/// Dec in [-pi/2, pi/2], and distance in km.
fn sun_equatorial_ra_dec_dist(
    engine: &Engine,
    jd_tdb: f64,
) -> Result<(f64, f64, f64), VedicError> {
    let query = Query {
        target: Body::Sun,
        observer: Observer::Body(Body::Earth),
        frame: Frame::IcrfJ2000,
        epoch_tdb_jd: jd_tdb,
    };
    let state = engine.query(query)?;
    let sph = cartesian_to_spherical(&state.position_km);
    Ok((sph.lon_rad, sph.lat_rad, sph.distance_km))
}

/// Compute solar angular semidiameter from Earth-Sun distance.
///
/// Returns semidiameter in arcminutes.
/// Varies ~15.7' (aphelion) to ~16.3' (perihelion).
fn solar_semidiameter_arcmin(distance_km: f64) -> f64 {
    (SUN_RADIUS_KM / distance_km).asin().to_degrees() * 60.0
}

/// Compute a single rise/set event for the Sun.
///
/// # Arguments
/// * `engine` — ephemeris engine for querying Sun position
/// * `lsk` — leap second kernel for UTC-TDB conversion
/// * `eop` — IERS Earth Orientation Parameters for UTC-UT1 conversion
/// * `location` — observer geographic location
/// * `event` — the event type (sunrise, sunset, twilight variant)
/// * `jd_utc_noon` — approximate local noon on the desired date (UTC JD).
///   Use [`approximate_local_noon_jd`] to compute from calendar date + longitude.
/// * `config` — refraction, limb, and altitude parameters
///
/// # Returns
/// * `RiseSetResult::Event` with the event time in JD TDB
/// * `RiseSetResult::NeverRises` if the Sun stays below the horizon (polar night)
/// * `RiseSetResult::NeverSets` if the Sun stays above the horizon (midnight sun)
pub fn compute_rise_set(
    engine: &Engine,
    lsk: &LeapSecondKernel,
    eop: &EopKernel,
    location: &GeoLocation,
    event: RiseSetEvent,
    jd_utc_noon: f64,
    config: &RiseSetConfig,
) -> Result<RiseSetResult, VedicError> {
    let phi = location.latitude_rad();

    // Convert noon UTC to TDB for initial Sun query
    let noon_utc_s = jd_to_tdb_seconds(jd_utc_noon); // UTC seconds past J2000
    let noon_tdb_s = lsk.utc_to_tdb(noon_utc_s);
    let jd_tdb_noon = tdb_seconds_to_jd(noon_tdb_s);

    // Initial Sun RA/Dec/distance at noon
    let (ra, dec, dist) = sun_equatorial_ra_dec_dist(engine, jd_tdb_noon)?;
    let semidiameter = solar_semidiameter_arcmin(dist);

    // Target altitude (negative = below horizon)
    let h0_deg = config.target_altitude_deg(event, semidiameter, location.altitude_m);
    let h0_rad = h0_deg.to_radians();

    // Hour angle at target altitude
    let cos_h0 = (h0_rad.sin() - phi.sin() * dec.sin()) / (phi.cos() * dec.cos());

    // Polar check
    if cos_h0 > 1.0 {
        return Ok(RiseSetResult::NeverRises);
    }
    if cos_h0 < -1.0 {
        return Ok(RiseSetResult::NeverSets);
    }

    let h0 = cos_h0.acos(); // hour angle in radians, always positive

    // Convert noon UTC to UT1 for sidereal time
    let jd_ut1_noon = eop.utc_to_ut1_jd(jd_utc_noon)?;
    let gmst_noon = gmst_rad(jd_ut1_noon);
    let lst_noon = local_sidereal_time_rad(gmst_noon, location.longitude_rad());

    // Sun hour angle at noon (should be close to 0)
    let ha_noon = (lst_noon - ra).rem_euclid(TAU);
    // Normalize to [-pi, pi]
    let ha_noon = if ha_noon > std::f64::consts::PI {
        ha_noon - TAU
    } else {
        ha_noon
    };

    // Transit time (when HA = 0): correct noon by the offset
    // HA advances at ~1.00274 rev/day = TAU * 1.00274 rad/day
    let sidereal_rate = TAU * 1.002_737_811_911_354_6; // rad/day
    let transit_correction = -ha_noon / sidereal_rate; // days
    let jd_utc_transit = jd_utc_noon + transit_correction;

    // Initial estimate of event time
    let h0_days = h0 / sidereal_rate;
    let mut jd_utc_event = if event.is_rising() {
        jd_utc_transit - h0_days
    } else {
        jd_utc_transit + h0_days
    };

    // Iterative refinement
    for _ in 0..MAX_ITERATIONS {
        // Convert event UTC to TDB
        let event_utc_s = jd_to_tdb_seconds(jd_utc_event);
        let event_tdb_s = lsk.utc_to_tdb(event_utc_s);
        let jd_tdb_event = tdb_seconds_to_jd(event_tdb_s);

        // Recompute Sun RA/Dec/distance at event time
        let (ra_i, dec_i, dist_i) = sun_equatorial_ra_dec_dist(engine, jd_tdb_event)?;
        let sd_i = solar_semidiameter_arcmin(dist_i);

        // Recompute target altitude with updated semidiameter
        let h0_deg_i = config.target_altitude_deg(event, sd_i, location.altitude_m);
        let h0_rad_i = h0_deg_i.to_radians();

        // Recompute hour angle at event time
        let cos_h_i = (h0_rad_i.sin() - phi.sin() * dec_i.sin()) / (phi.cos() * dec_i.cos());
        if cos_h_i > 1.0 {
            return Ok(RiseSetResult::NeverRises);
        }
        if cos_h_i < -1.0 {
            return Ok(RiseSetResult::NeverSets);
        }
        let h_target = cos_h_i.acos();

        // Compute actual HA at event time via GMST
        let jd_ut1_event = eop.utc_to_ut1_jd(jd_utc_event)?;
        let gmst_event = gmst_rad(jd_ut1_event);
        let lst_event = local_sidereal_time_rad(gmst_event, location.longitude_rad());
        let mut ha_actual = (lst_event - ra_i).rem_euclid(TAU);
        if ha_actual > std::f64::consts::PI {
            ha_actual -= TAU;
        }

        // For rising events, target HA is negative; for setting, positive
        let ha_target = if event.is_rising() {
            -h_target
        } else {
            h_target
        };

        // Correction in days
        let mut dha = ha_target - ha_actual;
        // Normalize dha to [-pi, pi]
        if dha > std::f64::consts::PI {
            dha -= TAU;
        } else if dha < -std::f64::consts::PI {
            dha += TAU;
        }
        let correction = dha / sidereal_rate;

        jd_utc_event += correction;

        if correction.abs() < CONVERGENCE_DAYS {
            break;
        }
    }

    // Convert final UTC event time to TDB
    let final_utc_s = jd_to_tdb_seconds(jd_utc_event);
    let final_tdb_s = lsk.utc_to_tdb(final_utc_s);
    let jd_tdb_final = tdb_seconds_to_jd(final_tdb_s);

    Ok(RiseSetResult::Event {
        jd_tdb: jd_tdb_final,
        event,
    })
}

/// Compute all 8 rise/set events for a day.
///
/// Returns results in chronological order:
/// AstronomicalDawn, NauticalDawn, CivilDawn, Sunrise,
/// Sunset, CivilDusk, NauticalDusk, AstronomicalDusk.
///
/// Each event is computed independently; if one event cannot occur (e.g.,
/// NeverRises at high latitudes), it is included as NeverRises/NeverSets.
pub fn compute_all_events(
    engine: &Engine,
    lsk: &LeapSecondKernel,
    eop: &EopKernel,
    location: &GeoLocation,
    jd_utc_noon: f64,
    config: &RiseSetConfig,
) -> Result<Vec<RiseSetResult>, VedicError> {
    let events = [
        RiseSetEvent::AstronomicalDawn,
        RiseSetEvent::NauticalDawn,
        RiseSetEvent::CivilDawn,
        RiseSetEvent::Sunrise,
        RiseSetEvent::Sunset,
        RiseSetEvent::CivilDusk,
        RiseSetEvent::NauticalDusk,
        RiseSetEvent::AstronomicalDusk,
    ];

    let mut results = Vec::with_capacity(events.len());
    for &evt in &events {
        results.push(compute_rise_set(engine, lsk, eop, location, evt, jd_utc_noon, config)?);
    }
    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_noon_greenwich() {
        let jd_0h = 2_460_000.5;
        let noon = approximate_local_noon_jd(jd_0h, 0.0);
        assert!((noon - (jd_0h + 0.5)).abs() < 1e-10);
    }

    #[test]
    fn local_noon_east_90() {
        let jd_0h = 2_460_000.5;
        let noon = approximate_local_noon_jd(jd_0h, 90.0);
        // 90 deg east → noon is 6 hours earlier in UT
        assert!((noon - (jd_0h + 0.25)).abs() < 1e-10);
    }

    #[test]
    fn local_noon_west_90() {
        let jd_0h = 2_460_000.5;
        let noon = approximate_local_noon_jd(jd_0h, -90.0);
        // 90 deg west → noon is 6 hours later in UT
        assert!((noon - (jd_0h + 0.75)).abs() < 1e-10);
    }

    #[test]
    fn solar_semidiameter_typical() {
        // 1 AU ≈ 149_597_870.7 km → semidiameter ≈ 16 arcmin
        let sd = solar_semidiameter_arcmin(149_597_870.7);
        assert!(
            (sd - 16.0).abs() < 0.5,
            "semidiameter at 1 AU = {sd}, expected ~16'"
        );
    }

    #[test]
    fn solar_semidiameter_varies() {
        // Perihelion (~147.1e6 km) vs aphelion (~152.1e6 km)
        let sd_peri = solar_semidiameter_arcmin(147_100_000.0);
        let sd_aph = solar_semidiameter_arcmin(152_100_000.0);
        assert!(sd_peri > sd_aph, "perihelion SD should be larger");
        assert!(sd_peri > 16.0, "perihelion SD ~ 16.3', got {sd_peri}");
        assert!(sd_aph < 16.0, "aphelion SD ~ 15.7', got {sd_aph}");
    }
}
