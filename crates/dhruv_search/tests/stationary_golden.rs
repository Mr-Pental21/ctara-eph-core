//! Golden-value integration tests for stationary point and max-speed search.
//!
//! Validates against well-known Mercury/Mars retrograde station dates.
//! Requires kernel files (de442s.bsp, naif0012.tls). Skips gracefully if absent.

use std::path::Path;

use dhruv_core::{Body, Engine, EngineConfig};
use dhruv_search::{
    MaxSpeedType, SearchError, StationaryConfig, StationType, next_max_speed, next_stationary,
    prev_stationary, search_stationary,
};

const SPK_PATH: &str = "../../kernels/data/de442s.bsp";
const LSK_PATH: &str = "../../kernels/data/naif0012.tls";

fn load_engine() -> Option<Engine> {
    if !Path::new(SPK_PATH).exists() || !Path::new(LSK_PATH).exists() {
        eprintln!("Skipping stationary_golden: kernel files not found");
        return None;
    }
    let config = EngineConfig::with_single_spk(
        SPK_PATH.into(),
        LSK_PATH.into(),
        1024,
        false,
    );
    Engine::new(config).ok()
}

fn jd_from_date(year: i32, month: u32, day: f64) -> f64 {
    dhruv_time::calendar_to_jd(year, month, day)
}

/// Mercury station retrograde ~2024-Apr-01.
/// Mercury goes retrograde around April 1, 2024 (widely published).
#[test]
fn mercury_station_retrograde_apr_2024() {
    let Some(engine) = load_engine() else { return };
    let jd_start = jd_from_date(2024, 3, 1.0);
    let config = StationaryConfig::inner_planet();
    let result = next_stationary(&engine, Body::Mercury, jd_start, &config)
        .expect("search should succeed");
    let event = result.expect("should find a Mercury station");

    // Mercury station retrograde ~2024-Apr-01 ± a few days
    let expected_jd = jd_from_date(2024, 4, 1.0);
    let diff_days = (event.jd_tdb - expected_jd).abs();
    assert!(
        diff_days < 3.0,
        "Mercury station off by {diff_days:.1} days, got JD {}, expected ~JD {}",
        event.jd_tdb, expected_jd
    );
    assert_eq!(event.station_type, StationType::StationRetrograde);
    assert_eq!(event.body, Body::Mercury);
    assert!(event.longitude_deg >= 0.0 && event.longitude_deg < 360.0);
}

/// Mercury station direct ~2024-Apr-25.
/// Mercury goes direct around April 25, 2024.
#[test]
fn mercury_station_direct_apr_2024() {
    let Some(engine) = load_engine() else { return };
    // Start after the retrograde station
    let jd_start = jd_from_date(2024, 4, 5.0);
    let config = StationaryConfig::inner_planet();
    let result = next_stationary(&engine, Body::Mercury, jd_start, &config)
        .expect("search should succeed");
    let event = result.expect("should find a Mercury direct station");

    let expected_jd = jd_from_date(2024, 4, 25.0);
    let diff_days = (event.jd_tdb - expected_jd).abs();
    assert!(
        diff_days < 3.0,
        "Mercury direct station off by {diff_days:.1} days, got JD {}",
        event.jd_tdb
    );
    assert_eq!(event.station_type, StationType::StationDirect);
}

/// Mercury station retrograde ~2024-Aug-05.
#[test]
fn mercury_station_retrograde_aug_2024() {
    let Some(engine) = load_engine() else { return };
    let jd_start = jd_from_date(2024, 7, 1.0);
    let config = StationaryConfig::inner_planet();
    let result = next_stationary(&engine, Body::Mercury, jd_start, &config)
        .expect("search should succeed");
    let event = result.expect("should find a Mercury station");

    let expected_jd = jd_from_date(2024, 8, 5.0);
    let diff_days = (event.jd_tdb - expected_jd).abs();
    assert!(
        diff_days < 3.0,
        "Mercury station off by {diff_days:.1} days, got JD {}",
        event.jd_tdb
    );
    assert_eq!(event.station_type, StationType::StationRetrograde);
}

/// Mars retrograde 2024-2025: search range finds both station R and station D.
/// Mars station retrograde ~2024-Dec-06, station direct ~2025-Feb-24.
#[test]
fn mars_retrograde_2024_2025() {
    let Some(engine) = load_engine() else { return };
    let jd_start = jd_from_date(2024, 10, 1.0);
    let jd_end = jd_from_date(2025, 4, 1.0);
    let config = StationaryConfig::inner_planet();
    let events = search_stationary(&engine, Body::Mars, jd_start, jd_end, &config)
        .expect("search should succeed");

    assert!(
        events.len() >= 2,
        "expected at least 2 stations for Mars, got {}",
        events.len()
    );

    // First event should be station retrograde
    let station_r = &events[0];
    assert_eq!(station_r.station_type, StationType::StationRetrograde);
    let expected_r = jd_from_date(2024, 12, 6.0);
    let diff_r = (station_r.jd_tdb - expected_r).abs();
    assert!(
        diff_r < 3.0,
        "Mars station R off by {diff_r:.1} days, got JD {}",
        station_r.jd_tdb
    );

    // Second event should be station direct
    let station_d = &events[1];
    assert_eq!(station_d.station_type, StationType::StationDirect);
    let expected_d = jd_from_date(2025, 2, 24.0);
    let diff_d = (station_d.jd_tdb - expected_d).abs();
    assert!(
        diff_d < 3.0,
        "Mars station D off by {diff_d:.1} days, got JD {}",
        station_d.jd_tdb
    );
}

/// prev_stationary finds an earlier event.
#[test]
fn prev_stationary_mercury() {
    let Some(engine) = load_engine() else { return };
    let jd_start = jd_from_date(2024, 5, 1.0);
    let config = StationaryConfig::inner_planet();
    let result = prev_stationary(&engine, Body::Mercury, jd_start, &config)
        .expect("search should succeed");
    let event = result.expect("should find a previous Mercury station");

    // Should find a station before May 2024 — the direct station ~Apr 25 or earlier
    assert!(
        event.jd_tdb < jd_start,
        "prev_stationary should find event before start"
    );
}

/// Sun correctly rejected for stationary search.
#[test]
fn sun_rejected_for_stationary() {
    let Some(engine) = load_engine() else { return };
    let config = StationaryConfig::inner_planet();
    let result = next_stationary(&engine, Body::Sun, jd_from_date(2024, 1, 1.0), &config);
    assert!(matches!(result, Err(SearchError::InvalidConfig(_))));
}

/// Moon correctly rejected for stationary search.
#[test]
fn moon_rejected_for_stationary() {
    let Some(engine) = load_engine() else { return };
    let config = StationaryConfig::inner_planet();
    let result = next_stationary(&engine, Body::Moon, jd_from_date(2024, 1, 1.0), &config);
    assert!(matches!(result, Err(SearchError::InvalidConfig(_))));
}

/// Earth correctly rejected for stationary search.
#[test]
fn earth_rejected_for_stationary() {
    let Some(engine) = load_engine() else { return };
    let config = StationaryConfig::inner_planet();
    let result = next_stationary(&engine, Body::Earth, jd_from_date(2024, 1, 1.0), &config);
    assert!(matches!(result, Err(SearchError::InvalidConfig(_))));
}

/// Moon is allowed for max_speed search.
#[test]
fn moon_max_speed_allowed() {
    let Some(engine) = load_engine() else { return };
    let config = StationaryConfig::inner_planet();
    let result = next_max_speed(&engine, Body::Moon, jd_from_date(2024, 1, 1.0), &config);
    assert!(result.is_ok(), "Moon should be allowed for max_speed");
    let event = result.unwrap();
    assert!(event.is_some(), "should find a Moon max-speed event");
}

/// Mercury max_speed returns a significant speed value.
#[test]
fn mercury_max_speed_significant() {
    let Some(engine) = load_engine() else { return };
    let config = StationaryConfig::inner_planet();
    let result = next_max_speed(&engine, Body::Mercury, jd_from_date(2024, 1, 1.0), &config)
        .expect("search should succeed");
    let event = result.expect("should find a Mercury max-speed event");

    // Mercury's direct speed peak is typically > 1 deg/day
    assert!(
        event.speed_deg_per_day.abs() > 0.5,
        "Mercury max speed = {} deg/day, expected > 0.5",
        event.speed_deg_per_day
    );
    assert!(event.longitude_deg >= 0.0 && event.longitude_deg < 360.0);
}

/// Saturn station retrograde 2024 — outer planet test.
/// Saturn station retrograde ~2024-Jun-29.
#[test]
fn saturn_station_retrograde_2024() {
    let Some(engine) = load_engine() else { return };
    let jd_start = jd_from_date(2024, 5, 1.0);
    let config = StationaryConfig::outer_planet();
    let result = next_stationary(&engine, Body::Saturn, jd_start, &config)
        .expect("search should succeed");
    let event = result.expect("should find a Saturn station");

    let expected_jd = jd_from_date(2024, 6, 29.0);
    let diff_days = (event.jd_tdb - expected_jd).abs();
    assert!(
        diff_days < 3.0,
        "Saturn station off by {diff_days:.1} days, got JD {}",
        event.jd_tdb
    );
    assert_eq!(event.station_type, StationType::StationRetrograde);
}

/// Max speed classification: Mercury direct vs retrograde.
#[test]
fn max_speed_classifies_direct_and_retrograde() {
    let Some(engine) = load_engine() else { return };
    // Search a full year to find both direct and retrograde max-speed events
    let jd_start = jd_from_date(2024, 1, 1.0);
    let jd_end = jd_from_date(2024, 12, 31.0);
    let config = StationaryConfig::inner_planet();
    let events = dhruv_search::search_max_speed(&engine, Body::Mercury, jd_start, jd_end, &config)
        .expect("search should succeed");

    let has_direct = events.iter().any(|e| e.speed_type == MaxSpeedType::MaxDirect);
    let has_retro = events.iter().any(|e| e.speed_type == MaxSpeedType::MaxRetrograde);
    assert!(has_direct, "should have at least one MaxDirect event");
    assert!(has_retro, "should have at least one MaxRetrograde event");
}

/// Earth rejected for max_speed.
#[test]
fn earth_rejected_for_max_speed() {
    let Some(engine) = load_engine() else { return };
    let config = StationaryConfig::inner_planet();
    let result = next_max_speed(&engine, Body::Earth, jd_from_date(2024, 1, 1.0), &config);
    assert!(matches!(result, Err(SearchError::InvalidConfig(_))));
}
