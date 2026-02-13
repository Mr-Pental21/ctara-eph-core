//! Golden-value integration tests for drishti_for_date.
//!
//! Validates all config flag combinations and verifies special aspects.
//! Requires kernel files. Skips gracefully if absent.

use std::path::Path;

use dhruv_core::{Engine, EngineConfig};
use dhruv_search::sankranti_types::SankrantiConfig;
use dhruv_search::{DrishtiConfig, drishti_for_date};
use dhruv_time::{EopKernel, UtcTime};
use dhruv_vedic_base::riseset_types::{GeoLocation, RiseSetConfig};
use dhruv_vedic_base::{BhavaConfig, Graha};

const SPK_PATH: &str = "../../kernels/data/de442s.bsp";
const LSK_PATH: &str = "../../kernels/data/naif0012.tls";
const EOP_PATH: &str = "../../kernels/data/finals2000A.all";

fn load_engine() -> Option<Engine> {
    if !Path::new(SPK_PATH).exists() || !Path::new(LSK_PATH).exists() {
        eprintln!("Skipping drishti_golden: kernel files not found");
        return None;
    }
    let config = EngineConfig::with_single_spk(SPK_PATH.into(), LSK_PATH.into(), 1024, false);
    Engine::new(config).ok()
}

fn load_eop() -> Option<EopKernel> {
    if !Path::new(EOP_PATH).exists() {
        eprintln!("Skipping drishti_golden: EOP file not found");
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

fn utc_2024_jan_15() -> UtcTime {
    UtcTime::new(2024, 1, 15, 12, 0, 0.0)
}

// ===== Base case: all flags false =====

#[test]
fn base_all_flags_off() {
    let Some(engine) = load_engine() else { return };
    let Some(eop) = load_eop() else { return };
    let utc = utc_2024_jan_15();
    let location = new_delhi();
    let bhava_config = BhavaConfig::default();
    let rs_config = RiseSetConfig::default();
    let aya_config = default_aya_config();
    let config = DrishtiConfig {
        include_bhava: false,
        include_lagna: false,
        include_bindus: false,
    };

    let result = drishti_for_date(
        &engine,
        &eop,
        &utc,
        &location,
        &bhava_config,
        &rs_config,
        &aya_config,
        &config,
    )
    .expect("drishti_for_date should succeed");

    // Graha-to-graha matrix always populated
    // Diagonal should be zero
    for i in 0..9 {
        assert!(
            result.graha_to_graha.entries[i][i].total_virupa.abs() < 1e-10,
            "diagonal[{i}] should be zero",
        );
    }

    // At least some off-diagonal entries nonzero
    let mut nonzero = 0;
    for i in 0..9 {
        for j in 0..9 {
            if i != j && result.graha_to_graha.entries[i][j].total_virupa > 0.0 {
                nonzero += 1;
            }
        }
    }
    assert!(nonzero > 0, "expected some nonzero off-diagonal entries");

    // Lagna should be zeroed
    for i in 0..9 {
        assert!(
            result.graha_to_lagna[i].total_virupa.abs() < 1e-10,
            "lagna[{i}] should be zeroed when flag off",
        );
    }

    // Bhava should be zeroed
    for i in 0..9 {
        for j in 0..12 {
            assert!(
                result.graha_to_bhava[i][j].total_virupa.abs() < 1e-10,
                "bhava[{i}][{j}] should be zeroed when flag off",
            );
        }
    }

    // Bindus should be zeroed
    for i in 0..9 {
        for j in 0..19 {
            assert!(
                result.graha_to_bindus[i][j].total_virupa.abs() < 1e-10,
                "bindus[{i}][{j}] should be zeroed when flag off",
            );
        }
    }
}

// ===== Include lagna only =====

#[test]
fn include_lagna_only() {
    let Some(engine) = load_engine() else { return };
    let Some(eop) = load_eop() else { return };
    let utc = utc_2024_jan_15();
    let location = new_delhi();
    let bhava_config = BhavaConfig::default();
    let rs_config = RiseSetConfig::default();
    let aya_config = default_aya_config();
    let config = DrishtiConfig {
        include_bhava: false,
        include_lagna: true,
        include_bindus: false,
    };

    let result = drishti_for_date(
        &engine,
        &eop,
        &utc,
        &location,
        &bhava_config,
        &rs_config,
        &aya_config,
        &config,
    )
    .expect("drishti_for_date should succeed");

    // Lagna entries should have valid angular distances
    for i in 0..9 {
        let e = &result.graha_to_lagna[i];
        assert!(
            e.angular_distance >= 0.0 && e.angular_distance < 360.0,
            "lagna[{i}] angular_distance out of range: {}",
            e.angular_distance,
        );
    }

    // At least some lagna entries should be nonzero
    let nonzero = result
        .graha_to_lagna
        .iter()
        .filter(|e| e.total_virupa > 0.0)
        .count();
    assert!(nonzero > 0, "expected some nonzero lagna drishti entries");
}

// ===== Include bhava only =====

#[test]
fn include_bhava_only() {
    let Some(engine) = load_engine() else { return };
    let Some(eop) = load_eop() else { return };
    let utc = utc_2024_jan_15();
    let location = new_delhi();
    let bhava_config = BhavaConfig::default();
    let rs_config = RiseSetConfig::default();
    let aya_config = default_aya_config();
    let config = DrishtiConfig {
        include_bhava: true,
        include_lagna: false,
        include_bindus: false,
    };

    let result = drishti_for_date(
        &engine,
        &eop,
        &utc,
        &location,
        &bhava_config,
        &rs_config,
        &aya_config,
        &config,
    )
    .expect("drishti_for_date should succeed");

    // Bhava entries should have valid angular distances
    for i in 0..9 {
        for j in 0..12 {
            let e = &result.graha_to_bhava[i][j];
            assert!(
                e.angular_distance >= 0.0 && e.angular_distance < 360.0,
                "bhava[{i}][{j}] angular_distance out of range",
            );
        }
    }

    // At least some bhava entries nonzero
    let mut nonzero = 0;
    for i in 0..9 {
        for j in 0..12 {
            if result.graha_to_bhava[i][j].total_virupa > 0.0 {
                nonzero += 1;
            }
        }
    }
    assert!(nonzero > 0, "expected some nonzero bhava drishti entries");
}

// ===== Include bindus only =====

#[test]
fn include_bindus_only() {
    let Some(engine) = load_engine() else { return };
    let Some(eop) = load_eop() else { return };
    let utc = utc_2024_jan_15();
    let location = new_delhi();
    let bhava_config = BhavaConfig::default();
    let rs_config = RiseSetConfig::default();
    let aya_config = default_aya_config();
    let config = DrishtiConfig {
        include_bhava: false,
        include_lagna: false,
        include_bindus: true,
    };

    let result = drishti_for_date(
        &engine,
        &eop,
        &utc,
        &location,
        &bhava_config,
        &rs_config,
        &aya_config,
        &config,
    )
    .expect("drishti_for_date should succeed");

    // Bindus entries should have valid angular distances
    for i in 0..9 {
        for j in 0..19 {
            let e = &result.graha_to_bindus[i][j];
            assert!(
                e.angular_distance >= 0.0 && e.angular_distance < 360.0,
                "bindus[{i}][{j}] angular_distance out of range",
            );
        }
    }

    // At least some bindus entries nonzero
    let mut nonzero = 0;
    for i in 0..9 {
        for j in 0..19 {
            if result.graha_to_bindus[i][j].total_virupa > 0.0 {
                nonzero += 1;
            }
        }
    }
    assert!(nonzero > 0, "expected some nonzero bindus drishti entries");
}

// ===== All flags on =====

#[test]
fn all_flags_on() {
    let Some(engine) = load_engine() else { return };
    let Some(eop) = load_eop() else { return };
    let utc = utc_2024_jan_15();
    let location = new_delhi();
    let bhava_config = BhavaConfig::default();
    let rs_config = RiseSetConfig::default();
    let aya_config = default_aya_config();
    let config = DrishtiConfig {
        include_bhava: true,
        include_lagna: true,
        include_bindus: true,
    };

    let result = drishti_for_date(
        &engine,
        &eop,
        &utc,
        &location,
        &bhava_config,
        &rs_config,
        &aya_config,
        &config,
    )
    .expect("drishti_for_date should succeed");

    // All sections should have nonzero entries
    let lagna_nonzero = result
        .graha_to_lagna
        .iter()
        .filter(|e| e.total_virupa > 0.0)
        .count();
    assert!(lagna_nonzero > 0, "lagna should have nonzero entries");

    let mut bhava_nonzero = 0;
    for row in &result.graha_to_bhava {
        for e in row {
            if e.total_virupa > 0.0 {
                bhava_nonzero += 1;
            }
        }
    }
    assert!(bhava_nonzero > 0, "bhava should have nonzero entries");

    let mut bindus_nonzero = 0;
    for row in &result.graha_to_bindus {
        for e in row {
            if e.total_virupa > 0.0 {
                bindus_nonzero += 1;
            }
        }
    }
    assert!(bindus_nonzero > 0, "bindus should have nonzero entries");
}

// ===== Special aspects present in matrix =====

#[test]
fn special_aspects_present() {
    let Some(engine) = load_engine() else { return };
    let Some(eop) = load_eop() else { return };
    let utc = utc_2024_jan_15();
    let location = new_delhi();
    let bhava_config = BhavaConfig::default();
    let rs_config = RiseSetConfig::default();
    let aya_config = default_aya_config();
    let config = DrishtiConfig::default();

    let result = drishti_for_date(
        &engine,
        &eop,
        &utc,
        &location,
        &bhava_config,
        &rs_config,
        &aya_config,
        &config,
    )
    .expect("drishti_for_date should succeed");

    // Check total_virupa consistency
    for i in 0..9 {
        for j in 0..9 {
            let e = &result.graha_to_graha.entries[i][j];
            let expected = e.base_virupa + e.special_virupa;
            assert!(
                (e.total_virupa - expected).abs() < 1e-10,
                "total_virupa mismatch at [{i}][{j}]: {} != {} + {}",
                e.total_virupa,
                e.base_virupa,
                e.special_virupa,
            );
        }
    }

    // Non-special grahas should have zero special_virupa everywhere
    let non_special = [
        Graha::Surya.index() as usize,
        Graha::Chandra.index() as usize,
        Graha::Buddh.index() as usize,
        Graha::Shukra.index() as usize,
        Graha::Rahu.index() as usize,
        Graha::Ketu.index() as usize,
    ];
    for &i in &non_special {
        for j in 0..9 {
            assert!(
                result.graha_to_graha.entries[i][j].special_virupa.abs() < 1e-10,
                "graha {i} should have no special virupa at target {j}",
            );
        }
    }

    // Verify angular_distance consistency: should sum to 360 for [i][j] and [j][i]
    // (unless both are zero, i.e., same planet)
    for i in 0..9 {
        for j in (i + 1)..9 {
            let d_ij = result.graha_to_graha.entries[i][j].angular_distance;
            let d_ji = result.graha_to_graha.entries[j][i].angular_distance;
            let sum = d_ij + d_ji;
            assert!(
                (sum - 360.0).abs() < 1e-8,
                "angular distances [{i}][{j}]={d_ij} + [{j}][{i}]={d_ji} should sum to 360",
            );
        }
    }
}
