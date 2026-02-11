//! Vedic planet (graha) enum and rashi lordship.
//!
//! The 9 grahas form the foundation of all Vedic jyotish calculations.
//! Each rashi has a planetary lord, which is a universal Vedic convention.
//!
//! Clean-room implementation from standard Vedic jyotish texts (BPHS).

use crate::rashi::Rashi;

/// The 9 Vedic grahas.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Graha {
    Surya,
    Chandra,
    Mangal,
    Buddh,
    Guru,
    Shukra,
    Shani,
    Rahu,
    Ketu,
}

/// All 9 grahas in traditional order.
pub const ALL_GRAHAS: [Graha; 9] = [
    Graha::Surya,
    Graha::Chandra,
    Graha::Mangal,
    Graha::Buddh,
    Graha::Guru,
    Graha::Shukra,
    Graha::Shani,
    Graha::Rahu,
    Graha::Ketu,
];

/// The 7 classical grahas (sapta grahas), excluding Rahu and Ketu.
/// Used for Ashtakavarga and other calculations that only consider the 7 planets.
pub const SAPTA_GRAHAS: [Graha; 7] = [
    Graha::Surya,
    Graha::Chandra,
    Graha::Mangal,
    Graha::Buddh,
    Graha::Guru,
    Graha::Shukra,
    Graha::Shani,
];

/// Kaksha (portion) values for each sapta graha, used in Indu Lagna calculation.
/// Order matches SAPTA_GRAHAS: Sun=30, Moon=16, Mars=6, Mercury=8, Jupiter=10, Venus=12, Saturn=1.
pub const GRAHA_KAKSHA_VALUES: [u8; 7] = [30, 16, 6, 8, 10, 12, 1];

impl Graha {
    /// Sanskrit name of the graha.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Surya => "Surya",
            Self::Chandra => "Chandra",
            Self::Mangal => "Mangal",
            Self::Buddh => "Buddh",
            Self::Guru => "Guru",
            Self::Shukra => "Shukra",
            Self::Shani => "Shani",
            Self::Rahu => "Rahu",
            Self::Ketu => "Ketu",
        }
    }

    /// English name of the graha.
    pub const fn english_name(self) -> &'static str {
        match self {
            Self::Surya => "Sun",
            Self::Chandra => "Moon",
            Self::Mangal => "Mars",
            Self::Buddh => "Mercury",
            Self::Guru => "Jupiter",
            Self::Shukra => "Venus",
            Self::Shani => "Saturn",
            Self::Rahu => "Rahu",
            Self::Ketu => "Ketu",
        }
    }

    /// 0-based index into ALL_GRAHAS.
    pub const fn index(self) -> u8 {
        match self {
            Self::Surya => 0,
            Self::Chandra => 1,
            Self::Mangal => 2,
            Self::Buddh => 3,
            Self::Guru => 4,
            Self::Shukra => 5,
            Self::Shani => 6,
            Self::Rahu => 7,
            Self::Ketu => 8,
        }
    }

    /// NAIF body code for ephemeris queries. Returns None for Rahu/Ketu
    /// (computed mathematically, not from SPK kernels).
    pub const fn naif_code(self) -> Option<i32> {
        match self {
            Self::Surya => Some(10),
            Self::Chandra => Some(301),
            Self::Mangal => Some(499),
            Self::Buddh => Some(199),
            Self::Guru => Some(599),
            Self::Shukra => Some(299),
            Self::Shani => Some(699),
            Self::Rahu | Self::Ketu => None,
        }
    }

    /// Kaksha value for Indu Lagna calculation.
    /// Returns 0 for Rahu/Ketu (not used in standard calculation).
    pub const fn kaksha_value(self) -> u8 {
        match self {
            Self::Surya => 30,
            Self::Chandra => 16,
            Self::Mangal => 6,
            Self::Buddh => 8,
            Self::Guru => 10,
            Self::Shukra => 12,
            Self::Shani => 1,
            Self::Rahu | Self::Ketu => 0,
        }
    }
}

/// Get the planetary lord of a rashi.
///
/// Standard Vedic lordship assignment (BPHS, universal convention):
/// - Mesha/Vrischika → Mangal (Mars)
/// - Vrishabha/Tula → Shukra (Venus)
/// - Mithuna/Kanya → Buddh (Mercury)
/// - Karka → Chandra (Moon)
/// - Simha → Surya (Sun)
/// - Dhanu/Meena → Guru (Jupiter)
/// - Makara/Kumbha → Shani (Saturn)
pub const fn rashi_lord(rashi: Rashi) -> Graha {
    match rashi {
        Rashi::Mesha => Graha::Mangal,
        Rashi::Vrishabha => Graha::Shukra,
        Rashi::Mithuna => Graha::Buddh,
        Rashi::Karka => Graha::Chandra,
        Rashi::Simha => Graha::Surya,
        Rashi::Kanya => Graha::Buddh,
        Rashi::Tula => Graha::Shukra,
        Rashi::Vrischika => Graha::Mangal,
        Rashi::Dhanu => Graha::Guru,
        Rashi::Makara => Graha::Shani,
        Rashi::Kumbha => Graha::Shani,
        Rashi::Meena => Graha::Guru,
    }
}

/// Get the lord of a rashi by 0-based index.
///
/// Returns None if index >= 12.
pub fn rashi_lord_by_index(rashi_index: u8) -> Option<Graha> {
    if rashi_index >= 12 {
        return None;
    }
    Some(rashi_lord(crate::rashi::ALL_RASHIS[rashi_index as usize]))
}

/// Compute the n-th rashi from a given rashi (0-based indices, 1-based offset).
///
/// `nth_rashi_from(0, 1)` = 0 (same rashi), `nth_rashi_from(0, 2)` = 1 (next rashi).
/// Offset is 1-based (1 = same sign, 12 = previous sign).
pub fn nth_rashi_from(rashi_index: u8, offset: u8) -> u8 {
    ((rashi_index as u16 + offset as u16 - 1) % 12) as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_grahas_count() {
        assert_eq!(ALL_GRAHAS.len(), 9);
    }

    #[test]
    fn sapta_grahas_count() {
        assert_eq!(SAPTA_GRAHAS.len(), 7);
    }

    #[test]
    fn graha_indices_sequential() {
        for (i, g) in ALL_GRAHAS.iter().enumerate() {
            assert_eq!(g.index() as usize, i);
        }
    }

    #[test]
    fn graha_names_nonempty() {
        for g in ALL_GRAHAS {
            assert!(!g.name().is_empty());
            assert!(!g.english_name().is_empty());
        }
    }

    #[test]
    fn naif_codes_present_for_sapta_grahas() {
        for g in SAPTA_GRAHAS {
            assert!(g.naif_code().is_some(), "{} should have NAIF code", g.name());
        }
    }

    #[test]
    fn naif_codes_none_for_nodes() {
        assert!(Graha::Rahu.naif_code().is_none());
        assert!(Graha::Ketu.naif_code().is_none());
    }

    #[test]
    fn rashi_lordship_mesha() {
        assert_eq!(rashi_lord(Rashi::Mesha), Graha::Mangal);
    }

    #[test]
    fn rashi_lordship_simha() {
        assert_eq!(rashi_lord(Rashi::Simha), Graha::Surya);
    }

    #[test]
    fn rashi_lordship_karka() {
        assert_eq!(rashi_lord(Rashi::Karka), Graha::Chandra);
    }

    #[test]
    fn rashi_lordship_dual_ruled() {
        // Mars rules both Mesha and Vrischika
        assert_eq!(rashi_lord(Rashi::Mesha), Graha::Mangal);
        assert_eq!(rashi_lord(Rashi::Vrischika), Graha::Mangal);
        // Venus rules both Vrishabha and Tula
        assert_eq!(rashi_lord(Rashi::Vrishabha), Graha::Shukra);
        assert_eq!(rashi_lord(Rashi::Tula), Graha::Shukra);
        // Mercury rules both Mithuna and Kanya
        assert_eq!(rashi_lord(Rashi::Mithuna), Graha::Buddh);
        assert_eq!(rashi_lord(Rashi::Kanya), Graha::Buddh);
        // Jupiter rules both Dhanu and Meena
        assert_eq!(rashi_lord(Rashi::Dhanu), Graha::Guru);
        assert_eq!(rashi_lord(Rashi::Meena), Graha::Guru);
        // Saturn rules both Makara and Kumbha
        assert_eq!(rashi_lord(Rashi::Makara), Graha::Shani);
        assert_eq!(rashi_lord(Rashi::Kumbha), Graha::Shani);
    }

    #[test]
    fn rashi_lord_by_index_valid() {
        assert_eq!(rashi_lord_by_index(0), Some(Graha::Mangal));
        assert_eq!(rashi_lord_by_index(4), Some(Graha::Surya));
        assert_eq!(rashi_lord_by_index(11), Some(Graha::Guru));
    }

    #[test]
    fn rashi_lord_by_index_invalid() {
        assert_eq!(rashi_lord_by_index(12), None);
        assert_eq!(rashi_lord_by_index(255), None);
    }

    #[test]
    fn nth_rashi_same() {
        assert_eq!(nth_rashi_from(0, 1), 0);
        assert_eq!(nth_rashi_from(5, 1), 5);
    }

    #[test]
    fn nth_rashi_wrap() {
        assert_eq!(nth_rashi_from(11, 2), 0);
        assert_eq!(nth_rashi_from(0, 12), 11);
    }

    #[test]
    fn nth_rashi_eighth() {
        // 8th from Mesha (0): offset 8 → index 7 (Vrischika)
        assert_eq!(nth_rashi_from(0, 8), 7);
    }

    #[test]
    fn kaksha_values_match_array() {
        for (i, g) in SAPTA_GRAHAS.iter().enumerate() {
            assert_eq!(g.kaksha_value(), GRAHA_KAKSHA_VALUES[i]);
        }
    }
}
