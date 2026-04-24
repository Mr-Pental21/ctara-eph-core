//! Chara (Jaimini) dasha — rashi-based, variable periods.
//!
//! Period = distance from rashi to its lord (in signs).
//! Odd signs count forward, even signs count reverse.
//! If distance = 0 (lord in own sign), period = 12 years.
//! Special handling for Vrischika and Kumbha dual lordships.
//!
//! Starting rashi: lagna rashi. Direction: odd=forward, even=reverse.
//! Sub-period method: EqualFromSame (÷12).

use super::balance::rashi_birth_balance;
use super::rashi_dasha::{rashi_hierarchy, rashi_snapshot};
use super::rashi_strength::RashiDashaInputs;
use super::rashi_util::is_odd_sign;
use super::types::{
    DAYS_PER_YEAR, DashaEntity, DashaHierarchy, DashaLevel, DashaPeriod, DashaSnapshot, DashaSystem,
};
use super::variation::{DashaVariationConfig, SubPeriodMethod};
use crate::error::VedicError;
use crate::graha::Graha;
use crate::graha_relationships::{debilitation_degree, exaltation_degree};

/// Default sub-period method for Chara dasha.
pub const CHARA_DEFAULT_METHOD: SubPeriodMethod = SubPeriodMethod::EqualFromSame;

/// Total Chara dasha cycle years (sum of all 12 rashi periods).
/// Variable per chart — computed dynamically.
fn chara_total_years(inputs: &RashiDashaInputs) -> f64 {
    let mut total = 0.0;
    for r in 0..12u8 {
        total += chara_period_years(r, inputs);
    }
    total
}

/// Compute Chara dasha period (years) for a given rashi.
///
/// For odd signs: count signs forward from rashi to lord's rashi, minus 1.
/// For even signs: count signs reverse from rashi to lord's rashi, minus 1.
/// If result is 0, period = 12 years.
///
pub fn chara_period_years(rashi_index: u8, inputs: &RashiDashaInputs) -> f64 {
    let r = rashi_index % 12;
    let resolved = effective_chara_lord(r, inputs);

    if resolved.force_twelve_years {
        return 12.0;
    }

    let period = base_chara_period_years(r, resolved.lord_rashi);
    apply_year_adjustment(period, resolved.year_adjustment)
}

fn base_chara_period_years(rashi_index: u8, lord_rashi: u8) -> f64 {
    let r = rashi_index % 12;
    let lord_rashi = lord_rashi % 12;
    let distance = if is_odd_sign(r) {
        super::rashi_util::count_signs_forward(r, lord_rashi)
    } else {
        super::rashi_util::count_signs_reverse(r, lord_rashi)
    };
    let period = distance - 1;
    if period == 0 { 12.0 } else { period as f64 }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct EffectiveCharaLord {
    lord_rashi: u8,
    force_twelve_years: bool,
    year_adjustment: i8,
}

fn effective_chara_lord(rashi_index: u8, inputs: &RashiDashaInputs) -> EffectiveCharaLord {
    let r = rashi_index % 12;
    match r {
        7 => effective_dual_lord(r, Graha::Mangal, Graha::Ketu, inputs),
        10 => effective_dual_lord(r, Graha::Shani, Graha::Rahu, inputs),
        _ => {
            let lord = primary_lord_for_chara(r);
            EffectiveCharaLord {
                lord_rashi: inputs.graha_rashi(lord),
                force_twelve_years: false,
                year_adjustment: year_adjustment(lord, inputs),
            }
        }
    }
}

fn effective_dual_lord(
    target_rashi: u8,
    primary_lord: Graha,
    node_lord: Graha,
    inputs: &RashiDashaInputs,
) -> EffectiveCharaLord {
    let primary_rashi = inputs.graha_rashi(primary_lord);
    let node_rashi = inputs.graha_rashi(node_lord);
    let primary_in_target_own = primary_rashi == target_rashi;
    let node_in_target_own = node_rashi == target_rashi;

    if primary_in_target_own && node_in_target_own {
        return EffectiveCharaLord {
            lord_rashi: target_rashi,
            force_twelve_years: true,
            year_adjustment: 0,
        };
    }

    let selected = if primary_in_target_own != node_in_target_own {
        if primary_in_target_own {
            node_lord
        } else {
            primary_lord
        }
    } else {
        stronger_dual_lord(target_rashi, primary_lord, node_lord, inputs)
    };

    EffectiveCharaLord {
        lord_rashi: inputs.graha_rashi(selected),
        force_twelve_years: false,
        year_adjustment: year_adjustment(selected, inputs),
    }
}

fn stronger_dual_lord(
    target_rashi: u8,
    first: Graha,
    second: Graha,
    inputs: &RashiDashaInputs,
) -> Graha {
    let first_rashi = inputs.graha_rashi(first);
    let second_rashi = inputs.graha_rashi(second);

    let first_conj = conjunction_count(first, inputs);
    let second_conj = conjunction_count(second, inputs);
    let mut selected = if first_conj != second_conj {
        if first_conj > second_conj {
            first
        } else {
            second
        }
    } else {
        let first_modality = modality_strength(first_rashi);
        let second_modality = modality_strength(second_rashi);
        if first_modality != second_modality {
            if first_modality > second_modality {
                first
            } else {
                second
            }
        } else {
            let first_years = base_chara_period_years(target_rashi, first_rashi);
            let second_years = base_chara_period_years(target_rashi, second_rashi);
            if (first_years - second_years).abs() > 1e-10 {
                if first_years > second_years {
                    first
                } else {
                    second
                }
            } else {
                first
            }
        }
    };

    let first_exalted = is_in_exaltation_sign(first, inputs);
    let second_exalted = is_in_exaltation_sign(second, inputs);
    if first_exalted != second_exalted {
        selected = if first_exalted { first } else { second };
    }

    selected
}

fn primary_lord_for_chara(rashi_index: u8) -> Graha {
    match rashi_index % 12 {
        0 | 7 => Graha::Mangal,
        1 | 6 => Graha::Shukra,
        2 | 5 => Graha::Buddh,
        3 => Graha::Chandra,
        4 => Graha::Surya,
        8 | 11 => Graha::Guru,
        9 | 10 => Graha::Shani,
        _ => Graha::Surya,
    }
}

fn conjunction_count(graha: Graha, inputs: &RashiDashaInputs) -> u8 {
    let graha_rashi = inputs.graha_rashi(graha);
    let mut count = 0u8;
    for other in crate::graha::ALL_GRAHAS {
        if other != graha && inputs.graha_rashi(other) == graha_rashi {
            count += 1;
        }
    }
    count
}

fn modality_strength(rashi_index: u8) -> u8 {
    match rashi_index % 3 {
        0 => 0, // movable
        1 => 1, // fixed
        _ => 2, // dual
    }
}

fn year_adjustment(graha: Graha, inputs: &RashiDashaInputs) -> i8 {
    if is_in_exaltation_sign(graha, inputs) {
        1
    } else if is_in_debilitation_sign(graha, inputs) {
        -1
    } else {
        0
    }
}

fn is_in_exaltation_sign(graha: Graha, inputs: &RashiDashaInputs) -> bool {
    exaltation_degree(graha)
        .map(|deg| inputs.graha_rashi(graha) == ((deg / 30.0).floor() as u8 % 12))
        .unwrap_or(false)
}

fn is_in_debilitation_sign(graha: Graha, inputs: &RashiDashaInputs) -> bool {
    debilitation_degree(graha)
        .map(|deg| inputs.graha_rashi(graha) == ((deg / 30.0).floor() as u8 % 12))
        .unwrap_or(false)
}

fn apply_year_adjustment(period: f64, adjustment: i8) -> f64 {
    (period + adjustment as f64).max(0.0)
}

/// Generate level-0 (mahadasha) periods for Chara dasha.
///
/// Starting rashi: lagna rashi.
/// Direction: odd lagna = forward through zodiac, even lagna = reverse.
pub fn chara_level0(birth_jd: f64, inputs: &RashiDashaInputs) -> Vec<DashaPeriod> {
    let start = inputs.lagna_rashi_index;
    let forward = is_odd_sign(start);

    let first_period_years = chara_period_years(start, inputs);
    let first_period_days = first_period_years * DAYS_PER_YEAR;
    let (balance_days, _frac) = rashi_birth_balance(inputs.lagna_sidereal_lon, first_period_days);

    let mut periods = Vec::with_capacity(12);
    let mut cursor = birth_jd;

    for i in 0..12u8 {
        let rashi = if forward {
            (start + i) % 12
        } else {
            (start + 12 - i) % 12
        };

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

/// Full hierarchy for Chara dasha.
pub fn chara_hierarchy(
    birth_jd: f64,
    inputs: &RashiDashaInputs,
    max_level: u8,
    variation: &DashaVariationConfig,
) -> Result<DashaHierarchy, VedicError> {
    let level0 = chara_level0(birth_jd, inputs);
    let total = chara_total_years(inputs);
    let period_fn = |r: u8| chara_period_years(r, inputs);
    rashi_hierarchy(
        DashaSystem::Chara,
        birth_jd,
        level0,
        &period_fn,
        total,
        CHARA_DEFAULT_METHOD,
        max_level,
        variation,
    )
}

/// Snapshot for Chara dasha.
pub fn chara_snapshot(
    birth_jd: f64,
    inputs: &RashiDashaInputs,
    query_jd: f64,
    max_level: u8,
    variation: &DashaVariationConfig,
) -> DashaSnapshot {
    let level0 = chara_level0(birth_jd, inputs);
    let total = chara_total_years(inputs);
    let period_fn = |r: u8| chara_period_years(r, inputs);
    rashi_snapshot(
        DashaSystem::Chara,
        level0,
        &period_fn,
        total,
        CHARA_DEFAULT_METHOD,
        query_jd,
        max_level,
        variation,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dasha::rashi_dasha::rashi_children;
    use crate::dasha::rashi_strength::RashiDashaInputs;

    /// Helper: create inputs with grahas placed at specific rashis.
    fn make_test_inputs() -> RashiDashaInputs {
        // Lagna at 15 deg (Mesha), Sun at 40 (Vrishabha), Moon at 75 (Mithuna),
        // Mars at 195 (Tula), Mercury at 160 (Kanya), Jupiter at 250 (Dhanu),
        // Venus at 310 (Kumbha), Saturn at 100 (Karka), Rahu at 10, Ketu at 190
        let lons = [40.0, 75.0, 195.0, 160.0, 250.0, 310.0, 100.0, 10.0, 190.0];
        RashiDashaInputs::new(lons, 15.0) // Lagna in Mesha
    }

    fn make_inputs_from_rashis(overrides: &[(Graha, u8)]) -> RashiDashaInputs {
        let mut lons = [0.0, 31.0, 62.0, 93.0, 124.0, 155.0, 186.0, 217.0, 248.0];
        for &(graha, rashi) in overrides {
            lons[graha.index() as usize] = rashi as f64 * 30.0 + 1.0;
        }
        RashiDashaInputs::new(lons, 15.0)
    }

    #[test]
    fn chara_period_lord_in_own_sign() {
        // If lord is in its own sign, distance=1, period=1-1=0 → 12 years
        // Place Mars at 0 (Mesha) → lord of Mesha is Mars → distance=1, period=0 → 12y
        let mut lons = [0.0; 9];
        lons[2] = 0.0; // Mars in Mesha
        let inputs = RashiDashaInputs::new(lons, 0.0);
        assert!((chara_period_years(0, &inputs) - 12.0).abs() < 1e-10);
    }

    #[test]
    fn chara_period_basic_odd() {
        // Mesha (odd, index 0), lord=Mars(idx 2).
        // Mars at 195 deg (Tula, index 6).
        // Forward count from 0 to 6 = 7. Period = 7-1 = 6 years.
        let inputs = make_test_inputs();
        assert!((chara_period_years(0, &inputs) - 6.0).abs() < 1e-10);
    }

    #[test]
    fn chara_period_basic_even() {
        // Vrishabha (even, index 1), lord=Venus(idx 5).
        // Venus at 310 deg (Kumbha, index 10).
        // Reverse count from 1 to 10: 1→0→11→10 = 4. Period = 4-1 = 3 years.
        let inputs = make_test_inputs();
        assert!((chara_period_years(1, &inputs) - 3.0).abs() < 1e-10);
    }

    #[test]
    fn chara_dual_lords_conjunct_in_owned_sign_force_twelve_years() {
        let inputs = make_inputs_from_rashis(&[(Graha::Shani, 10), (Graha::Rahu, 10)]);
        assert!((chara_period_years(10, &inputs) - 12.0).abs() < 1e-10);

        let inputs = make_inputs_from_rashis(&[(Graha::Mangal, 7), (Graha::Ketu, 7)]);
        assert!((chara_period_years(7, &inputs) - 12.0).abs() < 1e-10);
    }

    #[test]
    fn chara_dual_lords_one_in_own_sign_counts_to_other_lord() {
        let inputs = make_inputs_from_rashis(&[(Graha::Rahu, 10), (Graha::Shani, 2)]);
        assert!((chara_period_years(10, &inputs) - 4.0).abs() < 1e-10);
    }

    #[test]
    fn chara_dual_lords_choose_more_conjoined_lord() {
        let inputs =
            make_inputs_from_rashis(&[(Graha::Shani, 3), (Graha::Rahu, 4), (Graha::Surya, 3)]);
        assert!((chara_period_years(10, &inputs) - 5.0).abs() < 1e-10);
    }

    #[test]
    fn chara_dual_lords_modality_tiebreak_uses_dual_over_fixed() {
        let inputs = make_inputs_from_rashis(&[(Graha::Shani, 1), (Graha::Rahu, 2)]);
        assert!((chara_period_years(10, &inputs) - 4.0).abs() < 1e-10);
    }

    #[test]
    fn chara_dual_lords_same_modality_tiebreak_uses_higher_years() {
        let inputs = make_inputs_from_rashis(&[(Graha::Shani, 1), (Graha::Rahu, 4)]);
        assert!((chara_period_years(10, &inputs) - 6.0).abs() < 1e-10);
    }

    #[test]
    fn chara_dual_lords_exaltation_overrides_higher_years_and_adds_one() {
        let inputs = make_inputs_from_rashis(&[(Graha::Shani, 6), (Graha::Rahu, 1)]);
        assert!((chara_period_years(10, &inputs) - 9.0).abs() < 1e-10);
    }

    #[test]
    fn chara_period_debilitated_lord_loses_one_year() {
        let inputs = make_inputs_from_rashis(&[(Graha::Mangal, 3)]);
        assert!((chara_period_years(0, &inputs) - 2.0).abs() < 1e-10);
    }

    #[test]
    fn chara_level0_12_periods() {
        let inputs = make_test_inputs();
        let periods = chara_level0(2451545.0, &inputs);
        assert_eq!(periods.len(), 12);
    }

    #[test]
    fn chara_level0_starts_from_lagna() {
        let inputs = make_test_inputs();
        let periods = chara_level0(2451545.0, &inputs);
        // Lagna in Mesha (index 0)
        assert_eq!(periods[0].entity, DashaEntity::Rashi(0));
    }

    #[test]
    fn chara_level0_no_gaps() {
        let inputs = make_test_inputs();
        let periods = chara_level0(2451545.0, &inputs);
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
    fn chara_children_start_from_parent_and_wrap() {
        let inputs = make_test_inputs();
        let parent = DashaPeriod {
            entity: DashaEntity::Rashi(1),
            start_jd: 2451545.0,
            end_jd: 2451545.0 + 360.0,
            level: DashaLevel::Mahadasha,
            order: 1,
            parent_idx: 0,
        };

        let total = chara_total_years(&inputs);
        let children = rashi_children(
            &parent,
            &|r| chara_period_years(r, &inputs),
            total,
            CHARA_DEFAULT_METHOD,
            CHARA_DEFAULT_METHOD,
        );

        assert_eq!(children.len(), 12);
        assert_eq!(children[0].entity, DashaEntity::Rashi(1));
        assert_eq!(children[1].entity, DashaEntity::Rashi(0));
        assert_eq!(children[2].entity, DashaEntity::Rashi(11));
        assert_eq!(children[11].entity, DashaEntity::Rashi(2));
    }

    #[test]
    fn chara_hierarchy_depth_2() {
        let inputs = make_test_inputs();
        let var = DashaVariationConfig::default();
        let h = chara_hierarchy(2451545.0, &inputs, 2, &var).unwrap();
        assert_eq!(h.levels.len(), 3);
        assert_eq!(h.levels[0].len(), 12);
        assert_eq!(h.levels[1].len(), 144);
    }

    #[test]
    fn chara_snapshot_matches() {
        let inputs = make_test_inputs();
        let var = DashaVariationConfig::default();
        let birth_jd = 2451545.0;
        let query_jd = birth_jd + 1000.0;

        let h = chara_hierarchy(birth_jd, &inputs, 2, &var).unwrap();
        let snap = chara_snapshot(birth_jd, &inputs, query_jd, 2, &var);

        assert_eq!(snap.periods.len(), 3);
        for (level, sp) in snap.periods.iter().enumerate() {
            let active = h.levels[level]
                .iter()
                .find(|p| p.start_jd <= query_jd && query_jd < p.end_jd)
                .expect("should find active");
            assert_eq!(sp.entity, active.entity);
        }
    }
}
