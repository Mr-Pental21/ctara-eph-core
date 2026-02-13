//! Integration tests for LSK loading and UTC/TDB conversion (require naif0012.tls).

use std::path::Path;

use dhruv_time::{LeapSecondKernel, calendar_to_jd, jd_to_tdb_seconds};

fn lsk_path() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../kernels/data/naif0012.tls")
}

fn load_lsk() -> Option<LeapSecondKernel> {
    let path = lsk_path();
    if !path.exists() {
        eprintln!("Skipping: LSK not found at {}", path.display());
        return None;
    }
    Some(LeapSecondKernel::load(&path).expect("should load naif0012.tls"))
}

#[test]
fn load_real_lsk() {
    let lsk = match load_lsk() {
        Some(l) => l,
        None => return,
    };

    // naif0012.tls should have 28 leap second entries (10s in 1972 through 37s in 2017).
    assert!(
        lsk.data().leap_seconds.len() >= 28,
        "expected >= 28 leap seconds, got {}",
        lsk.data().leap_seconds.len()
    );

    // Last entry should be 37s.
    let last = lsk.data().leap_seconds.last().unwrap();
    assert!(
        (last.0 - 37.0).abs() < 1e-10,
        "last leap second value: {}",
        last.0
    );
}

#[test]
fn utc_tdb_roundtrip_with_real_lsk() {
    let lsk = match load_lsk() {
        Some(l) => l,
        None => return,
    };

    // 2024-Jun-15 00:00:00 UTC
    let utc_jd = calendar_to_jd(2024, 6, 15.0);
    let utc_s = jd_to_tdb_seconds(utc_jd);

    let tdb_s = lsk.utc_to_tdb(utc_s);
    let recovered_utc_s = lsk.tdb_to_utc(tdb_s);

    assert!(
        (utc_s - recovered_utc_s).abs() < 1e-9,
        "roundtrip error: {:.3e} s",
        (utc_s - recovered_utc_s).abs()
    );
}
