//! Chara (Jaimini) dasha — rashi-based, variable periods.
//!
//! Period = distance from rashi to its lord (in signs).
//! Odd signs count forward, even signs count reverse.
//! If distance = 0 (lord in own sign), period = 12 years.
//! Special handling for Scorpio co-lordship (Mars/Ketu).
//!
//! Starting rashi: lagna rashi. Direction: odd=forward, even=reverse.
//! Sub-period method: EqualFromNext (÷12).

use super::balance::rashi_birth_balance;
use super::rashi_dasha::{rashi_hierarchy, rashi_snapshot};
use super::rashi_strength::RashiDashaInputs;
use super::rashi_util::is_odd_sign;
use super::types::{
    DAYS_PER_YEAR, DashaEntity, DashaHierarchy, DashaLevel, DashaPeriod, DashaSnapshot, DashaSystem,
};
use super::variation::{DashaVariationConfig, SubPeriodMethod};
use crate::error::VedicError;

/// Default sub-period method for Chara dasha.
pub const CHARA_DEFAULT_METHOD: SubPeriodMethod = SubPeriodMethod::EqualFromNext;

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
/// Scorpio special: Mars is the lord. If Ketu is in Scorpio, use Ketu's position instead.
pub fn chara_period_years(rashi_index: u8, inputs: &RashiDashaInputs) -> f64 {
    let r = rashi_index % 12;
    let lord_rashi = effective_lord_rashi(r, inputs);

    let distance = if is_odd_sign(r) {
        super::rashi_util::count_signs_forward(r, lord_rashi)
    } else {
        super::rashi_util::count_signs_reverse(r, lord_rashi)
    };

    let period = distance - 1;
    if period == 0 { 12.0 } else { period as f64 }
}

/// Get the effective lord's rashi, handling Scorpio's dual lordship.
///
/// For Scorpio (index 7): use Mars normally, but if Ketu is in Scorpio, use Ketu's rashi.
/// For Aquarius (index 10): use Saturn normally, but if Rahu is in Aquarius, use Rahu's rashi.
fn effective_lord_rashi(rashi_index: u8, inputs: &RashiDashaInputs) -> u8 {
    // For Scorpio (7) and Aquarius (10), standard lordship applies (Mars/Saturn).
    // Co-lordship with Ketu/Rahu is a variant some texts use but the standard
    // BPHS Chara dasha uses the primary lord only.
    inputs.lord_rashi(rashi_index)
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
    use crate::dasha::rashi_strength::RashiDashaInputs;

    /// Helper: create inputs with grahas placed at specific rashis.
    fn make_test_inputs() -> RashiDashaInputs {
        // Lagna at 15 deg (Mesha), Sun at 40 (Vrishabha), Moon at 75 (Mithuna),
        // Mars at 195 (Tula), Mercury at 160 (Kanya), Jupiter at 250 (Dhanu),
        // Venus at 310 (Kumbha), Saturn at 100 (Karka), Rahu at 10, Ketu at 190
        let lons = [40.0, 75.0, 195.0, 160.0, 250.0, 310.0, 100.0, 10.0, 190.0];
        RashiDashaInputs::new(lons, 15.0) // Lagna in Mesha
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
