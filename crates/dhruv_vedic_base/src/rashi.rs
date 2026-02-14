//! Rashi (zodiac sign) and DMS (degrees-minutes-seconds) computation.
//!
//! The ecliptic circle is divided into 12 equal signs of 30 degrees each.
//! Given a sidereal longitude, we identify which rashi the point falls in
//! and express the position as degrees-minutes-seconds within that sign.
//!
//! Clean-room implementation from universal Vedic convention:
//! 12 rashis of 30 deg each, starting from Mesha (Aries) at 0 deg.
//! See `docs/clean_room_rashi_nakshatra.md`.

use crate::ayanamsha::{AyanamshaSystem, ayanamsha_deg, jd_tdb_to_centuries};

/// The 12 rashis (zodiac signs) starting from Mesha (Aries).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Rashi {
    Mesha,
    Vrishabha,
    Mithuna,
    Karka,
    Simha,
    Kanya,
    Tula,
    Vrischika,
    Dhanu,
    Makara,
    Kumbha,
    Meena,
}

/// All 12 rashis in order, for FFI indexing (0 = Mesha, 11 = Meena).
pub const ALL_RASHIS: [Rashi; 12] = [
    Rashi::Mesha,
    Rashi::Vrishabha,
    Rashi::Mithuna,
    Rashi::Karka,
    Rashi::Simha,
    Rashi::Kanya,
    Rashi::Tula,
    Rashi::Vrischika,
    Rashi::Dhanu,
    Rashi::Makara,
    Rashi::Kumbha,
    Rashi::Meena,
];

impl Rashi {
    /// Sanskrit name of the rashi.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Mesha => "Mesha",
            Self::Vrishabha => "Vrishabha",
            Self::Mithuna => "Mithuna",
            Self::Karka => "Karka",
            Self::Simha => "Simha",
            Self::Kanya => "Kanya",
            Self::Tula => "Tula",
            Self::Vrischika => "Vrischika",
            Self::Dhanu => "Dhanu",
            Self::Makara => "Makara",
            Self::Kumbha => "Kumbha",
            Self::Meena => "Meena",
        }
    }

    /// Western (English) name of the rashi.
    pub const fn western_name(self) -> &'static str {
        match self {
            Self::Mesha => "Aries",
            Self::Vrishabha => "Taurus",
            Self::Mithuna => "Gemini",
            Self::Karka => "Cancer",
            Self::Simha => "Leo",
            Self::Kanya => "Virgo",
            Self::Tula => "Libra",
            Self::Vrischika => "Scorpio",
            Self::Dhanu => "Sagittarius",
            Self::Makara => "Capricorn",
            Self::Kumbha => "Aquarius",
            Self::Meena => "Pisces",
        }
    }

    /// 0-based index (Mesha=0 .. Meena=11).
    pub const fn index(self) -> u8 {
        match self {
            Self::Mesha => 0,
            Self::Vrishabha => 1,
            Self::Mithuna => 2,
            Self::Karka => 3,
            Self::Simha => 4,
            Self::Kanya => 5,
            Self::Tula => 6,
            Self::Vrischika => 7,
            Self::Dhanu => 8,
            Self::Makara => 9,
            Self::Kumbha => 10,
            Self::Meena => 11,
        }
    }

    /// All 12 rashis in order.
    pub const fn all() -> &'static [Rashi; 12] {
        &ALL_RASHIS
    }
}

/// Degrees-minutes-seconds representation of an angle.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Dms {
    /// Whole degrees (0..29 within a rashi, or 0..359 standalone).
    pub degrees: u16,
    /// Arc-minutes (0..59).
    pub minutes: u8,
    /// Arc-seconds (0.0..60.0), may include fractional part.
    pub seconds: f64,
}

/// Full rashi position result.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RashiInfo {
    /// The rashi (zodiac sign).
    pub rashi: Rashi,
    /// 0-based rashi index (0 = Mesha).
    pub rashi_index: u8,
    /// Position within the rashi as DMS.
    pub dms: Dms,
    /// Decimal degrees within the rashi [0.0, 30.0).
    pub degrees_in_rashi: f64,
}

/// Convert DMS back to decimal degrees.
pub fn dms_to_deg(dms: &Dms) -> f64 {
    dms.degrees as f64 + dms.minutes as f64 / 60.0 + dms.seconds / 3600.0
}

/// Convert decimal degrees to degrees-minutes-seconds.
///
/// Handles negative input by taking absolute value.
pub fn deg_to_dms(deg: f64) -> Dms {
    let d = deg.abs();
    let total_degrees = d.floor() as u16;
    let remainder = (d - total_degrees as f64) * 60.0;
    let minutes = remainder.floor() as u8;
    let seconds = (remainder - minutes as f64) * 60.0;
    Dms {
        degrees: total_degrees,
        minutes,
        seconds,
    }
}

/// Normalize longitude to [0, 360).
fn normalize_360(deg: f64) -> f64 {
    let r = deg % 360.0;
    if r < 0.0 { r + 360.0 } else { r }
}

/// Determine rashi from sidereal ecliptic longitude.
///
/// The input is a sidereal longitude in degrees (tropical minus ayanamsha).
/// Each rashi spans exactly 30 degrees: Mesha = [0, 30), Vrishabha = [30, 60), etc.
pub fn rashi_from_longitude(sidereal_lon_deg: f64) -> RashiInfo {
    let lon = normalize_360(sidereal_lon_deg);
    let rashi_idx = (lon / 30.0).floor() as u8;
    // Clamp to 11 in case of floating point edge (exactly 360.0)
    let rashi_idx = rashi_idx.min(11);
    let degrees_in_rashi = lon - (rashi_idx as f64) * 30.0;
    let rashi = ALL_RASHIS[rashi_idx as usize];
    let dms = deg_to_dms(degrees_in_rashi);

    RashiInfo {
        rashi,
        rashi_index: rashi_idx,
        dms,
        degrees_in_rashi,
    }
}

/// Convenience: determine rashi from tropical longitude + ayanamsha.
///
/// Computes `sidereal = tropical - ayanamsha(system, jd_tdb, use_nutation)`,
/// then calls [`rashi_from_longitude`].
pub fn rashi_from_tropical(
    tropical_lon_deg: f64,
    system: AyanamshaSystem,
    jd_tdb: f64,
    use_nutation: bool,
) -> RashiInfo {
    let t = jd_tdb_to_centuries(jd_tdb);
    let aya = ayanamsha_deg(system, t, use_nutation);
    rashi_from_longitude(tropical_lon_deg - aya)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_rashis_count() {
        assert_eq!(ALL_RASHIS.len(), 12);
    }

    #[test]
    fn rashi_indices_sequential() {
        for (i, r) in ALL_RASHIS.iter().enumerate() {
            assert_eq!(r.index() as usize, i);
        }
    }

    #[test]
    fn rashi_names_nonempty() {
        for r in ALL_RASHIS {
            assert!(!r.name().is_empty());
            assert!(!r.western_name().is_empty());
        }
    }

    #[test]
    fn deg_to_dms_zero() {
        let d = deg_to_dms(0.0);
        assert_eq!(d.degrees, 0);
        assert_eq!(d.minutes, 0);
        assert!(d.seconds.abs() < 1e-10);
    }

    #[test]
    fn deg_to_dms_known() {
        // 23.853 deg = 23 deg 51' 10.8"
        let d = deg_to_dms(23.853);
        assert_eq!(d.degrees, 23);
        assert_eq!(d.minutes, 51);
        assert!((d.seconds - 10.8).abs() < 0.01);
    }

    #[test]
    fn deg_to_dms_exact_minutes() {
        // 10.5 deg = 10 deg 30' 0"
        let d = deg_to_dms(10.5);
        assert_eq!(d.degrees, 10);
        assert_eq!(d.minutes, 30);
        assert!(d.seconds.abs() < 0.01);
    }

    #[test]
    fn rashi_boundary_0() {
        let info = rashi_from_longitude(0.0);
        assert_eq!(info.rashi, Rashi::Mesha);
        assert_eq!(info.rashi_index, 0);
        assert!(info.degrees_in_rashi.abs() < 1e-10);
    }

    #[test]
    fn rashi_boundary_30() {
        let info = rashi_from_longitude(30.0);
        assert_eq!(info.rashi, Rashi::Vrishabha);
        assert_eq!(info.rashi_index, 1);
        assert!(info.degrees_in_rashi.abs() < 1e-10);
    }

    #[test]
    fn rashi_all_boundaries() {
        for i in 0..12u8 {
            let lon = i as f64 * 30.0;
            let info = rashi_from_longitude(lon);
            assert_eq!(info.rashi_index, i, "boundary at {lon} deg");
        }
    }

    #[test]
    fn rashi_mid_sign() {
        let info = rashi_from_longitude(45.5);
        assert_eq!(info.rashi, Rashi::Vrishabha);
        assert!((info.degrees_in_rashi - 15.5).abs() < 1e-10);
    }

    #[test]
    fn rashi_wrap_around() {
        let info = rashi_from_longitude(365.0);
        assert_eq!(info.rashi, Rashi::Mesha);
        assert!((info.degrees_in_rashi - 5.0).abs() < 1e-10);
    }

    #[test]
    fn rashi_negative() {
        let info = rashi_from_longitude(-10.0);
        assert_eq!(info.rashi, Rashi::Meena); // 350 deg
        assert!((info.degrees_in_rashi - 20.0).abs() < 1e-10);
    }

    #[test]
    fn rashi_last_sign() {
        let info = rashi_from_longitude(350.0);
        assert_eq!(info.rashi, Rashi::Meena);
        assert_eq!(info.rashi_index, 11);
    }

    #[test]
    fn rashi_dms_within_sign() {
        // 45.5 deg → Vrishabha, 15 deg 30' 0"
        let info = rashi_from_longitude(45.5);
        assert_eq!(info.dms.degrees, 15);
        assert_eq!(info.dms.minutes, 30);
        assert!(info.dms.seconds.abs() < 0.01);
    }

    #[test]
    fn rashi_from_tropical_lahiri_j2000() {
        // Tropical ~280.5 deg, Lahiri aya ~23.853 → sidereal ~256.65 → Dhanu (8)
        let info = rashi_from_tropical(280.5, AyanamshaSystem::Lahiri, 2_451_545.0, false);
        assert_eq!(info.rashi, Rashi::Dhanu);
    }
}
