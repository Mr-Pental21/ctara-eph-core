//! Tithi (lunar day) computation.
//!
//! The synodic month is divided into 30 tithis, each spanning 12 degrees of
//! Moon-Sun elongation. Shukla Paksha (bright half) runs from 0-180 degrees
//! (tithis 0-14), Krishna Paksha (dark half) from 180-360 degrees (tithis 15-29).
//!
//! Clean-room implementation from standard Vedic convention.

/// The two pakshas (fortnights) of a lunar month.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Paksha {
    /// Bright half (waxing moon), elongation 0-180 deg.
    Shukla,
    /// Dark half (waning moon), elongation 180-360 deg.
    Krishna,
}

impl Paksha {
    /// Name of the paksha.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Shukla => "Shukla",
            Self::Krishna => "Krishna",
        }
    }
}

/// The 30 tithis of a lunar month.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Tithi {
    ShuklaPratipada,
    ShuklaDwitiya,
    ShuklaTritiya,
    ShuklaChaturthi,
    ShuklaPanchami,
    ShuklaShashthi,
    ShuklaSaptami,
    ShuklaAshtami,
    ShuklaNavami,
    ShuklaDashami,
    ShuklaEkadashi,
    ShuklaDwadashi,
    ShuklaTrayodashi,
    ShuklaChaturdashi,
    Purnima,
    KrishnaPratipada,
    KrishnaDwitiya,
    KrishnaTritiya,
    KrishnaChaturthi,
    KrishnaPanchami,
    KrishnaShashthi,
    KrishnaSaptami,
    KrishnaAshtami,
    KrishnaNavami,
    KrishnaDashami,
    KrishnaEkadashi,
    KrishnaDwadashi,
    KrishnaTrayodashi,
    KrishnaChaturdashi,
    Amavasya,
}

/// All 30 tithis in order, for FFI indexing (0 = ShuklaPratipada, 29 = Amavasya).
pub const ALL_TITHIS: [Tithi; 30] = [
    Tithi::ShuklaPratipada,
    Tithi::ShuklaDwitiya,
    Tithi::ShuklaTritiya,
    Tithi::ShuklaChaturthi,
    Tithi::ShuklaPanchami,
    Tithi::ShuklaShashthi,
    Tithi::ShuklaSaptami,
    Tithi::ShuklaAshtami,
    Tithi::ShuklaNavami,
    Tithi::ShuklaDashami,
    Tithi::ShuklaEkadashi,
    Tithi::ShuklaDwadashi,
    Tithi::ShuklaTrayodashi,
    Tithi::ShuklaChaturdashi,
    Tithi::Purnima,
    Tithi::KrishnaPratipada,
    Tithi::KrishnaDwitiya,
    Tithi::KrishnaTritiya,
    Tithi::KrishnaChaturthi,
    Tithi::KrishnaPanchami,
    Tithi::KrishnaShashthi,
    Tithi::KrishnaSaptami,
    Tithi::KrishnaAshtami,
    Tithi::KrishnaNavami,
    Tithi::KrishnaDashami,
    Tithi::KrishnaEkadashi,
    Tithi::KrishnaDwadashi,
    Tithi::KrishnaTrayodashi,
    Tithi::KrishnaChaturdashi,
    Tithi::Amavasya,
];

/// Degrees per tithi.
pub const TITHI_SEGMENT_DEG: f64 = 12.0;

impl Tithi {
    /// Full name of the tithi.
    pub const fn name(self) -> &'static str {
        match self {
            Self::ShuklaPratipada => "Shukla Pratipada",
            Self::ShuklaDwitiya => "Shukla Dwitiya",
            Self::ShuklaTritiya => "Shukla Tritiya",
            Self::ShuklaChaturthi => "Shukla Chaturthi",
            Self::ShuklaPanchami => "Shukla Panchami",
            Self::ShuklaShashthi => "Shukla Shashthi",
            Self::ShuklaSaptami => "Shukla Saptami",
            Self::ShuklaAshtami => "Shukla Ashtami",
            Self::ShuklaNavami => "Shukla Navami",
            Self::ShuklaDashami => "Shukla Dashami",
            Self::ShuklaEkadashi => "Shukla Ekadashi",
            Self::ShuklaDwadashi => "Shukla Dwadashi",
            Self::ShuklaTrayodashi => "Shukla Trayodashi",
            Self::ShuklaChaturdashi => "Shukla Chaturdashi",
            Self::Purnima => "Purnima",
            Self::KrishnaPratipada => "Krishna Pratipada",
            Self::KrishnaDwitiya => "Krishna Dwitiya",
            Self::KrishnaTritiya => "Krishna Tritiya",
            Self::KrishnaChaturthi => "Krishna Chaturthi",
            Self::KrishnaPanchami => "Krishna Panchami",
            Self::KrishnaShashthi => "Krishna Shashthi",
            Self::KrishnaSaptami => "Krishna Saptami",
            Self::KrishnaAshtami => "Krishna Ashtami",
            Self::KrishnaNavami => "Krishna Navami",
            Self::KrishnaDashami => "Krishna Dashami",
            Self::KrishnaEkadashi => "Krishna Ekadashi",
            Self::KrishnaDwadashi => "Krishna Dwadashi",
            Self::KrishnaTrayodashi => "Krishna Trayodashi",
            Self::KrishnaChaturdashi => "Krishna Chaturdashi",
            Self::Amavasya => "Amavasya",
        }
    }

    /// 0-based index (0 = ShuklaPratipada, 29 = Amavasya).
    pub const fn index(self) -> u8 {
        match self {
            Self::ShuklaPratipada => 0,
            Self::ShuklaDwitiya => 1,
            Self::ShuklaTritiya => 2,
            Self::ShuklaChaturthi => 3,
            Self::ShuklaPanchami => 4,
            Self::ShuklaShashthi => 5,
            Self::ShuklaSaptami => 6,
            Self::ShuklaAshtami => 7,
            Self::ShuklaNavami => 8,
            Self::ShuklaDashami => 9,
            Self::ShuklaEkadashi => 10,
            Self::ShuklaDwadashi => 11,
            Self::ShuklaTrayodashi => 12,
            Self::ShuklaChaturdashi => 13,
            Self::Purnima => 14,
            Self::KrishnaPratipada => 15,
            Self::KrishnaDwitiya => 16,
            Self::KrishnaTritiya => 17,
            Self::KrishnaChaturthi => 18,
            Self::KrishnaPanchami => 19,
            Self::KrishnaShashthi => 20,
            Self::KrishnaSaptami => 21,
            Self::KrishnaAshtami => 22,
            Self::KrishnaNavami => 23,
            Self::KrishnaDashami => 24,
            Self::KrishnaEkadashi => 25,
            Self::KrishnaDwadashi => 26,
            Self::KrishnaTrayodashi => 27,
            Self::KrishnaChaturdashi => 28,
            Self::Amavasya => 29,
        }
    }

    /// Paksha of this tithi.
    pub const fn paksha(self) -> Paksha {
        if self.index() < 15 {
            Paksha::Shukla
        } else {
            Paksha::Krishna
        }
    }

    /// 1-based tithi number within the paksha (1-15).
    pub const fn tithi_in_paksha(self) -> u8 {
        (self.index() % 15) + 1
    }
}

/// Result of tithi-from-elongation computation (pure geometry, no times).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TithiPosition {
    /// The tithi.
    pub tithi: Tithi,
    /// 0-based tithi index (0..29).
    pub tithi_index: u8,
    /// Paksha (Shukla or Krishna).
    pub paksha: Paksha,
    /// 1-based tithi number within the paksha (1-15).
    pub tithi_in_paksha: u8,
    /// Degrees into the current tithi [0, 12).
    pub degrees_in_tithi: f64,
}

/// Normalize angle to [0, 360).
fn normalize_360(deg: f64) -> f64 {
    let r = deg % 360.0;
    if r < 0.0 { r + 360.0 } else { r }
}

/// Determine tithi from Moon-Sun elongation (tropical coordinates).
///
/// Elongation = (Moon_lon - Sun_lon) mod 360, in degrees [0, 360).
/// Each tithi spans 12 degrees. Ayanamsha cancels in the difference.
pub fn tithi_from_elongation(elongation_deg: f64) -> TithiPosition {
    let elong = normalize_360(elongation_deg);
    let idx = (elong / TITHI_SEGMENT_DEG).floor() as u8;
    let idx = idx.min(29);
    let tithi = ALL_TITHIS[idx as usize];
    let degrees_in_tithi = elong - (idx as f64) * TITHI_SEGMENT_DEG;

    TithiPosition {
        tithi,
        tithi_index: idx,
        paksha: tithi.paksha(),
        tithi_in_paksha: tithi.tithi_in_paksha(),
        degrees_in_tithi,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_tithis_count() {
        assert_eq!(ALL_TITHIS.len(), 30);
    }

    #[test]
    fn tithi_indices_sequential() {
        for (i, t) in ALL_TITHIS.iter().enumerate() {
            assert_eq!(t.index() as usize, i);
        }
    }

    #[test]
    fn tithi_names_nonempty() {
        for t in ALL_TITHIS {
            assert!(!t.name().is_empty());
        }
    }

    #[test]
    fn tithi_paksha_correct() {
        for t in &ALL_TITHIS[..15] {
            assert_eq!(t.paksha(), Paksha::Shukla);
        }
        for t in &ALL_TITHIS[15..] {
            assert_eq!(t.paksha(), Paksha::Krishna);
        }
    }

    #[test]
    fn tithi_in_paksha_range() {
        for t in ALL_TITHIS {
            let tip = t.tithi_in_paksha();
            assert!((1..=15).contains(&tip), "{:?} tip={}", t, tip);
        }
    }

    #[test]
    fn tithi_from_elongation_zero() {
        let pos = tithi_from_elongation(0.0);
        assert_eq!(pos.tithi, Tithi::ShuklaPratipada);
        assert_eq!(pos.tithi_index, 0);
        assert!(pos.degrees_in_tithi.abs() < 1e-10);
    }

    #[test]
    fn tithi_from_elongation_180() {
        let pos = tithi_from_elongation(180.0);
        assert_eq!(pos.tithi, Tithi::KrishnaPratipada);
        assert_eq!(pos.tithi_index, 15);
    }

    #[test]
    fn tithi_purnima_at_174() {
        let pos = tithi_from_elongation(174.0);
        assert_eq!(pos.tithi, Tithi::Purnima);
        assert_eq!(pos.tithi_index, 14);
        assert!((pos.degrees_in_tithi - 6.0).abs() < 1e-10);
    }

    #[test]
    fn tithi_amavasya_at_354() {
        let pos = tithi_from_elongation(354.0);
        assert_eq!(pos.tithi, Tithi::Amavasya);
        assert_eq!(pos.tithi_index, 29);
        assert!((pos.degrees_in_tithi - 6.0).abs() < 1e-10);
    }

    #[test]
    fn tithi_all_boundaries() {
        for i in 0..30u8 {
            let elong = i as f64 * 12.0;
            let pos = tithi_from_elongation(elong);
            assert_eq!(pos.tithi_index, i, "boundary at {elong} deg");
        }
    }

    #[test]
    fn tithi_wrap_around() {
        let pos = tithi_from_elongation(366.0);
        assert_eq!(pos.tithi, Tithi::ShuklaPratipada);
        assert!((pos.degrees_in_tithi - 6.0).abs() < 1e-10);
    }

    #[test]
    fn tithi_negative() {
        let pos = tithi_from_elongation(-6.0);
        // -6 mod 360 = 354 → Amavasya
        assert_eq!(pos.tithi, Tithi::Amavasya);
    }
}
