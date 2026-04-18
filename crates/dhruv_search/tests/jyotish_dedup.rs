//! Integration tests for in-call deduplication refactors.
//!
//! These validate public result stability for direct amsha charts, single-item
//! endpoints, and mixed full-kundali flows. Requires kernel files and skips
//! gracefully when they are absent.

use std::path::Path;

use dhruv_core::{Engine, EngineConfig};
use dhruv_search::sankranti_types::SankrantiConfig;
use dhruv_search::{
    AmshaChartScope, AmshaSelectionConfig, FullKundaliConfig, amsha_charts_for_date,
    avastha_for_date, avastha_for_graha, bhavabala_for_bhava, bhavabala_for_date,
    full_kundali_for_date, panchang_for_date, shadbala_for_date, shadbala_for_graha,
    vimsopaka_for_date, vimsopaka_for_graha,
};
use dhruv_time::{EopKernel, UtcTime};
use dhruv_vedic_base::riseset_types::{GeoLocation, RiseSetConfig};
use dhruv_vedic_base::{
    Amsha, AmshaRequest, BhavaConfig, D2_CANCER_LEO_ONLY_VARIATION_CODE,
    Graha, NodeDignityPolicy,
};

const SPK_PATH: &str = "../../kernels/data/de442s.bsp";
const LSK_PATH: &str = "../../kernels/data/naif0012.tls";
const EOP_PATH: &str = "../../kernels/data/finals2000A.all";
const EPS: f64 = 1e-9;

fn load_engine() -> Option<Engine> {
    if !Path::new(SPK_PATH).exists() || !Path::new(LSK_PATH).exists() {
        eprintln!("Skipping jyotish_dedup: kernel files not found");
        return None;
    }
    let config = EngineConfig::with_single_spk(SPK_PATH.into(), LSK_PATH.into(), 1024, false);
    Engine::new(config).ok()
}

fn load_eop() -> Option<EopKernel> {
    if !Path::new(EOP_PATH).exists() {
        eprintln!("Skipping jyotish_dedup: EOP file not found");
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

fn approx_eq(left: f64, right: f64) -> bool {
    (left - right).abs() <= EPS
}

fn default_amsha_selection() -> AmshaSelectionConfig {
    let mut selection = AmshaSelectionConfig {
        count: 1,
        ..AmshaSelectionConfig::default()
    };
    selection.codes[0] = Amsha::D2.code();
    selection.variations[0] = 1;
    selection
}

#[test]
fn amsha_charts_duplicate_requests_preserve_order_and_distinguish_variation() {
    let Some(engine) = load_engine() else { return };
    let Some(eop) = load_eop() else { return };
    let utc = utc_2024_jan_15();
    let location = new_delhi();
    let bhava_config = BhavaConfig::default();
    let riseset_config = RiseSetConfig::default();
    let aya_config = default_aya_config();
    let scope = AmshaChartScope {
        include_bhava_cusps: true,
        include_arudha_padas: true,
        include_upagrahas: true,
        include_sphutas: true,
        include_special_lagnas: true,
    };
    let requests = [
        AmshaRequest::new(Amsha::D9),
        AmshaRequest::new(Amsha::D9),
        AmshaRequest::new(Amsha::D2),
        AmshaRequest::with_variation(Amsha::D2, D2_CANCER_LEO_ONLY_VARIATION_CODE),
    ];

    let result = amsha_charts_for_date(
        &engine,
        &eop,
        &utc,
        &location,
        &bhava_config,
        &riseset_config,
        &aya_config,
        &requests,
        &scope,
    )
    .expect("amsha_charts_for_date should succeed");

    assert_eq!(result.charts.len(), 4);
    assert_eq!(result.charts[0].variation_code, result.charts[1].variation_code);
    for i in 0..9 {
        assert!(approx_eq(
            result.charts[0].grahas[i].sidereal_longitude,
            result.charts[1].grahas[i].sidereal_longitude,
        ));
        assert_eq!(
            result.charts[0].grahas[i].rashi_index,
            result.charts[1].grahas[i].rashi_index
        );
    }

    assert_eq!(result.charts[2].amsha, Amsha::D2);
    assert_eq!(result.charts[3].amsha, Amsha::D2);
    assert_ne!(result.charts[2].variation_code, result.charts[3].variation_code);
    assert!(
        result.charts[2]
            .grahas
            .iter()
            .zip(result.charts[3].grahas.iter())
            .any(|(left, right)| !approx_eq(left.sidereal_longitude, right.sidereal_longitude))
    );
}

#[test]
fn single_item_endpoints_match_full_results() {
    let Some(engine) = load_engine() else { return };
    let Some(eop) = load_eop() else { return };
    let utc = utc_2024_jan_15();
    let location = new_delhi();
    let bhava_config = BhavaConfig::default();
    let riseset_config = RiseSetConfig::default();
    let aya_config = default_aya_config();
    let selection = default_amsha_selection();

    let shadbala = shadbala_for_date(
        &engine,
        &eop,
        &utc,
        &location,
        &bhava_config,
        &riseset_config,
        &aya_config,
        &selection,
    )
    .expect("shadbala_for_date should succeed");
    let shadbala_single = shadbala_for_graha(
        &engine,
        &eop,
        &utc,
        &location,
        &bhava_config,
        &riseset_config,
        &aya_config,
        &selection,
        Graha::Guru,
    )
    .expect("shadbala_for_graha should succeed");
    assert_eq!(shadbala_single.graha, Graha::Guru);
    assert!(approx_eq(
        shadbala_single.total_shashtiamsas,
        shadbala.entries[Graha::Guru.index() as usize].total_shashtiamsas,
    ));

    let vimsopaka = vimsopaka_for_date(
        &engine,
        &eop,
        &utc,
        &location,
        &aya_config,
        NodeDignityPolicy::SignLordBased,
        &selection,
    )
    .expect("vimsopaka_for_date should succeed");
    let vimsopaka_single = vimsopaka_for_graha(
        &engine,
        &eop,
        &utc,
        &location,
        &aya_config,
        NodeDignityPolicy::SignLordBased,
        &selection,
        Graha::Rahu,
    )
    .expect("vimsopaka_for_graha should succeed");
    let full_vim = vimsopaka.entries[Graha::Rahu.index() as usize];
    assert!(approx_eq(vimsopaka_single.shadvarga, full_vim.shadvarga));
    assert!(approx_eq(vimsopaka_single.saptavarga, full_vim.saptavarga));
    assert!(approx_eq(vimsopaka_single.dashavarga, full_vim.dashavarga));
    assert!(approx_eq(
        vimsopaka_single.shodasavarga,
        full_vim.shodasavarga
    ));

    let avastha = avastha_for_date(
        &engine,
        &eop,
        &location,
        &utc,
        &bhava_config,
        &riseset_config,
        &aya_config,
        NodeDignityPolicy::SignLordBased,
        &selection,
    )
    .expect("avastha_for_date should succeed");
    let avastha_single = avastha_for_graha(
        &engine,
        &eop,
        &location,
        &utc,
        &bhava_config,
        &riseset_config,
        &aya_config,
        NodeDignityPolicy::SignLordBased,
        &selection,
        Graha::Shukra,
    )
    .expect("avastha_for_graha should succeed");
    let full_avastha = avastha.entries[Graha::Shukra.index() as usize];
    assert_eq!(avastha_single.baladi, full_avastha.baladi);
    assert_eq!(avastha_single.jagradadi, full_avastha.jagradadi);
    assert_eq!(avastha_single.deeptadi, full_avastha.deeptadi);
    assert_eq!(avastha_single.lajjitadi, full_avastha.lajjitadi);
    assert_eq!(
        avastha_single.sayanadi.avastha,
        full_avastha.sayanadi.avastha
    );
    assert_eq!(
        avastha_single.sayanadi.sub_states,
        full_avastha.sayanadi.sub_states
    );

    let bhavabala = bhavabala_for_date(
        &engine,
        &eop,
        &utc,
        &location,
        &bhava_config,
        &riseset_config,
        &aya_config,
    )
    .expect("bhavabala_for_date should succeed");
    let bhavabala_single = bhavabala_for_bhava(
        &engine,
        &eop,
        &utc,
        &location,
        &bhava_config,
        &riseset_config,
        &aya_config,
        5,
    )
    .expect("bhavabala_for_bhava should succeed");
    assert_eq!(bhavabala_single, bhavabala.entries[4]);
}

#[test]
fn full_kundali_mixed_sections_match_standalone_results() {
    let Some(engine) = load_engine() else { return };
    let Some(eop) = load_eop() else { return };
    let utc = utc_2024_jan_15();
    let location = new_delhi();
    let bhava_config = BhavaConfig::default();
    let riseset_config = RiseSetConfig::default();
    let aya_config = default_aya_config();
    let selection = default_amsha_selection();

    let mut config = FullKundaliConfig::default();
    config.include_amshas = true;
    config.include_shadbala = true;
    config.include_bhavabala = true;
    config.include_vimsopaka = true;
    config.include_avastha = true;
    config.include_panchang = true;
    config.include_calendar = true;
    config.amsha_scope = AmshaChartScope {
        include_bhava_cusps: true,
        include_arudha_padas: true,
        include_upagrahas: true,
        include_sphutas: true,
        include_special_lagnas: true,
    };
    config.amsha_selection = selection;

    let kundali = full_kundali_for_date(
        &engine,
        &eop,
        &utc,
        &location,
        &bhava_config,
        &riseset_config,
        &aya_config,
        &config,
    )
    .expect("full_kundali_for_date should succeed");

    let standalone_shadbala = shadbala_for_date(
        &engine,
        &eop,
        &utc,
        &location,
        &bhava_config,
        &riseset_config,
        &aya_config,
        &selection,
    )
    .expect("standalone shadbala should succeed");
    let standalone_vimsopaka = vimsopaka_for_date(
        &engine,
        &eop,
        &utc,
        &location,
        &aya_config,
        config.node_dignity_policy,
        &selection,
    )
    .expect("standalone vimsopaka should succeed");
    let standalone_avastha = avastha_for_date(
        &engine,
        &eop,
        &location,
        &utc,
        &bhava_config,
        &riseset_config,
        &aya_config,
        config.node_dignity_policy,
        &selection,
    )
    .expect("standalone avastha should succeed");
    let standalone_panchang = panchang_for_date(
        &engine,
        &eop,
        &utc,
        &location,
        &riseset_config,
        &aya_config,
        true,
    )
    .expect("standalone panchang should succeed");

    let kundali_shadbala = kundali.shadbala.expect("kundali shadbala present");
    let kundali_bhavabala = kundali.bhavabala.expect("kundali bhavabala present");
    let kundali_vimsopaka = kundali.vimsopaka.expect("kundali vimsopaka present");
    let kundali_avastha = kundali.avastha.expect("kundali avastha present");
    let kundali_panchang = kundali.panchang.expect("kundali panchang present");
    let kundali_amshas = kundali.amshas.expect("kundali amshas present");

    assert!(approx_eq(
        kundali_shadbala.entries[Graha::Guru.index() as usize].total_shashtiamsas,
        standalone_shadbala.entries[Graha::Guru.index() as usize].total_shashtiamsas,
    ));
    assert_eq!(kundali_bhavabala.entries[4].bhava_number, 5);
    assert!(approx_eq(
        kundali_vimsopaka.entries[Graha::Rahu.index() as usize].shodasavarga,
        standalone_vimsopaka.entries[Graha::Rahu.index() as usize].shodasavarga,
    ));
    assert_eq!(
        kundali_avastha.entries[Graha::Shukra.index() as usize]
            .sayanadi
            .avastha,
        standalone_avastha.entries[Graha::Shukra.index() as usize]
            .sayanadi
            .avastha,
    );
    assert_eq!(kundali_panchang.masa, standalone_panchang.masa);
    assert_eq!(kundali_panchang.varsha, standalone_panchang.varsha);
    assert!(
        kundali_amshas
            .charts
            .iter()
            .any(|chart| chart.amsha == Amsha::D2
                && chart.variation_code == D2_CANCER_LEO_ONLY_VARIATION_CODE)
    );
}
