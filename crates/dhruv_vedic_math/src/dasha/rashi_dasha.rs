//! Generic rashi-based dasha engine (shared infrastructure for all 10 rashi systems).
//!
//! Parallel to `nakshatra.rs` and `yogini.rs`, this module provides the common
//! computation tiers (children, complete_level, hierarchy, snapshot) that all
//! rashi-based systems share. Each system provides its own `level0` function
//! (starting rashi, direction, period calculation) but reuses these shared tiers.

use crate::error::VedicError;

use super::query::find_active_period;
use super::subperiod::{equal_children, proportional_children};
use super::types::{
    DAYS_PER_YEAR, DashaEntity, DashaHierarchy, DashaLevel, DashaPeriod, DashaSnapshot,
    DashaSystem, MAX_DASHA_LEVEL, MAX_PERIODS_PER_LEVEL,
};
use super::variation::{DashaVariationConfig, SubPeriodMethod};

/// Generate rashi-based children for a parent period.
///
/// `period_years`: closure that returns the period in years for each rashi index.
/// `default_method`: the system's default sub-period method.
/// `method`: the actual method to use (may be overridden by variation).
pub fn rashi_children(
    parent: &DashaPeriod,
    period_years_fn: &dyn Fn(u8) -> f64,
    total_years: f64,
    _default_method: SubPeriodMethod,
    method: SubPeriodMethod,
) -> Vec<DashaPeriod> {
    let child_level = match parent.level.child_level() {
        Some(l) => l,
        None => return Vec::new(),
    };

    let parent_rashi = match parent.entity {
        DashaEntity::Rashi(r) => r,
        _ => return Vec::new(),
    };

    let forward = super::rashi_util::is_odd_sign(parent_rashi);

    match method {
        SubPeriodMethod::ProportionalFromParent => {
            let seq = build_rashi_period_sequence(parent_rashi, forward, true, period_years_fn);
            let total_days = total_years * DAYS_PER_YEAR;
            proportional_children(parent, &seq, total_days, child_level, 0)
        }
        SubPeriodMethod::ProportionalFromNext => {
            let seq = build_rashi_period_sequence(parent_rashi, forward, false, period_years_fn);
            let total_days = total_years * DAYS_PER_YEAR;
            proportional_children(parent, &seq, total_days, child_level, 0)
        }
        SubPeriodMethod::EqualFromSame => {
            let seq_entities = build_rashi_entity_sequence(parent_rashi, forward, true);
            equal_children(parent, &seq_entities, child_level, 0)
        }
        SubPeriodMethod::EqualFromNext => {
            let seq_entities = build_rashi_entity_sequence(parent_rashi, forward, false);
            equal_children(parent, &seq_entities, child_level, 0)
        }
    }
}

/// Build a 12-rashi sequence with actual period values.
fn build_rashi_period_sequence(
    start_rashi: u8,
    forward: bool,
    from_parent: bool,
    period_years_fn: &dyn Fn(u8) -> f64,
) -> Vec<(DashaEntity, f64)> {
    let actual_start = if from_parent {
        start_rashi
    } else if forward {
        (start_rashi + 1) % 12
    } else {
        (start_rashi + 11) % 12
    };

    let mut seq = Vec::with_capacity(12);
    for i in 0..12u8 {
        let rashi = if forward {
            (actual_start + i) % 12
        } else {
            (actual_start + 12 - i) % 12
        };
        let period_days = period_years_fn(rashi) * DAYS_PER_YEAR;
        seq.push((DashaEntity::Rashi(rashi), period_days));
    }
    seq
}

/// Build a 12-rashi entity-only sequence (for equal-division methods).
fn build_rashi_entity_sequence(
    start_rashi: u8,
    forward: bool,
    from_parent: bool,
) -> Vec<DashaEntity> {
    let actual_start = if from_parent {
        start_rashi
    } else if forward {
        (start_rashi + 1) % 12
    } else {
        (start_rashi + 11) % 12
    };

    let mut seq = Vec::with_capacity(12);
    for i in 0..12u8 {
        let rashi = if forward {
            (actual_start + i) % 12
        } else {
            (actual_start + 12 - i) % 12
        };
        seq.push(DashaEntity::Rashi(rashi));
    }
    seq
}

/// Complete a level from its parent level (Tier 3).
pub fn rashi_complete_level(
    parent_level: &[DashaPeriod],
    period_years_fn: &dyn Fn(u8) -> f64,
    total_years: f64,
    _child_level: DashaLevel,
    default_method: SubPeriodMethod,
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
        let mut children =
            rashi_children(parent, period_years_fn, total_years, default_method, method);
        // Fix parent_idx for complete level
        for c in &mut children {
            c.parent_idx = pidx as u32;
        }
        result.extend(children);
    }

    Ok(result)
}

/// Build full hierarchy from a level-0 generator (Tier 4).
///
/// `level0_fn`: system-specific function that generates level-0 periods.
/// `period_years_fn`: returns period years for a given rashi index.
/// `total_years`: sum of all 12 rashi periods.
#[allow(clippy::too_many_arguments)]
pub fn rashi_hierarchy(
    system: DashaSystem,
    birth_jd: f64,
    level0: Vec<DashaPeriod>,
    period_years_fn: &dyn Fn(u8) -> f64,
    total_years: f64,
    default_method: SubPeriodMethod,
    max_level: u8,
    variation: &DashaVariationConfig,
) -> Result<DashaHierarchy, VedicError> {
    let max_level = max_level.min(MAX_DASHA_LEVEL);
    let mut levels: Vec<Vec<DashaPeriod>> = vec![level0];

    for depth in 1..=max_level {
        let child_level = match DashaLevel::from_u8(depth) {
            Some(l) => l,
            None => break,
        };
        let method = variation.method_for_level(depth - 1, default_method);
        let parent = &levels[(depth - 1) as usize];
        let children = rashi_complete_level(
            parent,
            period_years_fn,
            total_years,
            child_level,
            default_method,
            method,
        )?;
        levels.push(children);
    }

    Ok(DashaHierarchy {
        system,
        birth_jd,
        levels,
    })
}

/// Find active periods at query_jd without materializing full hierarchy (Tier 5).
#[allow(clippy::too_many_arguments)]
pub fn rashi_snapshot(
    system: DashaSystem,
    level0: Vec<DashaPeriod>,
    period_years_fn: &dyn Fn(u8) -> f64,
    total_years: f64,
    default_method: SubPeriodMethod,
    query_jd: f64,
    max_level: u8,
    variation: &DashaVariationConfig,
) -> DashaSnapshot {
    let max_level = max_level.min(MAX_DASHA_LEVEL);
    let mut active_periods: Vec<DashaPeriod> = Vec::with_capacity((max_level + 1) as usize);

    let active_idx = match find_active_period(&level0, query_jd) {
        Some(idx) => idx,
        None => {
            return DashaSnapshot {
                system,
                query_jd,
                periods: active_periods,
            };
        }
    };
    active_periods.push(level0[active_idx]);

    let mut current_parent = level0[active_idx];
    for depth in 1..=max_level {
        let method = variation.method_for_level(depth - 1, default_method);
        let children = rashi_children(
            &current_parent,
            period_years_fn,
            total_years,
            default_method,
            method,
        );
        match find_active_period(&children, query_jd) {
            Some(idx) => {
                active_periods.push(children[idx]);
                current_parent = children[idx];
            }
            None => break,
        }
    }

    DashaSnapshot {
        system,
        query_jd,
        periods: active_periods,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dasha::types::DAYS_PER_YEAR;

    /// Helper: create a simple level-0 with fixed 7y per rashi, forward from Mesha.
    fn test_level0(birth_jd: f64) -> Vec<DashaPeriod> {
        let mut periods = Vec::with_capacity(12);
        let mut cursor = birth_jd;
        for i in 0..12u8 {
            let duration = 7.0 * DAYS_PER_YEAR;
            let end = cursor + duration;
            periods.push(DashaPeriod {
                entity: DashaEntity::Rashi(i),
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

    fn test_period_years(_rashi: u8) -> f64 {
        7.0
    }

    #[test]
    fn rashi_children_equal_12() {
        let level0 = test_level0(2451545.0);
        let children = rashi_children(
            &level0[0],
            &test_period_years,
            84.0,
            SubPeriodMethod::EqualFromSame,
            SubPeriodMethod::EqualFromSame,
        );
        assert_eq!(children.len(), 12);
        // First child starts at parent start
        assert!((children[0].start_jd - level0[0].start_jd).abs() < 1e-10);
        // Last child ends at parent end
        assert!((children[11].end_jd - level0[0].end_jd).abs() < 1e-10);
    }

    #[test]
    fn rashi_children_proportional_sum() {
        let level0 = test_level0(2451545.0);
        let children = rashi_children(
            &level0[0],
            &test_period_years,
            84.0,
            SubPeriodMethod::ProportionalFromParent,
            SubPeriodMethod::ProportionalFromParent,
        );
        assert_eq!(children.len(), 12);
        assert!((children.last().unwrap().end_jd - level0[0].end_jd).abs() < 1e-10);
    }

    #[test]
    fn rashi_hierarchy_levels() {
        let birth_jd = 2451545.0;
        let level0 = test_level0(birth_jd);
        let var = DashaVariationConfig::default();
        let h = rashi_hierarchy(
            DashaSystem::Chara,
            birth_jd,
            level0,
            &test_period_years,
            84.0,
            SubPeriodMethod::EqualFromSame,
            2,
            &var,
        )
        .unwrap();

        assert_eq!(h.levels.len(), 3);
        assert_eq!(h.levels[0].len(), 12);
        assert_eq!(h.levels[1].len(), 144); // 12*12
        assert_eq!(h.levels[2].len(), 1728); // 12*12*12
    }

    #[test]
    fn rashi_snapshot_matches() {
        let birth_jd = 2451545.0;
        let level0 = test_level0(birth_jd);
        let var = DashaVariationConfig::default();
        let query_jd = birth_jd + 1000.0;

        let h = rashi_hierarchy(
            DashaSystem::Chara,
            birth_jd,
            level0.clone(),
            &test_period_years,
            84.0,
            SubPeriodMethod::EqualFromSame,
            2,
            &var,
        )
        .unwrap();

        let snap = rashi_snapshot(
            DashaSystem::Chara,
            level0,
            &test_period_years,
            84.0,
            SubPeriodMethod::EqualFromSame,
            query_jd,
            2,
            &var,
        );

        assert_eq!(snap.periods.len(), 3);
        for (level, snap_period) in snap.periods.iter().enumerate() {
            let active_in_h = h.levels[level]
                .iter()
                .find(|p| p.start_jd <= query_jd && query_jd < p.end_jd)
                .expect("should find active period");
            assert_eq!(snap_period.entity, active_in_h.entity);
        }
    }

    #[test]
    fn build_rashi_entity_sequence_forward() {
        let seq = build_rashi_entity_sequence(0, true, true);
        assert_eq!(seq[0], DashaEntity::Rashi(0));
        assert_eq!(seq[1], DashaEntity::Rashi(1));
        assert_eq!(seq[11], DashaEntity::Rashi(11));
    }

    #[test]
    fn build_rashi_entity_sequence_reverse() {
        let seq = build_rashi_entity_sequence(0, false, true);
        assert_eq!(seq[0], DashaEntity::Rashi(0));
        assert_eq!(seq[1], DashaEntity::Rashi(11));
        assert_eq!(seq[11], DashaEntity::Rashi(1));
    }

    #[test]
    fn build_rashi_entity_sequence_from_next() {
        let seq = build_rashi_entity_sequence(0, true, false);
        assert_eq!(seq[0], DashaEntity::Rashi(1));
        assert_eq!(seq[11], DashaEntity::Rashi(0));
    }
}
