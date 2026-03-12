//! Driga dasha — rashi-based, grouped by sign type.
//!
//! Signs are grouped by type: Chara, Sthira, Dvisvabhava.
//! Each group has 4 signs. Traversal order: Chara→Sthira→Dvisvabhava.
//! Periods: Chara=7y, Sthira=8y, Dvisvabhava=9y. Total = 96y.
//! Starting sign within each group: 9th lord aspecting the group's signs.
//! Sub-period method: ProportionalFromParent.

use super::balance::rashi_birth_balance;
use super::rashi_dasha::{rashi_hierarchy, rashi_snapshot};
use super::rashi_strength::RashiDashaInputs;
use super::rashi_util::{SignType, is_odd_sign, sign_type};
use super::types::{
    DAYS_PER_YEAR, DashaEntity, DashaHierarchy, DashaLevel, DashaPeriod, DashaSnapshot, DashaSystem,
};
use super::variation::{DashaVariationConfig, SubPeriodMethod};
use crate::error::VedicError;

/// Default sub-period method for Driga dasha.
pub const DRIGA_DEFAULT_METHOD: SubPeriodMethod = SubPeriodMethod::ProportionalFromParent;

/// Total Driga dasha cycle: 96 years.
pub const DRIGA_TOTAL_YEARS: f64 = 96.0;

/// Fixed period (years) by sign type (same as Sthira).
pub fn driga_period_years(rashi_index: u8) -> f64 {
    match sign_type(rashi_index) {
        SignType::Chara => 7.0,
        SignType::Sthira => 8.0,
        SignType::Dvisvabhava => 9.0,
    }
}

/// Get the 4 signs of a given type, in zodiacal order.
fn signs_of_type(st: SignType) -> [u8; 4] {
    let offset = match st {
        SignType::Chara => 0,
        SignType::Sthira => 1,
        SignType::Dvisvabhava => 2,
    };
    [offset, offset + 3, offset + 6, offset + 9]
}

/// Determine starting sign within a group based on lagna.
/// Uses the first sign in the group that contains lagna or is closest.
fn starting_sign_in_group(group: &[u8; 4], inputs: &RashiDashaInputs) -> u8 {
    // Check if lagna rashi is in this group
    for &s in group {
        if s == inputs.lagna_rashi_index {
            return s;
        }
    }
    // Otherwise, start from the first sign in the group
    group[0]
}

/// Generate the ordered 12-rashi sequence for Driga dasha.
///
/// Order: all Chara signs, then Sthira, then Dvisvabhava.
/// Within each group, start from the appropriate sign based on lagna.
fn driga_sequence(inputs: &RashiDashaInputs) -> Vec<u8> {
    let mut seq = Vec::with_capacity(12);

    for &st in &[SignType::Chara, SignType::Sthira, SignType::Dvisvabhava] {
        let group = signs_of_type(st);
        let start = starting_sign_in_group(&group, inputs);
        let start_pos = group.iter().position(|&s| s == start).unwrap_or(0);
        let forward = is_odd_sign(start);

        for i in 0..4u8 {
            let idx = if forward {
                (start_pos + i as usize) % 4
            } else {
                (start_pos + 4 - i as usize) % 4
            };
            seq.push(group[idx]);
        }
    }

    seq
}

/// Generate level-0 periods for Driga dasha.
pub fn driga_level0(birth_jd: f64, inputs: &RashiDashaInputs) -> Vec<DashaPeriod> {
    let sequence = driga_sequence(inputs);

    let first_rashi = sequence[0];
    let first_period_days = driga_period_years(first_rashi) * DAYS_PER_YEAR;
    let (balance_days, _frac) = rashi_birth_balance(inputs.lagna_sidereal_lon, first_period_days);

    let mut periods = Vec::with_capacity(12);
    let mut cursor = birth_jd;

    for (i, &rashi) in sequence.iter().enumerate() {
        let full_period_days = driga_period_years(rashi) * DAYS_PER_YEAR;
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

/// Full hierarchy for Driga dasha.
pub fn driga_hierarchy(
    birth_jd: f64,
    inputs: &RashiDashaInputs,
    max_level: u8,
    variation: &DashaVariationConfig,
) -> Result<DashaHierarchy, VedicError> {
    let level0 = driga_level0(birth_jd, inputs);
    rashi_hierarchy(
        DashaSystem::Driga,
        birth_jd,
        level0,
        &driga_period_years,
        DRIGA_TOTAL_YEARS,
        DRIGA_DEFAULT_METHOD,
        max_level,
        variation,
    )
}

/// Snapshot for Driga dasha.
pub fn driga_snapshot(
    birth_jd: f64,
    inputs: &RashiDashaInputs,
    query_jd: f64,
    max_level: u8,
    variation: &DashaVariationConfig,
) -> DashaSnapshot {
    let level0 = driga_level0(birth_jd, inputs);
    rashi_snapshot(
        DashaSystem::Driga,
        level0,
        &driga_period_years,
        DRIGA_TOTAL_YEARS,
        DRIGA_DEFAULT_METHOD,
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
    fn signs_of_type_chara() {
        let s = signs_of_type(SignType::Chara);
        assert_eq!(s, [0, 3, 6, 9]); // Mesha, Karka, Tula, Makara
    }

    #[test]
    fn signs_of_type_sthira() {
        let s = signs_of_type(SignType::Sthira);
        assert_eq!(s, [1, 4, 7, 10]); // Vrishabha, Simha, Vrischika, Kumbha
    }

    #[test]
    fn signs_of_type_dvi() {
        let s = signs_of_type(SignType::Dvisvabhava);
        assert_eq!(s, [2, 5, 8, 11]); // Mithuna, Kanya, Dhanu, Meena
    }

    #[test]
    fn driga_sequence_12_unique() {
        let inputs = make_test_inputs();
        let seq = driga_sequence(&inputs);
        assert_eq!(seq.len(), 12);
        // All 12 rashis should appear exactly once
        let mut seen = [false; 12];
        for &r in &seq {
            assert!(!seen[r as usize], "rashi {} appears twice", r);
            seen[r as usize] = true;
        }
    }

    #[test]
    fn driga_level0_12_periods() {
        let inputs = make_test_inputs();
        let periods = driga_level0(2451545.0, &inputs);
        assert_eq!(periods.len(), 12);
    }

    #[test]
    fn driga_level0_no_gaps() {
        let inputs = make_test_inputs();
        let periods = driga_level0(2451545.0, &inputs);
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
    fn driga_hierarchy_depth_1() {
        let inputs = make_test_inputs();
        let var = DashaVariationConfig::default();
        let h = driga_hierarchy(2451545.0, &inputs, 1, &var).unwrap();
        assert_eq!(h.levels.len(), 2);
        assert_eq!(h.levels[0].len(), 12);
    }
}
