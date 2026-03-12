//! Ayanamsha computation for 20 sidereal reference systems.
//!
//! The ayanamsha is the angular offset between the tropical zodiac (defined
//! by the vernal equinox) and a sidereal zodiac (anchored to fixed stars).
//! As the equinox precesses westward, the ayanamsha increases over time.
//!
//! Each system is defined by its J2000.0 reference value (the J2000 ecliptic
//! longitude of the sidereal zero point). The ayanamsha at any epoch is
//! computed by precessing that direction to the ecliptic-of-date using the
//! full 3D ecliptic precession matrix and reading off the longitude.
//!
//! Clean-room implementation: all reference values derived independently from
//! published system definitions. See `docs/clean_room_ayanamsha.md`.

use crate::ayanamsha_anchor::{
    anchor_relative_ayanamsha_deg, anchor_relative_ayanamsha_deg_on_plane,
};
use crate::ayanamsha_tara::{tara_anchor_ayanamsha_deg, tara_anchor_ayanamsha_deg_on_plane};
use dhruv_frames::{
    DEFAULT_PRECESSION_MODEL, PrecessionModel, ReferencePlane, nutation_iau2000b,
    precess_ecliptic_j2000_to_date_with_model,
};
use dhruv_tara::TaraCatalog;
use dhruv_time::J2000_JD;

/// Sidereal reference systems for ayanamsha computation.
///
/// Each variant defines a different convention for anchoring the sidereal
/// zodiac to the fixed stars. The differences reduce to a single parameter:
/// the ayanamsha value at J2000.0.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AyanamshaSystem {
    /// Lahiri (Chitrapaksha): Spica at 0 Libra sidereal.
    /// Indian government standard (Calendar Reform Committee, 1957).
    Lahiri,

    /// True Lahiri: same anchor as Lahiri, but uses the true
    /// (nutation-corrected) equinox instead of the mean equinox.
    TrueLahiri,

    /// Krishnamurti Paddhati (KP): sub-lord system, minimal offset from Lahiri.
    KP,

    /// B.V. Raman: from "Hindu Predictive Astrology".
    /// Zero ayanamsha year approximately 397 CE.
    Raman,

    /// Fagan-Bradley: primary Western sidereal system.
    /// Synetic Vernal Point calibrated by Cyril Fagan and Donald Bradley.
    FaganBradley,

    /// Pushya Paksha: delta Cancri (Pushya) at 16 deg Cancer (106 deg sidereal).
    PushyaPaksha,

    /// Rohini Paksha: Aldebaran at 15 deg 47 min Taurus.
    RohiniPaksha,

    /// Robert DeLuce ayanamsha (1930s).
    DeLuce,

    /// Djwal Khul: esoteric astrology (Alice Bailey tradition).
    DjwalKhul,

    /// Hipparchos: derived from Hipparchus observations (~128 BCE).
    Hipparchos,

    /// Sassanian: Sassanid-era Persian astronomical tradition.
    Sassanian,

    /// Deva-Dutta ayanamsha.
    DevaDutta,

    /// Usha-Shashi ayanamsha.
    UshaShashi,

    /// Sri Yukteshwar: from "The Holy Science" (1894).
    Yukteshwar,

    /// J.N. Bhasin ayanamsha.
    JnBhasin,

    /// Chandra Hari ayanamsha.
    ChandraHari,

    /// Jagganatha ayanamsha.
    Jagganatha,

    /// Surya Siddhanta: ancient Indian treatise.
    /// Uses IAU precession for consistency (not traditional 54 arcsec/yr).
    SuryaSiddhanta,

    /// Galactic Center at 0 deg Sagittarius sidereal.
    GalacticCenter0Sag,

    /// Aldebaran at 15 deg Taurus sidereal.
    Aldebaran15Tau,
}

/// All 20 ayanamsha systems in enum order.
const ALL_SYSTEMS: [AyanamshaSystem; 20] = [
    AyanamshaSystem::Lahiri,
    AyanamshaSystem::TrueLahiri,
    AyanamshaSystem::KP,
    AyanamshaSystem::Raman,
    AyanamshaSystem::FaganBradley,
    AyanamshaSystem::PushyaPaksha,
    AyanamshaSystem::RohiniPaksha,
    AyanamshaSystem::DeLuce,
    AyanamshaSystem::DjwalKhul,
    AyanamshaSystem::Hipparchos,
    AyanamshaSystem::Sassanian,
    AyanamshaSystem::DevaDutta,
    AyanamshaSystem::UshaShashi,
    AyanamshaSystem::Yukteshwar,
    AyanamshaSystem::JnBhasin,
    AyanamshaSystem::ChandraHari,
    AyanamshaSystem::Jagganatha,
    AyanamshaSystem::SuryaSiddhanta,
    AyanamshaSystem::GalacticCenter0Sag,
    AyanamshaSystem::Aldebaran15Tau,
];

impl AyanamshaSystem {
    /// Reference ayanamsha at J2000.0 in degrees.
    ///
    /// Each value is independently derived from the system's published
    /// definition (star anchor or zero-ayanamsha epoch). See
    /// `docs/clean_room_ayanamsha.md` for derivation details.
    pub const fn reference_j2000_deg(self) -> f64 {
        match self {
            // MEAN anchor: IAE gazette 23°15'00.658" minus IAU 2000B nutation
            // at 1956-03-21, back-computed to J2000 via 3D Vondrák precession.
            // Must stay synchronized with anchor_spec(Lahiri).lon_j2000_deg.
            Self::Lahiri => 23.857_052_898_247_307,
            // Same mean anchor baseline as Lahiri; nutation applied separately.
            Self::TrueLahiri => 23.857_052_898_247_307,
            // Krishnamurti: minimal offset from Lahiri
            Self::KP => 23.850,
            // B.V. Raman: zero year ~397 CE
            Self::Raman => 22.370,
            // Fagan-Bradley SVP calibration
            Self::FaganBradley => 24.736,
            // delta Cancri (Asellus Australis) at 106 deg sidereal
            // 128.722 (J2000 ecl lon) - 106.0 = 22.722
            Self::PushyaPaksha => 22.722,
            // Aldebaran at 15 deg 47 min Taurus (69.789 - 45.783)
            Self::RohiniPaksha => 24.006,
            // Robert DeLuce
            Self::DeLuce => 21.619,
            // Esoteric/Bailey tradition
            Self::DjwalKhul => 22.883,
            // Hipparchus ~128 BCE
            Self::Hipparchos => 21.176,
            // Sassanid Persian tradition
            Self::Sassanian => 19.765,
            // Deva-Dutta
            Self::DevaDutta => 22.474,
            // Usha-Shashi
            Self::UshaShashi => 20.103,
            // Sri Yukteshwar, "The Holy Science"
            Self::Yukteshwar => 22.376,
            // J.N. Bhasin
            Self::JnBhasin => 22.376,
            // Chandra Hari: λ Sco at 240° sidereal → J2000 ecl lon 264.586 - 240.0
            Self::ChandraHari => 24.586,
            // Jagganatha: Spica at 180° sidereal on invariable plane.
            // Spica J2000 ecliptic lon ≈ 203.841° (HGCA catalog), target 180°.
            // Fallback reference ≈ 203.841 - 180.0 = 23.841 (ecliptic approximation).
            Self::Jagganatha => 23.841,
            // Surya Siddhanta (IAU precession back-computed)
            Self::SuryaSiddhanta => 22.459,
            // Galactic Center at 0° Sag: J2000 ecl lon 266.840 - 240.0
            Self::GalacticCenter0Sag => 26.840,
            // Aldebaran at 15 deg Taurus (69.789 - 45.0)
            Self::Aldebaran15Tau => 24.789,
        }
    }

    /// Whether this system is computed by locking an anchor to a sidereal longitude.
    ///
    /// Anchor-relative systems do not use the legacy "reference + precession" model.
    pub const fn is_anchor_relative(self) -> bool {
        matches!(
            self,
            Self::Lahiri
                | Self::TrueLahiri
                | Self::PushyaPaksha
                | Self::RohiniPaksha
                | Self::Aldebaran15Tau
                | Self::GalacticCenter0Sag
                | Self::ChandraHari
                | Self::Jagganatha
        )
    }

    /// Default reference plane for this ayanamsha system.
    ///
    /// Most systems use the ecliptic. Jagganatha uses the invariable plane,
    /// measuring all longitudes on the plane perpendicular to the solar
    /// system's angular momentum vector.
    pub const fn default_reference_plane(self) -> ReferencePlane {
        match self {
            Self::Jagganatha => ReferencePlane::Invariable,
            _ => ReferencePlane::Ecliptic,
        }
    }

    /// All 20 defined ayanamsha systems.
    pub const fn all() -> &'static [AyanamshaSystem] {
        &ALL_SYSTEMS
    }
}

/// Mean ayanamsha in degrees at a given epoch.
///
/// # Arguments
/// * `system` — the sidereal reference system
/// * `t_centuries` — Julian centuries of TDB since J2000.0
///
/// # Method
/// The sidereal zero point (at J2000 ecliptic longitude `reference_j2000_deg`)
/// is precessed to the ecliptic-of-date using the full 3D ecliptic precession
/// matrix, and its longitude on the ecliptic-of-date is the ayanamsha.
/// This is consistent with tropical longitudes computed via the same 3D matrix.
pub fn ayanamsha_mean_deg(system: AyanamshaSystem, t_centuries: f64) -> f64 {
    ayanamsha_mean_deg_with_model(system, t_centuries, DEFAULT_PRECESSION_MODEL)
}

/// Mean ayanamsha in degrees at a given epoch for the selected precession model.
///
/// For star-anchored systems, uses the embedded HGCA star catalog with
/// proper-motion-corrected positions. Falls back to static anchor coordinates
/// for non-star-anchored systems (Lahiri, KP, Raman, etc.).
pub fn ayanamsha_mean_deg_with_model(
    system: AyanamshaSystem,
    t_centuries: f64,
    model: PrecessionModel,
) -> f64 {
    // Use embedded catalog (proper-motion-corrected star positions)
    if let Some(aya) =
        tara_anchor_ayanamsha_deg(system, t_centuries, model, TaraCatalog::embedded())
    {
        return aya;
    }
    // Static fallback for non-star-anchored systems
    ayanamsha_mean_deg_static_with_model(system, t_centuries, model)
}

/// Compute ayanamsha on a specified reference plane.
///
/// When `plane == Invariable`, the sidereal zero point is projected to the
/// invariable plane (no precession needed — the plane is fixed).
/// When `plane == Ecliptic`, this is identical to [`ayanamsha_deg_with_model`].
///
/// Nutation is skipped when `plane == Invariable` (nutation is an
/// ecliptic/equatorial concept, not meaningful on the invariable plane).
pub fn ayanamsha_deg_on_plane(
    system: AyanamshaSystem,
    t_centuries: f64,
    use_nutation: bool,
    model: PrecessionModel,
    plane: ReferencePlane,
) -> f64 {
    match plane {
        ReferencePlane::Ecliptic => {
            ayanamsha_deg_with_model(system, t_centuries, use_nutation, model)
        }
        ReferencePlane::Invariable => {
            // Nutation not applicable on invariable plane.
            ayanamsha_mean_deg_on_plane(system, t_centuries, model, plane)
        }
    }
}

/// Plane-aware mean ayanamsha with catalog support.
///
/// When `catalog` is `None`, uses the embedded HGCA catalog.
pub fn ayanamsha_deg_with_catalog_on_plane(
    system: AyanamshaSystem,
    t_centuries: f64,
    use_nutation: bool,
    catalog: Option<&TaraCatalog>,
    model: PrecessionModel,
    plane: ReferencePlane,
) -> f64 {
    match plane {
        ReferencePlane::Ecliptic => {
            ayanamsha_deg_with_catalog_and_model(system, t_centuries, use_nutation, catalog, model)
        }
        ReferencePlane::Invariable => {
            // Nutation not applicable on invariable plane.
            let effective_catalog = catalog.unwrap_or_else(|| TaraCatalog::embedded());
            if let Some(aya) = tara_anchor_ayanamsha_deg_on_plane(
                system,
                t_centuries,
                model,
                effective_catalog,
                plane,
            ) {
                return aya;
            }
            ayanamsha_mean_deg_static_on_plane(system, t_centuries, model, plane)
        }
    }
}

/// Mean ayanamsha on a specified reference plane.
///
/// For star-anchored systems, uses the embedded HGCA star catalog.
/// Falls back to static anchor coordinates for non-star-anchored systems.
fn ayanamsha_mean_deg_on_plane(
    system: AyanamshaSystem,
    t_centuries: f64,
    model: PrecessionModel,
    plane: ReferencePlane,
) -> f64 {
    match plane {
        ReferencePlane::Ecliptic => ayanamsha_mean_deg_with_model(system, t_centuries, model),
        ReferencePlane::Invariable => {
            // Use embedded catalog first
            if let Some(aya) = tara_anchor_ayanamsha_deg_on_plane(
                system,
                t_centuries,
                model,
                TaraCatalog::embedded(),
                plane,
            ) {
                return aya;
            }
            // Static fallback
            ayanamsha_mean_deg_static_on_plane(system, t_centuries, model, plane)
        }
    }
}

/// Compute ayanamsha on a specified reference plane from a J2000 reference longitude.
///
/// - `Ecliptic`: precess J2000 ecliptic direction to ecliptic-of-date.
/// - `Invariable`: project J2000 ecliptic direction to invariable plane (no precession).
fn ayanamsha_3d_on_plane(
    ref_j2000_deg: f64,
    t_centuries: f64,
    model: PrecessionModel,
    plane: ReferencePlane,
) -> f64 {
    match plane {
        ReferencePlane::Ecliptic => ayanamsha_3d(ref_j2000_deg, t_centuries, model),
        ReferencePlane::Invariable => {
            use dhruv_frames::{cartesian_to_spherical, ecliptic_to_invariable};
            let ref_rad = ref_j2000_deg.to_radians();
            let v = [ref_rad.cos(), ref_rad.sin(), 0.0];
            let inv = ecliptic_to_invariable(&v);
            cartesian_to_spherical(&inv).lon_deg.rem_euclid(360.0)
        }
    }
}

/// Compute ayanamsha by precessing the sidereal zero point to ecliptic-of-date.
///
/// `ref_j2000_deg` is the J2000 ecliptic longitude of the sidereal zero point.
/// Returns its longitude on the ecliptic-of-date, which equals the ayanamsha.
fn ayanamsha_3d(ref_j2000_deg: f64, t_centuries: f64, model: PrecessionModel) -> f64 {
    if t_centuries.abs() < 1e-15 {
        return ref_j2000_deg;
    }
    let ref_rad = ref_j2000_deg.to_radians();
    let v = [ref_rad.cos(), ref_rad.sin(), 0.0];
    let v_date = precess_ecliptic_j2000_to_date_with_model(&v, t_centuries, model);
    v_date[1].atan2(v_date[0]).to_degrees().rem_euclid(360.0)
}

/// "True"-mode ayanamsha helper in degrees.
///
/// Adds `delta_psi_arcsec` (nutation in longitude) to the mean ayanamsha
/// for all systems.
///
/// # Arguments
/// * `system` — the sidereal reference system
/// * `t_centuries` — Julian centuries of TDB since J2000.0
/// * `delta_psi_arcsec` — nutation in longitude in arcseconds (from an
///   external nutation model such as IAU 2000B)
pub fn ayanamsha_true_deg(system: AyanamshaSystem, t_centuries: f64, delta_psi_arcsec: f64) -> f64 {
    ayanamsha_true_deg_with_model(
        system,
        t_centuries,
        delta_psi_arcsec,
        DEFAULT_PRECESSION_MODEL,
    )
}

/// "True"-mode ayanamsha helper for the selected precession model, in degrees.
///
/// `delta_psi_arcsec` is applied for all systems.
pub fn ayanamsha_true_deg_with_model(
    system: AyanamshaSystem,
    t_centuries: f64,
    delta_psi_arcsec: f64,
    model: PrecessionModel,
) -> f64 {
    ayanamsha_mean_deg_with_model(system, t_centuries, model) + delta_psi_arcsec / 3600.0
}

/// Compute ayanamsha, optionally with nutation correction.
///
/// When `use_nutation` is true, nutation in longitude (Δψ) is computed
/// internally via IAU 2000B and added to the mean ayanamsha for all systems.
///
/// When `use_nutation` is false, this returns the same value as
/// [`ayanamsha_mean_deg`].
///
/// # Arguments
/// * `system` — the sidereal reference system
/// * `t_centuries` — Julian centuries of TDB since J2000.0
/// * `use_nutation` — whether to apply nutation correction
pub fn ayanamsha_deg(system: AyanamshaSystem, t_centuries: f64, use_nutation: bool) -> f64 {
    ayanamsha_deg_with_model(system, t_centuries, use_nutation, DEFAULT_PRECESSION_MODEL)
}

/// Compute ayanamsha, optionally with nutation correction, with a selected precession model.
///
/// When `use_nutation` is true, nutation in longitude (Δψ) is added for all systems.
pub fn ayanamsha_deg_with_model(
    system: AyanamshaSystem,
    t_centuries: f64,
    use_nutation: bool,
    model: PrecessionModel,
) -> f64 {
    let mean = ayanamsha_mean_deg_with_model(system, t_centuries, model);
    if use_nutation {
        let (delta_psi_arcsec, _) = nutation_iau2000b(t_centuries);
        mean + delta_psi_arcsec / 3600.0
    } else {
        mean
    }
}

/// Mean ayanamsha using star catalog for proper-motion-corrected anchors.
///
/// When `catalog` is `Some`, star-anchored systems use the provided catalog.
/// When `catalog` is `None`, uses the embedded HGCA catalog (identical to
/// [`ayanamsha_mean_deg`]).
pub fn ayanamsha_mean_deg_with_catalog(
    system: AyanamshaSystem,
    t_centuries: f64,
    catalog: Option<&TaraCatalog>,
) -> f64 {
    ayanamsha_mean_deg_with_catalog_and_model(
        system,
        t_centuries,
        catalog,
        DEFAULT_PRECESSION_MODEL,
    )
}

/// Mean ayanamsha with catalog and precession model selection.
///
/// When `catalog` is `Some`, uses the provided catalog. When `None`, uses
/// the embedded HGCA catalog (identical to [`ayanamsha_mean_deg_with_model`]).
pub fn ayanamsha_mean_deg_with_catalog_and_model(
    system: AyanamshaSystem,
    t_centuries: f64,
    catalog: Option<&TaraCatalog>,
    model: PrecessionModel,
) -> f64 {
    // Use provided catalog, or fall back to embedded catalog
    let effective_catalog = catalog.unwrap_or_else(|| TaraCatalog::embedded());
    if let Some(aya) = tara_anchor_ayanamsha_deg(system, t_centuries, model, effective_catalog) {
        return aya;
    }
    // Static fallback for non-star-anchored systems
    ayanamsha_mean_deg_static_with_model(system, t_centuries, model)
}

/// Compute ayanamsha with optional nutation and star catalog.
///
/// When `catalog` is `Some`, uses the provided catalog. When `None`, uses
/// the embedded HGCA catalog (identical to [`ayanamsha_deg`]).
pub fn ayanamsha_deg_with_catalog(
    system: AyanamshaSystem,
    t_centuries: f64,
    use_nutation: bool,
    catalog: Option<&TaraCatalog>,
) -> f64 {
    ayanamsha_deg_with_catalog_and_model(
        system,
        t_centuries,
        use_nutation,
        catalog,
        DEFAULT_PRECESSION_MODEL,
    )
}

/// Compute ayanamsha with optional nutation, star catalog, and precession model.
pub fn ayanamsha_deg_with_catalog_and_model(
    system: AyanamshaSystem,
    t_centuries: f64,
    use_nutation: bool,
    catalog: Option<&TaraCatalog>,
    model: PrecessionModel,
) -> f64 {
    let mean = ayanamsha_mean_deg_with_catalog_and_model(system, t_centuries, catalog, model);
    if use_nutation {
        let (delta_psi_arcsec, _) = nutation_iau2000b(t_centuries);
        mean + delta_psi_arcsec / 3600.0
    } else {
        mean
    }
}

// ---- Static (no-catalog) API for testing and validation ----

/// Mean ayanamsha using static anchor coordinates only (no star catalog).
///
/// Uses hardcoded J2000 ecliptic coordinates for star-anchored systems.
/// Production code should use [`ayanamsha_mean_deg`] which uses the embedded
/// HGCA star catalog with proper-motion-corrected positions.
pub fn ayanamsha_mean_deg_static(system: AyanamshaSystem, t_centuries: f64) -> f64 {
    ayanamsha_mean_deg_static_with_model(system, t_centuries, DEFAULT_PRECESSION_MODEL)
}

/// Mean ayanamsha using static anchor coordinates with explicit precession model.
pub fn ayanamsha_mean_deg_static_with_model(
    system: AyanamshaSystem,
    t_centuries: f64,
    model: PrecessionModel,
) -> f64 {
    if let Some(aya) = anchor_relative_ayanamsha_deg(system, t_centuries, model) {
        aya
    } else {
        ayanamsha_3d(system.reference_j2000_deg(), t_centuries, model)
    }
}

/// Ayanamsha (with optional nutation) using static anchor coordinates only.
pub fn ayanamsha_deg_static(system: AyanamshaSystem, t_centuries: f64, use_nutation: bool) -> f64 {
    let mean = ayanamsha_mean_deg_static(system, t_centuries);
    if use_nutation {
        let (dpsi, _) = nutation_iau2000b(t_centuries);
        mean + dpsi / 3600.0
    } else {
        mean
    }
}

/// Plane-aware mean ayanamsha using static anchor coordinates only.
///
/// For `ReferencePlane::Invariable`, projects the hardcoded J2000 ecliptic
/// anchor to the invariable plane.
pub fn ayanamsha_mean_deg_static_on_plane(
    system: AyanamshaSystem,
    t_centuries: f64,
    model: PrecessionModel,
    plane: ReferencePlane,
) -> f64 {
    match plane {
        ReferencePlane::Ecliptic => {
            ayanamsha_mean_deg_static_with_model(system, t_centuries, model)
        }
        ReferencePlane::Invariable => {
            if let Some(aya) =
                anchor_relative_ayanamsha_deg_on_plane(system, t_centuries, model, plane)
            {
                aya
            } else {
                ayanamsha_3d_on_plane(system.reference_j2000_deg(), t_centuries, model, plane)
            }
        }
    }
}

/// Convert a Julian Date in TDB to Julian centuries since J2000.0.
pub fn jd_tdb_to_centuries(jd_tdb: f64) -> f64 {
    (jd_tdb - J2000_JD) / 36525.0
}

/// Convert TDB seconds past J2000.0 to Julian centuries.
pub fn tdb_seconds_to_centuries(tdb_s: f64) -> f64 {
    tdb_s / (36525.0 * 86_400.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_systems_count() {
        assert_eq!(AyanamshaSystem::all().len(), 20);
    }

    #[test]
    fn lahiri_at_j2000() {
        let val = ayanamsha_mean_deg(AyanamshaSystem::Lahiri, 0.0);
        assert!(
            (val - AyanamshaSystem::Lahiri.reference_j2000_deg()).abs() < 1e-12,
            "Lahiri at J2000 = {val}"
        );
    }

    #[test]
    fn precession_forward() {
        let at_0 = ayanamsha_mean_deg(AyanamshaSystem::Lahiri, 0.0);
        let at_1 = ayanamsha_mean_deg(AyanamshaSystem::Lahiri, 1.0);
        let diff = at_1 - at_0;
        // ~1.397 deg/century
        assert!((diff - 1.397).abs() < 0.01, "one century drift = {diff}");
    }

    #[test]
    fn precession_backward() {
        let at_0 = ayanamsha_mean_deg(AyanamshaSystem::Lahiri, 0.0);
        let at_neg = ayanamsha_mean_deg(AyanamshaSystem::Lahiri, -1.0);
        assert!(at_neg < at_0, "Lahiri should decrease for past epochs");
    }

    #[test]
    fn true_lahiri_zero_nutation() {
        let t = 0.5;
        let mean = ayanamsha_mean_deg(AyanamshaSystem::TrueLahiri, t);
        let true_val = ayanamsha_true_deg(AyanamshaSystem::TrueLahiri, t, 0.0);
        assert!((true_val - mean).abs() < 1e-15);
    }

    #[test]
    fn true_deg_applies_delta_psi() {
        let delta_psi = 17.0; // arcseconds
        let t = 0.5;
        let mean = ayanamsha_mean_deg(AyanamshaSystem::TrueLahiri, t);
        let true_val = ayanamsha_true_deg(AyanamshaSystem::TrueLahiri, t, delta_psi);
        assert!(
            (true_val - (mean + delta_psi / 3600.0)).abs() < 1e-10,
            "true_val = {true_val}, expected = {}",
            mean + delta_psi / 3600.0
        );
    }

    #[test]
    fn true_deg_applies_nutation_all_systems() {
        let dpsi = 17.0; // arcseconds
        let mean = ayanamsha_mean_deg(AyanamshaSystem::Lahiri, 0.0);
        let true_val = ayanamsha_true_deg(AyanamshaSystem::Lahiri, 0.0, dpsi);
        assert!(
            (true_val - (mean + dpsi / 3600.0)).abs() < 1e-10,
            "true_val = {true_val}, expected = {}",
            mean + dpsi / 3600.0
        );
    }

    #[test]
    fn all_references_in_range() {
        for &sys in AyanamshaSystem::all() {
            let val = sys.reference_j2000_deg();
            assert!(
                (19.0..=28.0).contains(&val),
                "{sys:?} reference = {val}, outside [19, 28]"
            );
        }
    }

    #[test]
    fn century_conversions() {
        let jd = 2_460_000.5;
        let t = jd_tdb_to_centuries(jd);
        let jd_back = t * 36525.0 + J2000_JD;
        assert!((jd_back - jd).abs() < 1e-12);

        let s = 1_000_000.0;
        let t2 = tdb_seconds_to_centuries(s);
        let s_back = t2 * 36525.0 * 86_400.0;
        assert!((s_back - s).abs() < 1e-6);
    }

    #[test]
    fn ayanamsha_deg_without_nutation_matches_mean() {
        let t = 0.24;
        for &sys in AyanamshaSystem::all() {
            let unified = ayanamsha_deg(sys, t, false);
            let mean = ayanamsha_mean_deg(sys, t);
            assert!(
                (unified - mean).abs() < 1e-15,
                "{sys:?}: unified={unified}, mean={mean}"
            );
        }
    }

    #[test]
    fn nutation_flag_adds_dpsi() {
        let t = 0.24;
        let with = ayanamsha_deg(AyanamshaSystem::TrueLahiri, t, true);
        let without = ayanamsha_deg(AyanamshaSystem::TrueLahiri, t, false);
        let (dpsi_arcsec, _) = nutation_iau2000b(t);
        let expected_diff = dpsi_arcsec / 3600.0;
        assert!(
            (with - without - expected_diff).abs() < 1e-10,
            "diff={}, expected={}",
            with - without,
            expected_diff
        );
    }

    #[test]
    fn nutation_flag_adds_dpsi_lahiri() {
        let t = 0.24;
        let with = ayanamsha_deg(AyanamshaSystem::Lahiri, t, true);
        let without = ayanamsha_deg(AyanamshaSystem::Lahiri, t, false);
        let (dpsi_arcsec, _) = nutation_iau2000b(t);
        let expected_diff = dpsi_arcsec / 3600.0;
        assert!(
            (with - without - expected_diff).abs() < 1e-10,
            "diff={}, expected={}",
            with - without,
            expected_diff
        );
    }

    #[test]
    fn with_model_wrappers_match_default() {
        let t = 0.37;
        let sys = AyanamshaSystem::Lahiri;
        let mean_default = ayanamsha_mean_deg(sys, t);
        let mean_explicit = ayanamsha_mean_deg_with_model(sys, t, DEFAULT_PRECESSION_MODEL);
        assert!((mean_default - mean_explicit).abs() < 1e-15);

        let aya_default = ayanamsha_deg(sys, t, true);
        let aya_explicit = ayanamsha_deg_with_model(sys, t, true, DEFAULT_PRECESSION_MODEL);
        assert!((aya_default - aya_explicit).abs() < 1e-15);
    }

    #[test]
    fn vondrak_model_path_is_available() {
        let t = 25.0;
        let sys = AyanamshaSystem::Lahiri;
        let iau = ayanamsha_mean_deg_with_model(sys, t, PrecessionModel::Iau2006);
        let vondrak = ayanamsha_mean_deg_with_model(sys, t, PrecessionModel::Vondrak2011);
        assert!((iau - vondrak).abs() > 1e-6);
    }

    #[test]
    fn lahiri_true_at_1956_matches_gazette() {
        let t_1956 = (2_435_553.5 - 2_451_545.0) / 36525.0;
        let gazette = 23.0 + 15.0 / 60.0 + 0.658 / 3600.0;
        let val = ayanamsha_deg(AyanamshaSystem::Lahiri, t_1956, true);
        assert!(
            (val - gazette).abs() < 1e-6,
            "Lahiri true at 1956 = {val}, gazette = {gazette}"
        );
    }

    #[test]
    fn lahiri_mean_at_1956() {
        let t_1956 = (2_435_553.5 - 2_451_545.0) / 36525.0;
        let gazette = 23.0 + 15.0 / 60.0 + 0.658 / 3600.0;
        let (dpsi_arcsec, _) = nutation_iau2000b(t_1956);
        let expected_mean = gazette - dpsi_arcsec / 3600.0;
        let val = ayanamsha_deg(AyanamshaSystem::Lahiri, t_1956, false);
        assert!(
            (val - expected_mean).abs() < 1e-6,
            "Lahiri mean at 1956 = {val}, expected = {expected_mean}"
        );
    }

    #[test]
    fn default_uses_embedded_catalog() {
        // Default path (no catalog param) must equal explicit embedded catalog path
        let t = 0.24;
        for &sys in AyanamshaSystem::all() {
            let default_val = ayanamsha_mean_deg(sys, t);
            let explicit_catalog =
                ayanamsha_mean_deg_with_catalog(sys, t, Some(TaraCatalog::embedded()));
            assert!(
                (default_val - explicit_catalog).abs() < 1e-15,
                "{sys:?}: default={default_val}, explicit_catalog={explicit_catalog}"
            );
        }
    }

    #[test]
    fn with_catalog_none_uses_embedded() {
        // _with_catalog(None) must also use embedded catalog, not static
        let t = 0.24;
        for &sys in AyanamshaSystem::all() {
            let with_none = ayanamsha_mean_deg_with_catalog(sys, t, None);
            let default_val = ayanamsha_mean_deg(sys, t);
            assert!(
                (with_none - default_val).abs() < 1e-15,
                "{sys:?}: with_catalog(None)={with_none}, default={default_val}"
            );
        }
    }

    #[test]
    fn static_path_self_consistent() {
        let t = 0.24;
        for &sys in AyanamshaSystem::all() {
            let static_val = ayanamsha_mean_deg_static(sys, t);
            assert!(
                static_val > 19.0 && static_val < 28.0,
                "{sys:?}: {static_val}"
            );
        }
    }

    #[test]
    fn static_and_default_bounded_difference() {
        // Static vs default (catalog) should differ by less than 5"
        // at J2000, since anchor coords were derived from the same catalog
        let t = 0.0;
        for &sys in AyanamshaSystem::all() {
            let default_val = ayanamsha_mean_deg(sys, t);
            let static_val = ayanamsha_mean_deg_static(sys, t);
            let diff_arcsec = (default_val - static_val).abs() * 3600.0;
            assert!(
                diff_arcsec < 5.0,
                "{sys:?}: default={default_val}, static={static_val} (diff={diff_arcsec}\")"
            );
        }
    }

    #[test]
    fn reference_consistent_with_anchor_chandrahari() {
        // ChandraHari anchor: lon=264.585715, target=240.0 → ref ≈ 24.586
        let ref_deg = AyanamshaSystem::ChandraHari.reference_j2000_deg();
        assert!(
            (ref_deg - 24.586).abs() < 0.001,
            "ChandraHari reference={ref_deg}, expected ~24.586"
        );
    }

    #[test]
    fn reference_consistent_with_anchor_gc() {
        // GalacticCenter0Sag anchor: lon=266.839617, target=240.0 → ref ≈ 26.840
        let ref_deg = AyanamshaSystem::GalacticCenter0Sag.reference_j2000_deg();
        assert!(
            (ref_deg - 26.840).abs() < 0.001,
            "GC reference={ref_deg}, expected ~26.840"
        );
    }
}
