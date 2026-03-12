//! Kala dasha static data and configuration.
//!
//! Kala is the only graha-based dasha system in BPHS (Chapter 46).
//! Period durations are variable, determined by birth time relative to
//! sunrise/sunset. Each graha's period = kala_years × serial_number.
//!
//! Source: BPHS Chapter 46 (public domain Vedic text).

use crate::graha::Graha;

use super::types::DashaEntity;
use super::variation::SubPeriodMethod;

/// The 4 Kala periods that divide the day.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KalaPeriod {
    /// Twilight around sunrise (5 ghatikas before + 5 after).
    Khanda,
    /// Daytime (between Khanda end and Sudha start).
    Mugdha,
    /// Twilight around sunset (5 ghatikas before + 5 after).
    Sudha,
    /// Nighttime (between Sudha end and next Khanda start).
    Poorna,
}

/// Duration of one ghatika in days.
pub const GHATIKA_DAYS: f64 = 24.0 / 1440.0; // 24 minutes = 1/60 day

/// Half-width of twilight periods in ghatikas.
const TWILIGHT_HALF_GHATIKAS: f64 = 5.0;

/// Multiplier for twilight periods (Khanda, Sudha).
const TWILIGHT_MULTIPLIER: f64 = 4.0 / 15.0;

/// Multiplier for day/night periods (Mugdha, Poorna).
const DAYNIGHT_MULTIPLIER: f64 = 2.0 / 15.0;

/// Graha sequence for Kala dasha with serial numbers (period ratios).
/// Always starts from Surya. Serial number IS the period in "kala years".
pub const KALA_GRAHA_SEQUENCE: [(Graha, f64); 9] = [
    (Graha::Surya, 1.0),
    (Graha::Chandra, 2.0),
    (Graha::Mangal, 3.0),
    (Graha::Buddh, 4.0),
    (Graha::Guru, 5.0),
    (Graha::Shukra, 6.0),
    (Graha::Shani, 7.0),
    (Graha::Rahu, 8.0),
    (Graha::Ketu, 9.0),
];

/// Sum of all serial numbers (1+2+...+9 = 45).
pub const KALA_SERIAL_SUM: f64 = 45.0;

/// Default sub-period method for Kala dasha.
pub const KALA_DEFAULT_METHOD: SubPeriodMethod = SubPeriodMethod::ProportionalFromParent;

/// Result of determining which Kala period the birth falls in.
#[derive(Debug, Clone, Copy)]
pub struct KalaInfo {
    /// Which of the 4 Kala periods.
    pub period: KalaPeriod,
    /// Ghatikas elapsed from start of that period.
    pub ghatikas_passed: f64,
    /// Multiplier for this period type.
    pub multiplier: f64,
    /// Computed kala figure in years.
    pub kala_years: f64,
}

/// Determine the Kala period and compute the kala figure.
///
/// All inputs are JD UTC.
///
/// The day is divided into 4 periods:
/// - **Khanda**: sunrise ± 5 ghatikas (twilight)
/// - **Mugdha**: from Khanda end to Sudha start (daytime)
/// - **Sudha**: sunset ± 5 ghatikas (twilight)
/// - **Poorna**: from Sudha end to next Khanda start (nighttime)
pub fn compute_kala_info(birth_jd: f64, sunrise_jd: f64, sunset_jd: f64) -> KalaInfo {
    let twilight_half = TWILIGHT_HALF_GHATIKAS * GHATIKA_DAYS;

    let khanda_start = sunrise_jd - twilight_half;
    let khanda_end = sunrise_jd + twilight_half;
    let sudha_start = sunset_jd - twilight_half;
    let sudha_end = sunset_jd + twilight_half;

    let (period, period_start, multiplier) = if birth_jd >= khanda_start && birth_jd < khanda_end {
        (KalaPeriod::Khanda, khanda_start, TWILIGHT_MULTIPLIER)
    } else if birth_jd >= khanda_end && birth_jd < sudha_start {
        (KalaPeriod::Mugdha, khanda_end, DAYNIGHT_MULTIPLIER)
    } else if birth_jd >= sudha_start && birth_jd < sudha_end {
        (KalaPeriod::Sudha, sudha_start, TWILIGHT_MULTIPLIER)
    } else {
        // Poorna: night period (after sudha_end OR before khanda_start)
        let start = if birth_jd >= sudha_end {
            sudha_end
        } else {
            // Before sunrise next day — use previous day's sudha_end
            // Approximate: sudha_end of previous day ≈ sunset - 1 day + twilight_half
            sudha_end - 1.0
        };
        (KalaPeriod::Poorna, start, DAYNIGHT_MULTIPLIER)
    };

    let elapsed_days = (birth_jd - period_start).max(0.0);
    let ghatikas_passed = elapsed_days / GHATIKA_DAYS;
    let kala_years = ghatikas_passed * multiplier;

    KalaInfo {
        period,
        ghatikas_passed,
        multiplier,
        kala_years,
    }
}

/// Build the entity sequence with period ratios for sub-period generation.
///
/// Each entry is `(DashaEntity::Graha(g), serial_ratio)` where serial_ratio
/// is the graha's serial number. Total = 45.
pub fn kala_entity_sequence() -> Vec<(DashaEntity, f64)> {
    KALA_GRAHA_SEQUENCE
        .iter()
        .map(|&(g, serial)| (DashaEntity::Graha(g), serial))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kala_khanda_period() {
        // Birth 3 ghatikas into Khanda (3gh × 24min = 72min before sunrise → actually
        // sunrise at 6:00, birth at 5:12 → 48 min before sunrise = 2 ghatikas before.
        // Let's use: sunrise = JD+0.25 (6:00), birth = sunrise - 2gh in days
        let sunrise = 2451545.25;
        let sunset = 2451545.75;
        let birth = sunrise - 2.0 * GHATIKA_DAYS; // 2 ghatikas before sunrise = in Khanda

        let info = compute_kala_info(birth, sunrise, sunset);
        assert_eq!(info.period, KalaPeriod::Khanda);
        // 5gh - 2gh = 3gh from Khanda start
        assert!((info.ghatikas_passed - 3.0).abs() < 0.01);
        assert!((info.multiplier - 4.0 / 15.0).abs() < 1e-10);
        assert!((info.kala_years - 3.0 * 4.0 / 15.0).abs() < 0.01);
    }

    #[test]
    fn kala_mugdha_period() {
        // Birth during daytime: well after Khanda, before Sudha
        let sunrise = 2451545.25;
        let sunset = 2451545.75;
        let khanda_end = sunrise + TWILIGHT_HALF_GHATIKAS * GHATIKA_DAYS;
        let birth = khanda_end + 10.0 * GHATIKA_DAYS; // 10 ghatikas into Mugdha

        let info = compute_kala_info(birth, sunrise, sunset);
        assert_eq!(info.period, KalaPeriod::Mugdha);
        assert!((info.ghatikas_passed - 10.0).abs() < 0.01);
        assert!((info.multiplier - 2.0 / 15.0).abs() < 1e-10);
    }

    #[test]
    fn kala_sudha_period() {
        // Birth during sunset twilight
        let sunrise = 2451545.25;
        let sunset = 2451545.75;
        let sudha_start = sunset - TWILIGHT_HALF_GHATIKAS * GHATIKA_DAYS;
        let birth = sudha_start + 3.0 * GHATIKA_DAYS; // 3 ghatikas into Sudha

        let info = compute_kala_info(birth, sunrise, sunset);
        assert_eq!(info.period, KalaPeriod::Sudha);
        assert!((info.ghatikas_passed - 3.0).abs() < 0.01);
        assert!((info.multiplier - 4.0 / 15.0).abs() < 1e-10);
    }

    #[test]
    fn kala_poorna_period() {
        // Birth during nighttime (after Sudha end)
        let sunrise = 2451545.25;
        let sunset = 2451545.75;
        let sudha_end = sunset + TWILIGHT_HALF_GHATIKAS * GHATIKA_DAYS;
        let birth = sudha_end + 5.0 * GHATIKA_DAYS; // 5 ghatikas into Poorna

        let info = compute_kala_info(birth, sunrise, sunset);
        assert_eq!(info.period, KalaPeriod::Poorna);
        assert!((info.ghatikas_passed - 5.0).abs() < 0.01);
        assert!((info.multiplier - 2.0 / 15.0).abs() < 1e-10);
    }

    #[test]
    fn kala_entity_sequence_sum() {
        let seq = kala_entity_sequence();
        assert_eq!(seq.len(), 9);
        let total: f64 = seq.iter().map(|(_, s)| s).sum();
        assert!((total - KALA_SERIAL_SUM).abs() < 1e-10);
    }

    #[test]
    fn kala_serial_numbers() {
        // Verify graha ordering and serial numbers
        assert_eq!(KALA_GRAHA_SEQUENCE[0], (Graha::Surya, 1.0));
        assert_eq!(KALA_GRAHA_SEQUENCE[8], (Graha::Ketu, 9.0));
    }

    #[test]
    fn total_cycle_is_45_times_kala() {
        // For kala_years = 2.0, total = 2.0 × 45 = 90 years
        let kala_years = 2.0;
        let total: f64 = KALA_GRAHA_SEQUENCE
            .iter()
            .map(|&(_, serial)| serial * kala_years)
            .sum();
        assert!((total - 90.0).abs() < 1e-10);
    }

    #[test]
    fn ghatika_days_correct() {
        // 1 ghatika = 24 minutes = 24/1440 days
        assert!((GHATIKA_DAYS - 24.0 / 1440.0).abs() < 1e-15);
        // 60 ghatikas = 1 day
        assert!((60.0 * GHATIKA_DAYS - 1.0).abs() < 1e-10);
    }
}
