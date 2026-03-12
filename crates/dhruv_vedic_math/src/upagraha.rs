//! Upagraha (shadow planet) calculations.
//!
//! 11 Upagrahas:
//! - Sun-based (5): Dhooma, Vyatipata, Parivesha, Indra Chapa, Upaketu
//!   Chain calculation from Sun's longitude.
//! - Time-based (6): Gulika, Maandi, Kaala, Mrityu, Artha Prahara, Yama Ghantaka
//!   Lagna at the start/end of a planet's portion of the day or night.
//!
//! Day/night is divided into 8 equal portions, each ruled by a planet in
//! weekday-specific order. The upagraha longitude is the ascendant (lagna)
//! at the start (or end for Maandi) of the relevant planet's portion.
//!
//! Clean-room implementation from BPHS (Brihat Parashara Hora Shastra).
//! See `docs/clean_room_upagraha.md`.

use crate::util::normalize_360;

/// The 11 Upagrahas.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Upagraha {
    // Time-based
    Gulika,
    Maandi,
    Kaala,
    Mrityu,
    ArthaPrahara,
    YamaGhantaka,
    // Sun-based
    Dhooma,
    Vyatipata,
    Parivesha,
    IndraChapa,
    Upaketu,
}

/// All 11 upagrahas in canonical order.
pub const ALL_UPAGRAHAS: [Upagraha; 11] = [
    Upagraha::Gulika,
    Upagraha::Maandi,
    Upagraha::Kaala,
    Upagraha::Mrityu,
    Upagraha::ArthaPrahara,
    Upagraha::YamaGhantaka,
    Upagraha::Dhooma,
    Upagraha::Vyatipata,
    Upagraha::Parivesha,
    Upagraha::IndraChapa,
    Upagraha::Upaketu,
];

impl Upagraha {
    /// Name of the upagraha.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Gulika => "Gulika",
            Self::Maandi => "Maandi",
            Self::Kaala => "Kaala",
            Self::Mrityu => "Mrityu",
            Self::ArthaPrahara => "Artha Prahara",
            Self::YamaGhantaka => "Yama Ghantaka",
            Self::Dhooma => "Dhooma",
            Self::Vyatipata => "Vyatipata",
            Self::Parivesha => "Parivesha",
            Self::IndraChapa => "Indra Chapa",
            Self::Upaketu => "Upaketu",
        }
    }

    /// 0-based index (matches position in ALL_UPAGRAHAS).
    pub const fn index(self) -> u8 {
        match self {
            Self::Gulika => 0,
            Self::Maandi => 1,
            Self::Kaala => 2,
            Self::Mrityu => 3,
            Self::ArthaPrahara => 4,
            Self::YamaGhantaka => 5,
            Self::Dhooma => 6,
            Self::Vyatipata => 7,
            Self::Parivesha => 8,
            Self::IndraChapa => 9,
            Self::Upaketu => 10,
        }
    }

    /// Whether this is a time-based upagraha (requires sunrise/sunset/lagna).
    pub const fn is_time_based(self) -> bool {
        matches!(
            self,
            Self::Gulika
                | Self::Maandi
                | Self::Kaala
                | Self::Mrityu
                | Self::ArthaPrahara
                | Self::YamaGhantaka
        )
    }
}

// ---------------------------------------------------------------------------
// Sun-based upagrahas (pure math on sidereal Sun longitude)
// ---------------------------------------------------------------------------

/// The 5 sun-based upagraha longitudes.
#[derive(Debug, Clone, Copy)]
pub struct SunBasedUpagrahas {
    pub dhooma: f64,
    pub vyatipata: f64,
    pub parivesha: f64,
    pub indra_chapa: f64,
    pub upaketu: f64,
}

/// Compute the 5 sun-based upagrahas from the Sun's sidereal longitude.
///
/// Chain calculation (BPHS):
/// 1. Dhooma = Sun + 133°20'
/// 2. Vyatipata = 360° - Dhooma
/// 3. Parivesha = Vyatipata + 180°
/// 4. Indra Chapa = 360° - Parivesha
/// 5. Upaketu = Indra Chapa + 16°40'
pub fn sun_based_upagrahas(sun_sid_lon: f64) -> SunBasedUpagrahas {
    let dhooma = normalize_360(sun_sid_lon + 133.0 + 20.0 / 60.0); // 133°20'
    let vyatipata = normalize_360(360.0 - dhooma);
    let parivesha = normalize_360(vyatipata + 180.0);
    let indra_chapa = normalize_360(360.0 - parivesha);
    let upaketu = normalize_360(indra_chapa + 16.0 + 40.0 / 60.0); // 16°40'

    SunBasedUpagrahas {
        dhooma,
        vyatipata,
        parivesha,
        indra_chapa,
        upaketu,
    }
}

// ---------------------------------------------------------------------------
// Time-based upagrahas: portion index computation (pure math)
// ---------------------------------------------------------------------------

// Planet indices for the 8-fold portion sequence.
// 0=Sun, 1=Moon, 2=Mars, 3=Mercury, 4=Jupiter, 5=Venus, 6=Saturn, 7=Rahu
const PLANET_SUN: u8 = 0;
const PLANET_MARS: u8 = 2;
const PLANET_MERCURY: u8 = 3;
const PLANET_JUPITER: u8 = 4;
const PLANET_RAHU: u8 = 7;

/// Night-start planet index for each weekday (0=Sunday through 6=Saturday).
///
/// The night sequence starts from a different planet than the day ruler.
/// The pattern: night_start(weekday) = (weekday + 4) % 7
const NIGHT_START: [u8; 7] = [4, 5, 6, 0, 1, 2, 3];

/// Compute the 0-based portion index (0-7) for a planet during daytime.
///
/// Day portions start with the weekday's ruling planet:
/// - Sunday: Sun, Moon, Mars, Mercury, Jupiter, Venus, Saturn, Rahu
/// - Monday: Moon, Mars, Mercury, Jupiter, Venus, Saturn, Rahu, Sun
/// - etc.
///
/// Arguments:
/// - `weekday`: 0=Sunday (Ravivaar) through 6=Saturday (Shanivaar)
/// - `planet`: 0=Sun, 1=Moon, 2=Mars, 3=Mercury, 4=Jupiter, 5=Venus, 6=Saturn, 7=Rahu
pub fn day_portion_index(weekday: u8, planet: u8) -> u8 {
    ((planet as i8 - weekday as i8 + 8) % 8) as u8
}

/// Compute the 0-based portion index (0-7) for a planet during nighttime.
///
/// Night portions start from a shifted planet (offset by 4 from the day ruler):
/// - Sunday night: Jupiter, Venus, Saturn, Rahu, Sun, Moon, Mars, Mercury
/// - Monday night: Venus, Saturn, Rahu, Sun, Moon, Mars, Mercury, Jupiter
/// - etc.
pub fn night_portion_index(weekday: u8, planet: u8) -> u8 {
    let start = NIGHT_START[weekday as usize];
    ((planet as i8 - start as i8 + 8) % 8) as u8
}

/// Compute the JD range for a specific portion of day or night.
///
/// Divides the period [base_jd, end_jd] into 8 equal portions and returns
/// the start and end JD of the given portion index (0-7).
pub fn portion_jd_range(portion_index: u8, base_jd: f64, end_jd: f64) -> (f64, f64) {
    let total = end_jd - base_jd;
    let portion = total / 8.0;
    let start = base_jd + portion_index as f64 * portion;
    (start, start + portion)
}

/// Upagraha-to-planet mapping for time-based upagrahas.
///
/// Returns (planet_index, use_end) where:
/// - planet_index: planet whose portion to use (0-7)
/// - use_end: true = use end of portion (Maandi), false = use start
pub const fn time_upagraha_planet(upagraha: Upagraha) -> (u8, bool) {
    match upagraha {
        Upagraha::Gulika => (PLANET_RAHU, false), // start of Rahu's portion
        Upagraha::Maandi => (PLANET_RAHU, true),  // end of Rahu's portion
        Upagraha::Kaala => (PLANET_SUN, false),   // start of Sun's portion
        Upagraha::Mrityu => (PLANET_MARS, false), // start of Mars's portion
        Upagraha::ArthaPrahara => (PLANET_MERCURY, false), // start of Mercury's portion
        Upagraha::YamaGhantaka => (PLANET_JUPITER, false), // start of Jupiter's portion
        // Sun-based upagrahas don't use portions
        _ => (0, false),
    }
}

// ---------------------------------------------------------------------------
// Combined result
// ---------------------------------------------------------------------------

/// All 11 upagraha longitudes (sidereal degrees).
#[derive(Debug, Clone, Copy)]
pub struct AllUpagrahas {
    pub gulika: f64,
    pub maandi: f64,
    pub kaala: f64,
    pub mrityu: f64,
    pub artha_prahara: f64,
    pub yama_ghantaka: f64,
    pub dhooma: f64,
    pub vyatipata: f64,
    pub parivesha: f64,
    pub indra_chapa: f64,
    pub upaketu: f64,
}

impl AllUpagrahas {
    /// Get longitude by upagraha.
    pub fn longitude(&self, u: Upagraha) -> f64 {
        match u {
            Upagraha::Gulika => self.gulika,
            Upagraha::Maandi => self.maandi,
            Upagraha::Kaala => self.kaala,
            Upagraha::Mrityu => self.mrityu,
            Upagraha::ArthaPrahara => self.artha_prahara,
            Upagraha::YamaGhantaka => self.yama_ghantaka,
            Upagraha::Dhooma => self.dhooma,
            Upagraha::Vyatipata => self.vyatipata,
            Upagraha::Parivesha => self.parivesha,
            Upagraha::IndraChapa => self.indra_chapa,
            Upagraha::Upaketu => self.upaketu,
        }
    }
}

// ---------------------------------------------------------------------------
// Time-based upagraha computation helper
// ---------------------------------------------------------------------------

/// The 6 time-based upagrahas in order.
pub const TIME_BASED_UPAGRAHAS: [Upagraha; 6] = [
    Upagraha::Gulika,
    Upagraha::Maandi,
    Upagraha::Kaala,
    Upagraha::Mrityu,
    Upagraha::ArthaPrahara,
    Upagraha::YamaGhantaka,
];

/// Compute the JD at which to evaluate the lagna for a time-based upagraha.
///
/// Arguments:
/// - `upagraha`: which time-based upagraha
/// - `weekday`: 0=Sunday through 6=Saturday
/// - `is_day`: true if birth is during daytime
/// - `day_start_jd`: sunrise JD
/// - `day_end_jd`: sunset JD
/// - `night_end_jd`: next sunrise JD
///
/// Returns the JD at which the lagna should be computed.
pub fn time_upagraha_jd(
    upagraha: Upagraha,
    weekday: u8,
    is_day: bool,
    day_start_jd: f64,
    day_end_jd: f64,
    night_end_jd: f64,
) -> f64 {
    let (planet, use_end) = time_upagraha_planet(upagraha);

    let index = if is_day {
        day_portion_index(weekday, planet)
    } else {
        night_portion_index(weekday, planet)
    };

    let (base_jd, end_jd) = if is_day {
        (day_start_jd, day_end_jd)
    } else {
        (day_end_jd, night_end_jd)
    };

    let (start, end) = portion_jd_range(index, base_jd, end_jd);
    if use_end { end } else { start }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_upagrahas_count() {
        assert_eq!(ALL_UPAGRAHAS.len(), 11);
    }

    #[test]
    fn upagraha_indices_sequential() {
        for (i, u) in ALL_UPAGRAHAS.iter().enumerate() {
            assert_eq!(u.index() as usize, i);
        }
    }

    #[test]
    fn upagraha_names_nonempty() {
        for u in ALL_UPAGRAHAS {
            assert!(!u.name().is_empty());
        }
    }

    #[test]
    fn time_based_classification() {
        assert!(Upagraha::Gulika.is_time_based());
        assert!(Upagraha::Maandi.is_time_based());
        assert!(Upagraha::Kaala.is_time_based());
        assert!(Upagraha::Mrityu.is_time_based());
        assert!(Upagraha::ArthaPrahara.is_time_based());
        assert!(Upagraha::YamaGhantaka.is_time_based());
        assert!(!Upagraha::Dhooma.is_time_based());
        assert!(!Upagraha::Upaketu.is_time_based());
    }

    // --- Sun-based upagrahas ---

    #[test]
    fn sun_based_chain() {
        let sun = 100.0;
        let result = sun_based_upagrahas(sun);

        // Dhooma = 100 + 133.333... = 233.333...
        let expected_dhooma = 100.0 + 133.0 + 20.0 / 60.0;
        assert!((result.dhooma - expected_dhooma).abs() < 1e-8);

        // Vyatipata = 360 - Dhooma
        let expected_vyati = normalize_360(360.0 - expected_dhooma);
        assert!((result.vyatipata - expected_vyati).abs() < 1e-8);

        // Parivesha = Vyatipata + 180
        let expected_parivesha = normalize_360(expected_vyati + 180.0);
        assert!((result.parivesha - expected_parivesha).abs() < 1e-8);

        // Indra Chapa = 360 - Parivesha
        let expected_indra = normalize_360(360.0 - expected_parivesha);
        assert!((result.indra_chapa - expected_indra).abs() < 1e-8);

        // Upaketu = Indra Chapa + 16.666...
        let expected_upaketu = normalize_360(expected_indra + 16.0 + 40.0 / 60.0);
        assert!((result.upaketu - expected_upaketu).abs() < 1e-8);
    }

    #[test]
    fn sun_based_at_zero() {
        let r = sun_based_upagrahas(0.0);
        // Dhooma = 133.333...
        assert!((r.dhooma - (133.0 + 20.0 / 60.0)).abs() < 1e-8);
        // Verify all in [0, 360)
        assert!(r.dhooma >= 0.0 && r.dhooma < 360.0);
        assert!(r.vyatipata >= 0.0 && r.vyatipata < 360.0);
        assert!(r.parivesha >= 0.0 && r.parivesha < 360.0);
        assert!(r.indra_chapa >= 0.0 && r.indra_chapa < 360.0);
        assert!(r.upaketu >= 0.0 && r.upaketu < 360.0);
    }

    // --- Portion index computation ---

    #[test]
    fn day_portion_sunday_sun_first() {
        // Sunday (0): Sun is first (index 0)
        assert_eq!(day_portion_index(0, 0), 0);
    }

    #[test]
    fn day_portion_sunday_rahu_last() {
        // Sunday (0): Rahu is last (index 7)
        assert_eq!(day_portion_index(0, 7), 7);
    }

    #[test]
    fn day_portion_monday_moon_first() {
        // Monday (1): Moon(1) is first
        assert_eq!(day_portion_index(1, 1), 0);
    }

    #[test]
    fn day_portion_monday_sun_last() {
        // Monday (1): Sun(0) is last (index 7)
        assert_eq!(day_portion_index(1, 0), 7);
    }

    #[test]
    fn day_portion_tuesday_mars_first() {
        // Tuesday (2): Mars(2) is first
        assert_eq!(day_portion_index(2, 2), 0);
    }

    #[test]
    fn day_portion_matches_python_rahu_day() {
        // Python WEEKDAY_RAHU_DAY_INDEX (Mon=6,Tue=5,Wed=4,Thu=3,Fri=2,Sat=1,Sun=7)
        // Our convention: Sun=0, Mon=1, ..., Sat=6
        let expected = [7u8, 6, 5, 4, 3, 2, 1];
        for w in 0..7u8 {
            assert_eq!(
                day_portion_index(w, PLANET_RAHU),
                expected[w as usize],
                "day rahu weekday={w}"
            );
        }
    }

    #[test]
    fn day_portion_matches_python_sun_day() {
        // Python WEEKDAY_SUN_DAY_INDEX (Mon=7,Tue=6,Wed=5,Thu=4,Fri=3,Sat=2,Sun=0)
        let expected = [0u8, 7, 6, 5, 4, 3, 2];
        for w in 0..7u8 {
            assert_eq!(
                day_portion_index(w, PLANET_SUN),
                expected[w as usize],
                "day sun weekday={w}"
            );
        }
    }

    #[test]
    fn night_portion_sunday_jupiter_first() {
        // Sunday night (0): start=Jupiter(4), so Jupiter is first (index 0)
        assert_eq!(night_portion_index(0, 4), 0);
    }

    #[test]
    fn night_portion_matches_python_rahu_night() {
        // Python WEEKDAY_RAHU_NIGHT_INDEX (Mon=2,Tue=1,Wed=7,Thu=6,Fri=5,Sat=4,Sun=3)
        let expected = [3u8, 2, 1, 7, 6, 5, 4];
        for w in 0..7u8 {
            assert_eq!(
                night_portion_index(w, PLANET_RAHU),
                expected[w as usize],
                "night rahu weekday={w}"
            );
        }
    }

    #[test]
    fn night_portion_matches_python_sun_night() {
        // Python WEEKDAY_SUN_NIGHT_INDEX (Mon=3,Tue=2,Wed=0,Thu=7,Fri=6,Sat=5,Sun=4)
        let expected = [4u8, 3, 2, 0, 7, 6, 5];
        for w in 0..7u8 {
            assert_eq!(
                night_portion_index(w, PLANET_SUN),
                expected[w as usize],
                "night sun weekday={w}"
            );
        }
    }

    #[test]
    fn night_portion_matches_python_mars_night() {
        // Python WEEKDAY_MARS_NIGHT_INDEX (Mon=5,Tue=4,Wed=2,Thu=1,Fri=0,Sat=7,Sun=6)
        let expected = [6u8, 5, 4, 2, 1, 0, 7];
        for w in 0..7u8 {
            assert_eq!(
                night_portion_index(w, PLANET_MARS),
                expected[w as usize],
                "night mars weekday={w}"
            );
        }
    }

    // --- Portion JD range ---

    #[test]
    fn portion_jd_range_basic() {
        let (start, end) = portion_jd_range(0, 100.0, 100.5);
        assert!((start - 100.0).abs() < 1e-12);
        assert!((end - 100.0625).abs() < 1e-12); // 0.5/8 = 0.0625
    }

    #[test]
    fn portion_jd_range_last() {
        let (start, end) = portion_jd_range(7, 100.0, 100.5);
        assert!((end - 100.5).abs() < 1e-12);
        assert!((start - (100.0 + 7.0 * 0.0625)).abs() < 1e-12);
    }

    // --- AllUpagrahas accessor ---

    #[test]
    fn all_upagrahas_longitude_accessor() {
        let all = AllUpagrahas {
            gulika: 10.0,
            maandi: 20.0,
            kaala: 30.0,
            mrityu: 40.0,
            artha_prahara: 50.0,
            yama_ghantaka: 60.0,
            dhooma: 70.0,
            vyatipata: 80.0,
            parivesha: 90.0,
            indra_chapa: 100.0,
            upaketu: 110.0,
        };
        assert!((all.longitude(Upagraha::Gulika) - 10.0).abs() < 1e-12);
        assert!((all.longitude(Upagraha::Upaketu) - 110.0).abs() < 1e-12);
    }
}
