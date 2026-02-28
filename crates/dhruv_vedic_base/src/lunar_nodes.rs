//! Lunar node (Rahu/Ketu) longitude computation.
//!
//! Provides mean and true positions of the Moon's ascending node (Rahu)
//! and descending node (Ketu = Rahu + 180 deg).
//!
//! Mean node: polynomial from IERS Conventions 2010, Table 5.2e (the 5th
//! Delaunay argument, already in `dhruv_frames::fundamental_arguments`).
//!
//! True node (pure-math): mean + 50-term sin/cos perturbation series fitted
//! by matching pursuit against DE442s osculating node output (1900–2100).
//! RMS residual ≈ 5″ vs the osculating node.
//!
//! True node (engine-aware): osculating node from Moon state vectors via
//! `r × v` cross product — the most accurate path.
//!
//! Clean-room implementation. See `docs/clean_room_lunar_nodes.md`.

use dhruv_core::{Body, Engine, Frame, Observer, Query};
use dhruv_frames::{
    DEFAULT_PRECESSION_MODEL, PrecessionModel, ReferencePlane, fundamental_arguments,
    icrf_to_ecliptic, icrf_to_invariable, precess_ecliptic_j2000_to_date_with_model,
};

use crate::error::VedicError;

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
///
/// The default is `True`, matching standard Vedic/jyotish practice where
/// true nodes are preferred.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum NodeMode {
    /// Mean node: smooth polynomial motion only.
    Mean,
    ///
    /// In pure math mode (`lunar_node_deg`) this is a 50-term perturbation
    /// series fitted against DE442s.  In engine-aware mode
    /// (`lunar_node_deg_for_epoch`) this is the osculating node from the
    /// Moon state vector.
    #[default]
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

/// Short-period perturbation correction for the true (osculating) node, in
/// degrees.
///
/// 50 sin+cos terms fitted by matching pursuit against the osculating node
/// computed from DE442s Moon state vectors over 1900–2100.  Each term is a
/// linear combination of the five Delaunay fundamental arguments (l, l′, F,
/// D, Ω) from IERS Conventions 2010.
///
/// Coefficients are entirely self-derived (black-box fit against our own
/// engine output).  RMS residual ≈ 5″, max residual ≈ 20″ over the
/// fitting interval.
///
/// `args` = `[l, l', F, D, Omega]` in radians (from `fundamental_arguments`).
fn node_perturbation_deg(args: &[f64; 5]) -> f64 {
    // [nl, nl', nF, nD, nΩ, sin_coeff_deg, cos_coeff_deg]
    // Sorted by amplitude descending.  Fitted against DE442s osculating node.
    #[rustfmt::skip]
    static TERMS: [[f64; 7]; 50] = [
        // nl   nl'    nF    nD   nΩ   sin(deg)           cos(deg)
        [ 0.0,  0.0, -2.0,  2.0, 0.0, -1.4979239402150,  -0.0001059954323],
        [ 0.0, -1.0,  0.0,  0.0, 0.0,  0.1498131460645,  -0.0002566757114],
        [ 0.0,  0.0,  0.0, -2.0, 0.0,  0.1226972764118,   0.0000633183059],
        [ 0.0,  0.0, -2.0,  0.0, 0.0, -0.1176387004673,   0.0000037920799],
        [-2.0,  0.0,  2.0,  0.0, 0.0,  0.0801154663235,   0.0000045983752],
        [ 0.0, -1.0, -2.0,  2.0, 0.0, -0.0615035637560,  -0.0000230232572],
        [-1.0,  0.0,  0.0,  2.0, 0.0,  0.0490306269972,  -0.0000627376583],
        [-1.0,  0.0,  2.0,  0.0, 0.0, -0.0409462201268,   0.0001215828415],
        [-1.0,  0.0,  0.0,  0.0, 0.0, -0.0326640101257,  -0.0000083779560],
        [ 0.0, -1.0,  2.0, -2.0, 0.0, -0.0323676251895,   0.0000461986351],
        [ 0.0, -1.0,  1.0, -1.0, 2.0,  0.0105862544290,   0.0247349213374],
        [ 0.0,  0.0, -4.0,  4.0, 0.0,  0.0196523144925,  -0.0000130241587],
        [-1.0,  0.0, -2.0,  2.0, 0.0,  0.0180124482540,  -0.0000014144096],
        [-1.0,  0.0,  2.0, -2.0, 0.0,  0.0150193555877,   0.0000079768060],
        [-2.0,  0.0,  0.0,  2.0, 0.0,  0.0149525434406,  -0.0000115038755],
        [ 0.0, -1.0,  0.0,  2.0, 0.0, -0.0077970609612,   0.0000051298335],
        [-1.0,  0.0,  0.0, -2.0, 0.0,  0.0044753709267,  -0.0000008380343],
        [-1.0,  0.0, -2.0,  0.0, 0.0, -0.0043333719096,  -0.0000007123179],
        [-1.0,  0.0,  0.0,  1.0, 0.0, -0.0042435671107,   0.0000005299924],
        [ 0.0, -1.0,  1.0, -1.0, 1.0,  0.0006018308748,   0.0033749627600],
        [ 0.0, -1.0,  2.0,  0.0, 0.0,  0.0031019394365,  -0.0000177404232],
        [-1.0, -1.0,  0.0,  2.0, 0.0,  0.0030710763677,  -0.0000039706595],
        [ 0.0,  0.0, -4.0,  2.0, 0.0,  0.0029128692216,  -0.0000044198555],
        [ 0.0, -1.0, -2.0,  0.0, 0.0, -0.0027589128046,  -0.0000008005825],
        [-1.0,  0.0, -2.0,  4.0, 0.0, -0.0024689956523,   0.0000014212893],
        [-2.0,  0.0,  4.0, -2.0, 0.0, -0.0021054933220,  -0.0000018198760],
        [ 0.0,  0.0,  0.0, -1.0, 0.0,  0.0020573466822,  -0.0000086245967],
        [ 0.0, -2.0, -2.0,  2.0, 0.0, -0.0019272942128,  -0.0000048694608],
        [ 0.0, -2.0,  0.0,  0.0, 0.0,  0.0018419389048,  -0.0000219712994],
        [ 0.0,  0.0, -2.0,  4.0, 0.0, -0.0017027717609,  -0.0000036069022],
        [-1.0,  0.0,  2.0, -1.0, 0.0, -0.0016217910062,   0.0000018771620],
        [ 0.0, -1.0, -4.0,  4.0, 0.0,  0.0016146139731,   0.0000012605950],
        [ 0.0, -1.0,  0.0, -2.0, 0.0, -0.0014688298642,  -0.0000003017546],
        [-2.0,  0.0, -2.0,  4.0, 0.0, -0.0012702416125,   0.0000032064006],
        [-1.0,  1.0,  0.0,  0.0, 0.0, -0.0012680607596,   0.0000019674753],
        [-1.0,  1.0,  2.0,  0.0, 0.0, -0.0012423007025,  -0.0000012197705],
        [-1.0,  0.0,  0.0,  4.0, 0.0, -0.0011066872687,   0.0000003689599],
        [-1.0,  1.0,  2.0, -2.0, 0.0,  0.0010945100799,   0.0000053974541],
        [ 0.0,  0.0, -2.0,  1.0, 0.0, -0.0010615232349,  -0.0000003272183],
        [-1.0,  0.0,  4.0, -2.0, 0.0,  0.0010610200700,   0.0000026862412],
        [-1.0, -1.0, -2.0,  2.0, 0.0,  0.0010261101587,  -0.0000001341425],
        [ 0.0,  0.0, -2.0,  3.0, 0.0,  0.0010218527830,   0.0000012554115],
        [-1.0,  0.0,  2.0,  2.0, 0.0,  0.0009986768280,  -0.0000001030201],
        [-1.0, -1.0,  2.0,  0.0, 0.0, -0.0009420095351,   0.0000010485120],
        [ 0.0, -1.0,  4.0, -4.0, 0.0,  0.0008538382564,  -0.0000016699627],
        [-2.0,  1.0,  2.0,  0.0, 0.0,  0.0007779981254,  -0.0000007401061],
        [-2.0, -2.0,  3.0, -1.0, 1.0, -0.0001081629910,  -0.0006983566589],
        [ 0.0, -1.0, -1.0,  1.0, 2.0, -0.0002644205545,  -0.0006499502213],
        [ 0.0,  0.0, -2.0,  2.0,-1.0,  0.0006989603379,   0.0001074642632],
        [-1.0, -1.0,  0.0,  0.0, 0.0, -0.0006731873598,  -0.0000011196782],
    ];

    let mut correction = 0.0_f64;
    for term in &TERMS {
        let angle = term[0] * args[0]
            + term[1] * args[1]
            + term[2] * args[2]
            + term[3] * args[3]
            + term[4] * args[4];
        let (sn, cs) = angle.sin_cos();
        correction += term[5] * sn + term[6] * cs;
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

fn cross(a: &[f64; 3], b: &[f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

/// Osculating Rahu (ascending node) ecliptic-of-date longitude in degrees.
///
/// Uses the Moon geocentric state vector from the ephemeris engine:
/// 1. Query Moon relative to Earth in ICRF/J2000.
/// 2. Rotate to J2000 ecliptic.
/// 3. Compute orbital angular momentum vector `h = r × v`.
/// 4. Precess `h` to ecliptic-of-date using the selected precession model.
/// 5. Ascending node direction is `N = k × h` where `k = (0,0,1)`.
/// 6. Longitude = `atan2(Ny, Nx)`.
fn osculating_rahu_deg_with_model(
    engine: &Engine,
    jd_tdb: f64,
    precession_model: PrecessionModel,
) -> Result<f64, VedicError> {
    osculating_rahu_deg_on_plane(engine, jd_tdb, precession_model, ReferencePlane::Ecliptic)
}

/// Osculating Rahu longitude on the specified reference plane.
///
/// For `Ecliptic`: standard ecliptic-of-date ascending node (existing behavior).
/// For `Invariable`: ascending node of Moon's orbit relative to the invariable plane.
fn osculating_rahu_deg_on_plane(
    engine: &Engine,
    jd_tdb: f64,
    precession_model: PrecessionModel,
    plane: ReferencePlane,
) -> Result<f64, VedicError> {
    let query = Query {
        target: Body::Moon,
        observer: Observer::Body(Body::Earth),
        frame: Frame::IcrfJ2000,
        epoch_tdb_jd: jd_tdb,
    };
    let state = engine.query(query)?;

    // Transform position and velocity to the target reference plane
    let (r_plane, v_plane) = match plane {
        ReferencePlane::Ecliptic => {
            let r = icrf_to_ecliptic(&state.position_km);
            let v = icrf_to_ecliptic(&state.velocity_km_s);
            // Precess from J2000 ecliptic to ecliptic-of-date
            let t = (jd_tdb - 2_451_545.0) / 36525.0;
            let r_date = precess_ecliptic_j2000_to_date_with_model(&r, t, precession_model);
            let v_date = precess_ecliptic_j2000_to_date_with_model(&v, t, precession_model);
            (r_date, v_date)
        }
        ReferencePlane::Invariable => {
            // Invariable plane is fixed — no precession needed
            let r = icrf_to_invariable(&state.position_km);
            let v = icrf_to_invariable(&state.velocity_km_s);
            (r, v)
        }
    };

    let h = cross(&r_plane, &v_plane);
    let h_norm2 = h[0] * h[0] + h[1] * h[1] + h[2] * h[2];
    if h_norm2 < 1e-30 {
        return Err(VedicError::InvalidInput("moon angular momentum too small"));
    }

    // N = k × h = (-hy, hx, 0)
    let nx = -h[1];
    let ny = h[0];
    if nx.abs() < 1e-15 && ny.abs() < 1e-15 {
        return Err(VedicError::InvalidInput(
            "ascending node direction ill-defined",
        ));
    }

    Ok(normalize_deg(f64::atan2(ny, nx).to_degrees()))
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

/// Engine-aware lunar node longitude in degrees [0, 360).
///
/// - `NodeMode::Mean`: mean node polynomial (same as [`lunar_node_deg`]).
/// - `NodeMode::True`: osculating node from Moon state vectors.
///
/// This is the preferred API when an ephemeris engine is available.
pub fn lunar_node_deg_for_epoch(
    engine: &Engine,
    node: LunarNode,
    jd_tdb: f64,
    mode: NodeMode,
) -> Result<f64, VedicError> {
    lunar_node_deg_for_epoch_with_model(engine, node, jd_tdb, mode, DEFAULT_PRECESSION_MODEL)
}

/// Engine-aware lunar node longitude in degrees [0, 360) with explicit
/// precession model control for osculating true-node evaluation.
pub fn lunar_node_deg_for_epoch_with_model(
    engine: &Engine,
    node: LunarNode,
    jd_tdb: f64,
    mode: NodeMode,
    precession_model: PrecessionModel,
) -> Result<f64, VedicError> {
    let rahu = match mode {
        NodeMode::Mean => {
            let t = (jd_tdb - 2_451_545.0) / 36525.0;
            mean_rahu_deg(t)
        }
        NodeMode::True => osculating_rahu_deg_with_model(engine, jd_tdb, precession_model)?,
    };

    let out = match node {
        LunarNode::Rahu => rahu,
        LunarNode::Ketu => normalize_deg(rahu + 180.0),
    };
    Ok(out)
}

/// Engine-aware lunar node longitude on a specified reference plane.
///
/// For `Ecliptic`: standard ecliptic-of-date ascending node.
/// For `Invariable`: ascending node of Moon's orbit relative to the invariable plane.
/// Mean node mode always returns ecliptic longitude regardless of plane (the polynomial
/// is defined on the ecliptic).
pub fn lunar_node_deg_for_epoch_on_plane(
    engine: &Engine,
    node: LunarNode,
    jd_tdb: f64,
    mode: NodeMode,
    precession_model: PrecessionModel,
    plane: ReferencePlane,
) -> Result<f64, VedicError> {
    let rahu = match mode {
        NodeMode::Mean => {
            let t = (jd_tdb - 2_451_545.0) / 36525.0;
            mean_rahu_deg(t)
        }
        NodeMode::True => osculating_rahu_deg_on_plane(engine, jd_tdb, precession_model, plane)?,
    };

    let out = match node {
        LunarNode::Rahu => rahu,
        LunarNode::Ketu => normalize_deg(rahu + 180.0),
    };
    Ok(out)
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
    fn default_is_true() {
        assert_eq!(NodeMode::default(), NodeMode::True);
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
