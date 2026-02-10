//! Ayanamsha computation for 20 sidereal reference systems.
//!
//! The ayanamsha is the angular offset between the tropical zodiac (defined
//! by the vernal equinox) and a sidereal zodiac (anchored to fixed stars).
//! As the equinox precesses westward, the ayanamsha increases over time.
//!
//! Each system is defined by its J2000.0 reference value. The ayanamsha at
//! any epoch is computed by adding the IAU 2006 general precession to that
//! reference.
//!
//! Clean-room implementation: all reference values derived independently from
//! published system definitions. See `docs/clean_room_ayanamsha.md`.

use dhruv_frames::general_precession_longitude_deg;
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
            // Indian govt gazette, Spica at 0 deg Libra sidereal
            Self::Lahiri => 23.853,
            // Same anchor as Lahiri; nutation applied separately
            Self::TrueLahiri => 23.853,
            // Krishnamurti: minimal offset from Lahiri
            Self::KP => 23.850,
            // B.V. Raman: zero year ~397 CE
            Self::Raman => 22.370,
            // Fagan-Bradley SVP calibration
            Self::FaganBradley => 24.736,
            // delta Cancri at 106 deg sidereal
            Self::PushyaPaksha => 21.000,
            // Aldebaran at 15 deg 47 min Taurus
            Self::RohiniPaksha => 24.087,
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
            // Chandra Hari
            Self::ChandraHari => 23.250,
            // Jagganatha
            Self::Jagganatha => 23.250,
            // Surya Siddhanta (IAU precession back-computed)
            Self::SuryaSiddhanta => 22.459,
            // Galactic Center at 0 deg Sagittarius
            Self::GalacticCenter0Sag => 26.860,
            // Aldebaran at 15 deg Taurus
            Self::Aldebaran15Tau => 24.870,
        }
    }

    /// Whether this system uses the true (nutation-corrected) equinox.
    ///
    /// Only `TrueLahiri` returns `true`. All other systems use the mean equinox.
    pub const fn uses_true_equinox(self) -> bool {
        matches!(self, Self::TrueLahiri)
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
/// # Formula
/// `ayanamsha(T) = reference_j2000 + p_A(T) / 3600`
///
/// where p_A is the IAU 2006 general precession in ecliptic longitude (arcsec).
pub fn ayanamsha_mean_deg(system: AyanamshaSystem, t_centuries: f64) -> f64 {
    system.reference_j2000_deg() + general_precession_longitude_deg(t_centuries)
}

/// True (nutation-corrected) ayanamsha in degrees.
///
/// For `TrueLahiri`, adds `delta_psi_arcsec` (nutation in longitude) to the
/// mean value. For all other systems, returns the mean value unchanged.
///
/// # Arguments
/// * `system` — the sidereal reference system
/// * `t_centuries` — Julian centuries of TDB since J2000.0
/// * `delta_psi_arcsec` — nutation in longitude in arcseconds (from an
///   external nutation model such as IAU 2000B)
pub fn ayanamsha_true_deg(
    system: AyanamshaSystem,
    t_centuries: f64,
    delta_psi_arcsec: f64,
) -> f64 {
    let mean = ayanamsha_mean_deg(system, t_centuries);
    if system.uses_true_equinox() {
        mean + delta_psi_arcsec / 3600.0
    } else {
        mean
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
            (val - AyanamshaSystem::Lahiri.reference_j2000_deg()).abs() < 1e-15,
            "Lahiri at J2000 = {val}"
        );
    }

    #[test]
    fn precession_forward() {
        let at_0 = ayanamsha_mean_deg(AyanamshaSystem::Lahiri, 0.0);
        let at_1 = ayanamsha_mean_deg(AyanamshaSystem::Lahiri, 1.0);
        let diff = at_1 - at_0;
        // ~1.397 deg/century
        assert!(
            (diff - 1.397).abs() < 0.01,
            "one century drift = {diff}"
        );
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
    fn true_lahiri_adds_nutation() {
        let delta_psi = 17.0; // arcseconds, typical nutation amplitude
        let true_val = ayanamsha_true_deg(AyanamshaSystem::TrueLahiri, 0.0, delta_psi);
        let expected = AyanamshaSystem::TrueLahiri.reference_j2000_deg() + 17.0 / 3600.0;
        assert!(
            (true_val - expected).abs() < 1e-10,
            "true_val = {true_val}, expected = {expected}"
        );
    }

    #[test]
    fn non_true_ignores_nutation() {
        let mean = ayanamsha_mean_deg(AyanamshaSystem::Lahiri, 0.0);
        let true_val = ayanamsha_true_deg(AyanamshaSystem::Lahiri, 0.0, 999.0);
        assert!((true_val - mean).abs() < 1e-15);
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
    fn only_true_lahiri_uses_true_equinox() {
        for &sys in AyanamshaSystem::all() {
            if sys == AyanamshaSystem::TrueLahiri {
                assert!(sys.uses_true_equinox());
            } else {
                assert!(!sys.uses_true_equinox(), "{sys:?} should not use true equinox");
            }
        }
    }
}
