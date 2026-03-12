//! Ayana (solstice period) enumeration.
//!
//! - Uttarayana: Sun in Makara through Mithuna (sidereal 270°-360° and 0°-90°)
//! - Dakshinayana: Sun in Karka through Dhanu (sidereal 90°-270°)
//!
//! Clean-room: universal Vedic convention, no copyrighted source.

/// The two solstice periods.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Ayana {
    Uttarayana,
    Dakshinayana,
}

/// All 2 ayanas in order, for indexing (0 = Uttarayana, 1 = Dakshinayana).
pub const ALL_AYANAS: [Ayana; 2] = [Ayana::Uttarayana, Ayana::Dakshinayana];

impl Ayana {
    /// Sanskrit name of the ayana.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Uttarayana => "Uttarayana",
            Self::Dakshinayana => "Dakshinayana",
        }
    }

    /// 0-based index (Uttarayana=0, Dakshinayana=1).
    pub const fn index(self) -> u8 {
        match self {
            Self::Uttarayana => 0,
            Self::Dakshinayana => 1,
        }
    }
}

/// Determine ayana from sidereal Sun longitude.
///
/// Uttarayana: 270 <= lon < 360 OR 0 <= lon < 90 (Makara through Mithuna).
/// Dakshinayana: 90 <= lon < 270 (Karka through Dhanu).
pub fn ayana_from_sidereal_longitude(lon: f64) -> Ayana {
    let l = lon.rem_euclid(360.0);
    if (270.0..360.0).contains(&l) || (0.0..90.0).contains(&l) {
        Ayana::Uttarayana
    } else {
        Ayana::Dakshinayana
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uttarayana_at_270() {
        assert_eq!(ayana_from_sidereal_longitude(270.0), Ayana::Uttarayana);
    }

    #[test]
    fn uttarayana_at_0() {
        assert_eq!(ayana_from_sidereal_longitude(0.0), Ayana::Uttarayana);
    }

    #[test]
    fn uttarayana_at_45() {
        assert_eq!(ayana_from_sidereal_longitude(45.0), Ayana::Uttarayana);
    }

    #[test]
    fn uttarayana_at_350() {
        assert_eq!(ayana_from_sidereal_longitude(350.0), Ayana::Uttarayana);
    }

    #[test]
    fn dakshinayana_at_90() {
        assert_eq!(ayana_from_sidereal_longitude(90.0), Ayana::Dakshinayana);
    }

    #[test]
    fn dakshinayana_at_180() {
        assert_eq!(ayana_from_sidereal_longitude(180.0), Ayana::Dakshinayana);
    }

    #[test]
    fn dakshinayana_at_269() {
        assert_eq!(ayana_from_sidereal_longitude(269.0), Ayana::Dakshinayana);
    }

    #[test]
    fn negative_wraps() {
        // -10 deg → 350 deg → Uttarayana
        assert_eq!(ayana_from_sidereal_longitude(-10.0), Ayana::Uttarayana);
    }

    #[test]
    fn all_ayanas_count() {
        assert_eq!(ALL_AYANAS.len(), 2);
    }
}
