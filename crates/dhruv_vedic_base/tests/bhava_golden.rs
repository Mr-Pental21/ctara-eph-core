//! Golden-value integration tests for bhava (house) computation.
//!
//! Requires kernel files (de442s.bsp, naif0012.tls) AND IERS EOP file
//! (finals2000A.all). Skips gracefully if any are absent.

use std::f64::consts::{PI, TAU};
use std::path::Path;

use dhruv_core::{Body, Engine, EngineConfig};
use dhruv_frames::OBLIQUITY_J2000_RAD;
use dhruv_time::{EopKernel, LeapSecondKernel, gmst_rad, local_sidereal_time_rad};
use dhruv_vedic_base::{
    BhavaConfig, BhavaReferenceMode, BhavaStartingPoint, BhavaSystem, GeoLocation, compute_bhavas,
    lagna_and_mc_rad, lagna_longitude_rad,
};

const SPK_PATH: &str = "../../data/de442s.bsp";
const LSK_PATH: &str = "../../data/naif0012.tls";
const EOP_PATH: &str = "../../data/finals2000A.all";

fn load_test_resources() -> Option<(Engine, LeapSecondKernel, EopKernel)> {
    if !Path::new(SPK_PATH).exists()
        || !Path::new(LSK_PATH).exists()
        || !Path::new(EOP_PATH).exists()
    {
        eprintln!("Skipping bhava_golden: kernel/EOP files not found");
        return None;
    }

    let config = EngineConfig {
        spk_paths: vec![SPK_PATH.into()],
        lsk_path: LSK_PATH.into(),
        cache_capacity: 1024,
        strict_validation: false,
    };
    let engine = Engine::new(config).ok()?;
    let lsk = LeapSecondKernel::load(Path::new(LSK_PATH)).ok()?;
    let eop = EopKernel::load(Path::new(EOP_PATH)).ok()?;
    Some((engine, lsk, eop))
}

/// JD UTC for a date at 0h UT.
fn jd_0h_utc(year: i32, month: u32, day: u32) -> f64 {
    dhruv_time::calendar_to_jd(year, month, day as f64)
}

#[test]
fn new_delhi_lagna_reasonable_range() {
    let Some((engine, lsk, eop)) = load_test_resources() else {
        return;
    };
    let loc = GeoLocation::new(28.6139, 77.209, 0.0);
    let jd_utc = jd_0h_utc(2024, 3, 20) + 0.5; // noon UT

    let config = BhavaConfig {
        system: BhavaSystem::Equal,
        starting_point: BhavaStartingPoint::Lagna,
        reference_mode: BhavaReferenceMode::StartOfFirst,
    };

    let result = compute_bhavas(&engine, &lsk, &eop, &loc, jd_utc, &config)
        .expect("compute_bhavas should succeed");

    // Ascendant should be in [0, 360)
    assert!(
        result.lagna_deg >= 0.0 && result.lagna_deg < 360.0,
        "Lagna = {} deg, out of range",
        result.lagna_deg
    );

    // MC should be in [0, 360)
    assert!(
        result.mc_deg >= 0.0 && result.mc_deg < 360.0,
        "MC = {} deg, out of range",
        result.mc_deg
    );
}

#[test]
fn equal_cusps_30_deg_intervals() {
    let Some((engine, lsk, eop)) = load_test_resources() else {
        return;
    };
    let loc = GeoLocation::new(28.6139, 77.209, 0.0);
    let jd_utc = jd_0h_utc(2024, 3, 20) + 0.5;

    let config = BhavaConfig::default();
    let result = compute_bhavas(&engine, &lsk, &eop, &loc, jd_utc, &config)
        .expect("compute_bhavas should succeed");

    // Each cusp should be 30 deg from the previous
    for i in 0..12 {
        let next = (i + 1) % 12;
        let diff = (result.bhavas[next].cusp_deg - result.bhavas[i].cusp_deg).rem_euclid(360.0);
        assert!(
            (diff - 30.0).abs() < 0.01,
            "cusp diff [{i}->{next}] = {diff}, expected 30",
        );
    }

    // First cusp should equal Ascendant
    assert!(
        (result.bhavas[0].cusp_deg - result.lagna_deg).abs() < 0.01,
        "cusp 1 = {}, Asc = {}",
        result.bhavas[0].cusp_deg,
        result.lagna_deg
    );
}

#[test]
fn sripati_angular_cusps_match_asc_ic_desc_mc() {
    let Some((engine, lsk, eop)) = load_test_resources() else {
        return;
    };
    let loc = GeoLocation::new(28.6139, 77.209, 0.0);
    let jd_utc = jd_0h_utc(2024, 3, 20) + 0.5;

    let config = BhavaConfig {
        system: BhavaSystem::Sripati,
        starting_point: BhavaStartingPoint::Lagna,
        reference_mode: BhavaReferenceMode::StartOfFirst,
    };
    let result = compute_bhavas(&engine, &lsk, &eop, &loc, jd_utc, &config)
        .expect("compute_bhavas should succeed");

    // Cusp 1 = Asc
    assert!(
        (result.bhavas[0].cusp_deg - result.lagna_deg).abs() < 0.01,
        "cusp 1 = {}, Asc = {}",
        result.bhavas[0].cusp_deg,
        result.lagna_deg
    );

    // Cusp 10 = MC
    assert!(
        (result.bhavas[9].cusp_deg - result.mc_deg).abs() < 0.01,
        "cusp 10 = {}, MC = {}",
        result.bhavas[9].cusp_deg,
        result.mc_deg
    );

    // Cusp 7 = Desc = Asc + 180
    let desc = (result.lagna_deg + 180.0).rem_euclid(360.0);
    assert!(
        (result.bhavas[6].cusp_deg - desc)
            .abs()
            .min((result.bhavas[6].cusp_deg - desc + 360.0).abs())
            < 0.01,
        "cusp 7 = {}, Desc = {}",
        result.bhavas[6].cusp_deg,
        desc
    );

    // Cusp 4 = IC = MC + 180
    let ic = (result.mc_deg + 180.0).rem_euclid(360.0);
    assert!(
        (result.bhavas[3].cusp_deg - ic)
            .abs()
            .min((result.bhavas[3].cusp_deg - ic + 360.0).abs())
            < 0.01,
        "cusp 4 = {}, IC = {}",
        result.bhavas[3].cusp_deg,
        ic
    );
}

#[test]
fn kp_placidus_valid_cusps() {
    let Some((engine, lsk, eop)) = load_test_resources() else {
        return;
    };
    let loc = GeoLocation::new(28.6139, 77.209, 0.0); // New Delhi, moderate latitude
    let jd_utc = jd_0h_utc(2024, 3, 20) + 0.5;

    let config = BhavaConfig {
        system: BhavaSystem::KP,
        starting_point: BhavaStartingPoint::Lagna,
        reference_mode: BhavaReferenceMode::StartOfFirst,
    };
    let result = compute_bhavas(&engine, &lsk, &eop, &loc, jd_utc, &config)
        .expect("KP should succeed at moderate latitude");

    // All cusps should be in [0, 360)
    for (i, b) in result.bhavas.iter().enumerate() {
        assert!(
            b.cusp_deg >= 0.0 && b.cusp_deg < 360.0,
            "KP cusp[{i}] = {} out of range",
            b.cusp_deg
        );
    }

    // Cusp 1 ≈ Asc, Cusp 10 ≈ MC
    assert!(
        (result.bhavas[0].cusp_deg - result.lagna_deg).abs() < 0.01,
        "KP cusp 1 should match Asc"
    );
    assert!(
        (result.bhavas[9].cusp_deg - result.mc_deg).abs() < 0.01,
        "KP cusp 10 should match MC"
    );
}

#[test]
fn all_10_systems_produce_valid_results() {
    let Some((engine, lsk, eop)) = load_test_resources() else {
        return;
    };
    let loc = GeoLocation::new(28.6139, 77.209, 0.0); // moderate latitude
    let jd_utc = jd_0h_utc(2024, 3, 20) + 0.5;

    for &system in BhavaSystem::all() {
        let config = BhavaConfig {
            system,
            starting_point: BhavaStartingPoint::Lagna,
            reference_mode: BhavaReferenceMode::StartOfFirst,
        };

        let result = compute_bhavas(&engine, &lsk, &eop, &loc, jd_utc, &config)
            .unwrap_or_else(|e| panic!("{system:?} failed: {e}"));

        for (i, b) in result.bhavas.iter().enumerate() {
            assert!(
                b.cusp_deg >= 0.0 && b.cusp_deg < 360.0,
                "{system:?} cusp[{i}] = {} out of range",
                b.cusp_deg
            );
            assert_eq!(b.number, (i as u8) + 1);
        }
    }
}

#[test]
fn body_longitude_starting_point_sun() {
    let Some((engine, lsk, eop)) = load_test_resources() else {
        return;
    };
    let loc = GeoLocation::new(28.6139, 77.209, 0.0);
    let jd_utc = jd_0h_utc(2024, 3, 20) + 0.5;

    let config = BhavaConfig {
        system: BhavaSystem::Equal,
        starting_point: BhavaStartingPoint::BodyLongitude(Body::Sun),
        reference_mode: BhavaReferenceMode::StartOfFirst,
    };

    let result = compute_bhavas(&engine, &lsk, &eop, &loc, jd_utc, &config)
        .expect("BodyLongitude(Sun) should succeed");

    // Cusp 1 should be near the Sun's ecliptic longitude (~0 deg Aries at equinox)
    // On 2024-03-20 (near vernal equinox), Sun ≈ 0 deg ecliptic
    assert!(
        result.bhavas[0].cusp_deg < 10.0 || result.bhavas[0].cusp_deg > 350.0,
        "cusp 1 = {} deg, expected near 0 (equinox Sun)",
        result.bhavas[0].cusp_deg
    );
}

#[test]
fn extreme_latitude_kp_returns_error() {
    let Some((engine, lsk, eop)) = load_test_resources() else {
        return;
    };
    let loc = GeoLocation::new(70.0, 25.0, 0.0); // Tromso-like latitude
    let jd_utc = jd_0h_utc(2024, 3, 20) + 0.5;

    let config = BhavaConfig {
        system: BhavaSystem::KP,
        starting_point: BhavaStartingPoint::Lagna,
        reference_mode: BhavaReferenceMode::StartOfFirst,
    };

    let result = compute_bhavas(&engine, &lsk, &eop, &loc, jd_utc, &config);
    assert!(result.is_err(), "KP at lat 70 should fail");
}

#[test]
fn middle_of_first_shifts_cusps() {
    let Some((engine, lsk, eop)) = load_test_resources() else {
        return;
    };
    let loc = GeoLocation::new(28.6139, 77.209, 0.0);
    let jd_utc = jd_0h_utc(2024, 3, 20) + 0.5;

    let config_start = BhavaConfig {
        system: BhavaSystem::Equal,
        starting_point: BhavaStartingPoint::Lagna,
        reference_mode: BhavaReferenceMode::StartOfFirst,
    };
    let config_mid = BhavaConfig {
        system: BhavaSystem::Equal,
        starting_point: BhavaStartingPoint::Lagna,
        reference_mode: BhavaReferenceMode::MiddleOfFirst,
    };

    let result_start = compute_bhavas(&engine, &lsk, &eop, &loc, jd_utc, &config_start).unwrap();
    let result_mid = compute_bhavas(&engine, &lsk, &eop, &loc, jd_utc, &config_mid).unwrap();

    // For equal houses, middle-of-first shifts all cusps back by 15 deg
    let diff = (result_start.bhavas[0].cusp_deg - result_mid.bhavas[0].cusp_deg).rem_euclid(360.0);
    assert!((diff - 15.0).abs() < 0.01, "shift = {diff}, expected 15");
}

/// Helper: verify the ascendant (tropical, radians) is a rising point (H < 0).
fn assert_lagna_is_rising(tropical_lagna_rad: f64, lst_rad: f64) {
    let eps = OBLIQUITY_J2000_RAD;
    let lambda = tropical_lagna_rad;
    let ra = f64::atan2(lambda.sin() * eps.cos(), lambda.cos()).rem_euclid(TAU);
    let mut h = (lst_rad - ra).rem_euclid(TAU);
    if h > PI {
        h -= TAU;
    }
    assert!(
        h < 0.0,
        "H = {:.4} rad ({:.2} deg) — ascendant should be rising (H < 0)",
        h,
        h.to_degrees()
    );
}

/// Verify the lagna from compute_bhavas is a rising point (eastern horizon).
#[test]
fn lagna_is_rising_point() {
    let Some((_engine, _lsk, eop)) = load_test_resources() else {
        return;
    };
    let loc = GeoLocation::new(28.6139, 77.209, 0.0);
    let jd_utc = jd_0h_utc(2024, 3, 20) + 0.5;

    let config = BhavaConfig {
        system: BhavaSystem::Equal,
        starting_point: BhavaStartingPoint::Lagna,
        reference_mode: BhavaReferenceMode::StartOfFirst,
    };
    let result = compute_bhavas(&_engine, &_lsk, &eop, &loc, jd_utc, &config)
        .expect("compute_bhavas should succeed");

    // Reconstruct LST via the same chain as production code
    let jd_ut1 = eop.utc_to_ut1_jd(jd_utc).expect("EOP lookup");
    let gmst = gmst_rad(jd_ut1);
    let lst = local_sidereal_time_rad(gmst, loc.longitude_rad());

    let tropical_lagna_rad = result.lagna_deg.to_radians();
    assert_lagna_is_rising(tropical_lagna_rad, lst);
}

/// Verify Desc (cusp 7) = Lagna + 180 deg for Sripati system.
#[test]
fn descendant_is_lagna_plus_180() {
    let Some((engine, lsk, eop)) = load_test_resources() else {
        return;
    };
    let loc = GeoLocation::new(28.6139, 77.209, 0.0);
    let jd_utc = jd_0h_utc(2024, 3, 20) + 0.5;

    let config = BhavaConfig {
        system: BhavaSystem::Sripati,
        starting_point: BhavaStartingPoint::Lagna,
        reference_mode: BhavaReferenceMode::StartOfFirst,
    };
    let result = compute_bhavas(&engine, &lsk, &eop, &loc, jd_utc, &config)
        .expect("compute_bhavas should succeed");

    let desc = result.bhavas[6].cusp_deg;
    let expected = (result.lagna_deg + 180.0).rem_euclid(360.0);
    let err = (desc - expected).rem_euclid(360.0);
    let err = err.min(360.0 - err);
    assert!(
        err < 0.01,
        "Desc = {desc:.4}, expected Lagna+180 = {expected:.4}"
    );
}

/// Verify lagna_longitude_rad and lagna_and_mc_rad agree.
#[test]
fn lagna_longitude_and_lagna_and_mc_agree() {
    let Some((_engine, lsk, eop)) = load_test_resources() else {
        return;
    };
    let loc = GeoLocation::new(28.6139, 77.209, 0.0);
    let jd_utc = jd_0h_utc(2024, 3, 20) + 0.5;

    let asc1 =
        lagna_longitude_rad(&lsk, &eop, &loc, jd_utc).expect("lagna_longitude_rad should succeed");
    let (asc2, _mc) =
        lagna_and_mc_rad(&lsk, &eop, &loc, jd_utc).expect("lagna_and_mc_rad should succeed");

    assert!(
        (asc1 - asc2).abs() < 1e-15,
        "lagna_longitude_rad={asc1}, lagna_and_mc_rad={asc2}"
    );
}
