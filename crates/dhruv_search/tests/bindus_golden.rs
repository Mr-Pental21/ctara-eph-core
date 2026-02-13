//! Golden-value integration tests for core_bindus.
//!
//! Validates all config flag combinations and cross-checks against
//! standalone functions (arudha_padas_for_date, special_lagnas_for_date,
//! all_upagrahas_for_date).
//! Requires kernel files. Skips gracefully if absent.

use std::path::Path;

use dhruv_core::{Engine, EngineConfig};
use dhruv_search::sankranti_types::SankrantiConfig;
use dhruv_search::{BindusConfig, core_bindus};
use dhruv_time::{EopKernel, UtcTime};
use dhruv_vedic_base::BhavaConfig;
use dhruv_vedic_base::riseset_types::{GeoLocation, RiseSetConfig};

const SPK_PATH: &str = "../../kernels/data/de442s.bsp";
const LSK_PATH: &str = "../../kernels/data/naif0012.tls";
const EOP_PATH: &str = "../../kernels/data/finals2000A.all";

fn load_engine() -> Option<Engine> {
    if !Path::new(SPK_PATH).exists() || !Path::new(LSK_PATH).exists() {
        eprintln!("Skipping bindus_golden: kernel files not found");
        return None;
    }
    let config = EngineConfig::with_single_spk(SPK_PATH.into(), LSK_PATH.into(), 1024, false);
    Engine::new(config).ok()
}

fn load_eop() -> Option<EopKernel> {
    if !Path::new(EOP_PATH).exists() {
        eprintln!("Skipping bindus_golden: EOP file not found");
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
    let config = BindusConfig {
        include_nakshatra: false,
        include_bhava: false,
    };

    let result = core_bindus(
        &engine,
        &eop,
        &utc,
        &location,
        &bhava_config,
        &rs_config,
        &aya_config,
        &config,
    )
    .expect("core_bindus should succeed");

    // All 12 arudha padas should have valid sidereal longitudes [0, 360)
    for (i, entry) in result.arudha_padas.iter().enumerate() {
        assert!(
            entry.sidereal_longitude >= 0.0 && entry.sidereal_longitude < 360.0,
            "arudha_pada[{}] longitude out of range: {}",
            i,
            entry.sidereal_longitude,
        );
        // Sentinel values for disabled features
        assert_eq!(
            entry.nakshatra_index, 255,
            "arudha_pada[{}] nakshatra should be sentinel",
            i
        );
        assert_eq!(entry.pada, 0, "arudha_pada[{}] pada should be sentinel", i);
        assert_eq!(
            entry.bhava_number, 0,
            "arudha_pada[{}] bhava should be sentinel",
            i
        );
    }

    // 7 sensitive points: valid longitudes, sentinel nak/bhava
    let points = [
        ("bhrigu_bindu", result.bhrigu_bindu),
        ("pranapada_lagna", result.pranapada_lagna),
        ("gulika", result.gulika),
        ("maandi", result.maandi),
        ("hora_lagna", result.hora_lagna),
        ("ghati_lagna", result.ghati_lagna),
        ("sree_lagna", result.sree_lagna),
    ];
    for (name, entry) in &points {
        assert!(
            entry.sidereal_longitude >= 0.0 && entry.sidereal_longitude < 360.0,
            "{} longitude out of range: {}",
            name,
            entry.sidereal_longitude,
        );
        assert_eq!(
            entry.nakshatra_index, 255,
            "{} nakshatra should be sentinel",
            name
        );
        assert_eq!(entry.pada, 0, "{} pada should be sentinel", name);
        assert_eq!(entry.bhava_number, 0, "{} bhava should be sentinel", name);
    }
}

// ===== Nakshatra flag =====

#[test]
fn include_nakshatra() {
    let Some(engine) = load_engine() else { return };
    let Some(eop) = load_eop() else { return };
    let utc = utc_2024_jan_15();
    let location = new_delhi();
    let bhava_config = BhavaConfig::default();
    let rs_config = RiseSetConfig::default();
    let aya_config = default_aya_config();
    let config = BindusConfig {
        include_nakshatra: true,
        include_bhava: false,
    };

    let result = core_bindus(
        &engine,
        &eop,
        &utc,
        &location,
        &bhava_config,
        &rs_config,
        &aya_config,
        &config,
    )
    .expect("core_bindus should succeed");

    // Check all 19 points have valid nakshatra
    let mut all_entries: Vec<(&str, &dhruv_search::GrahaEntry)> = Vec::new();
    for (i, entry) in result.arudha_padas.iter().enumerate() {
        all_entries.push(("arudha_pada", entry));
        let _ = i;
    }
    all_entries.push(("bhrigu_bindu", &result.bhrigu_bindu));
    all_entries.push(("pranapada_lagna", &result.pranapada_lagna));
    all_entries.push(("gulika", &result.gulika));
    all_entries.push(("maandi", &result.maandi));
    all_entries.push(("hora_lagna", &result.hora_lagna));
    all_entries.push(("ghati_lagna", &result.ghati_lagna));
    all_entries.push(("sree_lagna", &result.sree_lagna));

    for (name, entry) in &all_entries {
        assert!(
            entry.nakshatra_index <= 26,
            "{} nakshatra_index should be 0-26, got {}",
            name,
            entry.nakshatra_index,
        );
        assert!(
            entry.pada >= 1 && entry.pada <= 4,
            "{} pada should be 1-4, got {}",
            name,
            entry.pada,
        );
        // Bhava should still be sentinel
        assert_eq!(entry.bhava_number, 0, "{} bhava should be sentinel", name);
    }
}

// ===== Bhava flag =====

#[test]
fn include_bhava() {
    let Some(engine) = load_engine() else { return };
    let Some(eop) = load_eop() else { return };
    let utc = utc_2024_jan_15();
    let location = new_delhi();
    let bhava_config = BhavaConfig::default();
    let rs_config = RiseSetConfig::default();
    let aya_config = default_aya_config();
    let config = BindusConfig {
        include_nakshatra: false,
        include_bhava: true,
    };

    let result = core_bindus(
        &engine,
        &eop,
        &utc,
        &location,
        &bhava_config,
        &rs_config,
        &aya_config,
        &config,
    )
    .expect("core_bindus should succeed");

    for (i, entry) in result.arudha_padas.iter().enumerate() {
        assert!(
            entry.bhava_number >= 1 && entry.bhava_number <= 12,
            "arudha_pada[{}] bhava should be 1-12, got {}",
            i,
            entry.bhava_number,
        );
        // Nakshatra should still be sentinel
        assert_eq!(
            entry.nakshatra_index, 255,
            "arudha_pada[{}] nakshatra should be sentinel",
            i
        );
    }

    let points = [
        ("bhrigu_bindu", result.bhrigu_bindu),
        ("pranapada_lagna", result.pranapada_lagna),
        ("gulika", result.gulika),
        ("maandi", result.maandi),
        ("hora_lagna", result.hora_lagna),
        ("ghati_lagna", result.ghati_lagna),
        ("sree_lagna", result.sree_lagna),
    ];
    for (name, entry) in &points {
        assert!(
            entry.bhava_number >= 1 && entry.bhava_number <= 12,
            "{} bhava should be 1-12, got {}",
            name,
            entry.bhava_number,
        );
        assert_eq!(
            entry.nakshatra_index, 255,
            "{} nakshatra should be sentinel",
            name
        );
    }
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
    let config = BindusConfig {
        include_nakshatra: true,
        include_bhava: true,
    };

    let result = core_bindus(
        &engine,
        &eop,
        &utc,
        &location,
        &bhava_config,
        &rs_config,
        &aya_config,
        &config,
    )
    .expect("core_bindus should succeed");

    // Check all 19 points
    for (i, entry) in result.arudha_padas.iter().enumerate() {
        assert!(
            entry.sidereal_longitude >= 0.0 && entry.sidereal_longitude < 360.0,
            "arudha_pada[{}] longitude out of range",
            i,
        );
        assert!(
            entry.nakshatra_index <= 26,
            "arudha_pada[{}] nakshatra out of range",
            i
        );
        assert!(
            entry.pada >= 1 && entry.pada <= 4,
            "arudha_pada[{}] pada out of range",
            i
        );
        assert!(
            entry.bhava_number >= 1 && entry.bhava_number <= 12,
            "arudha_pada[{}] bhava out of range",
            i,
        );
    }

    let points = [
        ("bhrigu_bindu", result.bhrigu_bindu),
        ("pranapada_lagna", result.pranapada_lagna),
        ("gulika", result.gulika),
        ("maandi", result.maandi),
        ("hora_lagna", result.hora_lagna),
        ("ghati_lagna", result.ghati_lagna),
        ("sree_lagna", result.sree_lagna),
    ];
    for (name, entry) in &points {
        assert!(
            entry.sidereal_longitude >= 0.0 && entry.sidereal_longitude < 360.0,
            "{} longitude out of range",
            name,
        );
        assert!(
            entry.nakshatra_index <= 26,
            "{} nakshatra out of range",
            name
        );
        assert!(
            entry.pada >= 1 && entry.pada <= 4,
            "{} pada out of range",
            name
        );
        assert!(
            entry.bhava_number >= 1 && entry.bhava_number <= 12,
            "{} bhava out of range",
            name,
        );
    }
}

// ===== Cross-check: arudha padas match standalone =====

#[test]
fn arudha_padas_match_standalone() {
    let Some(engine) = load_engine() else { return };
    let Some(eop) = load_eop() else { return };
    let utc = utc_2024_jan_15();
    let location = new_delhi();
    let bhava_config = BhavaConfig::default();
    let rs_config = RiseSetConfig::default();
    let aya_config = default_aya_config();
    let config = BindusConfig::default();

    let bindus = core_bindus(
        &engine,
        &eop,
        &utc,
        &location,
        &bhava_config,
        &rs_config,
        &aya_config,
        &config,
    )
    .expect("core_bindus should succeed");

    let standalone = dhruv_search::arudha_padas_for_date(
        &engine,
        &eop,
        &utc,
        &location,
        &bhava_config,
        &aya_config,
    )
    .expect("arudha_padas_for_date should succeed");

    for i in 0..12 {
        let diff = (bindus.arudha_padas[i].sidereal_longitude - standalone[i].longitude_deg).abs();
        assert!(
            diff < 1e-10,
            "arudha_pada[{}] mismatch: bindus={:.10}, standalone={:.10}",
            i,
            bindus.arudha_padas[i].sidereal_longitude,
            standalone[i].longitude_deg,
        );
    }
}

// ===== Cross-check: special lagnas match standalone =====

#[test]
fn special_lagnas_match_standalone() {
    let Some(engine) = load_engine() else { return };
    let Some(eop) = load_eop() else { return };
    let utc = utc_2024_jan_15();
    let location = new_delhi();
    let bhava_config = BhavaConfig::default();
    let rs_config = RiseSetConfig::default();
    let aya_config = default_aya_config();
    let config = BindusConfig::default();

    let bindus = core_bindus(
        &engine,
        &eop,
        &utc,
        &location,
        &bhava_config,
        &rs_config,
        &aya_config,
        &config,
    )
    .expect("core_bindus should succeed");

    let standalone = dhruv_search::special_lagnas_for_date(
        &engine,
        &eop,
        &utc,
        &location,
        &rs_config,
        &aya_config,
    )
    .expect("special_lagnas_for_date should succeed");

    let checks = [
        (
            "hora_lagna",
            bindus.hora_lagna.sidereal_longitude,
            standalone.hora_lagna,
        ),
        (
            "ghati_lagna",
            bindus.ghati_lagna.sidereal_longitude,
            standalone.ghati_lagna,
        ),
        (
            "pranapada_lagna",
            bindus.pranapada_lagna.sidereal_longitude,
            standalone.pranapada_lagna,
        ),
        (
            "sree_lagna",
            bindus.sree_lagna.sidereal_longitude,
            standalone.sree_lagna,
        ),
    ];
    for (name, bindus_lon, standalone_lon) in &checks {
        let diff = (bindus_lon - standalone_lon).abs();
        assert!(
            diff < 1e-10,
            "{} mismatch: bindus={:.10}, standalone={:.10}",
            name,
            bindus_lon,
            standalone_lon,
        );
    }
}

// ===== Cross-check: gulika/maandi match standalone upagrahas =====

#[test]
fn gulika_maandi_match_upagrahas() {
    let Some(engine) = load_engine() else { return };
    let Some(eop) = load_eop() else { return };
    let utc = utc_2024_jan_15();
    let location = new_delhi();
    let bhava_config = BhavaConfig::default();
    let rs_config = RiseSetConfig::default();
    let aya_config = default_aya_config();
    let config = BindusConfig::default();

    let bindus = core_bindus(
        &engine,
        &eop,
        &utc,
        &location,
        &bhava_config,
        &rs_config,
        &aya_config,
        &config,
    )
    .expect("core_bindus should succeed");

    let standalone = dhruv_search::all_upagrahas_for_date(
        &engine,
        &eop,
        &utc,
        &location,
        &rs_config,
        &aya_config,
    )
    .expect("all_upagrahas_for_date should succeed");

    let gulika_diff = (bindus.gulika.sidereal_longitude - standalone.gulika).abs();
    assert!(
        gulika_diff < 1e-10,
        "gulika mismatch: bindus={:.10}, standalone={:.10}",
        bindus.gulika.sidereal_longitude,
        standalone.gulika,
    );

    let maandi_diff = (bindus.maandi.sidereal_longitude - standalone.maandi).abs();
    assert!(
        maandi_diff < 1e-10,
        "maandi mismatch: bindus={:.10}, standalone={:.10}",
        bindus.maandi.sidereal_longitude,
        standalone.maandi,
    );
}
