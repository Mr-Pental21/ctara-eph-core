//! Types for bhava (house) system computation.
//!
//! Provides enums for 10 house systems, configuration types, and result types
//! used by the bhava computation module.

use dhruv_core::Body;

/// The 10 supported house division systems.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BhavaSystem {
    /// Equal houses: each house spans exactly 30 degrees.
    Equal,
    /// Surya Siddhanta equal houses (same algorithm as Equal, traditional name).
    SuryaSiddhanta,
    /// Sripati (Porphyry): trisect the four quadrant arcs.
    Sripati,
    /// KP (Krishnamurti Paddhati), uses Placidus algorithm.
    KP,
    /// Koch: time-based division using MC-to-horizon.
    Koch,
    /// Regiomontanus: 30-degree equator arcs from East Point, projected to ecliptic.
    Regiomontanus,
    /// Campanus: 30-degree prime vertical arcs, projected to ecliptic.
    Campanus,
    /// Axial Rotation (Meridian): RAMC + 30-degree equator arcs.
    AxialRotation,
    /// Topocentric (Polich-Page): tangent-ratio semi-arc.
    Topocentric,
    /// Alcabitus: semi-arc equator division + projection.
    Alcabitus,
}

/// All 10 bhava systems in enum order, for FFI indexing.
pub const ALL_BHAVA_SYSTEMS: [BhavaSystem; 10] = [
    BhavaSystem::Equal,
    BhavaSystem::SuryaSiddhanta,
    BhavaSystem::Sripati,
    BhavaSystem::KP,
    BhavaSystem::Koch,
    BhavaSystem::Regiomontanus,
    BhavaSystem::Campanus,
    BhavaSystem::AxialRotation,
    BhavaSystem::Topocentric,
    BhavaSystem::Alcabitus,
];

impl BhavaSystem {
    /// All 10 defined bhava systems.
    pub const fn all() -> &'static [BhavaSystem] {
        &ALL_BHAVA_SYSTEMS
    }

    /// Whether this system depends on geographic latitude.
    ///
    /// Latitude-dependent systems fail for |lat| > 66.5 degrees.
    pub const fn latitude_dependent(self) -> bool {
        matches!(
            self,
            Self::KP | Self::Koch | Self::Topocentric | Self::Alcabitus
        )
    }

    /// Whether this system uses simple equal (30-deg) division.
    pub const fn is_equal_division(self) -> bool {
        matches!(self, Self::Equal | Self::SuryaSiddhanta)
    }
}

/// What defines the starting point for house cusps.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BhavaStartingPoint {
    /// Use the Lagna (Ascendant) as the starting point (default).
    Lagna,
    /// Use a body's ecliptic longitude as the starting point.
    BodyLongitude(Body),
    /// Use an arbitrary ecliptic degree as the starting point.
    CustomDeg(f64),
}

impl Default for BhavaStartingPoint {
    fn default() -> Self {
        Self::Lagna
    }
}

/// Whether the starting point is the start or middle of bhava 1.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum BhavaReferenceMode {
    /// The starting point is at the cusp (beginning) of bhava 1.
    #[default]
    StartOfFirst,
    /// The starting point is at the middle of bhava 1.
    MiddleOfFirst,
}

/// Configuration for bhava computation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BhavaConfig {
    /// Which house system to use.
    pub system: BhavaSystem,
    /// What point defines the starting cusp.
    pub starting_point: BhavaStartingPoint,
    /// Whether the starting point is the start or middle of bhava 1.
    pub reference_mode: BhavaReferenceMode,
}

impl Default for BhavaConfig {
    fn default() -> Self {
        Self {
            system: BhavaSystem::Equal,
            starting_point: BhavaStartingPoint::Lagna,
            reference_mode: BhavaReferenceMode::StartOfFirst,
        }
    }
}

/// A single bhava (house) result.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Bhava {
    /// House number, 1-12.
    pub number: u8,
    /// Ecliptic longitude of the cusp in degrees, [0, 360).
    pub cusp_deg: f64,
    /// Start of this bhava in degrees, [0, 360).
    pub start_deg: f64,
    /// End of this bhava in degrees, [0, 360). Equals next bhava's start.
    pub end_deg: f64,
}

/// Full result of a bhava computation: 12 bhavas plus Lagna/MC.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BhavaResult {
    /// The 12 bhavas, indexed 0..12 (bhava[0] = house 1).
    pub bhavas: [Bhava; 12],
    /// Ecliptic longitude of the Lagna (Ascendant) in degrees, [0, 360).
    pub lagna_deg: f64,
    /// Ecliptic longitude of the MC in degrees, [0, 360).
    pub mc_deg: f64,
}

/// Normalize an angle to [0, 360) degrees.
pub(crate) fn normalize_deg(deg: f64) -> f64 {
    deg.rem_euclid(360.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_systems_count() {
        assert_eq!(BhavaSystem::all().len(), 10);
    }

    #[test]
    fn default_config() {
        let c = BhavaConfig::default();
        assert_eq!(c.system, BhavaSystem::Equal);
        assert_eq!(c.starting_point, BhavaStartingPoint::Lagna);
        assert_eq!(c.reference_mode, BhavaReferenceMode::StartOfFirst);
    }

    #[test]
    fn normalize_deg_positive() {
        assert!((normalize_deg(370.0) - 10.0).abs() < 1e-10);
    }

    #[test]
    fn normalize_deg_negative() {
        assert!((normalize_deg(-10.0) - 350.0).abs() < 1e-10);
    }

    #[test]
    fn normalize_deg_zero() {
        assert!((normalize_deg(0.0)).abs() < 1e-10);
    }

    #[test]
    fn normalize_deg_360() {
        assert!((normalize_deg(360.0)).abs() < 1e-10);
    }

    #[test]
    fn equal_systems_are_equal() {
        assert!(BhavaSystem::Equal.is_equal_division());
        assert!(BhavaSystem::SuryaSiddhanta.is_equal_division());
        assert!(!BhavaSystem::Sripati.is_equal_division());
        assert!(!BhavaSystem::KP.is_equal_division());
    }

    #[test]
    fn latitude_dependent_systems() {
        assert!(BhavaSystem::KP.latitude_dependent());
        assert!(BhavaSystem::Koch.latitude_dependent());
        assert!(BhavaSystem::Topocentric.latitude_dependent());
        assert!(BhavaSystem::Alcabitus.latitude_dependent());
        assert!(!BhavaSystem::Equal.latitude_dependent());
        assert!(!BhavaSystem::Sripati.latitude_dependent());
        assert!(!BhavaSystem::Regiomontanus.latitude_dependent());
        assert!(!BhavaSystem::Campanus.latitude_dependent());
        assert!(!BhavaSystem::AxialRotation.latitude_dependent());
    }
}
