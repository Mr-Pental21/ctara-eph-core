//! Star-ephemeris ayanamsha integration.
//!
//! Replaces static J2000 ecliptic coordinates with dynamic,
//! proper-motion-corrected star positions from `dhruv_tara`.

use crate::ayanamsha::AyanamshaSystem;
use dhruv_frames::{
    PrecessionModel, ReferencePlane, cartesian_to_spherical, icrf_to_ecliptic, icrf_to_invariable,
    precess_ecliptic_j2000_to_date_with_model,
};
use dhruv_tara::{TaraCatalog, TaraId, galactic_center_icrs};
use dhruv_vedic_math::normalize_360;

/// Anchor specification mapping an AyanamshaSystem to a TaraId.
pub(crate) struct TaraAnchorSpec {
    pub tara_id: TaraId,
    pub target_sidereal_lon_deg: f64,
}

/// Maps star-anchored ayanamsha systems to their TaraId + target sidereal longitude.
///
/// Returns `None` for systems that are not star-anchored (Lahiri uses a gazette epoch
/// anchor, not a direct star lookup).
pub(crate) fn anchor_tara_spec(system: AyanamshaSystem) -> Option<TaraAnchorSpec> {
    match system {
        AyanamshaSystem::TrueLahiri => Some(TaraAnchorSpec {
            tara_id: TaraId::Chitra,
            target_sidereal_lon_deg: 180.0,
        }),
        AyanamshaSystem::PushyaPaksha => Some(TaraAnchorSpec {
            tara_id: TaraId::DeltaCnc,
            target_sidereal_lon_deg: 106.0,
        }),
        AyanamshaSystem::RohiniPaksha => Some(TaraAnchorSpec {
            tara_id: TaraId::Aldebaran,
            target_sidereal_lon_deg: 45.783_333,
        }),
        AyanamshaSystem::Aldebaran15Tau => Some(TaraAnchorSpec {
            tara_id: TaraId::Aldebaran,
            target_sidereal_lon_deg: 45.0,
        }),
        AyanamshaSystem::GalacticCenter0Sag => Some(TaraAnchorSpec {
            tara_id: TaraId::GalacticCenter,
            target_sidereal_lon_deg: 240.0,
        }),
        AyanamshaSystem::ChandraHari => Some(TaraAnchorSpec {
            tara_id: TaraId::LambdaSco,
            target_sidereal_lon_deg: 240.0,
        }),
        // Jagganatha: Spica (Chitra) at 180° sidereal on invariable plane.
        AyanamshaSystem::Jagganatha => Some(TaraAnchorSpec {
            tara_id: TaraId::Chitra,
            target_sidereal_lon_deg: 180.0,
        }),
        _ => None,
    }
}

/// Compute the mean tropical ecliptic longitude of a star at the given epoch.
///
/// Pipeline: catalog entry → propagate ICRS with PM → ICRS→ecliptic J2000 →
/// precess to ecliptic of date → read longitude.
///
/// Uses the caller's precession model for consistency. No nutation applied.
fn star_tropical_longitude_deg(
    catalog: &TaraCatalog,
    spec: &TaraAnchorSpec,
    t_centuries: f64,
    model: PrecessionModel,
) -> Option<f64> {
    // Galactic Center: fixed ICRS direction (no proper motion, no catalog lookup)
    if spec.tara_id == TaraId::GalacticCenter {
        let icrs = galactic_center_icrs();
        return Some(icrs_to_tropical_longitude(&icrs, t_centuries, model));
    }

    let entry = catalog.get(spec.tara_id)?;

    // Compute dt from catalog reference epoch to target epoch
    let j2000_jd = 2_451_545.0;
    let days_per_year = 365.25;
    let epoch_jd = j2000_jd + (catalog.reference_epoch_jy - 2000.0) * days_per_year;
    let target_jd = j2000_jd + t_centuries * 36525.0;
    let dt_years = (target_jd - epoch_jd) / days_per_year;

    // Propagate ICRS position with proper motion
    let icrs_pos = dhruv_tara::propagation::propagate_cartesian_au(
        entry.ra_deg,
        entry.dec_deg,
        entry.parallax_mas,
        entry.pm_ra_mas_yr,
        entry.pm_dec_mas_yr,
        entry.radial_velocity_km_s,
        dt_years,
    );

    // Normalize to unit vector
    let r =
        (icrs_pos[0] * icrs_pos[0] + icrs_pos[1] * icrs_pos[1] + icrs_pos[2] * icrs_pos[2]).sqrt();
    if r == 0.0 {
        return None;
    }
    let unit = [icrs_pos[0] / r, icrs_pos[1] / r, icrs_pos[2] / r];

    Some(icrs_to_tropical_longitude(&unit, t_centuries, model))
}

/// Convert an ICRS unit direction vector to tropical ecliptic longitude at the given epoch.
fn icrs_to_tropical_longitude(
    icrs_unit: &[f64; 3],
    t_centuries: f64,
    model: PrecessionModel,
) -> f64 {
    let ecl_j2000 = icrf_to_ecliptic(icrs_unit);
    let ecl_of_date = precess_ecliptic_j2000_to_date_with_model(&ecl_j2000, t_centuries, model);
    cartesian_to_spherical(&ecl_of_date).lon_deg
}

/// Compute star-ephemeris ayanamsha for systems with catalog star anchors.
///
/// Returns `Some(ayanamsha_deg)` if:
/// - The system has a tara anchor spec
/// - The star is found in the catalog (or is Galactic Center)
///
/// Returns `None` otherwise (caller falls back to static path).
pub(crate) fn tara_anchor_ayanamsha_deg(
    system: AyanamshaSystem,
    t_centuries: f64,
    model: PrecessionModel,
    catalog: &TaraCatalog,
) -> Option<f64> {
    let spec = anchor_tara_spec(system)?;
    let star_lon = star_tropical_longitude_deg(catalog, &spec, t_centuries, model)?;
    Some(normalize_360(star_lon - spec.target_sidereal_lon_deg))
}

/// Convert an ICRS unit direction to longitude on the specified reference plane.
fn icrs_to_longitude_on_plane(
    icrs_unit: &[f64; 3],
    t_centuries: f64,
    model: PrecessionModel,
    plane: ReferencePlane,
) -> f64 {
    match plane {
        ReferencePlane::Ecliptic => icrs_to_tropical_longitude(icrs_unit, t_centuries, model),
        ReferencePlane::Invariable => {
            let inv = icrf_to_invariable(icrs_unit);
            cartesian_to_spherical(&inv).lon_deg
        }
    }
}

/// Compute star longitude on the specified reference plane.
fn star_longitude_deg_on_plane(
    catalog: &TaraCatalog,
    spec: &TaraAnchorSpec,
    t_centuries: f64,
    model: PrecessionModel,
    plane: ReferencePlane,
) -> Option<f64> {
    if spec.tara_id == TaraId::GalacticCenter {
        let icrs = galactic_center_icrs();
        return Some(icrs_to_longitude_on_plane(&icrs, t_centuries, model, plane));
    }

    let entry = catalog.get(spec.tara_id)?;
    let j2000_jd = 2_451_545.0;
    let days_per_year = 365.25;
    let epoch_jd = j2000_jd + (catalog.reference_epoch_jy - 2000.0) * days_per_year;
    let target_jd = j2000_jd + t_centuries * 36525.0;
    let dt_years = (target_jd - epoch_jd) / days_per_year;

    let icrs_pos = dhruv_tara::propagation::propagate_cartesian_au(
        entry.ra_deg,
        entry.dec_deg,
        entry.parallax_mas,
        entry.pm_ra_mas_yr,
        entry.pm_dec_mas_yr,
        entry.radial_velocity_km_s,
        dt_years,
    );

    let r =
        (icrs_pos[0] * icrs_pos[0] + icrs_pos[1] * icrs_pos[1] + icrs_pos[2] * icrs_pos[2]).sqrt();
    if r == 0.0 {
        return None;
    }
    let unit = [icrs_pos[0] / r, icrs_pos[1] / r, icrs_pos[2] / r];

    Some(icrs_to_longitude_on_plane(&unit, t_centuries, model, plane))
}

/// Plane-aware star-ephemeris ayanamsha.
pub(crate) fn tara_anchor_ayanamsha_deg_on_plane(
    system: AyanamshaSystem,
    t_centuries: f64,
    model: PrecessionModel,
    catalog: &TaraCatalog,
    plane: ReferencePlane,
) -> Option<f64> {
    let spec = anchor_tara_spec(system)?;
    let star_lon = star_longitude_deg_on_plane(catalog, &spec, t_centuries, model, plane)?;
    Some(normalize_360(star_lon - spec.target_sidereal_lon_deg))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_star_anchored_systems_have_spec() {
        let systems = [
            AyanamshaSystem::TrueLahiri,
            AyanamshaSystem::PushyaPaksha,
            AyanamshaSystem::RohiniPaksha,
            AyanamshaSystem::Aldebaran15Tau,
            AyanamshaSystem::GalacticCenter0Sag,
            AyanamshaSystem::ChandraHari,
        ];
        for sys in systems {
            assert!(
                anchor_tara_spec(sys).is_some(),
                "{sys:?} should have tara anchor spec"
            );
        }
    }

    #[test]
    fn lahiri_has_no_tara_spec() {
        // Lahiri uses gazette epoch anchor, not a star
        assert!(anchor_tara_spec(AyanamshaSystem::Lahiri).is_none());
    }

    #[test]
    fn non_anchor_systems_have_no_tara_spec() {
        let non_anchor = [
            AyanamshaSystem::KP,
            AyanamshaSystem::Raman,
            AyanamshaSystem::FaganBradley,
            AyanamshaSystem::DeLuce,
            AyanamshaSystem::Yukteshwar,
        ];
        for sys in non_anchor {
            assert!(
                anchor_tara_spec(sys).is_none(),
                "{sys:?} should NOT have tara anchor spec"
            );
        }
    }

    #[test]
    fn gc_tropical_longitude_at_j2000() {
        // GC ecliptic longitude at J2000 should be ~266.84°
        let icrs = galactic_center_icrs();
        let lon = icrs_to_tropical_longitude(&icrs, 0.0, PrecessionModel::Iau2006);
        assert!(
            (lon - 266.84).abs() < 0.1,
            "GC J2000 ecliptic lon: {lon}° (expected ~266.84°)"
        );
    }
}
