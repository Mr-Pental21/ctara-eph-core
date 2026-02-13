//! Edge-epoch boundary tests.
//!
//! Validates behaviour at kernel coverage boundaries and near segment
//! junctions within DE442s.

use std::path::PathBuf;

use dhruv_core::{Body, Engine, EngineConfig, EngineError, Frame, Observer, Query};

fn kernel_paths() -> (PathBuf, PathBuf) {
    let base = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../kernels/data");
    (base.join("de442s.bsp"), base.join("naif0012.tls"))
}

fn engine() -> Option<Engine> {
    let (spk, lsk) = kernel_paths();
    if !spk.exists() || !lsk.exists() {
        eprintln!("Skipping edge-epoch tests: kernel files not found");
        return None;
    }
    Some(
        Engine::new(EngineConfig::with_single_spk(spk, lsk, 256, true))
            .expect("engine should load"),
    )
}

fn earth_ssb_query(epoch_tdb_jd: f64) -> Query {
    Query {
        target: Body::Earth,
        observer: Observer::SolarSystemBarycenter,
        frame: Frame::IcrfJ2000,
        epoch_tdb_jd,
    }
}

/// DE442s covers ~1550-Jan-01 to ~2650-Jan-22.
/// Start: ~JD 2287184.5, End: ~JD 2688976.5

#[test]
fn query_at_kernel_start_boundary() {
    // Near the start of DE442s coverage (~1900). Should succeed.
    let engine = match engine() {
        Some(e) => e,
        None => return,
    };

    // JD 2415020.5 = 1899-Dec-31 — should be within DE442s
    let result = engine.query(earth_ssb_query(2_415_020.5));
    assert!(
        result.is_ok(),
        "query near kernel start should succeed: {result:?}"
    );

    let state = result.unwrap();
    let r = (state.position_km[0].powi(2)
        + state.position_km[1].powi(2)
        + state.position_km[2].powi(2))
    .sqrt();
    let au_km = 1.496e8;
    assert!(
        r > 0.5 * au_km && r < 1.5 * au_km,
        "Earth-SSB distance at kernel start {r:.0} km should be ~1 AU"
    );
}

#[test]
fn query_at_kernel_end_boundary() {
    // Near the end of DE442s coverage (~2100). Should succeed.
    let engine = match engine() {
        Some(e) => e,
        None => return,
    };

    // JD 2488068.5 = ~2099-Dec-20 — should be within DE442s
    let result = engine.query(earth_ssb_query(2_488_068.5));
    assert!(
        result.is_ok(),
        "query near kernel end should succeed: {result:?}"
    );

    let state = result.unwrap();
    let r = (state.position_km[0].powi(2)
        + state.position_km[1].powi(2)
        + state.position_km[2].powi(2))
    .sqrt();
    let au_km = 1.496e8;
    assert!(
        r > 0.5 * au_km && r < 1.5 * au_km,
        "Earth-SSB distance at kernel end {r:.0} km should be ~1 AU"
    );
}

#[test]
fn query_before_kernel_start_fails() {
    // Well before DE442s coverage (~1550) — should fail.
    let engine = match engine() {
        Some(e) => e,
        None => return,
    };

    // JD 2200000.5 = ~1293-Nov-22 — well before DE442s
    let result = engine.query(earth_ssb_query(2_200_000.5));
    assert!(result.is_err(), "query before kernel start should fail");
}

#[test]
fn query_after_kernel_end_fails() {
    // Well after DE442s coverage (~2650) — should fail.
    let engine = match engine() {
        Some(e) => e,
        None => return,
    };

    // JD 2700000.5 = ~2680-Feb-24 — well after DE442s
    let result = engine.query(earth_ssb_query(2_700_000.5));
    assert!(result.is_err(), "query after kernel end should fail");
}

#[test]
fn query_all_bodies_at_j2000() {
    // Smoke test: every supported body should return a valid result at J2000.
    let engine = match engine() {
        Some(e) => e,
        None => return,
    };

    let bodies = [
        Body::Sun,
        Body::Mercury,
        Body::Venus,
        Body::Earth,
        Body::Moon,
        Body::Mars,
        Body::Jupiter,
        Body::Saturn,
        Body::Uranus,
        Body::Neptune,
        Body::Pluto,
    ];

    for &body in &bodies {
        let query = Query {
            target: body,
            observer: Observer::SolarSystemBarycenter,
            frame: Frame::IcrfJ2000,
            epoch_tdb_jd: 2_451_545.0,
        };
        let result = engine.query(query);
        assert!(
            result.is_ok(),
            "query for {body:?} wrt SSB at J2000 should succeed: {result:?}"
        );

        let state = result.unwrap();
        // All positions should be finite and non-zero.
        let r = (state.position_km[0].powi(2)
            + state.position_km[1].powi(2)
            + state.position_km[2].powi(2))
        .sqrt();
        assert!(
            r > 0.0 && r.is_finite(),
            "{body:?}: position magnitude {r} invalid"
        );
    }
}

#[test]
fn query_multiple_epochs_consistency() {
    // Query at a sequence of epochs across the kernel range.
    // Verify Earth-SSB distance stays within 0.9–1.1 AU (orbital variation).
    let engine = match engine() {
        Some(e) => e,
        None => return,
    };

    let au_km = 1.496e8;
    let epochs = [
        2_440_000.5, // 1968-May-24
        2_445_000.5, // 1981-Dec-18
        2_450_000.5, // 1995-Oct-09
        2_451_545.0, // 2000-Jan-01 (J2000)
        2_455_000.5, // 2009-Jun-18
        2_460_000.5, // 2023-Feb-25
        2_465_000.5, // 2036-Oct-28
        2_470_000.5, // 2050-Jul-23
        2_480_000.5, // 2077-Dec-23
    ];

    for &jd in &epochs {
        let state = engine
            .query(earth_ssb_query(jd))
            .unwrap_or_else(|e| panic!("query at JD {jd} failed: {e}"));
        let r = (state.position_km[0].powi(2)
            + state.position_km[1].powi(2)
            + state.position_km[2].powi(2))
        .sqrt();
        assert!(
            r > 0.9 * au_km && r < 1.1 * au_km,
            "Earth-SSB distance at JD {jd}: {r:.0} km is outside 0.9-1.1 AU"
        );
    }
}

#[test]
fn extreme_epoch_values_rejected() {
    let engine = match engine() {
        Some(e) => e,
        None => return,
    };

    // Infinity should be rejected at query validation.
    let result = engine.query(earth_ssb_query(f64::INFINITY));
    assert!(matches!(result, Err(EngineError::InvalidQuery(_))));

    let result = engine.query(earth_ssb_query(f64::NEG_INFINITY));
    assert!(matches!(result, Err(EngineError::InvalidQuery(_))));

    // NaN should be rejected.
    let result = engine.query(earth_ssb_query(f64::NAN));
    assert!(matches!(result, Err(EngineError::InvalidQuery(_))));
}

#[test]
fn moon_across_epoch_range() {
    // Moon-Earth distance should stay within 356,000-407,000 km (perigee/apogee)
    // at any epoch in the kernel range.
    let engine = match engine() {
        Some(e) => e,
        None => return,
    };

    let epochs = [
        2_451_545.0,  // J2000
        2_451_545.25, // +6 hours
        2_451_545.5,  // +12 hours
        2_451_560.0,  // +15 days (half lunar orbit)
        2_451_575.0,  // +30 days (full lunar orbit)
        2_460_000.5,  // 2023-Feb-25
    ];

    for &jd in &epochs {
        let state = engine
            .query(Query {
                target: Body::Moon,
                observer: Observer::Body(Body::Earth),
                frame: Frame::IcrfJ2000,
                epoch_tdb_jd: jd,
            })
            .unwrap_or_else(|e| panic!("Moon-Earth query at JD {jd} failed: {e}"));

        let r = (state.position_km[0].powi(2)
            + state.position_km[1].powi(2)
            + state.position_km[2].powi(2))
        .sqrt();
        assert!(
            r > 350_000.0 && r < 410_000.0,
            "Moon-Earth distance at JD {jd}: {r:.0} km outside expected range"
        );
    }
}
