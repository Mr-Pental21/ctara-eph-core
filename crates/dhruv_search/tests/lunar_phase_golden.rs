//! Golden-value integration tests for Purnima/Amavasya search.
//!
//! Validates against NASA new/full moon dates.
//! Requires kernel files. Skips gracefully if absent.

use std::path::Path;

use dhruv_core::{Engine, EngineConfig};
use dhruv_search::{
    next_amavasya, next_purnima, prev_amavasya, prev_purnima, search_amavasyas, search_purnimas,
};
use dhruv_time::UtcTime;

const SPK_PATH: &str = "../../kernels/data/de442s.bsp";
const LSK_PATH: &str = "../../kernels/data/naif0012.tls";

fn load_engine() -> Option<Engine> {
    if !Path::new(SPK_PATH).exists() || !Path::new(LSK_PATH).exists() {
        eprintln!("Skipping lunar_phase_golden: kernel files not found");
        return None;
    }
    let config = EngineConfig::with_single_spk(SPK_PATH.into(), LSK_PATH.into(), 1024, false);
    Engine::new(config).ok()
}

/// NASA: Full Moon 2024-Jan-25 ~17:54 UTC
#[test]
fn purnima_jan_2024() {
    let Some(engine) = load_engine() else { return };
    let utc = UtcTime::new(2024, 1, 1, 0, 0, 0.0);
    let event = next_purnima(&engine, &utc)
        .unwrap()
        .expect("should find purnima");
    assert_eq!(event.utc.year, 2024);
    assert_eq!(event.utc.month, 1);
    assert_eq!(event.utc.day, 25);
    // Within 2 hours of 17:54 UTC
    let diff_hours = (event.utc.hour as f64 + event.utc.minute as f64 / 60.0 - 17.9).abs();
    assert!(
        diff_hours < 2.0,
        "off by {diff_hours:.1}h, got {}",
        event.utc
    );
}

/// NASA: New Moon 2024-Jan-11 ~11:57 UTC
#[test]
fn amavasya_jan_2024() {
    let Some(engine) = load_engine() else { return };
    let utc = UtcTime::new(2024, 1, 1, 0, 0, 0.0);
    let event = next_amavasya(&engine, &utc)
        .unwrap()
        .expect("should find amavasya");
    assert_eq!(event.utc.year, 2024);
    assert_eq!(event.utc.month, 1);
    assert_eq!(event.utc.day, 11);
    let diff_hours = (event.utc.hour as f64 + event.utc.minute as f64 / 60.0 - 11.95).abs();
    assert!(
        diff_hours < 2.0,
        "off by {diff_hours:.1}h, got {}",
        event.utc
    );
}

/// NASA: Full Moon 2024-Feb-24 ~12:30 UTC
#[test]
fn purnima_feb_2024() {
    let Some(engine) = load_engine() else { return };
    let utc = UtcTime::new(2024, 2, 1, 0, 0, 0.0);
    let event = next_purnima(&engine, &utc)
        .unwrap()
        .expect("should find purnima");
    assert_eq!(event.utc.year, 2024);
    assert_eq!(event.utc.month, 2);
    assert_eq!(event.utc.day, 24);
}

/// Search for all Purnimas in 2024 — should find 12 or 13
#[test]
fn search_purnimas_2024() {
    let Some(engine) = load_engine() else { return };
    let start = UtcTime::new(2024, 1, 1, 0, 0, 0.0);
    let end = UtcTime::new(2025, 1, 1, 0, 0, 0.0);
    let events = search_purnimas(&engine, &start, &end).unwrap();
    // A year has 12 or 13 full moons
    assert!(
        events.len() >= 12 && events.len() <= 13,
        "expected 12-13 purnimas, got {}",
        events.len()
    );
    // Should be in chronological order
    for w in events.windows(2) {
        let jd0 = w[0].utc.to_jd_tdb(engine.lsk());
        let jd1 = w[1].utc.to_jd_tdb(engine.lsk());
        assert!(jd1 > jd0, "events not in order");
    }
}

/// Search for all Amavasyas in 2024
#[test]
fn search_amavasyas_2024() {
    let Some(engine) = load_engine() else { return };
    let start = UtcTime::new(2024, 1, 1, 0, 0, 0.0);
    let end = UtcTime::new(2025, 1, 1, 0, 0, 0.0);
    let events = search_amavasyas(&engine, &start, &end).unwrap();
    assert!(
        events.len() >= 12 && events.len() <= 13,
        "expected 12-13 amavasyas, got {}",
        events.len()
    );
}

/// prev_purnima should find a full moon before the given date
#[test]
fn prev_purnima_from_feb_2024() {
    let Some(engine) = load_engine() else { return };
    let utc = UtcTime::new(2024, 2, 15, 0, 0, 0.0);
    let event = prev_purnima(&engine, &utc)
        .unwrap()
        .expect("should find prev purnima");
    // Should find Jan 25 full moon
    assert_eq!(event.utc.year, 2024);
    assert_eq!(event.utc.month, 1);
    assert_eq!(event.utc.day, 25);
}

/// prev_amavasya should find a new moon before the given date
#[test]
fn prev_amavasya_from_feb_2024() {
    let Some(engine) = load_engine() else { return };
    let utc = UtcTime::new(2024, 2, 1, 0, 0, 0.0);
    let event = prev_amavasya(&engine, &utc)
        .unwrap()
        .expect("should find prev amavasya");
    // Should find Jan 11 new moon
    assert_eq!(event.utc.year, 2024);
    assert_eq!(event.utc.month, 1);
    assert_eq!(event.utc.day, 11);
}

/// Moon longitude at full moon should be ~180 deg from Sun
#[test]
fn purnima_longitude_opposition() {
    let Some(engine) = load_engine() else { return };
    let utc = UtcTime::new(2024, 3, 1, 0, 0, 0.0);
    let event = next_purnima(&engine, &utc)
        .unwrap()
        .expect("should find purnima");
    let diff = (event.moon_longitude_deg - event.sun_longitude_deg).abs();
    let sep = if diff > 180.0 { 360.0 - diff } else { diff };
    assert!(
        (sep - 180.0).abs() < 1.0,
        "expected ~180 deg separation, got {sep:.2}"
    );
}

/// Moon longitude at new moon should be ~0 deg from Sun
#[test]
fn amavasya_longitude_conjunction() {
    let Some(engine) = load_engine() else { return };
    let utc = UtcTime::new(2024, 3, 1, 0, 0, 0.0);
    let event = next_amavasya(&engine, &utc)
        .unwrap()
        .expect("should find amavasya");
    let diff = (event.moon_longitude_deg - event.sun_longitude_deg).abs();
    let sep = if diff > 180.0 { 360.0 - diff } else { diff };
    assert!(sep < 1.0, "expected ~0 deg separation, got {sep:.2}");
}
