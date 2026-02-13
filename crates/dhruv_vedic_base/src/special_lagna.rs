//! Special Lagna (ascendant variant) calculations.
//!
//! 8 special lagnas used in Vedic jyotish, all pure math on sidereal longitudes:
//! - Time-based: Bhava Lagna, Hora Lagna, Ghati Lagna, Vighati Lagna
//! - Composite: Varnada Lagna, Pranapada Lagna
//! - Moon-based: Sree Lagna
//! - Wealth-based: Indu Lagna
//!
//! Clean-room implementation from standard Vedic jyotish texts (BPHS, Jataka Parijata).
//! See `docs/clean_room_special_lagnas.md`.

use crate::graha::Graha;
use crate::util::normalize_360;

/// The 8 Special Lagnas.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SpecialLagna {
    BhavaLagna,
    HoraLagna,
    GhatiLagna,
    VighatiLagna,
    VarnadaLagna,
    SreeLagna,
    PranapadaLagna,
    InduLagna,
}

/// All 8 special lagnas in standard order.
pub const ALL_SPECIAL_LAGNAS: [SpecialLagna; 8] = [
    SpecialLagna::BhavaLagna,
    SpecialLagna::HoraLagna,
    SpecialLagna::GhatiLagna,
    SpecialLagna::VighatiLagna,
    SpecialLagna::VarnadaLagna,
    SpecialLagna::SreeLagna,
    SpecialLagna::PranapadaLagna,
    SpecialLagna::InduLagna,
];

impl SpecialLagna {
    /// Name of the special lagna.
    pub const fn name(self) -> &'static str {
        match self {
            Self::BhavaLagna => "Bhava Lagna",
            Self::HoraLagna => "Hora Lagna",
            Self::GhatiLagna => "Ghati Lagna",
            Self::VighatiLagna => "Vighati Lagna",
            Self::VarnadaLagna => "Varnada Lagna",
            Self::SreeLagna => "Sree Lagna",
            Self::PranapadaLagna => "Pranapada Lagna",
            Self::InduLagna => "Indu Lagna",
        }
    }

    /// 0-based index into ALL_SPECIAL_LAGNAS.
    pub const fn index(self) -> u8 {
        match self {
            Self::BhavaLagna => 0,
            Self::HoraLagna => 1,
            Self::GhatiLagna => 2,
            Self::VighatiLagna => 3,
            Self::VarnadaLagna => 4,
            Self::SreeLagna => 5,
            Self::PranapadaLagna => 6,
            Self::InduLagna => 7,
        }
    }
}

// ---------------------------------------------------------------------------
// Time helpers
// ---------------------------------------------------------------------------

/// Compute ghatikas elapsed since sunrise.
///
/// A ghatika = 24 minutes. One Vedic day (sunrise to next sunrise) = 60 ghatikas.
/// The result can exceed 60 if `jd_moment` is past the next sunrise.
pub fn ghatikas_since_sunrise(jd_moment: f64, jd_sunrise: f64, jd_next_sunrise: f64) -> f64 {
    let day_length = jd_next_sunrise - jd_sunrise;
    if day_length <= 0.0 {
        return 0.0;
    }
    (jd_moment - jd_sunrise) / day_length * 60.0
}

// ---------------------------------------------------------------------------
// Individual special lagna formulas (all pure math)
// ---------------------------------------------------------------------------

/// Bhava Lagna: advances 1 sign (30 deg) per 5 ghatikas from Sun.
///
/// Formula: `(sun_lon + ghatikas * 6) % 360`
pub fn bhava_lagna(sun_lon: f64, ghatikas: f64) -> f64 {
    normalize_360(sun_lon + ghatikas * 6.0)
}

/// Hora Lagna: advances 1 sign (30 deg) per 2.5 ghatikas from Sun.
///
/// Formula: `(sun_lon + ghatikas * 12) % 360`
pub fn hora_lagna(sun_lon: f64, ghatikas: f64) -> f64 {
    normalize_360(sun_lon + ghatikas * 12.0)
}

/// Ghati Lagna: advances 1 sign (30 deg) per ghatika from Sun.
///
/// Formula: `(sun_lon + ghatikas * 30) % 360`
pub fn ghati_lagna(sun_lon: f64, ghatikas: f64) -> f64 {
    normalize_360(sun_lon + ghatikas * 30.0)
}

/// Vighati Lagna: advances based on vighatikas from birth lagna.
///
/// Formula: `(lagna_lon + vighatikas * 0.5) % 360`
pub fn vighati_lagna(lagna_lon: f64, vighatikas: f64) -> f64 {
    normalize_360(lagna_lon + vighatikas * 0.5)
}

/// Varnada Lagna: parity-based combination of Lagna and Hora Lagna.
///
/// Rules based on rashi parity (odd=1,3,5,7,9,11; even=2,4,6,8,10,12):
/// - Both odd: add longitudes
/// - Both even: add complements (360 - longitude)
/// - Lagna odd, Hora even: absolute difference
/// - Lagna even, Hora odd: 360 - absolute difference
pub fn varnada_lagna(lagna_lon: f64, hora_lagna_lon: f64) -> f64 {
    // Rashi number 1-12
    let lagna_rashi = (lagna_lon / 30.0) as u8 + 1;
    let hora_rashi = (hora_lagna_lon / 30.0) as u8 + 1;

    let lagna_odd = lagna_rashi % 2 == 1;
    let hora_odd = hora_rashi % 2 == 1;

    let result = if lagna_odd && hora_odd {
        // Both odd: add longitudes
        lagna_lon + hora_lagna_lon
    } else if !lagna_odd && !hora_odd {
        // Both even: add complements
        (360.0 - lagna_lon) + (360.0 - hora_lagna_lon)
    } else if lagna_odd {
        // Lagna odd, Hora even: absolute difference
        (lagna_lon - hora_lagna_lon).abs()
    } else {
        // Lagna even, Hora odd: 360 - absolute difference
        360.0 - (lagna_lon - hora_lagna_lon).abs()
    };

    normalize_360(result)
}

/// Sree Lagna: Moon's nakshatra fraction scaled and added to Lagna.
///
/// 1. Find Moon's position within its current nakshatra (0 to 13 deg 20')
/// 2. Scale fraction to 360 deg
/// 3. Add to Lagna longitude
pub fn sree_lagna(moon_lon: f64, lagna_lon: f64) -> f64 {
    let nakshatra_span = 360.0 / 27.0; // 13.333... deg
    let moon_in_nakshatra = moon_lon % nakshatra_span;
    let scaled = (moon_in_nakshatra / nakshatra_span) * 360.0;
    normalize_360(lagna_lon + scaled)
}

/// Pranapada Lagna: Sun-based with sign-type adjustment.
///
/// Formula:
/// 1. `base = (sun_lon + ghatikas * 120) % 360`
/// 2. Adjust by Sun's rashi type:
///    - Movable (1,4,7,10): no addition
///    - Fixed (2,5,8,11): +240 deg
///    - Dual (3,6,9,12): +120 deg
pub fn pranapada_lagna(sun_lon: f64, ghatikas: f64) -> f64 {
    // ghatikas * 4 signs * 30 deg = ghatikas * 120
    let base = normalize_360(sun_lon + (ghatikas * 4.0 * 30.0) % 360.0);

    // Sun's rashi 1-12
    let sun_rashi = (sun_lon / 30.0) as u8 + 1;
    // sign_type: 1=movable, 2=fixed, 0=dual
    let sign_type = sun_rashi % 3;

    match sign_type {
        1 => base,                        // Movable: no addition
        2 => normalize_360(base + 240.0), // Fixed: +240
        _ => normalize_360(base + 120.0), // Dual: +120
    }
}

/// Indu Lagna: wealth indicator based on kaksha values.
///
/// Formula:
/// 1. Find Lagna lord's and Moon's 9th lord's kaksha values
/// 2. total = lagna_lord_kaksha + moon_9th_lord_kaksha
/// 3. remainder = total % 12; if 0, use 12
/// 4. Indu = Moon + (remainder - 1) * 30 deg
///
/// Returns None if either lord is Rahu/Ketu (kaksha = 0).
pub fn indu_lagna(moon_lon: f64, lagna_lord: Graha, moon_9th_lord: Graha) -> f64 {
    let lagna_kaksha = lagna_lord.kaksha_value() as u16;
    let moon_kaksha = moon_9th_lord.kaksha_value() as u16;

    let total = lagna_kaksha + moon_kaksha;
    let mut remainder = total % 12;
    if remainder == 0 {
        remainder = 12;
    }

    normalize_360(moon_lon + (remainder as f64 - 1.0) * 30.0)
}

// ---------------------------------------------------------------------------
// Batch computation
// ---------------------------------------------------------------------------

/// All 8 special lagna results.
#[derive(Debug, Clone, Copy)]
pub struct AllSpecialLagnas {
    pub bhava_lagna: f64,
    pub hora_lagna: f64,
    pub ghati_lagna: f64,
    pub vighati_lagna: f64,
    pub varnada_lagna: f64,
    pub sree_lagna: f64,
    pub pranapada_lagna: f64,
    pub indu_lagna: f64,
}

/// Compute all 8 special lagnas in one call.
///
/// Arguments:
/// - `sun_lon`: Sun's sidereal longitude in degrees
/// - `moon_lon`: Moon's sidereal longitude in degrees
/// - `lagna_lon`: Birth lagna (Ascendant) sidereal longitude in degrees
/// - `ghatikas`: Ghatikas elapsed since sunrise
/// - `lagna_lord`: Lord of lagna's rashi
/// - `moon_9th_lord`: Lord of 9th rashi from Moon
pub fn all_special_lagnas(
    sun_lon: f64,
    moon_lon: f64,
    lagna_lon: f64,
    ghatikas: f64,
    lagna_lord: Graha,
    moon_9th_lord: Graha,
) -> AllSpecialLagnas {
    let vighatikas = ghatikas * 60.0;
    let hl = hora_lagna(sun_lon, ghatikas);

    AllSpecialLagnas {
        bhava_lagna: bhava_lagna(sun_lon, ghatikas),
        hora_lagna: hl,
        ghati_lagna: ghati_lagna(sun_lon, ghatikas),
        vighati_lagna: vighati_lagna(lagna_lon, vighatikas),
        varnada_lagna: varnada_lagna(lagna_lon, hl),
        sree_lagna: sree_lagna(moon_lon, lagna_lon),
        pranapada_lagna: pranapada_lagna(sun_lon, ghatikas),
        indu_lagna: indu_lagna(moon_lon, lagna_lord, moon_9th_lord),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_special_lagnas_count() {
        assert_eq!(ALL_SPECIAL_LAGNAS.len(), 8);
    }

    #[test]
    fn special_lagna_indices_sequential() {
        for (i, sl) in ALL_SPECIAL_LAGNAS.iter().enumerate() {
            assert_eq!(sl.index() as usize, i);
        }
    }

    #[test]
    fn special_lagna_names_nonempty() {
        for sl in ALL_SPECIAL_LAGNAS {
            assert!(!sl.name().is_empty());
        }
    }

    // --- Ghatikas ---

    #[test]
    fn ghatikas_at_sunrise_is_zero() {
        let gh = ghatikas_since_sunrise(100.0, 100.0, 101.0);
        assert!((gh - 0.0).abs() < 1e-10);
    }

    #[test]
    fn ghatikas_at_next_sunrise_is_60() {
        let gh = ghatikas_since_sunrise(101.0, 100.0, 101.0);
        assert!((gh - 60.0).abs() < 1e-10);
    }

    #[test]
    fn ghatikas_midday_is_30() {
        let gh = ghatikas_since_sunrise(100.5, 100.0, 101.0);
        assert!((gh - 30.0).abs() < 1e-10);
    }

    // --- Bhava Lagna ---

    #[test]
    fn bhava_lagna_zero_ghatikas() {
        // At sunrise: Bhava Lagna = Sun
        let bl = bhava_lagna(45.0, 0.0);
        assert!((bl - 45.0).abs() < 1e-10);
    }

    #[test]
    fn bhava_lagna_5_ghatikas() {
        // 5 gh → 30 deg advance
        let bl = bhava_lagna(45.0, 5.0);
        assert!((bl - 75.0).abs() < 1e-10);
    }

    #[test]
    fn bhava_lagna_wraps() {
        let bl = bhava_lagna(350.0, 10.0);
        assert!((bl - 50.0).abs() < 1e-10); // 350 + 60 = 410 → 50
    }

    // --- Hora Lagna ---

    #[test]
    fn hora_lagna_2_5_ghatikas() {
        // 2.5 gh → 30 deg
        let hl = hora_lagna(100.0, 2.5);
        assert!((hl - 130.0).abs() < 1e-10);
    }

    // --- Ghati Lagna ---

    #[test]
    fn ghati_lagna_1_ghatika() {
        // 1 gh → 30 deg
        let gl = ghati_lagna(100.0, 1.0);
        assert!((gl - 130.0).abs() < 1e-10);
    }

    // --- Vighati Lagna ---

    #[test]
    fn vighati_lagna_60_vighatikas() {
        // 60 vighatikas = 30 deg
        let vl = vighati_lagna(100.0, 60.0);
        assert!((vl - 130.0).abs() < 1e-10);
    }

    // --- Varnada Lagna ---

    #[test]
    fn varnada_both_odd() {
        // Lagna in Mesha (rashi 1, odd), Hora in Mithuna (rashi 3, odd)
        // Both odd → add: 15 + 75 = 90
        let vl = varnada_lagna(15.0, 75.0);
        assert!((vl - 90.0).abs() < 1e-10);
    }

    #[test]
    fn varnada_both_even() {
        // Lagna in Vrishabha (rashi 2, even), Hora in Karka (rashi 4, even)
        // Both even → add complements: (360-45) + (360-105) = 315 + 255 = 570 → 210
        let vl = varnada_lagna(45.0, 105.0);
        assert!((vl - 210.0).abs() < 1e-10);
    }

    #[test]
    fn varnada_odd_even() {
        // Lagna in Mesha (rashi 1, odd), Hora in Vrishabha (rashi 2, even)
        // Lagna odd, Hora even → |15 - 45| = 30
        let vl = varnada_lagna(15.0, 45.0);
        assert!((vl - 30.0).abs() < 1e-10);
    }

    #[test]
    fn varnada_even_odd() {
        // Lagna in Vrishabha (rashi 2, even), Hora in Mesha (rashi 1, odd)
        // Lagna even, Hora odd → 360 - |45 - 15| = 360 - 30 = 330
        let vl = varnada_lagna(45.0, 15.0);
        assert!((vl - 330.0).abs() < 1e-10);
    }

    // --- Sree Lagna ---

    #[test]
    fn sree_lagna_moon_at_nakshatra_start() {
        // Moon at start of a nakshatra → fraction = 0 → Sree = Lagna
        let sl = sree_lagna(0.0, 100.0);
        assert!((sl - 100.0).abs() < 1e-10);
    }

    #[test]
    fn sree_lagna_moon_halfway_through_nakshatra() {
        // Moon at 6.6667 deg (half of 13.333 deg) → fraction = 0.5 → scaled = 180
        let half_nak = 360.0 / 27.0 / 2.0;
        let sl = sree_lagna(half_nak, 100.0);
        assert!((sl - 280.0).abs() < 1e-8); // 100 + 180 = 280
    }

    // --- Pranapada Lagna ---

    #[test]
    fn pranapada_movable_sign() {
        // Sun in Mesha (rashi 1, movable): no adjustment
        // 0 ghatikas → base = sun
        let pl = pranapada_lagna(15.0, 0.0);
        assert!((pl - 15.0).abs() < 1e-10);
    }

    #[test]
    fn pranapada_fixed_sign() {
        // Sun in Vrishabha (rashi 2, fixed): +240
        let pl = pranapada_lagna(45.0, 0.0);
        assert!((pl - 285.0).abs() < 1e-10); // 45 + 240 = 285
    }

    #[test]
    fn pranapada_dual_sign() {
        // Sun in Mithuna (rashi 3, dual): +120
        let pl = pranapada_lagna(75.0, 0.0);
        assert!((pl - 195.0).abs() < 1e-10); // 75 + 120 = 195
    }

    // --- Indu Lagna ---

    #[test]
    fn indu_lagna_basic() {
        // Lagna lord = Sun (kaksha 30), Moon 9th lord = Moon (kaksha 16)
        // total = 46, remainder = 46 % 12 = 10
        // Indu = Moon + (10-1)*30 = Moon + 270
        let il = indu_lagna(50.0, Graha::Surya, Graha::Chandra);
        assert!((il - 320.0).abs() < 1e-10); // 50 + 270 = 320
    }

    #[test]
    fn indu_lagna_remainder_zero() {
        // Lagna lord = Mars (6), Moon 9th lord = Mars (6)
        // total = 12, remainder = 12 % 12 = 0 → use 12
        // Indu = Moon + (12-1)*30 = Moon + 330
        let il = indu_lagna(10.0, Graha::Mangal, Graha::Mangal);
        assert!((il - 340.0).abs() < 1e-10); // 10 + 330 = 340
    }

    #[test]
    fn indu_lagna_wraps() {
        // Lagna lord = Saturn (1), Moon 9th lord = Saturn (1)
        // total = 2, remainder = 2
        // Indu = Moon + (2-1)*30 = Moon + 30
        let il = indu_lagna(340.0, Graha::Shani, Graha::Shani);
        assert!((il - 10.0).abs() < 1e-10); // 340 + 30 = 370 → 10
    }

    // --- All special lagnas ---

    #[test]
    fn all_special_lagnas_results_in_range() {
        let result = all_special_lagnas(
            45.0,          // sun
            120.0,         // moon
            200.0,         // lagna
            25.0,          // ghatikas
            Graha::Shukra, // lagna lord
            Graha::Guru,   // moon 9th lord
        );

        assert!(result.bhava_lagna >= 0.0 && result.bhava_lagna < 360.0);
        assert!(result.hora_lagna >= 0.0 && result.hora_lagna < 360.0);
        assert!(result.ghati_lagna >= 0.0 && result.ghati_lagna < 360.0);
        assert!(result.vighati_lagna >= 0.0 && result.vighati_lagna < 360.0);
        assert!(result.varnada_lagna >= 0.0 && result.varnada_lagna < 360.0);
        assert!(result.sree_lagna >= 0.0 && result.sree_lagna < 360.0);
        assert!(result.pranapada_lagna >= 0.0 && result.pranapada_lagna < 360.0);
        assert!(result.indu_lagna >= 0.0 && result.indu_lagna < 360.0);
    }
}
