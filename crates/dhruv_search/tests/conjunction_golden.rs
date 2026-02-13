//! Golden-value integration tests for conjunction/opposition search.
//!
//! Validates against JPL Horizons new/full moon dates and planetary conjunctions.
//! Requires kernel files (de442s.bsp, naif0012.tls). Skips gracefully if absent.

use std::path::Path;

use dhruv_core::{Body, Engine, EngineConfig};
use dhruv_search::{ConjunctionConfig, next_conjunction, prev_conjunction, search_conjunctions};

const SPK_PATH: &str = "../../kernels/data/de442s.bsp";
const LSK_PATH: &str = "../../kernels/data/naif0012.tls";

fn load_engine() -> Option<Engine> {
    if !Path::new(SPK_PATH).exists() || !Path::new(LSK_PATH).exists() {
        eprintln!("Skipping conjunction_golden: kernel files not found");
        return None;
    }
    let config = EngineConfig::with_single_spk(SPK_PATH.into(), LSK_PATH.into(), 1024, false);
    Engine::new(config).ok()
}

fn jd_from_date(year: i32, month: u32, day: f64) -> f64 {
    dhruv_time::calendar_to_jd(year, month, day)
}

/// New moon: Sun-Moon conjunction (0 deg).
/// 2024-Jan-11 ~11:57 UTC → JD TDB ~2460320.0
/// Horizons: 2024-Jan-11 11:57 UTC
#[test]
fn new_moon_jan_2024() {
    let Some(engine) = load_engine() else { return };
    let jd_start = jd_from_date(2024, 1, 1.0);
    let config = ConjunctionConfig::conjunction(0.5);
    let result = next_conjunction(&engine, Body::Sun, Body::Moon, jd_start, &config)
        .expect("search should succeed");
    let event = result.expect("should find a new moon");

    // New moon ~2024-Jan-11 11:57 UTC ≈ JD 2460320.998
    let expected_jd = jd_from_date(2024, 1, 11.498); // ~11:57 UTC
    let diff_hours = (event.jd_tdb - expected_jd).abs() * 24.0;
    assert!(
        diff_hours < 2.0,
        "new moon off by {diff_hours:.1}h, got JD {}, expected ~JD {}",
        event.jd_tdb,
        expected_jd
    );
    // Separation should be near 0
    assert!(
        event.actual_separation_deg < 1.0,
        "separation = {} deg",
        event.actual_separation_deg
    );
}

/// Full moon: Sun-Moon opposition (180 deg).
/// 2024-Jan-25 ~17:54 UTC
#[test]
fn full_moon_jan_2024() {
    let Some(engine) = load_engine() else { return };
    let jd_start = jd_from_date(2024, 1, 20.0);
    let config = ConjunctionConfig::opposition(0.5);
    let result = next_conjunction(&engine, Body::Sun, Body::Moon, jd_start, &config)
        .expect("search should succeed");
    let event = result.expect("should find a full moon");

    let expected_jd = jd_from_date(2024, 1, 25.746); // ~17:54 UTC
    let diff_hours = (event.jd_tdb - expected_jd).abs() * 24.0;
    assert!(
        diff_hours < 2.0,
        "full moon off by {diff_hours:.1}h, got JD {}, expected ~JD {}",
        event.jd_tdb,
        expected_jd
    );
    assert!(
        (event.actual_separation_deg - 180.0).abs() < 1.0,
        "separation = {} deg",
        event.actual_separation_deg
    );
}

/// Jupiter-Saturn conjunction: 2020-Dec-21 ~18:22 UTC.
/// The "Great Conjunction" — closest in centuries.
#[test]
fn jupiter_saturn_conjunction_2020() {
    let Some(engine) = load_engine() else { return };
    let jd_start = jd_from_date(2020, 11, 1.0);
    let config = ConjunctionConfig::conjunction(2.0);
    let result = next_conjunction(&engine, Body::Jupiter, Body::Saturn, jd_start, &config)
        .expect("search should succeed");
    let event = result.expect("should find Jupiter-Saturn conjunction");

    let expected_jd = jd_from_date(2020, 12, 21.765); // ~18:22 UTC
    let diff_days = (event.jd_tdb - expected_jd).abs();
    assert!(
        diff_days < 1.0,
        "great conjunction off by {diff_days:.2} days, got JD {}, expected ~JD {}",
        event.jd_tdb,
        expected_jd
    );
    // Separation should be very small (<1 deg)
    assert!(
        event.actual_separation_deg < 1.0,
        "separation = {} deg",
        event.actual_separation_deg
    );
}

/// Sun-Moon aspect: first quarter moon.
/// With body1=Sun, body2=Moon, first quarter (Moon 90° ahead) means
/// lon_Sun - lon_Moon = -90° = 270° in [0, 360).
#[test]
fn first_quarter_moon_jan_2024() {
    let Some(engine) = load_engine() else { return };
    let jd_start = jd_from_date(2024, 1, 15.0);
    let config = ConjunctionConfig::aspect(270.0, 0.5);
    let result = next_conjunction(&engine, Body::Sun, Body::Moon, jd_start, &config)
        .expect("search should succeed");
    let event = result.expect("should find first quarter");

    // First quarter ~2024-Jan-18
    let expected_jd = jd_from_date(2024, 1, 18.0);
    let diff_days = (event.jd_tdb - expected_jd).abs();
    assert!(diff_days < 2.0, "first quarter off by {diff_days:.1} days");
    assert!(
        (event.actual_separation_deg - 270.0).abs() < 2.0,
        "separation = {} deg, expected ~270",
        event.actual_separation_deg
    );
}

/// Search for multiple new moons in a 3-month window.
#[test]
fn multiple_new_moons_q1_2024() {
    let Some(engine) = load_engine() else { return };
    let jd_start = jd_from_date(2024, 1, 1.0);
    let jd_end = jd_from_date(2024, 4, 1.0);
    let config = ConjunctionConfig::conjunction(0.5);
    let events = search_conjunctions(&engine, Body::Sun, Body::Moon, jd_start, jd_end, &config)
        .expect("search should succeed");

    // ~3 new moons in 3 months
    assert!(
        events.len() >= 2 && events.len() <= 4,
        "found {} new moons, expected 2-4",
        events.len()
    );

    // Check they're ~29.5 days apart
    for window in events.windows(2) {
        let gap = window[1].jd_tdb - window[0].jd_tdb;
        assert!(
            (gap - 29.5).abs() < 2.0,
            "gap between new moons = {gap:.1} days, expected ~29.5"
        );
    }
}

/// Backward search for previous new moon.
#[test]
fn prev_new_moon() {
    let Some(engine) = load_engine() else { return };
    let jd = jd_from_date(2024, 2, 1.0);
    let config = ConjunctionConfig::conjunction(0.5);
    let result = prev_conjunction(&engine, Body::Sun, Body::Moon, jd, &config)
        .expect("search should succeed");
    let event = result.expect("should find previous new moon");

    // Previous new moon ~2024-Jan-11
    assert!(
        event.jd_tdb < jd,
        "previous event should be before search date"
    );
    let expected_jd = jd_from_date(2024, 1, 11.5);
    let diff_days = (event.jd_tdb - expected_jd).abs();
    assert!(diff_days < 2.0, "prev new moon off by {diff_days:.1} days");
}
