//! Kala dasha engine — the only graha-based dasha system (BPHS Ch.46).
//!
//! Kala uses birth time relative to sunrise/sunset to compute a "kala figure"
//! that determines all period durations. Each graha's period = kala_years × serial.
//!
//! Key differences from nakshatra-based systems:
//! - Requires sunrise/sunset JD (not Moon longitude)
//! - No birth balance — all periods start from birth
//! - Period durations are variable (depend on kala figure)
//! - Always starts with Surya
//!
//! Implements 6 computation tiers parallel to nakshatra engine.

use crate::error::VedicError;

use super::kala_data::{
    KALA_DEFAULT_METHOD, KALA_GRAHA_SEQUENCE, KALA_SERIAL_SUM, compute_kala_info,
    kala_entity_sequence,
};
use super::query::find_active_period;
use super::subperiod::generate_children;
use super::types::{
    DAYS_PER_YEAR, DashaEntity, DashaHierarchy, DashaLevel, DashaPeriod, DashaSnapshot,
    DashaSystem, MAX_DASHA_LEVEL, MAX_PERIODS_PER_LEVEL,
};
use super::variation::DashaVariationConfig;

// ── Tier 0: Level-0 (Mahadasha) generation ───────────────────────────

/// Generate all level-0 (mahadasha) periods for Kala dasha.
///
/// Kala has no birth balance. 9 periods start at birth, each with
/// duration = kala_years × serial_number × DAYS_PER_YEAR.
pub fn kala_level0(birth_jd: f64, sunrise_jd: f64, sunset_jd: f64) -> Vec<DashaPeriod> {
    let info = compute_kala_info(birth_jd, sunrise_jd, sunset_jd);
    let kala_years = info.kala_years;

    let mut periods = Vec::with_capacity(9);
    let mut cursor = birth_jd;

    for (i, &(graha, serial)) in KALA_GRAHA_SEQUENCE.iter().enumerate() {
        let period_days = kala_years * serial * DAYS_PER_YEAR;
        let end = cursor + period_days;
        periods.push(DashaPeriod {
            entity: DashaEntity::Graha(graha),
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

/// Get the level-0 period for a specific graha entity.
pub fn kala_level0_entity(
    birth_jd: f64,
    sunrise_jd: f64,
    sunset_jd: f64,
    entity: DashaEntity,
) -> Option<DashaPeriod> {
    let periods = kala_level0(birth_jd, sunrise_jd, sunset_jd);
    periods.into_iter().find(|p| p.entity == entity)
}

// ── Tier 1: Single child period ──────────────────────────────────────

/// Calculate one specific entity's sub-period within a parent.
pub fn kala_child_period(
    parent: &DashaPeriod,
    child_entity: DashaEntity,
    method: super::variation::SubPeriodMethod,
) -> Option<DashaPeriod> {
    let child_level = parent.level.child_level()?;
    let children = kala_children(parent, method);
    children
        .into_iter()
        .find(|c| c.entity == child_entity && c.level == child_level)
}

// ── Tier 2: All children of one parent ───────────────────────────────

/// Calculate all child periods for a single parent period.
///
/// Sub-period duration = (serial / 45) × parent_duration.
/// Sequence starts from parent's graha (ProportionalFromParent).
pub fn kala_children(
    parent: &DashaPeriod,
    method: super::variation::SubPeriodMethod,
) -> Vec<DashaPeriod> {
    let child_level = match parent.level.child_level() {
        Some(l) => l,
        None => return Vec::new(),
    };
    let seq = kala_entity_sequence();
    // Total period = KALA_SERIAL_SUM (ratios, not actual days)
    generate_children(parent, &seq, KALA_SERIAL_SUM, child_level, 0, method)
}

// ── Tier 3: Complete level from parent level ─────────────────────────

/// Calculate complete level N given all periods at level N-1.
pub fn kala_complete_level(
    parent_level: &[DashaPeriod],
    child_level: DashaLevel,
    method: super::variation::SubPeriodMethod,
) -> Result<Vec<DashaPeriod>, VedicError> {
    let estimated = parent_level.len() * 9;
    if estimated > MAX_PERIODS_PER_LEVEL {
        return Err(VedicError::InvalidInput(
            "dasha level would exceed MAX_PERIODS_PER_LEVEL",
        ));
    }

    let seq = kala_entity_sequence();
    let mut result = Vec::with_capacity(estimated);

    for (pidx, parent) in parent_level.iter().enumerate() {
        let children = generate_children(
            parent,
            &seq,
            KALA_SERIAL_SUM,
            child_level,
            pidx as u32,
            method,
        );
        result.extend(children);
    }

    Ok(result)
}

// ── Tier 4: Full hierarchy (levels 0..N) ─────────────────────────────

/// Calculate complete Kala dasha hierarchy from birth.
pub fn kala_hierarchy(
    birth_jd: f64,
    sunrise_jd: f64,
    sunset_jd: f64,
    max_level: u8,
    variation: &DashaVariationConfig,
) -> Result<DashaHierarchy, VedicError> {
    let max_level = max_level.min(MAX_DASHA_LEVEL);
    let level0 = kala_level0(birth_jd, sunrise_jd, sunset_jd);
    let mut levels: Vec<Vec<DashaPeriod>> = vec![level0];

    for depth in 1..=max_level {
        let child_level = match DashaLevel::from_u8(depth) {
            Some(l) => l,
            None => break,
        };
        let method = variation.method_for_level(depth - 1, KALA_DEFAULT_METHOD);
        let parent = &levels[(depth - 1) as usize];
        let children = kala_complete_level(parent, child_level, method)?;
        levels.push(children);
    }

    Ok(DashaHierarchy {
        system: DashaSystem::Kala,
        birth_jd,
        levels,
    })
}

// ── Tier 5: Snapshot-only path ───────────────────────────────────────

/// Find active Kala dasha periods at query_jd without materializing full hierarchy.
pub fn kala_snapshot(
    birth_jd: f64,
    sunrise_jd: f64,
    sunset_jd: f64,
    query_jd: f64,
    max_level: u8,
    variation: &DashaVariationConfig,
) -> DashaSnapshot {
    let max_level = max_level.min(MAX_DASHA_LEVEL);
    let level0 = kala_level0(birth_jd, sunrise_jd, sunset_jd);
    let mut active_periods: Vec<DashaPeriod> = Vec::with_capacity((max_level + 1) as usize);

    let active_idx = match find_active_period(&level0, query_jd) {
        Some(idx) => idx,
        None => {
            return DashaSnapshot {
                system: DashaSystem::Kala,
                query_jd,
                periods: active_periods,
            };
        }
    };
    active_periods.push(level0[active_idx]);

    let mut current_parent = level0[active_idx];
    for depth in 1..=max_level {
        let method = variation.method_for_level(depth - 1, KALA_DEFAULT_METHOD);
        let children = kala_children(&current_parent, method);
        match find_active_period(&children, query_jd) {
            Some(idx) => {
                active_periods.push(children[idx]);
                current_parent = children[idx];
            }
            None => break,
        }
    }

    DashaSnapshot {
        system: DashaSystem::Kala,
        query_jd,
        periods: active_periods,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dasha::kala_data::GHATIKA_DAYS;
    use crate::dasha::variation::SubPeriodMethod;
    use crate::graha::Graha;

    fn test_sunrise_sunset() -> (f64, f64) {
        // Sunrise at 6:00 UTC, sunset at 18:00 UTC on J2000
        (2451545.25, 2451545.75)
    }

    #[test]
    fn kala_level0_always_starts_surya() {
        let (sunrise, sunset) = test_sunrise_sunset();
        // Birth during daytime (10:00 UTC = JD + 0.4167)
        let birth = 2451545.0 + 10.0 / 24.0;
        let periods = kala_level0(birth, sunrise, sunset);

        assert_eq!(periods.len(), 9);
        assert_eq!(periods[0].entity, DashaEntity::Graha(Graha::Surya));
        assert_eq!(periods[8].entity, DashaEntity::Graha(Graha::Ketu));
    }

    #[test]
    fn kala_no_birth_balance() {
        // All periods start from birth, first period starts at birth_jd
        let (sunrise, sunset) = test_sunrise_sunset();
        let birth = 2451545.0 + 10.0 / 24.0;
        let periods = kala_level0(birth, sunrise, sunset);

        assert!((periods[0].start_jd - birth).abs() < 1e-10);
    }

    #[test]
    fn kala_total_cycle_is_45_times_kala_years() {
        let (sunrise, sunset) = test_sunrise_sunset();
        let khanda_end = sunrise + 5.0 * GHATIKA_DAYS;
        // Birth 10 ghatikas into Mugdha → kala_years = 10 × 2/15
        let birth = khanda_end + 10.0 * GHATIKA_DAYS;
        let periods = kala_level0(birth, sunrise, sunset);

        let expected_kala_years = 10.0 * 2.0 / 15.0;
        let total_days: f64 = periods.iter().map(|p| p.duration_days()).sum();
        let total_years = total_days / DAYS_PER_YEAR;
        let expected_total = expected_kala_years * KALA_SERIAL_SUM;

        assert!((total_years - expected_total).abs() < 0.01);
    }

    #[test]
    fn kala_period_ratios_correct() {
        let (sunrise, sunset) = test_sunrise_sunset();
        let birth = sunrise + 3.0 * GHATIKA_DAYS; // In Khanda
        let periods = kala_level0(birth, sunrise, sunset);

        // Surya (serial 1) should be 1/9 of Ketu (serial 9)
        let surya_dur = periods[0].duration_days();
        let ketu_dur = periods[8].duration_days();
        if surya_dur > 0.0 {
            assert!((ketu_dur / surya_dur - 9.0).abs() < 0.01);
        }
    }

    #[test]
    fn kala_adjacent_periods_no_gaps() {
        let (sunrise, sunset) = test_sunrise_sunset();
        let birth = 2451545.0 + 10.0 / 24.0;
        let periods = kala_level0(birth, sunrise, sunset);

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
    fn kala_children_count_and_sum() {
        let (sunrise, sunset) = test_sunrise_sunset();
        let birth = 2451545.0 + 10.0 / 24.0;
        let periods = kala_level0(birth, sunrise, sunset);

        let children = kala_children(&periods[0], SubPeriodMethod::ProportionalFromParent);
        assert_eq!(children.len(), 9);

        // First child is Surya (parent entity)
        assert_eq!(children[0].entity, DashaEntity::Graha(Graha::Surya));

        // Last child ends at parent end
        assert!((children.last().unwrap().end_jd - periods[0].end_jd).abs() < 1e-10);
        // First child starts at parent start
        assert!((children[0].start_jd - periods[0].start_jd).abs() < 1e-10);
    }

    #[test]
    fn kala_children_start_from_parent_graha() {
        let (sunrise, sunset) = test_sunrise_sunset();
        let birth = 2451545.0 + 10.0 / 24.0;
        let periods = kala_level0(birth, sunrise, sunset);

        // Children of Chandra (index 1) should start from Chandra
        let children = kala_children(&periods[1], SubPeriodMethod::ProportionalFromParent);
        assert_eq!(children[0].entity, DashaEntity::Graha(Graha::Chandra));
        assert_eq!(children[1].entity, DashaEntity::Graha(Graha::Mangal));
        // Wraps around: last entity before Chandra is Surya
        assert_eq!(children[8].entity, DashaEntity::Graha(Graha::Surya));
    }

    #[test]
    fn kala_hierarchy_level_counts() {
        let (sunrise, sunset) = test_sunrise_sunset();
        let birth = 2451545.0 + 10.0 / 24.0;
        let var = DashaVariationConfig::default();
        let h = kala_hierarchy(birth, sunrise, sunset, 2, &var).unwrap();

        assert_eq!(h.levels.len(), 3);
        assert_eq!(h.levels[0].len(), 9);
        assert_eq!(h.levels[1].len(), 81);
        assert_eq!(h.levels[2].len(), 729);
        assert_eq!(h.system, DashaSystem::Kala);
    }

    #[test]
    fn kala_snapshot_matches_hierarchy() {
        let (sunrise, sunset) = test_sunrise_sunset();
        let birth = 2451545.0 + 10.0 / 24.0;
        let var = DashaVariationConfig::default();
        let query_jd = birth + 500.0; // ~1.4 years after birth

        let h = kala_hierarchy(birth, sunrise, sunset, 2, &var).unwrap();
        let snap = kala_snapshot(birth, sunrise, sunset, query_jd, 2, &var);

        assert_eq!(snap.periods.len(), 3);
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
    fn kala_level0_entity_lookup() {
        let (sunrise, sunset) = test_sunrise_sunset();
        let birth = 2451545.0 + 10.0 / 24.0;
        let result = kala_level0_entity(birth, sunrise, sunset, DashaEntity::Graha(Graha::Guru));
        assert!(result.is_some());
        let period = result.unwrap();
        assert_eq!(period.entity, DashaEntity::Graha(Graha::Guru));
    }

    #[test]
    fn kala_zero_kala_years_produces_zero_duration_periods() {
        // Birth exactly at Khanda start → 0 ghatikas → kala_years = 0
        let sunrise = 2451545.25;
        let sunset = 2451545.75;
        let khanda_start = sunrise - 5.0 * GHATIKA_DAYS;
        let periods = kala_level0(khanda_start, sunrise, sunset);

        assert_eq!(periods.len(), 9);
        // All periods have zero duration
        for p in &periods {
            assert!((p.duration_days()).abs() < 1e-10);
        }
    }
}
