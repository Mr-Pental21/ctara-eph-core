//! Combustion (Asta/Moudhya) detection for grahas.
//!
//! A graha is combust when it is too close to the Sun, with thresholds
//! varying by planet and retrograde status per BPHS.
//!
//! Clean-room implementation from BPHS combustion thresholds.

use crate::graha::{ALL_GRAHAS, Graha};
use crate::util::normalize_360;

/// BPHS combustion threshold (degrees from Sun) for a graha.
///
/// Returns `None` for Sun, Rahu, and Ketu (not applicable).
/// For Mercury and Venus, retrograde thresholds are tighter.
pub fn combustion_threshold(graha: Graha, is_retrograde: bool) -> Option<f64> {
    match graha {
        Graha::Surya | Graha::Rahu | Graha::Ketu => None,
        Graha::Chandra => Some(12.0),
        Graha::Mangal => Some(17.0),
        Graha::Buddh => {
            if is_retrograde {
                Some(12.0)
            } else {
                Some(14.0)
            }
        }
        Graha::Guru => Some(11.0),
        Graha::Shukra => {
            if is_retrograde {
                Some(8.0)
            } else {
                Some(10.0)
            }
        }
        Graha::Shani => Some(15.0),
    }
}

/// Check if a single graha is combust (too close to the Sun).
///
/// Uses angular distance on the ecliptic circle. A graha at exactly the
/// threshold distance is **not** combust (strict less-than).
/// Returns `false` for Sun, Rahu, and Ketu.
pub fn is_combust(graha: Graha, graha_sid_lon: f64, sun_sid_lon: f64, is_retrograde: bool) -> bool {
    let threshold = match combustion_threshold(graha, is_retrograde) {
        Some(t) => t,
        None => return false,
    };
    let diff = (normalize_360(graha_sid_lon) - normalize_360(sun_sid_lon)).abs();
    let angular_distance = if diff > 180.0 { 360.0 - diff } else { diff };
    angular_distance < threshold
}

/// Check combustion status for all 9 grahas at once.
///
/// `sidereal_lons[0]` = Sun. Indices follow `Graha::index()`.
/// `is_retrograde` flags: only meaningful for indices 0-6 (sapta grahas);
/// Rahu/Ketu always return false.
pub fn all_combustion_status(sidereal_lons: &[f64; 9], is_retrograde: &[bool; 9]) -> [bool; 9] {
    let sun_lon = sidereal_lons[Graha::Surya.index() as usize];
    let mut result = [false; 9];
    for graha in ALL_GRAHAS {
        let idx = graha.index() as usize;
        result[idx] = is_combust(graha, sidereal_lons[idx], sun_lon, is_retrograde[idx]);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn threshold_sun_none() {
        assert!(combustion_threshold(Graha::Surya, false).is_none());
    }

    #[test]
    fn threshold_rahu_none() {
        assert!(combustion_threshold(Graha::Rahu, false).is_none());
    }

    #[test]
    fn threshold_ketu_none() {
        assert!(combustion_threshold(Graha::Ketu, false).is_none());
    }

    #[test]
    fn threshold_moon() {
        assert_eq!(combustion_threshold(Graha::Chandra, false), Some(12.0));
        assert_eq!(combustion_threshold(Graha::Chandra, true), Some(12.0));
    }

    #[test]
    fn threshold_mars() {
        assert_eq!(combustion_threshold(Graha::Mangal, false), Some(17.0));
        assert_eq!(combustion_threshold(Graha::Mangal, true), Some(17.0));
    }

    #[test]
    fn threshold_mercury_direct_vs_retrograde() {
        assert_eq!(combustion_threshold(Graha::Buddh, false), Some(14.0));
        assert_eq!(combustion_threshold(Graha::Buddh, true), Some(12.0));
    }

    #[test]
    fn threshold_jupiter() {
        assert_eq!(combustion_threshold(Graha::Guru, false), Some(11.0));
    }

    #[test]
    fn threshold_venus_direct_vs_retrograde() {
        assert_eq!(combustion_threshold(Graha::Shukra, false), Some(10.0));
        assert_eq!(combustion_threshold(Graha::Shukra, true), Some(8.0));
    }

    #[test]
    fn threshold_saturn() {
        assert_eq!(combustion_threshold(Graha::Shani, false), Some(15.0));
    }

    #[test]
    fn is_combust_sun_always_false() {
        assert!(!is_combust(Graha::Surya, 100.0, 100.0, false));
    }

    #[test]
    fn is_combust_rahu_always_false() {
        assert!(!is_combust(Graha::Rahu, 100.0, 100.0, false));
    }

    #[test]
    fn is_combust_ketu_always_false() {
        assert!(!is_combust(Graha::Ketu, 100.0, 100.0, false));
    }

    #[test]
    fn is_combust_moon_within_threshold() {
        // Moon at 5 deg from Sun: 5 < 12 → combust
        assert!(is_combust(Graha::Chandra, 105.0, 100.0, false));
    }

    #[test]
    fn is_combust_moon_outside_threshold() {
        // Moon at 15 deg from Sun: 15 >= 12 → not combust
        assert!(!is_combust(Graha::Chandra, 115.0, 100.0, false));
    }

    #[test]
    fn boundary_exactly_at_threshold_not_combust() {
        // Mars at exactly 17 deg from Sun: 17 is NOT < 17 → not combust
        assert!(!is_combust(Graha::Mangal, 117.0, 100.0, false));
    }

    #[test]
    fn boundary_just_inside_combust() {
        // Mars at 16.999 deg from Sun: < 17 → combust
        assert!(is_combust(Graha::Mangal, 116.999, 100.0, false));
    }

    #[test]
    fn is_combust_wraparound() {
        // Sun at 355, Mars at 5 → distance = 10 < 17 → combust
        assert!(is_combust(Graha::Mangal, 5.0, 355.0, false));
    }

    #[test]
    fn mercury_retrograde_tighter_threshold() {
        // Mercury at 13 deg from Sun, direct: 13 < 14 → combust
        assert!(is_combust(Graha::Buddh, 113.0, 100.0, false));
        // Mercury at 13 deg from Sun, retrograde: 13 >= 12 → NOT combust
        assert!(!is_combust(Graha::Buddh, 113.0, 100.0, true));
    }

    #[test]
    fn venus_retrograde_tighter_threshold() {
        // Venus at 9 deg from Sun, direct: 9 < 10 → combust
        assert!(is_combust(Graha::Shukra, 109.0, 100.0, false));
        // Venus at 9 deg from Sun, retrograde: 9 >= 8 → NOT combust
        assert!(!is_combust(Graha::Shukra, 109.0, 100.0, true));
    }

    #[test]
    fn all_combustion_status_basic() {
        // Sun at 100, Moon at 105 (dist=5<12→combust), Mars at 200 (dist=100→not)
        let mut lons = [0.0; 9];
        lons[0] = 100.0; // Sun
        lons[1] = 105.0; // Moon
        lons[2] = 200.0; // Mars
        lons[3] = 100.0; // Mercury = same as Sun, dist=0<14→combust
        lons[4] = 300.0; // Jupiter, dist=200→min(200,160)=160→not
        lons[5] = 350.0; // Venus, dist=110→not
        lons[6] = 50.0; // Saturn, dist=50→not
        lons[7] = 100.0; // Rahu, always false
        lons[8] = 100.0; // Ketu, always false

        let retro = [false; 9];
        let result = all_combustion_status(&lons, &retro);

        assert!(!result[0]); // Sun
        assert!(result[1]); // Moon
        assert!(!result[2]); // Mars
        assert!(result[3]); // Mercury
        assert!(!result[4]); // Jupiter
        assert!(!result[5]); // Venus
        assert!(!result[6]); // Saturn
        assert!(!result[7]); // Rahu
        assert!(!result[8]); // Ketu
    }
}
