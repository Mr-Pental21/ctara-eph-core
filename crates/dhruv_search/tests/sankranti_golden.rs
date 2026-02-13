//! Golden-value integration tests for Sankranti search.
//!
//! Validates Sun entering rashis against known almanac dates.
//! Requires kernel files. Skips gracefully if absent.

use std::path::Path;

use dhruv_core::{Engine, EngineConfig};
use dhruv_search::sankranti_types::SankrantiConfig;
use dhruv_search::{
    next_sankranti, next_specific_sankranti, prev_sankranti, prev_specific_sankranti,
    search_sankrantis,
};
use dhruv_time::UtcTime;
use dhruv_vedic_base::Rashi;

const SPK_PATH: &str = "../../kernels/data/de442s.bsp";
const LSK_PATH: &str = "../../kernels/data/naif0012.tls";

fn load_engine() -> Option<Engine> {
    if !Path::new(SPK_PATH).exists() || !Path::new(LSK_PATH).exists() {
        eprintln!("Skipping sankranti_golden: kernel files not found");
        return None;
    }
    let config = EngineConfig::with_single_spk(SPK_PATH.into(), LSK_PATH.into(), 1024, false);
    Engine::new(config).ok()
}

fn default_config() -> SankrantiConfig {
    SankrantiConfig::default_lahiri()
}

/// Makar Sankranti 2024: Sun enters Makara ~Jan 14-15
#[test]
fn makar_sankranti_2024() {
    let Some(engine) = load_engine() else { return };
    let utc = UtcTime::new(2024, 1, 1, 0, 0, 0.0);
    let config = default_config();
    let event = next_specific_sankranti(&engine, &utc, Rashi::Makara, &config)
        .unwrap()
        .expect("should find Makar Sankranti");
    assert_eq!(event.utc.year, 2024);
    assert_eq!(event.utc.month, 1);
    // Makar Sankranti typically falls on Jan 14 or 15
    assert!(
        event.utc.day == 14 || event.utc.day == 15,
        "expected day 14 or 15, got {}",
        event.utc.day
    );
    assert_eq!(event.rashi, Rashi::Makara);
    // Sidereal longitude should be very close to 270 deg
    assert!(
        (event.sun_sidereal_longitude_deg - 270.0).abs() < 0.1,
        "expected ~270 deg, got {:.4}",
        event.sun_sidereal_longitude_deg
    );
}

/// Mesha Sankranti 2024: Sun enters Mesha ~April 13-14
#[test]
fn mesha_sankranti_2024() {
    let Some(engine) = load_engine() else { return };
    let utc = UtcTime::new(2024, 3, 1, 0, 0, 0.0);
    let config = default_config();
    let event = next_specific_sankranti(&engine, &utc, Rashi::Mesha, &config)
        .unwrap()
        .expect("should find Mesha Sankranti");
    assert_eq!(event.utc.year, 2024);
    assert_eq!(event.utc.month, 4);
    // Mesha Sankranti is typically April 13-14
    assert!(
        event.utc.day >= 13 && event.utc.day <= 15,
        "expected day 13-15, got {}",
        event.utc.day
    );
    assert_eq!(event.rashi, Rashi::Mesha);
}

/// next_sankranti from Jan 1 should find the next rashi transition
#[test]
fn next_sankranti_from_jan_2024() {
    let Some(engine) = load_engine() else { return };
    let utc = UtcTime::new(2024, 1, 1, 0, 0, 0.0);
    let config = default_config();
    let event = next_sankranti(&engine, &utc, &config)
        .unwrap()
        .expect("should find a sankranti");
    assert_eq!(event.utc.year, 2024);
    // Sidereal longitude should be close to a multiple of 30
    let boundary = (event.rashi_index as f64) * 30.0;
    assert!(
        (event.sun_sidereal_longitude_deg - boundary).abs() < 0.1,
        "expected ~{boundary} deg, got {:.4}",
        event.sun_sidereal_longitude_deg
    );
}

/// prev_sankranti from Feb 2024 should find Makar Sankranti
#[test]
fn prev_sankranti_from_feb_2024() {
    let Some(engine) = load_engine() else { return };
    let utc = UtcTime::new(2024, 2, 1, 0, 0, 0.0);
    let config = default_config();
    let event = prev_sankranti(&engine, &utc, &config)
        .unwrap()
        .expect("should find prev sankranti");
    assert_eq!(event.utc.year, 2024);
    assert_eq!(event.utc.month, 1);
    assert_eq!(event.rashi, Rashi::Makara);
}

/// Search for all 12 sankrantis in 2024
#[test]
fn search_all_sankrantis_2024() {
    let Some(engine) = load_engine() else { return };
    let start = UtcTime::new(2024, 1, 1, 0, 0, 0.0);
    let end = UtcTime::new(2025, 1, 1, 0, 0, 0.0);
    let config = default_config();
    let events = search_sankrantis(&engine, &start, &end, &config).unwrap();
    // A year should have 12 sankrantis (one per rashi)
    assert_eq!(
        events.len(),
        12,
        "expected 12 sankrantis, got {}",
        events.len()
    );
    // Each rashi should appear exactly once
    let mut seen = [false; 12];
    for ev in &events {
        assert!(
            !seen[ev.rashi_index as usize],
            "duplicate rashi: {}",
            ev.rashi.name()
        );
        seen[ev.rashi_index as usize] = true;
    }
    // All rashis seen
    assert!(seen.iter().all(|&s| s), "not all rashis found");
}

/// prev_specific_sankranti for Karka (Cancer) from Aug 2024
#[test]
fn prev_karka_sankranti_from_aug_2024() {
    let Some(engine) = load_engine() else { return };
    let utc = UtcTime::new(2024, 8, 1, 0, 0, 0.0);
    let config = default_config();
    let event = prev_specific_sankranti(&engine, &utc, Rashi::Karka, &config)
        .unwrap()
        .expect("should find prev Karka Sankranti");
    assert_eq!(event.utc.year, 2024);
    // Karka Sankranti (Sun entering Cancer) is typically mid-July
    assert_eq!(event.utc.month, 7);
    assert_eq!(event.rashi, Rashi::Karka);
}
