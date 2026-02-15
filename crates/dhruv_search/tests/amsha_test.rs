//! Integration tests for amsha (divisional chart) orchestration.
//!
//! Requires kernel files. Skips gracefully if absent.

use std::path::Path;

use dhruv_core::{Engine, EngineConfig};
use dhruv_search::sankranti_types::SankrantiConfig;
use dhruv_search::{
    AmshaChartScope, AmshaSelectionConfig, FullKundaliConfig, GrahaPositionsConfig,
    amsha_charts_for_date, amsha_charts_from_kundali, full_kundali_for_date,
};
use dhruv_vedic_base::riseset_types::{GeoLocation, RiseSetConfig};
use dhruv_vedic_base::{Amsha, AmshaRequest, AmshaVariation, BhavaConfig};

use dhruv_time::{EopKernel, UtcTime};

const SPK_PATH: &str = "../../kernels/data/de442s.bsp";
const LSK_PATH: &str = "../../kernels/data/naif0012.tls";
const EOP_PATH: &str = "../../kernels/data/finals2000A.all";

fn load_engine() -> Option<Engine> {
    if !Path::new(SPK_PATH).exists() || !Path::new(LSK_PATH).exists() {
        eprintln!("Skipping amsha_test: kernel files not found");
        return None;
    }
    let config = EngineConfig::with_single_spk(SPK_PATH.into(), LSK_PATH.into(), 1024, false);
    Engine::new(config).ok()
}

fn load_eop() -> Option<EopKernel> {
    if !Path::new(EOP_PATH).exists() {
        eprintln!("Skipping amsha_test: EOP file not found");
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
fn amsha_charts_basic_d9() {
    let Some(engine) = load_engine() else { return };
    let Some(eop) = load_eop() else { return };
    let utc = utc_2024_jan_15();
    let location = new_delhi();
    let bhava_config = BhavaConfig::default();
    let rs_config = RiseSetConfig::default();
    let aya_config = default_aya_config();

    let requests = [AmshaRequest::new(Amsha::D9)];
    let scope = AmshaChartScope::default();

    let result = amsha_charts_for_date(
        &engine,
        &eop,
        &utc,
        &location,
        &bhava_config,
        &rs_config,
        &aya_config,
        &requests,
        &scope,
    )
    .expect("amsha_charts_for_date should succeed");

    assert_eq!(result.charts.len(), 1);
    let chart = &result.charts[0];
    assert_eq!(chart.amsha, Amsha::D9);
    assert_eq!(chart.variation, AmshaVariation::TraditionalParashari);

    // All 9 grahas should have valid longitudes
    for entry in &chart.grahas {
        assert!(
            entry.sidereal_longitude >= 0.0 && entry.sidereal_longitude < 360.0,
            "graha lon out of range: {}",
            entry.sidereal_longitude
        );
        assert!(entry.rashi_index < 12);
    }

    // Lagna should also be valid
    assert!(chart.lagna.sidereal_longitude >= 0.0 && chart.lagna.sidereal_longitude < 360.0);

    // No optional sections
    assert!(chart.bhava_cusps.is_none());
    assert!(chart.arudha_padas.is_none());
    assert!(chart.upagrahas.is_none());
    assert!(chart.sphutas.is_none());
    assert!(chart.special_lagnas.is_none());
}

#[test]
fn amsha_charts_multiple() {
    let Some(engine) = load_engine() else { return };
    let Some(eop) = load_eop() else { return };
    let utc = utc_2024_jan_15();
    let location = new_delhi();
    let bhava_config = BhavaConfig::default();
    let rs_config = RiseSetConfig::default();
    let aya_config = default_aya_config();

    let requests = [
        AmshaRequest::new(Amsha::D1),
        AmshaRequest::new(Amsha::D9),
        AmshaRequest::new(Amsha::D10),
    ];
    let scope = AmshaChartScope::default();

    let result = amsha_charts_for_date(
        &engine,
        &eop,
        &utc,
        &location,
        &bhava_config,
        &rs_config,
        &aya_config,
        &requests,
        &scope,
    )
    .expect("amsha_charts_for_date should succeed");

    assert_eq!(result.charts.len(), 3);
    assert_eq!(result.charts[0].amsha, Amsha::D1);
    assert_eq!(result.charts[1].amsha, Amsha::D9);
    assert_eq!(result.charts[2].amsha, Amsha::D10);
}

#[test]
fn amsha_d1_matches_graha_positions() {
    let Some(engine) = load_engine() else { return };
    let Some(eop) = load_eop() else { return };
    let utc = utc_2024_jan_15();
    let location = new_delhi();
    let bhava_config = BhavaConfig::default();
    let rs_config = RiseSetConfig::default();
    let aya_config = default_aya_config();

    // Compute D1 amsha chart
    let requests = [AmshaRequest::new(Amsha::D1)];
    let scope = AmshaChartScope::default();
    let amsha_result = amsha_charts_for_date(
        &engine,
        &eop,
        &utc,
        &location,
        &bhava_config,
        &rs_config,
        &aya_config,
        &requests,
        &scope,
    )
    .expect("amsha_charts should succeed");

    // Compute graha positions directly
    let gp_config = GrahaPositionsConfig {
        include_lagna: true,
        ..Default::default()
    };
    let gp = dhruv_search::graha_positions(
        &engine,
        &eop,
        &utc,
        &location,
        &bhava_config,
        &aya_config,
        &gp_config,
    )
    .expect("graha_positions should succeed");

    // D1 should match raw positions
    let d1 = &amsha_result.charts[0];
    for i in 0..9 {
        assert!(
            (d1.grahas[i].sidereal_longitude - gp.grahas[i].sidereal_longitude).abs() < 0.001,
            "D1 graha {} mismatch: {} vs {}",
            i,
            d1.grahas[i].sidereal_longitude,
            gp.grahas[i].sidereal_longitude
        );
    }
    assert!(
        (d1.lagna.sidereal_longitude - gp.lagna.sidereal_longitude).abs() < 0.001,
        "D1 lagna mismatch"
    );
}

#[test]
fn amsha_with_scope_flags() {
    let Some(engine) = load_engine() else { return };
    let Some(eop) = load_eop() else { return };
    let utc = utc_2024_jan_15();
    let location = new_delhi();
    let bhava_config = BhavaConfig::default();
    let rs_config = RiseSetConfig::default();
    let aya_config = default_aya_config();

    let requests = [AmshaRequest::new(Amsha::D9)];
    let scope = AmshaChartScope {
        include_bhava_cusps: true,
        include_arudha_padas: true,
        include_upagrahas: true,
        include_sphutas: true,
        include_special_lagnas: true,
    };

    let result = amsha_charts_for_date(
        &engine,
        &eop,
        &utc,
        &location,
        &bhava_config,
        &rs_config,
        &aya_config,
        &requests,
        &scope,
    )
    .expect("amsha_charts with full scope should succeed");

    let chart = &result.charts[0];
    assert!(chart.bhava_cusps.is_some());
    assert!(chart.arudha_padas.is_some());
    assert!(chart.upagrahas.is_some());
    assert!(chart.sphutas.is_some());
    assert!(chart.special_lagnas.is_some());

    let cusps = chart.bhava_cusps.as_ref().unwrap();
    for c in cusps {
        assert!(c.sidereal_longitude >= 0.0 && c.sidereal_longitude < 360.0);
    }
}

#[test]
fn amsha_from_kundali_matches_direct() {
    let Some(engine) = load_engine() else { return };
    let Some(eop) = load_eop() else { return };
    let utc = utc_2024_jan_15();
    let location = new_delhi();
    let bhava_config = BhavaConfig::default();
    let rs_config = RiseSetConfig::default();
    let aya_config = default_aya_config();

    let requests = [AmshaRequest::new(Amsha::D9)];
    let scope = AmshaChartScope::default();

    // Compute via direct function
    let direct = amsha_charts_for_date(
        &engine,
        &eop,
        &utc,
        &location,
        &bhava_config,
        &rs_config,
        &aya_config,
        &requests,
        &scope,
    )
    .expect("direct should succeed");

    // Compute via full kundali
    let kundali_config = FullKundaliConfig {
        include_graha_positions: true,
        graha_positions_config: GrahaPositionsConfig {
            include_lagna: true,
            ..Default::default()
        },
        ..Default::default()
    };
    let kundali = full_kundali_for_date(
        &engine,
        &eop,
        &utc,
        &location,
        &bhava_config,
        &rs_config,
        &aya_config,
        &kundali_config,
    )
    .expect("kundali should succeed");

    let from_kundali =
        amsha_charts_from_kundali(&kundali, None, &requests, &scope).expect("from_kundali");

    // Should match
    for i in 0..9 {
        assert!(
            (direct.charts[0].grahas[i].sidereal_longitude
                - from_kundali.charts[0].grahas[i].sidereal_longitude)
                .abs()
                < 0.001,
            "graha {} mismatch",
            i
        );
    }
}

#[test]
fn full_kundali_with_amshas() {
    let Some(engine) = load_engine() else { return };
    let Some(eop) = load_eop() else { return };
    let utc = utc_2024_jan_15();
    let location = new_delhi();
    let bhava_config = BhavaConfig::default();
    let rs_config = RiseSetConfig::default();
    let aya_config = default_aya_config();

    let mut sel = AmshaSelectionConfig::default();
    sel.count = 3;
    sel.codes[0] = 9; // D9
    sel.codes[1] = 10; // D10
    sel.codes[2] = 12; // D12

    let config = FullKundaliConfig {
        include_graha_positions: true,
        include_amshas: true,
        graha_positions_config: GrahaPositionsConfig {
            include_lagna: true,
            ..Default::default()
        },
        amsha_selection: sel,
        ..Default::default()
    };

    let result = full_kundali_for_date(
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

    assert!(result.amshas.is_some());
    let amshas = result.amshas.unwrap();
    assert_eq!(amshas.charts.len(), 3);
    assert_eq!(amshas.charts[0].amsha, Amsha::D9);
    assert_eq!(amshas.charts[1].amsha, Amsha::D10);
    assert_eq!(amshas.charts[2].amsha, Amsha::D12);
}

#[test]
fn validation_inapplicable_variation() {
    let Some(engine) = load_engine() else { return };
    let Some(eop) = load_eop() else { return };
    let utc = utc_2024_jan_15();
    let location = new_delhi();
    let bhava_config = BhavaConfig::default();
    let rs_config = RiseSetConfig::default();
    let aya_config = default_aya_config();

    // HoraCancerLeoOnly on D9 should fail
    let requests = [AmshaRequest::with_variation(
        Amsha::D9,
        AmshaVariation::HoraCancerLeoOnly,
    )];
    let scope = AmshaChartScope::default();

    let result = amsha_charts_for_date(
        &engine,
        &eop,
        &utc,
        &location,
        &bhava_config,
        &rs_config,
        &aya_config,
        &requests,
        &scope,
    );
    assert!(result.is_err());
}

#[test]
fn validation_unknown_amsha_code() {
    let Some(engine) = load_engine() else { return };
    let Some(eop) = load_eop() else { return };
    let utc = utc_2024_jan_15();
    let location = new_delhi();
    let bhava_config = BhavaConfig::default();
    let rs_config = RiseSetConfig::default();
    let aya_config = default_aya_config();

    let mut sel = AmshaSelectionConfig::default();
    sel.count = 1;
    sel.codes[0] = 999; // invalid

    let config = FullKundaliConfig {
        include_graha_positions: true,
        include_amshas: true,
        graha_positions_config: GrahaPositionsConfig {
            include_lagna: true,
            ..Default::default()
        },
        amsha_selection: sel,
        ..Default::default()
    };

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
    assert!(result.is_err());
}
