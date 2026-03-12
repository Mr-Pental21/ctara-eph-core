//! Kendradi dasha — rashi-based with Kendra→Panapara→Apoklima grouping.
//!
//! Signs are traversed in groups:
//! - Kendra (1,4,7,10) = houses 1,4,7,10 from starting point
//! - Panapara (2,5,8,11) = houses 2,5,8,11
//! - Apoklima (3,6,9,12) = houses 3,6,9,12
//!
//! Periods use Chara period years (variable, chart-dependent).
//! Three variants:
//! - Kendradi: starting from stronger of lagna or 7th
//! - KarakaKendradi: starting from Atmakaraka's sign
//! - KarakaKendradiGraha: starting from Atmakaraka's sign (graha-based sub-periods)
//!
//! Sub-period method: ProportionalFromParent.

use super::balance::rashi_birth_balance;
use super::chara::chara_period_years;
use super::rashi_dasha::{rashi_hierarchy, rashi_snapshot};
use super::rashi_strength::{RashiDashaInputs, atmakaraka, stronger_rashi};
use super::rashi_util::is_odd_sign;
use super::types::{
    DAYS_PER_YEAR, DashaEntity, DashaHierarchy, DashaLevel, DashaPeriod, DashaSnapshot, DashaSystem,
};
use super::variation::{DashaVariationConfig, SubPeriodMethod};
use crate::error::VedicError;

/// Default sub-period method for Kendradi dasha.
pub const KENDRADI_DEFAULT_METHOD: SubPeriodMethod = SubPeriodMethod::ProportionalFromParent;

/// Total Kendradi cycle years (chart-dependent, same as Chara total).
fn kendradi_total_years(inputs: &RashiDashaInputs) -> f64 {
    (0..12).map(|r| chara_period_years(r, inputs)).sum()
}

/// Generate the K→P→A rashi sequence from a starting rashi.
///
/// Kendra: start, start+3, start+6, start+9
/// Panapara: start+1, start+4, start+7, start+10
/// Apoklima: start+2, start+5, start+8, start+11
fn kendradi_sequence(start: u8) -> Vec<u8> {
    let forward = is_odd_sign(start);
    let mut seq = Vec::with_capacity(12);

    // Kendra positions: 0, 3, 6, 9 from start
    let kendra_offsets = [0u8, 3, 6, 9];
    let panapara_offsets = [1u8, 4, 7, 10];
    let apoklima_offsets = [2u8, 5, 8, 11];

    for group in &[
        &kendra_offsets[..],
        &panapara_offsets[..],
        &apoklima_offsets[..],
    ] {
        for &offset in *group {
            let rashi = if forward {
                (start + offset) % 12
            } else {
                (start + 12 - offset) % 12
            };
            seq.push(rashi);
        }
    }

    seq
}

/// Generate level-0 periods for Kendradi dasha.
fn kendradi_level0_from_start(
    birth_jd: f64,
    start: u8,
    inputs: &RashiDashaInputs,
) -> Vec<DashaPeriod> {
    let sequence = kendradi_sequence(start);

    let first_rashi = sequence[0];
    let first_period_days = chara_period_years(first_rashi, inputs) * DAYS_PER_YEAR;
    let (balance_days, _frac) = rashi_birth_balance(inputs.lagna_sidereal_lon, first_period_days);

    let mut periods = Vec::with_capacity(12);
    let mut cursor = birth_jd;

    for (i, &rashi) in sequence.iter().enumerate() {
        let full_period_days = chara_period_years(rashi, inputs) * DAYS_PER_YEAR;
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

/// Determine starting rashi for standard Kendradi (stronger of lagna/7th).
fn kendradi_start(inputs: &RashiDashaInputs) -> u8 {
    let lagna = inputs.lagna_rashi_index;
    let seventh = (lagna + 6) % 12;
    stronger_rashi(lagna, seventh, inputs)
}

/// Determine starting rashi for Karaka Kendradi (Atmakaraka's sign).
fn karaka_kendradi_start(inputs: &RashiDashaInputs) -> u8 {
    let ak = atmakaraka(inputs);
    inputs.graha_rashi(ak)
}

// ── Kendradi ──

/// Generate level-0 for standard Kendradi.
pub fn kendradi_level0(birth_jd: f64, inputs: &RashiDashaInputs) -> Vec<DashaPeriod> {
    let start = kendradi_start(inputs);
    kendradi_level0_from_start(birth_jd, start, inputs)
}

/// Full hierarchy for Kendradi.
pub fn kendradi_hierarchy(
    birth_jd: f64,
    inputs: &RashiDashaInputs,
    max_level: u8,
    variation: &DashaVariationConfig,
) -> Result<DashaHierarchy, VedicError> {
    let level0 = kendradi_level0(birth_jd, inputs);
    let total = kendradi_total_years(inputs);
    let period_fn = |r: u8| chara_period_years(r, inputs);
    rashi_hierarchy(
        DashaSystem::Kendradi,
        birth_jd,
        level0,
        &period_fn,
        total,
        KENDRADI_DEFAULT_METHOD,
        max_level,
        variation,
    )
}

/// Snapshot for Kendradi.
pub fn kendradi_snapshot(
    birth_jd: f64,
    inputs: &RashiDashaInputs,
    query_jd: f64,
    max_level: u8,
    variation: &DashaVariationConfig,
) -> DashaSnapshot {
    let level0 = kendradi_level0(birth_jd, inputs);
    let total = kendradi_total_years(inputs);
    let period_fn = |r: u8| chara_period_years(r, inputs);
    rashi_snapshot(
        DashaSystem::Kendradi,
        level0,
        &period_fn,
        total,
        KENDRADI_DEFAULT_METHOD,
        query_jd,
        max_level,
        variation,
    )
}

// ── Karaka Kendradi ──

/// Generate level-0 for Karaka Kendradi.
pub fn karaka_kendradi_level0(birth_jd: f64, inputs: &RashiDashaInputs) -> Vec<DashaPeriod> {
    let start = karaka_kendradi_start(inputs);
    kendradi_level0_from_start(birth_jd, start, inputs)
}

/// Full hierarchy for Karaka Kendradi.
pub fn karaka_kendradi_hierarchy(
    birth_jd: f64,
    inputs: &RashiDashaInputs,
    max_level: u8,
    variation: &DashaVariationConfig,
) -> Result<DashaHierarchy, VedicError> {
    let level0 = karaka_kendradi_level0(birth_jd, inputs);
    let total = kendradi_total_years(inputs);
    let period_fn = |r: u8| chara_period_years(r, inputs);
    rashi_hierarchy(
        DashaSystem::KarakaKendradi,
        birth_jd,
        level0,
        &period_fn,
        total,
        KENDRADI_DEFAULT_METHOD,
        max_level,
        variation,
    )
}

/// Snapshot for Karaka Kendradi.
pub fn karaka_kendradi_snapshot(
    birth_jd: f64,
    inputs: &RashiDashaInputs,
    query_jd: f64,
    max_level: u8,
    variation: &DashaVariationConfig,
) -> DashaSnapshot {
    let level0 = karaka_kendradi_level0(birth_jd, inputs);
    let total = kendradi_total_years(inputs);
    let period_fn = |r: u8| chara_period_years(r, inputs);
    rashi_snapshot(
        DashaSystem::KarakaKendradi,
        level0,
        &period_fn,
        total,
        KENDRADI_DEFAULT_METHOD,
        query_jd,
        max_level,
        variation,
    )
}

// ── Karaka Kendradi Graha ──

/// Generate level-0 for Karaka Kendradi Graha.
/// Same starting point as Karaka Kendradi, but graha-based sub-periods
/// are handled at the variation/dispatch level.
pub fn karaka_kendradi_graha_level0(birth_jd: f64, inputs: &RashiDashaInputs) -> Vec<DashaPeriod> {
    let start = karaka_kendradi_start(inputs);
    kendradi_level0_from_start(birth_jd, start, inputs)
}

/// Full hierarchy for Karaka Kendradi Graha.
pub fn karaka_kendradi_graha_hierarchy(
    birth_jd: f64,
    inputs: &RashiDashaInputs,
    max_level: u8,
    variation: &DashaVariationConfig,
) -> Result<DashaHierarchy, VedicError> {
    let level0 = karaka_kendradi_graha_level0(birth_jd, inputs);
    let total = kendradi_total_years(inputs);
    let period_fn = |r: u8| chara_period_years(r, inputs);
    rashi_hierarchy(
        DashaSystem::KarakaKendradiGraha,
        birth_jd,
        level0,
        &period_fn,
        total,
        KENDRADI_DEFAULT_METHOD,
        max_level,
        variation,
    )
}

/// Snapshot for Karaka Kendradi Graha.
pub fn karaka_kendradi_graha_snapshot(
    birth_jd: f64,
    inputs: &RashiDashaInputs,
    query_jd: f64,
    max_level: u8,
    variation: &DashaVariationConfig,
) -> DashaSnapshot {
    let level0 = karaka_kendradi_graha_level0(birth_jd, inputs);
    let total = kendradi_total_years(inputs);
    let period_fn = |r: u8| chara_period_years(r, inputs);
    rashi_snapshot(
        DashaSystem::KarakaKendradiGraha,
        level0,
        &period_fn,
        total,
        KENDRADI_DEFAULT_METHOD,
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
    fn kendradi_sequence_from_mesha() {
        // Mesha(0), odd → forward
        // K: 0,3,6,9  P: 1,4,7,10  A: 2,5,8,11
        let seq = kendradi_sequence(0);
        assert_eq!(seq.len(), 12);
        assert_eq!(&seq[0..4], &[0, 3, 6, 9]);
        assert_eq!(&seq[4..8], &[1, 4, 7, 10]);
        assert_eq!(&seq[8..12], &[2, 5, 8, 11]);
    }

    #[test]
    fn kendradi_sequence_all_unique() {
        let seq = kendradi_sequence(0);
        let mut seen = [false; 12];
        for &r in &seq {
            assert!(!seen[r as usize], "rashi {} appears twice", r);
            seen[r as usize] = true;
        }
    }

    #[test]
    fn kendradi_sequence_even_start() {
        // Vrishabha(1), even → reverse
        // K: 1,10,7,4  P: 0,9,6,3  A: 11,8,5,2
        let seq = kendradi_sequence(1);
        assert_eq!(seq[0], 1);
        assert_eq!(seq[1], 10); // 1-3=10
        assert_eq!(seq[2], 7); // 1-6=7
        assert_eq!(seq[3], 4); // 1-9=4
    }

    #[test]
    fn kendradi_level0_12_periods() {
        let inputs = make_test_inputs();
        let periods = kendradi_level0(2451545.0, &inputs);
        assert_eq!(periods.len(), 12);
    }

    #[test]
    fn kendradi_level0_no_gaps() {
        let inputs = make_test_inputs();
        let periods = kendradi_level0(2451545.0, &inputs);
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
    fn kendradi_hierarchy_depth_1() {
        let inputs = make_test_inputs();
        let var = DashaVariationConfig::default();
        let h = kendradi_hierarchy(2451545.0, &inputs, 1, &var).unwrap();
        assert_eq!(h.levels.len(), 2);
        assert_eq!(h.levels[0].len(), 12);
    }

    #[test]
    fn karaka_kendradi_starts_from_ak() {
        let inputs = make_test_inputs();
        let ak = atmakaraka(&inputs);
        let ak_rashi = inputs.graha_rashi(ak);
        let periods = karaka_kendradi_level0(2451545.0, &inputs);
        // First period should have AK's rashi
        assert_eq!(periods[0].entity, DashaEntity::Rashi(ak_rashi));
    }

    #[test]
    fn karaka_kendradi_graha_level0_12() {
        let inputs = make_test_inputs();
        let periods = karaka_kendradi_graha_level0(2451545.0, &inputs);
        assert_eq!(periods.len(), 12);
    }

    #[test]
    fn all_three_variants_produce_12_periods() {
        let inputs = make_test_inputs();
        assert_eq!(kendradi_level0(2451545.0, &inputs).len(), 12);
        assert_eq!(karaka_kendradi_level0(2451545.0, &inputs).len(), 12);
        assert_eq!(karaka_kendradi_graha_level0(2451545.0, &inputs).len(), 12);
    }
}
