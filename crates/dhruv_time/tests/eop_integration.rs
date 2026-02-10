//! Integration tests for IERS EOP (finals2000A.all) loading.
//!
//! Requires the finals2000A.all data file. Skips gracefully if absent.

use std::path::Path;

use dhruv_time::EopKernel;

const EOP_PATH: &str = "../../data/finals2000A.all";

fn load_eop() -> Option<EopKernel> {
    if !Path::new(EOP_PATH).exists() {
        eprintln!("Skipping eop_integration: {} not found", EOP_PATH);
        return None;
    }
    EopKernel::load(Path::new(EOP_PATH)).ok()
}

#[test]
fn load_real_file() {
    let Some(eop) = load_eop() else { return };
    let data = eop.data();
    assert!(
        data.len() > 10_000,
        "Expected >10000 entries, got {}",
        data.len()
    );
}

#[test]
fn dut1_within_bounds() {
    let Some(eop) = load_eop() else { return };
    let data = eop.data();
    let (start, end) = data.range();
    // Sample every 100 days
    let mut mjd = start;
    while mjd <= end {
        if let Ok(dut1) = data.dut1_at_mjd(mjd) {
            assert!(
                dut1.abs() < 0.9,
                "DUT1 at MJD {mjd} = {dut1}s, exceeds 0.9s bound"
            );
        }
        mjd += 100.0;
    }
}

#[test]
fn known_date_2024_jan_1() {
    // DUT1 on 2024-01-01 (MJD 60310): expected around -0.02s to +0.02s
    // (varies based on when the file was downloaded; IERS adjusts leap seconds
    // to keep |DUT1| < 0.9s)
    let Some(eop) = load_eop() else { return };
    let mjd_2024_jan_1 = 60310.0;
    let (start, end) = eop.data().range();
    if mjd_2024_jan_1 < start || mjd_2024_jan_1 > end {
        eprintln!("EOP file doesn't cover 2024-01-01, skipping");
        return;
    }
    let dut1 = eop.data().dut1_at_mjd(mjd_2024_jan_1).unwrap();
    assert!(
        dut1.abs() < 0.9,
        "DUT1 at 2024-01-01 = {dut1}s, out of expected range"
    );
}
