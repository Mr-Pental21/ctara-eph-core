//! Integration tests for Shadbala & Vimsopaka Bala orchestration.
//!
//! Requires kernel files. Skips gracefully if absent.

use std::path::Path;

use dhruv_core::{Engine, EngineConfig};
use dhruv_search::sankranti_types::SankrantiConfig;
use dhruv_search::{
    FullKundaliConfig, GrahaPositionsConfig, shadbala_for_date, shadbala_for_graha,
    vimsopaka_for_date, vimsopaka_for_graha,
};
use dhruv_vedic_base::riseset_types::{GeoLocation, RiseSetConfig};
use dhruv_vedic_base::{BhavaConfig, Graha, NodeDignityPolicy};

use dhruv_time::{EopKernel, UtcTime};

const SPK_PATH: &str = "../../kernels/data/de442s.bsp";
const LSK_PATH: &str = "../../kernels/data/naif0012.tls";
const EOP_PATH: &str = "../../kernels/data/finals2000A.all";

fn load_engine() -> Option<Engine> {
    if !Path::new(SPK_PATH).exists() || !Path::new(LSK_PATH).exists() {
        eprintln!("Skipping shadbala_test: kernel files not found");
        return None;
    }
    let config = EngineConfig::with_single_spk(SPK_PATH.into(), LSK_PATH.into(), 1024, false);
    Engine::new(config).ok()
}

fn load_eop() -> Option<EopKernel> {
    if !Path::new(EOP_PATH).exists() {
        eprintln!("Skipping shadbala_test: EOP file not found");
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

#[test]
fn shadbala_all_seven_valid() {
    let Some(engine) = load_engine() else { return };
    let Some(eop) = load_eop() else { return };
    let utc = utc_2024_jan_15();
    let location = new_delhi();
    let bhava_config = BhavaConfig::default();
    let rs_config = RiseSetConfig::default();
    let aya_config = default_aya_config();

    let result = shadbala_for_date(
        &engine,
        &eop,
        &utc,
        &location,
        &bhava_config,
        &rs_config,
        &aya_config,
    )
    .expect("shadbala_for_date should succeed");

    for (i, entry) in result.entries.iter().enumerate() {
        assert_eq!(entry.graha.index() as usize, i, "graha ordering");
        assert!(entry.total_shashtiamsas > 0.0, "total > 0 for graha {i}");
        assert!(entry.total_rupas > 0.0, "rupas > 0 for graha {i}");
        assert!(entry.required_strength > 0.0, "required > 0 for graha {i}");
        assert!(entry.naisargika > 0.0, "naisargika > 0 for graha {i}");
        // Sub-bala ranges: sthana total should be positive
        assert!(entry.sthana.total >= 0.0, "sthana >= 0 for graha {i}");
        // Dig bala in [0, 60]
        assert!(
            entry.dig >= 0.0 && entry.dig <= 60.0,
            "dig in range for graha {i}"
        );
        // Kala bala: total can be wide but should be finite
        assert!(entry.kala.total.is_finite(), "kala finite for graha {i}");
        // Cheshta bala in [0, 60]
        assert!(
            entry.cheshta >= 0.0 && entry.cheshta <= 60.0,
            "cheshta in range for graha {i}"
        );
    }
}

#[test]
fn shadbala_naisargika_matches_constants() {
    let Some(engine) = load_engine() else { return };
    let Some(eop) = load_eop() else { return };
    let utc = utc_2024_jan_15();
    let location = new_delhi();
    let bhava_config = BhavaConfig::default();
    let rs_config = RiseSetConfig::default();
    let aya_config = default_aya_config();

    let result = shadbala_for_date(
        &engine,
        &eop,
        &utc,
        &location,
        &bhava_config,
        &rs_config,
        &aya_config,
    )
    .expect("shadbala_for_date should succeed");

    let expected = [60.0, 51.43, 17.14, 25.71, 34.29, 42.86, 8.57];
    for (i, &exp) in expected.iter().enumerate() {
        assert!(
            (result.entries[i].naisargika - exp).abs() < 0.01,
            "naisargika mismatch for graha {i}: got {}, expected {}",
            result.entries[i].naisargika,
            exp
        );
    }
}

#[test]
fn shadbala_single_graha_matches_all() {
    let Some(engine) = load_engine() else { return };
    let Some(eop) = load_eop() else { return };
    let utc = utc_2024_jan_15();
    let location = new_delhi();
    let bhava_config = BhavaConfig::default();
    let rs_config = RiseSetConfig::default();
    let aya_config = default_aya_config();

    let all = shadbala_for_date(
        &engine,
        &eop,
        &utc,
        &location,
        &bhava_config,
        &rs_config,
        &aya_config,
    )
    .expect("shadbala_for_date should succeed");

    for i in 0..7 {
        let graha = dhruv_vedic_base::SAPTA_GRAHAS[i];
        let single = shadbala_for_graha(
            &engine,
            &eop,
            &utc,
            &location,
            &bhava_config,
            &rs_config,
            &aya_config,
            graha,
        )
        .expect("shadbala_for_graha should succeed");
        assert!(
            (single.total_shashtiamsas - all.entries[i].total_shashtiamsas).abs() < 0.01,
            "single vs all mismatch for graha {i}"
        );
    }
}

#[test]
fn shadbala_rejects_rahu() {
    let Some(engine) = load_engine() else { return };
    let Some(eop) = load_eop() else { return };
    let utc = utc_2024_jan_15();
    let location = new_delhi();
    let bhava_config = BhavaConfig::default();
    let rs_config = RiseSetConfig::default();
    let aya_config = default_aya_config();

    let result = shadbala_for_graha(
        &engine,
        &eop,
        &utc,
        &location,
        &bhava_config,
        &rs_config,
        &aya_config,
        Graha::Rahu,
    );
    assert!(result.is_err(), "shadbala should reject Rahu");
}

#[test]
fn shadbala_rejects_ketu() {
    let Some(engine) = load_engine() else { return };
    let Some(eop) = load_eop() else { return };
    let utc = utc_2024_jan_15();
    let location = new_delhi();
    let bhava_config = BhavaConfig::default();
    let rs_config = RiseSetConfig::default();
    let aya_config = default_aya_config();

    let result = shadbala_for_graha(
        &engine,
        &eop,
        &utc,
        &location,
        &bhava_config,
        &rs_config,
        &aya_config,
        Graha::Ketu,
    );
    assert!(result.is_err(), "shadbala should reject Ketu");
}

#[test]
fn vimsopaka_all_nine_valid() {
    let Some(engine) = load_engine() else { return };
    let Some(eop) = load_eop() else { return };
    let utc = utc_2024_jan_15();
    let location = new_delhi();
    let aya_config = default_aya_config();
    let policy = NodeDignityPolicy::default();

    let result = vimsopaka_for_date(&engine, &eop, &utc, &location, &aya_config, policy)
        .expect("vimsopaka_for_date should succeed");

    for (i, entry) in result.entries.iter().enumerate() {
        assert_eq!(entry.graha.index() as usize, i, "graha ordering");
        // All scores in [0, 20]
        assert!(
            entry.shadvarga >= 0.0 && entry.shadvarga <= 20.0,
            "shadvarga in [0,20] for graha {i}"
        );
        assert!(
            entry.saptavarga >= 0.0 && entry.saptavarga <= 20.0,
            "saptavarga in [0,20] for graha {i}"
        );
        assert!(
            entry.dashavarga >= 0.0 && entry.dashavarga <= 20.0,
            "dashavarga in [0,20] for graha {i}"
        );
        assert!(
            entry.shodasavarga >= 0.0 && entry.shodasavarga <= 20.0,
            "shodasavarga in [0,20] for graha {i}"
        );
    }
}

#[test]
fn vimsopaka_rahu_valid() {
    let Some(engine) = load_engine() else { return };
    let Some(eop) = load_eop() else { return };
    let utc = utc_2024_jan_15();
    let location = new_delhi();
    let aya_config = default_aya_config();

    let result = vimsopaka_for_graha(
        &engine,
        &eop,
        &utc,
        &location,
        &aya_config,
        NodeDignityPolicy::SignLordBased,
        Graha::Rahu,
    )
    .expect("vimsopaka should accept Rahu");
    assert!(result.shadvarga >= 0.0 && result.shadvarga <= 20.0);
}

#[test]
fn vimsopaka_single_graha_matches_all() {
    let Some(engine) = load_engine() else { return };
    let Some(eop) = load_eop() else { return };
    let utc = utc_2024_jan_15();
    let location = new_delhi();
    let aya_config = default_aya_config();
    let policy = NodeDignityPolicy::default();

    let all = vimsopaka_for_date(&engine, &eop, &utc, &location, &aya_config, policy)
        .expect("vimsopaka_for_date should succeed");

    for i in 0..9 {
        let graha = dhruv_vedic_base::ALL_GRAHAS[i];
        let single =
            vimsopaka_for_graha(&engine, &eop, &utc, &location, &aya_config, policy, graha)
                .expect("vimsopaka_for_graha should succeed");
        assert!(
            (single.shodasavarga - all.entries[i].shodasavarga).abs() < 0.01,
            "single vs all mismatch for graha {i}"
        );
    }
}

#[test]
fn vimsopaka_always_sama_policy() {
    let Some(engine) = load_engine() else { return };
    let Some(eop) = load_eop() else { return };
    let utc = utc_2024_jan_15();
    let location = new_delhi();
    let aya_config = default_aya_config();

    let result = vimsopaka_for_date(
        &engine,
        &eop,
        &utc,
        &location,
        &aya_config,
        NodeDignityPolicy::AlwaysSama,
    )
    .expect("vimsopaka_for_date should succeed");

    // With AlwaysSama, Rahu/Ketu get Sama dignity (7 points) in every varga.
    // shadvarga: sum(weight * 7) / sum(weight) = 7.0
    let rahu = &result.entries[Graha::Rahu.index() as usize];
    assert!(
        (rahu.shadvarga - 7.0).abs() < 0.01,
        "Rahu shadvarga with AlwaysSama should be 7.0, got {}",
        rahu.shadvarga
    );
    let ketu = &result.entries[Graha::Ketu.index() as usize];
    assert!(
        (ketu.shadvarga - 7.0).abs() < 0.01,
        "Ketu shadvarga with AlwaysSama should be 7.0, got {}",
        ketu.shadvarga
    );
}

#[test]
fn full_kundali_with_shadbala_vimsopaka() {
    let Some(engine) = load_engine() else { return };
    let Some(eop) = load_eop() else { return };
    let utc = utc_2024_jan_15();
    let location = new_delhi();
    let bhava_config = BhavaConfig::default();
    let rs_config = RiseSetConfig::default();
    let aya_config = default_aya_config();

    let config = FullKundaliConfig {
        include_shadbala: true,
        include_vimsopaka: true,
        graha_positions_config: GrahaPositionsConfig {
            include_lagna: true,
            ..GrahaPositionsConfig::default()
        },
        ..FullKundaliConfig::default()
    };

    let result = dhruv_search::full_kundali_for_date(
        &engine,
        &eop,
        &utc,
        &location,
        &bhava_config,
        &rs_config,
        &aya_config,
        &config,
    )
    .expect("full_kundali_for_date should succeed");

    let shadbala = result.shadbala.expect("shadbala should be Some");
    assert_eq!(shadbala.entries.len(), 7);
    for entry in &shadbala.entries {
        assert!(entry.total_shashtiamsas > 0.0);
    }

    let vimsopaka = result.vimsopaka.expect("vimsopaka should be Some");
    assert_eq!(vimsopaka.entries.len(), 9);
    for entry in &vimsopaka.entries {
        assert!(entry.shodasavarga >= 0.0 && entry.shodasavarga <= 20.0);
    }
}
