//! Kaal Chakra dasha static data — 24 Dasha Progressions and mappings.
//!
//! Each progression is a sequence of 9 rashis with fixed durations.
//! 27 nakshatras × 4 padas map to one of the 24 DPs.
//!
//! Source: BPHS Chapters 46-53 (public domain Vedic text).
//! Data transcribed from standard BPHS translation tables.

use super::variation::SubPeriodMethod;

/// Fixed rashi durations in years (indexed by rashi 0-11: Mesha..Meena).
pub const KCD_RASHI_YEARS: [f64; 12] = [
    7.0,  // 0  Mesha (Aries)
    16.0, // 1  Vrishabha (Taurus)
    9.0,  // 2  Mithuna (Gemini)
    21.0, // 3  Karka (Cancer)
    5.0,  // 4  Simha (Leo)
    9.0,  // 5  Kanya (Virgo)
    16.0, // 6  Tula (Libra)
    7.0,  // 7  Vrischika (Scorpio)
    10.0, // 8  Dhanu (Sagittarius)
    4.0,  // 9  Makara (Capricorn)
    4.0,  // 10 Kumbha (Aquarius)
    10.0, // 11 Meena (Pisces)
];

/// A single Dasha Progression: 9-rashi sequence with total span.
#[derive(Debug, Clone, Copy)]
pub struct DashaProgression {
    /// 9 rashi indices (0-11) in traversal order.
    pub rashis: [u8; 9],
    /// Total span in years (sum of the 9 rashi durations).
    pub span: f64,
}

impl DashaProgression {
    /// Get the duration in years for a specific position (0-8) in the sequence.
    pub fn duration_at(&self, position: usize) -> f64 {
        if position < 9 {
            KCD_RASHI_YEARS[self.rashis[position] as usize]
        } else {
            0.0
        }
    }
}

/// Build a DashaProgression from a rashi sequence.
const fn dp(rashis: [u8; 9]) -> DashaProgression {
    let mut span = 0.0;
    let mut i = 0;
    while i < 9 {
        span += KCD_RASHI_YEARS[rashis[i] as usize];
        i += 1;
    }
    DashaProgression { rashis, span }
}

// Rashi index constants for readability
const MESH: u8 = 0;
const VRSH: u8 = 1;
const MITH: u8 = 2;
const KARK: u8 = 3;
const SIMH: u8 = 4;
const KANY: u8 = 5;
const TULA: u8 = 6;
const VRSC: u8 = 7;
const DHAN: u8 = 8;
const MAKR: u8 = 9;
const KUMB: u8 = 10;
const MEEN: u8 = 11;

/// All 24 Dasha Progressions (DP0..DP23).
///
/// DP0-11: Direct progressions (Table VI from BPHS).
/// DP12-23: Indirect progressions (Table VII from BPHS).
pub const ALL_DPS: [DashaProgression; 24] = [
    // ── Direct progressions (Table VI) ──
    // DP0:  Pada 1, Navamsha Mesha, span=100
    dp([MESH, VRSH, MITH, KARK, SIMH, KANY, TULA, VRSC, DHAN]),
    // DP1:  Pada 2, Navamsha Vrishabha, span=85
    dp([MAKR, KUMB, MEEN, VRSC, TULA, KANY, KARK, SIMH, MITH]),
    // DP2:  Pada 3, Navamsha Mithuna, span=83
    dp([VRSH, MESH, MEEN, KUMB, MAKR, DHAN, MESH, VRSH, MITH]),
    // DP3:  Pada 4, Navamsha Karka, span=86
    dp([KARK, SIMH, KANY, TULA, VRSC, DHAN, MAKR, KUMB, MEEN]),
    // DP4:  Pada 1, Navamsha Simha, span=100
    dp([VRSC, TULA, KANY, KARK, SIMH, MITH, VRSH, MESH, MEEN]),
    // DP5:  Pada 2, Navamsha Kanya, span=85
    dp([KUMB, MAKR, DHAN, MESH, VRSH, MITH, KARK, SIMH, KANY]),
    // DP6:  Pada 3, Navamsha Tula, span=83
    dp([TULA, VRSC, DHAN, MAKR, KUMB, MEEN, VRSC, TULA, KANY]),
    // DP7:  Pada 4, Navamsha Vrischika, span=86
    dp([KARK, SIMH, MITH, VRSH, MESH, MEEN, KUMB, MAKR, DHAN]),
    // DP8:  Pada 1, Navamsha Dhanu, span=100 (same as DP0)
    dp([MESH, VRSH, MITH, KARK, SIMH, KANY, TULA, VRSC, DHAN]),
    // DP9:  Pada 2, Navamsha Makara, span=85 (same as DP1)
    dp([MAKR, KUMB, MEEN, VRSC, TULA, KANY, KARK, SIMH, MITH]),
    // DP10: Pada 3, Navamsha Kumbha, span=83 (same as DP2)
    dp([VRSH, MESH, MEEN, KUMB, MAKR, DHAN, MESH, VRSH, MITH]),
    // DP11: Pada 4, Navamsha Meena, span=86 (same as DP3)
    dp([KARK, SIMH, KANY, TULA, VRSC, DHAN, MAKR, KUMB, MEEN]),
    // ── Indirect progressions (Table VII) ──
    // DP12: Pada 1, Navamsha Vrischika, span=86
    dp([DHAN, MAKR, KUMB, MEEN, MESH, VRSH, MITH, SIMH, KARK]),
    // DP13: Pada 2, Navamsha Tula, span=83
    dp([KANY, TULA, VRSC, MEEN, KUMB, MAKR, DHAN, VRSC, TULA]),
    // DP14: Pada 3, Navamsha Kanya, span=85
    dp([KANY, SIMH, KARK, MITH, VRSH, MESH, DHAN, MAKR, KUMB]),
    // DP15: Pada 4, Navamsha Simha, span=100
    dp([MEEN, MESH, VRSH, MITH, SIMH, KARK, KANY, TULA, VRSC]),
    // DP16: Pada 1, Navamsha Karka, span=86
    dp([MEEN, KUMB, MAKR, DHAN, VRSC, TULA, KANY, SIMH, KARK]),
    // DP17: Pada 2, Navamsha Mithuna, span=83
    dp([MITH, VRSH, MESH, DHAN, MAKR, KUMB, MEEN, MESH, VRSH]),
    // DP18: Pada 3, Navamsha Vrishabha, span=85
    dp([MITH, SIMH, KARK, KANY, TULA, VRSC, MEEN, KUMB, MAKR]),
    // DP19: Pada 4, Navamsha Mesha, span=100
    dp([DHAN, VRSC, TULA, KANY, SIMH, KARK, MITH, VRSH, MESH]),
    // DP20: Pada 1, Navamsha Meena, span=86 (same as DP16)
    dp([MEEN, KUMB, MAKR, DHAN, VRSC, TULA, KANY, SIMH, KARK]),
    // DP21: Pada 2, Navamsha Kumbha, span=83 (same as DP17)
    dp([MITH, VRSH, MESH, DHAN, MAKR, KUMB, MEEN, MESH, VRSH]),
    // DP22: Pada 3, Navamsha Makara, span=85 (same as DP18)
    dp([MITH, SIMH, KARK, KANY, TULA, VRSC, MEEN, KUMB, MAKR]),
    // DP23: Pada 4, Navamsha Dhanu, span=100 (same as DP19)
    dp([DHAN, VRSC, TULA, KANY, SIMH, KARK, MITH, VRSH, MESH]),
];

/// Nakshatra-to-DP mapping: `KCD_NAKSHATRA_PADA_MAP[nak_idx][pada_idx]` → DP index (0-23).
///
/// 27 nakshatras × 4 padas. Pada index is 0-based internally.
///
/// Group assignments from BPHS:
/// - Group 1 (Table VI, DP 0-3):  Ashwini(0), Punarvasu(6), Hasta(12), Mula(18), PBhadra(24)
/// - Group 2 (Table VI, DP 4-7):  Bharani(1), Pushya(7), Chitra(13), PAshadha(19), UBhadra(25)
/// - Group 3 (Table VI, DP 8-11): Kritika(2), Ashlesha(8), Swati(14), UAshadha(20), Revati(26)
/// - Group 4 (Table VII, DP 12-15): Rohini(3), Magha(9), Vishakha(15), Shravana(21)
/// - Group 5 (Table VII, DP 16-19): Mrigashira(4), PPhalguni(10), Anuradha(16), Dhanishta(22)
/// - Group 6 (Table VII, DP 20-23): Ardra(5), UPhalguni(11), Jyeshtha(17), Shatabhisha(23)
pub const KCD_NAKSHATRA_PADA_MAP: [[u8; 4]; 27] = [
    [0, 1, 2, 3],     // 0  Ashwini     → Group 1 (DP 0-3)
    [4, 5, 6, 7],     // 1  Bharani     → Group 2 (DP 4-7)
    [8, 9, 10, 11],   // 2  Kritika     → Group 3 (DP 8-11)
    [12, 13, 14, 15], // 3  Rohini      → Group 4 (DP 12-15)
    [16, 17, 18, 19], // 4  Mrigashira  → Group 5 (DP 16-19)
    [20, 21, 22, 23], // 5  Ardra       → Group 6 (DP 20-23)
    [0, 1, 2, 3],     // 6  Punarvasu   → Group 1
    [4, 5, 6, 7],     // 7  Pushya      → Group 2
    [8, 9, 10, 11],   // 8  Ashlesha    → Group 3
    [12, 13, 14, 15], // 9  Magha       → Group 4
    [16, 17, 18, 19], // 10 PPhalguni   → Group 5
    [20, 21, 22, 23], // 11 UPhalguni   → Group 6
    [0, 1, 2, 3],     // 12 Hasta       → Group 1
    [4, 5, 6, 7],     // 13 Chitra      → Group 2
    [8, 9, 10, 11],   // 14 Swati       → Group 3
    [12, 13, 14, 15], // 15 Vishakha    → Group 4
    [16, 17, 18, 19], // 16 Anuradha    → Group 5
    [20, 21, 22, 23], // 17 Jyeshtha    → Group 6
    [0, 1, 2, 3],     // 18 Mula        → Group 1
    [4, 5, 6, 7],     // 19 PAshadha    → Group 2
    [8, 9, 10, 11],   // 20 UAshadha    → Group 3
    [12, 13, 14, 15], // 21 Shravana    → Group 4
    [16, 17, 18, 19], // 22 Dhanishta   → Group 5
    [20, 21, 22, 23], // 23 Shatabhisha → Group 6
    [0, 1, 2, 3],     // 24 PBhadra     → Group 1
    [4, 5, 6, 7],     // 25 UBhadra     → Group 2
    [8, 9, 10, 11],   // 26 Revati      → Group 3
];

/// Default sub-period method for Kaal Chakra.
///
/// Sub-periods use proportional distribution with the same fixed rashi durations,
/// starting from the parent rashi.
pub const KCD_DEFAULT_METHOD: SubPeriodMethod = SubPeriodMethod::ProportionalFromParent;

/// Number of rashis per DP sequence.
pub const KCD_RASHIS_PER_DP: usize = 9;

/// Span of one nakshatra pada in degrees (360 / 27 / 4 = 3.333...).
pub const PADA_DEGREES: f64 = 360.0 / 27.0 / 4.0;

/// Get the DP index for a given nakshatra index (0-26) and pada (0-3).
pub fn kcd_dp_index(nakshatra_index: u8, pada_index: u8) -> u8 {
    let nak = (nakshatra_index % 27) as usize;
    let pada = (pada_index % 4) as usize;
    KCD_NAKSHATRA_PADA_MAP[nak][pada]
}

/// Get the DashaProgression for a given nakshatra index and pada.
pub fn kcd_progression(nakshatra_index: u8, pada_index: u8) -> &'static DashaProgression {
    let idx = kcd_dp_index(nakshatra_index, pada_index) as usize;
    &ALL_DPS[idx]
}

/// Calculate Moon's pada (0-3) from its position within a nakshatra.
pub fn pada_from_nakshatra_position(position_in_nakshatra: f64) -> u8 {
    let nakshatra_span = 360.0 / 27.0;
    let pada = (position_in_nakshatra / (nakshatra_span / 4.0)).floor() as u8;
    pada.min(3)
}

/// Calculate birth balance for Kaal Chakra dasha.
///
/// Returns `(starting_position, balance_days)`:
/// - `starting_position`: index (0-8) into the DP's rashi sequence where the birth falls
/// - `balance_days`: remaining days in that starting rashi
///
/// The Moon's fractional position within its pada determines how much of the
/// DP's total span has elapsed.
pub fn kcd_birth_balance(moon_sidereal_lon: f64, dp: &DashaProgression) -> (usize, f64) {
    let nakshatra_span = 360.0 / 27.0;
    let lon = crate::util::normalize_360(moon_sidereal_lon);

    // Position within the current nakshatra
    let position_in_nak = lon % nakshatra_span;
    // Position within the current pada
    let pada_span = nakshatra_span / 4.0;
    let position_in_pada = position_in_nak % pada_span;
    let elapsed_fraction = position_in_pada / pada_span;

    // Total elapsed years in the DP
    let elapsed_years = elapsed_fraction * dp.span;

    // Find which rashi in the sequence contains the elapsed point
    let mut cumulative = 0.0;
    for i in 0..KCD_RASHIS_PER_DP {
        let rashi_years = dp.duration_at(i);
        if cumulative + rashi_years >= elapsed_years {
            let elapsed_in_rashi = elapsed_years - cumulative;
            let balance_years = rashi_years - elapsed_in_rashi;
            let balance_days = balance_years * super::types::DAYS_PER_YEAR;
            return (i, balance_days);
        }
        cumulative += rashi_years;
    }

    // Fallback: start from first rashi with full duration
    (0, dp.duration_at(0) * super::types::DAYS_PER_YEAR)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dp_spans_are_correct() {
        // Verify known spans
        assert!((ALL_DPS[0].span - 100.0).abs() < 1e-10); // DP0
        assert!((ALL_DPS[1].span - 85.0).abs() < 1e-10); // DP1
        assert!((ALL_DPS[2].span - 83.0).abs() < 1e-10); // DP2
        assert!((ALL_DPS[3].span - 86.0).abs() < 1e-10); // DP3
    }

    #[test]
    fn all_dp_spans_sum_correctly() {
        for (i, dp_item) in ALL_DPS.iter().enumerate() {
            let sum: f64 = dp_item
                .rashis
                .iter()
                .map(|&r| KCD_RASHI_YEARS[r as usize])
                .sum();
            assert!(
                (sum - dp_item.span).abs() < 1e-10,
                "DP{} span mismatch: sum={}, span={}",
                i,
                sum,
                dp_item.span
            );
        }
    }

    #[test]
    fn nakshatra_pada_map_all_valid() {
        for (nak, padas) in KCD_NAKSHATRA_PADA_MAP.iter().enumerate() {
            for (pada, &dp_idx) in padas.iter().enumerate() {
                assert!(
                    (dp_idx as usize) < 24,
                    "nak {} pada {} maps to invalid DP {}",
                    nak,
                    pada,
                    dp_idx
                );
            }
        }
    }

    #[test]
    fn group_mapping_consistency() {
        // Group 1 nakshatras: 0(Ashwini), 6(Punarvasu), 12(Hasta), 18(Mula), 24(PBhadra)
        let group1 = [0, 6, 12, 18, 24];
        for &nak in &group1 {
            assert_eq!(KCD_NAKSHATRA_PADA_MAP[nak as usize], [0, 1, 2, 3]);
        }
    }

    #[test]
    fn kcd_dp_index_basic() {
        assert_eq!(kcd_dp_index(0, 0), 0); // Ashwini pada 1 → DP0
        assert_eq!(kcd_dp_index(0, 3), 3); // Ashwini pada 4 → DP3
        assert_eq!(kcd_dp_index(3, 0), 12); // Rohini pada 1 → DP12
    }

    #[test]
    fn kcd_birth_balance_at_pada_start() {
        // Moon at exact start of a pada → 0 elapsed → full first rashi period
        let dp_item = &ALL_DPS[0]; // DP0: starts with Mesha (7y)
        let (pos, balance) = kcd_birth_balance(0.0, dp_item); // 0 deg = Ashwini pada 1 start
        assert_eq!(pos, 0);
        assert!((balance - 7.0 * 365.25).abs() < 1.0);
    }

    #[test]
    fn kcd_birth_balance_at_pada_midpoint() {
        // Moon at midpoint of Ashwini pada 1 → 50% elapsed through DP0 (100y span)
        let dp_item = &ALL_DPS[0];
        let mid = PADA_DEGREES / 2.0; // 1.6667 degrees
        let (pos, balance) = kcd_birth_balance(mid, dp_item);
        // 50% of 100y = 50y elapsed
        // DP0 rashis: Mesha(7) + Vrish(16) + Mith(9) + Kark(21) = 53y at pos 3
        // So 50y falls in Karka (pos 3): cumulative before = 32y, elapsed_in_karka = 18y
        // balance = 21 - 18 = 3 years
        assert_eq!(pos, 3);
        assert!((balance - 3.0 * 365.25).abs() < 2.0);
    }

    #[test]
    fn pada_from_position() {
        let nak_span = 360.0 / 27.0;
        assert_eq!(pada_from_nakshatra_position(0.0), 0);
        assert_eq!(pada_from_nakshatra_position(nak_span / 4.0 - 0.01), 0);
        assert_eq!(pada_from_nakshatra_position(nak_span / 4.0), 1);
        assert_eq!(pada_from_nakshatra_position(nak_span - 0.01), 3);
    }

    #[test]
    fn rashi_years_sum() {
        let total: f64 = KCD_RASHI_YEARS.iter().sum();
        // Sum: 7+16+9+21+5+9+16+7+10+4+4+10 = 118
        assert!((total - 118.0).abs() < 1e-10);
    }
}
