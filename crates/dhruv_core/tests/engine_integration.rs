//! Integration tests for the query engine (require de442s.bsp + naif0012.tls).

use std::path::PathBuf;
use std::sync::Arc;

use dhruv_core::*;

fn kernel_paths() -> (PathBuf, PathBuf) {
    let base = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../kernels/data");
    (base.join("de442s.bsp"), base.join("naif0012.tls"))
}

fn real_engine() -> Option<Engine> {
    let (spk, lsk) = kernel_paths();
    if !spk.exists() || !lsk.exists() {
        eprintln!("Skipping: kernel files not found");
        return None;
    }
    Some(
        Engine::new(EngineConfig {
            spk_paths: vec![spk],
            lsk_path: lsk,
            cache_capacity: 256,
            strict_validation: true,
        })
        .expect("should load engine"),
    )
}

#[test]
fn query_rejects_non_finite_epoch() {
    let engine = match real_engine() {
        Some(e) => e,
        None => return,
    };
    let query = Query {
        target: Body::Earth,
        observer: Observer::SolarSystemBarycenter,
        frame: Frame::IcrfJ2000,
        epoch_tdb_jd: f64::NAN,
    };
    assert!(matches!(engine.query(query), Err(EngineError::InvalidQuery(_))));
}

#[test]
fn query_rejects_same_target_observer() {
    let engine = match real_engine() {
        Some(e) => e,
        None => return,
    };
    let query = Query {
        target: Body::Earth,
        observer: Observer::Body(Body::Earth),
        frame: Frame::IcrfJ2000,
        epoch_tdb_jd: 2_451_545.0,
    };
    assert!(matches!(engine.query(query), Err(EngineError::UnsupportedQuery(_))));
}

#[test]
fn query_earth_ssb_at_j2000() {
    let engine = match real_engine() {
        Some(e) => e,
        None => return,
    };
    let query = Query {
        target: Body::Earth,
        observer: Observer::SolarSystemBarycenter,
        frame: Frame::IcrfJ2000,
        epoch_tdb_jd: 2_451_545.0, // J2000.0
    };
    let state = engine.query(query).expect("should succeed");

    // Earth should be ~1 AU from SSB.
    let r = (state.position_km[0].powi(2)
        + state.position_km[1].powi(2)
        + state.position_km[2].powi(2))
    .sqrt();
    let au_km = 1.496e8;
    assert!(
        r > 0.5 * au_km && r < 1.5 * au_km,
        "Earth-SSB distance {r:.0} km not ~1 AU"
    );
}

#[test]
fn query_mars_earth_at_2460000() {
    let engine = match real_engine() {
        Some(e) => e,
        None => return,
    };
    let query = Query {
        target: Body::Mars,
        observer: Observer::Body(Body::Earth),
        frame: Frame::IcrfJ2000,
        epoch_tdb_jd: 2_460_000.5,
    };
    let state = engine.query(query).expect("should succeed");

    // Mars-Earth distance should be between 0.3 AU and 2.7 AU.
    let r = (state.position_km[0].powi(2)
        + state.position_km[1].powi(2)
        + state.position_km[2].powi(2))
    .sqrt();
    let au_km = 1.496e8;
    assert!(
        r > 0.3 * au_km && r < 2.7 * au_km,
        "Mars-Earth distance {r:.0} km out of range"
    );
}

#[test]
fn query_deterministic() {
    let engine = match real_engine() {
        Some(e) => e,
        None => return,
    };
    let query = Query {
        target: Body::Jupiter,
        observer: Observer::Body(Body::Sun),
        frame: Frame::IcrfJ2000,
        epoch_tdb_jd: 2_460_000.5,
    };
    let first = engine.query(query).expect("should succeed");
    let second = engine.query(query).expect("should succeed");
    assert_eq!(first, second);
}

#[test]
fn query_ecliptic_frame() {
    let engine = match real_engine() {
        Some(e) => e,
        None => return,
    };
    // Query Earth in both frames and verify magnitude is preserved.
    let q_icrf = Query {
        target: Body::Earth,
        observer: Observer::SolarSystemBarycenter,
        frame: Frame::IcrfJ2000,
        epoch_tdb_jd: 2_451_545.0,
    };
    let q_ecl = Query {
        target: Body::Earth,
        observer: Observer::SolarSystemBarycenter,
        frame: Frame::EclipticJ2000,
        epoch_tdb_jd: 2_451_545.0,
    };
    let s_icrf = engine.query(q_icrf).unwrap();
    let s_ecl = engine.query(q_ecl).unwrap();

    let r_icrf = (s_icrf.position_km[0].powi(2)
        + s_icrf.position_km[1].powi(2)
        + s_icrf.position_km[2].powi(2))
    .sqrt();
    let r_ecl = (s_ecl.position_km[0].powi(2)
        + s_ecl.position_km[1].powi(2)
        + s_ecl.position_km[2].powi(2))
    .sqrt();
    assert!(
        (r_icrf - r_ecl).abs() < 1e-6,
        "magnitude not preserved: ICRF={r_icrf}, ECL={r_ecl}"
    );

    // Ecliptic latitude of Earth should be ~0 (Earth orbits in the ecliptic plane).
    let spherical = dhruv_frames::cartesian_to_spherical(&s_ecl.position_km);
    assert!(
        spherical.lat_deg.abs() < 1.0,
        "Earth ecliptic latitude {:.4} deg should be ~0",
        spherical.lat_deg
    );
}

#[test]
fn moon_relative_to_earth() {
    let engine = match real_engine() {
        Some(e) => e,
        None => return,
    };
    let query = Query {
        target: Body::Moon,
        observer: Observer::Body(Body::Earth),
        frame: Frame::IcrfJ2000,
        epoch_tdb_jd: 2_451_545.0,
    };
    let state = engine.query(query).expect("should succeed");

    // Moon-Earth distance should be ~384,400 km (within 50,000 km for elliptical orbit).
    let r = (state.position_km[0].powi(2)
        + state.position_km[1].powi(2)
        + state.position_km[2].powi(2))
    .sqrt();
    assert!(
        r > 340_000.0 && r < 420_000.0,
        "Moon-Earth distance {r:.0} km out of expected range"
    );
}

#[test]
fn context_avoids_redundant_evaluations() {
    let engine = match real_engine() {
        Some(e) => e,
        None => return,
    };
    // Moon wrt Earth: both resolve through EMB(3)->SSB(0),
    // so the second resolution should get cache hits.
    let query = Query {
        target: Body::Moon,
        observer: Observer::Body(Body::Earth),
        frame: Frame::IcrfJ2000,
        epoch_tdb_jd: 2_451_545.0,
    };
    let (state, stats) = engine.query_with_stats(query).expect("should succeed");

    // Verify result is still correct.
    let r = (state.position_km[0].powi(2)
        + state.position_km[1].powi(2)
        + state.position_km[2].powi(2))
    .sqrt();
    assert!(r > 340_000.0 && r < 420_000.0, "Moon-Earth distance {r:.0} km");

    // Moon chain: 301->3, 3->0 (2 evals)
    // Earth chain: 399->3 (1 eval, but 3->0 is cached = 1 hit)
    assert!(
        stats.cache_hits > 0,
        "expected cache hits for shared EMB->SSB hop, got {stats:?}"
    );
}

#[test]
fn query_batch_matches_individual() {
    let engine = match real_engine() {
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
    let epoch = 2_451_545.0;
    let queries: Vec<Query> = bodies
        .iter()
        .map(|&b| Query {
            target: b,
            observer: Observer::SolarSystemBarycenter,
            frame: Frame::IcrfJ2000,
            epoch_tdb_jd: epoch,
        })
        .collect();

    let batch_results = engine.query_batch(&queries);
    assert_eq!(batch_results.len(), queries.len());

    for (i, q) in queries.iter().enumerate() {
        let individual = engine.query(*q).expect("individual query should succeed");
        let batch = batch_results[i].as_ref().expect("batch query should succeed");
        assert_eq!(
            individual, *batch,
            "mismatch for body {:?}",
            q.target
        );
    }
}

#[test]
fn query_batch_empty() {
    let engine = match real_engine() {
        Some(e) => e,
        None => return,
    };
    let results = engine.query_batch(&[]);
    assert!(results.is_empty());
}

#[test]
fn query_batch_mixed_epochs() {
    let engine = match real_engine() {
        Some(e) => e,
        None => return,
    };
    let queries = vec![
        Query {
            target: Body::Earth,
            observer: Observer::SolarSystemBarycenter,
            frame: Frame::IcrfJ2000,
            epoch_tdb_jd: 2_451_545.0,
        },
        Query {
            target: Body::Mars,
            observer: Observer::SolarSystemBarycenter,
            frame: Frame::IcrfJ2000,
            epoch_tdb_jd: 2_460_000.5,
        },
        Query {
            target: Body::Moon,
            observer: Observer::Body(Body::Earth),
            frame: Frame::IcrfJ2000,
            epoch_tdb_jd: 2_451_545.0,
        },
    ];
    let results = engine.query_batch(&queries);
    assert_eq!(results.len(), 3);
    for (i, r) in results.iter().enumerate() {
        assert!(r.is_ok(), "query {i} failed: {:?}", r.as_ref().err());
    }
}

struct DummyDerived;

impl DerivedComputation for DummyDerived {
    fn name(&self) -> &'static str {
        "dummy"
    }

    fn compute(
        &self,
        _engine: &Engine,
        _query: &Query,
        state: &StateVector,
    ) -> Result<DerivedValue, EngineError> {
        Ok(DerivedValue::Scalar(state.position_km[0]))
    }
}

#[test]
fn query_with_derived_works() {
    let engine = match real_engine() {
        Some(e) => e,
        None => return,
    };
    let query = Query {
        target: Body::Mars,
        observer: Observer::Body(Body::Earth),
        frame: Frame::IcrfJ2000,
        epoch_tdb_jd: 2_460_000.5,
    };
    let (state, value) = engine
        .query_with_derived(query, &DummyDerived)
        .expect("should succeed");
    assert_eq!(value, DerivedValue::Scalar(state.position_km[0]));
}

#[test]
fn multi_kernel_loads_and_queries() {
    let (spk, lsk) = kernel_paths();
    if !spk.exists() || !lsk.exists() {
        eprintln!("Skipping: kernel files not found");
        return;
    }
    // Load the same kernel twice to exercise multi-kernel code paths.
    let engine = Engine::new(EngineConfig {
        spk_paths: vec![spk.clone(), spk],
        lsk_path: lsk,
        cache_capacity: 256,
        strict_validation: true,
    })
    .expect("should load multi-kernel engine");

    assert_eq!(engine.spk_kernels().len(), 2);

    let query = Query {
        target: Body::Earth,
        observer: Observer::SolarSystemBarycenter,
        frame: Frame::IcrfJ2000,
        epoch_tdb_jd: 2_451_545.0,
    };
    let state = engine.query(query).expect("should succeed with multi-kernel");

    let r = (state.position_km[0].powi(2)
        + state.position_km[1].powi(2)
        + state.position_km[2].powi(2))
    .sqrt();
    let au_km = 1.496e8;
    assert!(
        r > 0.5 * au_km && r < 1.5 * au_km,
        "Earth-SSB distance {r:.0} km not ~1 AU"
    );
}

#[test]
fn concurrent_queries_produce_identical_results() {
    let engine = match real_engine() {
        Some(e) => Arc::new(e),
        None => return,
    };
    let query = Query {
        target: Body::Moon,
        observer: Observer::Body(Body::Earth),
        frame: Frame::IcrfJ2000,
        epoch_tdb_jd: 2_451_545.0,
    };
    let expected = engine.query(query).expect("baseline should succeed");

    let handles: Vec<_> = (0..8)
        .map(|_| {
            let engine = Arc::clone(&engine);
            std::thread::spawn(move || engine.query(query))
        })
        .collect();

    for handle in handles {
        let result = handle.join().expect("thread should not panic");
        let state = result.expect("concurrent query should succeed");
        assert_eq!(state, expected, "concurrent result differs from baseline");
    }
}
