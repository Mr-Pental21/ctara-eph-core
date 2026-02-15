//! Yogini dasha engine (8 yoginis, 36-year cycle).
//!
//! Implements 6 computation tiers parallel to the nakshatra engine,
//! but using DashaEntity::Yogini entities instead of DashaEntity::Graha.

use crate::error::VedicError;

use super::balance::nakshatra_birth_balance;
use super::query::find_active_period;
use super::subperiod::generate_children;
use super::types::{
    DashaHierarchy, DashaLevel, DashaPeriod, DashaSnapshot, DashaSystem, MAX_DASHA_LEVEL,
    MAX_PERIODS_PER_LEVEL,
};
use super::variation::{DashaVariationConfig, SubPeriodMethod};
use super::yogini_data::YoginiDashaConfig;

// ── Tier 0: Level-0 (Mahadasha) generation ───────────────────────────

/// Generate all level-0 (mahadasha) periods for Yogini dasha.
pub fn yogini_level0(
    birth_jd: f64,
    moon_sidereal_lon: f64,
    config: &YoginiDashaConfig,
) -> Vec<DashaPeriod> {
    let nak_idx = {
        let lon = crate::util::normalize_360(moon_sidereal_lon);
        (lon / crate::nakshatra::NAKSHATRA_SPAN_27).floor() as u8
    }
    .min(26);

    let start_yogini_idx = config.starting_yogini_idx(nak_idx) as usize;
    let entry_period = config.entry_period_days(nak_idx);
    let (_nak, balance_days, _frac) = nakshatra_birth_balance(moon_sidereal_lon, entry_period);

    let n = config.yogini_sequence.len();
    let mut periods = Vec::with_capacity(n);
    let mut cursor = birth_jd;

    for offset in 0..n {
        let seq_idx = (start_yogini_idx + offset) % n;
        let entity = config.yogini_sequence[seq_idx];
        let full_period = config.periods_days[seq_idx];

        let duration = if offset == 0 {
            balance_days
        } else {
            full_period
        };

        let end = cursor + duration;
        periods.push(DashaPeriod {
            entity,
            start_jd: cursor,
            end_jd: end,
            level: DashaLevel::Mahadasha,
            order: (offset as u16) + 1,
            parent_idx: 0,
        });
        cursor = end;
    }

    periods
}

/// Get the level-0 period for a specific yogini entity.
pub fn yogini_level0_entity(
    birth_jd: f64,
    moon_sidereal_lon: f64,
    config: &YoginiDashaConfig,
    entity: super::types::DashaEntity,
) -> Option<DashaPeriod> {
    let periods = yogini_level0(birth_jd, moon_sidereal_lon, config);
    periods.into_iter().find(|p| p.entity == entity)
}

// ── Tier 1: Single child period ──────────────────────────────────────

/// Calculate one specific entity's sub-period within a parent.
pub fn yogini_child_period(
    parent: &DashaPeriod,
    child_entity: super::types::DashaEntity,
    config: &YoginiDashaConfig,
    method: SubPeriodMethod,
) -> Option<DashaPeriod> {
    let child_level = parent.level.child_level()?;
    let children = yogini_children(parent, config, method);
    children
        .into_iter()
        .find(|c| c.entity == child_entity && c.level == child_level)
}

// ── Tier 2: All children of one parent ───────────────────────────────

/// Calculate all child periods for a single parent period.
pub fn yogini_children(
    parent: &DashaPeriod,
    config: &YoginiDashaConfig,
    method: SubPeriodMethod,
) -> Vec<DashaPeriod> {
    let child_level = match parent.level.child_level() {
        Some(l) => l,
        None => return Vec::new(),
    };
    let seq = config.entity_sequence();
    generate_children(
        parent,
        &seq,
        config.total_period_days,
        child_level,
        0,
        method,
    )
}

// ── Tier 3: Complete level from parent level ─────────────────────────

/// Calculate complete level N given all periods at level N-1.
pub fn yogini_complete_level(
    parent_level: &[DashaPeriod],
    config: &YoginiDashaConfig,
    child_level: DashaLevel,
    method: SubPeriodMethod,
) -> Result<Vec<DashaPeriod>, VedicError> {
    let n = config.yogini_sequence.len();
    let estimated = parent_level.len() * n;
    if estimated > MAX_PERIODS_PER_LEVEL {
        return Err(VedicError::InvalidInput(
            "dasha level would exceed MAX_PERIODS_PER_LEVEL",
        ));
    }

    let seq = config.entity_sequence();
    let mut result = Vec::with_capacity(estimated);

    for (pidx, parent) in parent_level.iter().enumerate() {
        let children = generate_children(
            parent,
            &seq,
            config.total_period_days,
            child_level,
            pidx as u32,
            method,
        );
        result.extend(children);
    }

    Ok(result)
}

// ── Tier 4: Full hierarchy (levels 0..N) ─────────────────────────────

/// Calculate birth balance, then all levels from 0 to max_level.
pub fn yogini_hierarchy(
    birth_jd: f64,
    moon_sidereal_lon: f64,
    config: &YoginiDashaConfig,
    max_level: u8,
    variation: &DashaVariationConfig,
) -> Result<DashaHierarchy, VedicError> {
    let max_level = max_level.min(MAX_DASHA_LEVEL);
    let level0 = yogini_level0(birth_jd, moon_sidereal_lon, config);
    let mut levels: Vec<Vec<DashaPeriod>> = vec![level0];

    for depth in 1..=max_level {
        let child_level = match DashaLevel::from_u8(depth) {
            Some(l) => l,
            None => break,
        };
        let method = variation.method_for_level(depth - 1, config.default_method);
        let parent = &levels[(depth - 1) as usize];
        let children = yogini_complete_level(parent, config, child_level, method)?;
        levels.push(children);
    }

    Ok(DashaHierarchy {
        system: DashaSystem::Yogini,
        birth_jd,
        levels,
    })
}

// ── Tier 5: Snapshot-only path ───────────────────────────────────────

/// Find active periods at query_jd without materializing full hierarchy.
pub fn yogini_snapshot(
    birth_jd: f64,
    moon_sidereal_lon: f64,
    config: &YoginiDashaConfig,
    query_jd: f64,
    max_level: u8,
    variation: &DashaVariationConfig,
) -> DashaSnapshot {
    let max_level = max_level.min(MAX_DASHA_LEVEL);
    let level0 = yogini_level0(birth_jd, moon_sidereal_lon, config);
    let mut active_periods: Vec<DashaPeriod> = Vec::with_capacity((max_level + 1) as usize);

    let active_idx = match find_active_period(&level0, query_jd) {
        Some(idx) => idx,
        None => {
            return DashaSnapshot {
                system: DashaSystem::Yogini,
                query_jd,
                periods: active_periods,
            };
        }
    };
    active_periods.push(level0[active_idx]);

    let mut current_parent = level0[active_idx];
    for depth in 1..=max_level {
        let method = variation.method_for_level(depth - 1, config.default_method);
        let children = yogini_children(&current_parent, config, method);
        match find_active_period(&children, query_jd) {
            Some(idx) => {
                active_periods.push(children[idx]);
                current_parent = children[idx];
            }
            None => break,
        }
    }

    DashaSnapshot {
        system: DashaSystem::Yogini,
        query_jd,
        periods: active_periods,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dasha::types::{DAYS_PER_YEAR, DashaEntity};
    use crate::dasha::yogini_data::yogini_config;

    #[test]
    fn yogini_level0_at_ardra() {
        // Moon at Ardra start (5 * 13.333 = 66.667 deg) → Mangala yogini
        let cfg = yogini_config();
        let birth_jd = 2451545.0;
        let ardra_start = 5.0 * crate::nakshatra::NAKSHATRA_SPAN_27;
        let periods = yogini_level0(birth_jd, ardra_start, &cfg);

        assert_eq!(periods.len(), 8);
        assert_eq!(periods[0].entity, DashaEntity::Yogini(0)); // Mangala
        let mangala_years = periods[0].duration_days() / DAYS_PER_YEAR;
        assert!(
            (mangala_years - 1.0).abs() < 0.01,
            "Mangala should be 1y, got {mangala_years}"
        );
    }

    #[test]
    fn yogini_total_span_36_at_nak_start() {
        let cfg = yogini_config();
        let birth_jd = 2451545.0;
        let ardra_start = 5.0 * crate::nakshatra::NAKSHATRA_SPAN_27;
        let periods = yogini_level0(birth_jd, ardra_start, &cfg);

        let total_days: f64 = periods.iter().map(|p| p.duration_days()).sum();
        let total_years = total_days / DAYS_PER_YEAR;
        assert!(
            (total_years - 36.0).abs() < 1e-6,
            "Total should be 36y at nak start, got {total_years}"
        );
    }

    #[test]
    fn yogini_hierarchy_depth_2() {
        let cfg = yogini_config();
        let var = DashaVariationConfig::default();
        let h = yogini_hierarchy(2451545.0, 100.0, &cfg, 2, &var).unwrap();

        assert_eq!(h.system, DashaSystem::Yogini);
        assert_eq!(h.levels.len(), 3);
        assert_eq!(h.levels[0].len(), 8); // 8 mahadashas
        assert_eq!(h.levels[1].len(), 64); // 8*8 antardashas
        assert_eq!(h.levels[2].len(), 512); // 8*8*8
    }

    #[test]
    fn yogini_snapshot_matches_hierarchy() {
        let cfg = yogini_config();
        let var = DashaVariationConfig::default();
        let birth_jd = 2451545.0;
        let moon = 200.0;
        let query_jd = birth_jd + 1000.0;

        let h = yogini_hierarchy(birth_jd, moon, &cfg, 2, &var).unwrap();
        let snap = yogini_snapshot(birth_jd, moon, &cfg, query_jd, 2, &var);

        assert_eq!(snap.periods.len(), 3);
        for (level, snap_period) in snap.periods.iter().enumerate() {
            let active_in_h = h.levels[level]
                .iter()
                .find(|p| p.start_jd <= query_jd && query_jd < p.end_jd)
                .expect("should find active period in hierarchy");
            assert_eq!(snap_period.entity, active_in_h.entity);
        }
    }

    #[test]
    fn yogini_adjacent_no_gaps() {
        let cfg = yogini_config();
        let periods = yogini_level0(2451545.0, 150.0, &cfg);
        for i in 1..periods.len() {
            assert!(
                (periods[i].start_jd - periods[i - 1].end_jd).abs() < 1e-10,
                "gap between periods {} and {}",
                i - 1,
                i
            );
        }
    }

    #[test]
    fn yogini_children_sum_to_parent() {
        let cfg = yogini_config();
        let periods = yogini_level0(2451545.0, 0.0, &cfg);
        let parent = &periods[0];
        let children = yogini_children(parent, &cfg, SubPeriodMethod::ProportionalFromParent);

        assert_eq!(children.len(), 8);
        assert!((children.last().unwrap().end_jd - parent.end_jd).abs() < 1e-10);
        assert!((children[0].start_jd - parent.start_jd).abs() < 1e-10);
    }
}
