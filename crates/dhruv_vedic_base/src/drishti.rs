//! Graha drishti (planetary aspect) calculation using virupa strength.
//!
//! Computes how strongly each graha aspects any sidereal point using the
//! classical piecewise virupa formula, with special bonuses for Mars (4th/8th),
//! Jupiter (5th/9th), and Saturn (3rd/10th).
//!
//! Clean-room implementation from standard Vedic jyotish texts (BPHS).

use crate::graha::Graha;
use crate::util::normalize_360;

/// Aspect strength for a single graha→target pair.
#[derive(Debug, Clone, Copy)]
pub struct DrishtiEntry {
    /// Angular distance from source to target in [0, 360).
    pub angular_distance: f64,
    /// Base virupa from the piecewise formula.
    pub base_virupa: f64,
    /// Planet-specific bonus (Mars/Jupiter/Saturn only).
    pub special_virupa: f64,
    /// Total virupa = base + special.
    pub total_virupa: f64,
}

impl DrishtiEntry {
    /// Zeroed sentinel entry.
    pub const fn zero() -> Self {
        Self {
            angular_distance: 0.0,
            base_virupa: 0.0,
            special_virupa: 0.0,
            total_virupa: 0.0,
        }
    }
}

/// 9×9 graha-to-graha drishti matrix.
#[derive(Debug, Clone, Copy)]
pub struct GrahaDrishtiMatrix {
    /// `entries[source][target]` — indexed by `Graha::index()`.
    pub entries: [[DrishtiEntry; 9]; 9],
}

/// Piecewise base virupa for a given angular distance.
///
/// The formula maps angular separation to aspect strength (virupa units):
/// - `[0, 30)`:   0
/// - `[30, 90)`:  `(A - 30) * 0.75`       → 0..45
/// - `[90, 150)`: `45 - (A - 90) * 0.75`   → 45..0
/// - `[150, 180)`: `(A - 150) * 2`          → 0..60
/// - `[180, 300)`: `60 - (A - 180) * 0.5`   → 60..0
/// - `[300, 360)`: 0
pub fn base_virupa(angular_distance: f64) -> f64 {
    let a = normalize_360(angular_distance);
    if a < 30.0 {
        0.0
    } else if a < 90.0 {
        (a - 30.0) * 0.75
    } else if a < 150.0 {
        45.0 - (a - 90.0) * 0.75
    } else if a < 180.0 {
        (a - 150.0) * 2.0
    } else if a < 300.0 {
        60.0 - (a - 180.0) * 0.5
    } else {
        0.0
    }
}

/// Planet-specific bonus virupa for special aspects.
///
/// - Mars: +15 if angular distance is in `[90, 120)` or `[210, 240)`
/// - Jupiter: +30 if angular distance is in `[120, 150)` or `[240, 270)`
/// - Saturn: +45 if angular distance is in `[60, 90)` or `[270, 300)`
/// - All others: 0
pub fn special_virupa(graha: Graha, angular_distance: f64) -> f64 {
    let a = normalize_360(angular_distance);
    match graha {
        Graha::Mangal => {
            if (90.0..120.0).contains(&a) || (210.0..240.0).contains(&a) {
                15.0
            } else {
                0.0
            }
        }
        Graha::Guru => {
            if (120.0..150.0).contains(&a) || (240.0..270.0).contains(&a) {
                30.0
            } else {
                0.0
            }
        }
        Graha::Shani => {
            if (60.0..90.0).contains(&a) || (270.0..300.0).contains(&a) {
                45.0
            } else {
                0.0
            }
        }
        _ => 0.0,
    }
}

/// Compute drishti from a single graha to a single sidereal point.
///
/// Angular distance = `normalize_360(target_lon - source_lon)`.
pub fn graha_drishti(graha: Graha, source_lon: f64, target_lon: f64) -> DrishtiEntry {
    let angular_distance = normalize_360(target_lon - source_lon);
    let base = base_virupa(angular_distance);
    let special = special_virupa(graha, angular_distance);
    DrishtiEntry {
        angular_distance,
        base_virupa: base,
        special_virupa: special,
        total_virupa: base + special,
    }
}

/// Compute the full 9×9 graha drishti matrix.
///
/// Self-aspect (diagonal) entries are zeroed.
pub fn graha_drishti_matrix(longitudes: &[f64; 9]) -> GrahaDrishtiMatrix {
    use crate::graha::ALL_GRAHAS;

    let mut entries = [[DrishtiEntry::zero(); 9]; 9];
    for src in ALL_GRAHAS {
        let si = src.index() as usize;
        for tgt in ALL_GRAHAS {
            let ti = tgt.index() as usize;
            if si == ti {
                continue; // self-aspect stays zero
            }
            entries[si][ti] = graha_drishti(src, longitudes[si], longitudes[ti]);
        }
    }
    GrahaDrishtiMatrix { entries }
}

#[cfg(test)]
mod tests {
    use super::*;

    const EPS: f64 = 1e-10;

    // --- base_virupa boundary tests ---

    #[test]
    fn base_virupa_at_0() {
        assert!((base_virupa(0.0)).abs() < EPS);
    }

    #[test]
    fn base_virupa_at_15() {
        // Inside [0, 30): 0
        assert!((base_virupa(15.0)).abs() < EPS);
    }

    #[test]
    fn base_virupa_at_30() {
        // Start of [30, 90): (30-30)*0.75 = 0
        assert!((base_virupa(30.0)).abs() < EPS);
    }

    #[test]
    fn base_virupa_at_60() {
        // (60-30)*0.75 = 22.5
        assert!((base_virupa(60.0) - 22.5).abs() < EPS);
    }

    #[test]
    fn base_virupa_at_90() {
        // Boundary: from [30,90) last value approaches 45
        // At 90: enters [90,150), 45-(90-90)*0.75 = 45
        assert!((base_virupa(90.0) - 45.0).abs() < EPS);
    }

    #[test]
    fn base_virupa_at_120() {
        // 45 - (120-90)*0.75 = 45 - 22.5 = 22.5
        assert!((base_virupa(120.0) - 22.5).abs() < EPS);
    }

    #[test]
    fn base_virupa_at_150() {
        // [150,180): (150-150)*2 = 0
        assert!((base_virupa(150.0)).abs() < EPS);
    }

    #[test]
    fn base_virupa_at_165() {
        // (165-150)*2 = 30
        assert!((base_virupa(165.0) - 30.0).abs() < EPS);
    }

    #[test]
    fn base_virupa_at_180() {
        // [180,300): 60-(180-180)*0.5 = 60
        assert!((base_virupa(180.0) - 60.0).abs() < EPS);
    }

    #[test]
    fn base_virupa_at_240() {
        // 60-(240-180)*0.5 = 60-30 = 30
        assert!((base_virupa(240.0) - 30.0).abs() < EPS);
    }

    #[test]
    fn base_virupa_at_300() {
        // [300,360): 0
        assert!((base_virupa(300.0)).abs() < EPS);
    }

    #[test]
    fn base_virupa_at_330() {
        assert!((base_virupa(330.0)).abs() < EPS);
    }

    #[test]
    fn base_virupa_at_360() {
        // normalize_360(360) = 0, which is [0,30) → 0
        assert!((base_virupa(360.0)).abs() < EPS);
    }

    // --- special_virupa tests ---

    #[test]
    fn mars_special_at_100() {
        // Mars in [90, 120): +15
        assert!((special_virupa(Graha::Mangal, 100.0) - 15.0).abs() < EPS);
    }

    #[test]
    fn mars_special_at_225() {
        // Mars in [210, 240): +15
        assert!((special_virupa(Graha::Mangal, 225.0) - 15.0).abs() < EPS);
    }

    #[test]
    fn mars_no_special_at_150() {
        assert!((special_virupa(Graha::Mangal, 150.0)).abs() < EPS);
    }

    #[test]
    fn jupiter_special_at_135() {
        // Jupiter in [120, 150): +30
        assert!((special_virupa(Graha::Guru, 135.0) - 30.0).abs() < EPS);
    }

    #[test]
    fn jupiter_special_at_250() {
        // Jupiter in [240, 270): +30
        assert!((special_virupa(Graha::Guru, 250.0) - 30.0).abs() < EPS);
    }

    #[test]
    fn jupiter_no_special_at_180() {
        assert!((special_virupa(Graha::Guru, 180.0)).abs() < EPS);
    }

    #[test]
    fn saturn_special_at_75() {
        // Saturn in [60, 90): +45
        assert!((special_virupa(Graha::Shani, 75.0) - 45.0).abs() < EPS);
    }

    #[test]
    fn saturn_special_at_285() {
        // Saturn in [270, 300): +45
        assert!((special_virupa(Graha::Shani, 285.0) - 45.0).abs() < EPS);
    }

    #[test]
    fn saturn_no_special_at_180() {
        assert!((special_virupa(Graha::Shani, 180.0)).abs() < EPS);
    }

    #[test]
    fn no_special_for_sun() {
        for a in [60.0, 90.0, 120.0, 135.0, 180.0, 225.0, 270.0, 285.0] {
            assert!(
                (special_virupa(Graha::Surya, a)).abs() < EPS,
                "Sun should have no special at {a}"
            );
        }
    }

    #[test]
    fn no_special_for_moon() {
        for a in [60.0, 90.0, 120.0, 135.0, 180.0, 225.0, 270.0, 285.0] {
            assert!((special_virupa(Graha::Chandra, a)).abs() < EPS);
        }
    }

    #[test]
    fn no_special_for_mercury() {
        for a in [75.0, 100.0, 135.0, 250.0, 285.0] {
            assert!((special_virupa(Graha::Buddh, a)).abs() < EPS);
        }
    }

    #[test]
    fn no_special_for_venus() {
        for a in [75.0, 100.0, 135.0, 250.0, 285.0] {
            assert!((special_virupa(Graha::Shukra, a)).abs() < EPS);
        }
    }

    #[test]
    fn no_special_for_rahu_ketu() {
        for g in [Graha::Rahu, Graha::Ketu] {
            for a in [75.0, 100.0, 135.0, 250.0, 285.0] {
                assert!((special_virupa(g, a)).abs() < EPS);
            }
        }
    }

    // --- graha_drishti tests ---

    #[test]
    fn graha_drishti_basic() {
        let entry = graha_drishti(Graha::Surya, 0.0, 180.0);
        assert!((entry.angular_distance - 180.0).abs() < EPS);
        assert!((entry.base_virupa - 60.0).abs() < EPS);
        assert!((entry.special_virupa).abs() < EPS);
        assert!((entry.total_virupa - 60.0).abs() < EPS);
    }

    #[test]
    fn graha_drishti_wraparound() {
        // source=350, target=20 → distance=30
        let entry = graha_drishti(Graha::Surya, 350.0, 20.0);
        assert!((entry.angular_distance - 30.0).abs() < EPS);
        // At 30: (30-30)*0.75 = 0
        assert!((entry.base_virupa).abs() < EPS);
    }

    #[test]
    fn graha_drishti_mars_special() {
        // Mars at 0°, target at 100° → distance=100 in [90,120)
        let entry = graha_drishti(Graha::Mangal, 0.0, 100.0);
        assert!((entry.angular_distance - 100.0).abs() < EPS);
        // base: 45 - (100-90)*0.75 = 45 - 7.5 = 37.5
        assert!((entry.base_virupa - 37.5).abs() < EPS);
        assert!((entry.special_virupa - 15.0).abs() < EPS);
        assert!((entry.total_virupa - 52.5).abs() < EPS);
    }

    #[test]
    fn graha_drishti_jupiter_special() {
        // Jupiter at 0°, target at 135° → distance=135 in [120,150)
        let entry = graha_drishti(Graha::Guru, 0.0, 135.0);
        // base: 45 - (135-90)*0.75 = 45 - 33.75 = 11.25
        assert!((entry.base_virupa - 11.25).abs() < EPS);
        assert!((entry.special_virupa - 30.0).abs() < EPS);
        assert!((entry.total_virupa - 41.25).abs() < EPS);
    }

    #[test]
    fn graha_drishti_saturn_special() {
        // Saturn at 0°, target at 75° → distance=75 in [60,90)
        let entry = graha_drishti(Graha::Shani, 0.0, 75.0);
        // base: (75-30)*0.75 = 33.75
        assert!((entry.base_virupa - 33.75).abs() < EPS);
        assert!((entry.special_virupa - 45.0).abs() < EPS);
        assert!((entry.total_virupa - 78.75).abs() < EPS);
    }

    // --- graha_drishti_matrix tests ---

    #[test]
    fn matrix_diagonal_zero() {
        let lons = [0.0, 30.0, 60.0, 90.0, 120.0, 150.0, 180.0, 210.0, 240.0];
        let matrix = graha_drishti_matrix(&lons);
        for i in 0..9 {
            let e = &matrix.entries[i][i];
            assert!(
                (e.angular_distance).abs() < EPS,
                "diagonal[{i}] angular_distance"
            );
            assert!((e.base_virupa).abs() < EPS, "diagonal[{i}] base_virupa");
            assert!(
                (e.special_virupa).abs() < EPS,
                "diagonal[{i}] special_virupa"
            );
            assert!((e.total_virupa).abs() < EPS, "diagonal[{i}] total_virupa");
        }
    }

    #[test]
    fn matrix_is_asymmetric() {
        // Mars (idx 2) → Jupiter (idx 4): distance = 120-60 = 60
        // Jupiter (idx 4) → Mars (idx 2): distance = 360-(120-60) = 300
        let lons = [0.0, 30.0, 60.0, 90.0, 120.0, 150.0, 180.0, 210.0, 240.0];
        let matrix = graha_drishti_matrix(&lons);
        let mars_to_jupiter = &matrix.entries[2][4];
        let jupiter_to_mars = &matrix.entries[4][2];
        // angular distances should differ
        assert!((mars_to_jupiter.angular_distance - 60.0).abs() < EPS);
        assert!((jupiter_to_mars.angular_distance - 300.0).abs() < EPS);
    }

    #[test]
    fn matrix_off_diagonal_nonzero() {
        // With spread-out longitudes, most off-diagonal entries should be nonzero
        let lons = [10.0, 50.0, 100.0, 140.0, 200.0, 250.0, 310.0, 330.0, 350.0];
        let matrix = graha_drishti_matrix(&lons);
        let mut nonzero_count = 0;
        for i in 0..9 {
            for j in 0..9 {
                if i != j && matrix.entries[i][j].total_virupa > 0.0 {
                    nonzero_count += 1;
                }
            }
        }
        // Most of 72 off-diagonal entries should be nonzero
        assert!(
            nonzero_count > 30,
            "expected many nonzero entries, got {nonzero_count}"
        );
    }

    #[test]
    fn matrix_mars_special_present() {
        // Place Mars at 0° and another graha at 100° (distance 100, in [90,120))
        let mut lons = [0.0; 9];
        lons[Graha::Mangal.index() as usize] = 0.0;
        lons[Graha::Surya.index() as usize] = 100.0;
        let matrix = graha_drishti_matrix(&lons);
        let entry = &matrix.entries[Graha::Mangal.index() as usize][Graha::Surya.index() as usize];
        assert!(
            (entry.special_virupa - 15.0).abs() < EPS,
            "Mars special aspect"
        );
    }

    #[test]
    fn matrix_jupiter_special_present() {
        let mut lons = [0.0; 9];
        lons[Graha::Guru.index() as usize] = 0.0;
        lons[Graha::Surya.index() as usize] = 135.0;
        let matrix = graha_drishti_matrix(&lons);
        let entry = &matrix.entries[Graha::Guru.index() as usize][Graha::Surya.index() as usize];
        assert!(
            (entry.special_virupa - 30.0).abs() < EPS,
            "Jupiter special aspect"
        );
    }

    #[test]
    fn matrix_saturn_special_present() {
        let mut lons = [0.0; 9];
        lons[Graha::Shani.index() as usize] = 0.0;
        lons[Graha::Surya.index() as usize] = 75.0;
        let matrix = graha_drishti_matrix(&lons);
        let entry = &matrix.entries[Graha::Shani.index() as usize][Graha::Surya.index() as usize];
        assert!(
            (entry.special_virupa - 45.0).abs() < EPS,
            "Saturn special aspect"
        );
    }
}
