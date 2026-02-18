//! Integration tests for Dasha orchestration.
//!
//! Requires kernel files. Skips gracefully if absent.

use std::path::Path;

use dhruv_core::{Engine, EngineConfig};
use dhruv_search::sankranti_types::SankrantiConfig;
use dhruv_search::{
    DashaInputs, DashaSelectionConfig, FullKundaliConfig, dasha_hierarchy_for_birth,
    dasha_hierarchy_with_inputs, dasha_snapshot_at, full_kundali_for_date,
    graha_sidereal_longitudes,
};
use dhruv_time::{EopKernel, UtcTime};
use dhruv_vedic_base::BhavaConfig;
use dhruv_vedic_base::dasha::{
    DashaEntity, DashaLevel, DashaSystem, DashaVariationConfig, RashiDashaInputs,
};
use dhruv_vedic_base::riseset_types::{GeoLocation, RiseSetConfig};

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
    assert!(
        result.is_ok(),
        "hierarchy should succeed: {:?}",
        result.err()
    );

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
    let total_days =
        hierarchy.levels[0].last().unwrap().end_jd - hierarchy.levels[0].first().unwrap().start_jd;
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
    assert!(
        result.is_ok(),
        "snapshot should succeed: {:?}",
        result.err()
    );

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

/// Kala (graha-based) dasha should produce valid hierarchy.
#[test]
fn kala_hierarchy_valid() {
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
        DashaSystem::Kala,
        1,
        &bhava_config,
        &rs_config,
        &aya_config,
        &variation,
    );
    assert!(result.is_ok(), "Kala hierarchy: {:?}", result.err());
    let h = result.unwrap();
    assert_eq!(h.system, DashaSystem::Kala);
    assert_eq!(h.levels.len(), 2);
    assert_eq!(h.levels[0].len(), 9); // 9 graha periods
}

/// Kaal Chakra (special) dasha should produce valid hierarchy.
#[test]
fn kaal_chakra_hierarchy_valid() {
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
        DashaSystem::KaalChakra,
        1,
        &bhava_config,
        &rs_config,
        &aya_config,
        &variation,
    );
    assert!(result.is_ok(), "KaalChakra hierarchy: {:?}", result.err());
    let h = result.unwrap();
    assert_eq!(h.system, DashaSystem::KaalChakra);
    assert_eq!(h.levels.len(), 2);
    // current DP (remaining rashis from start_pos) + next DP (9 rashis)
    assert!(h.levels[0].len() >= 10 && h.levels[0].len() <= 18);
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
        assert_eq!(
            h.levels[0].len(),
            12,
            "{:?} should have 12 mahadasha periods",
            system
        );

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

// ====================================================================
// New integration tests (Phase D)
// ====================================================================

/// Chakra with real sunrise data: daytime birth should match Day hardcode.
#[test]
fn chakra_birth_period_from_sunrise() {
    let Some(engine) = load_engine() else { return };
    let Some(eop) = load_eop() else { return };
    // 1990-01-15 06:30 UTC at New Delhi is after sunrise (~06:15 local, ~00:45 UTC in Jan)
    // so it's daytime — should match the old BirthPeriod::Day result.
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
        DashaSystem::Chakra,
        1,
        &bhava_config,
        &rs_config,
        &aya_config,
        &variation,
    );
    assert!(result.is_ok(), "Chakra hierarchy: {:?}", result.err());
    let h = result.unwrap();
    assert_eq!(h.system, DashaSystem::Chakra);
    assert_eq!(h.levels[0].len(), 12);
}

/// DashaInputs API: works for nakshatra, rashi, and kala systems.
#[test]
fn dasha_hierarchy_with_inputs_api() {
    let Some(engine) = load_engine() else { return };
    let Some(eop) = load_eop() else { return };
    let utc = birth_utc();
    let location = new_delhi();
    let aya_config = default_aya_config();
    let variation = DashaVariationConfig::default();

    // Compute prerequisite data
    let jd_tdb = utc.to_jd_tdb(engine.lsk());
    let graha_lons = graha_sidereal_longitudes(
        &engine,
        jd_tdb,
        aya_config.ayanamsha_system,
        aya_config.use_nutation,
    )
    .unwrap();
    let moon_sid = graha_lons.longitudes[1]; // Chandra

    // Nakshatra system (Vimshottari) — needs moon
    let birth_jd = {
        let y = utc.year as f64;
        let m = utc.month as f64;
        let d = utc.day as f64
            + utc.hour as f64 / 24.0
            + utc.minute as f64 / 1440.0
            + utc.second / 86400.0;
        let (y2, m2) = if m <= 2.0 {
            (y - 1.0, m + 12.0)
        } else {
            (y, m)
        };
        let a = (y2 / 100.0).floor();
        let b = 2.0 - a + (a / 4.0).floor();
        (365.25 * (y2 + 4716.0)).floor() + (30.6001 * (m2 + 1.0)).floor() + d + b - 1524.5
    };

    let inputs_nak = DashaInputs {
        moon_sid_lon: Some(moon_sid),
        rashi_inputs: None,
        sunrise_sunset: None,
    };
    let h_vim = dasha_hierarchy_with_inputs(
        birth_jd,
        DashaSystem::Vimshottari,
        1,
        &variation,
        &inputs_nak,
    );
    assert!(h_vim.is_ok(), "Vimshottari with inputs: {:?}", h_vim.err());
    assert_eq!(h_vim.unwrap().system, DashaSystem::Vimshottari);

    // Rashi system (Chara) — needs rashi inputs
    let lagna_rad =
        dhruv_vedic_base::lagna_longitude_rad(engine.lsk(), &eop, &location, birth_jd).unwrap();
    let t = dhruv_vedic_base::ayanamsha::jd_tdb_to_centuries(jd_tdb);
    let aya = dhruv_vedic_base::ayanamsha::ayanamsha_deg(
        aya_config.ayanamsha_system,
        t,
        aya_config.use_nutation,
    );
    let lagna_sid = dhruv_vedic_base::util::normalize_360(lagna_rad.to_degrees() - aya);
    let ri = RashiDashaInputs::new(graha_lons.longitudes, lagna_sid);

    let inputs_rashi = DashaInputs {
        moon_sid_lon: None,
        rashi_inputs: Some(&ri),
        sunrise_sunset: None,
    };
    let h_chara =
        dasha_hierarchy_with_inputs(birth_jd, DashaSystem::Chara, 1, &variation, &inputs_rashi);
    assert!(h_chara.is_ok(), "Chara with inputs: {:?}", h_chara.err());
    assert_eq!(h_chara.unwrap().system, DashaSystem::Chara);

    // Kala system — needs sunrise/sunset
    let rs_config = RiseSetConfig::default();
    let jd_midnight = birth_jd.floor() + 0.5;
    let jd_noon = dhruv_vedic_base::approximate_local_noon_jd(jd_midnight, location.longitude_deg);
    let sunrise_result = dhruv_vedic_base::riseset::compute_rise_set(
        &engine,
        engine.lsk(),
        &eop,
        &location,
        dhruv_vedic_base::riseset_types::RiseSetEvent::Sunrise,
        jd_noon,
        &rs_config,
    )
    .unwrap();
    let sunrise_jd = match sunrise_result {
        dhruv_vedic_base::riseset_types::RiseSetResult::Event { jd_tdb, .. } => jd_tdb,
        _ => panic!("expected sunrise event"),
    };
    let sunset_result = dhruv_vedic_base::riseset::compute_rise_set(
        &engine,
        engine.lsk(),
        &eop,
        &location,
        dhruv_vedic_base::riseset_types::RiseSetEvent::Sunset,
        jd_noon,
        &rs_config,
    )
    .unwrap();
    let sunset_jd = match sunset_result {
        dhruv_vedic_base::riseset_types::RiseSetResult::Event { jd_tdb, .. } => jd_tdb,
        _ => panic!("expected sunset event"),
    };
    let inputs_kala = DashaInputs {
        moon_sid_lon: None,
        rashi_inputs: None,
        sunrise_sunset: Some((sunrise_jd, sunset_jd)),
    };
    let h_kala =
        dasha_hierarchy_with_inputs(birth_jd, DashaSystem::Kala, 1, &variation, &inputs_kala);
    assert!(h_kala.is_ok(), "Kala with inputs: {:?}", h_kala.err());
    assert_eq!(h_kala.unwrap().system, DashaSystem::Kala);
}

/// Deprecated _with_moon output is bit-identical to _with_inputs.
#[test]
#[allow(deprecated)]
fn deprecated_with_moon_parity() {
    let Some(engine) = load_engine() else { return };
    let utc = birth_utc();
    let aya_config = default_aya_config();
    let variation = DashaVariationConfig::default();

    let jd_tdb = utc.to_jd_tdb(engine.lsk());
    let graha_lons = graha_sidereal_longitudes(
        &engine,
        jd_tdb,
        aya_config.ayanamsha_system,
        aya_config.use_nutation,
    )
    .unwrap();
    let moon_sid = graha_lons.longitudes[1];

    let birth_jd = {
        let y = utc.year as f64;
        let m = utc.month as f64;
        let d = utc.day as f64
            + utc.hour as f64 / 24.0
            + utc.minute as f64 / 1440.0
            + utc.second / 86400.0;
        let (y2, m2) = if m <= 2.0 {
            (y - 1.0, m + 12.0)
        } else {
            (y, m)
        };
        let a = (y2 / 100.0).floor();
        let b = 2.0 - a + (a / 4.0).floor();
        (365.25 * (y2 + 4716.0)).floor() + (30.6001 * (m2 + 1.0)).floor() + d + b - 1524.5
    };

    // Via deprecated _with_moon
    let h_old = dhruv_search::dasha_hierarchy_with_moon(
        birth_jd,
        moon_sid,
        None,
        None,
        DashaSystem::Vimshottari,
        2,
        &variation,
    )
    .unwrap();

    // Via _with_inputs
    let inputs = DashaInputs {
        moon_sid_lon: Some(moon_sid),
        rashi_inputs: None,
        sunrise_sunset: None,
    };
    let h_new =
        dasha_hierarchy_with_inputs(birth_jd, DashaSystem::Vimshottari, 2, &variation, &inputs)
            .unwrap();

    // Bit-identical comparison
    assert_eq!(h_old.levels.len(), h_new.levels.len());
    for (li, (old_level, new_level)) in h_old.levels.iter().zip(h_new.levels.iter()).enumerate() {
        assert_eq!(
            old_level.len(),
            new_level.len(),
            "level {li} count mismatch"
        );
        for (pi, (old_p, new_p)) in old_level.iter().zip(new_level.iter()).enumerate() {
            assert_eq!(
                old_p.start_jd, new_p.start_jd,
                "level {li} period {pi} start_jd mismatch"
            );
            assert_eq!(
                old_p.end_jd, new_p.end_jd,
                "level {li} period {pi} end_jd mismatch"
            );
            assert_eq!(
                old_p.entity, new_p.entity,
                "level {li} period {pi} entity mismatch"
            );
        }
    }
}

/// Helper: build a FullKundaliConfig with dasha enabled.
fn kundali_config_with_dasha(dasha_config: DashaSelectionConfig) -> FullKundaliConfig {
    FullKundaliConfig {
        include_dasha: true,
        dasha_config,
        // Minimal: disable most sections for speed
        include_graha_positions: false,
        include_bindus: false,
        include_drishti: false,
        include_ashtakavarga: false,
        include_upagrahas: false,
        include_special_lagnas: false,
        include_amshas: false,
        include_shadbala: false,
        include_vimsopaka: false,
        include_avastha: false,
        ..FullKundaliConfig::default()
    }
}

/// FullKundali with single dasha system, no snapshot.
#[test]
fn full_kundali_with_dasha() {
    let Some(engine) = load_engine() else { return };
    let Some(eop) = load_eop() else { return };
    let utc = birth_utc();
    let location = new_delhi();
    let bhava_config = BhavaConfig::default();
    let rs_config = RiseSetConfig::default();
    let aya_config = default_aya_config();

    let mut dasha_config = DashaSelectionConfig::default();
    dasha_config.count = 1;
    dasha_config.systems[0] = DashaSystem::Vimshottari as u8;
    let config = kundali_config_with_dasha(dasha_config);

    let result = full_kundali_for_date(
        &engine,
        &eop,
        &utc,
        &location,
        &bhava_config,
        &rs_config,
        &aya_config,
        &config,
    );
    assert!(result.is_ok(), "full kundali: {:?}", result.err());
    let r = result.unwrap();
    assert!(r.dasha.is_some());
    assert_eq!(r.dasha.as_ref().unwrap().len(), 1);
    assert_eq!(
        r.dasha.as_ref().unwrap()[0].system,
        DashaSystem::Vimshottari
    );
    assert!(r.dasha_snapshots.is_none()); // no snapshot_jd
}

/// FullKundali with snapshot_jd.
#[test]
fn full_kundali_dasha_snapshot() {
    let Some(engine) = load_engine() else { return };
    let Some(eop) = load_eop() else { return };
    let utc = birth_utc();
    let location = new_delhi();
    let bhava_config = BhavaConfig::default();
    let rs_config = RiseSetConfig::default();
    let aya_config = default_aya_config();

    let mut dasha_config = DashaSelectionConfig::default();
    dasha_config.count = 1;
    dasha_config.systems[0] = DashaSystem::Vimshottari as u8;
    // Set snapshot query to a date well within the dasha range
    dasha_config.snapshot_jd = Some(2460000.0); // ~2023
    let config = kundali_config_with_dasha(dasha_config);

    let r = full_kundali_for_date(
        &engine,
        &eop,
        &utc,
        &location,
        &bhava_config,
        &rs_config,
        &aya_config,
        &config,
    )
    .unwrap();
    assert!(r.dasha.is_some());
    assert_eq!(r.dasha.as_ref().unwrap().len(), 1);
    assert!(r.dasha_snapshots.is_some());
    assert_eq!(r.dasha_snapshots.as_ref().unwrap().len(), 1);
}

/// FullKundali with 3 systems, ordering preserved.
#[test]
fn full_kundali_dasha_multiple_systems() {
    let Some(engine) = load_engine() else { return };
    let Some(eop) = load_eop() else { return };
    let utc = birth_utc();
    let location = new_delhi();
    let bhava_config = BhavaConfig::default();
    let rs_config = RiseSetConfig::default();
    let aya_config = default_aya_config();

    let mut dasha_config = DashaSelectionConfig::default();
    dasha_config.count = 3;
    dasha_config.systems[0] = DashaSystem::Vimshottari as u8;
    dasha_config.systems[1] = DashaSystem::Chara as u8;
    dasha_config.systems[2] = DashaSystem::Kala as u8;
    let config = kundali_config_with_dasha(dasha_config);

    let r = full_kundali_for_date(
        &engine,
        &eop,
        &utc,
        &location,
        &bhava_config,
        &rs_config,
        &aya_config,
        &config,
    )
    .unwrap();
    let dashas = r.dasha.unwrap();
    assert_eq!(dashas.len(), 3);
    assert_eq!(dashas[0].system, DashaSystem::Vimshottari);
    assert_eq!(dashas[1].system, DashaSystem::Chara);
    assert_eq!(dashas[2].system, DashaSystem::Kala);
}

/// Partial failure: Kala fails at polar location, Vimshottari succeeds.
#[test]
fn full_kundali_dasha_fallback_partial() {
    let Some(engine) = load_engine() else { return };
    let Some(eop) = load_eop() else { return };
    // Polar location where sunrise may fail during polar night
    let polar = GeoLocation::new(89.0, 0.0, 0.0);
    // Use a date during polar night (Dec solstice)
    let utc = UtcTime::new(1990, 12, 21, 12, 0, 0.0);
    let bhava_config = BhavaConfig::default();
    let rs_config = RiseSetConfig::default();
    let aya_config = default_aya_config();

    let mut dasha_config = DashaSelectionConfig::default();
    dasha_config.count = 2;
    dasha_config.systems[0] = DashaSystem::Vimshottari as u8;
    dasha_config.systems[1] = DashaSystem::Kala as u8; // needs sunrise — may fail
    let config = kundali_config_with_dasha(dasha_config);

    let r = full_kundali_for_date(
        &engine,
        &eop,
        &utc,
        &polar,
        &bhava_config,
        &rs_config,
        &aya_config,
        &config,
    )
    .unwrap();
    // Vimshottari should succeed; Kala may fail at polar location
    assert!(r.dasha.is_some());
    let dashas = r.dasha.unwrap();
    assert!(dashas.len() >= 1);
    assert_eq!(dashas[0].system, DashaSystem::Vimshottari);
}

/// All fail → dasha is None.
#[test]
fn full_kundali_dasha_fallback_all_fail() {
    let Some(engine) = load_engine() else { return };
    let Some(eop) = load_eop() else { return };
    // Polar location during polar night
    let polar = GeoLocation::new(89.0, 0.0, 0.0);
    let utc = UtcTime::new(1990, 12, 21, 12, 0, 0.0);
    let bhava_config = BhavaConfig::default();
    let rs_config = RiseSetConfig::default();
    let aya_config = default_aya_config();

    let mut dasha_config = DashaSelectionConfig::default();
    dasha_config.count = 1;
    dasha_config.systems[0] = DashaSystem::Kala as u8; // needs sunrise — fails at pole
    let config = kundali_config_with_dasha(dasha_config);

    let r = full_kundali_for_date(
        &engine,
        &eop,
        &utc,
        &polar,
        &bhava_config,
        &rs_config,
        &aya_config,
        &config,
    )
    .unwrap();
    assert!(r.dasha.is_none());
}

/// include_dasha: false → dasha is None.
#[test]
fn full_kundali_dasha_disabled() {
    let Some(engine) = load_engine() else { return };
    let Some(eop) = load_eop() else { return };
    let utc = birth_utc();
    let location = new_delhi();
    let bhava_config = BhavaConfig::default();
    let rs_config = RiseSetConfig::default();
    let aya_config = default_aya_config();

    let config = FullKundaliConfig {
        include_dasha: false,
        ..FullKundaliConfig::default()
    };
    let r = full_kundali_for_date(
        &engine,
        &eop,
        &utc,
        &location,
        &bhava_config,
        &rs_config,
        &aya_config,
        &config,
    )
    .unwrap();
    assert!(r.dasha.is_none());
    assert!(r.dasha_snapshots.is_none());
}

/// count: 0 → dasha is None.
#[test]
fn full_kundali_dasha_zero_count() {
    let Some(engine) = load_engine() else { return };
    let Some(eop) = load_eop() else { return };
    let utc = birth_utc();
    let location = new_delhi();
    let bhava_config = BhavaConfig::default();
    let rs_config = RiseSetConfig::default();
    let aya_config = default_aya_config();

    let config = FullKundaliConfig {
        include_dasha: true,
        dasha_config: DashaSelectionConfig::default(), // count = 0
        ..FullKundaliConfig::default()
    };
    let r = full_kundali_for_date(
        &engine,
        &eop,
        &utc,
        &location,
        &bhava_config,
        &rs_config,
        &aya_config,
        &config,
    )
    .unwrap();
    assert!(r.dasha.is_none());
    assert!(r.dasha_snapshots.is_none());
}

/// Ordering preserved: [Vimshottari, Kala-fail, Chara] → [Vimshottari, Chara].
#[test]
fn fallback_ordering_preserved() {
    let Some(engine) = load_engine() else { return };
    let Some(eop) = load_eop() else { return };
    // Polar location during polar night — Kala will fail
    let polar = GeoLocation::new(89.0, 0.0, 0.0);
    let utc = UtcTime::new(1990, 12, 21, 12, 0, 0.0);
    let bhava_config = BhavaConfig::default();
    let rs_config = RiseSetConfig::default();
    let aya_config = default_aya_config();

    let mut dasha_config = DashaSelectionConfig::default();
    dasha_config.count = 3;
    dasha_config.systems[0] = DashaSystem::Vimshottari as u8;
    dasha_config.systems[1] = DashaSystem::Kala as u8; // fails — no sunrise
    dasha_config.systems[2] = DashaSystem::Chara as u8;
    let config = kundali_config_with_dasha(dasha_config);

    let r = full_kundali_for_date(
        &engine,
        &eop,
        &utc,
        &polar,
        &bhava_config,
        &rs_config,
        &aya_config,
        &config,
    )
    .unwrap();
    let dashas = r.dasha.unwrap();
    // Kala skipped, Vimshottari and Chara survive in order
    assert_eq!(dashas.len(), 2);
    assert_eq!(dashas[0].system, DashaSystem::Vimshottari);
    assert_eq!(dashas[1].system, DashaSystem::Chara);
}

/// Cross-validate Moon sidereal longitude: graha_lons[1] vs moon_sidereal_longitude_at.
#[test]
fn moon_lon_graha_vs_standalone() {
    let Some(engine) = load_engine() else { return };
    let utc = birth_utc();
    let aya_config = default_aya_config();
    let jd_tdb = utc.to_jd_tdb(engine.lsk());

    let graha_lons = graha_sidereal_longitudes(
        &engine,
        jd_tdb,
        aya_config.ayanamsha_system,
        aya_config.use_nutation,
    )
    .unwrap();
    let moon_from_graha = graha_lons.longitudes[1];

    let moon_standalone = dhruv_search::moon_sidereal_longitude_at(&engine, jd_tdb, &aya_config)
        .expect("moon lon should succeed");

    let delta = (moon_from_graha - moon_standalone).abs();
    assert!(
        delta < 1e-10,
        "Moon sidereal lon mismatch: graha={moon_from_graha}, standalone={moon_standalone}, delta={delta}"
    );
}

/// Cross-validate RashiDashaInputs built two ways.
#[test]
fn rashi_inputs_from_ctx_vs_standalone() {
    let Some(engine) = load_engine() else { return };
    let Some(eop) = load_eop() else { return };
    let utc = birth_utc();
    let location = new_delhi();
    let aya_config = default_aya_config();

    let jd_tdb = utc.to_jd_tdb(engine.lsk());

    // Method 1: same as what _with_ctx uses (graha_sidereal_longitudes + lagna_sid)
    let graha_lons = graha_sidereal_longitudes(
        &engine,
        jd_tdb,
        aya_config.ayanamsha_system,
        aya_config.use_nutation,
    )
    .unwrap();

    let birth_jd = {
        let y = utc.year as f64;
        let m = utc.month as f64;
        let d = utc.day as f64
            + utc.hour as f64 / 24.0
            + utc.minute as f64 / 1440.0
            + utc.second / 86400.0;
        let (y2, m2) = if m <= 2.0 {
            (y - 1.0, m + 12.0)
        } else {
            (y, m)
        };
        let a = (y2 / 100.0).floor();
        let b = 2.0 - a + (a / 4.0).floor();
        (365.25 * (y2 + 4716.0)).floor() + (30.6001 * (m2 + 1.0)).floor() + d + b - 1524.5
    };

    let lagna_rad =
        dhruv_vedic_base::lagna_longitude_rad(engine.lsk(), &eop, &location, birth_jd).unwrap();
    let t = dhruv_vedic_base::ayanamsha::jd_tdb_to_centuries(jd_tdb);
    let aya = dhruv_vedic_base::ayanamsha::ayanamsha_deg(
        aya_config.ayanamsha_system,
        t,
        aya_config.use_nutation,
    );
    let lagna_sid_1 = dhruv_vedic_base::util::normalize_360(lagna_rad.to_degrees() - aya);
    let ri_1 = RashiDashaInputs::new(graha_lons.longitudes, lagna_sid_1);

    // Method 2: use both methods produce same hierarchy output (structural validation)
    let inputs = DashaInputs {
        moon_sid_lon: None,
        rashi_inputs: Some(&ri_1),
        sunrise_sunset: None,
    };
    let h1 = dasha_hierarchy_with_inputs(
        birth_jd,
        DashaSystem::Chara,
        1,
        &DashaVariationConfig::default(),
        &inputs,
    )
    .unwrap();

    let h2 = dasha_hierarchy_for_birth(
        &engine,
        &eop,
        &utc,
        &location,
        DashaSystem::Chara,
        1,
        &BhavaConfig::default(),
        &RiseSetConfig::default(),
        &aya_config,
        &DashaVariationConfig::default(),
    )
    .unwrap();

    // Compare level 0 periods
    assert_eq!(h1.levels[0].len(), h2.levels[0].len());
    for (i, (p1, p2)) in h1.levels[0].iter().zip(h2.levels[0].iter()).enumerate() {
        assert!(
            (p1.start_jd - p2.start_jd).abs() < 1e-10,
            "period {i} start_jd mismatch: {:.12} vs {:.12}",
            p1.start_jd,
            p2.start_jd,
        );
        assert!(
            (p1.end_jd - p2.end_jd).abs() < 1e-10,
            "period {i} end_jd mismatch: {:.12} vs {:.12}",
            p1.end_jd,
            p2.end_jd,
        );
        assert_eq!(p1.entity, p2.entity, "period {i} entity mismatch");
    }
}

/// Non-Chakra parity: Vimshottari + Chara golden values match pre-change output.
#[test]
fn non_chakra_parity_golden_values() {
    let Some(engine) = load_engine() else { return };
    let Some(eop) = load_eop() else { return };
    let utc = birth_utc();
    let location = new_delhi();
    let bhava_config = BhavaConfig::default();
    let rs_config = RiseSetConfig::default();
    let aya_config = default_aya_config();
    let variation = DashaVariationConfig::default();

    // Golden values captured from unmodified codebase:
    let vim = dasha_hierarchy_for_birth(
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
    )
    .unwrap();
    assert_eq!(
        vim.levels[0][0].entity,
        DashaEntity::Graha(dhruv_vedic_base::Graha::Shukra)
    );
    assert!(
        (vim.levels[0][0].start_jd - 2447906.770833).abs() < 1e-6,
        "Vimshottari start_jd: {:.10}",
        vim.levels[0][0].start_jd
    );
    assert!(
        (vim.levels[0][0].end_jd - 2450982.473393).abs() < 1e-6,
        "Vimshottari end_jd: {:.10}",
        vim.levels[0][0].end_jd
    );

    let chara = dasha_hierarchy_for_birth(
        &engine,
        &eop,
        &utc,
        &location,
        DashaSystem::Chara,
        1,
        &bhava_config,
        &rs_config,
        &aya_config,
        &variation,
    )
    .unwrap();
    assert_eq!(chara.levels[0][0].entity, DashaEntity::Rashi(0));
    assert!(
        (chara.levels[0][0].start_jd - 2447906.770833).abs() < 1e-6,
        "Chara start_jd: {:.10}",
        chara.levels[0][0].start_jd
    );
    assert!(
        (chara.levels[0][0].end_jd - 2450200.268691).abs() < 1e-6,
        "Chara end_jd: {:.10}",
        chara.levels[0][0].end_jd
    );
}
