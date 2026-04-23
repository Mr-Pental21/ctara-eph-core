//! Integration tests for Shadbala & Vimsopaka Bala orchestration.
//!
//! Requires kernel files. Skips gracefully if absent.

use std::path::Path;

use dhruv_core::{Engine, EngineConfig};
use dhruv_search::sankranti_types::SankrantiConfig;
use dhruv_search::{
    AmshaSelectionConfig, FullKundaliConfig, GrahaPositionsConfig, balas_for_date,
    bhavabala_for_bhava, bhavabala_for_date, shadbala_for_date, shadbala_for_graha,
    vimsopaka_for_date, vimsopaka_for_graha,
};
use dhruv_vedic_base::riseset_types::{GeoLocation, RiseSetConfig};
use dhruv_vedic_base::{BhavaConfig, ChandraBeneficRule, Graha, NodeDignityPolicy, cheshta_bala};

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

#[test]
fn shadbala_cheshta_uses_correction_model_motion() {
    let engine = match load_engine() {
        Some(e) => e,
        None => return,
    };
    let eop = match load_eop() {
        Some(e) => e,
        None => return,
    };
    let utc = UtcTime {
        year: 2026,
        month: 4,
        day: 17,
        hour: 13,
        minute: 25,
        second: 39.0,
    };
    let location = new_delhi();
    let bhava_config = BhavaConfig::default();
    let riseset_config = RiseSetConfig::default();
    let aya_config = default_aya_config();
    let shadbala = shadbala_for_date(
        &engine,
        &eop,
        &utc,
        &location,
        &bhava_config,
        &riseset_config,
        &aya_config,
        &AmshaSelectionConfig::default(),
    )
    .expect("shadbala should succeed");
    let mangal = shadbala
        .entries
        .iter()
        .find(|entry| entry.graha == Graha::Mangal)
        .expect("Mangal entry");
    let graha_lons = dhruv_search::graha_longitudes(
        &engine,
        utc.to_jd_tdb(engine.lsk()),
        &dhruv_search::GrahaLongitudesConfig::sidereal_with_model(
            aya_config.ayanamsha_system,
            aya_config.use_nutation,
            aya_config.precession_model,
            aya_config.reference_plane,
        ),
    )
    .expect("graha longitudes should succeed");
    let cheshta_config = dhruv_search::GrahaLongitudesConfig::sidereal_with_model(
        aya_config.ayanamsha_system,
        aya_config.use_nutation,
        aya_config.precession_model,
        aya_config.reference_plane,
    );
    let motions = dhruv_search::jyotish::cheshta_motion_entries(
        &engine,
        utc.to_jd_tdb(engine.lsk()),
        &cheshta_config,
        &graha_lons.longitudes,
    )
    .expect("cheshta motions should succeed");
    let motion = motions[Graha::Mangal.index() as usize].expect("Mangal motion");
    let expected = cheshta_bala(
        Graha::Mangal,
        motion.madhyama_longitude,
        graha_lons.longitude(Graha::Mangal),
        motion.chaloccha_longitude,
    );
    assert!((mangal.cheshta - expected).abs() < 1e-9);
}

#[test]
fn cheshta_motion_maps_interior_and_exterior_differently() {
    let engine = match load_engine() {
        Some(e) => e,
        None => return,
    };
    let utc = UtcTime {
        year: 2026,
        month: 4,
        day: 17,
        hour: 13,
        minute: 25,
        second: 39.0,
    };
    let aya_config = default_aya_config();
    let config = dhruv_search::GrahaLongitudesConfig::sidereal_with_model(
        aya_config.ayanamsha_system,
        aya_config.use_nutation,
        aya_config.precession_model,
        aya_config.reference_plane,
    );
    let jd_tdb = utc.to_jd_tdb(engine.lsk());
    let graha_lons = dhruv_search::graha_longitudes(&engine, jd_tdb, &config).expect("longitudes");
    let motions = dhruv_search::jyotish::cheshta_motion_entries(
        &engine,
        jd_tdb,
        &config,
        &graha_lons.longitudes,
    )
    .expect("cheshta motion");

    for graha in [Graha::Mangal, Graha::Guru, Graha::Shani] {
        let motion = motions[graha.index() as usize].expect("motion");
        assert!(
            (motion.madhyama_longitude - motion.graha_heliocentric_mean_longitude).abs() < 1e-12
        );
        assert!((motion.chaloccha_longitude - motion.mean_sun_longitude).abs() < 1e-12);
    }

    for graha in [Graha::Buddh, Graha::Shukra] {
        let motion = motions[graha.index() as usize].expect("motion");
        assert!((motion.madhyama_longitude - motion.mean_sun_longitude).abs() < 1e-12);
        assert!(
            (motion.chaloccha_longitude - motion.graha_heliocentric_mean_longitude).abs() < 1e-12
        );
        assert!(
            (motion.chaloccha_longitude - motion.mean_sun_longitude).abs() > 1.0,
            "interior chaloccha should not collapse to mean Sun for {graha:?}"
        );
    }
}

fn utc_2024_jan_15() -> UtcTime {
    UtcTime::new(2024, 1, 15, 12, 0, 0.0)
}

fn default_amsha_selection() -> AmshaSelectionConfig {
    AmshaSelectionConfig::default()
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
        &default_amsha_selection(),
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
fn shadbala_node_aspects_for_drik_bala_are_opt_in() {
    let Some(engine) = load_engine() else { return };
    let Some(eop) = load_eop() else { return };
    let utc = utc_2024_jan_15();
    let location = new_delhi();
    let default_bhava = BhavaConfig::default();
    let with_nodes_bhava = BhavaConfig {
        include_node_aspects_for_drik_bala: true,
        ..BhavaConfig::default()
    };
    let rs_config = RiseSetConfig::default();
    let aya_config = default_aya_config();

    let without_nodes = shadbala_for_date(
        &engine,
        &eop,
        &utc,
        &location,
        &default_bhava,
        &rs_config,
        &aya_config,
        &default_amsha_selection(),
    )
    .expect("default shadbala should succeed");
    let with_nodes = shadbala_for_date(
        &engine,
        &eop,
        &utc,
        &location,
        &with_nodes_bhava,
        &rs_config,
        &aya_config,
        &default_amsha_selection(),
    )
    .expect("node-aspect shadbala should succeed");

    let mut changed_drik = false;
    for (base, opted_in) in without_nodes.entries.iter().zip(with_nodes.entries.iter()) {
        assert!((base.sthana.total - opted_in.sthana.total).abs() < 1e-9);
        assert!((base.dig - opted_in.dig).abs() < 1e-9);
        assert!((base.kala.total - opted_in.kala.total).abs() < 1e-9);
        assert!((base.cheshta - opted_in.cheshta).abs() < 1e-9);
        assert!((base.naisargika - opted_in.naisargika).abs() < 1e-9);
        if (base.drik - opted_in.drik).abs() > 1e-9 {
            changed_drik = true;
        }
    }
    assert!(
        changed_drik,
        "expected Rahu/Ketu opt-in to change Drik Bala"
    );
}

#[test]
fn shadbala_guru_buddh_drik_divisor_is_opt_out() {
    let Some(engine) = load_engine() else { return };
    let Some(eop) = load_eop() else { return };
    let utc = utc_2024_jan_15();
    let location = new_delhi();
    let default_bhava = BhavaConfig::default();
    let full_guru_buddh_bhava = BhavaConfig {
        divide_guru_buddh_drishti_by_4_for_drik_bala: false,
        ..BhavaConfig::default()
    };
    let rs_config = RiseSetConfig::default();
    let aya_config = default_aya_config();

    let divided = shadbala_for_date(
        &engine,
        &eop,
        &utc,
        &location,
        &default_bhava,
        &rs_config,
        &aya_config,
        &default_amsha_selection(),
    )
    .expect("default shadbala should succeed");
    let full = shadbala_for_date(
        &engine,
        &eop,
        &utc,
        &location,
        &full_guru_buddh_bhava,
        &rs_config,
        &aya_config,
        &default_amsha_selection(),
    )
    .expect("full Guru/Buddh Drik Bala shadbala should succeed");

    let mut changed_drik = false;
    for (base, opted_out) in divided.entries.iter().zip(full.entries.iter()) {
        assert!((base.sthana.total - opted_out.sthana.total).abs() < 1e-9);
        assert!((base.dig - opted_out.dig).abs() < 1e-9);
        assert!((base.kala.total - opted_out.kala.total).abs() < 1e-9);
        assert!((base.cheshta - opted_out.cheshta).abs() < 1e-9);
        assert!((base.naisargika - opted_out.naisargika).abs() < 1e-9);
        if (base.drik - opted_out.drik).abs() > 1e-9 {
            changed_drik = true;
        }
    }
    assert!(
        changed_drik,
        "expected Guru/Buddh full-strength opt-out to change Drik Bala"
    );
}

#[test]
fn shadbala_chandra_benefic_rule_is_configurable() {
    let Some(engine) = load_engine() else { return };
    let Some(eop) = load_eop() else { return };
    let utc = UtcTime {
        year: 2026,
        month: 4,
        day: 17,
        hour: 13,
        minute: 25,
        second: 39.0,
    };
    let location = new_delhi();
    let brightness_bhava = BhavaConfig::default();
    let waxing_bhava = BhavaConfig {
        chandra_benefic_rule: ChandraBeneficRule::Waxing180,
        ..BhavaConfig::default()
    };
    let rs_config = RiseSetConfig::default();
    let aya_config = default_aya_config();

    let brightness = shadbala_for_date(
        &engine,
        &eop,
        &utc,
        &location,
        &brightness_bhava,
        &rs_config,
        &aya_config,
        &default_amsha_selection(),
    )
    .expect("default shadbala should succeed");
    let waxing = shadbala_for_date(
        &engine,
        &eop,
        &utc,
        &location,
        &waxing_bhava,
        &rs_config,
        &aya_config,
        &default_amsha_selection(),
    )
    .expect("waxing-rule shadbala should succeed");

    let mut changed_kala_or_drik = false;
    for (base, opted_in) in brightness.entries.iter().zip(waxing.entries.iter()) {
        assert!((base.sthana.total - opted_in.sthana.total).abs() < 1e-9);
        assert!((base.dig - opted_in.dig).abs() < 1e-9);
        assert!((base.cheshta - opted_in.cheshta).abs() < 1e-9);
        assert!((base.naisargika - opted_in.naisargika).abs() < 1e-9);
        if (base.kala.total - opted_in.kala.total).abs() > 1e-9
            || (base.drik - opted_in.drik).abs() > 1e-9
        {
            changed_kala_or_drik = true;
        }
    }
    assert!(
        changed_kala_or_drik,
        "expected Chandra rule to change nature-dependent bala"
    );
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
        &default_amsha_selection(),
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
        &default_amsha_selection(),
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
            &default_amsha_selection(),
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
        &default_amsha_selection(),
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
        &default_amsha_selection(),
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

    let result = vimsopaka_for_date(
        &engine,
        &eop,
        &utc,
        &location,
        &aya_config,
        policy,
        &default_amsha_selection(),
    )
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
        &default_amsha_selection(),
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

    let all = vimsopaka_for_date(
        &engine,
        &eop,
        &utc,
        &location,
        &aya_config,
        policy,
        &default_amsha_selection(),
    )
    .expect("vimsopaka_for_date should succeed");

    for i in 0..9 {
        let graha = dhruv_vedic_base::ALL_GRAHAS[i];
        let single = vimsopaka_for_graha(
            &engine,
            &eop,
            &utc,
            &location,
            &aya_config,
            policy,
            &default_amsha_selection(),
            graha,
        )
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
        &default_amsha_selection(),
    )
    .expect("vimsopaka_for_date should succeed");

    // With AlwaysSama, Rahu/Ketu get Sama dignity (10 points) in every varga.
    // shadvarga: sum(weight * 10) / sum(weight) = 10.0
    let rahu = &result.entries[Graha::Rahu.index() as usize];
    assert!(
        (rahu.shadvarga - 10.0).abs() < 0.01,
        "Rahu shadvarga with AlwaysSama should be 10.0, got {}",
        rahu.shadvarga
    );
    let ketu = &result.entries[Graha::Ketu.index() as usize];
    assert!(
        (ketu.shadvarga - 10.0).abs() < 0.01,
        "Ketu shadvarga with AlwaysSama should be 10.0, got {}",
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

#[test]
fn bhavabala_all_twelve_valid() {
    let Some(engine) = load_engine() else { return };
    let Some(eop) = load_eop() else { return };
    let utc = utc_2024_jan_15();
    let location = new_delhi();
    let bhava_config = BhavaConfig::default();
    let rs_config = RiseSetConfig::default();
    let aya_config = default_aya_config();

    let result = bhavabala_for_date(
        &engine,
        &eop,
        &utc,
        &location,
        &bhava_config,
        &rs_config,
        &aya_config,
    )
    .expect("bhavabala_for_date should succeed");

    assert_eq!(result.entries.len(), 12);
    for (index, entry) in result.entries.iter().enumerate() {
        assert_eq!(entry.bhava_number as usize, index + 1);
        assert!(entry.total_virupas.is_finite());
        assert!(entry.total_rupas.is_finite());
        assert!(entry.bhavadhipati > 0.0);
        assert!(entry.dig >= 0.0);
    }
}

#[test]
fn bhavabala_single_bhava_matches_all() {
    let Some(engine) = load_engine() else { return };
    let Some(eop) = load_eop() else { return };
    let utc = utc_2024_jan_15();
    let location = new_delhi();
    let bhava_config = BhavaConfig::default();
    let rs_config = RiseSetConfig::default();
    let aya_config = default_aya_config();

    let all = bhavabala_for_date(
        &engine,
        &eop,
        &utc,
        &location,
        &bhava_config,
        &rs_config,
        &aya_config,
    )
    .expect("bhavabala_for_date should succeed");
    let single = bhavabala_for_bhava(
        &engine,
        &eop,
        &utc,
        &location,
        &bhava_config,
        &rs_config,
        &aya_config,
        1,
    )
    .expect("bhavabala_for_bhava should succeed");

    assert!((single.total_virupas - all.entries[0].total_virupas).abs() < 0.01);
}

#[test]
fn bala_bundle_includes_all_requested_surfaces() {
    let Some(engine) = load_engine() else { return };
    let Some(eop) = load_eop() else { return };
    let utc = utc_2024_jan_15();
    let location = new_delhi();
    let bhava_config = BhavaConfig::default();
    let rs_config = RiseSetConfig::default();
    let aya_config = default_aya_config();

    let result = balas_for_date(
        &engine,
        &eop,
        &utc,
        &location,
        &bhava_config,
        &rs_config,
        &aya_config,
        NodeDignityPolicy::default(),
        &default_amsha_selection(),
    )
    .expect("balas_for_date should succeed");

    assert_eq!(result.shadbala.entries.len(), 7);
    assert_eq!(result.vimsopaka.entries.len(), 9);
    assert_eq!(result.bhavabala.entries.len(), 12);
    assert!(result.ashtakavarga.sav.total_points.iter().any(|&v| v > 0));
}

#[test]
fn standalone_bala_surfaces_honor_d2_variation_selection() {
    let Some(engine) = load_engine() else { return };
    let Some(eop) = load_eop() else { return };
    let utc = utc_2024_jan_15();
    let location = new_delhi();
    let bhava_config = BhavaConfig::default();
    let rs_config = RiseSetConfig::default();
    let aya_config = default_aya_config();
    let default_selection = default_amsha_selection();
    let mut varied_selection = AmshaSelectionConfig {
        count: 1,
        ..AmshaSelectionConfig::default()
    };
    varied_selection.codes[0] = 2;
    varied_selection.variations[0] = 1;

    let shadbala_default = shadbala_for_date(
        &engine,
        &eop,
        &utc,
        &location,
        &bhava_config,
        &rs_config,
        &aya_config,
        &default_selection,
    )
    .expect("default shadbala");
    let shadbala_varied = shadbala_for_date(
        &engine,
        &eop,
        &utc,
        &location,
        &bhava_config,
        &rs_config,
        &aya_config,
        &varied_selection,
    )
    .expect("varied shadbala");
    assert!(
        shadbala_default
            .entries
            .iter()
            .zip(shadbala_varied.entries.iter())
            .any(|(lhs, rhs)| (lhs.total_shashtiamsas - rhs.total_shashtiamsas).abs() > 0.0001),
        "D2 variation should affect shadbala totals",
    );

    let vimsopaka_default = vimsopaka_for_date(
        &engine,
        &eop,
        &utc,
        &location,
        &aya_config,
        NodeDignityPolicy::default(),
        &default_selection,
    )
    .expect("default vimsopaka");
    let vimsopaka_varied = vimsopaka_for_date(
        &engine,
        &eop,
        &utc,
        &location,
        &aya_config,
        NodeDignityPolicy::default(),
        &varied_selection,
    )
    .expect("varied vimsopaka");
    assert!(
        vimsopaka_default
            .entries
            .iter()
            .zip(vimsopaka_varied.entries.iter())
            .any(|(lhs, rhs)| (lhs.shadvarga - rhs.shadvarga).abs() > 0.0001),
        "D2 variation should affect vimsopaka scores",
    );

    let bundle_default = balas_for_date(
        &engine,
        &eop,
        &utc,
        &location,
        &bhava_config,
        &rs_config,
        &aya_config,
        NodeDignityPolicy::default(),
        &default_selection,
    )
    .expect("default bala bundle");
    let bundle_varied = balas_for_date(
        &engine,
        &eop,
        &utc,
        &location,
        &bhava_config,
        &rs_config,
        &aya_config,
        NodeDignityPolicy::default(),
        &varied_selection,
    )
    .expect("varied bala bundle");
    assert!(
        bundle_default
            .shadbala
            .entries
            .iter()
            .zip(bundle_varied.shadbala.entries.iter())
            .any(|(lhs, rhs)| (lhs.total_shashtiamsas - rhs.total_shashtiamsas).abs() > 0.0001),
        "D2 variation should flow through bundled bala output",
    );
}

#[test]
fn full_kundali_with_bhavabala() {
    let Some(engine) = load_engine() else { return };
    let Some(eop) = load_eop() else { return };
    let utc = utc_2024_jan_15();
    let location = new_delhi();
    let bhava_config = BhavaConfig::default();
    let rs_config = RiseSetConfig::default();
    let aya_config = default_aya_config();

    let config = FullKundaliConfig {
        include_shadbala: true,
        include_bhavabala: true,
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

    assert_eq!(
        result
            .bhavabala
            .expect("bhavabala should be Some")
            .entries
            .len(),
        12
    );
}
