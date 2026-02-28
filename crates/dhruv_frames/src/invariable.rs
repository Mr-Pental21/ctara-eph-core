//! Invariable plane transformations.
//!
//! The invariable plane is perpendicular to the total angular momentum vector
//! of the solar system. It is fixed (does not precess), unlike the ecliptic.
//!
//! Constants from Souami & Souchay (2012), "The solar system's invariable
//! plane", A&A 543, A133:
//! - Inclination to ecliptic J2000: i = 1°34'43.3" = 1.578694°
//! - Ascending node on ecliptic J2000: Ω = 107°34'56.2" = 107.582278°
//!
//! Rotation matrix from ecliptic J2000 to invariable plane:
//! R = R3(-Ω) · R1(-i) · R3(Ω)
//!
//! Since the invariable plane is fixed, NO precession is needed when computing
//! longitudes on it. This contrasts with the ecliptic, where precession must
//! be applied to get of-date coordinates.

use crate::rotation::{ecliptic_to_icrf, icrf_to_ecliptic};

/// Reference plane for positional measurements.
///
/// Most ayanamsha systems use the ecliptic (default). The Jagganatha
/// system uses the invariable plane.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ReferencePlane {
    /// Ecliptic plane (standard, precesses over time).
    #[default]
    Ecliptic,
    /// Invariable plane (fixed, perpendicular to solar system angular momentum).
    Invariable,
}

/// Inclination of the invariable plane to the ecliptic J2000 in degrees.
///
/// Souami & Souchay (2012), Table 2: i = 1°34'43.3"
pub const INVARIABLE_INCLINATION_DEG: f64 = 1.578_694;

/// Longitude of the ascending node of the invariable plane on the ecliptic
/// J2000 in degrees.
///
/// Souami & Souchay (2012), Table 2: Ω = 107°34'56.2"
pub const INVARIABLE_NODE_DEG: f64 = 107.582_278;

// Precomputed trigonometric values for the rotation matrix.
const I_RAD: f64 = INVARIABLE_INCLINATION_DEG * (std::f64::consts::PI / 180.0);
const OMEGA_RAD: f64 = INVARIABLE_NODE_DEG * (std::f64::consts::PI / 180.0);

// sin/cos of inclination (precomputed as const — valid because they are
// used in a lazy_static or computed at init; Rust const eval doesn't allow
// sin/cos, so we use the Taylor-series-quality constants).
//
// Actually we compute them at runtime via lazy_static. For const context we
// store the values derived from the known constants.
//
// sin(1.578694°) = 0.027_543_218_...
// cos(1.578694°) = 0.999_620_614_...
// sin(107.582278°) = 0.953_419_...
// cos(107.582278°) = -0.301_630_...

/// Precomputed rotation matrix elements for ecliptic→invariable transform.
///
/// R = R3(-Ω) · R1(-i) · R3(Ω)
///
/// Expanded:
/// R = [[cos²Ω + sin²Ω·cos i,   sinΩ·cosΩ·(cos i - 1),  sinΩ·sin i],
///      [sinΩ·cosΩ·(cos i - 1),  sin²Ω + cos²Ω·cos i,   -cosΩ·sin i],
///      [-sinΩ·sin i,            cosΩ·sin i,               cos i      ]]
struct RotationMatrix {
    r: [[f64; 3]; 3],
}

impl RotationMatrix {
    fn compute() -> Self {
        let (sin_i, cos_i) = I_RAD.sin_cos();
        let (sin_o, cos_o) = OMEGA_RAD.sin_cos();

        let r00 = cos_o * cos_o + sin_o * sin_o * cos_i;
        let r01 = sin_o * cos_o * (1.0 - cos_i);
        let r02 = sin_o * sin_i;
        let r10 = sin_o * cos_o * (1.0 - cos_i);
        let r11 = sin_o * sin_o + cos_o * cos_o * cos_i;
        let r12 = -cos_o * sin_i;
        let r20 = -sin_o * sin_i;
        let r21 = cos_o * sin_i;
        let r22 = cos_i;

        Self {
            r: [[r00, r01, r02], [r10, r11, r12], [r20, r21, r22]],
        }
    }

    fn forward(&self, v: &[f64; 3]) -> [f64; 3] {
        [
            self.r[0][0] * v[0] + self.r[0][1] * v[1] + self.r[0][2] * v[2],
            self.r[1][0] * v[0] + self.r[1][1] * v[1] + self.r[1][2] * v[2],
            self.r[2][0] * v[0] + self.r[2][1] * v[1] + self.r[2][2] * v[2],
        ]
    }

    /// Inverse (transpose for orthogonal matrix).
    fn inverse(&self, v: &[f64; 3]) -> [f64; 3] {
        [
            self.r[0][0] * v[0] + self.r[1][0] * v[1] + self.r[2][0] * v[2],
            self.r[0][1] * v[0] + self.r[1][1] * v[1] + self.r[2][1] * v[2],
            self.r[0][2] * v[0] + self.r[1][2] * v[1] + self.r[2][2] * v[2],
        ]
    }
}

// Thread-safe lazy initialization of the rotation matrix.
use std::sync::OnceLock;
static ROT_MATRIX: OnceLock<RotationMatrix> = OnceLock::new();

fn rotation_matrix() -> &'static RotationMatrix {
    ROT_MATRIX.get_or_init(RotationMatrix::compute)
}

/// Rotate a 3-vector from ecliptic J2000 to the invariable plane.
///
/// The invariable plane is inclined ~1.58° to the ecliptic J2000.
/// This is a small rotation, so the matrix is near-identity.
#[inline]
pub fn ecliptic_to_invariable(v: &[f64; 3]) -> [f64; 3] {
    rotation_matrix().forward(v)
}

/// Rotate a 3-vector from the invariable plane to ecliptic J2000.
///
/// Inverse of [`ecliptic_to_invariable`] (transpose of the rotation matrix).
#[inline]
pub fn invariable_to_ecliptic(v: &[f64; 3]) -> [f64; 3] {
    rotation_matrix().inverse(v)
}

/// Rotate a 3-vector from ICRF/J2000 equatorial to the invariable plane.
///
/// Composed as: ICRF → ecliptic J2000 → invariable plane.
#[inline]
pub fn icrf_to_invariable(v: &[f64; 3]) -> [f64; 3] {
    ecliptic_to_invariable(&icrf_to_ecliptic(v))
}

/// Rotate a 3-vector from the invariable plane to ICRF/J2000 equatorial.
///
/// Composed as: invariable → ecliptic J2000 → ICRF.
#[inline]
pub fn invariable_to_icrf(v: &[f64; 3]) -> [f64; 3] {
    ecliptic_to_icrf(&invariable_to_ecliptic(v))
}

/// Dispatch ICRF→plane rotation based on the reference plane.
///
/// - `Ecliptic`: returns ecliptic J2000 coordinates (use precession separately).
/// - `Invariable`: returns invariable plane coordinates (no precession needed).
#[inline]
pub fn icrf_to_reference_plane(v: &[f64; 3], plane: ReferencePlane) -> [f64; 3] {
    match plane {
        ReferencePlane::Ecliptic => icrf_to_ecliptic(v),
        ReferencePlane::Invariable => icrf_to_invariable(v),
    }
}

/// Project an ecliptic longitude (lat=0 point on ecliptic) to invariable-plane longitude.
///
/// Useful for lagna and bhava cusps, which are inherently ecliptic quantities.
/// When using the invariable plane, their ecliptic longitudes must be projected
/// before subtracting the invariable-plane ayanamsha.
pub fn ecliptic_lon_to_invariable_lon(ecl_lon_deg: f64) -> f64 {
    let rad = ecl_lon_deg.to_radians();
    let ecl_vec = [rad.cos(), rad.sin(), 0.0];
    let inv_vec = ecliptic_to_invariable(&ecl_vec);
    inv_vec[1].atan2(inv_vec[0]).to_degrees().rem_euclid(360.0)
}

/// Project an invariable-plane longitude (lat=0 on invariable) to ecliptic longitude.
///
/// Inverse of [`ecliptic_lon_to_invariable_lon`]. Used to recover ecliptic
/// tropical longitude from an invariable-plane longitude for bhava matching
/// (bhava cusps are ecliptic quantities).
pub fn invariable_lon_to_ecliptic_lon(inv_lon_deg: f64) -> f64 {
    let rad = inv_lon_deg.to_radians();
    let inv_vec = [rad.cos(), rad.sin(), 0.0];
    let ecl_vec = invariable_to_ecliptic(&inv_vec);
    ecl_vec[1].atan2(ecl_vec[0]).to_degrees().rem_euclid(360.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    const EPS: f64 = 1e-12;

    #[test]
    fn rotation_preserves_magnitude() {
        let v: [f64; 3] = [1.234e8, -5.678e7, 9.012e6];
        let r_orig = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
        let inv = ecliptic_to_invariable(&v);
        let r_inv = (inv[0] * inv[0] + inv[1] * inv[1] + inv[2] * inv[2]).sqrt();
        assert!(
            (r_orig - r_inv).abs() < EPS * r_orig,
            "magnitude: orig={r_orig}, inv={r_inv}"
        );
    }

    #[test]
    fn roundtrip_ecliptic_invariable() {
        let v = [1.0e8, -5.0e7, 3.0e7];
        let inv = ecliptic_to_invariable(&v);
        let back = invariable_to_ecliptic(&inv);
        for i in 0..3 {
            assert!(
                (v[i] - back[i]).abs() < EPS * v[i].abs().max(1.0),
                "axis {i}: {:.15e} != {:.15e}",
                v[i],
                back[i]
            );
        }
    }

    #[test]
    fn roundtrip_icrf_invariable() {
        let v = [1.0, 0.5, -0.3];
        let inv = icrf_to_invariable(&v);
        let back = invariable_to_icrf(&inv);
        for i in 0..3 {
            assert!(
                (v[i] - back[i]).abs() < 1e-14,
                "axis {i}: {:.15e} != {:.15e}",
                v[i],
                back[i]
            );
        }
    }

    #[test]
    fn ecliptic_pole_to_invariable_latitude() {
        // Ecliptic north pole [0, 0, 1] should map to invariable lat ≈ 90° - 1.578694° = 88.421°
        let pole = [0.0, 0.0, 1.0];
        let inv = ecliptic_to_invariable(&pole);
        let r = (inv[0] * inv[0] + inv[1] * inv[1] + inv[2] * inv[2]).sqrt();
        let lat_deg = (inv[2] / r).asin().to_degrees();
        let expected = 90.0 - INVARIABLE_INCLINATION_DEG;
        assert!(
            (lat_deg - expected).abs() < 0.001,
            "ecliptic pole → invariable lat = {lat_deg}°, expected ~{expected}°"
        );
    }

    #[test]
    fn node_direction_lat_zero_on_both_planes() {
        // The ascending node direction [cos Ω, sin Ω, 0] lies on both planes.
        // Its latitude in the invariable frame should be ~0.
        let omega_rad = INVARIABLE_NODE_DEG.to_radians();
        let v = [omega_rad.cos(), omega_rad.sin(), 0.0];
        let inv = ecliptic_to_invariable(&v);
        let r = (inv[0] * inv[0] + inv[1] * inv[1] + inv[2] * inv[2]).sqrt();
        let lat_deg = (inv[2] / r).asin().to_degrees();
        assert!(
            lat_deg.abs() < 1e-10,
            "node direction → invariable lat = {lat_deg}°, expected ~0°"
        );
    }

    #[test]
    fn dispatch_ecliptic_matches_direct() {
        let v = [1.0, 2.0, 3.0];
        let via_dispatch = icrf_to_reference_plane(&v, ReferencePlane::Ecliptic);
        let direct = icrf_to_ecliptic(&v);
        for i in 0..3 {
            assert!((via_dispatch[i] - direct[i]).abs() < 1e-15);
        }
    }

    #[test]
    fn dispatch_invariable_matches_direct() {
        let v = [1.0, 2.0, 3.0];
        let via_dispatch = icrf_to_reference_plane(&v, ReferencePlane::Invariable);
        let direct = icrf_to_invariable(&v);
        for i in 0..3 {
            assert!((via_dispatch[i] - direct[i]).abs() < 1e-15);
        }
    }

    #[test]
    fn ecliptic_lon_to_invariable_lon_at_node() {
        // At the ascending node Ω, both planes intersect → longitude should be ~Ω
        let inv_lon = ecliptic_lon_to_invariable_lon(INVARIABLE_NODE_DEG);
        assert!(
            (inv_lon - INVARIABLE_NODE_DEG).abs() < 1e-8,
            "at node: inv_lon={inv_lon}°, expected ~{INVARIABLE_NODE_DEG}°"
        );
    }

    #[test]
    fn ecliptic_lon_to_invariable_lon_near_zero() {
        // At ecl_lon=0°, the offset is small (~1.58° tilt, longitude shift < 1°)
        let inv_lon = ecliptic_lon_to_invariable_lon(0.0);
        assert!(
            inv_lon.min(360.0 - inv_lon) < 1.0,
            "at 0°: inv_lon={inv_lon}° should be near 0° or 360°"
        );
    }

    #[test]
    fn ecliptic_lon_to_invariable_lon_monotonic() {
        // The mapping should be monotonically increasing over [0°, 360°)
        let n = 360;
        let mut prev = ecliptic_lon_to_invariable_lon(0.0);
        let mut wraps = 0;
        for i in 1..=n {
            let ecl = i as f64;
            let inv = ecliptic_lon_to_invariable_lon(ecl);
            if inv < prev - 180.0 {
                wraps += 1; // wrap from ~360° to ~0°
            }
            prev = inv;
        }
        // Should wrap exactly once (like any monotonic mapping on [0°,360°))
        assert_eq!(wraps, 1, "expected exactly 1 wrap, got {wraps}");
    }

    #[test]
    fn ecliptic_lon_to_invariable_lon_range() {
        // Output should always be in [0°, 360°)
        for i in 0..360 {
            let inv = ecliptic_lon_to_invariable_lon(i as f64);
            assert!(
                (0.0..360.0).contains(&inv),
                "ecl={i}°: inv_lon={inv}° out of range"
            );
        }
    }

    #[test]
    fn default_reference_plane_is_ecliptic() {
        assert_eq!(ReferencePlane::default(), ReferencePlane::Ecliptic);
    }

    #[test]
    fn lon_roundtrip_ecl_inv_ecl() {
        // ecliptic → invariable → ecliptic roundtrip.  Not exact because
        // each step projects to lat=0 on its plane (discarding the small
        // out-of-plane component).  With ~1.58° tilt the worst-case error
        // is ~0.022° (near the antinodes, 90° from Ω).
        for deg in (0..360).step_by(15) {
            let ecl = deg as f64;
            let inv = ecliptic_lon_to_invariable_lon(ecl);
            let back = invariable_lon_to_ecliptic_lon(inv);
            let diff = (back - ecl + 180.0).rem_euclid(360.0) - 180.0;
            assert!(
                diff.abs() < 0.025,
                "ecl={ecl}° → inv={inv}° → ecl={back}°, diff={diff:.6}°"
            );
        }
    }

    #[test]
    fn invariable_lon_to_ecliptic_lon_at_node() {
        // At the node, both planes intersect → longitude preserved
        let ecl = invariable_lon_to_ecliptic_lon(INVARIABLE_NODE_DEG);
        assert!(
            (ecl - INVARIABLE_NODE_DEG).abs() < 1e-8,
            "at node: ecl={ecl}°, expected ~{INVARIABLE_NODE_DEG}°"
        );
    }

    #[test]
    fn near_identity_for_small_inclination() {
        // Since inclination is only ~1.58°, the rotation should be near-identity.
        // For a vector with no z-component, the output should be very close.
        let v = [1.0, 0.0, 0.0];
        let inv = ecliptic_to_invariable(&v);
        assert!(
            (inv[0] - 1.0).abs() < 0.001,
            "x should be near 1.0, got {}",
            inv[0]
        );
        assert!(
            inv[1].abs() < 0.001,
            "y should be near 0.0, got {}",
            inv[1]
        );
        assert!(
            inv[2].abs() < 0.03,
            "z should be small, got {}",
            inv[2]
        );
    }
}
