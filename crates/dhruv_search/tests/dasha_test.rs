//! Integration tests for Dasha orchestration.
//!
//! Requires kernel files. Skips gracefully if absent.

use std::path::Path;

use dhruv_core::{Engine, EngineConfig};
use dhruv_search::sankranti_types::SankrantiConfig;
use dhruv_search::{dasha_hierarchy_for_birth, dasha_snapshot_at};
use dhruv_time::{EopKernel, UtcTime};
use dhruv_vedic_base::dasha::{DashaLevel, DashaSystem, DashaVariationConfig};
use dhruv_vedic_base::riseset_types::{GeoLocation, RiseSetConfig};
use dhruv_vedic_base::BhavaConfig;

const SPK_PATH: &str = "../../kernels/data/de442s.bsp";
const LSK_PATH: &str = "../../kernels/data/naif0012.tls";
const EOP_PATH: &str = "../../kernels/data/finals2000A.all";

fn load_engine() -> Option<Engine> {
    if !Path::new(SPK_PATH).exists() || !Path::new(LSK_PATH).exists() {
        eprintln!("Skipping dasha_test: kernel files not found");
        return None;
    }
    let config = EngineConfig::with_single_spk(SPK_PATH.into(), LSK_PATH.into(), 1024, false);
    Engine::new(config).ok()
}

fn load_eop() -> Option<EopKernel> {
    if !Path::new(EOP_PATH).exists() {
        eprintln!("Skipping dasha_test: EOP file not found");
        return None;
    }
    EopKernel::load(Path::new(EOP_PATH)).ok()
}

fn default_aya_config() -> SankrantiConfig {
    SankrantiConfig::default_lahiri()
}

fn new_delhi() -> GeoLocation {
    GeoLocation::new(28.6139, 77.2090, 0.0)
}

/// Birth: 1990-01-15 06:30 UTC (New Delhi)
fn birth_utc() -> UtcTime {
    UtcTime::new(1990, 1, 15, 6, 30, 0.0)
}

/// Query: 2024-06-01 12:00 UTC
fn query_utc() -> UtcTime {
    UtcTime::new(2024, 6, 1, 12, 0, 0.0)
}

#[test]
fn vimshottari_hierarchy_valid() {
    let Some(engine) = load_engine() else { return };
    let Some(eop) = load_eop() else { return };
    let utc = birth_utc();
    let location = new_delhi();
    let bhava_config = BhavaConfig::default();
    let rs_config = RiseSetConfig::default();
    let aya_config = default_aya_config();
    let variation = DashaVariationConfig::default();

    let result = dasha_hierarchy_for_birth(
        &engine,
        &eop,
        &utc,
        &location,
        DashaSystem::Vimshottari,
        2,
        &bhava_config,
        &rs_config,
        &aya_config,
        &variation,
    );
    assert!(result.is_ok(), "hierarchy should succeed: {:?}", result.err());

    let hierarchy = result.unwrap();
    assert_eq!(hierarchy.system, DashaSystem::Vimshottari);
    assert_eq!(hierarchy.levels.len(), 3); // levels 0, 1, 2

    // Level 0: should have 9 mahadasha periods (Vimshottari has 9 grahas)
    assert_eq!(hierarchy.levels[0].len(), 9);

    // All level 0 periods should be Mahadasha
    for period in &hierarchy.levels[0] {
        assert_eq!(period.level, DashaLevel::Mahadasha);
    }

    // Adjacent periods should be contiguous
    for i in 1..hierarchy.levels[0].len() {
        let prev_end = hierarchy.levels[0][i - 1].end_jd;
        let curr_start = hierarchy.levels[0][i].start_jd;
        assert!(
            (prev_end - curr_start).abs() < 1e-10,
            "Mahadasha periods should be contiguous: prev_end={prev_end}, curr_start={curr_start}"
        );
    }

    // Level 1: should have 9*9 = 81 antardasha periods
    assert_eq!(hierarchy.levels[1].len(), 81);

    // Level 2: should have 81*9 = 729 pratyantardasha periods
    assert_eq!(hierarchy.levels[2].len(), 729);

    // Total time span should be <= 120 years (first period is partial due to balance)
    let total_days = hierarchy.levels[0].last().unwrap().end_jd
        - hierarchy.levels[0].first().unwrap().start_jd;
    let max_span = 120.0 * 365.25;
    assert!(
        total_days > 0.0 && total_days <= max_span + 1.0,
        "Total span should be positive and <= 120 years, got {total_days} days"
    );
}

#[test]
fn vimshottari_snapshot_valid() {
    let Some(engine) = load_engine() else { return };
    let Some(eop) = load_eop() else { return };
    let birth = birth_utc();
    let query = query_utc();
    let location = new_delhi();
    let bhava_config = BhavaConfig::default();
    let rs_config = RiseSetConfig::default();
    let aya_config = default_aya_config();
    let variation = DashaVariationConfig::default();

    let result = dasha_snapshot_at(
        &engine,
        &eop,
        &birth,
        &query,
        &location,
        DashaSystem::Vimshottari,
        2,
        &bhava_config,
        &rs_config,
        &aya_config,
        &variation,
    );
    assert!(result.is_ok(), "snapshot should succeed: {:?}", result.err());

    let snapshot = result.unwrap();
    assert_eq!(snapshot.system, DashaSystem::Vimshottari);
    assert_eq!(snapshot.periods.len(), 3); // levels 0, 1, 2

    // Each level's period should contain the query JD
    for (i, period) in snapshot.periods.iter().enumerate() {
        assert!(
            period.start_jd <= snapshot.query_jd && snapshot.query_jd < period.end_jd,
            "Level {i} period should contain query_jd: start={}, end={}, query={}",
            period.start_jd,
            period.end_jd,
            snapshot.query_jd
        );
    }

    // Level 0 should be Mahadasha
    assert_eq!(snapshot.periods[0].level, DashaLevel::Mahadasha);
    // Level 1 should be Antardasha
    assert_eq!(snapshot.periods[1].level, DashaLevel::Antardasha);
    // Level 2 should be Pratyantardasha
    assert_eq!(snapshot.periods[2].level, DashaLevel::Pratyantardasha);
}

/// All 10 nakshatra-based systems should produce valid hierarchies.
#[test]
fn all_nakshatra_systems_hierarchy_valid() {
    let Some(engine) = load_engine() else { return };
    let Some(eop) = load_eop() else { return };
    let utc = birth_utc();
    let location = new_delhi();
    let bhava_config = BhavaConfig::default();
    let rs_config = RiseSetConfig::default();
    let aya_config = default_aya_config();
    let variation = DashaVariationConfig::default();

    let systems = [
        DashaSystem::Vimshottari,
        DashaSystem::Ashtottari,
        DashaSystem::Shodsottari,
        DashaSystem::Dwadashottari,
        DashaSystem::Panchottari,
        DashaSystem::Shatabdika,
        DashaSystem::Chaturashiti,
        DashaSystem::DwisaptatiSama,
        DashaSystem::Shashtihayani,
        DashaSystem::ShatTrimshaSama,
    ];

    for system in systems {
        let result = dasha_hierarchy_for_birth(
            &engine,
            &eop,
            &utc,
            &location,
            system,
            2,
            &bhava_config,
            &rs_config,
            &aya_config,
            &variation,
        );
        assert!(
            result.is_ok(),
            "{:?} hierarchy should succeed: {:?}",
            system,
            result.err()
        );
        let h = result.unwrap();
        assert_eq!(h.system, system);
        assert_eq!(h.levels.len(), 3);
        assert!(!h.levels[0].is_empty());
    }
}

/// Yogini dasha should produce valid hierarchy and snapshot.
#[test]
fn yogini_hierarchy_and_snapshot_valid() {
    let Some(engine) = load_engine() else { return };
    let Some(eop) = load_eop() else { return };
    let birth = birth_utc();
    // Yogini cycle is only 36y; use query 10y after birth to stay well within range
    let query = UtcTime::new(2000, 6, 1, 12, 0, 0.0);
    let location = new_delhi();
    let bhava_config = BhavaConfig::default();
    let rs_config = RiseSetConfig::default();
    let aya_config = default_aya_config();
    let variation = DashaVariationConfig::default();

    let h_result = dasha_hierarchy_for_birth(
        &engine,
        &eop,
        &birth,
        &location,
        DashaSystem::Yogini,
        2,
        &bhava_config,
        &rs_config,
        &aya_config,
        &variation,
    );
    assert!(h_result.is_ok(), "Yogini hierarchy: {:?}", h_result.err());
    let h = h_result.unwrap();
    assert_eq!(h.system, DashaSystem::Yogini);
    assert_eq!(h.levels.len(), 3);
    assert_eq!(h.levels[0].len(), 8);

    let s_result = dasha_snapshot_at(
        &engine,
        &eop,
        &birth,
        &query,
        &location,
        DashaSystem::Yogini,
        2,
        &bhava_config,
        &rs_config,
        &aya_config,
        &variation,
    );
    assert!(s_result.is_ok(), "Yogini snapshot: {:?}", s_result.err());
    let snap = s_result.unwrap();
    assert_eq!(snap.system, DashaSystem::Yogini);
    assert_eq!(snap.periods.len(), 3);
}

/// Special systems not yet implemented should return error.
#[test]
fn unimplemented_system_returns_error() {
    let Some(engine) = load_engine() else { return };
    let Some(eop) = load_eop() else { return };
    let utc = birth_utc();
    let location = new_delhi();
    let bhava_config = BhavaConfig::default();
    let rs_config = RiseSetConfig::default();
    let aya_config = default_aya_config();
    let variation = DashaVariationConfig::default();

    let result = dasha_hierarchy_for_birth(
        &engine,
        &eop,
        &utc,
        &location,
        DashaSystem::Kala, // Kala is not yet implemented
        2,
        &bhava_config,
        &rs_config,
        &aya_config,
        &variation,
    );
    assert!(result.is_err(), "unimplemented system should return error");
}

/// All 10 rashi-based systems should produce valid hierarchies.
#[test]
fn all_rashi_systems_hierarchy_valid() {
    let Some(engine) = load_engine() else { return };
    let Some(eop) = load_eop() else { return };
    let utc = birth_utc();
    let location = new_delhi();
    let bhava_config = BhavaConfig::default();
    let rs_config = RiseSetConfig::default();
    let aya_config = default_aya_config();
    let variation = DashaVariationConfig::default();

    let systems = [
        DashaSystem::Chara,
        DashaSystem::Sthira,
        DashaSystem::Yogardha,
        DashaSystem::Driga,
        DashaSystem::Shoola,
        DashaSystem::Mandooka,
        DashaSystem::Chakra,
        DashaSystem::Kendradi,
        DashaSystem::KarakaKendradi,
        DashaSystem::KarakaKendradiGraha,
    ];

    for system in systems {
        let result = dasha_hierarchy_for_birth(
            &engine,
            &eop,
            &utc,
            &location,
            system,
            1,
            &bhava_config,
            &rs_config,
            &aya_config,
            &variation,
        );
        assert!(
            result.is_ok(),
            "{:?} hierarchy should succeed: {:?}",
            system,
            result.err()
        );
        let h = result.unwrap();
        assert_eq!(h.system, system);
        assert_eq!(h.levels.len(), 2); // level 0 + level 1
        assert_eq!(h.levels[0].len(), 12, "{:?} should have 12 mahadasha periods", system);

        // Adjacent periods contiguous
        for i in 1..h.levels[0].len() {
            let prev_end = h.levels[0][i - 1].end_jd;
            let curr_start = h.levels[0][i].start_jd;
            assert!(
                (prev_end - curr_start).abs() < 1e-10,
                "{:?}: gap between periods {} and {}",
                system,
                i - 1,
                i
            );
        }
    }
}

/// Rashi-based snapshot should find active periods.
#[test]
fn chara_snapshot_valid() {
    let Some(engine) = load_engine() else { return };
    let Some(eop) = load_eop() else { return };
    let birth = birth_utc();
    let query = UtcTime::new(2000, 6, 1, 12, 0, 0.0);
    let location = new_delhi();
    let bhava_config = BhavaConfig::default();
    let rs_config = RiseSetConfig::default();
    let aya_config = default_aya_config();
    let variation = DashaVariationConfig::default();

    let result = dasha_snapshot_at(
        &engine,
        &eop,
        &birth,
        &query,
        &location,
        DashaSystem::Chara,
        2,
        &bhava_config,
        &rs_config,
        &aya_config,
        &variation,
    );
    assert!(result.is_ok(), "Chara snapshot: {:?}", result.err());
    let snap = result.unwrap();
    assert_eq!(snap.system, DashaSystem::Chara);
    assert_eq!(snap.periods.len(), 3);

    // Each level should contain the query JD
    for (i, period) in snap.periods.iter().enumerate() {
        assert!(
            period.start_jd <= snap.query_jd && snap.query_jd < period.end_jd,
            "Level {i}: period does not contain query_jd"
        );
    }
}
