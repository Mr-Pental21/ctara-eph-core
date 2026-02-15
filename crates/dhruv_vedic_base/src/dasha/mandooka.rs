//! Mandooka (frog) dasha — rashi-based with jumping movement.
//!
//! Periods: Chara=7y, Sthira=8y, Dvisvabhava=9y. Total = 96y.
//! Starting rashi: stronger of lagna or 7th house sign.
//! Movement: jumps ±3 signs instead of sequential (+1/-1).
//! Direction: odd start = forward jumps (+3), even start = reverse jumps (-3).
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

/// Default sub-period method for Mandooka dasha.
pub const MANDOOKA_DEFAULT_METHOD: SubPeriodMethod = SubPeriodMethod::ProportionalFromParent;

/// Total Mandooka dasha cycle: 96 years.
pub const MANDOOKA_TOTAL_YEARS: f64 = 96.0;

/// Fixed period (years) by sign type.
pub fn mandooka_period_years(rashi_index: u8) -> f64 {
    match sign_type(rashi_index) {
        SignType::Chara => 7.0,
        SignType::Sthira => 8.0,
        SignType::Dvisvabhava => 9.0,
    }
}

/// Generate the 12-rashi sequence for Mandooka (frog jump: ±3 signs).
///
/// Odd starting sign: jumps forward by 3 (0→3→6→9→0+1→4→7→10→2→5→8→11).
/// Even starting sign: jumps backward by 3.
fn mandooka_sequence(start: u8) -> Vec<u8> {
    let forward = is_odd_sign(start);
    let jump: i8 = if forward { 3 } else { -3 };

    let mut seq = Vec::with_capacity(12);
    let mut current = start;

    for _ in 0..12 {
        seq.push(current);
        current = ((current as i16 + jump as i16).rem_euclid(12)) as u8;
    }

    seq
}

/// Generate level-0 periods for Mandooka dasha.
///
/// Starting rashi: stronger of lagna or 7th house sign.
pub fn mandooka_level0(birth_jd: f64, inputs: &RashiDashaInputs) -> Vec<DashaPeriod> {
    let lagna = inputs.lagna_rashi_index;
    let seventh = (lagna + 6) % 12;
    let start = stronger_rashi(lagna, seventh, inputs);

    let sequence = mandooka_sequence(start);

    let first_rashi = sequence[0];
    let first_period_days = mandooka_period_years(first_rashi) * DAYS_PER_YEAR;
    let (balance_days, _frac) = rashi_birth_balance(inputs.lagna_sidereal_lon, first_period_days);

    let mut periods = Vec::with_capacity(12);
    let mut cursor = birth_jd;

    for (i, &rashi) in sequence.iter().enumerate() {
        let full_period_days = mandooka_period_years(rashi) * DAYS_PER_YEAR;
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

/// Full hierarchy for Mandooka dasha.
pub fn mandooka_hierarchy(
    birth_jd: f64,
    inputs: &RashiDashaInputs,
    max_level: u8,
    variation: &DashaVariationConfig,
) -> Result<DashaHierarchy, VedicError> {
    let level0 = mandooka_level0(birth_jd, inputs);
    rashi_hierarchy(
        DashaSystem::Mandooka,
        birth_jd,
        level0,
        &mandooka_period_years,
        MANDOOKA_TOTAL_YEARS,
        MANDOOKA_DEFAULT_METHOD,
        max_level,
        variation,
    )
}

/// Snapshot for Mandooka dasha.
pub fn mandooka_snapshot(
    birth_jd: f64,
    inputs: &RashiDashaInputs,
    query_jd: f64,
    max_level: u8,
    variation: &DashaVariationConfig,
) -> DashaSnapshot {
    let level0 = mandooka_level0(birth_jd, inputs);
    rashi_snapshot(
        DashaSystem::Mandooka,
        level0,
        &mandooka_period_years,
        MANDOOKA_TOTAL_YEARS,
        MANDOOKA_DEFAULT_METHOD,
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
    fn mandooka_sequence_from_mesha() {
        // Mesha(0) is odd, jumps +3: 0,3,6,9,0,3,6,9 → wraps to cover all 12
        let seq = mandooka_sequence(0);
        assert_eq!(seq.len(), 12);
        assert_eq!(seq[0], 0);
        assert_eq!(seq[1], 3);
        assert_eq!(seq[2], 6);
        assert_eq!(seq[3], 9);
        assert_eq!(seq[4], 0); // wraps back
    }

    #[test]
    fn mandooka_sequence_from_vrishabha() {
        // Vrishabha(1) is even, jumps -3: 1,10,7,4,1,10,7,4...
        let seq = mandooka_sequence(1);
        assert_eq!(seq[0], 1);
        assert_eq!(seq[1], 10);
        assert_eq!(seq[2], 7);
        assert_eq!(seq[3], 4);
    }

    #[test]
    fn mandooka_total_96() {
        let total: f64 = (0..12).map(mandooka_period_years).sum();
        assert!((total - 96.0).abs() < 1e-10);
    }

    #[test]
    fn mandooka_level0_12_periods() {
        let inputs = make_test_inputs();
        let periods = mandooka_level0(2451545.0, &inputs);
        assert_eq!(periods.len(), 12);
    }

    #[test]
    fn mandooka_level0_no_gaps() {
        let inputs = make_test_inputs();
        let periods = mandooka_level0(2451545.0, &inputs);
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
    fn mandooka_hierarchy_depth_1() {
        let inputs = make_test_inputs();
        let var = DashaVariationConfig::default();
        let h = mandooka_hierarchy(2451545.0, &inputs, 1, &var).unwrap();
        assert_eq!(h.levels.len(), 2);
        assert_eq!(h.levels[0].len(), 12);
    }
}
