//! Integration tests for Graha Avastha orchestration.
//!
//! Requires kernel files. Skips gracefully if absent.

use std::path::Path;

use dhruv_core::{Engine, EngineConfig};
use dhruv_search::sankranti_types::SankrantiConfig;
use dhruv_search::{
    AmshaSelectionConfig, FullKundaliConfig, GrahaPositionsConfig, avastha_for_date,
    avastha_for_graha,
};
use dhruv_vedic_base::riseset_types::{GeoLocation, RiseSetConfig};
use dhruv_vedic_base::{BhavaConfig, Graha, NodeDignityPolicy};

use dhruv_time::{EopKernel, UtcTime};

const SPK_PATH: &str = "../../kernels/data/de442s.bsp";
const LSK_PATH: &str = "../../kernels/data/naif0012.tls";
const EOP_PATH: &str = "../../kernels/data/finals2000A.all";

fn load_engine() -> Option<Engine> {
    if !Path::new(SPK_PATH).exists() || !Path::new(LSK_PATH).exists() {
        eprintln!("Skipping avastha_test: kernel files not found");
        return None;
    }
    let config = EngineConfig::with_single_spk(SPK_PATH.into(), LSK_PATH.into(), 1024, false);
    Engine::new(config).ok()
}

fn load_eop() -> Option<EopKernel> {
    if !Path::new(EOP_PATH).exists() {
        eprintln!("Skipping avastha_test: EOP file not found");
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
    UtcTime::new(2024, 1, 15, 6, 30, 0.0)
}

fn default_amsha_selection() -> AmshaSelectionConfig {
    AmshaSelectionConfig::default()
}

#[test]
fn avastha_all_nine_valid() {
    let Some(engine) = load_engine() else { return };
    let Some(eop) = load_eop() else { return };
    let utc = utc_2024_jan_15();
    let location = new_delhi();
    let bhava_config = BhavaConfig::default();
    let rs_config = RiseSetConfig::default();
    let aya_config = default_aya_config();

    let result = avastha_for_date(
        &engine,
        &eop,
        &location,
        &utc,
        &bhava_config,
        &rs_config,
        &aya_config,
        NodeDignityPolicy::SignLordBased,
        &default_amsha_selection(),
    )
    .expect("avastha_for_date should succeed");

    for (i, entry) in result.entries.iter().enumerate() {
        // All strength factors should be in [0, 1]
        let bf = entry.baladi.strength_factor();
        assert!(
            bf >= 0.0 && bf <= 1.0,
            "baladi strength for graha {i}: {bf}"
        );
        let jf = entry.jagradadi.strength_factor();
        assert!(
            jf >= 0.0 && jf <= 1.0,
            "jagradadi strength for graha {i}: {jf}"
        );
        let df = entry.deeptadi.strength_factor();
        assert!(
            df >= 0.0 && df <= 1.0,
            "deeptadi strength for graha {i}: {df}"
        );
        if let Some(lajjitadi) = entry.lajjitadi {
            let lf = lajjitadi.strength_factor();
            assert!(
                lf >= 0.0 && lf <= 1.0,
                "lajjitadi strength for graha {i}: {lf}"
            );
        }
        // Sayanadi: code 0-11 (0=Nidra, 1=Sayana, ..., 11=Kautuka)
        assert!(
            entry.sayanadi.avastha.index() < 12,
            "sayanadi index for graha {i}"
        );
        // Sub-states: 5 entries, each index 0-2
        for (j, ss) in entry.sayanadi.sub_states.iter().enumerate() {
            assert!(
                ss.index() < 3,
                "sayanadi sub_state[{j}] index for graha {i}"
            );
        }
    }
}

#[test]
fn avastha_single_graha_matches_all() {
    let Some(engine) = load_engine() else { return };
    let Some(eop) = load_eop() else { return };
    let utc = utc_2024_jan_15();
    let location = new_delhi();
    let bhava_config = BhavaConfig::default();
    let rs_config = RiseSetConfig::default();
    let aya_config = default_aya_config();
    let policy = NodeDignityPolicy::SignLordBased;

    let all = avastha_for_date(
        &engine,
        &eop,
        &location,
        &utc,
        &bhava_config,
        &rs_config,
        &aya_config,
        policy,
        &default_amsha_selection(),
    )
    .expect("avastha_for_date should succeed");

    for i in 0..9 {
        let graha = dhruv_vedic_base::ALL_GRAHAS[i];
        let single = avastha_for_graha(
            &engine,
            &eop,
            &location,
            &utc,
            &bhava_config,
            &rs_config,
            &aya_config,
            policy,
            &default_amsha_selection(),
            graha,
        )
        .expect("avastha_for_graha should succeed");
        assert_eq!(
            single.baladi.index(),
            all.entries[i].baladi.index(),
            "baladi mismatch for graha {i}"
        );
        assert_eq!(
            single.jagradadi.index(),
            all.entries[i].jagradadi.index(),
            "jagradadi mismatch for graha {i}"
        );
        assert_eq!(
            single.deeptadi.index(),
            all.entries[i].deeptadi.index(),
            "deeptadi mismatch for graha {i}"
        );
        assert_eq!(
            single.deeptadi_states.mask(),
            all.entries[i].deeptadi_states.mask(),
            "deeptadi state-set mismatch for graha {i}"
        );
        assert_eq!(single.lajjitadi, all.entries[i].lajjitadi);
        assert_eq!(
            single.lajjitadi_states.mask(),
            all.entries[i].lajjitadi_states.mask(),
            "lajjitadi state-set mismatch for graha {i}"
        );
        assert_eq!(
            single.sayanadi.avastha.index(),
            all.entries[i].sayanadi.avastha.index(),
            "sayanadi mismatch for graha {i}"
        );
    }
}

#[test]
fn avastha_both_node_policies() {
    let Some(engine) = load_engine() else { return };
    let Some(eop) = load_eop() else { return };
    let utc = utc_2024_jan_15();
    let location = new_delhi();
    let bhava_config = BhavaConfig::default();
    let rs_config = RiseSetConfig::default();
    let aya_config = default_aya_config();

    let sign_lord = avastha_for_date(
        &engine,
        &eop,
        &location,
        &utc,
        &bhava_config,
        &rs_config,
        &aya_config,
        NodeDignityPolicy::SignLordBased,
        &default_amsha_selection(),
    )
    .expect("SignLordBased should succeed");

    let always_sama = avastha_for_date(
        &engine,
        &eop,
        &location,
        &utc,
        &bhava_config,
        &rs_config,
        &aya_config,
        NodeDignityPolicy::AlwaysSama,
        &default_amsha_selection(),
    )
    .expect("AlwaysSama should succeed");

    // With AlwaysSama, Rahu/Ketu always get Sama dignity → Jagradadi=Sushupta
    let rahu_idx = Graha::Rahu.index() as usize;
    let ketu_idx = Graha::Ketu.index() as usize;
    assert_eq!(always_sama.entries[rahu_idx].jagradadi.name(), "Sushupta");
    assert_eq!(always_sama.entries[ketu_idx].jagradadi.name(), "Sushupta");

    // Sapta grahas should be identical across both policies
    for i in 0..7 {
        assert_eq!(
            sign_lord.entries[i].baladi.index(),
            always_sama.entries[i].baladi.index(),
            "sapta graha {i} baladi should match across policies"
        );
    }
}

#[test]
fn avastha_accepts_explicit_default_amsha_selection() {
    let Some(engine) = load_engine() else { return };
    let Some(eop) = load_eop() else { return };
    let utc = utc_2024_jan_15();
    let location = new_delhi();
    let bhava_config = BhavaConfig::default();
    let rs_config = RiseSetConfig::default();
    let aya_config = default_aya_config();
    let default_selection = default_amsha_selection();
    let mut explicit_selection = AmshaSelectionConfig {
        count: 1,
        ..AmshaSelectionConfig::default()
    };
    explicit_selection.codes[0] = 9;

    let baseline = avastha_for_date(
        &engine,
        &eop,
        &location,
        &utc,
        &bhava_config,
        &rs_config,
        &aya_config,
        NodeDignityPolicy::SignLordBased,
        &default_selection,
    )
    .expect("baseline avastha");
    let explicit = avastha_for_date(
        &engine,
        &eop,
        &location,
        &utc,
        &bhava_config,
        &rs_config,
        &aya_config,
        NodeDignityPolicy::SignLordBased,
        &explicit_selection,
    )
    .expect("explicit avastha");

    for (lhs, rhs) in baseline.entries.iter().zip(explicit.entries.iter()) {
        assert_eq!(lhs.baladi.index(), rhs.baladi.index());
        assert_eq!(lhs.jagradadi.index(), rhs.jagradadi.index());
        assert_eq!(lhs.deeptadi.index(), rhs.deeptadi.index());
        assert_eq!(lhs.deeptadi_states.mask(), rhs.deeptadi_states.mask());
        assert_eq!(lhs.lajjitadi, rhs.lajjitadi);
        assert_eq!(lhs.lajjitadi_states.mask(), rhs.lajjitadi_states.mask());
        assert_eq!(lhs.sayanadi.avastha.index(), rhs.sayanadi.avastha.index());
    }
}

#[test]
fn full_kundali_with_avastha() {
    let Some(engine) = load_engine() else { return };
    let Some(eop) = load_eop() else { return };
    let utc = utc_2024_jan_15();
    let location = new_delhi();
    let bhava_config = BhavaConfig::default();
    let rs_config = RiseSetConfig::default();
    let aya_config = default_aya_config();

    let config = FullKundaliConfig {
        include_avastha: true,
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

    let avastha = result.avastha.expect("avastha should be Some");
    assert_eq!(avastha.entries.len(), 9);
    for entry in &avastha.entries {
        assert!(entry.baladi.strength_factor() >= 0.0);
        assert!(entry.sayanadi.avastha.index() < 12);
    }
}
