//! Kaal Chakra dasha engine — special rashi-based system (BPHS Ch.46-53).
//!
//! Uses Moon's nakshatra pada to look up one of 24 Dasha Progressions (DPs).
//! Each DP defines a fixed 9-rashi sequence with predetermined durations.
//! Birth balance is computed from Moon's fractional position within its pada.
//!
//! Level-0 generates periods from the birth pada's DP (with balance applied),
//! then continues with the next pada's full DP.
//! Sub-periods use proportional distribution with the same fixed rashi durations.
//!
//! Implements 6 computation tiers parallel to nakshatra/rashi engines.

use crate::error::VedicError;
use crate::nakshatra::NAKSHATRA_SPAN_27;

use super::kaal_chakra_data::{
    ALL_DPS, KCD_DEFAULT_METHOD, KCD_NAKSHATRA_PADA_MAP, KCD_RASHI_YEARS, KCD_RASHIS_PER_DP,
    kcd_birth_balance, pada_from_nakshatra_position,
};
use super::query::find_active_period;
use super::subperiod::generate_children;
use super::types::{
    DAYS_PER_YEAR, DashaEntity, DashaHierarchy, DashaLevel, DashaPeriod, DashaSnapshot,
    DashaSystem, MAX_DASHA_LEVEL, MAX_PERIODS_PER_LEVEL,
};
use super::variation::DashaVariationConfig;

// ── Tier 0: Level-0 (Mahadasha) generation ───────────────────────────

/// Generate all level-0 (mahadasha) periods for Kaal Chakra dasha.
///
/// Generates periods from:
/// 1. Current pada's DP (starting rashi with birth balance, then remaining rashis)
/// 2. Next pada's full DP (all 9 rashis)
///
/// This covers ~83-200 years, sufficient for a human lifespan.
pub fn kaal_chakra_level0(birth_jd: f64, moon_sidereal_lon: f64) -> Vec<DashaPeriod> {
    let lon = crate::util::normalize_360(moon_sidereal_lon);
    let nak_idx = (lon / NAKSHATRA_SPAN_27).floor() as u8;
    let nak_idx = nak_idx.min(26);
    let position_in_nak = lon - (nak_idx as f64) * NAKSHATRA_SPAN_27;
    let pada_idx = pada_from_nakshatra_position(position_in_nak);

    let dp_idx = KCD_NAKSHATRA_PADA_MAP[nak_idx as usize][pada_idx as usize] as usize;
    let dp = &ALL_DPS[dp_idx];

    let (start_pos, balance_days) = kcd_birth_balance(moon_sidereal_lon, dp);

    let mut periods = Vec::with_capacity(KCD_RASHIS_PER_DP * 2);
    let mut cursor = birth_jd;
    let mut order: u16 = 1;

    // Part A: remaining rashis from current DP (starting at start_pos with balance)
    for i in start_pos..KCD_RASHIS_PER_DP {
        let rashi = dp.rashis[i];
        let duration_days = if i == start_pos {
            balance_days
        } else {
            KCD_RASHI_YEARS[rashi as usize] * DAYS_PER_YEAR
        };

        let end = cursor + duration_days;
        periods.push(DashaPeriod {
            entity: DashaEntity::Rashi(rashi),
            start_jd: cursor,
            end_jd: end,
            level: DashaLevel::Mahadasha,
            order,
            parent_idx: 0,
        });
        cursor = end;
        order += 1;
    }

    // Part B: next pada's full DP
    let (next_nak_idx, next_pada_idx) = next_pada(nak_idx, pada_idx);
    let next_dp_idx =
        KCD_NAKSHATRA_PADA_MAP[next_nak_idx as usize][next_pada_idx as usize] as usize;
    let next_dp = &ALL_DPS[next_dp_idx];

    for i in 0..KCD_RASHIS_PER_DP {
        let rashi = next_dp.rashis[i];
        let duration_days = KCD_RASHI_YEARS[rashi as usize] * DAYS_PER_YEAR;

        let end = cursor + duration_days;
        periods.push(DashaPeriod {
            entity: DashaEntity::Rashi(rashi),
            start_jd: cursor,
            end_jd: end,
            level: DashaLevel::Mahadasha,
            order,
            parent_idx: 0,
        });
        cursor = end;
        order += 1;
    }

    periods
}

/// Get the level-0 period for a specific rashi entity.
pub fn kaal_chakra_level0_entity(
    birth_jd: f64,
    moon_sidereal_lon: f64,
    entity: DashaEntity,
) -> Option<DashaPeriod> {
    let periods = kaal_chakra_level0(birth_jd, moon_sidereal_lon);
    periods.into_iter().find(|p| p.entity == entity)
}

// ── Tier 2: All children of one parent ───────────────────────────────

/// Calculate all child periods for a single parent period.
///
/// Sub-periods use 12-rashi proportional distribution with fixed durations,
/// starting from the parent's rashi.
pub fn kaal_chakra_children(
    parent: &DashaPeriod,
    method: super::variation::SubPeriodMethod,
) -> Vec<DashaPeriod> {
    let child_level = match parent.level.child_level() {
        Some(l) => l,
        None => return Vec::new(),
    };
    let seq = kcd_entity_sequence_12();
    let total: f64 = KCD_RASHI_YEARS.iter().sum();
    generate_children(parent, &seq, total, child_level, 0, method)
}

// ── Tier 3: Complete level from parent level ─────────────────────────

/// Calculate complete level N given all periods at level N-1.
pub fn kaal_chakra_complete_level(
    parent_level: &[DashaPeriod],
    child_level: DashaLevel,
    method: super::variation::SubPeriodMethod,
) -> Result<Vec<DashaPeriod>, VedicError> {
    let estimated = parent_level.len() * 12;
    if estimated > MAX_PERIODS_PER_LEVEL {
        return Err(VedicError::InvalidInput(
            "dasha level would exceed MAX_PERIODS_PER_LEVEL",
        ));
    }

    let seq = kcd_entity_sequence_12();
    let total: f64 = KCD_RASHI_YEARS.iter().sum();
    let mut result = Vec::with_capacity(estimated);

    for (pidx, parent) in parent_level.iter().enumerate() {
        let children = generate_children(parent, &seq, total, child_level, pidx as u32, method);
        result.extend(children);
    }

    Ok(result)
}

// ── Tier 4: Full hierarchy (levels 0..N) ─────────────────────────────

/// Calculate complete Kaal Chakra hierarchy from birth.
pub fn kaal_chakra_hierarchy(
    birth_jd: f64,
    moon_sidereal_lon: f64,
    max_level: u8,
    variation: &DashaVariationConfig,
) -> Result<DashaHierarchy, VedicError> {
    let max_level = max_level.min(MAX_DASHA_LEVEL);
    let level0 = kaal_chakra_level0(birth_jd, moon_sidereal_lon);
    let mut levels: Vec<Vec<DashaPeriod>> = vec![level0];

    for depth in 1..=max_level {
        let child_level = match DashaLevel::from_u8(depth) {
            Some(l) => l,
            None => break,
        };
        let method = variation.method_for_level(depth - 1, KCD_DEFAULT_METHOD);
        let parent = &levels[(depth - 1) as usize];
        let children = kaal_chakra_complete_level(parent, child_level, method)?;
        levels.push(children);
    }

    Ok(DashaHierarchy {
        system: DashaSystem::KaalChakra,
        birth_jd,
        levels,
    })
}

// ── Tier 5: Snapshot-only path ───────────────────────────────────────

/// Find active Kaal Chakra periods at query_jd without materializing full hierarchy.
pub fn kaal_chakra_snapshot(
    birth_jd: f64,
    moon_sidereal_lon: f64,
    query_jd: f64,
    max_level: u8,
    variation: &DashaVariationConfig,
) -> DashaSnapshot {
    let max_level = max_level.min(MAX_DASHA_LEVEL);
    let level0 = kaal_chakra_level0(birth_jd, moon_sidereal_lon);
    let mut active_periods: Vec<DashaPeriod> = Vec::with_capacity((max_level + 1) as usize);

    let active_idx = match find_active_period(&level0, query_jd) {
        Some(idx) => idx,
        None => {
            return DashaSnapshot {
                system: DashaSystem::KaalChakra,
                query_jd,
                periods: active_periods,
            };
        }
    };
    active_periods.push(level0[active_idx]);

    let mut current_parent = level0[active_idx];
    for depth in 1..=max_level {
        let method = variation.method_for_level(depth - 1, KCD_DEFAULT_METHOD);
        let children = kaal_chakra_children(&current_parent, method);
        match find_active_period(&children, query_jd) {
            Some(idx) => {
                active_periods.push(children[idx]);
                current_parent = children[idx];
            }
            None => break,
        }
    }

    DashaSnapshot {
        system: DashaSystem::KaalChakra,
        query_jd,
        periods: active_periods,
    }
}

// ── Internal helpers ─────────────────────────────────────────────────

/// Advance to the next pada. Wraps from pada 3 of nakshatra 26 back to pada 0 of nakshatra 0.
fn next_pada(nak_idx: u8, pada_idx: u8) -> (u8, u8) {
    if pada_idx < 3 {
        (nak_idx, pada_idx + 1)
    } else {
        let next_nak = if nak_idx >= 26 { 0 } else { nak_idx + 1 };
        (next_nak, 0)
    }
}

/// Build 12-rashi entity sequence for sub-period generation (all rashis, fixed durations).
fn kcd_entity_sequence_12() -> Vec<(DashaEntity, f64)> {
    (0..12u8)
        .map(|r| (DashaEntity::Rashi(r), KCD_RASHI_YEARS[r as usize]))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const BIRTH_JD: f64 = 2451545.0;

    #[test]
    fn kaal_chakra_level0_ashwini_pada1() {
        // Moon at 0 deg (Ashwini pada 1 start) → DP0, no elapsed, full Mesha first
        let periods = kaal_chakra_level0(BIRTH_JD, 0.0);

        // DP0 has 9 rashis + next pada's 9 rashis = 18 periods
        assert_eq!(periods.len(), 18);

        // First rashi = Mesha (DP0 starts with Mesha)
        assert_eq!(periods[0].entity, DashaEntity::Rashi(0)); // Mesha

        // First period should have full Mesha duration (7 years)
        let first_years = periods[0].duration_days() / DAYS_PER_YEAR;
        assert!((first_years - 7.0).abs() < 0.01);
    }

    #[test]
    fn kaal_chakra_level0_partial_balance() {
        // Moon at midpoint of Ashwini pada 1 → 50% elapsed in DP0
        let mid_pada = NAKSHATRA_SPAN_27 / 8.0; // Half of pada 1
        let periods = kaal_chakra_level0(BIRTH_JD, mid_pada);

        // First period should have reduced balance
        let first_years = periods[0].duration_days() / DAYS_PER_YEAR;
        assert!(first_years < 21.0); // Less than full first rashi
    }

    #[test]
    fn kaal_chakra_adjacent_no_gaps() {
        let periods = kaal_chakra_level0(BIRTH_JD, 100.0);
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
    fn kaal_chakra_starts_at_birth() {
        let birth = BIRTH_JD + 1234.567;
        let periods = kaal_chakra_level0(birth, 50.0);
        assert!((periods[0].start_jd - birth).abs() < 1e-10);
    }

    #[test]
    fn kaal_chakra_children_12_rashis() {
        let periods = kaal_chakra_level0(BIRTH_JD, 0.0);
        let children = kaal_chakra_children(
            &periods[0],
            super::super::variation::SubPeriodMethod::ProportionalFromParent,
        );

        assert_eq!(children.len(), 12);
        // First child starts at parent start
        assert!((children[0].start_jd - periods[0].start_jd).abs() < 1e-10);
        // Last child ends at parent end
        assert!((children.last().unwrap().end_jd - periods[0].end_jd).abs() < 1e-10);
    }

    #[test]
    fn kaal_chakra_hierarchy_level_counts() {
        let var = DashaVariationConfig::default();
        let h = kaal_chakra_hierarchy(BIRTH_JD, 0.0, 1, &var).unwrap();

        assert_eq!(h.levels.len(), 2);
        assert_eq!(h.levels[0].len(), 18); // 9 + 9 from 2 padas
        assert_eq!(h.levels[1].len(), 18 * 12); // 12 sub-periods each
        assert_eq!(h.system, DashaSystem::KaalChakra);
    }

    #[test]
    fn kaal_chakra_snapshot_matches_hierarchy() {
        let var = DashaVariationConfig::default();
        let query_jd = BIRTH_JD + 2000.0; // ~5.5 years after birth

        let h = kaal_chakra_hierarchy(BIRTH_JD, 0.0, 1, &var).unwrap();
        let snap = kaal_chakra_snapshot(BIRTH_JD, 0.0, query_jd, 1, &var);

        assert_eq!(snap.periods.len(), 2);
        for (level, snap_period) in snap.periods.iter().enumerate() {
            let active_in_h = h.levels[level]
                .iter()
                .find(|p| p.start_jd <= query_jd && query_jd < p.end_jd)
                .expect("should find active period in hierarchy");
            assert_eq!(snap_period.entity, active_in_h.entity);
        }
    }

    #[test]
    fn next_pada_wraps() {
        assert_eq!(next_pada(0, 0), (0, 1));
        assert_eq!(next_pada(0, 3), (1, 0));
        assert_eq!(next_pada(26, 3), (0, 0)); // Wraps to Ashwini
    }

    #[test]
    fn kcd_entity_sequence_12_complete() {
        let seq = kcd_entity_sequence_12();
        assert_eq!(seq.len(), 12);
        // Verify all 12 rashis present
        for (i, (entity, dur)) in seq.iter().enumerate() {
            assert_eq!(*entity, DashaEntity::Rashi(i as u8));
            assert!(*dur > 0.0);
        }
    }

    #[test]
    fn different_nakshatras_different_dps() {
        // Ashwini pada 1 → DP0 (direct), Rohini pada 1 → DP12 (indirect)
        let periods_ashwini = kaal_chakra_level0(BIRTH_JD, 0.0);
        let periods_rohini = kaal_chakra_level0(BIRTH_JD, 40.0 + 0.5); // ~Rohini

        // They should start with different rashis
        assert_ne!(periods_ashwini[0].entity, periods_rohini[0].entity);
    }
}
