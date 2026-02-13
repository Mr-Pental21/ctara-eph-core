//! Lunar node (Rahu/Ketu) longitude computation.
//!
//! Provides mean and true positions of the Moon's ascending node (Rahu)
//! and descending node (Ketu = Rahu + 180 deg).
//!
//! Mean node: polynomial from IERS Conventions 2010, Table 5.2e (the 5th
//! Delaunay argument, already in `dhruv_frames::fundamental_arguments`).
//!
//! True node: mean + short-period perturbation corrections (13 sinusoidal
//! terms from Meeus, *Astronomical Algorithms* 2nd ed., Chapter 47).
//!
//! Clean-room implementation. See `docs/clean_room_lunar_nodes.md`.

use dhruv_frames::fundamental_arguments;

/// Which lunar node to compute.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LunarNode {
    /// Ascending node (Rahu / North Node).
    Rahu,
    /// Descending node (Ketu / South Node). Always Rahu + 180 deg.
    Ketu,
}

/// Array of all lunar node variants, in FFI index order.
pub const ALL_NODES: [LunarNode; 2] = [LunarNode::Rahu, LunarNode::Ketu];

impl LunarNode {
    /// All node variants in FFI index order.
    pub const fn all() -> &'static [LunarNode] {
        &ALL_NODES
    }
}

/// Mean or true (nutation-perturbed) node position.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum NodeMode {
    /// Mean node: smooth polynomial motion only.
    #[default]
    Mean,
    /// True node: mean + short-period perturbation corrections.
    True,
}

/// Array of all node mode variants, in FFI index order.
pub const ALL_MODES: [NodeMode; 2] = [NodeMode::Mean, NodeMode::True];

impl NodeMode {
    /// All mode variants in FFI index order.
    pub const fn all() -> &'static [NodeMode] {
        &ALL_MODES
    }
}

/// Normalize an angle to [0, 360) degrees.
fn normalize_deg(deg: f64) -> f64 {
    let r = deg % 360.0;
    if r < 0.0 { r + 360.0 } else { r }
}

/// Mean Rahu (ascending node) ecliptic longitude in degrees [0, 360).
///
/// `t` = Julian centuries of TDB since J2000.0.
///
/// Uses the 5th Delaunay fundamental argument (mean longitude of the
/// ascending node, Omega) from IERS Conventions 2010, Table 5.2e.
pub fn mean_rahu_deg(t: f64) -> f64 {
    let args = fundamental_arguments(t);
    // args[4] = Omega in radians
    normalize_deg(args[4].to_degrees())
}

/// Mean Ketu (descending node) ecliptic longitude in degrees [0, 360).
pub fn mean_ketu_deg(t: f64) -> f64 {
    normalize_deg(mean_rahu_deg(t) + 180.0)
}

/// Short-period perturbation correction for the true node, in degrees.
///
/// 13 sinusoidal terms from Meeus, *Astronomical Algorithms* (2nd ed.),
/// Chapter 47, Table 47.B. Each term is a sine of a linear combination
/// of Delaunay arguments, with amplitude in degrees.
///
/// `args` = `[l, l', F, D, Omega]` in radians (from `fundamental_arguments`).
fn node_perturbation_deg(args: &[f64; 5]) -> f64 {
    // Table 47.B coefficients: [nl, nl', nF, nD, nOmega, amplitude_deg]
    // Amplitudes from Meeus Ch. 47 (published textbook, public knowledge).
    #[rustfmt::skip]
    static TERMS: [[f64; 6]; 13] = [
        // nl   nl'   nF    nD    nOm   amplitude (deg)
        [ 0.0,  0.0,  0.0,  0.0,  1.0, -1.4979],
        [ 0.0,  0.0,  2.0, -2.0,  0.0,  0.1500],
        [ 0.0,  0.0,  2.0,  0.0,  0.0, -0.1226],
        [ 0.0,  0.0,  0.0,  0.0,  2.0,  0.1176],
        [ 1.0,  0.0,  0.0,  0.0,  0.0, -0.0801],
        [ 0.0,  1.0,  0.0,  0.0,  0.0,  0.0056],
        [ 0.0,  0.0,  2.0,  0.0, -2.0, -0.0047],
        [ 1.0,  0.0,  2.0,  0.0,  0.0, -0.0043],
        [ 0.0,  0.0,  2.0, -2.0,  2.0,  0.0040],
        [ 0.0,  1.0,  0.0,  0.0, -1.0,  0.0037],
        [ 0.0,  0.0,  0.0,  2.0,  0.0, -0.0030],
        [ 2.0,  0.0,  0.0,  0.0,  0.0, -0.0020],
        [ 0.0,  1.0,  2.0, -2.0,  0.0,  0.0015],
    ];

    let mut correction = 0.0_f64;
    for term in &TERMS {
        let angle = term[0] * args[0]
            + term[1] * args[1]
            + term[2] * args[2]
            + term[3] * args[3]
            + term[4] * args[4];
        correction += term[5] * angle.sin();
    }
    correction
}

/// True Rahu (ascending node) ecliptic longitude in degrees [0, 360).
///
/// Mean node + short-period perturbation corrections.
pub fn true_rahu_deg(t: f64) -> f64 {
    let args = fundamental_arguments(t);
    let mean = args[4].to_degrees();
    let perturbation = node_perturbation_deg(&args);
    normalize_deg(mean + perturbation)
}

/// True Ketu (descending node) ecliptic longitude in degrees [0, 360).
pub fn true_ketu_deg(t: f64) -> f64 {
    normalize_deg(true_rahu_deg(t) + 180.0)
}

/// Unified entry point: compute lunar node longitude in degrees [0, 360).
///
/// Matches the pattern of `ayanamsha_deg(system, t, use_nutation)`.
pub fn lunar_node_deg(node: LunarNode, t: f64, mode: NodeMode) -> f64 {
    match (node, mode) {
        (LunarNode::Rahu, NodeMode::Mean) => mean_rahu_deg(t),
        (LunarNode::Ketu, NodeMode::Mean) => mean_ketu_deg(t),
        (LunarNode::Rahu, NodeMode::True) => true_rahu_deg(t),
        (LunarNode::Ketu, NodeMode::True) => true_ketu_deg(t),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mean_rahu_at_j2000_approx_125() {
        // Omega at J2000 = 450160.398036 arcsec = 125.044556 deg
        let deg = mean_rahu_deg(0.0);
        assert!(
            (deg - 125.04).abs() < 0.1,
            "mean Rahu at J2000 = {deg}, expected ~125.04"
        );
    }

    #[test]
    fn ketu_is_180_opposite_rahu_mean() {
        for &t in &[0.0, 0.1, -0.5, 1.0] {
            let rahu = mean_rahu_deg(t);
            let ketu = mean_ketu_deg(t);
            let diff = normalize_deg(ketu - rahu);
            assert!(
                (diff - 180.0).abs() < 1e-10,
                "t={t}: Ketu-Rahu = {diff}, expected 180"
            );
        }
    }

    #[test]
    fn ketu_is_180_opposite_rahu_true() {
        for &t in &[0.0, 0.24, -0.3] {
            let rahu = true_rahu_deg(t);
            let ketu = true_ketu_deg(t);
            let diff = normalize_deg(ketu - rahu);
            assert!(
                (diff - 180.0).abs() < 1e-10,
                "t={t}: true Ketu-Rahu = {diff}, expected 180"
            );
        }
    }

    #[test]
    fn mean_node_retrograde_rate() {
        // Mean node regresses ~19.34 deg/year ≈ 1934 deg/century
        let t1 = 0.0;
        let t2 = 0.01; // 1 year = 0.01 century
        let r1 = mean_rahu_deg(t1);
        let r2 = mean_rahu_deg(t2);
        // Raw difference (may wrap). The node moves retrograde.
        let rate_per_century = (r2 - r1 + 360.0) % 360.0 - 360.0;
        let rate_per_year = rate_per_century / 0.01 * 0.01; // per year
        assert!(
            (rate_per_year - (-19.34)).abs() < 0.5,
            "regression rate = {rate_per_year} deg/yr, expected ~-19.34"
        );
    }

    #[test]
    fn perturbation_bounded() {
        // True - mean should be < 3 deg for any reasonable epoch
        for &t in &[0.0, 0.24, -1.0, 5.0] {
            let mean = mean_rahu_deg(t);
            let tr = true_rahu_deg(t);
            let mut diff = (tr - mean).abs();
            if diff > 180.0 {
                diff = 360.0 - diff;
            }
            assert!(
                diff < 3.0,
                "t={t}: |true - mean| = {diff}, should be < 3 deg"
            );
        }
    }

    #[test]
    fn perturbation_nonzero() {
        // At T=0.24, the perturbation should be nonzero
        let mean = mean_rahu_deg(0.24);
        let tr = true_rahu_deg(0.24);
        let mut diff = (tr - mean).abs();
        if diff > 180.0 {
            diff = 360.0 - diff;
        }
        assert!(diff > 0.001, "perturbation too small: {diff} deg");
    }

    #[test]
    fn normalization_range() {
        // All results should be in [0, 360)
        for &t in &[-5.0, -1.0, 0.0, 1.0, 5.0, 10.0] {
            for &mode in &[NodeMode::Mean, NodeMode::True] {
                for &node in &[LunarNode::Rahu, LunarNode::Ketu] {
                    let deg = lunar_node_deg(node, t, mode);
                    assert!(
                        (0.0..360.0).contains(&deg),
                        "node={node:?} mode={mode:?} t={t}: deg={deg} out of range"
                    );
                }
            }
        }
    }

    #[test]
    fn unified_api_matches_direct() {
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
}
