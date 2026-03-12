//! Yoga (luni-solar yoga) computation.
//!
//! The sum of Moon and Sun sidereal longitudes (mod 360) is divided into
//! 27 yogas, each spanning 360/27 = 13 deg 20 min (13.3333... degrees).
//!
//! Clean-room implementation from standard Vedic convention.

/// The 27 yogas.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Yoga {
    Vishkumbha,
    Priti,
    Ayushman,
    Saubhagya,
    Shobhana,
    Atiganda,
    Sukarma,
    Dhriti,
    Shoola,
    Ganda,
    Vriddhi,
    Dhruva,
    Vyaghata,
    Harshana,
    Vajra,
    Siddhi,
    Vyatipata,
    Variyana,
    Parigha,
    Shiva,
    Siddha,
    Sadhya,
    Shubha,
    Shukla,
    Brahma,
    Indra,
    Vaidhriti,
}

/// All 27 yogas in order, for FFI indexing (0 = Vishkumbha, 26 = Vaidhriti).
pub const ALL_YOGAS: [Yoga; 27] = [
    Yoga::Vishkumbha,
    Yoga::Priti,
    Yoga::Ayushman,
    Yoga::Saubhagya,
    Yoga::Shobhana,
    Yoga::Atiganda,
    Yoga::Sukarma,
    Yoga::Dhriti,
    Yoga::Shoola,
    Yoga::Ganda,
    Yoga::Vriddhi,
    Yoga::Dhruva,
    Yoga::Vyaghata,
    Yoga::Harshana,
    Yoga::Vajra,
    Yoga::Siddhi,
    Yoga::Vyatipata,
    Yoga::Variyana,
    Yoga::Parigha,
    Yoga::Shiva,
    Yoga::Siddha,
    Yoga::Sadhya,
    Yoga::Shubha,
    Yoga::Shukla,
    Yoga::Brahma,
    Yoga::Indra,
    Yoga::Vaidhriti,
];

/// Degrees per yoga (360/27 = 13.3333...).
pub const YOGA_SEGMENT_DEG: f64 = 360.0 / 27.0;

impl Yoga {
    /// Name of the yoga.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Vishkumbha => "Vishkumbha",
            Self::Priti => "Priti",
            Self::Ayushman => "Ayushman",
            Self::Saubhagya => "Saubhagya",
            Self::Shobhana => "Shobhana",
            Self::Atiganda => "Atiganda",
            Self::Sukarma => "Sukarma",
            Self::Dhriti => "Dhriti",
            Self::Shoola => "Shoola",
            Self::Ganda => "Ganda",
            Self::Vriddhi => "Vriddhi",
            Self::Dhruva => "Dhruva",
            Self::Vyaghata => "Vyaghata",
            Self::Harshana => "Harshana",
            Self::Vajra => "Vajra",
            Self::Siddhi => "Siddhi",
            Self::Vyatipata => "Vyatipata",
            Self::Variyana => "Variyana",
            Self::Parigha => "Parigha",
            Self::Shiva => "Shiva",
            Self::Siddha => "Siddha",
            Self::Sadhya => "Sadhya",
            Self::Shubha => "Shubha",
            Self::Shukla => "Shukla",
            Self::Brahma => "Brahma",
            Self::Indra => "Indra",
            Self::Vaidhriti => "Vaidhriti",
        }
    }

    /// 0-based index (0 = Vishkumbha, 26 = Vaidhriti).
    pub const fn index(self) -> u8 {
        match self {
            Self::Vishkumbha => 0,
            Self::Priti => 1,
            Self::Ayushman => 2,
            Self::Saubhagya => 3,
            Self::Shobhana => 4,
            Self::Atiganda => 5,
            Self::Sukarma => 6,
            Self::Dhriti => 7,
            Self::Shoola => 8,
            Self::Ganda => 9,
            Self::Vriddhi => 10,
            Self::Dhruva => 11,
            Self::Vyaghata => 12,
            Self::Harshana => 13,
            Self::Vajra => 14,
            Self::Siddhi => 15,
            Self::Vyatipata => 16,
            Self::Variyana => 17,
            Self::Parigha => 18,
            Self::Shiva => 19,
            Self::Siddha => 20,
            Self::Sadhya => 21,
            Self::Shubha => 22,
            Self::Shukla => 23,
            Self::Brahma => 24,
            Self::Indra => 25,
            Self::Vaidhriti => 26,
        }
    }
}

/// Result of yoga-from-sum computation (pure geometry, no times).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct YogaPosition {
    /// The yoga.
    pub yoga: Yoga,
    /// 0-based yoga index (0..26).
    pub yoga_index: u8,
    /// Degrees into the current yoga [0, 13.333...).
    pub degrees_in_yoga: f64,
}

/// Normalize angle to [0, 360).
fn normalize_360(deg: f64) -> f64 {
    let r = deg % 360.0;
    if r < 0.0 { r + 360.0 } else { r }
}

/// Determine yoga from the sum of Moon and Sun sidereal longitudes.
///
/// sum_deg = (Moon_sidereal + Sun_sidereal) mod 360.
/// Each yoga spans 360/27 = 13.3333... degrees.
pub fn yoga_from_sum(sum_deg: f64) -> YogaPosition {
    let sum = normalize_360(sum_deg);
    let idx = (sum / YOGA_SEGMENT_DEG).floor() as u8;
    let idx = idx.min(26);
    let yoga = ALL_YOGAS[idx as usize];
    let degrees_in_yoga = sum - (idx as f64) * YOGA_SEGMENT_DEG;

    YogaPosition {
        yoga,
        yoga_index: idx,
        degrees_in_yoga,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_yogas_count() {
        assert_eq!(ALL_YOGAS.len(), 27);
    }

    #[test]
    fn yoga_indices_sequential() {
        for (i, y) in ALL_YOGAS.iter().enumerate() {
            assert_eq!(y.index() as usize, i);
        }
    }

    #[test]
    fn yoga_names_nonempty() {
        for y in ALL_YOGAS {
            assert!(!y.name().is_empty());
        }
    }

    #[test]
    fn yoga_from_sum_zero() {
        let pos = yoga_from_sum(0.0);
        assert_eq!(pos.yoga, Yoga::Vishkumbha);
        assert_eq!(pos.yoga_index, 0);
        assert!(pos.degrees_in_yoga.abs() < 1e-10);
    }

    #[test]
    fn yoga_from_sum_last() {
        // Last yoga starts at 26 * (360/27) = 346.666... deg
        let pos = yoga_from_sum(350.0);
        assert_eq!(pos.yoga, Yoga::Vaidhriti);
        assert_eq!(pos.yoga_index, 26);
    }

    #[test]
    fn yoga_all_boundaries() {
        for i in 0..27u8 {
            let sum = i as f64 * YOGA_SEGMENT_DEG;
            let pos = yoga_from_sum(sum);
            assert_eq!(pos.yoga_index, i, "boundary at {sum} deg");
        }
    }

    #[test]
    fn yoga_degrees_in_yoga() {
        let seg = YOGA_SEGMENT_DEG;
        let pos = yoga_from_sum(seg + 5.0);
        assert_eq!(pos.yoga_index, 1); // Priti
        assert!((pos.degrees_in_yoga - 5.0).abs() < 1e-10);
    }

    #[test]
    fn yoga_wrap_around() {
        let pos = yoga_from_sum(365.0);
        assert_eq!(pos.yoga, Yoga::Vishkumbha);
        assert!((pos.degrees_in_yoga - 5.0).abs() < 1e-10);
    }

    #[test]
    fn yoga_segment_deg_correct() {
        assert!((YOGA_SEGMENT_DEG - 13.333_333_333_333_334).abs() < 1e-10);
    }
}
