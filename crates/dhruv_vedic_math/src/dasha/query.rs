//! Period lookup: binary search for active periods at a given JD.
//!
//! Interval convention: [start_jd, end_jd) — start is inclusive, end is exclusive.
//! Adjacent periods: period[n].end_jd == period[n+1].start_jd (no gaps, no overlaps).

use super::types::{DashaHierarchy, DashaPeriod, DashaSnapshot};

/// Binary search for the active period at query_jd within a sorted level.
///
/// Returns the index into the level array. Uses [start_jd, end_jd) convention.
/// Returns None if query_jd is outside all periods.
pub fn find_active_period(level: &[DashaPeriod], query_jd: f64) -> Option<usize> {
    if level.is_empty() {
        return None;
    }

    // Quick bounds check
    if query_jd < level[0].start_jd {
        return None;
    }
    let last = level.len() - 1;
    if query_jd >= level[last].end_jd {
        return None;
    }

    // Binary search: find rightmost period where start_jd <= query_jd
    let mut lo = 0usize;
    let mut hi = last;

    while lo < hi {
        let mid = lo + (hi - lo + 1).div_ceil(2);
        if level[mid].start_jd <= query_jd {
            lo = mid;
        } else {
            hi = mid - 1;
        }
    }

    // Verify the period contains query_jd
    if level[lo].start_jd <= query_jd && query_jd < level[lo].end_jd {
        Some(lo)
    } else {
        None
    }
}

/// Build a snapshot from a pre-computed hierarchy.
pub fn snapshot_from_hierarchy(hierarchy: &DashaHierarchy, query_jd: f64) -> DashaSnapshot {
    let mut periods = Vec::with_capacity(hierarchy.levels.len());

    for level in &hierarchy.levels {
        match find_active_period(level, query_jd) {
            Some(idx) => periods.push(level[idx]),
            None => break,
        }
    }

    DashaSnapshot {
        system: hierarchy.system,
        query_jd,
        periods,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dasha::types::{DashaEntity, DashaLevel};
    use crate::graha::Graha;

    fn sample_level() -> Vec<DashaPeriod> {
        vec![
            DashaPeriod {
                entity: DashaEntity::Graha(Graha::Ketu),
                start_jd: 100.0,
                end_jd: 200.0,
                level: DashaLevel::Mahadasha,
                order: 1,
                parent_idx: 0,
            },
            DashaPeriod {
                entity: DashaEntity::Graha(Graha::Shukra),
                start_jd: 200.0,
                end_jd: 400.0,
                level: DashaLevel::Mahadasha,
                order: 2,
                parent_idx: 0,
            },
            DashaPeriod {
                entity: DashaEntity::Graha(Graha::Surya),
                start_jd: 400.0,
                end_jd: 500.0,
                level: DashaLevel::Mahadasha,
                order: 3,
                parent_idx: 0,
            },
        ]
    }

    #[test]
    fn find_first_period() {
        let level = sample_level();
        assert_eq!(find_active_period(&level, 100.0), Some(0));
        assert_eq!(find_active_period(&level, 150.0), Some(0));
    }

    #[test]
    fn find_second_period() {
        let level = sample_level();
        assert_eq!(find_active_period(&level, 200.0), Some(1));
        assert_eq!(find_active_period(&level, 300.0), Some(1));
    }

    #[test]
    fn find_last_period() {
        let level = sample_level();
        assert_eq!(find_active_period(&level, 400.0), Some(2));
        assert_eq!(find_active_period(&level, 499.999), Some(2));
    }

    #[test]
    fn before_all_periods() {
        let level = sample_level();
        assert_eq!(find_active_period(&level, 99.0), None);
    }

    #[test]
    fn at_exact_end() {
        let level = sample_level();
        // end_jd is exclusive, so 500.0 is outside
        assert_eq!(find_active_period(&level, 500.0), None);
    }

    #[test]
    fn empty_level() {
        assert_eq!(find_active_period(&[], 100.0), None);
    }
}
