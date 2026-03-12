//! Masa (lunar month) enumeration and rashi-to-masa mapping.
//!
//! The 12 masas are in 1:1 correspondence with the 12 rashis:
//! Mesha→Chaitra, Vrishabha→Vaishakha, etc.
//!
//! Clean-room: universal Vedic convention, no copyrighted source.

/// The 12 lunar months.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Masa {
    Chaitra,
    Vaishakha,
    Jyeshtha,
    Ashadha,
    Shravana,
    Bhadrapada,
    Ashvina,
    Kartika,
    Margashirsha,
    Pausha,
    Magha,
    Phalguna,
}

/// All 12 masas in order, for indexing (0 = Chaitra, 11 = Phalguna).
pub const ALL_MASAS: [Masa; 12] = [
    Masa::Chaitra,
    Masa::Vaishakha,
    Masa::Jyeshtha,
    Masa::Ashadha,
    Masa::Shravana,
    Masa::Bhadrapada,
    Masa::Ashvina,
    Masa::Kartika,
    Masa::Margashirsha,
    Masa::Pausha,
    Masa::Magha,
    Masa::Phalguna,
];

impl Masa {
    /// Sanskrit name of the masa.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Chaitra => "Chaitra",
            Self::Vaishakha => "Vaishakha",
            Self::Jyeshtha => "Jyeshtha",
            Self::Ashadha => "Ashadha",
            Self::Shravana => "Shravana",
            Self::Bhadrapada => "Bhadrapada",
            Self::Ashvina => "Ashvina",
            Self::Kartika => "Kartika",
            Self::Margashirsha => "Margashirsha",
            Self::Pausha => "Pausha",
            Self::Magha => "Magha",
            Self::Phalguna => "Phalguna",
        }
    }

    /// 0-based index (Chaitra=0 .. Phalguna=11).
    pub const fn index(self) -> u8 {
        match self {
            Self::Chaitra => 0,
            Self::Vaishakha => 1,
            Self::Jyeshtha => 2,
            Self::Ashadha => 3,
            Self::Shravana => 4,
            Self::Bhadrapada => 5,
            Self::Ashvina => 6,
            Self::Kartika => 7,
            Self::Margashirsha => 8,
            Self::Pausha => 9,
            Self::Magha => 10,
            Self::Phalguna => 11,
        }
    }
}

/// Map rashi index (0=Mesha .. 11=Meena) to corresponding Masa.
///
/// Mesha→Chaitra, Vrishabha→Vaishakha, etc.
pub fn masa_from_rashi_index(idx: u8) -> Masa {
    ALL_MASAS[(idx % 12) as usize]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_masas_count() {
        assert_eq!(ALL_MASAS.len(), 12);
    }

    #[test]
    fn masa_indices_sequential() {
        for (i, m) in ALL_MASAS.iter().enumerate() {
            assert_eq!(m.index() as usize, i);
        }
    }

    #[test]
    fn masa_names_nonempty() {
        for m in ALL_MASAS {
            assert!(!m.name().is_empty());
        }
    }

    #[test]
    fn mesha_is_chaitra() {
        assert_eq!(masa_from_rashi_index(0), Masa::Chaitra);
    }

    #[test]
    fn meena_is_phalguna() {
        assert_eq!(masa_from_rashi_index(11), Masa::Phalguna);
    }

    #[test]
    fn wrap_around() {
        assert_eq!(masa_from_rashi_index(12), Masa::Chaitra);
    }
}
