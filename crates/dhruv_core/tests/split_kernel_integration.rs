//! Integration tests for generated DE441/DE442 split kernels.
//!
//! These tests require locally generated split BSP files under `kernels/data`.
//! They skip gracefully when the split files or parent kernels are absent.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use dhruv_core::{Body, Engine, EngineConfig, Frame, Observer, Query, StateVector};
use dhruv_time::{SECONDS_PER_DAY, tdb_seconds_to_jd};
use jpl_kernel::SpkKernel;

#[derive(Debug, Clone)]
struct SplitSpec {
    name: String,
    parent: String,
    notes: String,
}

fn repo_data_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../kernels/data")
}

fn manifest_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../kernels/manifest/de441_de442_splits.tsv")
}

fn load_manifest() -> Vec<SplitSpec> {
    let manifest = manifest_path();
    let text = std::fs::read_to_string(&manifest)
        .unwrap_or_else(|e| panic!("failed to read {}: {e}", manifest.display()));
    text.lines()
        .filter(|line| !line.trim().is_empty() && !line.starts_with('#'))
        .map(|line| {
            let fields: Vec<&str> = line.split('|').collect();
            assert_eq!(fields.len(), 9, "bad manifest row: {line}");
            SplitSpec {
                name: fields[0].to_string(),
                parent: fields[1].to_string(),
                notes: fields[8].to_string(),
            }
        })
        .collect()
}

fn engine_for(spk_paths: Vec<PathBuf>) -> Engine {
    let lsk = repo_data_dir().join("naif0012.tls");
    Engine::new(EngineConfig {
        spk_paths,
        lsk_path: lsk,
        cache_capacity: 256,
        strict_validation: true,
    })
    .expect("engine should load")
}

fn coverage(path: &Path) -> (f64, f64) {
    let kernel = SpkKernel::load(path).expect("split kernel should load");
    let start = kernel
        .segments()
        .iter()
        .map(|segment| segment.start_epoch)
        .fold(f64::NEG_INFINITY, f64::max);
    let end = kernel
        .segments()
        .iter()
        .map(|segment| segment.end_epoch)
        .fold(f64::INFINITY, f64::min);
    assert!(start.is_finite() && end.is_finite() && start < end);
    (start, end)
}

fn sample_epochs(start_tdb_s: f64, end_tdb_s: f64) -> [f64; 3] {
    let span = end_tdb_s - start_tdb_s;
    let margin = (32.0 * SECONDS_PER_DAY).min(span / 4.0);
    [
        start_tdb_s + margin,
        start_tdb_s + span / 2.0,
        end_tdb_s - margin,
    ]
}

fn bodies() -> [Body; 11] {
    [
        Body::Sun,
        Body::Moon,
        Body::Mercury,
        Body::Venus,
        Body::Mars,
        Body::Jupiter,
        Body::Saturn,
        Body::Uranus,
        Body::Neptune,
        Body::Pluto,
        Body::Earth,
    ]
}

fn query(engine: &Engine, body: Body, epoch_tdb_s: f64) -> StateVector {
    engine
        .query(Query {
            target: body,
            observer: Observer::SolarSystemBarycenter,
            frame: Frame::IcrfJ2000,
            epoch_tdb_jd: tdb_seconds_to_jd(epoch_tdb_s),
        })
        .unwrap_or_else(|e| panic!("query failed for {body:?} at {epoch_tdb_s}: {e}"))
}

fn assert_close(label: &str, actual: StateVector, expected: StateVector) {
    for i in 0..3 {
        let dp = (actual.position_km[i] - expected.position_km[i]).abs();
        assert!(
            dp <= 1e-5,
            "{label}: position[{i}] differs by {dp} km: actual={:?} expected={:?}",
            actual.position_km,
            expected.position_km
        );
        let dv = (actual.velocity_km_s[i] - expected.velocity_km_s[i]).abs();
        assert!(
            dv <= 1e-10,
            "{label}: velocity[{i}] differs by {dv} km/s: actual={:?} expected={:?}",
            actual.velocity_km_s,
            expected.velocity_km_s
        );
    }
}

#[test]
fn split_kernels_match_parent_one_by_one() {
    let data = repo_data_dir();
    let specs: Vec<SplitSpec> = load_manifest()
        .into_iter()
        .filter(|spec| data.join(&spec.name).exists())
        .collect();
    if specs.is_empty() {
        eprintln!("Skipping: no generated split BSP files found");
        return;
    }

    let mut by_parent: BTreeMap<String, Vec<SplitSpec>> = BTreeMap::new();
    for spec in specs {
        by_parent.entry(spec.parent.clone()).or_default().push(spec);
    }

    for (parent, specs) in by_parent {
        let parent_path = data.join(&parent);
        if !parent_path.exists() {
            eprintln!("Skipping parent group {parent}: parent kernel not found");
            continue;
        }
        let parent_engine = engine_for(vec![parent_path]);

        for spec in specs {
            let split_path = data.join(&spec.name);
            let split_engine = engine_for(vec![split_path.clone()]);
            let (start, end) = coverage(&split_path);

            for epoch in sample_epochs(start, end) {
                for body in bodies() {
                    let split_state = query(&split_engine, body, epoch);
                    let parent_state = query(&parent_engine, body, epoch);
                    assert_close(
                        &format!("{} {body:?} epoch {epoch}", spec.name),
                        split_state,
                        parent_state,
                    );
                }
            }

            let before_result = split_engine.query(Query {
                target: Body::Earth,
                observer: Observer::SolarSystemBarycenter,
                frame: Frame::IcrfJ2000,
                epoch_tdb_jd: tdb_seconds_to_jd(start - SECONDS_PER_DAY),
            });
            assert!(
                before_result.is_err(),
                "{} should fail just before its coverage start",
                spec.name
            );

            let after_result = split_engine.query(Query {
                target: Body::Earth,
                observer: Observer::SolarSystemBarycenter,
                frame: Frame::IcrfJ2000,
                epoch_tdb_jd: tdb_seconds_to_jd(end + SECONDS_PER_DAY),
            });
            assert!(
                after_result.is_err(),
                "{} should fail just after its coverage end",
                spec.name
            );
        }
    }
}

#[test]
fn split_kernel_groups_smoke_test() {
    let data = repo_data_dir();
    let specs: Vec<SplitSpec> = load_manifest()
        .into_iter()
        .filter(|spec| data.join(&spec.name).exists())
        .collect();
    if specs.len() < 18 {
        eprintln!("Skipping: all generated split BSP files are required for grouped smoke tests");
        return;
    }

    let de442: Vec<PathBuf> = specs
        .iter()
        .filter(|spec| spec.name.starts_with("DE442_"))
        .map(|spec| data.join(&spec.name))
        .collect();
    let before: Vec<PathBuf> = specs
        .iter()
        .filter(|spec| spec.notes.starts_with("before-de442"))
        .map(|spec| data.join(&spec.name))
        .collect();
    let after: Vec<PathBuf> = specs
        .iter()
        .filter(|spec| spec.notes.starts_with("after-de442"))
        .map(|spec| data.join(&spec.name))
        .collect();

    let de442_engine = engine_for(de442.clone());
    query(&de442_engine, Body::Mars, 0.0);
    drop(de442_engine);

    let before_engine = engine_for(before.clone());
    query(&before_engine, Body::Moon, -400_000_000_000.0);
    drop(before_engine);

    let after_engine = engine_for(after.clone());
    query(&after_engine, Body::Jupiter, 100_000_000_000.0);
    drop(after_engine);

    let mut full = Vec::new();
    full.extend(de442);
    full.extend(before);
    full.extend(after);
    let full_engine = engine_for(full);
    query(&full_engine, Body::Earth, 0.0);
    query(&full_engine, Body::Earth, -400_000_000_000.0);
    query(&full_engine, Body::Earth, 100_000_000_000.0);
}
