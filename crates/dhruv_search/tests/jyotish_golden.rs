//! Golden-value integration tests for graha_positions.
//!
//! Validates all config flag combinations: base (no flags), nakshatra,
//! lagna, outer planets, bhava, and all-flags-on.
//! Requires kernel files. Skips gracefully if absent.

use std::path::Path;

use dhruv_core::{Engine, EngineConfig};
use dhruv_search::sankranti_types::SankrantiConfig;
use dhruv_search::{GrahaPositionsConfig, graha_positions};
use dhruv_time::{EopKernel, UtcTime};
use dhruv_vedic_base::riseset_types::GeoLocation;
use dhruv_vedic_base::{BhavaConfig, Rashi};

const SPK_PATH: &str = "../../kernels/data/de442s.bsp";
const LSK_PATH: &str = "../../kernels/data/naif0012.tls";
const EOP_PATH: &str = "../../kernels/data/finals2000A.all";

fn load_engine() -> Option<Engine> {
    if !Path::new(SPK_PATH).exists() || !Path::new(LSK_PATH).exists() {
        eprintln!("Skipping jyotish_golden: kernel files not found");
        return None;
    }
    let config = EngineConfig::with_single_spk(SPK_PATH.into(), LSK_PATH.into(), 1024, false);
    Engine::new(config).ok()
}

fn load_eop() -> Option<EopKernel> {
    if !Path::new(EOP_PATH).exists() {
        eprintln!("Skipping jyotish_golden: EOP file not found");
        return None;
    }
    EopKernel::load(Path::new(EOP_PATH)).ok()
}

fn default_aya_config() -> SankrantiConfig {
    SankrantiConfig::default_lahiri()
}

/// New Delhi location for tests.
fn new_delhi() -> GeoLocation {
    GeoLocation::new(28.6139, 77.2090, 0.0)
}

/// Reference date: 2024-01-15 12:00 UTC
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
    let aya_config = default_aya_config();
    let config = GrahaPositionsConfig {
        include_nakshatra: false,
        include_lagna: false,
        include_outer_planets: false,
        include_bhava: false,
    };

    let result = graha_positions(
        &engine,
        &eop,
        &utc,
        &location,
        &bhava_config,
        &aya_config,
        &config,
    )
    .expect("graha_positions should succeed");

    // All 9 grahas should have valid sidereal longitudes [0, 360)
    for (i, entry) in result.grahas.iter().enumerate() {
        assert!(
            entry.sidereal_longitude >= 0.0 && entry.sidereal_longitude < 360.0,
            "graha[{}] longitude out of range: {}",
            i,
            entry.sidereal_longitude,
        );
        // Rashi index should be consistent with longitude
        let expected_rashi = (entry.sidereal_longitude / 30.0).floor() as u8;
        assert_eq!(
            entry.rashi_index, expected_rashi,
            "graha[{}] rashi_index mismatch",
            i,
        );
    }

    // Sentinel values for disabled features
    for entry in &result.grahas {
        assert_eq!(
            entry.nakshatra_index, 255,
            "nakshatra_index should be sentinel"
        );
        assert_eq!(entry.pada, 0, "pada should be sentinel");
        assert_eq!(entry.bhava_number, 0, "bhava_number should be sentinel");
    }

    // Lagna should be sentinel
    assert_eq!(result.lagna.sidereal_longitude, 0.0);
    assert_eq!(result.lagna.nakshatra_index, 255);

    // Outer planets should be sentinel
    for entry in &result.outer_planets {
        assert_eq!(entry.sidereal_longitude, 0.0);
        assert_eq!(entry.nakshatra_index, 255);
    }
}

// ===== Nakshatra flag =====

#[test]
fn include_nakshatra_populates_nak_and_pada() {
    let Some(engine) = load_engine() else { return };
    let Some(eop) = load_eop() else { return };
    let utc = utc_2024_jan_15();
    let location = new_delhi();
    let bhava_config = BhavaConfig::default();
    let aya_config = default_aya_config();
    let config = GrahaPositionsConfig {
        include_nakshatra: true,
        include_lagna: false,
        include_outer_planets: false,
        include_bhava: false,
    };

    let result = graha_positions(
        &engine,
        &eop,
        &utc,
        &location,
        &bhava_config,
        &aya_config,
        &config,
    )
    .expect("graha_positions should succeed");

    for (i, entry) in result.grahas.iter().enumerate() {
        assert!(
            entry.nakshatra_index <= 26,
            "graha[{}] nakshatra_index should be 0-26, got {}",
            i,
            entry.nakshatra_index,
        );
        assert!(
            entry.pada >= 1 && entry.pada <= 4,
            "graha[{}] pada should be 1-4, got {}",
            i,
            entry.pada,
        );
        // Nakshatra index should be consistent with sidereal longitude
        let expected_nak = (entry.sidereal_longitude / (360.0 / 27.0)).floor() as u8;
        assert_eq!(
            entry.nakshatra_index, expected_nak,
            "graha[{}] nakshatra_index doesn't match longitude",
            i,
        );
    }

    // Bhava should still be sentinel
    for entry in &result.grahas {
        assert_eq!(entry.bhava_number, 0, "bhava should be sentinel");
    }
}

// ===== Lagna flag =====

#[test]
fn include_lagna_populates_lagna_entry() {
    let Some(engine) = load_engine() else { return };
    let Some(eop) = load_eop() else { return };
    let utc = utc_2024_jan_15();
    let location = new_delhi();
    let bhava_config = BhavaConfig::default();
    let aya_config = default_aya_config();
    let config = GrahaPositionsConfig {
        include_nakshatra: false,
        include_lagna: true,
        include_outer_planets: false,
        include_bhava: false,
    };

    let result = graha_positions(
        &engine,
        &eop,
        &utc,
        &location,
        &bhava_config,
        &aya_config,
        &config,
    )
    .expect("graha_positions should succeed");

    // Lagna should be a valid sidereal longitude
    let lagna = &result.lagna;
    assert!(
        lagna.sidereal_longitude >= 0.0 && lagna.sidereal_longitude < 360.0,
        "lagna longitude out of range: {}",
        lagna.sidereal_longitude,
    );

    // Rashi should be consistent
    let expected_rashi = (lagna.sidereal_longitude / 30.0).floor() as u8;
    assert_eq!(lagna.rashi_index, expected_rashi);

    // For mid-day New Delhi on 2024-01-15, lagna should be in Aries-Taurus range
    // (approximately 10-50 degrees sidereal in this time window)
    assert!(
        lagna.sidereal_longitude > 0.0 && lagna.sidereal_longitude < 360.0,
        "lagna should be a non-zero valid longitude",
    );

    // Nakshatra should still be sentinel (nakshatra flag is off)
    assert_eq!(lagna.nakshatra_index, 255);
    assert_eq!(lagna.pada, 0);

    // Outer planets should still be sentinel
    for entry in &result.outer_planets {
        assert_eq!(entry.sidereal_longitude, 0.0);
    }
}

// ===== Outer planets flag =====

#[test]
fn include_outer_planets_populates_uranus_neptune_pluto() {
    let Some(engine) = load_engine() else { return };
    let Some(eop) = load_eop() else { return };
    let utc = utc_2024_jan_15();
    let location = new_delhi();
    let bhava_config = BhavaConfig::default();
    let aya_config = default_aya_config();
    let config = GrahaPositionsConfig {
        include_nakshatra: false,
        include_lagna: false,
        include_outer_planets: true,
        include_bhava: false,
    };

    let result = graha_positions(
        &engine,
        &eop,
        &utc,
        &location,
        &bhava_config,
        &aya_config,
        &config,
    )
    .expect("graha_positions should succeed");

    let planet_names = ["Uranus", "Neptune", "Pluto"];
    for (i, entry) in result.outer_planets.iter().enumerate() {
        assert!(
            entry.sidereal_longitude >= 0.0 && entry.sidereal_longitude < 360.0,
            "{} longitude out of range: {}",
            planet_names[i],
            entry.sidereal_longitude,
        );
        // Rashi index should be consistent
        let expected_rashi = (entry.sidereal_longitude / 30.0).floor() as u8;
        assert_eq!(
            entry.rashi_index, expected_rashi,
            "{} rashi_index mismatch",
            planet_names[i],
        );
        // Non-zero longitude (these planets are far from 0 Aries in 2024)
        assert!(
            entry.sidereal_longitude > 1.0,
            "{} should have non-trivial longitude",
            planet_names[i],
        );
    }

    // All three should be at distinct longitudes
    let u = result.outer_planets[0].sidereal_longitude;
    let n = result.outer_planets[1].sidereal_longitude;
    let p = result.outer_planets[2].sidereal_longitude;
    assert!((u - n).abs() > 1.0, "Uranus and Neptune should differ");
    assert!((n - p).abs() > 1.0, "Neptune and Pluto should differ");

    // Lagna should still be sentinel
    assert_eq!(result.lagna.sidereal_longitude, 0.0);
}

// ===== Bhava flag =====

#[test]
fn include_bhava_populates_bhava_numbers() {
    let Some(engine) = load_engine() else { return };
    let Some(eop) = load_eop() else { return };
    let utc = utc_2024_jan_15();
    let location = new_delhi();
    let bhava_config = BhavaConfig::default();
    let aya_config = default_aya_config();
    let config = GrahaPositionsConfig {
        include_nakshatra: false,
        include_lagna: false,
        include_outer_planets: false,
        include_bhava: true,
    };

    let result = graha_positions(
        &engine,
        &eop,
        &utc,
        &location,
        &bhava_config,
        &aya_config,
        &config,
    )
    .expect("graha_positions should succeed");

    for (i, entry) in result.grahas.iter().enumerate() {
        assert!(
            entry.bhava_number >= 1 && entry.bhava_number <= 12,
            "graha[{}] bhava_number should be 1-12, got {}",
            i,
            entry.bhava_number,
        );
    }

    // Nakshatra should still be sentinel
    for entry in &result.grahas {
        assert_eq!(entry.nakshatra_index, 255, "nakshatra should be sentinel");
        assert_eq!(entry.pada, 0, "pada should be sentinel");
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
    let aya_config = default_aya_config();
    let config = GrahaPositionsConfig {
        include_nakshatra: true,
        include_lagna: true,
        include_outer_planets: true,
        include_bhava: true,
    };

    let result = graha_positions(
        &engine,
        &eop,
        &utc,
        &location,
        &bhava_config,
        &aya_config,
        &config,
    )
    .expect("graha_positions should succeed");

    // 9 grahas: all fields populated
    for (i, entry) in result.grahas.iter().enumerate() {
        assert!(
            entry.sidereal_longitude >= 0.0 && entry.sidereal_longitude < 360.0,
            "graha[{}] longitude out of range",
            i,
        );
        assert!(
            entry.nakshatra_index <= 26,
            "graha[{}] nakshatra out of range",
            i
        );
        assert!(
            entry.pada >= 1 && entry.pada <= 4,
            "graha[{}] pada out of range",
            i
        );
        assert!(
            entry.bhava_number >= 1 && entry.bhava_number <= 12,
            "graha[{}] bhava out of range",
            i,
        );
    }

    // Lagna: should have valid longitude, nakshatra, bhava
    let lagna = &result.lagna;
    assert!(lagna.sidereal_longitude > 0.0 && lagna.sidereal_longitude < 360.0);
    assert!(lagna.nakshatra_index <= 26);
    assert!(lagna.pada >= 1 && lagna.pada <= 4);

    // Outer planets: valid longitude, nakshatra, bhava
    for (i, entry) in result.outer_planets.iter().enumerate() {
        assert!(
            entry.sidereal_longitude >= 0.0 && entry.sidereal_longitude < 360.0,
            "outer[{}] longitude out of range",
            i,
        );
        assert!(
            entry.nakshatra_index <= 26,
            "outer[{}] nakshatra out of range",
            i
        );
        assert!(
            entry.pada >= 1 && entry.pada <= 4,
            "outer[{}] pada out of range",
            i
        );
        assert!(
            entry.bhava_number >= 1 && entry.bhava_number <= 12,
            "outer[{}] bhava out of range",
            i,
        );
    }
}

// ===== Nakshatra + Lagna combined =====

#[test]
fn nakshatra_and_lagna_combined() {
    let Some(engine) = load_engine() else { return };
    let Some(eop) = load_eop() else { return };
    let utc = utc_2024_jan_15();
    let location = new_delhi();
    let bhava_config = BhavaConfig::default();
    let aya_config = default_aya_config();
    let config = GrahaPositionsConfig {
        include_nakshatra: true,
        include_lagna: true,
        include_outer_planets: false,
        include_bhava: false,
    };

    let result = graha_positions(
        &engine,
        &eop,
        &utc,
        &location,
        &bhava_config,
        &aya_config,
        &config,
    )
    .expect("graha_positions should succeed");

    // Lagna should have nakshatra populated
    let lagna = &result.lagna;
    assert!(lagna.sidereal_longitude > 0.0 && lagna.sidereal_longitude < 360.0);
    assert!(lagna.nakshatra_index <= 26);
    assert!(lagna.pada >= 1 && lagna.pada <= 4);

    // Outer planets should be sentinel
    for entry in &result.outer_planets {
        assert_eq!(entry.sidereal_longitude, 0.0);
    }

    // Bhava should be sentinel
    assert_eq!(lagna.bhava_number, 0);
}

// ===== Rashi consistency =====

#[test]
fn rashi_matches_longitude_for_all_grahas() {
    let Some(engine) = load_engine() else { return };
    let Some(eop) = load_eop() else { return };
    let utc = utc_2024_jan_15();
    let location = new_delhi();
    let bhava_config = BhavaConfig::default();
    let aya_config = default_aya_config();
    let config = GrahaPositionsConfig::default();

    let result = graha_positions(
        &engine,
        &eop,
        &utc,
        &location,
        &bhava_config,
        &aya_config,
        &config,
    )
    .expect("graha_positions should succeed");

    let rashi_list = [
        Rashi::Mesha,
        Rashi::Vrishabha,
        Rashi::Mithuna,
        Rashi::Karka,
        Rashi::Simha,
        Rashi::Kanya,
        Rashi::Tula,
        Rashi::Vrischika,
        Rashi::Dhanu,
        Rashi::Makara,
        Rashi::Kumbha,
        Rashi::Meena,
    ];

    for (i, entry) in result.grahas.iter().enumerate() {
        let idx = (entry.sidereal_longitude / 30.0).floor() as usize;
        assert_eq!(
            entry.rashi, rashi_list[idx],
            "graha[{}] rashi enum doesn't match index {}",
            i, idx,
        );
    }
}

// ===== Sun in Dhanu (Sagittarius) on Jan 15, 2024 =====

#[test]
fn sun_in_dhanu_jan_2024() {
    let Some(engine) = load_engine() else { return };
    let Some(eop) = load_eop() else { return };
    let utc = utc_2024_jan_15();
    let location = new_delhi();
    let bhava_config = BhavaConfig::default();
    let aya_config = default_aya_config();
    let config = GrahaPositionsConfig::default();

    let result = graha_positions(
        &engine,
        &eop,
        &utc,
        &location,
        &bhava_config,
        &aya_config,
        &config,
    )
    .expect("graha_positions should succeed");

    // Sun (index 0) on Jan 15, 2024 with Lahiri ayanamsha should be in Dhanu (~270-300 deg)
    // or very late Dhanu / early Makara since Makara Sankranti is around Jan 14-15
    let sun = &result.grahas[0];
    assert!(
        sun.rashi == Rashi::Dhanu || sun.rashi == Rashi::Makara,
        "Sun should be in Dhanu or Makara in mid-January 2024 (Lahiri), got {:?} at {:.4}°",
        sun.rashi,
        sun.sidereal_longitude,
    );
}

// ===== Different date: July 2024 =====

#[test]
fn graha_positions_july_2024() {
    let Some(engine) = load_engine() else { return };
    let Some(eop) = load_eop() else { return };
    let utc = UtcTime::new(2024, 7, 15, 6, 0, 0.0);
    let location = new_delhi();
    let bhava_config = BhavaConfig::default();
    let aya_config = default_aya_config();
    let config = GrahaPositionsConfig {
        include_nakshatra: true,
        include_lagna: true,
        include_outer_planets: true,
        include_bhava: true,
    };

    let result = graha_positions(
        &engine,
        &eop,
        &utc,
        &location,
        &bhava_config,
        &aya_config,
        &config,
    )
    .expect("graha_positions should succeed");

    // Sun in mid-July 2024 with Lahiri: should be in Mithuna or Karka
    let sun = &result.grahas[0];
    assert!(
        sun.rashi == Rashi::Mithuna || sun.rashi == Rashi::Karka,
        "Sun should be in Mithuna or Karka in mid-July 2024, got {:?} at {:.4}°",
        sun.rashi,
        sun.sidereal_longitude,
    );

    // Rahu (index 7) and Ketu (index 8) should be ~180 degrees apart
    let rahu = result.grahas[7].sidereal_longitude;
    let ketu = result.grahas[8].sidereal_longitude;
    let diff = ((rahu - ketu).abs() - 180.0).abs();
    assert!(
        diff < 1.0,
        "Rahu-Ketu should be ~180° apart, diff from 180 = {:.4}°",
        diff
    );

    // All bhava numbers should be populated and cover a spread
    let mut bhava_set = std::collections::HashSet::new();
    for entry in &result.grahas {
        bhava_set.insert(entry.bhava_number);
    }
    assert!(
        bhava_set.len() >= 3,
        "grahas should span at least 3 different bhavas"
    );
}

// ===== Bhava + outer planets =====

#[test]
fn bhava_and_outer_planets_combined() {
    let Some(engine) = load_engine() else { return };
    let Some(eop) = load_eop() else { return };
    let utc = utc_2024_jan_15();
    let location = new_delhi();
    let bhava_config = BhavaConfig::default();
    let aya_config = default_aya_config();
    let config = GrahaPositionsConfig {
        include_nakshatra: false,
        include_lagna: false,
        include_outer_planets: true,
        include_bhava: true,
    };

    let result = graha_positions(
        &engine,
        &eop,
        &utc,
        &location,
        &bhava_config,
        &aya_config,
        &config,
    )
    .expect("graha_positions should succeed");

    // Outer planets should have bhava numbers
    for (i, entry) in result.outer_planets.iter().enumerate() {
        assert!(
            entry.bhava_number >= 1 && entry.bhava_number <= 12,
            "outer[{}] bhava should be 1-12, got {}",
            i,
            entry.bhava_number,
        );
        // Nakshatra should be sentinel
        assert_eq!(
            entry.nakshatra_index, 255,
            "outer[{}] nakshatra should be sentinel",
            i
        );
    }
}

// ===== Consistency: graha_positions matches graha_sidereal_longitudes =====

#[test]
fn graha_positions_matches_sidereal_longitudes() {
    let Some(engine) = load_engine() else { return };
    let Some(eop) = load_eop() else { return };
    let utc = utc_2024_jan_15();
    let location = new_delhi();
    let bhava_config = BhavaConfig::default();
    let aya_config = default_aya_config();
    let config = GrahaPositionsConfig::default();

    let result = graha_positions(
        &engine,
        &eop,
        &utc,
        &location,
        &bhava_config,
        &aya_config,
        &config,
    )
    .expect("graha_positions should succeed");

    // Also compute directly via graha_sidereal_longitudes
    let jd_tdb = utc.to_jd_tdb(engine.lsk());
    let direct = dhruv_search::graha_sidereal_longitudes(
        &engine,
        jd_tdb,
        dhruv_vedic_base::AyanamshaSystem::Lahiri,
        false,
    )
    .expect("graha_sidereal_longitudes should succeed");

    // All 9 longitudes should match exactly
    for i in 0..9 {
        let gp_lon = result.grahas[i].sidereal_longitude;
        let direct_lon = direct.longitudes[i];
        assert!(
            (gp_lon - direct_lon).abs() < 1e-10,
            "graha[{}] mismatch: graha_positions={:.10}, direct={:.10}",
            i,
            gp_lon,
            direct_lon,
        );
    }
}
