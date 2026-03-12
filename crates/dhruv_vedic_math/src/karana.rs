//! Karana (half-tithi) computation.
//!
//! The synodic month is divided into 60 karanas, each spanning 6 degrees of
//! Moon-Sun elongation. There are 7 movable karanas and 4 fixed karanas.
//!
//! Traditional mapping:
//! - Karana 0: Kinstugna (fixed, second half of Amavasya)
//! - Karanas 1-56: cycle through 7 movable (Bava, Balava, Kaulava, Taitilla, Garija, Vanija, Vishti)
//! - Karana 57: Shakuni (fixed)
//! - Karana 58: Chatuspad (fixed)
//! - Karana 59: Naga (fixed)
//!
//! Clean-room implementation from standard Vedic convention.

/// The 11 karana names (7 movable + 4 fixed).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Karana {
    // 7 movable karanas (cycle through karanas 1-56)
    Bava,
    Balava,
    Kaulava,
    Taitilla,
    Garija,
    Vanija,
    Vishti,
    // 4 fixed karanas
    Shakuni,
    Chatuspad,
    Naga,
    Kinstugna,
}

/// All 11 karana names in order (movable first, then fixed), for FFI indexing.
pub const ALL_KARANAS: [Karana; 11] = [
    Karana::Bava,
    Karana::Balava,
    Karana::Kaulava,
    Karana::Taitilla,
    Karana::Garija,
    Karana::Vanija,
    Karana::Vishti,
    Karana::Shakuni,
    Karana::Chatuspad,
    Karana::Naga,
    Karana::Kinstugna,
];

/// The 7 movable karanas in cycle order.
const MOVABLE_KARANAS: [Karana; 7] = [
    Karana::Bava,
    Karana::Balava,
    Karana::Kaulava,
    Karana::Taitilla,
    Karana::Garija,
    Karana::Vanija,
    Karana::Vishti,
];

/// Degrees per karana.
pub const KARANA_SEGMENT_DEG: f64 = 6.0;

impl Karana {
    /// Name of the karana.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Bava => "Bava",
            Self::Balava => "Balava",
            Self::Kaulava => "Kaulava",
            Self::Taitilla => "Taitilla",
            Self::Garija => "Garija",
            Self::Vanija => "Vanija",
            Self::Vishti => "Vishti",
            Self::Shakuni => "Shakuni",
            Self::Chatuspad => "Chatuspad",
            Self::Naga => "Naga",
            Self::Kinstugna => "Kinstugna",
        }
    }

    /// Index in ALL_KARANAS (0 = Bava, 10 = Kinstugna).
    pub const fn index(self) -> u8 {
        match self {
            Self::Bava => 0,
            Self::Balava => 1,
            Self::Kaulava => 2,
            Self::Taitilla => 3,
            Self::Garija => 4,
            Self::Vanija => 5,
            Self::Vishti => 6,
            Self::Shakuni => 7,
            Self::Chatuspad => 8,
            Self::Naga => 9,
            Self::Kinstugna => 10,
        }
    }

    /// Whether this is a fixed karana.
    pub const fn is_fixed(self) -> bool {
        matches!(
            self,
            Self::Shakuni | Self::Chatuspad | Self::Naga | Self::Kinstugna
        )
    }
}

/// Result of karana-from-elongation computation (pure geometry, no times).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct KaranaPosition {
    /// The karana name.
    pub karana: Karana,
    /// 0-based karana sequence index within the synodic month (0..59).
    pub karana_index: u8,
    /// Degrees into the current karana [0, 6).
    pub degrees_in_karana: f64,
}

/// Normalize angle to [0, 360).
fn normalize_360(deg: f64) -> f64 {
    let r = deg % 360.0;
    if r < 0.0 { r + 360.0 } else { r }
}

/// Map karana sequence index (0-59) to the karana name using traditional rules.
fn karana_from_sequence_index(idx: u8) -> Karana {
    match idx {
        0 => Karana::Kinstugna,
        57 => Karana::Shakuni,
        58 => Karana::Chatuspad,
        59 => Karana::Naga,
        i => MOVABLE_KARANAS[((i - 1) % 7) as usize],
    }
}

/// Determine karana from Moon-Sun elongation (tropical coordinates).
///
/// Elongation = (Moon_lon - Sun_lon) mod 360, in degrees [0, 360).
/// Each karana spans 6 degrees. Ayanamsha cancels in the difference.
pub fn karana_from_elongation(elongation_deg: f64) -> KaranaPosition {
    let elong = normalize_360(elongation_deg);
    let idx = (elong / KARANA_SEGMENT_DEG).floor() as u8;
    let idx = idx.min(59);
    let karana = karana_from_sequence_index(idx);
    let degrees_in_karana = elong - (idx as f64) * KARANA_SEGMENT_DEG;

    KaranaPosition {
        karana,
        karana_index: idx,
        degrees_in_karana,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_karanas_count() {
        assert_eq!(ALL_KARANAS.len(), 11);
    }

    #[test]
    fn karana_indices_sequential() {
        for (i, k) in ALL_KARANAS.iter().enumerate() {
            assert_eq!(k.index() as usize, i);
        }
    }

    #[test]
    fn karana_names_nonempty() {
        for k in ALL_KARANAS {
            assert!(!k.name().is_empty());
        }
    }

    #[test]
    fn karana_fixed_correct() {
        assert!(Karana::Shakuni.is_fixed());
        assert!(Karana::Chatuspad.is_fixed());
        assert!(Karana::Naga.is_fixed());
        assert!(Karana::Kinstugna.is_fixed());
        assert!(!Karana::Bava.is_fixed());
        assert!(!Karana::Vishti.is_fixed());
    }

    #[test]
    fn karana_sequence_0_is_kinstugna() {
        let pos = karana_from_elongation(0.0);
        assert_eq!(pos.karana, Karana::Kinstugna);
        assert_eq!(pos.karana_index, 0);
    }

    #[test]
    fn karana_sequence_1_is_bava() {
        let pos = karana_from_elongation(6.0);
        assert_eq!(pos.karana, Karana::Bava);
        assert_eq!(pos.karana_index, 1);
    }

    #[test]
    fn karana_sequence_7_is_vishti() {
        // Index 7: (7-1) % 7 = 6 → Vishti
        let pos = karana_from_elongation(42.0);
        assert_eq!(pos.karana, Karana::Vishti);
        assert_eq!(pos.karana_index, 7);
    }

    #[test]
    fn karana_sequence_8_is_bava_again() {
        // Index 8: (8-1) % 7 = 0 → Bava
        let pos = karana_from_elongation(48.0);
        assert_eq!(pos.karana, Karana::Bava);
        assert_eq!(pos.karana_index, 8);
    }

    #[test]
    fn karana_sequence_57_is_shakuni() {
        let pos = karana_from_elongation(342.0);
        assert_eq!(pos.karana, Karana::Shakuni);
        assert_eq!(pos.karana_index, 57);
    }

    #[test]
    fn karana_sequence_58_is_chatuspad() {
        let pos = karana_from_elongation(348.0);
        assert_eq!(pos.karana, Karana::Chatuspad);
        assert_eq!(pos.karana_index, 58);
    }

    #[test]
    fn karana_sequence_59_is_naga() {
        let pos = karana_from_elongation(354.0);
        assert_eq!(pos.karana, Karana::Naga);
        assert_eq!(pos.karana_index, 59);
    }

    #[test]
    fn karana_movable_cycle_complete() {
        // Karanas 1-7 should be Bava..Vishti
        for i in 1..=7u8 {
            let pos = karana_from_elongation(i as f64 * 6.0);
            assert_eq!(pos.karana, MOVABLE_KARANAS[(i - 1) as usize]);
        }
    }

    #[test]
    fn karana_degrees_in_karana() {
        let pos = karana_from_elongation(9.0);
        assert_eq!(pos.karana_index, 1); // Bava
        assert!((pos.degrees_in_karana - 3.0).abs() < 1e-10);
    }

    #[test]
    fn karana_wrap_around() {
        let pos = karana_from_elongation(363.0);
        // 363 mod 360 = 3 → index 0 (Kinstugna), 3 deg in
        assert_eq!(pos.karana, Karana::Kinstugna);
        assert!((pos.degrees_in_karana - 3.0).abs() < 1e-10);
    }
}
