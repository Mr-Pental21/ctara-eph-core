//! Hora (planetary hour) computation.
//!
//! A Vedic day (sunrise to next sunrise) is divided into 24 equal horas.
//! The ruling planet of each hora follows the Chaldean sequence, starting
//! from the day lord (vaar lord) at sunrise.
//!
//! Chaldean (decreasing orbital period) sequence:
//! Surya, Shukra, Buddh, Chandra, Shani, Guru, Mangal
//!
//! Clean-room implementation: standard Chaldean planetary hour convention.

use crate::vaar::Vaar;

/// The 7 hora lords (planets), in Chaldean descending-speed order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Hora {
    Surya,
    Shukra,
    Buddh,
    Chandra,
    Shani,
    Guru,
    Mangal,
}

/// Chaldean sequence: Surya → Shukra → Buddh → Chandra → Shani → Guru → Mangal.
pub const CHALDEAN_SEQUENCE: [Hora; 7] = [
    Hora::Surya,
    Hora::Shukra,
    Hora::Buddh,
    Hora::Chandra,
    Hora::Shani,
    Hora::Guru,
    Hora::Mangal,
];

/// Number of horas per Vedic day.
pub const HORA_COUNT: u8 = 24;

impl Hora {
    /// Name of the hora lord.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Surya => "Surya",
            Self::Shukra => "Shukra",
            Self::Buddh => "Buddh",
            Self::Chandra => "Chandra",
            Self::Shani => "Shani",
            Self::Guru => "Guru",
            Self::Mangal => "Mangal",
        }
    }

    /// Index in CHALDEAN_SEQUENCE (0 = Surya, 6 = Mangal).
    pub const fn index(self) -> u8 {
        match self {
            Self::Surya => 0,
            Self::Shukra => 1,
            Self::Buddh => 2,
            Self::Chandra => 3,
            Self::Shani => 4,
            Self::Guru => 5,
            Self::Mangal => 6,
        }
    }
}

/// Map vaar to its corresponding day lord in the Chaldean sequence.
pub const fn vaar_day_lord(vaar: Vaar) -> Hora {
    match vaar {
        Vaar::Ravivaar => Hora::Surya,
        Vaar::Somvaar => Hora::Chandra,
        Vaar::Mangalvaar => Hora::Mangal,
        Vaar::Budhvaar => Hora::Buddh,
        Vaar::Guruvaar => Hora::Guru,
        Vaar::Shukravaar => Hora::Shukra,
        Vaar::Shanivaar => Hora::Shani,
    }
}

/// Determine the hora lord for a given hora index within a Vedic day.
///
/// `hora_index` is 0-based (0 = first hora at sunrise, 23 = last hora).
/// `vaar` is the weekday of the Vedic day (determined by sunrise).
///
/// The first hora is ruled by the day lord. Subsequent horas follow
/// the Chaldean sequence cyclically.
pub fn hora_at(vaar: Vaar, hora_index: u8) -> Hora {
    let day_lord = vaar_day_lord(vaar);
    let offset = day_lord.index();
    CHALDEAN_SEQUENCE[((offset as u16 + hora_index as u16) % 7) as usize]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chaldean_sequence_count() {
        assert_eq!(CHALDEAN_SEQUENCE.len(), 7);
    }

    #[test]
    fn hora_indices_sequential() {
        for (i, h) in CHALDEAN_SEQUENCE.iter().enumerate() {
            assert_eq!(h.index() as usize, i);
        }
    }

    #[test]
    fn hora_names_nonempty() {
        for h in CHALDEAN_SEQUENCE {
            assert!(!h.name().is_empty());
        }
    }

    #[test]
    fn sunday_first_hora_is_surya() {
        assert_eq!(hora_at(Vaar::Ravivaar, 0), Hora::Surya);
    }

    #[test]
    fn sunday_second_hora_is_shukra() {
        assert_eq!(hora_at(Vaar::Ravivaar, 1), Hora::Shukra);
    }

    #[test]
    fn monday_first_hora_is_chandra() {
        assert_eq!(hora_at(Vaar::Somvaar, 0), Hora::Chandra);
    }

    #[test]
    fn saturday_first_hora_is_shani() {
        assert_eq!(hora_at(Vaar::Shanivaar, 0), Hora::Shani);
    }

    #[test]
    fn hora_wraps_cyclically() {
        // Sunday hora 7 should wrap back to Surya (7 % 7 = 0)
        assert_eq!(hora_at(Vaar::Ravivaar, 7), Hora::Surya);
    }

    #[test]
    fn hora_all_24_valid() {
        for i in 0..24u8 {
            let _ = hora_at(Vaar::Ravivaar, i);
        }
    }

    #[test]
    fn vaar_day_lord_mapping() {
        assert_eq!(vaar_day_lord(Vaar::Ravivaar), Hora::Surya);
        assert_eq!(vaar_day_lord(Vaar::Somvaar), Hora::Chandra);
        assert_eq!(vaar_day_lord(Vaar::Mangalvaar), Hora::Mangal);
        assert_eq!(vaar_day_lord(Vaar::Budhvaar), Hora::Buddh);
        assert_eq!(vaar_day_lord(Vaar::Guruvaar), Hora::Guru);
        assert_eq!(vaar_day_lord(Vaar::Shukravaar), Hora::Shukra);
        assert_eq!(vaar_day_lord(Vaar::Shanivaar), Hora::Shani);
    }
}
