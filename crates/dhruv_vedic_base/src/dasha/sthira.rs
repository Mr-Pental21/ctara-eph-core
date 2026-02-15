//! Sthira dasha — rashi-based, fixed periods by sign type.
//!
//! Periods: Chara=7y, Sthira=8y, Dvisvabhava=9y. Total = 96y.
//! Starting rashi: the sign occupied by Brahma Graha.
//! Direction: odd start = forward, even start = reverse.
//! Sub-period method: ProportionalFromNext.

use super::balance::rashi_birth_balance;
use super::rashi_dasha::{rashi_hierarchy, rashi_snapshot};
use super::rashi_strength::{RashiDashaInputs, brahma_graha};
use super::rashi_util::{SignType, is_odd_sign, sign_type};
use super::types::{
    DAYS_PER_YEAR, DashaEntity, DashaHierarchy, DashaLevel, DashaPeriod, DashaSnapshot, DashaSystem,
};
use super::variation::{DashaVariationConfig, SubPeriodMethod};
use crate::error::VedicError;

/// Default sub-period method for Sthira dasha.
pub const STHIRA_DEFAULT_METHOD: SubPeriodMethod = SubPeriodMethod::ProportionalFromNext;

/// Total Sthira dasha cycle: 4×7 + 4×8 + 4×9 = 96 years.
pub const STHIRA_TOTAL_YEARS: f64 = 96.0;

/// Fixed period (years) by sign type.
pub fn sthira_period_years(rashi_index: u8) -> f64 {
    match sign_type(rashi_index) {
        SignType::Chara => 7.0,
        SignType::Sthira => 8.0,
        SignType::Dvisvabhava => 9.0,
    }
}

/// Generate level-0 periods for Sthira dasha.
///
/// Starting sign: sign occupied by Brahma Graha.
pub fn sthira_level0(birth_jd: f64, inputs: &RashiDashaInputs) -> Vec<DashaPeriod> {
    let brahma = brahma_graha(inputs);
    let start = inputs.graha_rashi(brahma);
    let forward = is_odd_sign(start);

    let first_period_days = sthira_period_years(start) * DAYS_PER_YEAR;
    let (balance_days, _frac) = rashi_birth_balance(inputs.lagna_sidereal_lon, first_period_days);

    let mut periods = Vec::with_capacity(12);
    let mut cursor = birth_jd;

    for i in 0..12u8 {
        let rashi = if forward {
            (start + i) % 12
        } else {
            (start + 12 - i) % 12
        };

        let full_period_days = sthira_period_years(rashi) * DAYS_PER_YEAR;
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

/// Full hierarchy for Sthira dasha.
pub fn sthira_hierarchy(
    birth_jd: f64,
    inputs: &RashiDashaInputs,
    max_level: u8,
    variation: &DashaVariationConfig,
) -> Result<DashaHierarchy, VedicError> {
    let level0 = sthira_level0(birth_jd, inputs);
    rashi_hierarchy(
        DashaSystem::Sthira,
        birth_jd,
        level0,
        &sthira_period_years,
        STHIRA_TOTAL_YEARS,
        STHIRA_DEFAULT_METHOD,
        max_level,
        variation,
    )
}

/// Snapshot for Sthira dasha.
pub fn sthira_snapshot(
    birth_jd: f64,
    inputs: &RashiDashaInputs,
    query_jd: f64,
    max_level: u8,
    variation: &DashaVariationConfig,
) -> DashaSnapshot {
    let level0 = sthira_level0(birth_jd, inputs);
    rashi_snapshot(
        DashaSystem::Sthira,
        level0,
        &sthira_period_years,
        STHIRA_TOTAL_YEARS,
        STHIRA_DEFAULT_METHOD,
        query_jd,
        max_level,
        variation,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dasha::rashi_strength::RashiDashaInputs;
    use crate::graha::Graha;

    fn make_test_inputs() -> RashiDashaInputs {
        // Venus at 15 (Mesha, odd, house 1) — qualifies as Brahma
        let mut lons = [60.0; 9]; // All in Mithuna by default
        lons[Graha::Shukra.index() as usize] = 15.0; // Venus in Mesha
        lons[Graha::Guru.index() as usize] = 250.0; // Jupiter in Dhanu (odd but house 9 > 7, doesn't qualify)
        lons[Graha::Shani.index() as usize] = 45.0; // Saturn in Vrishabha (even, doesn't qualify)
        RashiDashaInputs::new(lons, 0.0) // Lagna at Mesha
    }

    #[test]
    fn sthira_period_chara_sign() {
        assert!((sthira_period_years(0) - 7.0).abs() < 1e-10); // Mesha = Chara
    }

    #[test]
    fn sthira_period_sthira_sign() {
        assert!((sthira_period_years(1) - 8.0).abs() < 1e-10); // Vrishabha = Sthira
    }

    #[test]
    fn sthira_period_dvi_sign() {
        assert!((sthira_period_years(2) - 9.0).abs() < 1e-10); // Mithuna = Dvisvabhava
    }

    #[test]
    fn sthira_total_96() {
        let total: f64 = (0..12).map(sthira_period_years).sum();
        assert!((total - 96.0).abs() < 1e-10);
    }

    #[test]
    fn sthira_level0_12_periods() {
        let inputs = make_test_inputs();
        let periods = sthira_level0(2451545.0, &inputs);
        assert_eq!(periods.len(), 12);
    }

    #[test]
    fn sthira_level0_no_gaps() {
        let inputs = make_test_inputs();
        let periods = sthira_level0(2451545.0, &inputs);
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
    fn sthira_hierarchy_depth_1() {
        let inputs = make_test_inputs();
        let var = DashaVariationConfig::default();
        let h = sthira_hierarchy(2451545.0, &inputs, 1, &var).unwrap();
        assert_eq!(h.levels.len(), 2);
        assert_eq!(h.levels[0].len(), 12);
        assert_eq!(h.levels[1].len(), 144);
    }
}
