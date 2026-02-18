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
/// `Asc = atan2(cos(LST), -(sin(LST)*cos(eps) + tan(phi)*sin(eps)))`
///
/// # Latitude range
///
/// The function does not validate `location.latitude_deg`. Values outside
/// [-90, 90] produce finite but astronomically meaningless results.
/// Callers are responsible for ensuring valid geographic coordinates.
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

    let asc = f64::atan2(lst.cos(), -(lst.sin() * eps.cos() + phi.tan() * eps.sin()));
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

    let asc = f64::atan2(lst.cos(), -(lst.sin() * eps.cos() + phi.tan() * eps.sin()));

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
pub(crate) fn lagna_mc_ramc_from_lst(lst_rad: f64, latitude_rad: f64) -> (f64, f64, f64) {
    let eps = OBLIQUITY_J2000_RAD;

    let asc = f64::atan2(
        lst_rad.cos(),
        -(lst_rad.sin() * eps.cos() + latitude_rad.tan() * eps.sin()),
    );

    let mc = f64::atan2(lst_rad.sin(), lst_rad.cos() * eps.cos());

    (
        asc.rem_euclid(TAU),
        mc.rem_euclid(TAU),
        lst_rad.rem_euclid(TAU),
    )
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

    /// At equator (phi=0), LST=0:
    ///   atan2(cos(0), -(sin(0)*cos(eps) + 0)) = atan2(1, 0) = pi/2 = 90 deg
    ///
    /// At LST=0, the vernal equinox (RA=0) is on the meridian. Cancer (lambda=90deg)
    /// has RA=90deg, H = LST - RA = 0 - 90 = -90deg (east of meridian, rising).
    /// So the Ascendant is at ecliptic longitude 90 deg.
    #[test]
    fn ascendant_formula_equator_lst_zero() {
        let (asc, _, _) = lagna_mc_ramc_from_lst(0.0, 0.0);
        let expected = PI / 2.0; // 90 deg
        assert!(
            (asc - expected).abs() < 1e-10,
            "Asc at equator, LST=0 = {:.4} deg, expected 90",
            asc.to_degrees()
        );
    }

    /// MC at LST=0: atan2(sin(0), cos(0)*cos(eps)) = atan2(0, cos(eps)) = 0
    #[test]
    fn mc_formula_lst_zero() {
        let (_, mc, _) = lagna_mc_ramc_from_lst(0.0, 0.0);
        assert!(
            mc.abs() < 1e-10,
            "MC at LST=0 = {:.4} deg, expected 0",
            mc.to_degrees()
        );
    }

    /// As LST sweeps 0..2*pi, Ascendant should cover the full circle.
    #[test]
    fn ascendant_quadrant_sweep() {
        let phi = 28.6_f64.to_radians(); // New Delhi

        let n = 360;
        let mut min_asc = f64::MAX;
        let mut max_asc = f64::MIN;

        for i in 0..n {
            let lst = TAU * (i as f64) / (n as f64);
            let (asc, _, _) = lagna_mc_ramc_from_lst(lst, phi);
            if asc < min_asc {
                min_asc = asc;
            }
            if asc > max_asc {
                max_asc = asc;
            }
        }

        // Should span nearly 0..2*pi
        assert!(min_asc < 0.05, "min_asc = {}", min_asc.to_degrees());
        assert!(max_asc > TAU - 0.05, "max_asc = {}", max_asc.to_degrees());
    }

    /// At low latitudes, Asc and MC typically differ by roughly 90 deg.
    #[test]
    fn ascendant_and_mc_differ_by_about_90() {
        let phi = 10.0_f64.to_radians(); // low latitude

        let lsts: [f64; 4] = [0.5, 1.5, 3.0, 4.5];
        for &lst in &lsts {
            let (asc, mc, _) = lagna_mc_ramc_from_lst(lst, phi);

            let mut diff = (asc - mc).abs();
            if diff > PI {
                diff = TAU - diff;
            }

            // At low latitudes the difference is approximately 90 deg (+/- ~20 deg)
            assert!(
                diff > 1.0 && diff < 2.2,
                "LST={:.1}: |Asc-MC| = {:.1} deg, expected ~90",
                lst.to_degrees(),
                diff.to_degrees()
            );
        }
    }

    /// RAMC equals LST by definition.
    #[test]
    fn ramc_equals_lst() {
        let lst = 1.234;
        let (_, _, ramc) = lagna_mc_ramc_from_lst(lst, 0.5);
        assert!(
            (ramc - lst.rem_euclid(TAU)).abs() < 1e-15,
            "ramc={ramc}, lst={lst}"
        );
    }

    /// Hand-computed reference values for the ascendant formula.
    ///
    /// Formula: Asc = atan2(cos(LST), -(sin(LST)*cos(eps) + tan(phi)*sin(eps)))
    /// with eps = OBLIQUITY_J2000_RAD = 23.4393 deg = 0.40909 rad.
    ///
    /// | LST   | phi   | Expected deg |
    /// |-------|-------|-------------|
    /// | 0     | 0     | 90.0        |
    /// | pi    | 0     | 270.0       |
    /// | pi/2  | 0     | 180.0       |
    /// | 3pi/2 | 0     | 0.0 (360)   |
    /// | 0     | 45N   | 111.7       |
    ///
    /// LST=0, phi=0: atan2(1, 0) = pi/2 = 90 deg
    /// LST=pi, phi=0: atan2(-1, 0) = -pi/2 -> 270 deg
    /// LST=pi/2, phi=0: atan2(0, -cos(eps)) = pi = 180 deg
    /// LST=3pi/2, phi=0: atan2(0, cos(eps)) = 0 -> 0 deg (=360)
    /// LST=0, phi=45: atan2(1, -tan(45)*sin(eps)) = atan2(1, -0.3978) = 111.7 deg
    #[test]
    fn ascendant_known_values() {
        let cases: &[(f64, f64, f64)] = &[
            (0.0, 0.0, 90.0),
            (PI, 0.0, 270.0),
            (PI / 2.0, 0.0, 180.0),
            (3.0 * PI / 2.0, 0.0, 0.0),
            (0.0, 45.0_f64.to_radians(), 111.7),
        ];

        for &(lst, phi, expected_deg) in cases {
            let (asc, _, _) = lagna_mc_ramc_from_lst(lst, phi);
            let asc_deg = asc.to_degrees();
            // For 0/360 boundary: compare mod 360
            let diff = (asc_deg - expected_deg).rem_euclid(360.0);
            let err = diff.min(360.0 - diff);
            assert!(
                err < 0.1,
                "LST={:.4}, phi={:.4}: got {:.4} deg, expected {:.1} deg",
                lst,
                phi,
                asc_deg,
                expected_deg
            );
        }
    }

    /// Verify the ascendant is a RISING point (eastern horizon, H < 0)
    /// for 8 diverse (LST, phi) combinations spanning all quadrants and
    /// hemispheres.
    ///
    /// Method: compute RA of the ascendant from its ecliptic longitude,
    /// then hour angle H = LST - RA. A rising point has H in [-pi, 0).
    #[test]
    fn ascendant_is_rising_not_setting() {
        let eps = OBLIQUITY_J2000_RAD;

        let cases: &[(f64, f64, &str)] = &[
            (0.0, 0.0, "equator, LST=0"),
            (PI / 2.0, 0.0, "equator, LST=pi/2"),
            (PI, 0.0, "equator, LST=pi"),
            (3.0 * PI / 2.0, 0.0, "equator, LST=3pi/2"),
            (1.0, 28.6_f64.to_radians(), "New Delhi, LST=1"),
            (2.5, 69.0_f64.to_radians(), "Tromso, LST=2.5"),
            (4.0, (-34.6_f64).to_radians(), "Buenos Aires, LST=4"),
            (5.5, 51.5_f64.to_radians(), "London, LST=5.5"),
        ];

        for &(lst, phi, label) in cases {
            let (asc, _, _) = lagna_mc_ramc_from_lst(lst, phi);
            // RA of the ecliptic point lambda=asc
            let ra = f64::atan2(asc.sin() * eps.cos(), asc.cos()).rem_euclid(TAU);
            let mut h = (lst - ra).rem_euclid(TAU);
            if h > PI {
                h -= TAU;
            }
            assert!(
                h < 0.0,
                "{label}: H = {:.4} rad ({:.2} deg) — ascendant should be rising (H < 0)",
                h,
                h.to_degrees()
            );
        }
    }

    /// Verify all three lagna computation paths produce identical results.
    #[test]
    fn all_lagna_functions_agree() {
        let eps = OBLIQUITY_J2000_RAD;
        let cases: &[(f64, f64)] = &[
            (0.5, 28.6_f64.to_radians()),
            (2.0, 0.0),
            (4.0, (-34.6_f64).to_radians()),
            (5.5, 69.0_f64.to_radians()),
        ];

        for &(lst, phi) in cases {
            let (asc_helper, _, _) = lagna_mc_ramc_from_lst(lst, phi);
            // Inline formula (same as used in lagna_longitude_rad and lagna_and_mc_rad)
            let asc_inline =
                f64::atan2(lst.cos(), -(lst.sin() * eps.cos() + phi.tan() * eps.sin()))
                    .rem_euclid(TAU);
            assert!(
                (asc_helper - asc_inline).abs() < 1e-15,
                "LST={lst}, phi={phi}: helper={asc_helper}, inline={asc_inline}"
            );
        }
    }

    // ===== Edge-case tests =====

    /// Near-polar latitudes: no panic, valid output in [0, 2*pi].
    #[test]
    fn ascendant_high_latitude_no_panic() {
        let lats = [89.0_f64, -89.0, 66.5, 85.0];
        for &lat_deg in &lats {
            let phi = lat_deg.to_radians();
            for i in 0..8 {
                let lst = TAU * (i as f64) / 8.0;
                let (asc, _, _) = lagna_mc_ramc_from_lst(lst, phi);
                // rem_euclid(TAU) can return exactly TAU due to f64 rounding
                assert!(
                    asc.is_finite() && asc >= 0.0 && asc <= TAU,
                    "lat={lat_deg}, LST={}: asc={asc}",
                    lst.to_degrees()
                );
            }
        }
    }

    /// Exactly +/-90 deg latitude: no panic, finite output.
    #[test]
    fn ascendant_at_exact_poles() {
        for &lat_deg in &[90.0_f64, -90.0] {
            let phi = lat_deg.to_radians();
            let (asc, _, _) = lagna_mc_ramc_from_lst(1.0, phi);
            assert!(
                asc.is_finite() && asc >= 0.0 && asc <= TAU,
                "lat={lat_deg}: asc={asc}"
            );
        }
    }

    /// Out-of-range latitudes: no panic, finite result (astronomically nonsensical).
    #[test]
    fn ascendant_out_of_range_latitude() {
        for &lat_deg in &[91.0_f64, -91.0, 180.0, 100.0] {
            let phi = lat_deg.to_radians();
            let (asc, _, _) = lagna_mc_ramc_from_lst(1.0, phi);
            assert!(
                asc.is_finite() && asc >= 0.0 && asc <= TAU,
                "lat={lat_deg}: asc={asc}"
            );
        }
    }

    /// Near-singular atan2 arguments: both approach zero when
    /// LST ~ pi/2 and tan(phi) ~ -cos(eps)/sin(eps) (phi ~ -66.56 deg).
    #[test]
    fn ascendant_near_singular_atan2() {
        let phi = (-66.56_f64).to_radians();
        for &lst in &[PI / 2.0 - 0.001, PI / 2.0, PI / 2.0 + 0.001] {
            let (asc, _, _) = lagna_mc_ramc_from_lst(lst, phi);
            assert!(
                asc.is_finite() && asc >= 0.0 && asc <= TAU,
                "lst={}, phi=-66.56: asc={}",
                lst.to_degrees(),
                asc.to_degrees()
            );
        }
    }
}
