//! Mandooka (frog) dasha — rashi-based with jumping movement.
//!
//! Periods: Chara=7y, Sthira=8y, Dvisvabhava=9y. Total = 96y.
//! Starting rashi: stronger of lagna or 7th house sign.
//! Movement: jumps ±3 signs instead of sequential (+1/-1).
//! Direction: odd start = forward jumps (+3), even start = reverse jumps (-3).
//! Sub-period method: ProportionalFromParent.

use super::balance::rashi_birth_balance;
use super::query::find_active_period;
use super::rashi_strength::{RashiDashaInputs, stronger_rashi};
use super::rashi_util::{SignType, is_odd_sign, sign_type};
use super::subperiod::{equal_children, proportional_children};
use super::types::{
    DAYS_PER_YEAR, DashaEntity, DashaHierarchy, DashaLevel, DashaPeriod, DashaSnapshot,
    DashaSystem, MAX_DASHA_LEVEL, MAX_PERIODS_PER_LEVEL,
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
/// The full traversal covers three sign-type groups of four signs each.
/// Odd starting sign: jumps forward by 3 within each group, then advances to
/// the next group by +2. Even starting sign mirrors this in reverse.
fn mandooka_sequence(start: u8) -> Vec<u8> {
    let forward = is_odd_sign(start);
    let jump: i8 = if forward { 3 } else { -3 };
    let group_shift: i8 = if forward { 2 } else { -2 };

    let mut seq = Vec::with_capacity(12);
    for group in 0..3u8 {
        let seed = super::rashi_util::jump_rashi(start, group_shift * group as i8);
        let mut current = seed;
        for _ in 0..4 {
            seq.push(current);
            current = super::rashi_util::jump_rashi(current, jump);
        }
    }

    seq
}

fn mandooka_entity_sequence(parent_rashi: u8, method: SubPeriodMethod) -> Vec<DashaEntity> {
    let mut seq: Vec<DashaEntity> = mandooka_sequence(parent_rashi)
        .into_iter()
        .map(DashaEntity::Rashi)
        .collect();

    if matches!(
        method,
        SubPeriodMethod::ProportionalFromNext | SubPeriodMethod::EqualFromNext
    ) {
        seq.rotate_left(1);
    }

    seq
}

fn mandooka_period_sequence(parent_rashi: u8, method: SubPeriodMethod) -> Vec<(DashaEntity, f64)> {
    mandooka_entity_sequence(parent_rashi, method)
        .into_iter()
        .map(|entity| {
            let DashaEntity::Rashi(rashi) = entity else {
                unreachable!("mandooka sequences only contain rashis");
            };
            (entity, mandooka_period_years(rashi) * DAYS_PER_YEAR)
        })
        .collect()
}

pub fn mandooka_children(parent: &DashaPeriod, method: SubPeriodMethod) -> Vec<DashaPeriod> {
    let child_level = match parent.level.child_level() {
        Some(level) => level,
        None => return Vec::new(),
    };

    let DashaEntity::Rashi(parent_rashi) = parent.entity else {
        return Vec::new();
    };

    match method {
        SubPeriodMethod::ProportionalFromParent | SubPeriodMethod::ProportionalFromNext => {
            let seq = mandooka_period_sequence(parent_rashi, method);
            proportional_children(
                parent,
                &seq,
                MANDOOKA_TOTAL_YEARS * DAYS_PER_YEAR,
                child_level,
                0,
            )
        }
        SubPeriodMethod::EqualFromSame | SubPeriodMethod::EqualFromNext => {
            let seq = mandooka_entity_sequence(parent_rashi, method);
            equal_children(parent, &seq, child_level, 0)
        }
    }
}

pub fn mandooka_complete_level(
    parent_level: &[DashaPeriod],
    child_level: DashaLevel,
    method: SubPeriodMethod,
) -> Result<Vec<DashaPeriod>, VedicError> {
    let estimated = parent_level.len() * 12;
    if estimated > MAX_PERIODS_PER_LEVEL {
        return Err(VedicError::InvalidInput(
            "dasha level would exceed MAX_PERIODS_PER_LEVEL",
        ));
    }

    let mut result = Vec::with_capacity(estimated);
    for (pidx, parent) in parent_level.iter().enumerate() {
        let mut children = mandooka_children(parent, method);
        for child in &mut children {
            child.parent_idx = pidx as u32;
            child.level = child_level;
        }
        result.extend(children);
    }

    Ok(result)
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
    let max_level = max_level.min(MAX_DASHA_LEVEL);
    let mut levels: Vec<Vec<DashaPeriod>> = vec![level0];

    for depth in 1..=max_level {
        let child_level = match DashaLevel::from_u8(depth) {
            Some(level) => level,
            None => break,
        };
        let method = variation.method_for_level(depth - 1, MANDOOKA_DEFAULT_METHOD);
        let children = mandooka_complete_level(&levels[(depth - 1) as usize], child_level, method)?;
        levels.push(children);
    }

    Ok(DashaHierarchy {
        system: DashaSystem::Mandooka,
        birth_jd,
        levels,
    })
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
    let max_level = max_level.min(MAX_DASHA_LEVEL);
    let mut active_periods: Vec<DashaPeriod> = Vec::with_capacity((max_level + 1) as usize);

    let active_idx = match find_active_period(&level0, query_jd) {
        Some(idx) => idx,
        None => {
            return DashaSnapshot {
                system: DashaSystem::Mandooka,
                query_jd,
                periods: active_periods,
            };
        }
    };
    active_periods.push(level0[active_idx]);

    let mut current_parent = level0[active_idx];
    for depth in 1..=max_level {
        let method = variation.method_for_level(depth - 1, MANDOOKA_DEFAULT_METHOD);
        let children = mandooka_children(&current_parent, method);
        match find_active_period(&children, query_jd) {
            Some(idx) => {
                active_periods.push(children[idx]);
                current_parent = children[idx];
            }
            None => break,
        }
    }

    DashaSnapshot {
        system: DashaSystem::Mandooka,
        query_jd,
        periods: active_periods,
    }
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
        // Mesha(0) is odd, jumps +3 and wraps through all 12 rashis.
        let seq = mandooka_sequence(0);
        assert_eq!(seq.len(), 12);
        assert_eq!(seq, vec![0, 3, 6, 9, 2, 5, 8, 11, 4, 7, 10, 1]);
    }

    #[test]
    fn mandooka_sequence_from_vrishabha() {
        // Vrishabha(1) is even, so the same frog pattern runs in reverse.
        let seq = mandooka_sequence(1);
        assert_eq!(seq, vec![1, 10, 7, 4, 11, 8, 5, 2, 9, 6, 3, 0]);
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

    #[test]
    fn mandooka_children_start_from_parent_and_wrap() {
        let parent = DashaPeriod {
            entity: DashaEntity::Rashi(0),
            start_jd: 2451545.0,
            end_jd: 2451545.0 + 365.0,
            level: DashaLevel::Mahadasha,
            order: 1,
            parent_idx: 0,
        };

        let children = mandooka_children(&parent, MANDOOKA_DEFAULT_METHOD);
        let entities: Vec<_> = children.iter().map(|child| child.entity).collect();

        assert_eq!(
            entities,
            vec![
                DashaEntity::Rashi(0),
                DashaEntity::Rashi(3),
                DashaEntity::Rashi(6),
                DashaEntity::Rashi(9),
                DashaEntity::Rashi(2),
                DashaEntity::Rashi(5),
                DashaEntity::Rashi(8),
                DashaEntity::Rashi(11),
                DashaEntity::Rashi(4),
                DashaEntity::Rashi(7),
                DashaEntity::Rashi(10),
                DashaEntity::Rashi(1),
            ]
        );
        assert!((children[0].start_jd - parent.start_jd).abs() < 1e-10);
        assert!((children.last().unwrap().end_jd - parent.end_jd).abs() < 1e-10);
    }
}
