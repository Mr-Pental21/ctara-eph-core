//! Rashi strength comparison and chart inputs for rashi-based dasha systems.
//!
//! Implements the BPHS 7-rule hierarchy for determining the "stronger" of
//! two rashis. Used by Chara, Yogardha, Shoola, Mandooka, and Kendradi systems.
//!
//! Also defines `RashiDashaInputs`, the chart data required by all rashi-based
//! dasha computations. This struct is assembled by the orchestration layer.

use super::rashi_util::is_odd_sign;
use crate::graha::{ALL_GRAHAS, Graha, rashi_lord_by_index};
use crate::rashi::ALL_RASHIS;

/// Chart inputs needed by all rashi-based dasha systems.
///
/// Assembled by the orchestration layer (dhruv_search) from ephemeris queries.
/// The pure-math dasha layer receives this struct and does not call the engine.
#[derive(Debug, Clone, Copy)]
pub struct RashiDashaInputs {
    /// Sidereal longitudes of all 9 grahas [0..360), indexed by `Graha::index()`.
    pub graha_sidereal_lons: [f64; 9],
    /// Sidereal longitude of the lagna (ascendant) [0..360).
    pub lagna_sidereal_lon: f64,
    /// 0-based rashi index of lagna (0=Mesha..11=Meena).
    pub lagna_rashi_index: u8,
    /// 0-based rashi index for each of the 12 bhavas (whole-sign: bhava N = lagna_rashi + N-1).
    pub bhava_rashi_indices: [u8; 12],
}

impl RashiDashaInputs {
    /// Build inputs from graha longitudes and lagna longitude.
    ///
    /// Uses whole-sign houses: bhava 1 = lagna rashi, bhava 2 = next rashi, etc.
    pub fn new(graha_sidereal_lons: [f64; 9], lagna_sidereal_lon: f64) -> Self {
        let lagna_rashi_index = (lagna_sidereal_lon / 30.0).floor() as u8 % 12;
        let mut bhava_rashi_indices = [0u8; 12];
        for (i, slot) in bhava_rashi_indices.iter_mut().enumerate() {
            *slot = (lagna_rashi_index + i as u8) % 12;
        }
        Self {
            graha_sidereal_lons,
            lagna_sidereal_lon,
            lagna_rashi_index,
            bhava_rashi_indices,
        }
    }

    /// Get the rashi index (0-11) where a graha is placed.
    pub fn graha_rashi(&self, graha: Graha) -> u8 {
        let lon = self.graha_sidereal_lons[graha.index() as usize];
        (lon / 30.0).floor() as u8 % 12
    }

    /// Count how many of the 9 grahas occupy a given rashi.
    pub fn count_occupants(&self, rashi_index: u8) -> u8 {
        let mut count = 0u8;
        for g in ALL_GRAHAS {
            if self.graha_rashi(g) == rashi_index {
                count += 1;
            }
        }
        count
    }

    /// Get the sidereal longitude of a rashi's lord.
    pub fn lord_longitude(&self, rashi_index: u8) -> f64 {
        let lord = rashi_lord_by_index(rashi_index).unwrap_or(Graha::Surya);
        self.graha_sidereal_lons[lord.index() as usize]
    }

    /// Get the rashi index where a rashi's lord is placed.
    pub fn lord_rashi(&self, rashi_index: u8) -> u8 {
        let lord = rashi_lord_by_index(rashi_index).unwrap_or(Graha::Surya);
        self.graha_rashi(lord)
    }
}

/// Exaltation rashi indices for the 7 classical grahas (sapta grahas).
/// Sun=0(Mesha), Moon=1(Vrishabha), Mars=9(Makara), Mercury=5(Kanya),
/// Jupiter=3(Karka), Venus=11(Meena), Saturn=6(Tula).
const EXALTATION_RASHI: [u8; 7] = [0, 1, 9, 5, 3, 11, 6];

/// Exaltation degree within sign for sapta grahas.
/// Sun=10, Moon=3, Mars=28, Mercury=15, Jupiter=5, Venus=27, Saturn=20.
const EXALTATION_DEG: [f64; 7] = [10.0, 3.0, 28.0, 15.0, 5.0, 27.0, 20.0];

/// Determine the stronger of two rashis using a simplified BPHS hierarchy.
///
/// Rules applied in order (first decisive rule wins):
/// 1. More planets present (count occupants)
/// 2. Lord aspected by Jupiter or Mercury (natural benefics)
/// 3. Lord has higher exaltation-point closeness
/// 4. Odd-sign preference for odd pair, even for even
/// 5. Rashi whose lord has higher longitude (fallback)
/// 6. Higher rashi index (final tiebreaker)
pub fn stronger_rashi(a: u8, b: u8, inputs: &RashiDashaInputs) -> u8 {
    let a = a % 12;
    let b = b % 12;
    if a == b {
        return a;
    }

    // Rule 1: More occupants
    let occ_a = inputs.count_occupants(a);
    let occ_b = inputs.count_occupants(b);
    if occ_a != occ_b {
        return if occ_a > occ_b { a } else { b };
    }

    // Rule 2: Lord aspected by Jupiter/Mercury (benefic association)
    let lord_a_benefic = lord_has_benefic_association(a, inputs);
    let lord_b_benefic = lord_has_benefic_association(b, inputs);
    if lord_a_benefic != lord_b_benefic {
        return if lord_a_benefic { a } else { b };
    }

    // Rule 3: Lord closer to exaltation point
    let exalt_a = exaltation_closeness(a, inputs);
    let exalt_b = exaltation_closeness(b, inputs);
    if (exalt_a - exalt_b).abs() > 1e-10 {
        return if exalt_a > exalt_b { a } else { b };
    }

    // Rule 4: Odd/even preference
    let a_odd = is_odd_sign(a);
    let b_odd = is_odd_sign(b);
    if a_odd != b_odd {
        // Prefer odd sign in an odd-even pair
        return if a_odd { a } else { b };
    }

    // Rule 5: Lord with higher longitude
    let lord_lon_a = inputs.lord_longitude(a);
    let lord_lon_b = inputs.lord_longitude(b);
    if (lord_lon_a - lord_lon_b).abs() > 1e-10 {
        return if lord_lon_a > lord_lon_b { a } else { b };
    }

    // Rule 6: Higher rashi index (final tiebreaker)
    if a > b { a } else { b }
}

/// Check if a rashi's lord is in the same sign as Jupiter or Mercury.
fn lord_has_benefic_association(rashi_index: u8, inputs: &RashiDashaInputs) -> bool {
    let lord = rashi_lord_by_index(rashi_index).unwrap_or(Graha::Surya);
    let lord_rashi = inputs.graha_rashi(lord);
    let guru_rashi = inputs.graha_rashi(Graha::Guru);
    let buddh_rashi = inputs.graha_rashi(Graha::Buddh);

    // Don't count self-association
    if lord == Graha::Guru || lord == Graha::Buddh {
        // Check the other benefic only
        if lord == Graha::Guru {
            return lord_rashi == buddh_rashi;
        }
        return lord_rashi == guru_rashi;
    }

    lord_rashi == guru_rashi || lord_rashi == buddh_rashi
}

/// Compute exaltation closeness for a rashi's lord (0.0 = debilitated, 1.0 = exalted).
///
/// Only applies to sapta grahas (indices 0-6). For Rahu/Ketu lords, returns 0.5.
fn exaltation_closeness(rashi_index: u8, inputs: &RashiDashaInputs) -> f64 {
    let lord = rashi_lord_by_index(rashi_index).unwrap_or(Graha::Surya);
    let idx = lord.index() as usize;
    if idx > 6 {
        return 0.5; // Rahu/Ketu: neutral
    }

    let lord_lon = inputs.graha_sidereal_lons[idx];
    let exalt_lon = EXALTATION_RASHI[idx] as f64 * 30.0 + EXALTATION_DEG[idx];

    // Angular distance from exaltation point (0-180 scale)
    let diff = (lord_lon - exalt_lon).rem_euclid(360.0);
    let dist = if diff > 180.0 { 360.0 - diff } else { diff };

    // Closeness: 1.0 at exaltation, 0.0 at debilitation (180 deg away)
    1.0 - (dist / 180.0)
}

/// Determine the Brahma Graha for Sthira dasha.
///
/// Brahma Graha is the graha among Venus, Jupiter, and Saturn that:
/// 1. Is placed in an odd sign AND in houses 1-7 from lagna
/// 2. Among qualifying grahas, the one with the highest longitude
///
/// If no graha qualifies, falls back to the lord of the 6th house.
pub fn brahma_graha(inputs: &RashiDashaInputs) -> Graha {
    let candidates = [Graha::Shukra, Graha::Guru, Graha::Shani];

    let mut best: Option<(Graha, f64)> = None;

    for &graha in &candidates {
        let g_rashi = inputs.graha_rashi(graha);

        // Must be in odd sign
        if !is_odd_sign(g_rashi) {
            continue;
        }

        // Must be in houses 1-7 from lagna (bhava indices 0-6)
        let house = house_from_lagna(g_rashi, inputs.lagna_rashi_index);
        if house > 7 {
            continue;
        }

        let lon = inputs.graha_sidereal_lons[graha.index() as usize];
        match &best {
            Some((_, best_lon)) => {
                if lon > *best_lon {
                    best = Some((graha, lon));
                }
            }
            None => {
                best = Some((graha, lon));
            }
        }
    }

    match best {
        Some((g, _)) => g,
        None => {
            // Fallback: lord of the 6th house
            let sixth_rashi = inputs.bhava_rashi_indices[5];
            rashi_lord_by_index(sixth_rashi).unwrap_or(Graha::Guru)
        }
    }
}

/// Compute house number (1-12) of a rashi from the lagna rashi.
fn house_from_lagna(rashi_index: u8, lagna_rashi: u8) -> u8 {
    let diff = (rashi_index as i16 - lagna_rashi as i16).rem_euclid(12) as u8;
    diff + 1
}

/// Determine the Atmakaraka (highest-degree planet, excluding Rahu/Ketu).
///
/// Returns the graha with the highest degree within its sign among the 7 sapta grahas.
pub fn atmakaraka(inputs: &RashiDashaInputs) -> Graha {
    let mut max_deg = -1.0f64;
    let mut ak = Graha::Surya;

    for &graha in &crate::graha::SAPTA_GRAHAS {
        let lon = inputs.graha_sidereal_lons[graha.index() as usize];
        let deg_in_sign = lon % 30.0;
        if deg_in_sign > max_deg {
            max_deg = deg_in_sign;
            ak = graha;
        }
    }

    ak
}

/// Get the rashi name by index (for display/debug).
pub fn rashi_name(index: u8) -> &'static str {
    if index < 12 {
        ALL_RASHIS[index as usize].name()
    } else {
        "Unknown"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graha::Graha;

    /// Helper: create inputs with all grahas at specified longitudes.
    fn make_inputs(lons: [f64; 9], lagna_lon: f64) -> RashiDashaInputs {
        RashiDashaInputs::new(lons, lagna_lon)
    }

    #[test]
    fn inputs_new_whole_sign() {
        // Lagna at 45 deg (Vrishabha, index 1)
        let inputs = make_inputs([0.0; 9], 45.0);
        assert_eq!(inputs.lagna_rashi_index, 1);
        assert_eq!(inputs.bhava_rashi_indices[0], 1); // House 1 = Vrishabha
        assert_eq!(inputs.bhava_rashi_indices[1], 2); // House 2 = Mithuna
        assert_eq!(inputs.bhava_rashi_indices[11], 0); // House 12 = Mesha
    }

    #[test]
    fn count_occupants_basic() {
        // Sun at 15 (Mesha), Moon at 10 (Mesha), rest at 100+ (various)
        let lons = [15.0, 10.0, 100.0, 110.0, 120.0, 130.0, 140.0, 250.0, 280.0];
        let inputs = make_inputs(lons, 15.0);
        assert_eq!(inputs.count_occupants(0), 2); // Mesha: Sun+Moon
        assert_eq!(inputs.count_occupants(1), 0); // Vrishabha: none
    }

    #[test]
    fn graha_rashi_basic() {
        let lons = [15.0, 45.0, 75.0, 105.0, 135.0, 165.0, 195.0, 225.0, 255.0];
        let inputs = make_inputs(lons, 15.0);
        assert_eq!(inputs.graha_rashi(Graha::Surya), 0); // 15 deg → Mesha
        assert_eq!(inputs.graha_rashi(Graha::Chandra), 1); // 45 deg → Vrishabha
        assert_eq!(inputs.graha_rashi(Graha::Mangal), 2); // 75 deg → Mithuna
    }

    #[test]
    fn stronger_same_rashi() {
        let inputs = make_inputs([0.0; 9], 0.0);
        assert_eq!(stronger_rashi(0, 0, &inputs), 0);
    }

    #[test]
    fn stronger_by_occupants() {
        // Two grahas in Mesha (0), none in Vrishabha (1)
        let lons = [15.0, 10.0, 100.0, 110.0, 120.0, 130.0, 140.0, 250.0, 280.0];
        let inputs = make_inputs(lons, 15.0);
        assert_eq!(stronger_rashi(0, 1, &inputs), 0); // Mesha has more occupants
    }

    #[test]
    fn lord_longitude_basic() {
        // Mesha lord = Mangal (index 2), at 75 deg
        let lons = [15.0, 45.0, 75.0, 105.0, 135.0, 165.0, 195.0, 225.0, 255.0];
        let inputs = make_inputs(lons, 15.0);
        assert!((inputs.lord_longitude(0) - 75.0).abs() < 1e-10);
    }

    #[test]
    fn house_from_lagna_same() {
        assert_eq!(house_from_lagna(0, 0), 1);
    }

    #[test]
    fn house_from_lagna_seventh() {
        assert_eq!(house_from_lagna(6, 0), 7);
    }

    #[test]
    fn house_from_lagna_wrap() {
        assert_eq!(house_from_lagna(0, 11), 2);
    }

    #[test]
    fn brahma_graha_basic() {
        // Venus at 15 (Mesha, odd, house 1), Jupiter at 45 (Vrishabha, even), Saturn at 195 (Tula, odd, house 7)
        let mut lons = [0.0; 9];
        lons[Graha::Shukra.index() as usize] = 15.0; // Mesha (odd, house 1)
        lons[Graha::Guru.index() as usize] = 45.0; // Vrishabha (even, fails)
        lons[Graha::Shani.index() as usize] = 195.0; // Tula (odd, house 7)
        let inputs = make_inputs(lons, 0.0); // Lagna at Mesha

        let brahma = brahma_graha(&inputs);
        // Saturn has higher lon than Venus among qualifiers
        assert_eq!(brahma, Graha::Shani);
    }

    #[test]
    fn atmakaraka_basic() {
        // Sun at 29 deg (29 in sign), Moon at 5 deg (5 in sign)
        let mut lons = [0.0; 9];
        lons[0] = 29.0; // Sun: 29 deg in Mesha
        lons[1] = 5.0; // Moon: 5 deg in Mesha
        lons[2] = 18.0; // Mars: 18 deg
        let inputs = make_inputs(lons, 0.0);
        assert_eq!(atmakaraka(&inputs), Graha::Surya);
    }

    #[test]
    fn rashi_name_valid() {
        assert_eq!(rashi_name(0), "Mesha");
        assert_eq!(rashi_name(11), "Meena");
    }

    #[test]
    fn rashi_name_invalid() {
        assert_eq!(rashi_name(12), "Unknown");
    }
}
