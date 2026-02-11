//! Ashtakavarga (benefic points) calculations.
//!
//! Implements Bhinna Ashtakavarga (BAV), Sarvashtakavarga (SAV), and
//! Sodhana (reduction) operations per BPHS standard rules.
//!
//! For each of 7 grahas (Sun through Saturn), 8 contributors (7 grahas + Lagna)
//! assign benefic points to rashis based on their relative offset.
//!
//! Mathematical invariants (totals across 12 rashis, constant for ALL charts):
//! - Sun: 48, Moon: 49, Mars: 39, Mercury: 54, Jupiter: 56, Venus: 52, Saturn: 39
//! - SAV total: 337
//!
//! Clean-room implementation from BPHS (Brihat Parashara Hora Shastra).
//! See `docs/clean_room_ashtakavarga.md`.

// ---------------------------------------------------------------------------
// Rules table (bitmask encoding)
// ---------------------------------------------------------------------------

/// Build a bitmask from 1-based offset values.
/// Bit i is set if offset i appears in the list.
const fn bits(offsets: &[u8]) -> u16 {
    let mut mask = 0u16;
    let mut i = 0;
    while i < offsets.len() {
        mask |= 1u16 << offsets[i];
        i += 1;
    }
    mask
}

/// Ashtakavarga rules: RULES[target_graha][contributor] = bitmask of favorable offsets.
///
/// target_graha: 0=Sun, 1=Moon, 2=Mars, 3=Mercury, 4=Jupiter, 5=Venus, 6=Saturn
/// contributor:  0=Sun, 1=Moon, 2=Mars, 3=Mercury, 4=Jupiter, 5=Venus, 6=Saturn, 7=Lagna
///
/// Offsets are 1-based (1=same rashi, 2=next rashi, ..., 12=previous rashi).
/// Source: BPHS standard rules.
const RULES: [[u16; 8]; 7] = [
    // Sun (total: 48)
    [
        bits(&[1, 2, 4, 7, 8, 9, 10, 11]),     // from Sun
        bits(&[3, 6, 10, 11]),                    // from Moon
        bits(&[1, 2, 4, 7, 8, 9, 10, 11]),       // from Mars
        bits(&[3, 5, 6, 9, 10, 11, 12]),          // from Mercury
        bits(&[5, 6, 9, 11]),                      // from Jupiter
        bits(&[6, 7, 12]),                         // from Venus
        bits(&[1, 2, 4, 7, 8, 9, 10, 11]),       // from Saturn
        bits(&[3, 4, 6, 10, 11, 12]),             // from Lagna
    ],
    // Moon (total: 49)
    [
        bits(&[3, 6, 7, 8, 10, 11]),             // from Sun
        bits(&[1, 3, 6, 7, 10, 11]),             // from Moon
        bits(&[2, 3, 5, 6, 9, 10, 11]),          // from Mars
        bits(&[1, 3, 4, 5, 7, 8, 10, 11]),       // from Mercury
        bits(&[1, 4, 7, 8, 10, 11, 12]),          // from Jupiter
        bits(&[3, 4, 5, 7, 9, 10, 11]),           // from Venus
        bits(&[3, 5, 6, 11]),                      // from Saturn
        bits(&[3, 6, 10, 11]),                     // from Lagna
    ],
    // Mars (total: 39)
    [
        bits(&[3, 5, 6, 10, 11]),                 // from Sun
        bits(&[3, 6, 11]),                         // from Moon
        bits(&[1, 2, 4, 7, 8, 10, 11]),           // from Mars
        bits(&[3, 5, 6, 11]),                      // from Mercury
        bits(&[6, 10, 11, 12]),                    // from Jupiter
        bits(&[6, 8, 11, 12]),                     // from Venus
        bits(&[1, 4, 7, 8, 9, 10, 11]),           // from Saturn
        bits(&[1, 3, 6, 10, 11]),                  // from Lagna
    ],
    // Mercury (total: 54)
    [
        bits(&[5, 6, 9, 11, 12]),                 // from Sun
        bits(&[2, 4, 6, 8, 10, 11]),              // from Moon
        bits(&[1, 2, 4, 7, 8, 9, 10, 11]),        // from Mars
        bits(&[1, 3, 5, 6, 9, 10, 11, 12]),       // from Mercury
        bits(&[6, 8, 11, 12]),                     // from Jupiter
        bits(&[1, 2, 3, 4, 5, 8, 9, 11]),         // from Venus
        bits(&[1, 2, 4, 7, 8, 9, 10, 11]),        // from Saturn
        bits(&[1, 2, 4, 6, 8, 10, 11]),            // from Lagna
    ],
    // Jupiter (total: 56)
    [
        bits(&[1, 2, 3, 4, 7, 8, 9, 10, 11]),    // from Sun
        bits(&[2, 5, 7, 9, 11]),                   // from Moon
        bits(&[1, 2, 4, 7, 8, 10, 11]),            // from Mars
        bits(&[1, 2, 4, 5, 6, 9, 10, 11]),         // from Mercury
        bits(&[1, 2, 3, 4, 7, 8, 10, 11]),         // from Jupiter
        bits(&[2, 5, 6, 9, 10, 11]),               // from Venus
        bits(&[3, 5, 6, 12]),                       // from Saturn
        bits(&[1, 2, 4, 5, 6, 7, 9, 10, 11]),     // from Lagna
    ],
    // Venus (total: 52)
    [
        bits(&[8, 11, 12]),                        // from Sun
        bits(&[1, 2, 3, 4, 5, 8, 9, 11, 12]),     // from Moon
        bits(&[3, 4, 6, 9, 11, 12]),               // from Mars
        bits(&[3, 5, 6, 9, 11]),                    // from Mercury
        bits(&[5, 8, 9, 10, 11]),                   // from Jupiter
        bits(&[1, 2, 3, 4, 5, 8, 9, 10, 11]),      // from Venus
        bits(&[3, 4, 5, 8, 9, 10, 11]),             // from Saturn
        bits(&[1, 2, 3, 4, 5, 8, 9, 11]),           // from Lagna
    ],
    // Saturn (total: 39)
    [
        bits(&[1, 2, 4, 7, 8, 10, 11]),           // from Sun
        bits(&[3, 6, 11]),                          // from Moon
        bits(&[3, 5, 6, 10, 11, 12]),              // from Mars
        bits(&[6, 8, 9, 10, 11, 12]),              // from Mercury
        bits(&[5, 6, 11, 12]),                      // from Jupiter
        bits(&[6, 11, 12]),                         // from Venus
        bits(&[3, 5, 6, 11]),                       // from Saturn
        bits(&[1, 3, 4, 6, 10, 11]),               // from Lagna
    ],
];

/// Expected BAV totals per graha (for validation).
pub const BAV_TOTALS: [u8; 7] = [48, 49, 39, 54, 56, 52, 39];

/// Expected SAV total (constant for all charts).
pub const SAV_TOTAL: u16 = 337;

// ---------------------------------------------------------------------------
// Bhinna Ashtakavarga (BAV)
// ---------------------------------------------------------------------------

/// Bhinna Ashtakavarga for a single graha.
#[derive(Debug, Clone, Copy)]
pub struct BhinnaAshtakavarga {
    /// Target graha index (0=Sun through 6=Saturn).
    pub graha_index: u8,
    /// Benefic points per rashi (0-based index, max 8 points each).
    pub points: [u8; 12],
}

impl BhinnaAshtakavarga {
    /// Total points across all 12 rashis.
    pub fn total(&self) -> u8 {
        self.points.iter().sum()
    }
}

/// Calculate BAV for a single graha.
///
/// Arguments:
/// - `graha_index`: target graha (0=Sun through 6=Saturn)
/// - `graha_rashis`: 0-based rashi index for each graha (7 entries: Sun..Saturn)
/// - `lagna_rashi`: 0-based rashi index of the Ascendant
pub fn calculate_bav(
    graha_index: u8,
    graha_rashis: &[u8; 7],
    lagna_rashi: u8,
) -> BhinnaAshtakavarga {
    let rules = &RULES[graha_index as usize];
    let mut points = [0u8; 12];

    for rashi in 0u8..12 {
        for contributor in 0u8..8 {
            let contributor_rashi = if contributor < 7 {
                graha_rashis[contributor as usize]
            } else {
                lagna_rashi
            };

            // 1-based offset from contributor to this rashi
            let offset = ((rashi as i16 - contributor_rashi as i16 + 12) % 12 + 1) as u8;

            // Check if this offset gives a point
            if (rules[contributor as usize] >> offset) & 1 == 1 {
                points[rashi as usize] += 1;
            }
        }
    }

    BhinnaAshtakavarga {
        graha_index,
        points,
    }
}

/// Calculate BAV for all 7 grahas.
pub fn calculate_all_bav(
    graha_rashis: &[u8; 7],
    lagna_rashi: u8,
) -> [BhinnaAshtakavarga; 7] {
    let mut bavs = [BhinnaAshtakavarga {
        graha_index: 0,
        points: [0; 12],
    }; 7];
    for i in 0..7u8 {
        bavs[i as usize] = calculate_bav(i, graha_rashis, lagna_rashi);
    }
    bavs
}

// ---------------------------------------------------------------------------
// Sarvashtakavarga (SAV)
// ---------------------------------------------------------------------------

/// Sarvashtakavarga: combined points + sodhana reductions.
#[derive(Debug, Clone, Copy)]
pub struct SarvaAshtakavarga {
    /// SAV total per rashi (sum of all 7 BAVs).
    pub total_points: [u8; 12],
    /// After Trikona Sodhana (trine reduction).
    pub after_trikona: [u8; 12],
    /// After Ekadhipatya Sodhana (same-lord reduction).
    pub after_ekadhipatya: [u8; 12],
}

/// Calculate SAV from all 7 BAVs.
pub fn calculate_sav(bavs: &[BhinnaAshtakavarga; 7]) -> SarvaAshtakavarga {
    // Sum BAV points per rashi
    let mut total_points = [0u8; 12];
    for bav in bavs {
        for i in 0..12 {
            total_points[i] += bav.points[i];
        }
    }

    // Apply sodhana
    let after_trikona = trikona_sodhana(&total_points);
    let after_ekadhipatya = ekadhipatya_sodhana(&after_trikona);

    SarvaAshtakavarga {
        total_points,
        after_trikona,
        after_ekadhipatya,
    }
}

// ---------------------------------------------------------------------------
// Sodhana (reductions)
// ---------------------------------------------------------------------------

/// Trikona groups (0-based rashi indices):
/// - Fire:  Mesha(0), Simha(4), Dhanu(8)
/// - Earth: Vrishabha(1), Kanya(5), Makara(9)
/// - Air:   Mithuna(2), Tula(6), Kumbha(10)
/// - Water: Karka(3), Vrischika(7), Meena(11)
const TRIKONA_GROUPS: [[usize; 3]; 4] = [
    [0, 4, 8],
    [1, 5, 9],
    [2, 6, 10],
    [3, 7, 11],
];

/// Apply Trikona Sodhana: subtract minimum from each trikona triangle.
pub fn trikona_sodhana(totals: &[u8; 12]) -> [u8; 12] {
    let mut result = *totals;

    for group in &TRIKONA_GROUPS {
        let min_val = group.iter().map(|&i| result[i]).min().unwrap_or(0);
        for &i in group {
            result[i] -= min_val;
        }
    }

    result
}

/// Same-lord pairs for Ekadhipatya Sodhana (0-based rashi indices):
/// - Mercury rules Mithuna(2) and Kanya(5)
/// - Jupiter rules Dhanu(8) and Meena(11)
///
/// Mars (Mesha/Vrischika), Venus (Vrishabha/Tula), Saturn (Makara/Kumbha)
/// pairs are in different trikona groups and already reduced. Sun and Moon
/// each rule a single sign, so no pairs.
const EKADHIPATYA_PAIRS: [[usize; 2]; 2] = [
    [2, 5],   // Mercury: Mithuna(2), Kanya(5)
    [8, 11],  // Jupiter: Dhanu(8), Meena(11)
];

/// Apply Ekadhipatya Sodhana: subtract minimum from same-lord pairs.
///
/// Operates on the result of trikona_sodhana. For rashis not in a pair,
/// the trikona value is preserved.
pub fn ekadhipatya_sodhana(after_trikona: &[u8; 12]) -> [u8; 12] {
    let mut result = *after_trikona;

    for pair in &EKADHIPATYA_PAIRS {
        let min_val = result[pair[0]].min(result[pair[1]]);
        result[pair[0]] -= min_val;
        result[pair[1]] -= min_val;
    }

    result
}

// ---------------------------------------------------------------------------
// Combined result
// ---------------------------------------------------------------------------

/// Complete Ashtakavarga result: all 7 BAVs + SAV with sodhana.
#[derive(Debug, Clone, Copy)]
pub struct AshtakavargaResult {
    pub bavs: [BhinnaAshtakavarga; 7],
    pub sav: SarvaAshtakavarga,
}

/// Calculate complete Ashtakavarga for given planetary positions.
///
/// Arguments:
/// - `graha_rashis`: 0-based rashi index for 7 grahas (Sun..Saturn)
/// - `lagna_rashi`: 0-based rashi index of the Ascendant
pub fn calculate_ashtakavarga(
    graha_rashis: &[u8; 7],
    lagna_rashi: u8,
) -> AshtakavargaResult {
    let bavs = calculate_all_bav(graha_rashis, lagna_rashi);
    let sav = calculate_sav(&bavs);
    AshtakavargaResult { bavs, sav }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rules_table_bav_totals() {
        // Verify that the sum of favorable offsets per graha matches expected totals.
        // This is chart-independent: just count the bits in the rules.
        for (graha, &expected) in BAV_TOTALS.iter().enumerate() {
            let total: u32 = RULES[graha]
                .iter()
                .map(|&mask| mask.count_ones())
                .sum();
            assert_eq!(
                total as u8, expected,
                "BAV total mismatch for graha {graha}: got {total}, expected {expected}"
            );
        }
    }

    #[test]
    fn rules_table_sav_total() {
        let total: u32 = RULES
            .iter()
            .flat_map(|r| r.iter())
            .map(|&mask| mask.count_ones())
            .sum();
        assert_eq!(total as u16, SAV_TOTAL);
    }

    #[test]
    fn bav_totals_match_invariants() {
        // All planets at Mesha (0), Lagna at Mesha (0)
        let rashis = [0u8; 7];
        let bavs = calculate_all_bav(&rashis, 0);
        for (i, bav) in bavs.iter().enumerate() {
            assert_eq!(
                bav.total(),
                BAV_TOTALS[i],
                "BAV total for graha {i} doesn't match invariant"
            );
        }
    }

    #[test]
    fn bav_totals_any_position() {
        // Different positions: Sun=3, Moon=7, Mars=0, Mercury=11, Jupiter=5, Venus=9, Saturn=2
        let rashis = [3, 7, 0, 11, 5, 9, 2];
        let bavs = calculate_all_bav(&rashis, 1);
        for (i, bav) in bavs.iter().enumerate() {
            assert_eq!(
                bav.total(),
                BAV_TOTALS[i],
                "BAV total for graha {i} doesn't match at arbitrary position"
            );
        }
    }

    #[test]
    fn sav_total_is_337() {
        let rashis = [0u8; 7];
        let bavs = calculate_all_bav(&rashis, 0);
        let sav = calculate_sav(&bavs);
        let total: u16 = sav.total_points.iter().map(|&p| p as u16).sum();
        assert_eq!(total, 337);
    }

    #[test]
    fn sav_total_any_position() {
        let rashis = [5, 2, 8, 10, 1, 6, 4];
        let bavs = calculate_all_bav(&rashis, 9);
        let sav = calculate_sav(&bavs);
        let total: u16 = sav.total_points.iter().map(|&p| p as u16).sum();
        assert_eq!(total, 337);
    }

    #[test]
    fn trikona_sodhana_basic() {
        let totals = [28, 25, 30, 20, 32, 22, 35, 18, 25, 27, 40, 15];
        let result = trikona_sodhana(&totals);
        // Fire [0,4,8]: min(28,32,25)=25 → [3,7,0]
        assert_eq!(result[0], 3);
        assert_eq!(result[4], 7);
        assert_eq!(result[8], 0);
        // Earth [1,5,9]: min(25,22,27)=22 → [3,0,5]
        assert_eq!(result[1], 3);
        assert_eq!(result[5], 0);
        assert_eq!(result[9], 5);
        // Air [2,6,10]: min(30,35,40)=30 → [0,5,10]
        assert_eq!(result[2], 0);
        assert_eq!(result[6], 5);
        assert_eq!(result[10], 10);
        // Water [3,7,11]: min(20,18,15)=15 → [5,3,0]
        assert_eq!(result[3], 5);
        assert_eq!(result[7], 3);
        assert_eq!(result[11], 0);
    }

    #[test]
    fn ekadhipatya_sodhana_basic() {
        let after_trikona = [3, 3, 15, 5, 7, 12, 5, 3, 10, 5, 10, 8];
        let result = ekadhipatya_sodhana(&after_trikona);
        // Mercury pair [2,5]: min(15,12)=12 → [3,0]
        assert_eq!(result[2], 3);
        assert_eq!(result[5], 0);
        // Jupiter pair [8,11]: min(10,8)=8 → [2,0]
        assert_eq!(result[8], 2);
        assert_eq!(result[11], 0);
        // Other rashis unchanged
        assert_eq!(result[0], 3);
        assert_eq!(result[1], 3);
        assert_eq!(result[3], 5);
        assert_eq!(result[4], 7);
        assert_eq!(result[6], 5);
        assert_eq!(result[7], 3);
        assert_eq!(result[9], 5);
        assert_eq!(result[10], 10);
    }

    #[test]
    fn bav_points_in_range() {
        let rashis = [2, 8, 5, 0, 11, 3, 7];
        let bavs = calculate_all_bav(&rashis, 6);
        for bav in &bavs {
            for &p in &bav.points {
                assert!(p <= 8, "BAV point {} exceeds max 8", p);
            }
        }
    }

    #[test]
    fn full_ashtakavarga_result() {
        let rashis = [0, 3, 6, 9, 1, 4, 7];
        let result = calculate_ashtakavarga(&rashis, 10);
        // All BAV totals match
        for (i, bav) in result.bavs.iter().enumerate() {
            assert_eq!(bav.total(), BAV_TOTALS[i]);
        }
        // SAV total is 337
        let sav_total: u16 = result.sav.total_points.iter().map(|&p| p as u16).sum();
        assert_eq!(sav_total, 337);
        // Trikona reduces total (at least some values decrease)
        let trikona_total: u16 = result.sav.after_trikona.iter().map(|&p| p as u16).sum();
        assert!(trikona_total <= 337);
        // Ekadhipatya further reduces
        let ekadhi_total: u16 = result.sav.after_ekadhipatya.iter().map(|&p| p as u16).sum();
        assert!(ekadhi_total <= trikona_total);
    }
}
