//! Integration tests for config-driven `graha_longitudes()`.
//!
//! Requires kernel files. Skips gracefully if absent.

use std::path::Path;

use dhruv_core::{Engine, EngineConfig};
use dhruv_search::{GrahaLongitudesConfig, graha_longitudes, moving_osculating_apogees};
use dhruv_vedic_base::{ALL_GRAHAS, AyanamshaSystem, Graha, ayanamsha_deg, jd_tdb_to_centuries};

const SPK_PATH: &str = "../../kernels/data/de442s.bsp";
const LSK_PATH: &str = "../../kernels/data/naif0012.tls";

fn load_engine() -> Option<Engine> {
    if !Path::new(SPK_PATH).exists() || !Path::new(LSK_PATH).exists() {
        eprintln!("Skipping tropical_longitudes: kernel files not found");
        return None;
    }
    let config = EngineConfig::with_single_spk(SPK_PATH.into(), LSK_PATH.into(), 1024, false);
    Engine::new(config).ok()
}

#[test]
fn moving_osculating_apogees_batch_preserves_order_and_duplicates() {
    let engine = match load_engine() {
        Some(e) => e,
        None => return,
    };
    let result = moving_osculating_apogees(
        &engine,
        2_451_545.0,
        &GrahaLongitudesConfig::sidereal(AyanamshaSystem::Lahiri, false),
        &[Graha::Mangal, Graha::Buddh, Graha::Mangal, Graha::Shani],
    )
    .expect("moving apogees should succeed");

    assert_eq!(result.entries.len(), 4);
    assert_eq!(result.entries[0].graha, Graha::Mangal);
    assert_eq!(result.entries[1].graha, Graha::Buddh);
    assert_eq!(result.entries[2].graha, Graha::Mangal);
    assert_eq!(result.entries[3].graha, Graha::Shani);
    assert_eq!(
        result.entries[0].sidereal_longitude,
        result.entries[2].sidereal_longitude
    );
    for entry in result.entries {
        assert!(entry.sidereal_longitude >= 0.0 && entry.sidereal_longitude < 360.0);
        assert!(entry.reference_plane_longitude >= 0.0 && entry.reference_plane_longitude < 360.0);
        assert!(entry.ayanamsha_deg.is_finite());
    }
}

#[test]
fn moving_osculating_apogees_rejects_unsupported_graha() {
    let engine = match load_engine() {
        Some(e) => e,
        None => return,
    };
    let err = moving_osculating_apogees(
        &engine,
        2_451_545.0,
        &GrahaLongitudesConfig::sidereal(AyanamshaSystem::Lahiri, false),
        &[Graha::Surya],
    )
    .expect_err("Surya apogee should be rejected");
    assert!(err.to_string().contains("supports only"));
}

/// For ecliptic-plane ayanamsha systems, tropical ≈ sidereal + ayanamsha (mod 360)
/// within 1e-10° for all 9 grahas. Jagganatha is excluded because its sidereal
/// path uses the invariable plane.
#[test]
fn tropical_equals_sidereal_plus_ayanamsha() {
    let engine = match load_engine() {
        Some(e) => e,
        None => return,
    };

    let jd_tdb = 2_451_545.0; // J2000
    let tropical = graha_longitudes(&engine, jd_tdb, &GrahaLongitudesConfig::tropical(false))
        .expect("tropical longitudes should succeed");

    // Test against all ecliptic-plane systems (skip Jagganatha = invariable plane)
    let systems = AyanamshaSystem::all()
        .iter()
        .copied()
        .filter(|s| *s != AyanamshaSystem::Jagganatha);

    for system in systems {
        let sidereal = graha_longitudes(
            &engine,
            jd_tdb,
            &GrahaLongitudesConfig::sidereal(system, false),
        )
        .expect("sidereal longitudes should succeed");
        let t = jd_tdb_to_centuries(jd_tdb);
        let aya = ayanamsha_deg(system, t, false);

        for graha in ALL_GRAHAS {
            let trop = tropical.longitude(graha);
            let sid = sidereal.longitude(graha);
            let reconstructed = (sid + aya).rem_euclid(360.0);
            let diff = (trop - reconstructed).rem_euclid(360.0);
            let diff = if diff > 180.0 { diff - 360.0 } else { diff };
            assert!(
                diff.abs() < 1e-10,
                "{:?} {:?}: tropical={trop:.10}, sidereal+aya={reconstructed:.10}, diff={diff:.2e}",
                system,
                graha,
            );
        }
    }
}

/// All 9 tropical longitudes should be in [0, 360).
#[test]
fn tropical_longitudes_in_valid_range() {
    let engine = match load_engine() {
        Some(e) => e,
        None => return,
    };

    let jd_tdb = 2_451_545.0;
    let result = graha_longitudes(&engine, jd_tdb, &GrahaLongitudesConfig::tropical(false))
        .expect("tropical longitudes should succeed");

    for graha in ALL_GRAHAS {
        let lon = result.longitude(graha);
        assert!(
            (0.0..360.0).contains(&lon),
            "{:?}: tropical longitude {lon} not in [0, 360)",
            graha,
        );
    }
}
