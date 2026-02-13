//! IAU 2006 general precession in ecliptic longitude.
//!
//! The general precession p_A measures the accumulated westward motion of
//! the vernal equinox along the ecliptic since J2000.0. This is the
//! foundational quantity for computing ayanamsha at any epoch.
//!
//! Source: Capitaine, Wallace & Chapront 2003, _Astronomy & Astrophysics_
//! 412, 567-586 (Table 1). Also published in IERS Conventions 2010, Ch. 5.
//! Public domain (IAU standard).

/// IAU 2006 general precession in ecliptic longitude, in arcseconds.
///
/// # Arguments
/// * `t` — Julian centuries of TDB since J2000.0: `(JD_TDB - 2451545.0) / 36525.0`
///
/// # Returns
/// Accumulated precession in arcseconds. Positive means the equinox has
/// moved westward (tropical longitudes of stars have increased).
///
/// The dominant linear term is ~5028.80″/century ≈ 1.3969°/century.
pub fn general_precession_longitude_arcsec(t: f64) -> f64 {
    let t2 = t * t;
    let t3 = t2 * t;
    let t4 = t3 * t;
    let t5 = t4 * t;
    5028.796195 * t + 1.1054348 * t2 + 0.00007964 * t3 - 0.000023857 * t4 - 0.0000000383 * t5
}

/// IAU 2006 general precession in ecliptic longitude, in degrees.
///
/// Same as [`general_precession_longitude_arcsec`] but converted to degrees.
pub fn general_precession_longitude_deg(t: f64) -> f64 {
    general_precession_longitude_arcsec(t) / 3600.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_at_j2000() {
        assert_eq!(general_precession_longitude_arcsec(0.0), 0.0);
    }

    #[test]
    fn one_century_approx() {
        let p = general_precession_longitude_arcsec(1.0);
        // 5028.796195 + 1.1054348 + 0.00007964 - ... ≈ 5029.90
        assert!((p - 5029.90).abs() < 1.0, "p_A(1.0) = {p}");
    }

    #[test]
    fn negative_century() {
        let p = general_precession_longitude_arcsec(-1.0);
        assert!(p < 0.0, "p_A(-1.0) should be negative, got {p}");
    }

    #[test]
    fn rate_per_year() {
        // 1 year = 0.01 century
        let p = general_precession_longitude_arcsec(0.01);
        // ~50.29" per year
        assert!((p - 50.29).abs() < 0.1, "p_A(0.01) = {p}");
    }

    #[test]
    fn deg_conversion_consistent() {
        let t = 0.5;
        let arcsec = general_precession_longitude_arcsec(t);
        let deg = general_precession_longitude_deg(t);
        assert!((deg - arcsec / 3600.0).abs() < 1e-15);
    }
}
