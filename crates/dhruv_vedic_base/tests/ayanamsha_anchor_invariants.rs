//! Invariants for anchor-relative ayanamsha systems.
//!
//! Uses the static (no-catalog) path to verify the anchor-lock identity:
//! sidereal_lon(anchor) = target for all epochs.

use dhruv_frames::{cartesian_to_spherical, precess_ecliptic_j2000_to_date};
use dhruv_vedic_base::{
    AyanamshaSystem, ayanamsha_deg_static, ayanamsha_mean_deg_static, normalize_360,
};

#[derive(Clone, Copy)]
struct AnchorCase {
    system: AyanamshaSystem,
    lon_j2000_deg: f64,
    lat_j2000_deg: f64,
    target_sidereal_lon_deg: f64,
}

fn anchor_tropical_lon_deg(lon_j2000_deg: f64, lat_j2000_deg: f64, t_centuries: f64) -> f64 {
    let lon = lon_j2000_deg.to_radians();
    let lat = lat_j2000_deg.to_radians();
    let v = [lat.cos() * lon.cos(), lat.cos() * lon.sin(), lat.sin()];
    let v_date = precess_ecliptic_j2000_to_date(&v, t_centuries);
    cartesian_to_spherical(&v_date).lon_deg
}

#[test]
fn anchor_relative_systems_keep_anchor_sidereal_longitude_fixed() {
    let cases = [
        AnchorCase {
            system: AyanamshaSystem::TrueLahiri,
            lon_j2000_deg: 203.841_363,
            lat_j2000_deg: -2.054_491,
            target_sidereal_lon_deg: 180.0,
        },
        AnchorCase {
            system: AyanamshaSystem::PushyaPaksha,
            lon_j2000_deg: 128.722,
            lat_j2000_deg: 0.076,
            target_sidereal_lon_deg: 106.0,
        },
        AnchorCase {
            system: AyanamshaSystem::RohiniPaksha,
            lon_j2000_deg: 69.789_181,
            lat_j2000_deg: -5.467_329,
            target_sidereal_lon_deg: 46.666_667,
        },
        AnchorCase {
            system: AyanamshaSystem::Aldebaran15Tau,
            lon_j2000_deg: 69.789_181,
            lat_j2000_deg: -5.467_329,
            target_sidereal_lon_deg: 45.0,
        },
    ];

    for c in cases {
        for t in [-2.0, -1.0, -0.5, 0.0, 0.5, 1.0, 2.0] {
            let aya = ayanamsha_mean_deg_static(c.system, t);
            let anchor_lon = anchor_tropical_lon_deg(c.lon_j2000_deg, c.lat_j2000_deg, t);
            let sid = normalize_360(anchor_lon - aya);
            assert!(
                (sid - c.target_sidereal_lon_deg).abs() < 1e-9,
                "{:?} t={t}: sid={sid}, target={}",
                c.system,
                c.target_sidereal_lon_deg
            );
        }
    }
}

#[test]
fn nutation_toggle_affects_anchor_systems() {
    let t = 0.24;
    let with_nut = ayanamsha_deg_static(AyanamshaSystem::TrueLahiri, t, true);
    let without_nut = ayanamsha_deg_static(AyanamshaSystem::TrueLahiri, t, false);
    let (dpsi_arcsec, _) = dhruv_frames::nutation_iau2000b(t);
    let expected_diff = dpsi_arcsec / 3600.0;
    assert!(
        (with_nut - without_nut - expected_diff).abs() < 1e-10,
        "diff={}, expected={}",
        with_nut - without_nut,
        expected_diff
    );
}
