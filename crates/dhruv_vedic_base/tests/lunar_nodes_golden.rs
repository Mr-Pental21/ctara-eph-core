//! Golden validation tests for lunar node computation.
//!
//! All tests are pure math (no kernel files needed).

use dhruv_vedic_base::{
    LunarNode, NodeMode, lunar_node_deg, mean_ketu_deg, mean_rahu_deg, true_ketu_deg, true_rahu_deg,
};

/// Helper: normalize to [0, 360).
fn norm(deg: f64) -> f64 {
    let r = deg % 360.0;
    if r < 0.0 { r + 360.0 } else { r }
}

#[test]
fn mean_rahu_j2000_approx_125_04() {
    // Omega at J2000 = 450160.398036 arcsec = 125.04455... deg
    let deg = mean_rahu_deg(0.0);
    assert!(
        (deg - 125.044).abs() < 0.01,
        "mean Rahu at J2000 = {deg}, expected ~125.044"
    );
}

#[test]
fn regression_rate_approx_19_34_per_year() {
    // The mean node regresses at ~19.34 deg/year.
    // Over 1 year (0.01 century), Rahu should decrease by ~19.34 deg.
    let r0 = mean_rahu_deg(0.0);
    let r1 = mean_rahu_deg(0.01);
    // Compute signed difference accounting for wraparound
    let mut diff = r1 - r0;
    if diff > 180.0 {
        diff -= 360.0;
    }
    if diff < -180.0 {
        diff += 360.0;
    }
    assert!(
        (diff - (-19.34)).abs() < 0.5,
        "1-year regression = {diff} deg, expected ~-19.34"
    );
}

#[test]
fn full_cycle_approx_18_6_years() {
    // After ~18.6 years (0.186 century), Rahu should return near its starting position.
    let start = mean_rahu_deg(0.0);
    let end = mean_rahu_deg(0.186);
    let mut diff = (end - start).abs();
    if diff > 180.0 {
        diff = 360.0 - diff;
    }
    // Should be within ~10 deg of starting position
    assert!(
        diff < 15.0,
        "after 18.6yr, |diff| = {diff}, expected < 15 deg"
    );
}

#[test]
fn ketu_always_opposite_rahu_mean() {
    for &t in &[-2.0, -1.0, 0.0, 0.24, 1.0, 5.0] {
        let rahu = mean_rahu_deg(t);
        let ketu = mean_ketu_deg(t);
        let diff = norm(ketu - rahu);
        assert!((diff - 180.0).abs() < 1e-10, "t={t}: Ketu-Rahu = {diff}");
    }
}

#[test]
fn ketu_always_opposite_rahu_true() {
    for &t in &[-2.0, -1.0, 0.0, 0.24, 1.0, 5.0] {
        let rahu = true_rahu_deg(t);
        let ketu = true_ketu_deg(t);
        let diff = norm(ketu - rahu);
        assert!(
            (diff - 180.0).abs() < 1e-10,
            "t={t}: true Ketu-Rahu = {diff}"
        );
    }
}

#[test]
fn true_node_perturbation_nonzero_and_bounded() {
    // True - mean should be nonzero but < 3 deg
    for &t in &[0.0, 0.24, -1.0, 2.0] {
        let mean = mean_rahu_deg(t);
        let tr = true_rahu_deg(t);
        let mut diff = (tr - mean).abs();
        if diff > 180.0 {
            diff = 360.0 - diff;
        }
        assert!(
            diff < 3.0,
            "t={t}: |true - mean| = {diff} deg, expected < 3"
        );
    }
    // At J2000, perturbation should be nonzero
    let mean = mean_rahu_deg(0.0);
    let tr = true_rahu_deg(0.0);
    let mut diff = (tr - mean).abs();
    if diff > 180.0 {
        diff = 360.0 - diff;
    }
    assert!(diff > 0.001, "perturbation at J2000 too small: {diff}");
}

#[test]
fn unified_api_consistency() {
    let t = 0.24;
    assert_eq!(
        lunar_node_deg(LunarNode::Rahu, t, NodeMode::Mean),
        mean_rahu_deg(t)
    );
    assert_eq!(
        lunar_node_deg(LunarNode::Ketu, t, NodeMode::Mean),
        mean_ketu_deg(t)
    );
    assert_eq!(
        lunar_node_deg(LunarNode::Rahu, t, NodeMode::True),
        true_rahu_deg(t)
    );
    assert_eq!(
        lunar_node_deg(LunarNode::Ketu, t, NodeMode::True),
        true_ketu_deg(t)
    );
}

#[test]
fn all_outputs_in_valid_range() {
    for &t in &[-10.0, -1.0, 0.0, 1.0, 10.0] {
        for &node in LunarNode::all() {
            for &mode in NodeMode::all() {
                let deg = lunar_node_deg(node, t, mode);
                assert!(
                    (0.0..360.0).contains(&deg),
                    "node={node:?} mode={mode:?} t={t}: {deg} out of [0,360)"
                );
            }
        }
    }
}
