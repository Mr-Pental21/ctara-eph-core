//! Chakra dasha — rashi-based, fixed 10 years per rashi.
//!
//! Simplest rashi-based system: 10 years per rashi, 120y total.
//! Always forward direction regardless of sign parity.
//! Starting sign depends on birth period:
//!
//! - Day birth: lagna rashi
//! - Night birth: 7th from lagna
//! - Twilight: 9th from lagna (sandhya)
//!
//! Sub-period method: EqualFromSame (÷12).

use super::balance::rashi_birth_balance;
use super::rashi_dasha::{rashi_hierarchy, rashi_snapshot};
use super::rashi_strength::RashiDashaInputs;
use super::types::{
    DAYS_PER_YEAR, DashaEntity, DashaHierarchy, DashaLevel, DashaPeriod, DashaSnapshot, DashaSystem,
};
use super::variation::{DashaVariationConfig, SubPeriodMethod};
use crate::error::VedicError;

/// Default sub-period method for Chakra dasha.
pub const CHAKRA_DEFAULT_METHOD: SubPeriodMethod = SubPeriodMethod::EqualFromSame;

/// Total Chakra dasha cycle: 120 years.
pub const CHAKRA_TOTAL_YEARS: f64 = 120.0;

/// Fixed period: 10 years for every rashi.
pub fn chakra_period_years(_rashi_index: u8) -> f64 {
    10.0
}

/// Birth period: day, night, or twilight.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BirthPeriod {
    /// Born during daytime (after sunrise, before sunset).
    Day,
    /// Born during nighttime (after sunset, before next sunrise).
    Night,
    /// Born during twilight (sandhya — dawn or dusk).
    Twilight,
}

/// Determine starting rashi for Chakra dasha.
fn chakra_start(inputs: &RashiDashaInputs, birth_period: BirthPeriod) -> u8 {
    match birth_period {
        BirthPeriod::Day => inputs.lagna_rashi_index,
        BirthPeriod::Night => (inputs.lagna_rashi_index + 6) % 12,
        BirthPeriod::Twilight => (inputs.lagna_rashi_index + 8) % 12,
    }
}

/// Generate level-0 periods for Chakra dasha.
///
/// Always forward direction, fixed 10y per rashi.
pub fn chakra_level0(
    birth_jd: f64,
    inputs: &RashiDashaInputs,
    birth_period: BirthPeriod,
) -> Vec<DashaPeriod> {
    let start = chakra_start(inputs, birth_period);

    let first_period_days = 10.0 * DAYS_PER_YEAR;
    let (balance_days, _frac) = rashi_birth_balance(inputs.lagna_sidereal_lon, first_period_days);

    let mut periods = Vec::with_capacity(12);
    let mut cursor = birth_jd;

    for i in 0..12u8 {
        let rashi = (start + i) % 12; // Always forward

        let full_period_days = 10.0 * DAYS_PER_YEAR;
        let duration = if i == 0 {
            balance_days
        } else {
            full_period_days
        };

        let end = cursor + duration;
        periods.push(DashaPeriod {
            entity: DashaEntity::Rashi(rashi),
            start_jd: cursor,
            end_jd: end,
            level: DashaLevel::Mahadasha,
            order: (i as u16) + 1,
            parent_idx: 0,
        });
        cursor = end;
    }

    periods
}

/// Full hierarchy for Chakra dasha.
pub fn chakra_hierarchy(
    birth_jd: f64,
    inputs: &RashiDashaInputs,
    birth_period: BirthPeriod,
    max_level: u8,
    variation: &DashaVariationConfig,
) -> Result<DashaHierarchy, VedicError> {
    let level0 = chakra_level0(birth_jd, inputs, birth_period);
    rashi_hierarchy(
        DashaSystem::Chakra,
        birth_jd,
        level0,
        &chakra_period_years,
        CHAKRA_TOTAL_YEARS,
        CHAKRA_DEFAULT_METHOD,
        max_level,
        variation,
    )
}

/// Snapshot for Chakra dasha.
pub fn chakra_snapshot(
    birth_jd: f64,
    inputs: &RashiDashaInputs,
    birth_period: BirthPeriod,
    query_jd: f64,
    max_level: u8,
    variation: &DashaVariationConfig,
) -> DashaSnapshot {
    let level0 = chakra_level0(birth_jd, inputs, birth_period);
    rashi_snapshot(
        DashaSystem::Chakra,
        level0,
        &chakra_period_years,
        CHAKRA_TOTAL_YEARS,
        CHAKRA_DEFAULT_METHOD,
        query_jd,
        max_level,
        variation,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dasha::rashi_strength::RashiDashaInputs;

    fn make_test_inputs() -> RashiDashaInputs {
        let lons = [40.0, 75.0, 195.0, 160.0, 250.0, 310.0, 100.0, 10.0, 190.0];
        RashiDashaInputs::new(lons, 15.0) // Lagna in Mesha (0)
    }

    #[test]
    fn chakra_start_day() {
        let inputs = make_test_inputs();
        assert_eq!(chakra_start(&inputs, BirthPeriod::Day), 0);
    }

    #[test]
    fn chakra_start_night() {
        let inputs = make_test_inputs();
        assert_eq!(chakra_start(&inputs, BirthPeriod::Night), 6); // 7th from Mesha
    }

    #[test]
    fn chakra_start_twilight() {
        let inputs = make_test_inputs();
        assert_eq!(chakra_start(&inputs, BirthPeriod::Twilight), 8); // 9th from Mesha
    }

    #[test]
    fn chakra_period_always_10() {
        for r in 0..12 {
            assert!((chakra_period_years(r) - 10.0).abs() < 1e-10);
        }
    }

    #[test]
    fn chakra_total_120() {
        let total: f64 = (0..12).map(chakra_period_years).sum();
        assert!((total - 120.0).abs() < 1e-10);
    }

    #[test]
    fn chakra_level0_12_periods() {
        let inputs = make_test_inputs();
        let periods = chakra_level0(2451545.0, &inputs, BirthPeriod::Day);
        assert_eq!(periods.len(), 12);
    }

    #[test]
    fn chakra_level0_starts_from_lagna_day() {
        let inputs = make_test_inputs();
        let periods = chakra_level0(2451545.0, &inputs, BirthPeriod::Day);
        assert_eq!(periods[0].entity, DashaEntity::Rashi(0)); // Mesha
    }

    #[test]
    fn chakra_level0_always_forward() {
        let inputs = make_test_inputs();
        let periods = chakra_level0(2451545.0, &inputs, BirthPeriod::Day);
        // Should be sequential: 0, 1, 2, ..., 11
        for (i, p) in periods.iter().enumerate() {
            assert_eq!(p.entity, DashaEntity::Rashi(i as u8));
        }
    }

    #[test]
    fn chakra_level0_no_gaps() {
        let inputs = make_test_inputs();
        let periods = chakra_level0(2451545.0, &inputs, BirthPeriod::Day);
        for i in 1..periods.len() {
            assert!(
                (periods[i].start_jd - periods[i - 1].end_jd).abs() < 1e-10,
                "gap between {} and {}",
                i - 1,
                i
            );
        }
    }

    #[test]
    fn chakra_hierarchy_depth_2() {
        let inputs = make_test_inputs();
        let var = DashaVariationConfig::default();
        let h = chakra_hierarchy(2451545.0, &inputs, BirthPeriod::Day, 2, &var).unwrap();
        assert_eq!(h.levels.len(), 3);
        assert_eq!(h.levels[0].len(), 12);
        assert_eq!(h.levels[1].len(), 144);
    }
}
