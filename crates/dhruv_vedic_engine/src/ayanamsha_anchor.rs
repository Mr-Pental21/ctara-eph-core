//! Star-anchored ayanamsha helpers.
//!
//! These models compute ayanamsha from an anchor point that should stay at a
//! fixed sidereal longitude, instead of using a fixed J2000 offset.

use crate::ayanamsha::AyanamshaSystem;
use dhruv_frames::{
    PrecessionModel, ReferencePlane, cartesian_to_spherical, ecliptic_to_invariable,
    precess_ecliptic_j2000_to_date_with_model,
};
use dhruv_vedic_math::normalize_360;

#[derive(Debug, Clone, Copy)]
struct AnchorSpec {
    /// Anchor longitude in J2000 ecliptic degrees.
    lon_j2000_deg: f64,
    /// Anchor latitude in J2000 ecliptic degrees.
    lat_j2000_deg: f64,
    /// Target sidereal longitude that the anchor should keep.
    target_sidereal_lon_deg: f64,
}

fn anchor_spec(system: AyanamshaSystem) -> Option<AnchorSpec> {
    match system {
        // Lahiri (MEAN anchor): IAE gazette 23°15'00.658" at 1956-03-21
        // 00:00 TDT minus IAU 2000B nutation at that epoch (Δψ ≈ 16.78"),
        // then back-precessed to J2000 via 3D Vondrák precession.
        // The small lat tracks the ecliptic tilt between 1956 and J2000.
        AyanamshaSystem::Lahiri => Some(AnchorSpec {
            lon_j2000_deg: 23.857_052_898_247_307,
            lat_j2000_deg: 0.002_727_754_076_653,
            target_sidereal_lon_deg: 0.0,
        }),
        // Spica anchor. J2000 ecliptic from HGCA J2016.0 catalog propagated
        // back to J2000 (-16yr) via Butkevich & Lindegren proper motion,
        // then ICRS→ecliptic via IAU 2006 obliquity.
        // Derivation: ayanamsha_anchor::tests::derive_anchor_coordinates_from_catalog
        AyanamshaSystem::TrueLahiri => Some(AnchorSpec {
            lon_j2000_deg: 203.841_363,
            lat_j2000_deg: -2.054_491,
            target_sidereal_lon_deg: 180.0,
        }),
        // Pushya anchor: delta Cancri (Asellus Australis, HIP 42911) at 106° sidereal.
        // J2000 ecliptic from HGCA J2016.0 (RA 131.171°, Dec 18.153°) propagated
        // back to J2000 via proper motion, then converted to ecliptic coordinates.
        AyanamshaSystem::PushyaPaksha => Some(AnchorSpec {
            lon_j2000_deg: 128.722,
            lat_j2000_deg: 0.076,
            target_sidereal_lon_deg: 106.0,
        }),
        // Aldebaran anchor at 16°40' Taurus, matching the published
        // Rohini-paksha definition used by P. V. R. Narasimha Rao.
        // J2000 ecliptic from HGCA J2016.0 catalog, same derivation as TrueLahiri.
        AyanamshaSystem::RohiniPaksha => Some(AnchorSpec {
            lon_j2000_deg: 69.789_181,
            lat_j2000_deg: -5.467_329,
            target_sidereal_lon_deg: 46.666_667,
        }),
        // Aldebaran anchor at 15° Taurus.
        // Same Aldebaran J2000 ecliptic coordinates.
        AyanamshaSystem::Aldebaran15Tau => Some(AnchorSpec {
            lon_j2000_deg: 69.789_181,
            lat_j2000_deg: -5.467_329,
            target_sidereal_lon_deg: 45.0,
        }),
        // Galactic Center at 0° Sagittarius sidereal (240° sidereal longitude).
        // J2000 ecliptic coords computed from IAU 2000 GC ICRS direction
        // (α≈266.405°, δ≈-28.936°) via IAU 2006 obliquity rotation.
        AyanamshaSystem::GalacticCenter0Sag => Some(AnchorSpec {
            lon_j2000_deg: 266.839_617,
            lat_j2000_deg: -5.536_308,
            target_sidereal_lon_deg: 240.0,
        }),
        // Chandra Hari: λ Scorpii (Mula yogatara) at 0° Sagittarius sidereal.
        // J2000 ecliptic coords from SIMBAD ICRS (α=263.402°, δ=-37.104°)
        // via IAU 2006 obliquity rotation.
        AyanamshaSystem::ChandraHari => Some(AnchorSpec {
            lon_j2000_deg: 264.585_715,
            lat_j2000_deg: -13.788_451,
            target_sidereal_lon_deg: 240.0,
        }),
        // Jagganatha: Spica (Chitra) at 180° sidereal on the invariable plane.
        // Same J2000 ecliptic coords as TrueLahiri anchor (Spica from HGCA catalog).
        AyanamshaSystem::Jagganatha => Some(AnchorSpec {
            lon_j2000_deg: 203.841_363,
            lat_j2000_deg: -2.054_491,
            target_sidereal_lon_deg: 180.0,
        }),
        _ => None,
    }
}

fn anchor_tropical_longitude_deg(
    spec: AnchorSpec,
    t_centuries: f64,
    model: PrecessionModel,
) -> f64 {
    let lon = spec.lon_j2000_deg.to_radians();
    let lat = spec.lat_j2000_deg.to_radians();
    let v = [lat.cos() * lon.cos(), lat.cos() * lon.sin(), lat.sin()];
    let v_date = precess_ecliptic_j2000_to_date_with_model(&v, t_centuries, model);
    cartesian_to_spherical(&v_date).lon_deg
}

/// Star-relative ayanamsha for systems that are defined by anchor locking.
pub(crate) fn anchor_relative_ayanamsha_deg(
    system: AyanamshaSystem,
    t_centuries: f64,
    model: PrecessionModel,
) -> Option<f64> {
    let spec = anchor_spec(system)?;
    let anchor_lon = anchor_tropical_longitude_deg(spec, t_centuries, model);
    Some(normalize_360(anchor_lon - spec.target_sidereal_lon_deg))
}

/// Plane-aware anchor tropical longitude.
///
/// - `Ecliptic`: precess J2000 ecliptic coords to ecliptic-of-date (existing path).
/// - `Invariable`: project J2000 ecliptic coords to invariable plane (no precession).
fn anchor_tropical_longitude_deg_on_plane(
    spec: AnchorSpec,
    t_centuries: f64,
    model: PrecessionModel,
    plane: ReferencePlane,
) -> f64 {
    match plane {
        ReferencePlane::Ecliptic => anchor_tropical_longitude_deg(spec, t_centuries, model),
        ReferencePlane::Invariable => {
            let lon = spec.lon_j2000_deg.to_radians();
            let lat = spec.lat_j2000_deg.to_radians();
            let v = [lat.cos() * lon.cos(), lat.cos() * lon.sin(), lat.sin()];
            let inv = ecliptic_to_invariable(&v);
            cartesian_to_spherical(&inv).lon_deg
        }
    }
}

/// Plane-aware star-relative ayanamsha.
pub(crate) fn anchor_relative_ayanamsha_deg_on_plane(
    system: AyanamshaSystem,
    t_centuries: f64,
    model: PrecessionModel,
    plane: ReferencePlane,
) -> Option<f64> {
    let spec = anchor_spec(system)?;
    let anchor_lon = anchor_tropical_longitude_deg_on_plane(spec, t_centuries, model, plane);
    Some(normalize_360(anchor_lon - spec.target_sidereal_lon_deg))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Documents the derivation of AnchorSpec hardcoded values.
    /// Pipeline: HGCA J2016.0 catalog → propagate to J2000 (-16yr) → ICRS→ecliptic J2000.
    #[test]
    fn derive_anchor_coordinates_from_catalog() {
        use dhruv_tara::{TaraCatalog, TaraId, position_ecliptic};

        let cat = TaraCatalog::embedded();
        let j2000_jd = 2_451_545.0;

        // Verify each anchor star's hardcoded value matches catalog to <0.001° (3.6")
        let ecl = position_ecliptic(cat, TaraId::Chitra, j2000_jd).unwrap();
        let spec = anchor_spec(AyanamshaSystem::TrueLahiri).unwrap();
        assert!(
            (ecl.lon_deg - spec.lon_j2000_deg).abs() < 0.001,
            "Spica lon: catalog={:.6}, anchor={:.6}",
            ecl.lon_deg,
            spec.lon_j2000_deg
        );
        assert!(
            (ecl.lat_deg - spec.lat_j2000_deg).abs() < 0.001,
            "Spica lat: catalog={:.6}, anchor={:.6}",
            ecl.lat_deg,
            spec.lat_j2000_deg
        );

        let ecl = position_ecliptic(cat, TaraId::Aldebaran, j2000_jd).unwrap();
        let spec = anchor_spec(AyanamshaSystem::RohiniPaksha).unwrap();
        assert!(
            (ecl.lon_deg - spec.lon_j2000_deg).abs() < 0.001,
            "Aldebaran lon: catalog={:.6}, anchor={:.6}",
            ecl.lon_deg,
            spec.lon_j2000_deg
        );

        let ecl = position_ecliptic(cat, TaraId::DeltaCnc, j2000_jd).unwrap();
        let spec = anchor_spec(AyanamshaSystem::PushyaPaksha).unwrap();
        assert!(
            (ecl.lon_deg - spec.lon_j2000_deg).abs() < 0.001,
            "DeltaCnc lon: catalog={:.6}, anchor={:.6}",
            ecl.lon_deg,
            spec.lon_j2000_deg
        );

        let ecl = position_ecliptic(cat, TaraId::LambdaSco, j2000_jd).unwrap();
        let spec = anchor_spec(AyanamshaSystem::ChandraHari).unwrap();
        assert!(
            (ecl.lon_deg - spec.lon_j2000_deg).abs() < 0.001,
            "LambdaSco lon: catalog={:.6}, anchor={:.6}",
            ecl.lon_deg,
            spec.lon_j2000_deg
        );
    }

    #[test]
    fn lahiri_anchor_and_reference_consistent() {
        let spec = anchor_spec(AyanamshaSystem::Lahiri).unwrap();
        let ref_deg = AyanamshaSystem::Lahiri.reference_j2000_deg();
        assert!(
            (spec.lon_j2000_deg - ref_deg).abs() < 1e-15,
            "anchor lon={}, reference={}",
            spec.lon_j2000_deg,
            ref_deg
        );
    }

    #[test]
    fn converted_systems_are_anchor_relative() {
        for &sys in &[
            AyanamshaSystem::TrueLahiri,
            AyanamshaSystem::PushyaPaksha,
            AyanamshaSystem::RohiniPaksha,
            AyanamshaSystem::Aldebaran15Tau,
            AyanamshaSystem::GalacticCenter0Sag,
            AyanamshaSystem::ChandraHari,
        ] {
            assert!(
                anchor_spec(sys).is_some(),
                "{sys:?} should have anchor spec"
            );
        }
    }

    #[test]
    fn anchor_lock_invariant_true_lahiri() {
        let spec = anchor_spec(AyanamshaSystem::TrueLahiri).unwrap();
        for t in [-2.0, -1.0, 0.0, 0.5, 1.0, 2.0] {
            let aya = anchor_relative_ayanamsha_deg(
                AyanamshaSystem::TrueLahiri,
                t,
                PrecessionModel::Iau2006,
            )
            .unwrap();
            let anchor_lon = anchor_tropical_longitude_deg(spec, t, PrecessionModel::Iau2006);
            let sid = normalize_360(anchor_lon - aya);
            assert!(
                (sid - spec.target_sidereal_lon_deg).abs() < 1e-9,
                "t={t}: sid={sid}"
            );
        }
    }

    #[test]
    fn model_parameter_is_wired() {
        let t = 25.0;
        let a =
            anchor_relative_ayanamsha_deg(AyanamshaSystem::TrueLahiri, t, PrecessionModel::Iau2006)
                .unwrap();
        let b = anchor_relative_ayanamsha_deg(
            AyanamshaSystem::TrueLahiri,
            t,
            PrecessionModel::Vondrak2011,
        )
        .unwrap();
        assert!((a - b).abs() > 1e-6);
    }
}
