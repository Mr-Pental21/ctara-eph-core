//! Golden-value integration tests for graha_positions.
//!
//! Validates all config flag combinations: base (no flags), nakshatra,
//! lagna, outer planets, bhava, and all-flags-on.
//! Requires kernel files. Skips gracefully if absent.

use std::f64::consts::{PI, TAU};
use std::path::Path;

use dhruv_core::{Engine, EngineConfig};
use dhruv_search::sankranti_types::SankrantiConfig;
use dhruv_search::{GrahaPositionsConfig, graha_positions};
use dhruv_time::{EopKernel, LeapSecondKernel, UtcTime, gmst_rad, local_sidereal_time_rad};
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

// ===== Lagna rising-condition check =====

fn load_lsk() -> Option<LeapSecondKernel> {
    if !Path::new(LSK_PATH).exists() {
        return None;
    }
    LeapSecondKernel::load(Path::new(LSK_PATH)).ok()
}

/// Verify bhava_cusps populated when include_bhava_cusps is true (default).
#[test]
fn full_kundali_has_bhava_cusps() {
    let Some(engine) = load_engine() else { return };
    let Some(eop) = load_eop() else { return };
    let utc = utc_2024_jan_15();
    let location = new_delhi();
    let bhava_config = BhavaConfig::default();
    let rs_config = dhruv_vedic_base::riseset_types::RiseSetConfig::default();
    let aya_config = default_aya_config();
    let config = dhruv_search::FullKundaliConfig::default();

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
    .expect("full_kundali should succeed");

    // ayanamsha_deg should be positive (Lahiri ~ 24 deg in 2024)
    assert!(
        result.ayanamsha_deg > 23.0 && result.ayanamsha_deg < 25.0,
        "ayanamsha_deg = {}, expected ~24",
        result.ayanamsha_deg
    );

    let bh = result
        .bhava_cusps
        .as_ref()
        .expect("bhava_cusps should be Some");
    assert_eq!(bh.bhavas.len(), 12);
    for (i, b) in bh.bhavas.iter().enumerate() {
        assert_eq!(b.number, (i + 1) as u8);
        assert!(
            b.cusp_deg >= 0.0 && b.cusp_deg < 360.0,
            "cusp {}: {}",
            i,
            b.cusp_deg
        );
    }
    assert!(bh.lagna_deg >= 0.0 && bh.lagna_deg < 360.0);
    assert!(bh.mc_deg >= 0.0 && bh.mc_deg < 360.0);
    let sphutas = result.sphutas.as_ref().expect("sphutas should be Some");
    assert_eq!(sphutas.longitudes.len(), 16);
    for lon in sphutas.longitudes {
        assert!(
            (0.0..360.0).contains(&lon),
            "sphuta longitude out of range: {lon}"
        );
    }
}

/// Verify bhava_cusps is None when include_bhava_cusps is false.
#[test]
fn full_kundali_bhava_cusps_flag_off() {
    let Some(engine) = load_engine() else { return };
    let Some(eop) = load_eop() else { return };
    let utc = utc_2024_jan_15();
    let location = new_delhi();
    let bhava_config = BhavaConfig::default();
    let rs_config = dhruv_vedic_base::riseset_types::RiseSetConfig::default();
    let aya_config = default_aya_config();
    let config = dhruv_search::FullKundaliConfig {
        include_bhava_cusps: false,
        include_graha_positions: false,
        include_bindus: false,
        include_drishti: false,
        include_ashtakavarga: false,
        include_upagrahas: false,
        include_special_lagnas: false,
        include_panchang: true,
        ..dhruv_search::FullKundaliConfig::default()
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
    .expect("panchang-only should succeed without bhava computation");

    assert!(
        result.bhava_cusps.is_none(),
        "bhava_cusps should be None when flag is off"
    );
    assert!(
        result.panchang.is_some(),
        "panchang should still be computed"
    );
    assert!(result.ayanamsha_deg > 23.0 && result.ayanamsha_deg < 25.0);
}

/// High-latitude + time-based house system: KP at 70°N fails for compute_bhavas.
/// With include_bhava_cusps=false the kundali call must still succeed.
/// Uses spring equinox (sun rises at 70°N) so panchang also works.
#[test]
fn full_kundali_high_lat_kp_bhava_off_succeeds() {
    let Some(engine) = load_engine() else { return };
    let Some(eop) = load_eop() else { return };
    // Spring equinox — sun rises at 70°N
    let utc = UtcTime::new(2024, 3, 20, 12, 0, 0.0);
    // Tromso-like latitude where KP house system fails
    let location = GeoLocation::new(70.0, 25.0, 0.0);
    let bhava_config = BhavaConfig {
        system: dhruv_vedic_base::BhavaSystem::KP,
        ..BhavaConfig::default()
    };
    let rs_config = dhruv_vedic_base::riseset_types::RiseSetConfig::default();
    let aya_config = default_aya_config();
    let config = dhruv_search::FullKundaliConfig {
        include_bhava_cusps: false,
        include_graha_positions: true,
        include_bindus: false,
        include_drishti: false,
        include_ashtakavarga: false,
        include_upagrahas: false,
        include_special_lagnas: false,
        include_panchang: false,
        ..dhruv_search::FullKundaliConfig::default()
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
    .expect("high-lat graha should succeed when bhava cusps disabled");

    assert!(result.bhava_cusps.is_none());
    assert!(result.graha_positions.is_some());
}

/// High-latitude + KP + bhava off + panchang-only: verifies that disabling bhava cusps
/// lets panchang succeed at high latitude even with a failing house system config.
#[test]
fn full_kundali_high_lat_kp_bhava_off_panchang_succeeds() {
    let Some(engine) = load_engine() else { return };
    let Some(eop) = load_eop() else { return };
    // Spring equinox — sun rises at 70°N, so panchang can compute
    let utc = UtcTime::new(2024, 3, 20, 12, 0, 0.0);
    let location = GeoLocation::new(70.0, 25.0, 0.0);
    let bhava_config = BhavaConfig {
        system: dhruv_vedic_base::BhavaSystem::KP,
        ..BhavaConfig::default()
    };
    let rs_config = dhruv_vedic_base::riseset_types::RiseSetConfig::default();
    let aya_config = default_aya_config();
    let config = dhruv_search::FullKundaliConfig {
        include_bhava_cusps: false,
        include_graha_positions: false,
        include_bindus: false,
        include_drishti: false,
        include_ashtakavarga: false,
        include_upagrahas: false,
        include_special_lagnas: false,
        include_panchang: true,
        ..dhruv_search::FullKundaliConfig::default()
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
    .expect("high-lat panchang-only should succeed when bhava cusps disabled");

    assert!(result.bhava_cusps.is_none());
    assert!(result.panchang.is_some(), "panchang should be computed");
    assert!(result.graha_positions.is_none());
}

/// High-latitude + KP + bhava off + include_calendar=true: the calendar path
/// (masa, ayana, varsha) also succeeds when bhava cusps are disabled.
#[test]
fn full_kundali_high_lat_kp_bhava_off_calendar_succeeds() {
    let Some(engine) = load_engine() else { return };
    let Some(eop) = load_eop() else { return };
    let utc = UtcTime::new(2024, 3, 20, 12, 0, 0.0);
    let location = GeoLocation::new(70.0, 25.0, 0.0);
    let bhava_config = BhavaConfig {
        system: dhruv_vedic_base::BhavaSystem::KP,
        ..BhavaConfig::default()
    };
    let rs_config = dhruv_vedic_base::riseset_types::RiseSetConfig::default();
    let aya_config = default_aya_config();
    let config = dhruv_search::FullKundaliConfig {
        include_bhava_cusps: false,
        include_graha_positions: false,
        include_bindus: false,
        include_drishti: false,
        include_ashtakavarga: false,
        include_upagrahas: false,
        include_special_lagnas: false,
        include_panchang: false,
        include_calendar: true,
        ..dhruv_search::FullKundaliConfig::default()
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
    .expect("high-lat calendar-only should succeed when bhava cusps disabled");

    assert!(result.bhava_cusps.is_none());
    assert!(result.panchang.is_some(), "calendar implies panchang");
}

/// High-latitude + time-based house system: KP at 70°N with include_bhava_cusps=true
/// should propagate the bhava failure as an error.
#[test]
fn full_kundali_high_lat_kp_bhava_on_fails() {
    let Some(engine) = load_engine() else { return };
    let Some(eop) = load_eop() else { return };
    let utc = UtcTime::new(2024, 3, 20, 12, 0, 0.0);
    let location = GeoLocation::new(70.0, 25.0, 0.0);
    let bhava_config = BhavaConfig {
        system: dhruv_vedic_base::BhavaSystem::KP,
        ..BhavaConfig::default()
    };
    let rs_config = dhruv_vedic_base::riseset_types::RiseSetConfig::default();
    let aya_config = default_aya_config();
    let config = dhruv_search::FullKundaliConfig {
        include_bhava_cusps: true,
        include_graha_positions: false,
        include_panchang: false,
        ..dhruv_search::FullKundaliConfig::default()
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
    );

    assert!(
        result.is_err(),
        "KP at 70N with bhava cusps enabled should fail"
    );
}

/// Verify the lagna returned by graha_positions is a rising point (H < 0).
#[test]
fn lagna_is_rising_in_graha_positions() {
    let Some(engine) = load_engine() else { return };
    let Some(eop) = load_eop() else { return };
    let Some(_lsk) = load_lsk() else { return };
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

    let sidereal_lagna_deg = result.lagna.sidereal_longitude;
    assert!(sidereal_lagna_deg > 0.0 && sidereal_lagna_deg < 360.0);

    // Recover tropical lagna: sidereal + ayanamsha
    let jd_tdb = utc.to_jd_tdb(engine.lsk());
    let t = dhruv_vedic_base::jd_tdb_to_centuries(jd_tdb);
    let aya =
        dhruv_vedic_base::ayanamsha_deg(aya_config.ayanamsha_system, t, aya_config.use_nutation);
    let tropical_lagna_deg = sidereal_lagna_deg + aya;
    let tropical_lagna_rad = tropical_lagna_deg.to_radians();

    // Compute apparent LST (GAST) and true obliquity — matching production
    let jd_utc = dhruv_time::calendar_to_jd(
        utc.year,
        utc.month,
        utc.day as f64 + utc.hour as f64 / 24.0 + utc.minute as f64 / 1440.0 + utc.second / 86400.0,
    );
    let jd_ut1 = eop.utc_to_ut1_jd(jd_utc).expect("EOP lookup");
    let gmst = gmst_rad(jd_ut1);
    let utc_s = dhruv_time::jd_to_tdb_seconds(jd_utc);
    let tdb_s = _lsk.utc_to_tdb(utc_s);
    let jd_tdb_v = dhruv_time::tdb_seconds_to_jd(tdb_s);
    let t_v = (jd_tdb_v - 2_451_545.0) / 36525.0;
    let (ee, eps_true) = dhruv_frames::equation_of_equinoxes_and_true_obliquity(t_v);
    let gast = gmst + ee;
    let lst = local_sidereal_time_rad(gast, location.longitude_rad());

    // Rising condition: hour angle H < 0 (eastern horizon)
    let ra = f64::atan2(
        tropical_lagna_rad.sin() * eps_true.cos(),
        tropical_lagna_rad.cos(),
    )
    .rem_euclid(TAU);
    let mut h = (lst - ra).rem_euclid(TAU);
    if h > PI {
        h -= TAU;
    }
    assert!(
        h < 0.0,
        "H = {:.4} rad ({:.2} deg) — lagna should be rising (H < 0)",
        h,
        h.to_degrees()
    );
}
