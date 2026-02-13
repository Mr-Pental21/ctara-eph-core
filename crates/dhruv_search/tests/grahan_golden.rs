//! Golden-value integration tests for grahan computation.
//!
//! Validates against NASA Five Millennium Eclipse Catalog data.
//! Requires kernel files (de442s.bsp, naif0012.tls). Skips gracefully if absent.

use std::path::Path;

use dhruv_core::{Engine, EngineConfig};
use dhruv_search::{
    ChandraGrahanType, GrahanConfig, next_chandra_grahan, next_surya_grahan, prev_chandra_grahan,
    prev_surya_grahan, search_chandra_grahan, search_surya_grahan,
};

const SPK_PATH: &str = "../../kernels/data/de442s.bsp";
const LSK_PATH: &str = "../../kernels/data/naif0012.tls";

fn load_engine() -> Option<Engine> {
    if !Path::new(SPK_PATH).exists() || !Path::new(LSK_PATH).exists() {
        eprintln!("Skipping grahan_golden: kernel files not found");
        return None;
    }
    let config = EngineConfig::with_single_spk(SPK_PATH.into(), LSK_PATH.into(), 1024, false);
    Engine::new(config).ok()
}

fn jd_from_date(year: i32, month: u32, day: f64) -> f64 {
    dhruv_time::calendar_to_jd(year, month, day)
}

// ---------------------------------------------------------------------------
// Chandra grahan (lunar eclipses)
// ---------------------------------------------------------------------------

/// 2024-Mar-25: Penumbral chandra grahan
/// NASA catalog: Greatest eclipse 07:13 UTC
#[test]
fn chandra_grahan_2024_mar_penumbral() {
    let Some(engine) = load_engine() else { return };
    let jd_start = jd_from_date(2024, 3, 1.0);
    let config = GrahanConfig::default();
    let result = next_chandra_grahan(&engine, jd_start, &config).expect("search should succeed");
    let grahan = result.expect("should find a chandra grahan");

    // Should be in March 2024
    let expected_jd = jd_from_date(2024, 3, 25.3); // ~07:13 UTC
    let diff_hours = (grahan.greatest_grahan_jd - expected_jd).abs() * 24.0;
    assert!(
        diff_hours < 12.0,
        "chandra grahan off by {diff_hours:.1}h, got JD {}, expected ~JD {}",
        grahan.greatest_grahan_jd,
        expected_jd
    );
    assert_eq!(grahan.grahan_type, ChandraGrahanType::Penumbral);
}

/// 2025-Mar-14: Total chandra grahan
/// NASA catalog: Greatest eclipse ~06:59 UTC, magnitude 1.178
#[test]
fn chandra_grahan_2025_mar_total() {
    let Some(engine) = load_engine() else { return };
    let jd_start = jd_from_date(2025, 3, 1.0);
    let config = GrahanConfig::default();
    let result = next_chandra_grahan(&engine, jd_start, &config).expect("search should succeed");
    let grahan = result.expect("should find a chandra grahan");

    let expected_jd = jd_from_date(2025, 3, 14.29); // ~06:59 UTC
    let diff_hours = (grahan.greatest_grahan_jd - expected_jd).abs() * 24.0;
    assert!(
        diff_hours < 12.0,
        "chandra grahan off by {diff_hours:.1}h, got JD {}",
        grahan.greatest_grahan_jd
    );
    assert_eq!(grahan.grahan_type, ChandraGrahanType::Total);
    // Magnitude should be > 1 for total
    assert!(
        grahan.magnitude > 1.0,
        "total chandra grahan magnitude = {}, expected > 1",
        grahan.magnitude
    );
}

/// Search for chandra grahan in 2024 — should find 2 (Mar and Sep).
#[test]
fn chandra_grahan_2024_count() {
    let Some(engine) = load_engine() else { return };
    let jd_start = jd_from_date(2024, 1, 1.0);
    let jd_end = jd_from_date(2025, 1, 1.0);
    let config = GrahanConfig::default();
    let results =
        search_chandra_grahan(&engine, jd_start, jd_end, &config).expect("search should succeed");

    // 2024 has 2 chandra grahan: Mar 25 (penumbral) and Sep 18 (partial)
    assert!(
        results.len() >= 2,
        "found {} chandra grahan in 2024, expected at least 2",
        results.len()
    );
}

/// Penumbral-only filter: exclude penumbral grahan.
#[test]
fn penumbral_filter() {
    let Some(engine) = load_engine() else { return };
    let jd_start = jd_from_date(2024, 1, 1.0);
    let jd_end = jd_from_date(2025, 1, 1.0);
    let config = GrahanConfig {
        include_penumbral: false,
        ..Default::default()
    };
    let results =
        search_chandra_grahan(&engine, jd_start, jd_end, &config).expect("search should succeed");

    // With penumbral excluded, should have fewer grahan
    for e in &results {
        assert_ne!(
            e.grahan_type,
            ChandraGrahanType::Penumbral,
            "penumbral grahan should be filtered"
        );
    }
}

/// Backward search for previous chandra grahan.
#[test]
fn prev_chandra_grahan_from_2024() {
    let Some(engine) = load_engine() else { return };
    let jd = jd_from_date(2024, 3, 1.0);
    let config = GrahanConfig::default();
    let result = prev_chandra_grahan(&engine, jd, &config).expect("search should succeed");
    let grahan = result.expect("should find previous chandra grahan");

    // Previous chandra grahan should be before our search date
    assert!(grahan.greatest_grahan_jd < jd);
    // Contact times should be ordered: P1 < greatest < P4
    assert!(grahan.p1_jd < grahan.greatest_grahan_jd);
    assert!(grahan.greatest_grahan_jd < grahan.p4_jd);
}

// ---------------------------------------------------------------------------
// Surya grahan (solar eclipses)
// ---------------------------------------------------------------------------

/// 2024-Apr-08: Surya grahan (Total for surface observers).
/// NASA catalog: Greatest eclipse 18:18 UTC.
/// Geocentric classification may differ from surface classification.
#[test]
fn surya_grahan_2024_apr() {
    let Some(engine) = load_engine() else { return };
    let jd_start = jd_from_date(2024, 3, 1.0);
    let config = GrahanConfig::default();
    let result = next_surya_grahan(&engine, jd_start, &config).expect("search should succeed");
    let grahan = result.expect("should find a surya grahan");

    let expected_jd = jd_from_date(2024, 4, 8.763); // ~18:18 UTC
    let diff_hours = (grahan.greatest_grahan_jd - expected_jd).abs() * 24.0;
    assert!(
        diff_hours < 12.0,
        "surya grahan off by {diff_hours:.1}h, got JD {}, expected ~JD {}",
        grahan.greatest_grahan_jd,
        expected_jd
    );
    // Geocentric: could be Partial or Total depending on exact geometry
    // Magnitude should be close to 1.0 (Moon is close to Sun's size)
    assert!(
        grahan.magnitude > 0.90,
        "surya grahan magnitude = {}, expected > 0.90",
        grahan.magnitude
    );
}

/// 2024-Oct-02: Surya grahan (Annular for surface observers).
/// NASA catalog: Greatest eclipse ~18:45 UTC.
/// Geocentric classification may differ from surface classification.
#[test]
fn surya_grahan_2024_oct() {
    let Some(engine) = load_engine() else { return };
    let jd_start = jd_from_date(2024, 9, 1.0);
    let config = GrahanConfig::default();
    let result = next_surya_grahan(&engine, jd_start, &config).expect("search should succeed");
    let grahan = result.expect("should find a surya grahan");

    let expected_jd = jd_from_date(2024, 10, 2.78); // ~18:45 UTC
    let diff_hours = (grahan.greatest_grahan_jd - expected_jd).abs() * 24.0;
    assert!(
        diff_hours < 12.0,
        "surya grahan off by {diff_hours:.1}h, got JD {}",
        grahan.greatest_grahan_jd
    );
    // Geocentric: could be Partial or Annular
    assert!(
        grahan.magnitude > 0.90,
        "surya grahan magnitude = {}, expected > 0.90",
        grahan.magnitude
    );
}

/// Search for surya grahan in 2024 — should find 2 (Apr total, Oct annular).
#[test]
fn surya_grahan_2024_count() {
    let Some(engine) = load_engine() else { return };
    let jd_start = jd_from_date(2024, 1, 1.0);
    let jd_end = jd_from_date(2025, 1, 1.0);
    let config = GrahanConfig::default();
    let results =
        search_surya_grahan(&engine, jd_start, jd_end, &config).expect("search should succeed");

    assert!(
        results.len() >= 2,
        "found {} surya grahan in 2024, expected at least 2",
        results.len()
    );
}

/// Backward search for previous surya grahan.
#[test]
fn prev_surya_grahan_from_2024() {
    let Some(engine) = load_engine() else { return };
    let jd = jd_from_date(2024, 3, 1.0);
    let config = GrahanConfig::default();
    let result = prev_surya_grahan(&engine, jd, &config).expect("search should succeed");
    let grahan = result.expect("should find previous surya grahan");

    assert!(grahan.greatest_grahan_jd < jd);
    // Contact times C1 < greatest < C4 (if present)
    if let Some(c1) = grahan.c1_jd {
        assert!(c1 < grahan.greatest_grahan_jd);
    }
    if let Some(c4) = grahan.c4_jd {
        assert!(grahan.greatest_grahan_jd < c4);
    }
}
