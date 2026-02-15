//! Core types for dasha (planetary period) calculations.
//!
//! Dashas are hierarchical time-period systems from Vedic astrology (BPHS).
//! This module defines the fundamental data structures shared across all
//! 23 dasha systems.

use crate::graha::Graha;

/// Year length constant for dasha period calculations.
pub const DAYS_PER_YEAR: f64 = 365.25;

/// Maximum dasha depth. Levels 0-4 supported.
pub const MAX_DASHA_LEVEL: u8 = 4;

/// Default max level for queries (keeps output manageable).
pub const DEFAULT_DASHA_LEVEL: u8 = 2;

/// Hard cap on periods per level to prevent combinatorial explosion.
pub const MAX_PERIODS_PER_LEVEL: usize = 100_000;

/// Maximum dasha systems selectable in FullKundaliConfig.
pub const MAX_DASHA_SYSTEMS: usize = 8;

/// 5 hierarchical dasha levels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum DashaLevel {
    Mahadasha = 0,
    Antardasha = 1,
    Pratyantardasha = 2,
    Sookshmadasha = 3,
    Pranadasha = 4,
}

impl DashaLevel {
    /// Create from raw u8 value.
    pub fn from_u8(v: u8) -> Option<Self> {
        match v {
            0 => Some(Self::Mahadasha),
            1 => Some(Self::Antardasha),
            2 => Some(Self::Pratyantardasha),
            3 => Some(Self::Sookshmadasha),
            4 => Some(Self::Pranadasha),
            _ => None,
        }
    }

    /// Human-readable name.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Mahadasha => "Mahadasha",
            Self::Antardasha => "Antardasha",
            Self::Pratyantardasha => "Pratyantardasha",
            Self::Sookshmadasha => "Sookshmadasha",
            Self::Pranadasha => "Pranadasha",
        }
    }

    /// Next deeper level, if any.
    pub const fn child_level(self) -> Option<Self> {
        match self {
            Self::Mahadasha => Some(Self::Antardasha),
            Self::Antardasha => Some(Self::Pratyantardasha),
            Self::Pratyantardasha => Some(Self::Sookshmadasha),
            Self::Sookshmadasha => Some(Self::Pranadasha),
            Self::Pranadasha => None,
        }
    }
}

/// What entity rules a dasha period.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DashaEntity {
    /// Nakshatra-based and graha-based systems.
    Graha(Graha),
    /// Rashi-based systems (0-based index, 0=Mesha..11=Meena).
    Rashi(u8),
    /// Yogini system (0-based, 0..7 for 8 yoginis).
    Yogini(u8),
}

impl DashaEntity {
    /// Entity type code for FFI: 0=Graha, 1=Rashi, 2=Yogini.
    pub const fn type_code(&self) -> u8 {
        match self {
            Self::Graha(_) => 0,
            Self::Rashi(_) => 1,
            Self::Yogini(_) => 2,
        }
    }

    /// Entity index for FFI.
    pub const fn entity_index(&self) -> u8 {
        match self {
            Self::Graha(g) => g.index(),
            Self::Rashi(r) => *r,
            Self::Yogini(y) => *y,
        }
    }
}

/// A single dasha period.
#[derive(Debug, Clone, Copy)]
pub struct DashaPeriod {
    /// The entity ruling this period.
    pub entity: DashaEntity,
    /// JD UTC, inclusive.
    pub start_jd: f64,
    /// JD UTC, exclusive.
    pub end_jd: f64,
    /// Hierarchical level.
    pub level: DashaLevel,
    /// 1-indexed position among siblings.
    pub order: u16,
    /// Index into parent level's array (0 for level 0).
    pub parent_idx: u32,
}

impl DashaPeriod {
    /// Duration of the period in days.
    pub fn duration_days(&self) -> f64 {
        self.end_jd - self.start_jd
    }
}

/// All 23 dasha systems.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum DashaSystem {
    // Nakshatra-based (10)
    Vimshottari = 0,
    Ashtottari = 1,
    Shodsottari = 2,
    Dwadashottari = 3,
    Panchottari = 4,
    Shatabdika = 5,
    Chaturashiti = 6,
    DwisaptatiSama = 7,
    Shashtihayani = 8,
    ShatTrimshaSama = 9,
    // Yogini (1)
    Yogini = 10,
    // Rashi-based (7)
    Chara = 11,
    Sthira = 12,
    Yogardha = 13,
    Driga = 14,
    Shoola = 15,
    Mandooka = 16,
    Chakra = 17,
    // Graha-based (1)
    Kala = 18,
    // Special (1)
    KaalChakra = 19,
    // Kendradi variants (3)
    Kendradi = 20,
    KarakaKendradi = 21,
    KarakaKendradiGraha = 22,
}

/// All 23 dasha systems in order.
pub const ALL_DASHA_SYSTEMS: [DashaSystem; 23] = [
    DashaSystem::Vimshottari,
    DashaSystem::Ashtottari,
    DashaSystem::Shodsottari,
    DashaSystem::Dwadashottari,
    DashaSystem::Panchottari,
    DashaSystem::Shatabdika,
    DashaSystem::Chaturashiti,
    DashaSystem::DwisaptatiSama,
    DashaSystem::Shashtihayani,
    DashaSystem::ShatTrimshaSama,
    DashaSystem::Yogini,
    DashaSystem::Chara,
    DashaSystem::Sthira,
    DashaSystem::Yogardha,
    DashaSystem::Driga,
    DashaSystem::Shoola,
    DashaSystem::Mandooka,
    DashaSystem::Chakra,
    DashaSystem::Kala,
    DashaSystem::KaalChakra,
    DashaSystem::Kendradi,
    DashaSystem::KarakaKendradi,
    DashaSystem::KarakaKendradiGraha,
];

impl DashaSystem {
    /// Create from repr(u8) value.
    pub fn from_u8(v: u8) -> Option<Self> {
        if (v as usize) < ALL_DASHA_SYSTEMS.len() {
            Some(ALL_DASHA_SYSTEMS[v as usize])
        } else {
            None
        }
    }

    /// Human-readable name.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Vimshottari => "Vimshottari",
            Self::Ashtottari => "Ashtottari",
            Self::Shodsottari => "Shodsottari",
            Self::Dwadashottari => "Dwadashottari",
            Self::Panchottari => "Panchottari",
            Self::Shatabdika => "Shatabdika",
            Self::Chaturashiti => "Chaturashiti",
            Self::DwisaptatiSama => "Dwisaptati Sama",
            Self::Shashtihayani => "Shashtihayani",
            Self::ShatTrimshaSama => "Shat-Trimsha Sama",
            Self::Yogini => "Yogini",
            Self::Chara => "Chara",
            Self::Sthira => "Sthira",
            Self::Yogardha => "Yogardha",
            Self::Driga => "Driga",
            Self::Shoola => "Shoola",
            Self::Mandooka => "Mandooka",
            Self::Chakra => "Chakra",
            Self::Kala => "Kala",
            Self::KaalChakra => "Kaal Chakra",
            Self::Kendradi => "Kendradi",
            Self::KarakaKendradi => "Karaka Kendradi",
            Self::KarakaKendradiGraha => "Karaka Kendradi Graha",
        }
    }
}

/// Complete hierarchy for a dasha system.
#[derive(Debug, Clone)]
pub struct DashaHierarchy {
    /// Which system produced this hierarchy.
    pub system: DashaSystem,
    /// Birth JD UTC.
    pub birth_jd: f64,
    /// Levels: levels[0]=mahadasha, levels[1]=antardasha, etc.
    pub levels: Vec<Vec<DashaPeriod>>,
}

/// Active periods at a specific date (one per requested level).
#[derive(Debug, Clone)]
pub struct DashaSnapshot {
    /// Which system produced this snapshot.
    pub system: DashaSystem,
    /// The queried JD UTC.
    pub query_jd: f64,
    /// Active periods: periods[0]=active mahadasha, [1]=active antardasha, etc.
    pub periods: Vec<DashaPeriod>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dasha_level_from_u8() {
        assert_eq!(DashaLevel::from_u8(0), Some(DashaLevel::Mahadasha));
        assert_eq!(DashaLevel::from_u8(4), Some(DashaLevel::Pranadasha));
        assert_eq!(DashaLevel::from_u8(5), None);
    }

    #[test]
    fn dasha_level_child() {
        assert_eq!(
            DashaLevel::Mahadasha.child_level(),
            Some(DashaLevel::Antardasha)
        );
        assert_eq!(DashaLevel::Pranadasha.child_level(), None);
    }

    #[test]
    fn dasha_system_from_u8() {
        assert_eq!(DashaSystem::from_u8(0), Some(DashaSystem::Vimshottari));
        assert_eq!(
            DashaSystem::from_u8(22),
            Some(DashaSystem::KarakaKendradiGraha)
        );
        assert_eq!(DashaSystem::from_u8(23), None);
    }

    #[test]
    fn all_dasha_systems_count() {
        assert_eq!(ALL_DASHA_SYSTEMS.len(), 23);
    }

    #[test]
    fn entity_type_codes() {
        assert_eq!(DashaEntity::Graha(Graha::Surya).type_code(), 0);
        assert_eq!(DashaEntity::Rashi(0).type_code(), 1);
        assert_eq!(DashaEntity::Yogini(0).type_code(), 2);
    }

    #[test]
    fn days_per_year_constant() {
        assert!((DAYS_PER_YEAR - 365.25).abs() < 1e-15);
    }
}
