//! Birth balance calculation for dasha systems.
//!
//! - Nakshatra-based: computed from Moon's position within its nakshatra.
//! - Rashi-based: computed from lagna's position within its rashi.

use crate::nakshatra::NAKSHATRA_SPAN_27;
use crate::util::normalize_360;

/// Compute nakshatra birth balance for a nakshatra-based dasha system.
///
/// Returns `(nakshatra_index, balance_days, elapsed_fraction)`:
/// - `nakshatra_index`: 0-based index (0=Ashwini..26=Revati) of the Moon's nakshatra
/// - `balance_days`: remaining days in the starting graha's period
/// - `elapsed_fraction`: fraction of nakshatra already traversed [0, 1)
pub fn nakshatra_birth_balance(moon_sidereal_lon: f64, entry_period_days: f64) -> (u8, f64, f64) {
    let lon = normalize_360(moon_sidereal_lon);
    let nak_idx = (lon / NAKSHATRA_SPAN_27).floor() as u8;
    let nak_idx = nak_idx.min(26);
    let position_in_nak = lon - (nak_idx as f64) * NAKSHATRA_SPAN_27;
    let elapsed_fraction = position_in_nak / NAKSHATRA_SPAN_27;
    let balance_days = entry_period_days * (1.0 - elapsed_fraction);
    (nak_idx, balance_days, elapsed_fraction)
}

/// Compute rashi birth balance for a rashi-based dasha system.
///
/// Returns `(balance_days, elapsed_fraction)`:
/// - `balance_days`: remaining days in the starting rashi's period
/// - `elapsed_fraction`: fraction of rashi already traversed [0, 1)
///
/// The lagna's position within its rashi determines how much of the first
/// mahadasha period has elapsed.
pub fn rashi_birth_balance(lagna_sidereal_lon: f64, entry_period_days: f64) -> (f64, f64) {
    let lon = normalize_360(lagna_sidereal_lon);
    let rashi_idx = (lon / 30.0).floor() as u8;
    let rashi_idx = rashi_idx.min(11);
    let position_in_rashi = lon - (rashi_idx as f64) * 30.0;
    let elapsed_fraction = position_in_rashi / 30.0;
    let balance_days = entry_period_days * (1.0 - elapsed_fraction);
    (balance_days, elapsed_fraction)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn balance_at_start_of_nakshatra() {
        // Moon exactly at 0 deg (start of Ashwini)
        let (idx, balance, frac) = nakshatra_birth_balance(0.0, 2555.75);
        assert_eq!(idx, 0);
        assert!((balance - 2555.75).abs() < 1e-10);
        assert!(frac.abs() < 1e-10);
    }

    #[test]
    fn balance_at_midpoint() {
        // Moon at midpoint of Ashwini: 6.6667 deg
        let mid = NAKSHATRA_SPAN_27 / 2.0;
        let (idx, balance, frac) = nakshatra_birth_balance(mid, 2555.75);
        assert_eq!(idx, 0);
        assert!((frac - 0.5).abs() < 1e-10);
        assert!((balance - 2555.75 * 0.5).abs() < 1e-6);
    }

    #[test]
    fn balance_at_end_of_nakshatra() {
        // Moon just before end of Ashwini: almost 13.333 deg
        let near_end = NAKSHATRA_SPAN_27 - 0.001;
        let (idx, balance, _frac) = nakshatra_birth_balance(near_end, 2555.75);
        assert_eq!(idx, 0);
        assert!(balance < 1.0); // very small remaining balance
    }

    #[test]
    fn balance_rohini() {
        // Moon at 40 deg → Rohini (index 3), ~0 deg into Rohini
        // Rohini starts at 3 * 13.333 = 40.0 deg
        let (idx, balance, frac) = nakshatra_birth_balance(40.0, 3652.5);
        assert_eq!(idx, 3);
        assert!(frac.abs() < 1e-10);
        assert!((balance - 3652.5).abs() < 1e-10);
    }

    #[test]
    fn balance_wraps() {
        // Negative longitude wraps correctly
        let (idx, _, _) = nakshatra_birth_balance(-1.0, 1000.0);
        // -1 → 359 deg → Revati (index 26)
        assert_eq!(idx, 26);
    }

    // ── Rashi birth balance tests ──

    #[test]
    fn rashi_balance_at_rashi_start() {
        // Lagna at 0 deg (start of Mesha) → full period
        let (balance, frac) = rashi_birth_balance(0.0, 2555.75);
        assert!((balance - 2555.75).abs() < 1e-10);
        assert!(frac.abs() < 1e-10);
    }

    #[test]
    fn rashi_balance_at_midpoint() {
        // Lagna at 15 deg (midpoint of Mesha) → half period
        let (balance, frac) = rashi_birth_balance(15.0, 2555.75);
        assert!((frac - 0.5).abs() < 1e-10);
        assert!((balance - 2555.75 * 0.5).abs() < 1e-6);
    }

    #[test]
    fn rashi_balance_at_end() {
        // Lagna near end of Mesha: 29.999 deg → tiny balance
        let (balance, _frac) = rashi_birth_balance(29.999, 2555.75);
        assert!(balance < 1.0);
    }

    #[test]
    fn rashi_balance_second_sign() {
        // Lagna at 30 deg (start of Vrishabha) → full period
        let (balance, frac) = rashi_birth_balance(30.0, 3000.0);
        assert!(frac.abs() < 1e-10);
        assert!((balance - 3000.0).abs() < 1e-10);
    }

    #[test]
    fn rashi_balance_wraps() {
        // Negative longitude wraps correctly
        let (balance, _frac) = rashi_birth_balance(-1.0, 1000.0);
        // -1 → 359 deg → Meena (index 11), position 29 deg → small balance
        assert!(balance < 100.0);
    }
}
