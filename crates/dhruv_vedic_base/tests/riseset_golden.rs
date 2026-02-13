//! Golden-value tests for sunrise/sunset against USNO Solar Calculator.
//!
//! Requires kernel files (de442s.bsp, naif0012.tls) AND IERS EOP file
//! (finals2000A.all). Skips gracefully if any are absent.

use std::path::Path;

use dhruv_core::{Engine, EngineConfig};
use dhruv_time::{EopKernel, LeapSecondKernel};
use dhruv_vedic_base::{
    GeoLocation, RiseSetConfig, RiseSetEvent, RiseSetResult, approximate_local_noon_jd,
    compute_rise_set,
};

const SPK_PATH: &str = "../../data/de442s.bsp";
const LSK_PATH: &str = "../../data/naif0012.tls";
const EOP_PATH: &str = "../../data/finals2000A.all";

fn load_test_resources() -> Option<(Engine, LeapSecondKernel, EopKernel)> {
    if !Path::new(SPK_PATH).exists()
        || !Path::new(LSK_PATH).exists()
        || !Path::new(EOP_PATH).exists()
    {
        eprintln!("Skipping riseset_golden: kernel/EOP files not found");
        return None;
    }

    let config = EngineConfig {
        spk_paths: vec![SPK_PATH.into()],
        lsk_path: LSK_PATH.into(),
        cache_capacity: 1024,
        strict_validation: false,
    };
    let engine = Engine::new(config).ok()?;
    let lsk = LeapSecondKernel::load(Path::new(LSK_PATH)).ok()?;
    let eop = EopKernel::load(Path::new(EOP_PATH)).ok()?;
    Some((engine, lsk, eop))
}

/// Convert JD TDB to approximate hours UTC (for assertion messages).
fn jd_tdb_to_approx_utc_hours(jd_tdb: f64) -> f64 {
    // Rough: TDB ≈ UTC for display purposes
    let frac = jd_tdb - jd_tdb.floor();
    // JD starts at noon, so 0.0 frac = 12:00
    ((frac + 0.5).rem_euclid(1.0)) * 24.0
}

/// JD UTC for a date at 0h UT.
fn jd_0h_utc(year: i32, month: u32, day: u32) -> f64 {
    dhruv_time::calendar_to_jd(year, month, day as f64)
}

#[test]
fn new_delhi_equinox_sunrise() {
    let Some((engine, lsk, eop)) = load_test_resources() else {
        return;
    };
    let loc = GeoLocation::new(28.6139, 77.209, 0.0);
    let config = RiseSetConfig::default();
    let jd_0h = jd_0h_utc(2024, 3, 20);
    let noon = approximate_local_noon_jd(jd_0h, loc.longitude_deg);

    let result = compute_rise_set(
        &engine,
        &lsk,
        &eop,
        &loc,
        RiseSetEvent::Sunrise,
        noon,
        &config,
    )
    .unwrap();

    if let RiseSetResult::Event { jd_tdb, .. } = result {
        let hours = jd_tdb_to_approx_utc_hours(jd_tdb);
        // Expected: ~00:48 UTC (06:18 IST)
        assert!(
            (hours - 0.8).abs() < 0.1, // 2 min tolerance = ~0.033h, use wider for approx
            "New Delhi sunrise = {hours:.2}h UTC, expected ~0.8h"
        );
    } else {
        panic!("Expected sunrise event, got {result:?}");
    }
}

#[test]
fn new_delhi_equinox_sunset() {
    let Some((engine, lsk, eop)) = load_test_resources() else {
        return;
    };
    let loc = GeoLocation::new(28.6139, 77.209, 0.0);
    let config = RiseSetConfig::default();
    let jd_0h = jd_0h_utc(2024, 3, 20);
    let noon = approximate_local_noon_jd(jd_0h, loc.longitude_deg);

    let result = compute_rise_set(
        &engine,
        &lsk,
        &eop,
        &loc,
        RiseSetEvent::Sunset,
        noon,
        &config,
    )
    .unwrap();

    if let RiseSetResult::Event { jd_tdb, .. } = result {
        let hours = jd_tdb_to_approx_utc_hours(jd_tdb);
        // Expected: ~12.92 UTC (18:25 IST)
        assert!(
            (hours - 12.9).abs() < 0.1,
            "New Delhi sunset = {hours:.2}h UTC, expected ~12.9h"
        );
    } else {
        panic!("Expected sunset event, got {result:?}");
    }
}

#[test]
fn tromso_summer_never_sets() {
    let Some((engine, lsk, eop)) = load_test_resources() else {
        return;
    };
    let loc = GeoLocation::new(69.65, 18.96, 0.0);
    let config = RiseSetConfig::default();
    let jd_0h = jd_0h_utc(2024, 6, 21);
    let noon = approximate_local_noon_jd(jd_0h, loc.longitude_deg);

    let result = compute_rise_set(
        &engine,
        &lsk,
        &eop,
        &loc,
        RiseSetEvent::Sunrise,
        noon,
        &config,
    )
    .unwrap();

    assert_eq!(
        result,
        RiseSetResult::NeverSets,
        "Tromso summer solstice should be midnight sun"
    );
}

#[test]
fn tromso_winter_never_rises() {
    let Some((engine, lsk, eop)) = load_test_resources() else {
        return;
    };
    let loc = GeoLocation::new(69.65, 18.96, 0.0);
    let config = RiseSetConfig::default();
    let jd_0h = jd_0h_utc(2024, 12, 21);
    let noon = approximate_local_noon_jd(jd_0h, loc.longitude_deg);

    let result = compute_rise_set(
        &engine,
        &lsk,
        &eop,
        &loc,
        RiseSetEvent::Sunrise,
        noon,
        &config,
    )
    .unwrap();

    assert_eq!(
        result,
        RiseSetResult::NeverRises,
        "Tromso winter solstice should be polar night"
    );
}
