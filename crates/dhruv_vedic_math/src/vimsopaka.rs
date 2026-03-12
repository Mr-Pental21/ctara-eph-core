//! Vimsopaka Bala (20-point varga dignity strength) computation.
//!
//! Computes weighted dignity scores across different varga groupings:
//! - Shadvarga (6 vargas)
//! - Saptavarga (7 vargas)
//! - Dashavarga (10 vargas)
//! - Shodasavarga (16 vargas)
//!
//! **Target: navagraha (9).** Rahu/Ketu use configurable `NodeDignityPolicy`.
//!
//! Clean-room implementation from BPHS.

use crate::amsha::{Amsha, amsha_longitude};
use crate::error::VedicError;
use crate::graha::{ALL_GRAHAS, Graha};
use crate::graha_relationships::{
    Dignity, NodeDignityPolicy, dignity_in_rashi_with_positions, node_dignity_in_rashi,
};
use crate::util::normalize_360;

// ---------------------------------------------------------------------------
// 3a. Varga Groupings & Weights
// ---------------------------------------------------------------------------

/// A varga with its weight in a grouping.
#[derive(Debug, Clone, Copy)]
pub struct VargaWeight {
    pub amsha: Amsha,
    pub weight: f64,
}

/// Shadvarga: 6 vargas, weights sum to 20.
pub const SHADVARGA: [VargaWeight; 6] = [
    VargaWeight {
        amsha: Amsha::D1,
        weight: 6.0,
    },
    VargaWeight {
        amsha: Amsha::D2,
        weight: 2.0,
    },
    VargaWeight {
        amsha: Amsha::D3,
        weight: 4.0,
    },
    VargaWeight {
        amsha: Amsha::D9,
        weight: 5.0,
    },
    VargaWeight {
        amsha: Amsha::D12,
        weight: 2.0,
    },
    VargaWeight {
        amsha: Amsha::D30,
        weight: 1.0,
    },
];

/// Saptavarga: 7 vargas, weights sum to 20.
pub const SAPTAVARGA: [VargaWeight; 7] = [
    VargaWeight {
        amsha: Amsha::D1,
        weight: 5.0,
    },
    VargaWeight {
        amsha: Amsha::D2,
        weight: 2.0,
    },
    VargaWeight {
        amsha: Amsha::D3,
        weight: 3.0,
    },
    VargaWeight {
        amsha: Amsha::D7,
        weight: 2.5,
    },
    VargaWeight {
        amsha: Amsha::D9,
        weight: 4.5,
    },
    VargaWeight {
        amsha: Amsha::D12,
        weight: 2.0,
    },
    VargaWeight {
        amsha: Amsha::D30,
        weight: 1.0,
    },
];

/// Dashavarga: 10 vargas, weights sum to 20.
pub const DASHAVARGA: [VargaWeight; 10] = [
    VargaWeight {
        amsha: Amsha::D1,
        weight: 3.0,
    },
    VargaWeight {
        amsha: Amsha::D2,
        weight: 1.5,
    },
    VargaWeight {
        amsha: Amsha::D3,
        weight: 1.5,
    },
    VargaWeight {
        amsha: Amsha::D7,
        weight: 1.5,
    },
    VargaWeight {
        amsha: Amsha::D9,
        weight: 3.0,
    },
    VargaWeight {
        amsha: Amsha::D10,
        weight: 1.5,
    },
    VargaWeight {
        amsha: Amsha::D12,
        weight: 1.5,
    },
    VargaWeight {
        amsha: Amsha::D16,
        weight: 2.0,
    },
    VargaWeight {
        amsha: Amsha::D30,
        weight: 1.0,
    },
    VargaWeight {
        amsha: Amsha::D60,
        weight: 3.5,
    },
];

/// Shodasavarga: 16 vargas, weights sum to 20.
pub const SHODASAVARGA: [VargaWeight; 16] = [
    VargaWeight {
        amsha: Amsha::D1,
        weight: 3.5,
    },
    VargaWeight {
        amsha: Amsha::D2,
        weight: 1.0,
    },
    VargaWeight {
        amsha: Amsha::D3,
        weight: 1.0,
    },
    VargaWeight {
        amsha: Amsha::D4,
        weight: 0.5,
    },
    VargaWeight {
        amsha: Amsha::D7,
        weight: 0.5,
    },
    VargaWeight {
        amsha: Amsha::D9,
        weight: 3.0,
    },
    VargaWeight {
        amsha: Amsha::D10,
        weight: 0.5,
    },
    VargaWeight {
        amsha: Amsha::D12,
        weight: 0.5,
    },
    VargaWeight {
        amsha: Amsha::D16,
        weight: 2.0,
    },
    VargaWeight {
        amsha: Amsha::D20,
        weight: 0.5,
    },
    VargaWeight {
        amsha: Amsha::D24,
        weight: 0.5,
    },
    VargaWeight {
        amsha: Amsha::D27,
        weight: 0.5,
    },
    VargaWeight {
        amsha: Amsha::D30,
        weight: 1.0,
    },
    VargaWeight {
        amsha: Amsha::D40,
        weight: 0.5,
    },
    VargaWeight {
        amsha: Amsha::D45,
        weight: 0.5,
    },
    VargaWeight {
        amsha: Amsha::D60,
        weight: 4.0,
    },
];

// ---------------------------------------------------------------------------
// 3b. Vimsopaka Dignity Points
// ---------------------------------------------------------------------------

/// Convert a Dignity to Vimsopaka points (0-20 scale).
pub fn vimsopaka_dignity_points(dignity: Dignity) -> f64 {
    match dignity {
        Dignity::Exalted => 20.0,
        Dignity::Moolatrikone => 18.0,
        Dignity::OwnSign => 15.0,
        Dignity::AdhiMitra => 12.0,
        Dignity::Mitra => 10.0,
        Dignity::Sama => 7.0,
        Dignity::Shatru => 5.0,
        Dignity::AdhiShatru => 3.0,
        Dignity::Debilitated => 2.0,
    }
}

// ---------------------------------------------------------------------------
// 3c. Precomputed Inputs & Low-Level Entrypoint
// ---------------------------------------------------------------------------

/// Per-varga dignity entry for a single graha.
#[derive(Debug, Clone, Copy)]
pub struct VargaDignityEntry {
    pub amsha: Amsha,
    pub dignity: Dignity,
    pub points: f64,
    pub weight: f64,
}

/// Vimsopaka Bala result for a single graha.
#[derive(Debug, Clone)]
pub struct VimsopakaBala {
    pub score: f64,
    pub entries: Vec<VargaDignityEntry>,
}

/// Low-level: compute weighted average from precomputed entries.
///
/// Grouping inferred from entries.len(): 6=Shadvarga, 7=Saptavarga, 10=Dashavarga, 16=Shodasavarga.
pub fn vimsopaka_from_entries(entries: &[VargaDignityEntry]) -> Result<f64, VedicError> {
    let expected_grouping = match entries.len() {
        6 | 7 | 10 | 16 => entries.len(),
        _ => {
            return Err(VedicError::NoConvergence(
                "vimsopaka: invalid entry count (must be 6, 7, 10, or 16)",
            ));
        }
    };

    let grouping: &[VargaWeight] = match expected_grouping {
        6 => &SHADVARGA,
        7 => &SAPTAVARGA,
        10 => &DASHAVARGA,
        16 => &SHODASAVARGA,
        _ => unreachable!(),
    };

    // Validate amsha order matches grouping
    for (i, entry) in entries.iter().enumerate() {
        if entry.amsha != grouping[i].amsha {
            return Err(VedicError::NoConvergence(
                "vimsopaka: entry amsha order doesn't match grouping",
            ));
        }
    }

    let mut weighted_sum = 0.0;
    let mut total_weight = 0.0;
    for entry in entries {
        weighted_sum += entry.points * entry.weight;
        total_weight += entry.weight;
    }

    if total_weight > 0.0 {
        Ok(weighted_sum / total_weight)
    } else {
        Ok(0.0)
    }
}

// ---------------------------------------------------------------------------
// 3c cont. Full computation
// ---------------------------------------------------------------------------

/// Compute Vimsopaka Bala for a single graha using full computation.
///
/// Computes amsha positions for each varga, determines dignity per-varga
/// using per-varga rashi positions for temporal friendship.
pub fn vimsopaka_bala(
    graha: Graha,
    sidereal_lon: f64,
    all_sidereal_lons_9: &[f64; 9],
    vargas: &[VargaWeight],
    node_policy: NodeDignityPolicy,
) -> VimsopakaBala {
    let gi = graha.index() as usize;
    let is_node = matches!(graha, Graha::Rahu | Graha::Ketu);

    let mut entries = Vec::with_capacity(vargas.len());
    let mut weighted_sum = 0.0;
    let mut total_weight = 0.0;

    for vw in vargas {
        // Compute per-varga rashi indices for all 9 grahas
        let mut varga_rashi_9 = [0u8; 9];
        for (j, &lon) in all_sidereal_lons_9.iter().enumerate() {
            let varga_lon = if vw.amsha == Amsha::D1 {
                normalize_360(lon)
            } else {
                amsha_longitude(lon, vw.amsha, None)
            };
            varga_rashi_9[j] = (normalize_360(varga_lon) / 30.0).floor() as u8;
        }

        let rashi_idx = varga_rashi_9[gi];

        // Determine dignity
        let dignity = if is_node {
            node_dignity_in_rashi(graha, rashi_idx, &varga_rashi_9, node_policy)
        } else {
            // Extract sapta-graha rashi indices for compound friendship
            let mut sapta_rashi = [0u8; 7];
            sapta_rashi.copy_from_slice(&varga_rashi_9[..7]);
            // For exaltation/debilitation check, use the varga-specific longitude
            let varga_lon = if vw.amsha == Amsha::D1 {
                normalize_360(sidereal_lon)
            } else {
                amsha_longitude(sidereal_lon, vw.amsha, None)
            };
            dignity_in_rashi_with_positions(graha, varga_lon, rashi_idx, &sapta_rashi)
        };

        let points = vimsopaka_dignity_points(dignity);

        entries.push(VargaDignityEntry {
            amsha: vw.amsha,
            dignity,
            points,
            weight: vw.weight,
        });

        weighted_sum += points * vw.weight;
        total_weight += vw.weight;
    }

    let score = if total_weight > 0.0 {
        weighted_sum / total_weight
    } else {
        0.0
    };

    VimsopakaBala { score, entries }
}

/// Compute Vimsopaka Bala for all 9 navagrahas.
pub fn all_vimsopaka_balas(
    sidereal_lons: &[f64; 9],
    vargas: &[VargaWeight],
    node_policy: NodeDignityPolicy,
) -> [VimsopakaBala; 9] {
    // Can't use array init easily with non-Copy type, so build individually
    let mut results: Vec<VimsopakaBala> = Vec::with_capacity(9);
    for g in ALL_GRAHAS {
        let i = g.index() as usize;
        results.push(vimsopaka_bala(
            g,
            sidereal_lons[i],
            sidereal_lons,
            vargas,
            node_policy,
        ));
    }
    // Convert Vec to array
    let mut arr: [VimsopakaBala; 9] = std::array::from_fn(|_| VimsopakaBala {
        score: 0.0,
        entries: Vec::new(),
    });
    for (i, v) in results.into_iter().enumerate() {
        arr[i] = v;
    }
    arr
}

// ---------------------------------------------------------------------------
// 3d. Convenience Functions
// ---------------------------------------------------------------------------

/// Shadvarga Vimsopaka for a single graha.
pub fn shadvarga_vimsopaka(
    graha: Graha,
    sid_lon: f64,
    all_lons: &[f64; 9],
    policy: NodeDignityPolicy,
) -> VimsopakaBala {
    vimsopaka_bala(graha, sid_lon, all_lons, &SHADVARGA, policy)
}

/// Saptavarga Vimsopaka for a single graha.
pub fn saptavarga_vimsopaka(
    graha: Graha,
    sid_lon: f64,
    all_lons: &[f64; 9],
    policy: NodeDignityPolicy,
) -> VimsopakaBala {
    vimsopaka_bala(graha, sid_lon, all_lons, &SAPTAVARGA, policy)
}

/// Dashavarga Vimsopaka for a single graha.
pub fn dashavarga_vimsopaka(
    graha: Graha,
    sid_lon: f64,
    all_lons: &[f64; 9],
    policy: NodeDignityPolicy,
) -> VimsopakaBala {
    vimsopaka_bala(graha, sid_lon, all_lons, &DASHAVARGA, policy)
}

/// Shodasavarga Vimsopaka for a single graha.
pub fn shodasavarga_vimsopaka(
    graha: Graha,
    sid_lon: f64,
    all_lons: &[f64; 9],
    policy: NodeDignityPolicy,
) -> VimsopakaBala {
    vimsopaka_bala(graha, sid_lon, all_lons, &SHODASAVARGA, policy)
}

/// Shadvarga Vimsopaka for all 9 navagrahas.
pub fn all_shadvarga_vimsopaka(lons: &[f64; 9], policy: NodeDignityPolicy) -> [VimsopakaBala; 9] {
    all_vimsopaka_balas(lons, &SHADVARGA, policy)
}

/// Saptavarga Vimsopaka for all 9.
pub fn all_saptavarga_vimsopaka(lons: &[f64; 9], policy: NodeDignityPolicy) -> [VimsopakaBala; 9] {
    all_vimsopaka_balas(lons, &SAPTAVARGA, policy)
}

/// Dashavarga Vimsopaka for all 9.
pub fn all_dashavarga_vimsopaka(lons: &[f64; 9], policy: NodeDignityPolicy) -> [VimsopakaBala; 9] {
    all_vimsopaka_balas(lons, &DASHAVARGA, policy)
}

/// Shodasavarga Vimsopaka for all 9.
pub fn all_shodasavarga_vimsopaka(
    lons: &[f64; 9],
    policy: NodeDignityPolicy,
) -> [VimsopakaBala; 9] {
    all_vimsopaka_balas(lons, &SHODASAVARGA, policy)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    const EPS: f64 = 1e-6;

    // --- Weight sums ---

    #[test]
    fn shadvarga_weights_sum_20() {
        let sum: f64 = SHADVARGA.iter().map(|w| w.weight).sum();
        assert!((sum - 20.0).abs() < 0.01, "shadvarga sum = {sum}");
    }

    #[test]
    fn saptavarga_weights_sum_20() {
        let sum: f64 = SAPTAVARGA.iter().map(|w| w.weight).sum();
        assert!((sum - 20.0).abs() < 0.01, "saptavarga sum = {sum}");
    }

    #[test]
    fn dashavarga_weights_sum_20() {
        let sum: f64 = DASHAVARGA.iter().map(|w| w.weight).sum();
        assert!((sum - 20.0).abs() < 0.01, "dashavarga sum = {sum}");
    }

    #[test]
    fn shodasavarga_weights_sum_20() {
        let sum: f64 = SHODASAVARGA.iter().map(|w| w.weight).sum();
        assert!((sum - 20.0).abs() < 0.01, "shodasavarga sum = {sum}");
    }

    // --- Dignity Points ---

    #[test]
    fn dignity_points_exalted() {
        assert!((vimsopaka_dignity_points(Dignity::Exalted) - 20.0).abs() < EPS);
    }

    #[test]
    fn dignity_points_debilitated() {
        assert!((vimsopaka_dignity_points(Dignity::Debilitated) - 2.0).abs() < EPS);
    }

    #[test]
    fn dignity_points_sama() {
        assert!((vimsopaka_dignity_points(Dignity::Sama) - 7.0).abs() < EPS);
    }

    // --- Score Range ---

    #[test]
    fn vimsopaka_score_in_range() {
        // Test with known positions
        let lons = [10.0, 33.0, 298.0, 165.0, 95.0, 357.0, 200.0, 120.0, 300.0];
        for g in ALL_GRAHAS {
            let i = g.index() as usize;
            let result = shadvarga_vimsopaka(g, lons[i], &lons, NodeDignityPolicy::default());
            assert!(
                result.score >= 0.0 && result.score <= 20.0,
                "{:?}: score = {}",
                g,
                result.score
            );
            assert_eq!(result.entries.len(), 6, "{:?}: entry count", g);
        }
    }

    #[test]
    fn all_four_groupings_valid() {
        let lons = [45.0, 80.0, 150.0, 210.0, 280.0, 330.0, 15.0, 100.0, 250.0];
        let policy = NodeDignityPolicy::default();

        for g in ALL_GRAHAS {
            let i = g.index() as usize;
            let shad = shadvarga_vimsopaka(g, lons[i], &lons, policy);
            let sapt = saptavarga_vimsopaka(g, lons[i], &lons, policy);
            let dash = dashavarga_vimsopaka(g, lons[i], &lons, policy);
            let shod = shodasavarga_vimsopaka(g, lons[i], &lons, policy);

            for (name, result) in [
                ("shad", &shad),
                ("sapt", &sapt),
                ("dash", &dash),
                ("shod", &shod),
            ] {
                assert!(
                    result.score >= 0.0 && result.score <= 20.0,
                    "{:?} {name}: score = {}",
                    g,
                    result.score
                );
            }
        }
    }

    // --- Rahu/Ketu ---

    #[test]
    fn rahu_sign_lord_based_valid() {
        let lons = [45.0, 80.0, 150.0, 210.0, 280.0, 330.0, 15.0, 100.0, 250.0];
        let result = shadvarga_vimsopaka(
            Graha::Rahu,
            lons[7],
            &lons,
            NodeDignityPolicy::SignLordBased,
        );
        assert!(result.score >= 0.0 && result.score <= 20.0);
    }

    #[test]
    fn rahu_always_sama_all_neutral() {
        let lons = [45.0, 80.0, 150.0, 210.0, 280.0, 330.0, 15.0, 100.0, 250.0];
        let result =
            shadvarga_vimsopaka(Graha::Rahu, lons[7], &lons, NodeDignityPolicy::AlwaysSama);
        // All entries should have Sama dignity → 7 points → score = 7.0
        assert!(
            (result.score - 7.0).abs() < EPS,
            "AlwaysSama should give 7.0, got {}",
            result.score
        );
        for entry in &result.entries {
            assert_eq!(entry.dignity, Dignity::Sama);
            assert!((entry.points - 7.0).abs() < EPS);
        }
    }

    // --- vimsopaka_from_entries ---

    #[test]
    fn from_entries_matches_full() {
        let lons = [10.0, 33.0, 298.0, 165.0, 95.0, 357.0, 200.0, 120.0, 300.0];
        let full = shadvarga_vimsopaka(Graha::Surya, lons[0], &lons, NodeDignityPolicy::default());
        let from_entries = vimsopaka_from_entries(&full.entries).unwrap();
        assert!(
            (full.score - from_entries).abs() < EPS,
            "full={}, from_entries={}",
            full.score,
            from_entries
        );
    }

    #[test]
    fn from_entries_invalid_count() {
        let entries = vec![VargaDignityEntry {
            amsha: Amsha::D1,
            dignity: Dignity::Sama,
            points: 7.0,
            weight: 1.0,
        }]; // only 1 entry
        assert!(vimsopaka_from_entries(&entries).is_err());
    }

    // --- Single = all-grahas ---

    #[test]
    fn single_equals_all_shadvarga() {
        let lons = [45.0, 80.0, 150.0, 210.0, 280.0, 330.0, 15.0, 100.0, 250.0];
        let policy = NodeDignityPolicy::default();
        let all = all_shadvarga_vimsopaka(&lons, policy);
        for g in ALL_GRAHAS {
            let i = g.index() as usize;
            let single = shadvarga_vimsopaka(g, lons[i], &lons, policy);
            assert!(
                (all[i].score - single.score).abs() < EPS,
                "{:?}: all={}, single={}",
                g,
                all[i].score,
                single.score
            );
        }
    }
}
