//! Shoola dasha — rashi-based, fixed periods by sign type.
//!
//! Periods: Chara=7y, Sthira=8y, Dvisvabhava=9y. Total = 96y.
//! Starting rashi: stronger of 2nd or 8th house sign.
//! Direction: odd start = forward, even start = reverse.
//! Sub-period method: ProportionalFromParent.

use super::balance::rashi_birth_balance;
use super::rashi_dasha::{rashi_hierarchy, rashi_snapshot};
use super::rashi_strength::{RashiDashaInputs, stronger_rashi};
use super::rashi_util::{SignType, is_odd_sign, sign_type};
use super::types::{
    DAYS_PER_YEAR, DashaEntity, DashaHierarchy, DashaLevel, DashaPeriod, DashaSnapshot, DashaSystem,
};
use super::variation::{DashaVariationConfig, SubPeriodMethod};
use crate::error::VedicError;

/// Default sub-period method for Shoola dasha.
pub const SHOOLA_DEFAULT_METHOD: SubPeriodMethod = SubPeriodMethod::ProportionalFromParent;

/// Total Shoola dasha cycle: 96 years.
pub const SHOOLA_TOTAL_YEARS: f64 = 96.0;

/// Fixed period (years) by sign type.
pub fn shoola_period_years(rashi_index: u8) -> f64 {
    match sign_type(rashi_index) {
        SignType::Chara => 7.0,
        SignType::Sthira => 8.0,
        SignType::Dvisvabhava => 9.0,
    }
}

/// Generate level-0 periods for Shoola dasha.
///
/// Starting rashi: stronger of 2nd house or 8th house sign.
pub fn shoola_level0(birth_jd: f64, inputs: &RashiDashaInputs) -> Vec<DashaPeriod> {
    let second = inputs.bhava_rashi_indices[1]; // House 2
    let eighth = inputs.bhava_rashi_indices[7]; // House 8
    let start = stronger_rashi(second, eighth, inputs);
    let forward = is_odd_sign(start);

    let first_period_days = shoola_period_years(start) * DAYS_PER_YEAR;
    let (balance_days, _frac) = rashi_birth_balance(inputs.lagna_sidereal_lon, first_period_days);

    let mut periods = Vec::with_capacity(12);
    let mut cursor = birth_jd;

    for i in 0..12u8 {
        let rashi = if forward {
            (start + i) % 12
        } else {
            (start + 12 - i) % 12
        };

        let full_period_days = shoola_period_years(rashi) * DAYS_PER_YEAR;
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

/// Full hierarchy for Shoola dasha.
pub fn shoola_hierarchy(
    birth_jd: f64,
    inputs: &RashiDashaInputs,
    max_level: u8,
    variation: &DashaVariationConfig,
) -> Result<DashaHierarchy, VedicError> {
    let level0 = shoola_level0(birth_jd, inputs);
    rashi_hierarchy(
        DashaSystem::Shoola,
        birth_jd,
        level0,
        &shoola_period_years,
        SHOOLA_TOTAL_YEARS,
        SHOOLA_DEFAULT_METHOD,
        max_level,
        variation,
    )
}

/// Snapshot for Shoola dasha.
pub fn shoola_snapshot(
    birth_jd: f64,
    inputs: &RashiDashaInputs,
    query_jd: f64,
    max_level: u8,
    variation: &DashaVariationConfig,
) -> DashaSnapshot {
    let level0 = shoola_level0(birth_jd, inputs);
    rashi_snapshot(
        DashaSystem::Shoola,
        level0,
        &shoola_period_years,
        SHOOLA_TOTAL_YEARS,
        SHOOLA_DEFAULT_METHOD,
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
        RashiDashaInputs::new(lons, 15.0)
    }

    #[test]
    fn shoola_total_96() {
        let total: f64 = (0..12).map(shoola_period_years).sum();
        assert!((total - 96.0).abs() < 1e-10);
    }

    #[test]
    fn shoola_level0_12_periods() {
        let inputs = make_test_inputs();
        let periods = shoola_level0(2451545.0, &inputs);
        assert_eq!(periods.len(), 12);
    }

    #[test]
    fn shoola_level0_no_gaps() {
        let inputs = make_test_inputs();
        let periods = shoola_level0(2451545.0, &inputs);
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
    fn shoola_hierarchy_depth_1() {
        let inputs = make_test_inputs();
        let var = DashaVariationConfig::default();
        let h = shoola_hierarchy(2451545.0, &inputs, 1, &var).unwrap();
        assert_eq!(h.levels.len(), 2);
        assert_eq!(h.levels[0].len(), 12);
    }
}
