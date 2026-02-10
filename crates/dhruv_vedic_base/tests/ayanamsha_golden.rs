//! Golden-value tests for ayanamsha against published almanac values.
//!
//! No kernel files needed — ayanamsha depends only on the IAU 2006
//! precession polynomial (pure math).

use dhruv_vedic_base::{AyanamshaSystem, ayanamsha_mean_deg, jd_tdb_to_centuries};

/// Helper: JD TDB for a given year-month-day 0h TDB (approximate).
/// Uses a simple calendar-to-JD conversion.
fn jd_from_date(year: i32, month: u32, day: u32) -> f64 {
    dhruv_time::calendar_to_jd(year, month, day as f64)
}

#[test]
fn lahiri_at_j2000() {
    // Indian Astronomical Ephemeris: Lahiri at J2000.0 ≈ 23.85°
    let t = jd_tdb_to_centuries(2_451_545.0); // J2000.0
    let val = ayanamsha_mean_deg(AyanamshaSystem::Lahiri, t);
    assert!(
        (val - 23.85).abs() < 0.01,
        "Lahiri at J2000 = {val}, expected ~23.85"
    );
}

#[test]
fn lahiri_at_2024() {
    // Rashtriya Panchang 2024: Lahiri ayanamsha ~24.17°
    let jd = jd_from_date(2024, 1, 1);
    let t = jd_tdb_to_centuries(jd);
    let val = ayanamsha_mean_deg(AyanamshaSystem::Lahiri, t);
    assert!(
        (val - 24.19).abs() < 0.05,
        "Lahiri at 2024-01-01 = {val}, expected ~24.19"
    );
}

#[test]
fn fagan_bradley_at_j2000() {
    // Published Western sidereal tables: F-B at J2000.0 ≈ 24.74°
    let val = ayanamsha_mean_deg(AyanamshaSystem::FaganBradley, 0.0);
    assert!(
        (val - 24.74).abs() < 0.02,
        "FaganBradley at J2000 = {val}, expected ~24.74"
    );
}

#[test]
fn raman_at_j2000() {
    // B.V. Raman tables: Raman at J2000.0 ≈ 22.37°
    let val = ayanamsha_mean_deg(AyanamshaSystem::Raman, 0.0);
    assert!(
        (val - 22.37).abs() < 0.02,
        "Raman at J2000 = {val}, expected ~22.37"
    );
}

#[test]
fn all_systems_increase_over_century() {
    // All systems should increase by ~1.397°/century (precession rate)
    for &sys in AyanamshaSystem::all() {
        let at_0 = ayanamsha_mean_deg(sys, 0.0);
        let at_1 = ayanamsha_mean_deg(sys, 1.0);
        let diff = at_1 - at_0;
        assert!(
            (diff - 1.397).abs() < 0.01,
            "{sys:?}: century drift = {diff}, expected ~1.397"
        );
    }
}

#[test]
fn systems_ordered_at_j2000() {
    // Sassanian < UshaShashi < PushyaPaksha < ... < GalacticCenter0Sag
    // Just check min and max are reasonable
    let min = AyanamshaSystem::all()
        .iter()
        .map(|s| s.reference_j2000_deg())
        .fold(f64::INFINITY, f64::min);
    let max = AyanamshaSystem::all()
        .iter()
        .map(|s| s.reference_j2000_deg())
        .fold(f64::NEG_INFINITY, f64::max);
    assert!(min > 19.0, "minimum reference = {min}");
    assert!(max < 28.0, "maximum reference = {max}");
    assert!(max - min > 5.0, "range = {} too narrow", max - min);
}
