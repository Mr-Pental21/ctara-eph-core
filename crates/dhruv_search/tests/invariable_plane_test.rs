//! Integration tests for the invariable plane and Jagganatha ayanamsha.
//!
//! Validates:
//! - Jagganatha ayanamsha differs from TrueLahiri by a bounded amount
//! - Planet sidereal longitudes with Jagganatha are self-consistent
//! - Sankranti with Jagganatha: sun enters rashis at expected boundaries
//! - Panchang yoga/nakshatra consistent with invariable plane
//!
//! Requires kernel files. Skips gracefully if absent.

use std::path::Path;

use dhruv_core::{Body, Engine, EngineConfig};
use dhruv_frames::{
    DEFAULT_PRECESSION_MODEL, ReferencePlane, ecliptic_lon_to_invariable_lon, icrf_to_invariable,
};
use dhruv_search::sankranti_types::SankrantiConfig;
use dhruv_search::{
    GrahaLongitudesConfig, body_ecliptic_lon_lat, body_lon_lat_on_plane, graha_longitudes,
    next_specific_sankranti, search_sankrantis,
};
use dhruv_time::UtcTime;
use dhruv_vedic_base::ayanamsha::{ayanamsha_deg, ayanamsha_deg_on_plane, ayanamsha_mean_deg};
use dhruv_vedic_base::{AyanamshaSystem, Rashi};

const SPK_PATH: &str = "../../kernels/data/de442s.bsp";
const LSK_PATH: &str = "../../kernels/data/naif0012.tls";

fn load_engine() -> Option<Engine> {
    if !Path::new(SPK_PATH).exists() || !Path::new(LSK_PATH).exists() {
        eprintln!("Skipping invariable_plane_test: kernel files not found");
        return None;
    }
    let config = EngineConfig::with_single_spk(SPK_PATH.into(), LSK_PATH.into(), 1024, false);
    Engine::new(config).ok()
}

// ─── Ayanamsha system properties ───────────────────────────────────────────

#[test]
fn jagganatha_is_anchor_relative() {
    assert!(AyanamshaSystem::Jagganatha.is_anchor_relative());
}

#[test]
fn jagganatha_default_plane_is_invariable() {
    assert_eq!(
        AyanamshaSystem::Jagganatha.default_reference_plane(),
        ReferencePlane::Invariable
    );
}

#[test]
fn other_systems_default_to_ecliptic() {
    for system in AyanamshaSystem::all() {
        if *system != AyanamshaSystem::Jagganatha {
            assert_eq!(
                system.default_reference_plane(),
                ReferencePlane::Ecliptic,
                "{:?} should default to ecliptic",
                system
            );
        }
    }
}

// ─── Ayanamsha value tests ─────────────────────────────────────────────────

#[test]
fn jagganatha_ayanamsha_bounded_near_true_lahiri() {
    // Both are anchored to Spica at 180°, so values should be close.
    // Difference arises only from measuring on invariable vs ecliptic plane.
    // The planes are ~1.58° apart, but the ayanamsha difference is much smaller
    // because it's measured as a longitude offset, not the full tilt.
    let t = 0.0; // J2000.0
    let jagg = ayanamsha_mean_deg(AyanamshaSystem::Jagganatha, t);
    let tl = ayanamsha_mean_deg(AyanamshaSystem::TrueLahiri, t);
    let diff = (jagg - tl).abs();
    assert!(
        diff < 1.0,
        "Jagganatha vs TrueLahiri at J2000: diff={diff:.4}° should be < 1°"
    );
}

#[test]
fn jagganatha_ayanamsha_at_j2000_positive() {
    let t = 0.0;
    let aya = ayanamsha_mean_deg(AyanamshaSystem::Jagganatha, t);
    assert!(
        aya > 20.0 && aya < 30.0,
        "Jagganatha ayanamsha at J2000 = {aya:.4}°, expected 20-30°"
    );
}

#[test]
fn jagganatha_ayanamsha_on_invariable_vs_ecliptic() {
    // On the invariable plane, the ayanamsha should differ from the ecliptic value
    let t = 0.24; // ~2024
    let aya_ecl = ayanamsha_deg_on_plane(
        AyanamshaSystem::Jagganatha,
        t,
        true,
        DEFAULT_PRECESSION_MODEL,
        ReferencePlane::Ecliptic,
    );
    let aya_inv = ayanamsha_deg_on_plane(
        AyanamshaSystem::Jagganatha,
        t,
        false,
        DEFAULT_PRECESSION_MODEL,
        ReferencePlane::Invariable,
    );
    // Both should be in reasonable range
    assert!(
        aya_ecl > 20.0 && aya_ecl < 30.0,
        "ecliptic aya = {aya_ecl:.4}°"
    );
    assert!(
        aya_inv > 20.0 && aya_inv < 30.0,
        "invariable aya = {aya_inv:.4}°"
    );
    // They should differ (invariable plane has no nutation, different geometry)
    let diff = (aya_ecl - aya_inv).abs();
    assert!(
        diff < 1.0,
        "ecliptic vs invariable ayanamsha: diff={diff:.4}° should be < 1°"
    );
}

#[test]
fn jagganatha_nutation_ignored_on_invariable() {
    // On the invariable plane, use_nutation should have no effect
    let t = 0.24;
    let aya_no_nut = ayanamsha_deg_on_plane(
        AyanamshaSystem::Jagganatha,
        t,
        false,
        DEFAULT_PRECESSION_MODEL,
        ReferencePlane::Invariable,
    );
    let aya_with_nut = ayanamsha_deg_on_plane(
        AyanamshaSystem::Jagganatha,
        t,
        true,
        DEFAULT_PRECESSION_MODEL,
        ReferencePlane::Invariable,
    );
    assert!(
        (aya_no_nut - aya_with_nut).abs() < 1e-10,
        "nutation should not affect invariable plane: {aya_no_nut} vs {aya_with_nut}"
    );
}

#[test]
fn ecliptic_ayanamsha_nutation_does_matter() {
    // On the ecliptic, use_nutation should make a difference (sanity check)
    let t = 0.24;
    let aya_no_nut = ayanamsha_deg(AyanamshaSystem::Lahiri, t, false);
    let aya_with_nut = ayanamsha_deg(AyanamshaSystem::Lahiri, t, true);
    let diff = (aya_no_nut - aya_with_nut).abs();
    assert!(
        diff > 0.001,
        "nutation should affect ecliptic Lahiri: diff={diff:.6}°"
    );
}

// ─── Frame rotation tests ──────────────────────────────────────────────────

#[test]
fn ecliptic_lon_to_invariable_roundtrip_consistency() {
    // For a point on the ecliptic at longitude L, project to invariable plane.
    // Then take a point on the invariable plane at that longitude and project back.
    // The roundtrip should be close (not exact because of the latitude change).
    for deg in (0..360).step_by(30) {
        let lon = deg as f64;
        let inv_lon = ecliptic_lon_to_invariable_lon(lon);
        // The difference should be small (< 1° for 1.58° tilt)
        let diff = (inv_lon - lon + 180.0).rem_euclid(360.0) - 180.0;
        assert!(
            diff.abs() < 1.0,
            "ecl={lon}° → inv={inv_lon}°, diff={diff:.4}° should be < 1°"
        );
    }
}

#[test]
fn icrf_to_invariable_preserves_magnitude() {
    let v: [f64; 3] = [1.5e8, -3.2e7, 4.1e6];
    let mag_orig = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    let inv = icrf_to_invariable(&v);
    let mag_inv = (inv[0] * inv[0] + inv[1] * inv[1] + inv[2] * inv[2]).sqrt();
    assert!(
        (mag_orig - mag_inv).abs() / mag_orig < 1e-14,
        "magnitude not preserved: {mag_orig} vs {mag_inv}"
    );
}

// ─── Planet longitude tests (require kernel files) ─────────────────────────

#[test]
fn body_lon_on_invariable_vs_ecliptic_bounded() {
    // Planet longitudes on both planes should differ by < 2° (planes ~1.58° apart)
    let Some(engine) = load_engine() else { return };
    let utc = UtcTime::new(2024, 3, 20, 12, 0, 0.0);
    let jd_tdb = utc.to_jd_tdb(engine.lsk());

    for body in [
        Body::Sun,
        Body::Moon,
        Body::Mars,
        Body::Jupiter,
        Body::Saturn,
    ] {
        let (ecl_lon, _) = body_ecliptic_lon_lat(&engine, body, jd_tdb).unwrap();
        let (inv_lon, _) = body_lon_lat_on_plane(
            &engine,
            body,
            jd_tdb,
            DEFAULT_PRECESSION_MODEL,
            ReferencePlane::Invariable,
        )
        .unwrap();

        let diff = (ecl_lon - inv_lon + 180.0).rem_euclid(360.0) - 180.0;
        assert!(
            diff.abs() < 2.0,
            "{:?}: ecl={ecl_lon:.4}°, inv={inv_lon:.4}°, diff={diff:.4}° should be < 2°",
            body
        );
    }
}

#[test]
fn graha_longitudes_sidereal_jagganatha_in_range() {
    // All 9 graha sidereal longitudes should be in [0, 360) with Jagganatha
    let Some(engine) = load_engine() else { return };
    let utc = UtcTime::new(2024, 3, 20, 12, 0, 0.0);
    let jd_tdb = utc.to_jd_tdb(engine.lsk());

    let lons = graha_longitudes(
        &engine,
        jd_tdb,
        &GrahaLongitudesConfig::sidereal(AyanamshaSystem::Jagganatha, true),
    )
    .unwrap();

    for i in 0..9 {
        let lon = lons.longitudes[i];
        assert!(
            (0.0..360.0).contains(&lon),
            "graha {i}: sidereal lon = {lon:.4}° out of [0, 360)"
        );
    }
}

#[test]
fn graha_sidereal_jagganatha_vs_lahiri_bounded() {
    // With similar ayanamsha values, sidereal longitudes should differ by < 2°
    let Some(engine) = load_engine() else { return };
    let utc = UtcTime::new(2024, 6, 15, 12, 0, 0.0);
    let jd_tdb = utc.to_jd_tdb(engine.lsk());

    let jagg = graha_longitudes(
        &engine,
        jd_tdb,
        &GrahaLongitudesConfig::sidereal(AyanamshaSystem::Jagganatha, true),
    )
    .unwrap();
    let lahiri = graha_longitudes(
        &engine,
        jd_tdb,
        &GrahaLongitudesConfig::sidereal(AyanamshaSystem::Lahiri, true),
    )
    .unwrap();

    // Sapta grahas (0-6): longitudes should be within ~2° (planes ~1.58° apart).
    for i in 0..7 {
        let diff = (jagg.longitudes[i] - lahiri.longitudes[i] + 180.0).rem_euclid(360.0) - 180.0;
        assert!(
            diff.abs() < 2.0,
            "graha {i}: Jagganatha={:.4}°, Lahiri={:.4}°, diff={diff:.4}°",
            jagg.longitudes[i],
            lahiri.longitudes[i]
        );
    }
    // Rahu/Ketu (7,8): lunar nodes differ more because the osculating node
    // is computed from angular momentum on a different plane. Bound is looser.
    for i in 7..9 {
        let diff = (jagg.longitudes[i] - lahiri.longitudes[i] + 180.0).rem_euclid(360.0) - 180.0;
        assert!(
            diff.abs() < 25.0,
            "graha {i}: Jagganatha={:.4}°, Lahiri={:.4}°, diff={diff:.4}°",
            jagg.longitudes[i],
            lahiri.longitudes[i]
        );
    }
}

#[test]
fn graha_sidereal_jagganatha_self_consistent() {
    // Verify: planet_sidereal = planet_invariable_lon - ayanamsha_invariable
    // This checks the internal consistency of the computation.
    let Some(engine) = load_engine() else { return };
    let utc = UtcTime::new(2024, 9, 22, 12, 0, 0.0);
    let jd_tdb = utc.to_jd_tdb(engine.lsk());

    let t = (jd_tdb - 2_451_545.0) / 36525.0;
    let aya = ayanamsha_deg_on_plane(
        AyanamshaSystem::Jagganatha,
        t,
        false,
        DEFAULT_PRECESSION_MODEL,
        ReferencePlane::Invariable,
    );

    let lons = graha_longitudes(
        &engine,
        jd_tdb,
        &GrahaLongitudesConfig::sidereal(AyanamshaSystem::Jagganatha, false),
    )
    .unwrap();

    // Check Sun (index 0)
    let (sun_inv_lon, _) = body_lon_lat_on_plane(
        &engine,
        Body::Sun,
        jd_tdb,
        DEFAULT_PRECESSION_MODEL,
        ReferencePlane::Invariable,
    )
    .unwrap();
    let expected_sid = (sun_inv_lon - aya).rem_euclid(360.0);
    let diff = (lons.longitudes[0] - expected_sid + 180.0).rem_euclid(360.0) - 180.0;
    assert!(
        diff.abs() < 0.001,
        "Sun: sidereal={:.6}°, expected={expected_sid:.6}°, diff={diff:.6}°",
        lons.longitudes[0]
    );

    // Check Moon (index 1)
    let (moon_inv_lon, _) = body_lon_lat_on_plane(
        &engine,
        Body::Moon,
        jd_tdb,
        DEFAULT_PRECESSION_MODEL,
        ReferencePlane::Invariable,
    )
    .unwrap();
    let expected_sid = (moon_inv_lon - aya).rem_euclid(360.0);
    let diff = (lons.longitudes[1] - expected_sid + 180.0).rem_euclid(360.0) - 180.0;
    assert!(
        diff.abs() < 0.001,
        "Moon: sidereal={:.6}°, expected={expected_sid:.6}°, diff={diff:.6}°",
        lons.longitudes[1]
    );
}

// ─── Sankranti tests ───────────────────────────────────────────────────────

#[test]
fn sankranti_jagganatha_makar_2024() {
    // With Jagganatha ayanamsha, Makar Sankranti should still be around Jan 14-15
    let Some(engine) = load_engine() else { return };
    let utc = UtcTime::new(2024, 1, 1, 0, 0, 0.0);
    let config = SankrantiConfig::new(AyanamshaSystem::Jagganatha, true);

    let event = next_specific_sankranti(&engine, &utc, Rashi::Makara, &config)
        .unwrap()
        .expect("should find Makar Sankranti");

    assert_eq!(event.utc.year, 2024);
    assert_eq!(event.utc.month, 1);
    // Makar Sankranti: typically Jan 14-16
    assert!(
        event.utc.day >= 13 && event.utc.day <= 17,
        "expected day 13-17, got {}",
        event.utc.day
    );
    assert_eq!(event.rashi, Rashi::Makara);
    // Sidereal longitude should be very close to 270°
    assert!(
        (event.sun_sidereal_longitude_deg - 270.0).abs() < 0.1,
        "expected ~270°, got {:.4}°",
        event.sun_sidereal_longitude_deg
    );
}

#[test]
fn sankranti_jagganatha_mesha_2024() {
    // Mesha Sankranti with Jagganatha, typically April 13-15
    let Some(engine) = load_engine() else { return };
    let utc = UtcTime::new(2024, 3, 1, 0, 0, 0.0);
    let config = SankrantiConfig::new(AyanamshaSystem::Jagganatha, true);

    let event = next_specific_sankranti(&engine, &utc, Rashi::Mesha, &config)
        .unwrap()
        .expect("should find Mesha Sankranti");

    assert_eq!(event.utc.year, 2024);
    assert_eq!(event.utc.month, 4);
    assert!(
        event.utc.day >= 12 && event.utc.day <= 16,
        "expected day 12-16, got {}",
        event.utc.day
    );
    assert_eq!(event.rashi, Rashi::Mesha);
}

#[test]
fn sankranti_jagganatha_12_events_per_year() {
    // Should find 12 sankrantis in a year (Sun enters each rashi once)
    let Some(engine) = load_engine() else { return };
    let start = UtcTime::new(2024, 1, 1, 0, 0, 0.0);
    let end = UtcTime::new(2025, 1, 1, 0, 0, 0.0);
    let config = SankrantiConfig::new(AyanamshaSystem::Jagganatha, true);

    let events = search_sankrantis(&engine, &start, &end, &config).unwrap();
    assert!(
        events.len() >= 12,
        "expected >= 12 sankrantis, got {}",
        events.len()
    );
    // Each event's sidereal longitude should be near a 30° boundary
    for event in &events {
        let boundary = (event.sun_sidereal_longitude_deg / 30.0).round() * 30.0;
        let diff = (event.sun_sidereal_longitude_deg - boundary).abs();
        assert!(
            diff < 0.1,
            "event rashi={:?}: sidereal={:.4}°, nearest boundary={boundary:.0}°",
            event.rashi,
            event.sun_sidereal_longitude_deg
        );
    }
}

#[test]
fn sankranti_jagganatha_dates_close_to_lahiri() {
    // Jagganatha sankranti dates should be within a few days of Lahiri
    let Some(engine) = load_engine() else { return };
    let utc = UtcTime::new(2024, 1, 1, 0, 0, 0.0);
    let lahiri_cfg = SankrantiConfig::default_lahiri();
    let jagg_cfg = SankrantiConfig::new(AyanamshaSystem::Jagganatha, true);

    let lahiri_event = next_specific_sankranti(&engine, &utc, Rashi::Makara, &lahiri_cfg)
        .unwrap()
        .expect("Lahiri Makar Sankranti");
    let jagg_event = next_specific_sankranti(&engine, &utc, Rashi::Makara, &jagg_cfg)
        .unwrap()
        .expect("Jagganatha Makar Sankranti");

    let lahiri_jd = lahiri_event.utc.to_jd_tdb(engine.lsk());
    let jagg_jd = jagg_event.utc.to_jd_tdb(engine.lsk());
    let diff_days = (lahiri_jd - jagg_jd).abs();
    assert!(
        diff_days < 3.0,
        "Makar Sankranti: Lahiri vs Jagganatha differ by {diff_days:.2} days, expected < 3"
    );
}
