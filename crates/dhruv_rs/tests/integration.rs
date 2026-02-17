//! Integration tests for dhruv_rs (require de442s.bsp + naif0012.tls).

use std::path::PathBuf;
use std::sync::Once;

use dhruv_rs::*;

static INIT: Once = Once::new();

fn kernel_paths() -> (PathBuf, PathBuf) {
    let base = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../kernels/data");
    (base.join("de442s.bsp"), base.join("naif0012.tls"))
}

fn kernels_available() -> bool {
    let (spk, lsk) = kernel_paths();
    spk.exists() && lsk.exists()
}

fn ensure_init() -> bool {
    if !kernels_available() {
        eprintln!("Skipping: kernel files not found");
        return false;
    }

    INIT.call_once(|| {
        let (spk, lsk) = kernel_paths();
        let config = EngineConfig::with_single_spk(spk, lsk, 256, true);
        init(config).expect("engine init");
    });
    true
}

#[test]
fn is_initialized_after_init() {
    if !ensure_init() {
        return;
    }
    assert!(is_initialized());
}

#[test]
fn position_mars_from_earth() {
    if !ensure_init() {
        return;
    }
    let date = UtcDate::new(2024, 3, 20, 12, 0, 0.0);
    let coords = position(Body::Mars, Observer::Body(Body::Earth), date).unwrap();

    // Mars ecliptic longitude should be in [0, 360)
    assert!(coords.lon_deg >= 0.0);
    assert!(coords.lon_deg < 360.0);
    // Distance should be reasonable (0.3 AU – 2.7 AU)
    let au_km = 149_597_870.7;
    assert!(coords.distance_km > 0.3 * au_km);
    assert!(coords.distance_km < 2.7 * au_km);
}

#[test]
fn position_full_returns_velocities() {
    if !ensure_init() {
        return;
    }
    let date = UtcDate::new(2024, 3, 20, 12, 0, 0.0);
    let state = position_full(Body::Mars, Observer::Body(Body::Earth), date).unwrap();

    assert!(state.lon_deg >= 0.0);
    assert!(state.distance_km > 0.0);
    // Angular velocities should be non-zero for a planet
    assert!(state.lon_speed != 0.0);
}

#[test]
fn longitude_returns_degrees() {
    if !ensure_init() {
        return;
    }
    let date = UtcDate::new(2024, 3, 20, 12, 0, 0.0);
    let lon = longitude(Body::Sun, Observer::Body(Body::Earth), date).unwrap();

    assert!(lon >= 0.0);
    assert!(lon < 360.0);
}

#[test]
fn query_icrf() {
    if !ensure_init() {
        return;
    }
    let date = UtcDate::new(2024, 3, 20, 12, 0, 0.0);
    let state = query(
        Body::Earth,
        Observer::SolarSystemBarycenter,
        Frame::IcrfJ2000,
        date,
    )
    .unwrap();

    // Earth's distance from SSB should be ~1 AU
    let r = (state.position_km[0].powi(2)
        + state.position_km[1].powi(2)
        + state.position_km[2].powi(2))
    .sqrt();
    let au_km = 149_597_870.7;
    assert!((r / au_km - 1.0).abs() < 0.02);
}

#[test]
fn query_batch_multiple_bodies() {
    if !ensure_init() {
        return;
    }
    let date = UtcDate::new(2024, 3, 20, 12, 0, 0.0);
    let requests = vec![
        (
            Body::Mercury,
            Observer::Body(Body::Earth),
            Frame::EclipticJ2000,
            date,
        ),
        (
            Body::Venus,
            Observer::Body(Body::Earth),
            Frame::EclipticJ2000,
            date,
        ),
        (
            Body::Mars,
            Observer::Body(Body::Earth),
            Frame::EclipticJ2000,
            date,
        ),
    ];

    let results = query_batch(&requests).unwrap();
    assert_eq!(results.len(), 3);
    for r in &results {
        assert!(r.is_ok());
    }
}

#[test]
fn date_parse_roundtrip() {
    let d: UtcDate = "2024-06-15T18:30:00Z".parse().unwrap();
    assert_eq!(d.year, 2024);
    assert_eq!(d.month, 6);
    assert_eq!(d.day, 15);
    assert_eq!(d.hour, 18);
    assert_eq!(d.min, 30);
}

#[test]
fn error_before_init_in_fresh_process() {
    // This test verifies the error type contract.
    // It cannot truly test the not-initialized path because other tests
    // may have already called init() in this test binary.
    let e = DhruvError::NotInitialized;
    assert!(e.to_string().contains("not initialized"));
}

fn eop_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../kernels/data/finals2000A.all")
}

fn all_kernels_available() -> bool {
    kernels_available() && eop_path().exists()
}

fn load_eop() -> EopKernel {
    EopKernel::parse(&std::fs::read_to_string(eop_path()).unwrap()).unwrap()
}

#[test]
fn full_kundali_with_dasha() {
    if !ensure_init() || !all_kernels_available() {
        return;
    }
    let eop = load_eop();
    let date = UtcDate::new(1990, 1, 15, 6, 30, 0.0);
    let location = dhruv_vedic_base::GeoLocation::new(28.6139, 77.2090, 0.0);

    let mut dasha_config = dhruv_search::DashaSelectionConfig::default();
    dasha_config.count = 1;
    dasha_config.systems[0] = dhruv_vedic_base::dasha::DashaSystem::Vimshottari as u8;

    let config = dhruv_search::FullKundaliConfig {
        include_dasha: true,
        dasha_config,
        ..dhruv_search::FullKundaliConfig::default()
    };

    let result = full_kundali(
        date,
        &eop,
        &location,
        AyanamshaSystem::Lahiri,
        true,
        &config,
    )
    .unwrap();

    assert!(result.dasha.is_some(), "dasha should be present");
    let dasha = result.dasha.unwrap();
    assert_eq!(dasha.len(), 1);
    assert_eq!(
        dasha[0].system,
        dhruv_vedic_base::dasha::DashaSystem::Vimshottari
    );
    assert!(!dasha[0].levels.is_empty());
}
