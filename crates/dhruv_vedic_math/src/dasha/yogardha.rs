//! Yogardha dasha — average of Chara and Sthira periods.
//!
//! Period = (Chara_period + Sthira_period) / 2 for each rashi.
//! Starting rashi: stronger of lagna (1st house) or 7th house sign.
//! Direction: odd start = forward, even start = reverse.
//! Sub-period method: ProportionalFromNext.

use super::balance::rashi_birth_balance;
use super::chara::chara_period_years;
use super::rashi_dasha::{rashi_hierarchy, rashi_snapshot};
use super::rashi_strength::{RashiDashaInputs, stronger_rashi};
use super::rashi_util::is_odd_sign;
use super::sthira::sthira_period_years;
use super::types::{
    DAYS_PER_YEAR, DashaEntity, DashaHierarchy, DashaLevel, DashaPeriod, DashaSnapshot, DashaSystem,
};
use super::variation::{DashaVariationConfig, SubPeriodMethod};
use crate::error::VedicError;

/// Default sub-period method for Yogardha dasha.
pub const YOGARDHA_DEFAULT_METHOD: SubPeriodMethod = SubPeriodMethod::ProportionalFromNext;

/// Yogardha period = (Chara + Sthira) / 2.
pub fn yogardha_period_years(rashi_index: u8, inputs: &RashiDashaInputs) -> f64 {
    let chara = chara_period_years(rashi_index, inputs);
    let sthira = sthira_period_years(rashi_index);
    (chara + sthira) / 2.0
}

/// Total Yogardha cycle years (chart-dependent due to Chara component).
fn yogardha_total_years(inputs: &RashiDashaInputs) -> f64 {
    (0..12).map(|r| yogardha_period_years(r, inputs)).sum()
}

/// Generate level-0 periods for Yogardha dasha.
///
/// Starting rashi: stronger of lagna rashi or 7th house rashi.
pub fn yogardha_level0(birth_jd: f64, inputs: &RashiDashaInputs) -> Vec<DashaPeriod> {
    let lagna = inputs.lagna_rashi_index;
    let seventh = (lagna + 6) % 12;
    let start = stronger_rashi(lagna, seventh, inputs);
    let forward = is_odd_sign(start);

    let first_period_years = yogardha_period_years(start, inputs);
    let first_period_days = first_period_years * DAYS_PER_YEAR;
    let (balance_days, _frac) = rashi_birth_balance(inputs.lagna_sidereal_lon, first_period_days);

    let mut periods = Vec::with_capacity(12);
    let mut cursor = birth_jd;

    for i in 0..12u8 {
        let rashi = if forward {
            (start + i) % 12
        } else {
            (start + 12 - i) % 12
        };

        let full_period_days = yogardha_period_years(rashi, inputs) * DAYS_PER_YEAR;
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

/// Full hierarchy for Yogardha dasha.
pub fn yogardha_hierarchy(
    birth_jd: f64,
    inputs: &RashiDashaInputs,
    max_level: u8,
    variation: &DashaVariationConfig,
) -> Result<DashaHierarchy, VedicError> {
    let level0 = yogardha_level0(birth_jd, inputs);
    let total = yogardha_total_years(inputs);
    let period_fn = |r: u8| yogardha_period_years(r, inputs);
    rashi_hierarchy(
        DashaSystem::Yogardha,
        birth_jd,
        level0,
        &period_fn,
        total,
        YOGARDHA_DEFAULT_METHOD,
        max_level,
        variation,
    )
}

/// Snapshot for Yogardha dasha.
pub fn yogardha_snapshot(
    birth_jd: f64,
    inputs: &RashiDashaInputs,
    query_jd: f64,
    max_level: u8,
    variation: &DashaVariationConfig,
) -> DashaSnapshot {
    let level0 = yogardha_level0(birth_jd, inputs);
    let total = yogardha_total_years(inputs);
    let period_fn = |r: u8| yogardha_period_years(r, inputs);
    rashi_snapshot(
        DashaSystem::Yogardha,
        level0,
        &period_fn,
        total,
        YOGARDHA_DEFAULT_METHOD,
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
    fn yogardha_period_average() {
        let inputs = make_test_inputs();
        let chara = chara_period_years(0, &inputs);
        let sthira = sthira_period_years(0);
        let yogardha = yogardha_period_years(0, &inputs);
        assert!((yogardha - (chara + sthira) / 2.0).abs() < 1e-10);
    }

    #[test]
    fn yogardha_level0_12_periods() {
        let inputs = make_test_inputs();
        let periods = yogardha_level0(2451545.0, &inputs);
        assert_eq!(periods.len(), 12);
    }

    #[test]
    fn yogardha_level0_no_gaps() {
        let inputs = make_test_inputs();
        let periods = yogardha_level0(2451545.0, &inputs);
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
    fn yogardha_hierarchy_depth_1() {
        let inputs = make_test_inputs();
        let var = DashaVariationConfig::default();
        let h = yogardha_hierarchy(2451545.0, &inputs, 1, &var).unwrap();
        assert_eq!(h.levels.len(), 2);
        assert_eq!(h.levels[0].len(), 12);
    }
}
