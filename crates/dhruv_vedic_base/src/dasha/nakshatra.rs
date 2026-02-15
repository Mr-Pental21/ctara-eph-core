//! Generic nakshatra-based dasha engine (serves all 10 nakshatra systems).
//!
//! Implements 6 computation tiers:
//! - Tier 0: Level-0 (mahadasha) generation
//! - Tier 1: Single child period
//! - Tier 2: All children of one parent
//! - Tier 3: Complete level from parent level
//! - Tier 4: Full hierarchy (levels 0..N)
//! - Tier 5: Snapshot-only path (no full materialization)

use crate::error::VedicError;

use super::balance::nakshatra_birth_balance;
use super::nakshatra_data::NakshatraDashaConfig;
use super::query::find_active_period;
use super::subperiod::generate_children;
use super::types::{
    DashaEntity, DashaHierarchy, DashaLevel, DashaPeriod, DashaSnapshot, MAX_DASHA_LEVEL,
    MAX_PERIODS_PER_LEVEL,
};
use super::variation::{DashaVariationConfig, SubPeriodMethod};

// ── Tier 0: Level-0 (Mahadasha) generation ───────────────────────────

/// Generate all level-0 (mahadasha) periods from birth inputs.
pub fn nakshatra_level0(
    birth_jd: f64,
    moon_sidereal_lon: f64,
    config: &NakshatraDashaConfig,
) -> Vec<DashaPeriod> {
    let nak_idx = {
        let lon = crate::util::normalize_360(moon_sidereal_lon);
        (lon / crate::nakshatra::NAKSHATRA_SPAN_27).floor() as u8
    }
    .min(26);

    let start_graha_idx = config.starting_graha_idx(nak_idx) as usize;
    let entry_period = config.entry_period_days(nak_idx);
    let (_nak, balance_days, _frac) = nakshatra_birth_balance(moon_sidereal_lon, entry_period);

    let n = config.graha_sequence.len();
    let total_entries = n * config.cycle_count as usize;
    let mut periods = Vec::with_capacity(total_entries);
    let mut cursor = birth_jd;

    for cycle_offset in 0..total_entries {
        let seq_idx = (start_graha_idx + cycle_offset) % n;
        let graha = config.graha_sequence[seq_idx];
        let full_period = config.periods_days[seq_idx];

        let duration = if cycle_offset == 0 {
            balance_days
        } else {
            full_period
        };

        let end = cursor + duration;
        periods.push(DashaPeriod {
            entity: DashaEntity::Graha(graha),
            start_jd: cursor,
            end_jd: end,
            level: DashaLevel::Mahadasha,
            order: (cycle_offset as u16) + 1,
            parent_idx: 0,
        });
        cursor = end;
    }

    periods
}

/// Get the level-0 period for a specific entity.
pub fn nakshatra_level0_entity(
    birth_jd: f64,
    moon_sidereal_lon: f64,
    config: &NakshatraDashaConfig,
    entity: DashaEntity,
) -> Option<DashaPeriod> {
    let periods = nakshatra_level0(birth_jd, moon_sidereal_lon, config);
    periods.into_iter().find(|p| p.entity == entity)
}

// ── Tier 1: Single child period ──────────────────────────────────────

/// Calculate one specific entity's sub-period within a parent period.
pub fn nakshatra_child_period(
    parent: &DashaPeriod,
    child_entity: DashaEntity,
    config: &NakshatraDashaConfig,
    method: SubPeriodMethod,
) -> Option<DashaPeriod> {
    let child_level = parent.level.child_level()?;
    let children = nakshatra_children(parent, config, method);
    children
        .into_iter()
        .find(|c| c.entity == child_entity && c.level == child_level)
}

// ── Tier 2: All children of one parent ───────────────────────────────

/// Calculate all child periods for a single parent period.
pub fn nakshatra_children(
    parent: &DashaPeriod,
    config: &NakshatraDashaConfig,
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
pub fn nakshatra_complete_level(
    parent_level: &[DashaPeriod],
    config: &NakshatraDashaConfig,
    child_level: DashaLevel,
    method: SubPeriodMethod,
) -> Result<Vec<DashaPeriod>, VedicError> {
    let n = config.graha_sequence.len();
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
pub fn nakshatra_hierarchy(
    birth_jd: f64,
    moon_sidereal_lon: f64,
    config: &NakshatraDashaConfig,
    max_level: u8,
    variation: &DashaVariationConfig,
) -> Result<DashaHierarchy, VedicError> {
    let max_level = max_level.min(MAX_DASHA_LEVEL);
    let level0 = nakshatra_level0(birth_jd, moon_sidereal_lon, config);
    let mut levels: Vec<Vec<DashaPeriod>> = vec![level0];

    for depth in 1..=max_level {
        let child_level = match DashaLevel::from_u8(depth) {
            Some(l) => l,
            None => break,
        };
        let method = variation.method_for_level(depth - 1, config.default_method);
        let parent = &levels[(depth - 1) as usize];
        let children = nakshatra_complete_level(parent, config, child_level, method)?;
        levels.push(children);
    }

    Ok(DashaHierarchy {
        system: config.system,
        birth_jd,
        levels,
    })
}

// ── Tier 5: Snapshot-only path ───────────────────────────────────────

/// Find active periods at query_jd without materializing full hierarchy.
///
/// Generates only the chain of active periods from level 0 to max_level.
/// O(depth * sequence_length) instead of O(sequence_length^depth).
pub fn nakshatra_snapshot(
    birth_jd: f64,
    moon_sidereal_lon: f64,
    config: &NakshatraDashaConfig,
    query_jd: f64,
    max_level: u8,
    variation: &DashaVariationConfig,
) -> DashaSnapshot {
    let max_level = max_level.min(MAX_DASHA_LEVEL);
    let level0 = nakshatra_level0(birth_jd, moon_sidereal_lon, config);
    let mut active_periods: Vec<DashaPeriod> = Vec::with_capacity((max_level + 1) as usize);

    // Find active mahadasha
    let active_idx = match find_active_period(&level0, query_jd) {
        Some(idx) => idx,
        None => {
            return DashaSnapshot {
                system: config.system,
                query_jd,
                periods: active_periods,
            };
        }
    };
    active_periods.push(level0[active_idx]);

    // Drill down through levels
    let mut current_parent = level0[active_idx];
    for depth in 1..=max_level {
        let method = variation.method_for_level(depth - 1, config.default_method);
        let children = nakshatra_children(&current_parent, config, method);
        match find_active_period(&children, query_jd) {
            Some(idx) => {
                active_periods.push(children[idx]);
                current_parent = children[idx];
            }
            None => break,
        }
    }

    DashaSnapshot {
        system: config.system,
        query_jd,
        periods: active_periods,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dasha::nakshatra_data::vimshottari_config;
    use crate::dasha::types::DAYS_PER_YEAR;
    use crate::graha::Graha;

    #[test]
    fn vimshottari_ashwini_0_deg() {
        // Moon at 0 deg (Ashwini start) → Ketu mahadasha, full 7y, no balance deduction
        let cfg = vimshottari_config();
        let birth_jd = 2451545.0; // J2000
        let periods = nakshatra_level0(birth_jd, 0.0, &cfg);

        assert_eq!(periods.len(), 9);
        assert_eq!(periods[0].entity, DashaEntity::Graha(Graha::Ketu));
        let ketu_years = periods[0].duration_days() / DAYS_PER_YEAR;
        assert!((ketu_years - 7.0).abs() < 1e-6);

        // Total should be 120 years
        let total_days: f64 = periods.iter().map(|p| p.duration_days()).sum();
        let total_years = total_days / DAYS_PER_YEAR;
        assert!((total_years - 120.0).abs() < 1e-6);
    }

    #[test]
    fn vimshottari_rohini_40_deg() {
        // Moon at 40 deg = start of Rohini → Chandra mahadasha with full balance
        let cfg = vimshottari_config();
        let birth_jd = 2451545.0;
        let periods = nakshatra_level0(birth_jd, 40.0, &cfg);

        assert_eq!(periods[0].entity, DashaEntity::Graha(Graha::Chandra));
        // Rohini starts at exactly 40.0 deg, so full 10y balance
        let chandra_years = periods[0].duration_days() / DAYS_PER_YEAR;
        assert!((chandra_years - 10.0).abs() < 0.01);
    }

    #[test]
    fn vimshottari_partial_balance() {
        // Moon at 46.667 deg (mid-Rohini) → Chandra with ~5y balance
        let cfg = vimshottari_config();
        let birth_jd = 2451545.0;
        let mid_rohini = 40.0 + crate::nakshatra::NAKSHATRA_SPAN_27 / 2.0;
        let periods = nakshatra_level0(birth_jd, mid_rohini, &cfg);

        assert_eq!(periods[0].entity, DashaEntity::Graha(Graha::Chandra));
        let chandra_years = periods[0].duration_days() / DAYS_PER_YEAR;
        assert!((chandra_years - 5.0).abs() < 0.1);
    }

    #[test]
    fn vimshottari_adjacent_periods_no_gaps() {
        let cfg = vimshottari_config();
        let periods = nakshatra_level0(2451545.0, 100.0, &cfg);
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
    fn vimshottari_children_count() {
        let cfg = vimshottari_config();
        let periods = nakshatra_level0(2451545.0, 0.0, &cfg);
        let children =
            nakshatra_children(&periods[0], &cfg, SubPeriodMethod::ProportionalFromParent);
        assert_eq!(children.len(), 9);
        // First child should be same entity as parent (Ketu)
        assert_eq!(children[0].entity, DashaEntity::Graha(Graha::Ketu));
    }

    #[test]
    fn vimshottari_children_sum_to_parent() {
        let cfg = vimshottari_config();
        let periods = nakshatra_level0(2451545.0, 0.0, &cfg);
        let parent = &periods[0];
        let children = nakshatra_children(parent, &cfg, SubPeriodMethod::ProportionalFromParent);

        // Last child end == parent end
        assert!((children.last().unwrap().end_jd - parent.end_jd).abs() < 1e-10);
        // First child start == parent start
        assert!((children[0].start_jd - parent.start_jd).abs() < 1e-10);
    }

    #[test]
    fn vimshottari_hierarchy_level_counts() {
        let cfg = vimshottari_config();
        let var = DashaVariationConfig::default();
        let h = nakshatra_hierarchy(2451545.0, 0.0, &cfg, 2, &var).unwrap();

        assert_eq!(h.levels.len(), 3); // 0, 1, 2
        assert_eq!(h.levels[0].len(), 9); // 9 mahadashas
        assert_eq!(h.levels[1].len(), 81); // 9*9 antardashas
        assert_eq!(h.levels[2].len(), 729); // 9*9*9 pratyantardashas
    }

    #[test]
    fn vimshottari_snapshot_matches_hierarchy() {
        let cfg = vimshottari_config();
        let var = DashaVariationConfig::default();
        let birth_jd = 2451545.0;
        let moon = 100.0;
        let query_jd = birth_jd + 1000.0; // ~2.7 years after birth

        let h = nakshatra_hierarchy(birth_jd, moon, &cfg, 2, &var).unwrap();
        let snap = nakshatra_snapshot(birth_jd, moon, &cfg, query_jd, 2, &var);

        assert_eq!(snap.periods.len(), 3);
        // Verify snapshot matches hierarchy by checking entities match
        for (level, snap_period) in snap.periods.iter().enumerate() {
            let active_in_h = h.levels[level]
                .iter()
                .find(|p| p.start_jd <= query_jd && query_jd < p.end_jd)
                .expect("should find active period in hierarchy");
            assert_eq!(snap_period.entity, active_in_h.entity);
            assert!((snap_period.start_jd - active_in_h.start_jd).abs() < 1e-6);
        }
    }

    #[test]
    fn vimshottari_level0_entity_lookup() {
        let cfg = vimshottari_config();
        let birth_jd = 2451545.0;
        let result = nakshatra_level0_entity(birth_jd, 0.0, &cfg, DashaEntity::Graha(Graha::Ketu));
        assert!(result.is_some());
        let period = result.unwrap();
        assert_eq!(period.entity, DashaEntity::Graha(Graha::Ketu));
    }

    #[test]
    fn max_periods_cap() {
        // Level 4 of a 9-entity system: 9^5 = 59049, under cap
        // But level 5 would be 9^6 = 531441, over cap
        // We test that level 4 works fine
        let cfg = vimshottari_config();
        let var = DashaVariationConfig::default();
        let h = nakshatra_hierarchy(2451545.0, 0.0, &cfg, 4, &var).unwrap();
        assert_eq!(h.levels.len(), 5); // levels 0-4
    }
}
