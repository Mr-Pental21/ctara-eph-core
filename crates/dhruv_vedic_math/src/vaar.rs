//! Vaar (weekday) computation.
//!
//! The Vedic day (vaar) runs from sunrise to the next sunrise.
//! The weekday of the sunrise determines the vaar.
//!
//! Clean-room implementation: standard 7-day week cycle, universal convention.

/// The 7 vaars (weekdays).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Vaar {
    Ravivaar,
    Somvaar,
    Mangalvaar,
    Budhvaar,
    Guruvaar,
    Shukravaar,
    Shanivaar,
}

/// All 7 vaars in order (Sunday-first), for FFI indexing.
pub const ALL_VAARS: [Vaar; 7] = [
    Vaar::Ravivaar,
    Vaar::Somvaar,
    Vaar::Mangalvaar,
    Vaar::Budhvaar,
    Vaar::Guruvaar,
    Vaar::Shukravaar,
    Vaar::Shanivaar,
];

impl Vaar {
    /// Sanskrit name of the vaar.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Ravivaar => "Ravivaar",
            Self::Somvaar => "Somvaar",
            Self::Mangalvaar => "Mangalvaar",
            Self::Budhvaar => "Budhvaar",
            Self::Guruvaar => "Guruvaar",
            Self::Shukravaar => "Shukravaar",
            Self::Shanivaar => "Shanivaar",
        }
    }

    /// English weekday name.
    pub const fn english_name(self) -> &'static str {
        match self {
            Self::Ravivaar => "Sunday",
            Self::Somvaar => "Monday",
            Self::Mangalvaar => "Tuesday",
            Self::Budhvaar => "Wednesday",
            Self::Guruvaar => "Thursday",
            Self::Shukravaar => "Friday",
            Self::Shanivaar => "Saturday",
        }
    }

    /// 0-based index (Ravivaar=0, Shanivaar=6).
    pub const fn index(self) -> u8 {
        match self {
            Self::Ravivaar => 0,
            Self::Somvaar => 1,
            Self::Mangalvaar => 2,
            Self::Budhvaar => 3,
            Self::Guruvaar => 4,
            Self::Shukravaar => 5,
            Self::Shanivaar => 6,
        }
    }
}

/// Determine vaar from Julian Date.
///
/// Uses the standard JD weekday formula: weekday = (floor(JD + 0.5) + 1) mod 7
/// where 0=Sunday. This gives the civil weekday at 0h UT on the JD.
pub fn vaar_from_jd(jd: f64) -> Vaar {
    // JD 0.0 = Monday (JD 0.5 = noon Monday, Jan 1 4713 BC Julian)
    // floor(JD + 0.5) gives the Julian Day Number
    // (JDN + 1) mod 7: 0=Sunday, 1=Monday, ..., 6=Saturday
    let jdn = (jd + 0.5).floor() as i64;
    let weekday = ((jdn + 1) % 7 + 7) % 7; // ensure positive
    ALL_VAARS[weekday as usize]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_vaars_count() {
        assert_eq!(ALL_VAARS.len(), 7);
    }

    #[test]
    fn vaar_indices_sequential() {
        for (i, v) in ALL_VAARS.iter().enumerate() {
            assert_eq!(v.index() as usize, i);
        }
    }

    #[test]
    fn vaar_names_nonempty() {
        for v in ALL_VAARS {
            assert!(!v.name().is_empty());
            assert!(!v.english_name().is_empty());
        }
    }

    #[test]
    fn vaar_j2000_is_saturday() {
        // J2000.0 = 2000-01-01 12:00 TT, which is a Saturday
        let v = vaar_from_jd(2_451_545.0);
        assert_eq!(v, Vaar::Shanivaar);
    }

    #[test]
    fn vaar_known_monday() {
        // 2024-01-01 is a Monday → JD 2460310.5
        let v = vaar_from_jd(2_460_310.5);
        assert_eq!(v, Vaar::Somvaar);
    }

    #[test]
    fn vaar_known_sunday() {
        // 2024-01-07 is a Sunday → JD 2460316.5
        let v = vaar_from_jd(2_460_316.5);
        assert_eq!(v, Vaar::Ravivaar);
    }

    #[test]
    fn vaar_known_friday() {
        // 2024-01-05 is a Friday → JD 2460314.5
        let v = vaar_from_jd(2_460_314.5);
        assert_eq!(v, Vaar::Shukravaar);
    }
}
