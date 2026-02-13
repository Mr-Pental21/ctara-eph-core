//! Greenwich Mean Sidereal Time and Earth Rotation Angle.
//!
//! Provides the ERA and GMST needed for converting between celestial
//! (RA/Dec) and terrestrial (hour angle) coordinate systems.
//!
//! All functions take UT1 Julian Dates. Callers convert UTC→UT1 using
//! [`crate::EopKernel::utc_to_ut1_jd`] before calling these functions.
//!
//! Sources:
//! - ERA: IERS Conventions 2010, Eq. 5.15. Public domain.
//! - GMST polynomial: Capitaine et al. 2003, Table 2. Public domain.

use std::f64::consts::{PI, TAU};

use crate::julian::J2000_JD;

/// Arcseconds to radians: 1″ = π / (180 × 3600).
const ARCSEC_TO_RAD: f64 = PI / (180.0 * 3600.0);

/// Earth Rotation Angle at a given UT1 Julian Date.
///
/// θ = 2π × (0.7790572732640 + 1.00273781191135448 × Du)
/// where Du = JD_UT1 − 2451545.0.
///
/// Returns radians in [0, 2π).
///
/// Source: IERS Conventions 2010, Eq. 5.15.
pub fn earth_rotation_angle_rad(jd_ut1: f64) -> f64 {
    let du = jd_ut1 - J2000_JD;
    let theta = TAU * (0.779_057_273_264_0 + 1.002_737_811_911_354_6 * du);
    theta.rem_euclid(TAU)
}

/// Greenwich Mean Sidereal Time at a given UT1 Julian Date.
///
/// GMST = ERA + polynomial(T), where T = Julian centuries of UT1 from J2000.0.
///
/// Polynomial (arcseconds):
///   0.014506 + 4612.156534·T + 1.3915817·T² − 0.00000044·T³
///   − 0.000029956·T⁴ − 0.0000000368·T⁵
///
/// Returns radians in [0, 2π).
///
/// Source: Capitaine et al. 2003, Table 2.
pub fn gmst_rad(jd_ut1: f64) -> f64 {
    let era = earth_rotation_angle_rad(jd_ut1);
    let t = (jd_ut1 - J2000_JD) / 36525.0;
    let t2 = t * t;
    let t3 = t2 * t;
    let t4 = t3 * t;
    let t5 = t4 * t;

    let poly_arcsec = 0.014506 + 4612.156534 * t + 1.3915817 * t2
        - 0.00000044 * t3
        - 0.000029956 * t4
        - 0.0000000368 * t5;

    let gmst = era + poly_arcsec * ARCSEC_TO_RAD;
    gmst.rem_euclid(TAU)
}

/// Local Sidereal Time from GMST and observer east longitude.
///
/// LST = GMST + longitude_east_rad.
/// Returns radians in [0, 2π).
pub fn local_sidereal_time_rad(gmst: f64, longitude_east_rad: f64) -> f64 {
    (gmst + longitude_east_rad).rem_euclid(TAU)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn era_at_j2000_noon() {
        // At J2000.0 (JD 2451545.0), ERA ≈ 280.46°
        let theta = earth_rotation_angle_rad(J2000_JD);
        let theta_deg = theta.to_degrees();
        assert!(
            (theta_deg - 280.46).abs() < 0.1,
            "ERA at J2000 = {theta_deg}°, expected ~280.46°"
        );
    }

    #[test]
    fn gmst_j2000_midnight() {
        // At 2000-Jan-01 0h UT1 (JD 2451544.5), GMST ≈ 6h 39m 51.170s
        // = 6.66421... h = 99.963° approximately
        let gmst = gmst_rad(2_451_544.5);
        let gmst_deg = gmst.to_degrees();
        assert!(
            (gmst_deg - 99.97).abs() < 0.1,
            "GMST at J2000 midnight = {gmst_deg}°, expected ~99.97°"
        );
    }

    #[test]
    fn gmst_monotonic() {
        let g1 = gmst_rad(2_451_545.0);
        let g2 = gmst_rad(2_451_546.0);
        // GMST advances ~361° per day (one sidereal day > one solar day).
        // After rem_euclid, g2 should be about ~0.986° ahead of g1.
        // Just check they're not equal.
        assert!((g2 - g1).abs() > 0.001, "GMST should differ between days");
    }

    #[test]
    fn lst_east_offset() {
        let gmst = 1.0; // arbitrary
        let lst = local_sidereal_time_rad(gmst, PI / 2.0);
        let expected = (gmst + PI / 2.0).rem_euclid(TAU);
        assert!((lst - expected).abs() < 1e-15);
    }

    #[test]
    fn era_range() {
        // ERA should always be in [0, 2π)
        for &jd in &[2_451_545.0, 2_451_544.5, 2_460_000.5, 2_440_000.5] {
            let theta = earth_rotation_angle_rad(jd);
            assert!((0.0..TAU).contains(&theta), "ERA out of range: {theta}");
        }
    }

    #[test]
    fn gmst_range() {
        for &jd in &[2_451_545.0, 2_451_544.5, 2_460_000.5, 2_440_000.5] {
            let g = gmst_rad(jd);
            assert!((0.0..TAU).contains(&g), "GMST out of range: {g}");
        }
    }
}
