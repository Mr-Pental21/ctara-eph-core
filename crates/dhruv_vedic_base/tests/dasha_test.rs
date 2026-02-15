//! Integration tests for dasha pure-math calculations.
//!
//! These tests verify the pure-math dasha engine without requiring kernel files.

use dhruv_vedic_base::Graha;
use dhruv_vedic_base::dasha::{
    DAYS_PER_YEAR, DashaEntity, DashaLevel, DashaSystem, DashaVariationConfig, MAX_DASHA_LEVEL,
    nakshatra_config_for_system, nakshatra_hierarchy, nakshatra_level0, nakshatra_snapshot,
    snapshot_from_hierarchy, vimshottari_config, yogini_config, yogini_hierarchy, yogini_level0,
    yogini_snapshot,
};

/// Moon at 0° Aries (Ashwini nakshatra, index 0) → Ketu mahadasha, full 7y.
#[test]
fn vimshottari_moon_at_zero() {
    let cfg = vimshottari_config();
    let birth_jd = 2451545.0; // J2000
    let moon_lon = 0.0; // 0° = Ashwini, pad 1, start of nakshatra

    let level0 = nakshatra_level0(birth_jd, moon_lon, &cfg);
    assert_eq!(level0.len(), 9);

    // First period should be Ketu (Ashwini → Ketu in Vimshottari)
    assert_eq!(level0[0].entity, DashaEntity::Graha(Graha::Ketu));
    assert_eq!(level0[0].level, DashaLevel::Mahadasha);
    assert_eq!(level0[0].order, 1);

    // At 0°, the Moon is at the very start of Ashwini, so balance = full period
    let ketu_period_days = 7.0 * 365.25;
    let actual_duration = level0[0].duration_days();
    assert!(
        (actual_duration - ketu_period_days).abs() < 0.01,
        "Ketu mahadasha should be full 7y ({ketu_period_days} days), got {actual_duration}"
    );

    // Start JD should be birth JD
    assert!((level0[0].start_jd - birth_jd).abs() < 1e-10);

    // Second period should be Shukra (next in Vimshottari sequence)
    assert_eq!(level0[1].entity, DashaEntity::Graha(Graha::Shukra));
    let shukra_days = 20.0 * 365.25;
    let actual_shukra = level0[1].duration_days();
    assert!(
        (actual_shukra - shukra_days).abs() < 0.01,
        "Shukra mahadasha should be full 20y, got {actual_shukra}"
    );
}

/// Moon at 40° (Rohini nakshatra, index 3) → Chandra mahadasha with partial balance.
#[test]
fn vimshottari_moon_at_40() {
    let cfg = vimshottari_config();
    let birth_jd = 2451545.0;
    let moon_lon = 40.0;

    let level0 = nakshatra_level0(birth_jd, moon_lon, &cfg);
    assert_eq!(level0.len(), 9);

    // Nakshatra index = floor(40 / (360/27)) = floor(40/13.3333) = floor(3.0) = 3
    // Nakshatra 3 = Rohini → Chandra in Vimshottari
    assert_eq!(level0[0].entity, DashaEntity::Graha(Graha::Chandra));

    // elapsed_fraction = (40 % 13.3333) / 13.3333 = 0 / 13.3333 = 0.0
    // (Actually 40.0 / 13.3333 = 3.0 exactly, so fraction = 0.0, balance = full 10y)
    // Since Moon is at exactly the start of Rohini, balance should be full period
    let chandra_days = 10.0 * 365.25;
    let actual = level0[0].duration_days();
    assert!(
        (actual - chandra_days).abs() < 0.5,
        "Chandra balance should be close to full {chandra_days}, got {actual}"
    );

    // Next should be Mangal (Chandra→Mangal in Vimshottari sequence)
    assert_eq!(level0[1].entity, DashaEntity::Graha(Graha::Mangal));
}

/// When Moon is at 0° (start of nakshatra), total span should be exactly 120 years.
#[test]
fn vimshottari_total_span_120y_at_zero() {
    let cfg = vimshottari_config();
    let birth_jd = 2451545.0;
    let moon_lon = 0.0; // At start of nakshatra, balance = full period

    let level0 = nakshatra_level0(birth_jd, moon_lon, &cfg);
    let total_span = level0.last().unwrap().end_jd - level0.first().unwrap().start_jd;
    let expected = 120.0 * 365.25;

    assert!(
        (total_span - expected).abs() < 1e-6,
        "Total span should be exactly {expected} when balance is full, got {total_span}"
    );
}

/// When Moon is mid-nakshatra, total span should be < 120 years (partial first period).
#[test]
fn vimshottari_total_span_partial_balance() {
    let cfg = vimshottari_config();
    let birth_jd = 2451545.0;
    let moon_lon = 123.456; // Mid-nakshatra

    let level0 = nakshatra_level0(birth_jd, moon_lon, &cfg);
    let total_span = level0.last().unwrap().end_jd - level0.first().unwrap().start_jd;
    let max_span = 120.0 * 365.25;

    assert!(total_span > 0.0, "Total span should be positive");
    assert!(
        total_span <= max_span,
        "Total span should be <= {max_span}, got {total_span}"
    );
}

/// Full hierarchy at depth 2 should produce correct level counts.
#[test]
fn vimshottari_hierarchy_depth_2() {
    let cfg = vimshottari_config();
    let birth_jd = 2451545.0;
    let moon_lon = 100.0;
    let variation = DashaVariationConfig::default();

    let result = nakshatra_hierarchy(birth_jd, moon_lon, &cfg, 2, &variation);
    assert!(result.is_ok());

    let hierarchy = result.unwrap();
    assert_eq!(hierarchy.system, DashaSystem::Vimshottari);
    assert_eq!(hierarchy.levels.len(), 3);
    assert_eq!(hierarchy.levels[0].len(), 9); // 9 mahadashas
    assert_eq!(hierarchy.levels[1].len(), 81); // 9*9 antardashas
    assert_eq!(hierarchy.levels[2].len(), 729); // 81*9 pratyantardashas
}

/// Hierarchy at depth 0 should produce only mahadashas.
#[test]
fn vimshottari_hierarchy_depth_0() {
    let cfg = vimshottari_config();
    let birth_jd = 2451545.0;
    let moon_lon = 50.0;
    let variation = DashaVariationConfig::default();

    let result = nakshatra_hierarchy(birth_jd, moon_lon, &cfg, 0, &variation);
    assert!(result.is_ok());

    let hierarchy = result.unwrap();
    assert_eq!(hierarchy.levels.len(), 1);
    assert_eq!(hierarchy.levels[0].len(), 9);
}

/// Snapshot should match hierarchy lookup.
#[test]
fn snapshot_matches_hierarchy() {
    let cfg = vimshottari_config();
    let birth_jd = 2451545.0;
    let moon_lon = 75.0;
    let variation = DashaVariationConfig::default();
    let query_jd = birth_jd + 10_000.0; // ~27 years after birth

    let hierarchy = nakshatra_hierarchy(birth_jd, moon_lon, &cfg, 2, &variation).unwrap();
    let from_hierarchy = snapshot_from_hierarchy(&hierarchy, query_jd);
    let direct = nakshatra_snapshot(birth_jd, moon_lon, &cfg, query_jd, 2, &variation);

    assert_eq!(from_hierarchy.periods.len(), direct.periods.len());

    for (i, (h_period, d_period)) in from_hierarchy
        .periods
        .iter()
        .zip(direct.periods.iter())
        .enumerate()
    {
        assert_eq!(
            h_period.entity, d_period.entity,
            "Level {i} entity mismatch"
        );
        assert!(
            (h_period.start_jd - d_period.start_jd).abs() < 1e-10,
            "Level {i} start_jd mismatch"
        );
        assert!(
            (h_period.end_jd - d_period.end_jd).abs() < 1e-10,
            "Level {i} end_jd mismatch"
        );
    }
}

/// Last child end should snap to parent end (no drift).
#[test]
fn last_child_snaps_to_parent() {
    let cfg = vimshottari_config();
    let birth_jd = 2451545.0;
    let moon_lon = 200.0;
    let variation = DashaVariationConfig::default();

    let hierarchy = nakshatra_hierarchy(birth_jd, moon_lon, &cfg, 1, &variation).unwrap();

    // For each mahadasha, check that the last antardasha ends exactly at parent end
    for (parent_idx, parent) in hierarchy.levels[0].iter().enumerate() {
        let children: Vec<_> = hierarchy.levels[1]
            .iter()
            .filter(|c| c.parent_idx == parent_idx as u32)
            .collect();
        assert_eq!(
            children.len(),
            9,
            "Each mahadasha should have 9 antardashas"
        );

        let last_child = children.last().unwrap();
        assert!(
            (last_child.end_jd - parent.end_jd).abs() < 1e-10,
            "Last antardasha end ({}) should snap to mahadasha end ({})",
            last_child.end_jd,
            parent.end_jd
        );
    }
}

/// Hierarchy at depth 4 should succeed (max level).
#[test]
fn vimshottari_hierarchy_depth_4() {
    let cfg = vimshottari_config();
    let birth_jd = 2451545.0;
    let moon_lon = 150.0;
    let variation = DashaVariationConfig::default();

    let result = nakshatra_hierarchy(birth_jd, moon_lon, &cfg, MAX_DASHA_LEVEL, &variation);
    assert!(result.is_ok());

    let hierarchy = result.unwrap();
    assert_eq!(hierarchy.levels.len(), 5);
    assert_eq!(hierarchy.levels[0].len(), 9);
    assert_eq!(hierarchy.levels[1].len(), 81);
    assert_eq!(hierarchy.levels[2].len(), 729);
    assert_eq!(hierarchy.levels[3].len(), 6561);
    assert_eq!(hierarchy.levels[4].len(), 59049);
}

/// Contiguity: all levels should have no gaps between adjacent periods.
#[test]
fn all_levels_contiguous() {
    let cfg = vimshottari_config();
    let birth_jd = 2451545.0;
    let moon_lon = 300.0;
    let variation = DashaVariationConfig::default();

    let hierarchy = nakshatra_hierarchy(birth_jd, moon_lon, &cfg, 2, &variation).unwrap();

    for (lvl, level) in hierarchy.levels.iter().enumerate() {
        // Check siblings within same parent are contiguous
        let mut i = 0;
        while i < level.len() {
            // Find range of siblings with same parent_idx
            let parent = level[i].parent_idx;
            let mut j = i + 1;
            while j < level.len() && level[j].parent_idx == parent {
                let prev_end = level[j - 1].end_jd;
                let curr_start = level[j].start_jd;
                assert!(
                    (prev_end - curr_start).abs() < 1e-10,
                    "Level {lvl}, siblings {}/{}: gap between {prev_end} and {curr_start}",
                    j - 1,
                    j
                );
                j += 1;
            }
            i = j;
        }
    }
}

// ==========================================================================
// All 10 Nakshatra Systems: generic hierarchy + contiguity + snapshot tests
// ==========================================================================

/// Helper: run standard hierarchy tests for any nakshatra dasha system.
fn verify_nakshatra_system(
    system: DashaSystem,
    expected_total_years: f64,
    expected_grahas: usize,
    cycle_count: usize,
) {
    let cfg = nakshatra_config_for_system(system).unwrap();
    let birth_jd = 2451545.0;
    let moon_lon = 123.456;
    let variation = DashaVariationConfig::default();

    // Level 0: correct count of mahadashas
    let level0 = nakshatra_level0(birth_jd, moon_lon, &cfg);
    let expected_count = expected_grahas * cycle_count;
    assert_eq!(
        level0.len(),
        expected_count,
        "{:?}: expected {} mahadashas, got {}",
        system,
        expected_count,
        level0.len()
    );

    // Adjacent periods are contiguous
    for i in 1..level0.len() {
        assert!(
            (level0[i].start_jd - level0[i - 1].end_jd).abs() < 1e-10,
            "{:?}: gap between mahadashas {} and {}",
            system,
            i - 1,
            i
        );
    }

    // Total span <= full cycle years * cycle_count (partial balance reduces from max)
    let total_days = level0.last().unwrap().end_jd - level0.first().unwrap().start_jd;
    let max_days = expected_total_years * cycle_count as f64 * DAYS_PER_YEAR;
    assert!(
        total_days > 0.0 && total_days <= max_days + 1.0,
        "{:?}: total {total_days} days exceeds max {max_days}",
        system
    );

    // At nakshatra start, first period balance = full entry period
    // For most systems: entry_period = graha_period, so total = full cycle * cycle_count
    // For Shashtihayani: entry_period = graha_period / nak_count, so total < full
    let nak_start = 5.0 * (360.0 / 27.0); // Ardra start
    let level0_full = nakshatra_level0(birth_jd, nak_start, &cfg);
    let total_full_days: f64 = level0_full.iter().map(|p| p.duration_days()).sum();
    if !cfg.divide_period_by_nakshatra_count {
        let expected_full_years = expected_total_years * cycle_count as f64;
        let total_full_years = total_full_days / DAYS_PER_YEAR;
        assert!(
            (total_full_years - expected_full_years).abs() < 0.01,
            "{:?}: at nak start, total should be {}y, got {}y",
            system,
            expected_full_years,
            total_full_years
        );
    } else {
        // Shashtihayani: just verify it's positive and bounded
        assert!(
            total_full_days > 0.0,
            "{:?}: total should be positive",
            system
        );
    }

    // Hierarchy depth 1 (depth 2 can exceed MAX_PERIODS_PER_LEVEL for multi-cycle systems)
    let h = nakshatra_hierarchy(birth_jd, moon_lon, &cfg, 1, &variation).unwrap();
    assert_eq!(h.system, system);
    assert_eq!(h.levels.len(), 2);
    assert_eq!(h.levels[0].len(), expected_count);
    assert_eq!(h.levels[1].len(), expected_count * expected_grahas);

    // Snapshot matches hierarchy
    let query_jd = birth_jd + 1000.0;
    let snap = nakshatra_snapshot(birth_jd, moon_lon, &cfg, query_jd, 1, &variation);
    let from_h = snapshot_from_hierarchy(&h, query_jd);
    assert_eq!(snap.periods.len(), from_h.periods.len());
    for (i, (s, h_p)) in snap.periods.iter().zip(from_h.periods.iter()).enumerate() {
        assert_eq!(
            s.entity, h_p.entity,
            "{:?}: level {i} entity mismatch",
            system
        );
    }
}

#[test]
fn ashtottari_system() {
    verify_nakshatra_system(DashaSystem::Ashtottari, 108.0, 8, 1);
}

#[test]
fn shodsottari_system() {
    verify_nakshatra_system(DashaSystem::Shodsottari, 116.0, 8, 1);
}

#[test]
fn dwadashottari_system() {
    verify_nakshatra_system(DashaSystem::Dwadashottari, 112.0, 8, 1);
}

#[test]
fn panchottari_system() {
    verify_nakshatra_system(DashaSystem::Panchottari, 105.0, 7, 1);
}

#[test]
fn shatabdika_system() {
    verify_nakshatra_system(DashaSystem::Shatabdika, 100.0, 7, 1);
}

#[test]
fn chaturashiti_system() {
    verify_nakshatra_system(DashaSystem::Chaturashiti, 84.0, 7, 2);
}

#[test]
fn dwisaptati_system() {
    verify_nakshatra_system(DashaSystem::DwisaptatiSama, 72.0, 8, 2);
}

#[test]
fn shashtihayani_system() {
    verify_nakshatra_system(DashaSystem::Shashtihayani, 60.0, 8, 2);
}

#[test]
fn shat_trimsha_system() {
    verify_nakshatra_system(DashaSystem::ShatTrimshaSama, 36.0, 8, 3);
}

// ==========================================================================
// Yogini Dasha Tests
// ==========================================================================

/// Yogini: 8 mahadashas, 36y total at nakshatra start.
#[test]
fn yogini_level0_valid() {
    let cfg = yogini_config();
    let birth_jd = 2451545.0;
    let ardra_start = 5.0 * (360.0 / 27.0);
    let level0 = yogini_level0(birth_jd, ardra_start, &cfg);

    assert_eq!(level0.len(), 8);
    assert_eq!(level0[0].entity, DashaEntity::Yogini(0)); // Mangala

    let total_days: f64 = level0.iter().map(|p| p.duration_days()).sum();
    let total_years = total_days / DAYS_PER_YEAR;
    assert!(
        (total_years - 36.0).abs() < 1e-6,
        "Yogini total should be 36y at nak start, got {total_years}"
    );
}

/// Yogini: contiguous periods.
#[test]
fn yogini_contiguous() {
    let cfg = yogini_config();
    let level0 = yogini_level0(2451545.0, 200.0, &cfg);
    for i in 1..level0.len() {
        assert!(
            (level0[i].start_jd - level0[i - 1].end_jd).abs() < 1e-10,
            "Yogini: gap between periods {} and {}",
            i - 1,
            i
        );
    }
}

/// Yogini: hierarchy depth 2 gives 8/64/512.
#[test]
fn yogini_hierarchy_depth_2() {
    let cfg = yogini_config();
    let var = DashaVariationConfig::default();
    let h = yogini_hierarchy(2451545.0, 100.0, &cfg, 2, &var).unwrap();

    assert_eq!(h.system, DashaSystem::Yogini);
    assert_eq!(h.levels.len(), 3);
    assert_eq!(h.levels[0].len(), 8);
    assert_eq!(h.levels[1].len(), 64);
    assert_eq!(h.levels[2].len(), 512);
}

/// Yogini: snapshot matches hierarchy.
#[test]
fn yogini_snapshot_matches() {
    let cfg = yogini_config();
    let var = DashaVariationConfig::default();
    let birth_jd = 2451545.0;
    let moon = 150.0;
    let query_jd = birth_jd + 2000.0;

    let h = yogini_hierarchy(birth_jd, moon, &cfg, 2, &var).unwrap();
    let snap = yogini_snapshot(birth_jd, moon, &cfg, query_jd, 2, &var);

    assert_eq!(snap.periods.len(), 3);
    for (level, snap_period) in snap.periods.iter().enumerate() {
        let active_in_h = h.levels[level]
            .iter()
            .find(|p| p.start_jd <= query_jd && query_jd < p.end_jd)
            .expect("should find active period");
        assert_eq!(snap_period.entity, active_in_h.entity);
    }
}
