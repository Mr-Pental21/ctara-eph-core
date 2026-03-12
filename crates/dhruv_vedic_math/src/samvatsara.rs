//! Samvatsara (60-year cycle) enumeration.
//!
//! The 60 samvatsaras cycle continuously. The epoch is CE 1987 = Prabhava (order 1).
//!
//! Clean-room: standard Vedic 60-year cycle names, public domain.

/// The 60 samvatsaras (years) of the Vedic cycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[allow(missing_docs)]
pub enum Samvatsara {
    Prabhava,
    Vibhava,
    Shukla,
    Pramodoota,
    Prajothpatti,
    Angirasa,
    Shrimukha,
    Bhava,
    Yuva,
    Dhaatu,
    Eeshvara,
    Bahudhanya,
    Pramaathi,
    Vikrama,
    Vrisha,
    Chitrabhanu,
    Svabhanu,
    Taarana,
    Paarthiva,
    Vyaya,
    Sarvajit,
    Sarvadhari,
    Virodhi,
    Vikruti,
    Khara,
    Nandana,
    Vijaya,
    Jaya,
    Manmatha,
    Durmukhi,
    Hevilambi,
    Vilambi,
    Vikari,
    Sharvari,
    Plava,
    Shubhakrut,
    Shobhakrut,
    Krodhi,
    Vishvavasu,
    Paraabhava,
    Plavanga,
    Keelaka,
    Saumya,
    Sadharana,
    Virodhikrut,
    Paridhavi,
    Pramaadhi,
    Aananda,
    Raakshasa,
    Naala,
    Pingala,
    Kaalayukti,
    Siddharthi,
    Raudri,
    Durmathi,
    Dundubhi,
    Rudhirodgaari,
    Raktaakshi,
    Krodhana,
    Akshaya,
}

/// All 60 samvatsaras in order (index 0 = Prabhava).
pub const ALL_SAMVATSARAS: [Samvatsara; 60] = [
    Samvatsara::Prabhava,
    Samvatsara::Vibhava,
    Samvatsara::Shukla,
    Samvatsara::Pramodoota,
    Samvatsara::Prajothpatti,
    Samvatsara::Angirasa,
    Samvatsara::Shrimukha,
    Samvatsara::Bhava,
    Samvatsara::Yuva,
    Samvatsara::Dhaatu,
    Samvatsara::Eeshvara,
    Samvatsara::Bahudhanya,
    Samvatsara::Pramaathi,
    Samvatsara::Vikrama,
    Samvatsara::Vrisha,
    Samvatsara::Chitrabhanu,
    Samvatsara::Svabhanu,
    Samvatsara::Taarana,
    Samvatsara::Paarthiva,
    Samvatsara::Vyaya,
    Samvatsara::Sarvajit,
    Samvatsara::Sarvadhari,
    Samvatsara::Virodhi,
    Samvatsara::Vikruti,
    Samvatsara::Khara,
    Samvatsara::Nandana,
    Samvatsara::Vijaya,
    Samvatsara::Jaya,
    Samvatsara::Manmatha,
    Samvatsara::Durmukhi,
    Samvatsara::Hevilambi,
    Samvatsara::Vilambi,
    Samvatsara::Vikari,
    Samvatsara::Sharvari,
    Samvatsara::Plava,
    Samvatsara::Shubhakrut,
    Samvatsara::Shobhakrut,
    Samvatsara::Krodhi,
    Samvatsara::Vishvavasu,
    Samvatsara::Paraabhava,
    Samvatsara::Plavanga,
    Samvatsara::Keelaka,
    Samvatsara::Saumya,
    Samvatsara::Sadharana,
    Samvatsara::Virodhikrut,
    Samvatsara::Paridhavi,
    Samvatsara::Pramaadhi,
    Samvatsara::Aananda,
    Samvatsara::Raakshasa,
    Samvatsara::Naala,
    Samvatsara::Pingala,
    Samvatsara::Kaalayukti,
    Samvatsara::Siddharthi,
    Samvatsara::Raudri,
    Samvatsara::Durmathi,
    Samvatsara::Dundubhi,
    Samvatsara::Rudhirodgaari,
    Samvatsara::Raktaakshi,
    Samvatsara::Krodhana,
    Samvatsara::Akshaya,
];

impl Samvatsara {
    /// Sanskrit name of the samvatsara.
    pub fn name(self) -> &'static str {
        ALL_SAMVATSARA_NAMES[self.index() as usize]
    }

    /// 0-based index (Prabhava=0 .. Akshaya=59).
    pub const fn index(self) -> u8 {
        match self {
            Self::Prabhava => 0,
            Self::Vibhava => 1,
            Self::Shukla => 2,
            Self::Pramodoota => 3,
            Self::Prajothpatti => 4,
            Self::Angirasa => 5,
            Self::Shrimukha => 6,
            Self::Bhava => 7,
            Self::Yuva => 8,
            Self::Dhaatu => 9,
            Self::Eeshvara => 10,
            Self::Bahudhanya => 11,
            Self::Pramaathi => 12,
            Self::Vikrama => 13,
            Self::Vrisha => 14,
            Self::Chitrabhanu => 15,
            Self::Svabhanu => 16,
            Self::Taarana => 17,
            Self::Paarthiva => 18,
            Self::Vyaya => 19,
            Self::Sarvajit => 20,
            Self::Sarvadhari => 21,
            Self::Virodhi => 22,
            Self::Vikruti => 23,
            Self::Khara => 24,
            Self::Nandana => 25,
            Self::Vijaya => 26,
            Self::Jaya => 27,
            Self::Manmatha => 28,
            Self::Durmukhi => 29,
            Self::Hevilambi => 30,
            Self::Vilambi => 31,
            Self::Vikari => 32,
            Self::Sharvari => 33,
            Self::Plava => 34,
            Self::Shubhakrut => 35,
            Self::Shobhakrut => 36,
            Self::Krodhi => 37,
            Self::Vishvavasu => 38,
            Self::Paraabhava => 39,
            Self::Plavanga => 40,
            Self::Keelaka => 41,
            Self::Saumya => 42,
            Self::Sadharana => 43,
            Self::Virodhikrut => 44,
            Self::Paridhavi => 45,
            Self::Pramaadhi => 46,
            Self::Aananda => 47,
            Self::Raakshasa => 48,
            Self::Naala => 49,
            Self::Pingala => 50,
            Self::Kaalayukti => 51,
            Self::Siddharthi => 52,
            Self::Raudri => 53,
            Self::Durmathi => 54,
            Self::Dundubhi => 55,
            Self::Rudhirodgaari => 56,
            Self::Raktaakshi => 57,
            Self::Krodhana => 58,
            Self::Akshaya => 59,
        }
    }
}

const ALL_SAMVATSARA_NAMES: [&str; 60] = [
    "Prabhava",
    "Vibhava",
    "Shukla",
    "Pramodoota",
    "Prajothpatti",
    "Angirasa",
    "Shrimukha",
    "Bhava",
    "Yuva",
    "Dhaatu",
    "Eeshvara",
    "Bahudhanya",
    "Pramaathi",
    "Vikrama",
    "Vrisha",
    "Chitrabhanu",
    "Svabhanu",
    "Taarana",
    "Paarthiva",
    "Vyaya",
    "Sarvajit",
    "Sarvadhari",
    "Virodhi",
    "Vikruti",
    "Khara",
    "Nandana",
    "Vijaya",
    "Jaya",
    "Manmatha",
    "Durmukhi",
    "Hevilambi",
    "Vilambi",
    "Vikari",
    "Sharvari",
    "Plava",
    "Shubhakrut",
    "Shobhakrut",
    "Krodhi",
    "Vishvavasu",
    "Paraabhava",
    "Plavanga",
    "Keelaka",
    "Saumya",
    "Sadharana",
    "Virodhikrut",
    "Paridhavi",
    "Pramaadhi",
    "Aananda",
    "Raakshasa",
    "Naala",
    "Pingala",
    "Kaalayukti",
    "Siddharthi",
    "Raudri",
    "Durmathi",
    "Dundubhi",
    "Rudhirodgaari",
    "Raktaakshi",
    "Krodhana",
    "Akshaya",
];

/// Reference epoch: CE 1987 = Prabhava (order 1, index 0).
pub const SAMVATSARA_EPOCH_YEAR: i32 = 1987;

/// Determine the samvatsara for a given CE year.
///
/// Returns `(samvatsara, order)` where order is 1-based (1..=60).
pub fn samvatsara_from_year(ce_year: i32) -> (Samvatsara, u8) {
    let offset = (ce_year - SAMVATSARA_EPOCH_YEAR).rem_euclid(60) as u8;
    let samvatsara = ALL_SAMVATSARAS[offset as usize];
    (samvatsara, offset + 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_samvatsaras_count() {
        assert_eq!(ALL_SAMVATSARAS.len(), 60);
    }

    #[test]
    fn indices_sequential() {
        for (i, s) in ALL_SAMVATSARAS.iter().enumerate() {
            assert_eq!(s.index() as usize, i);
        }
    }

    #[test]
    fn names_nonempty() {
        for s in ALL_SAMVATSARAS {
            assert!(!s.name().is_empty());
        }
    }

    #[test]
    fn epoch_year_is_prabhava() {
        let (s, order) = samvatsara_from_year(1987);
        assert_eq!(s, Samvatsara::Prabhava);
        assert_eq!(order, 1);
    }

    #[test]
    fn year_1988_is_vibhava() {
        let (s, order) = samvatsara_from_year(1988);
        assert_eq!(s, Samvatsara::Vibhava);
        assert_eq!(order, 2);
    }

    #[test]
    fn year_2046_wraps_to_prabhava() {
        // 1987 + 60 = 2047
        let (s, order) = samvatsara_from_year(2047);
        assert_eq!(s, Samvatsara::Prabhava);
        assert_eq!(order, 1);
    }

    #[test]
    fn year_2024() {
        // 2024 - 1987 = 37, index 37 = Krodhi, order 38
        let (s, order) = samvatsara_from_year(2024);
        assert_eq!(s, Samvatsara::Krodhi);
        assert_eq!(order, 38);
    }

    #[test]
    fn year_before_epoch() {
        // 1986: 1986-1987 = -1, rem_euclid(60) = 59 → Akshaya, order 60
        let (s, order) = samvatsara_from_year(1986);
        assert_eq!(s, Samvatsara::Akshaya);
        assert_eq!(order, 60);
    }
}
