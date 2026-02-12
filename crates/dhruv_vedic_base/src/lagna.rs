//! Lagna (Ascendant) and MC (Midheaven) computation.
//!
//! Standalone reusable module implementing the standard spherical astronomy
//! formulas for the ecliptic longitude of the Lagna and MC.
//!
//! Sources: Meeus, "Astronomical Algorithms" (2nd ed), Chapter 13;
//! standard spherical astronomy (Montenbruck & Pfleger).
//! See `docs/clean_room_bhava.md`.

use std::f64::consts::TAU;

use dhruv_frames::OBLIQUITY_J2000_RAD;
use dhruv_time::{EopKernel, LeapSecondKernel, gmst_rad, local_sidereal_time_rad};

use crate::error::VedicError;
use crate::riseset_types::GeoLocation;

/// Compute Local Sidereal Time from JD UTC via the UTC->UT1->GMST->LST chain.
///
/// Returns LST in radians, range [0, 2*pi).
fn compute_lst_rad(
    _lsk: &LeapSecondKernel,
    eop: &EopKernel,
    location: &GeoLocation,
    jd_utc: f64,
) -> Result<f64, VedicError> {
    let jd_ut1 = eop.utc_to_ut1_jd(jd_utc)?;
    let gmst = gmst_rad(jd_ut1);
    let lst = local_sidereal_time_rad(gmst, location.longitude_rad());
    Ok(lst.rem_euclid(TAU))
}

/// Ecliptic longitude of the Lagna (Ascendant) in radians.
///
/// Formula (Meeus Ch. 13):
/// `Asc = atan2(-cos(LST), sin(LST)*cos(eps) + tan(phi)*sin(eps))`
///
/// Returns a value in [0, 2*pi).
pub fn lagna_longitude_rad(
    lsk: &LeapSecondKernel,
    eop: &EopKernel,
    location: &GeoLocation,
    jd_utc: f64,
) -> Result<f64, VedicError> {
    let lst = compute_lst_rad(lsk, eop, location, jd_utc)?;
    let eps = OBLIQUITY_J2000_RAD;
    let phi = location.latitude_rad();

    let asc = f64::atan2(
        -lst.cos(),
        lst.sin() * eps.cos() + phi.tan() * eps.sin(),
    );
    Ok(asc.rem_euclid(TAU))
}

/// Ecliptic longitude of the MC (Midheaven) in radians.
///
/// Formula: `MC = atan2(sin(LST), cos(LST)*cos(eps))`
///
/// Returns a value in [0, 2*pi).
pub fn mc_longitude_rad(
    lsk: &LeapSecondKernel,
    eop: &EopKernel,
    location: &GeoLocation,
    jd_utc: f64,
) -> Result<f64, VedicError> {
    let lst = compute_lst_rad(lsk, eop, location, jd_utc)?;
    let eps = OBLIQUITY_J2000_RAD;

    let mc = f64::atan2(lst.sin(), lst.cos() * eps.cos());
    Ok(mc.rem_euclid(TAU))
}

/// Compute both Lagna and MC (shares LST computation).
///
/// Returns `(lagna_rad, mc_rad)`, both in [0, 2*pi).
pub fn lagna_and_mc_rad(
    lsk: &LeapSecondKernel,
    eop: &EopKernel,
    location: &GeoLocation,
    jd_utc: f64,
) -> Result<(f64, f64), VedicError> {
    let lst = compute_lst_rad(lsk, eop, location, jd_utc)?;
    let eps = OBLIQUITY_J2000_RAD;
    let phi = location.latitude_rad();

    let asc = f64::atan2(
        -lst.cos(),
        lst.sin() * eps.cos() + phi.tan() * eps.sin(),
    );

    let mc = f64::atan2(lst.sin(), lst.cos() * eps.cos());

    Ok((asc.rem_euclid(TAU), mc.rem_euclid(TAU)))
}

/// RAMC (Right Ascension of the MC) in radians.
///
/// By definition, RAMC equals LST. Needed by Regiomontanus, Campanus, etc.
/// Returns a value in [0, 2*pi).
pub fn ramc_rad(
    lsk: &LeapSecondKernel,
    eop: &EopKernel,
    location: &GeoLocation,
    jd_utc: f64,
) -> Result<f64, VedicError> {
    compute_lst_rad(lsk, eop, location, jd_utc)
}

/// Internal helper: compute Lagna, MC, and RAMC from a pre-computed LST.
///
/// Used by bhava computation to avoid redundant LST calculations.
pub(crate) fn lagna_mc_ramc_from_lst(
    lst_rad: f64,
    latitude_rad: f64,
) -> (f64, f64, f64) {
    let eps = OBLIQUITY_J2000_RAD;

    let asc = f64::atan2(
        -lst_rad.cos(),
        lst_rad.sin() * eps.cos() + latitude_rad.tan() * eps.sin(),
    );

    let mc = f64::atan2(lst_rad.sin(), lst_rad.cos() * eps.cos());

    (asc.rem_euclid(TAU), mc.rem_euclid(TAU), lst_rad.rem_euclid(TAU))
}

/// Compute LST — public(crate) for bhava module reuse.
pub(crate) fn compute_lst_rad_pub(
    lsk: &LeapSecondKernel,
    eop: &EopKernel,
    location: &GeoLocation,
    jd_utc: f64,
) -> Result<f64, VedicError> {
    compute_lst_rad(lsk, eop, location, jd_utc)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::consts::PI;

    /// At equator (phi=0), LST=0: Asc should be 0 (Aries rising).
    ///
    /// atan2(-cos(0), sin(0)*cos(eps) + 0) = atan2(-1, 0) = -pi/2
    /// But normalized: -pi/2 + 2*pi = 3*pi/2 ... Let's verify.
    ///
    /// Actually, at LST=0:
    ///   numerator = -cos(0) = -1
    ///   denominator = sin(0)*cos(eps) + tan(0)*sin(eps) = 0
    ///   atan2(-1, 0) = -pi/2 → rem_euclid(TAU) = 3*pi/2 = 270 deg
    ///
    /// This is the Ascendant for LST=0 at equator. At LST=0, the vernal
    /// equinox (RA=0) is on the meridian, so the Ascendant (eastern horizon)
    /// is at ecliptic longitude 270 deg (0 Capricorn), which is correct.
    #[test]
    fn ascendant_formula_equator_lst_zero() {
        let eps = OBLIQUITY_J2000_RAD;
        let phi: f64 = 0.0; // equator
        let lst: f64 = 0.0;

        let asc = f64::atan2(
            -lst.cos(),
            lst.sin() * eps.cos() + phi.tan() * eps.sin(),
        )
        .rem_euclid(TAU);

        // At equator, LST=0: Asc = 270 deg = 3*pi/2
        let expected = 3.0 * PI / 2.0;
        assert!(
            (asc - expected).abs() < 1e-10,
            "Asc at equator, LST=0 = {} deg, expected 270",
            asc.to_degrees()
        );
    }

    /// MC at LST=0: atan2(sin(0), cos(0)*cos(eps)) = atan2(0, cos(eps)) = 0
    #[test]
    fn mc_formula_lst_zero() {
        let eps = OBLIQUITY_J2000_RAD;
        let lst: f64 = 0.0;

        let mc = f64::atan2(lst.sin(), lst.cos() * eps.cos()).rem_euclid(TAU);
        assert!(mc.abs() < 1e-10, "MC at LST=0 = {} deg, expected 0", mc.to_degrees());
    }

    /// As LST sweeps 0..2*pi, Ascendant should cover the full circle.
    #[test]
    fn ascendant_quadrant_sweep() {
        let eps = OBLIQUITY_J2000_RAD;
        let phi = 28.6_f64.to_radians(); // New Delhi

        let n = 360;
        let mut min_asc = f64::MAX;
        let mut max_asc = f64::MIN;

        for i in 0..n {
            let lst = TAU * (i as f64) / (n as f64);
            let asc = f64::atan2(
                -lst.cos(),
                lst.sin() * eps.cos() + phi.tan() * eps.sin(),
            )
            .rem_euclid(TAU);
            if asc < min_asc { min_asc = asc; }
            if asc > max_asc { max_asc = asc; }
        }

        // Should span nearly 0..2*pi
        assert!(min_asc < 0.05, "min_asc = {}", min_asc.to_degrees());
        assert!(max_asc > TAU - 0.05, "max_asc = {}", max_asc.to_degrees());
    }

    /// At low latitudes, Asc and MC typically differ by roughly 90 deg.
    #[test]
    fn ascendant_and_mc_differ_by_about_90() {
        let eps = OBLIQUITY_J2000_RAD;
        let phi = 10.0_f64.to_radians(); // low latitude

        // Sample a few LST values
        let lsts: [f64; 4] = [0.5, 1.5, 3.0, 4.5];
        for &lst in &lsts {
            let asc = f64::atan2(
                -lst.cos(),
                lst.sin() * eps.cos() + phi.tan() * eps.sin(),
            )
            .rem_euclid(TAU);

            let mc = f64::atan2(lst.sin(), lst.cos() * eps.cos()).rem_euclid(TAU);

            let mut diff = (asc - mc).abs();
            if diff > PI { diff = TAU - diff; }

            // At low latitudes the difference is approximately 90 deg (+/- ~20 deg)
            assert!(
                diff > 1.0 && diff < 2.2,
                "LST={:.1}: |Asc-MC| = {:.1} deg, expected ~90",
                lst.to_degrees(), diff.to_degrees()
            );
        }
    }

    /// RAMC equals LST by definition.
    #[test]
    fn ramc_equals_lst() {
        // Test the internal helper
        let lst = 1.234;
        let (_, _, ramc) = lagna_mc_ramc_from_lst(lst, 0.5);
        assert!(
            (ramc - lst.rem_euclid(TAU)).abs() < 1e-15,
            "ramc={ramc}, lst={lst}"
        );
    }
}
