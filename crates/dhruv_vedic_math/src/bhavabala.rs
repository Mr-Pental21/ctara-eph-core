//! Bhava Bala (house strength) formulas from BPHS Chapter 27.

use crate::graha::{ALL_GRAHAS, Graha, rashi_lord_by_index};
use crate::graha_relationships::BeneficNature;
use crate::rashi::rashi_from_longitude;
use crate::util::normalize_360;

/// Birth-period bucket used by the Bhava Bala rising-type rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BhavaBalaBirthPeriod {
    Day,
    Twilight,
    Night,
}

/// Anchor used for the BPHS Bhava Dig Bala subtraction rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BhavaDigAnchor {
    Ascendant,
    Descendant,
    Meridian,
    Nadir,
}

/// Low-level inputs needed to compute all 12 Bhava Balas.
#[derive(Debug, Clone, Copy)]
pub struct BhavaBalaInputs {
    /// Sidereal bhava cusp longitudes, house 1..12 at indices 0..11.
    pub cusp_sidereal_lons: [f64; 12],
    /// Sidereal ascendant longitude.
    pub ascendant_sidereal_lon: f64,
    /// Sidereal meridian (MC) longitude.
    pub meridian_sidereal_lon: f64,
    /// Bhava number (1-12, or 0 if unknown) for each graha in ALL_GRAHAS order.
    pub graha_bhava_numbers: [u8; 9],
    /// House-lord Shadbala totals in virupas for each bhava.
    pub house_lord_strengths: [f64; 12],
    /// Total drishti virupas from each graha to each bhava cusp.
    pub aspect_virupas: [[f64; 12]; 9],
    /// Whether Rahu/Ketu aspects should be included as malefic Bhava Drishti contributors.
    pub include_node_aspects: bool,
    /// Whether occupation and rising special rules should contribute to total Bhava Bala.
    pub include_special_rules: bool,
    /// Birth-period bucket for the sign-type bonus.
    pub birth_period: BhavaBalaBirthPeriod,
}

/// Full Bhava Bala breakdown for one bhava.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BhavaBalaEntry {
    pub bhava_number: u8,
    pub cusp_sidereal_lon: f64,
    pub rashi_index: u8,
    pub lord: Graha,
    pub bhavadhipati: f64,
    pub dig: f64,
    pub drishti: f64,
    pub occupation_bonus: f64,
    pub rising_bonus: f64,
    pub total_virupas: f64,
    pub total_rupas: f64,
}

/// Bhava Bala result for all 12 houses.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BhavaBalaResult {
    pub entries: [BhavaBalaEntry; 12],
}

const BENEFIC_GRAHAS: [Graha; 2] = [Graha::Chandra, Graha::Shukra];
const MALEFIC_GRAHAS: [Graha; 5] = [
    Graha::Surya,
    Graha::Mangal,
    Graha::Shani,
    Graha::Rahu,
    Graha::Ketu,
];

const fn is_shirshodaya(rashi_index: u8) -> bool {
    matches!(rashi_index, 2 | 4 | 5 | 6 | 7 | 10)
}

const fn is_prishtodaya(rashi_index: u8) -> bool {
    matches!(rashi_index, 0 | 1 | 3 | 8 | 9)
}

const fn is_dual_rashi(rashi_index: u8) -> bool {
    matches!(rashi_index, 11)
}

/// Resolve the BPHS anchor bucket for a sidereal bhava cusp.
pub fn bhava_dig_anchor(cusp_sidereal_lon: f64) -> BhavaDigAnchor {
    let info = rashi_from_longitude(cusp_sidereal_lon);
    let deg_in_rashi = info.degrees_in_rashi;
    match info.rashi_index {
        5 | 2 | 6 | 10 => BhavaDigAnchor::Descendant,
        8 if deg_in_rashi < 15.0 => BhavaDigAnchor::Descendant,
        0 | 1 | 4 => BhavaDigAnchor::Nadir,
        8 => BhavaDigAnchor::Nadir,
        9 if deg_in_rashi < 15.0 => BhavaDigAnchor::Nadir,
        3 | 7 => BhavaDigAnchor::Ascendant,
        9 | 11 => BhavaDigAnchor::Meridian,
        _ => BhavaDigAnchor::Ascendant,
    }
}

/// Compute Bhava Dig Bala for one bhava cusp in virupas.
pub fn bhava_dig_bala(
    cusp_sidereal_lon: f64,
    ascendant_sidereal_lon: f64,
    meridian_sidereal_lon: f64,
) -> f64 {
    let descendant = normalize_360(ascendant_sidereal_lon + 180.0);
    let nadir = normalize_360(meridian_sidereal_lon + 180.0);
    let anchor = match bhava_dig_anchor(cusp_sidereal_lon) {
        BhavaDigAnchor::Ascendant => ascendant_sidereal_lon,
        BhavaDigAnchor::Descendant => descendant,
        BhavaDigAnchor::Meridian => meridian_sidereal_lon,
        BhavaDigAnchor::Nadir => nadir,
    };
    let mut diff = normalize_360(cusp_sidereal_lon - anchor);
    if diff > 180.0 {
        diff = 360.0 - diff;
    }
    diff / 3.0
}

/// Compute the BPHS sign-type bonus for one bhava cusp.
pub fn bhava_rising_bonus(cusp_sidereal_lon: f64, birth_period: BhavaBalaBirthPeriod) -> f64 {
    let rashi_index = rashi_from_longitude(cusp_sidereal_lon).rashi_index;
    let matches_bonus = match birth_period {
        BhavaBalaBirthPeriod::Day => is_shirshodaya(rashi_index),
        BhavaBalaBirthPeriod::Twilight => is_dual_rashi(rashi_index),
        BhavaBalaBirthPeriod::Night => is_prishtodaya(rashi_index),
    };
    if matches_bonus { 15.0 } else { 0.0 }
}

/// Compute the BPHS occupation adjustment for one bhava.
pub fn bhava_occupation_bonus(bhava_number: u8, graha_bhava_numbers: &[u8; 9]) -> f64 {
    if bhava_number == 0 || bhava_number > 12 {
        return 0.0;
    }
    let mut total = 0.0;
    for graha in ALL_GRAHAS {
        if graha_bhava_numbers[graha.index() as usize] != bhava_number {
            continue;
        }
        total += match graha {
            Graha::Guru | Graha::Buddh => 60.0,
            Graha::Shani | Graha::Mangal | Graha::Surya => -60.0,
            _ => 0.0,
        };
    }
    total
}

/// Compute Bhava Drishti Bala from total drishti virupas for one bhava cusp.
pub fn bhava_drishti_bala(aspect_virupas_for_bhava: &[f64; 9], include_node_aspects: bool) -> f64 {
    let mut total = 0.0;
    for graha in ALL_GRAHAS {
        if !include_node_aspects && matches!(graha, Graha::Rahu | Graha::Ketu) {
            continue;
        }
        let virupas = aspect_virupas_for_bhava[graha.index() as usize];
        total += match graha {
            Graha::Buddh | Graha::Guru => virupas,
            _ if BENEFIC_GRAHAS.contains(&graha) => virupas / 4.0,
            _ if MALEFIC_GRAHAS.contains(&graha) => -virupas / 4.0,
            _ => 0.0,
        };
    }
    total
}

/// Compute one Bhava Bala entry from low-level inputs.
pub fn bhava_bala_entry(inputs: &BhavaBalaInputs, index: usize) -> BhavaBalaEntry {
    let cusp_sidereal_lon = normalize_360(inputs.cusp_sidereal_lons[index]);
    let rashi_info = rashi_from_longitude(cusp_sidereal_lon);
    let lord = rashi_lord_by_index(rashi_info.rashi_index).expect("valid rashi index");

    let mut aspect_virupas_for_bhava = [0.0; 9];
    for (graha_index, slot) in aspect_virupas_for_bhava.iter_mut().enumerate() {
        *slot = inputs.aspect_virupas[graha_index][index];
    }

    let bhava_number = index as u8 + 1;
    let bhavadhipati = inputs.house_lord_strengths[index];
    let dig = bhava_dig_bala(
        cusp_sidereal_lon,
        inputs.ascendant_sidereal_lon,
        inputs.meridian_sidereal_lon,
    );
    let drishti = bhava_drishti_bala(&aspect_virupas_for_bhava, inputs.include_node_aspects);
    let occupation_bonus = bhava_occupation_bonus(bhava_number, &inputs.graha_bhava_numbers);
    let rising_bonus = bhava_rising_bonus(cusp_sidereal_lon, inputs.birth_period);
    let special = if inputs.include_special_rules {
        occupation_bonus + rising_bonus
    } else {
        0.0
    };
    let total_virupas = bhavadhipati + dig + drishti + special;

    BhavaBalaEntry {
        bhava_number,
        cusp_sidereal_lon,
        rashi_index: rashi_info.rashi_index,
        lord,
        bhavadhipati,
        dig,
        drishti,
        occupation_bonus,
        rising_bonus,
        total_virupas,
        total_rupas: total_virupas / 60.0,
    }
}

/// Compute Bhava Bala for all 12 houses from low-level inputs.
pub fn calculate_bhava_bala(inputs: &BhavaBalaInputs) -> BhavaBalaResult {
    let mut entries = [BhavaBalaEntry {
        bhava_number: 1,
        cusp_sidereal_lon: 0.0,
        rashi_index: 0,
        lord: Graha::Mangal,
        bhavadhipati: 0.0,
        dig: 0.0,
        drishti: 0.0,
        occupation_bonus: 0.0,
        rising_bonus: 0.0,
        total_virupas: 0.0,
        total_rupas: 0.0,
    }; 12];
    for (index, entry) in entries.iter_mut().enumerate() {
        *entry = bhava_bala_entry(inputs, index);
    }
    BhavaBalaResult { entries }
}

/// Natural benefic/malefic classification used by Bhava Drishti Bala.
pub const fn bhava_drishti_nature(graha: Graha) -> Option<BeneficNature> {
    match graha {
        Graha::Buddh | Graha::Guru => None,
        Graha::Chandra | Graha::Shukra => Some(BeneficNature::Benefic),
        Graha::Surya | Graha::Mangal | Graha::Shani | Graha::Rahu | Graha::Ketu => {
            Some(BeneficNature::Malefic)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const EPS: f64 = 1e-9;

    #[test]
    fn dig_anchor_groups_match_bphs_rules() {
        assert_eq!(bhava_dig_anchor(165.0), BhavaDigAnchor::Descendant); // Kanya
        assert_eq!(bhava_dig_anchor(250.0), BhavaDigAnchor::Descendant); // Dhanu 10 deg
        assert_eq!(bhava_dig_anchor(20.0), BhavaDigAnchor::Nadir); // Mesha
        assert_eq!(bhava_dig_anchor(275.0), BhavaDigAnchor::Nadir); // Makara 5 deg
        assert_eq!(bhava_dig_anchor(100.0), BhavaDigAnchor::Ascendant); // Karka
        assert_eq!(bhava_dig_anchor(290.0), BhavaDigAnchor::Meridian); // Makara 20 deg
        assert_eq!(bhava_dig_anchor(340.0), BhavaDigAnchor::Meridian); // Meena
    }

    #[test]
    fn dig_bala_uses_smallest_angular_separation() {
        let dig = bhava_dig_bala(340.0, 15.0, 105.0);
        assert!((dig - (125.0 / 3.0)).abs() < EPS);
    }

    #[test]
    fn rising_bonus_matches_birth_period_rule() {
        assert!((bhava_rising_bonus(65.0, BhavaBalaBirthPeriod::Day) - 15.0).abs() < EPS); // Mithuna
        assert!(bhava_rising_bonus(65.0, BhavaBalaBirthPeriod::Twilight).abs() < EPS);
        assert!(bhava_rising_bonus(65.0, BhavaBalaBirthPeriod::Night).abs() < EPS);
        assert!((bhava_rising_bonus(250.0, BhavaBalaBirthPeriod::Night) - 15.0).abs() < EPS); // Dhanu
        assert!((bhava_rising_bonus(340.0, BhavaBalaBirthPeriod::Twilight) - 15.0).abs() < EPS); // Meena
    }

    #[test]
    fn occupation_bonus_stacks_positive_and_negative_planets() {
        let mut bhavas = [0u8; 9];
        bhavas[Graha::Guru.index() as usize] = 4;
        bhavas[Graha::Buddh.index() as usize] = 4;
        bhavas[Graha::Surya.index() as usize] = 4;
        bhavas[Graha::Mangal.index() as usize] = 4;
        assert!((bhava_occupation_bonus(4, &bhavas) - 0.0).abs() < EPS);
    }

    #[test]
    fn drishti_bala_applies_full_and_quarter_rules() {
        let mut virupas = [0.0; 9];
        virupas[Graha::Guru.index() as usize] = 40.0;
        virupas[Graha::Buddh.index() as usize] = 20.0;
        virupas[Graha::Chandra.index() as usize] = 16.0;
        virupas[Graha::Surya.index() as usize] = 12.0;
        virupas[Graha::Rahu.index() as usize] = 8.0;
        let result = bhava_drishti_bala(&virupas, true);
        assert!((result - (40.0 + 20.0 + 4.0 - 3.0 - 2.0)).abs() < EPS);
    }

    #[test]
    fn drishti_bala_excludes_nodes_by_default_policy() {
        let mut virupas = [0.0; 9];
        virupas[Graha::Guru.index() as usize] = 40.0;
        virupas[Graha::Rahu.index() as usize] = 20.0;
        virupas[Graha::Ketu.index() as usize] = 12.0;

        let without_nodes = bhava_drishti_bala(&virupas, false);
        let with_nodes = bhava_drishti_bala(&virupas, true);

        assert!((without_nodes - 40.0).abs() < EPS);
        assert!((with_nodes - (40.0 - 5.0 - 3.0)).abs() < EPS);
    }

    #[test]
    fn calculate_bhava_bala_total_includes_special_rules_when_enabled() {
        let mut aspect_virupas = [[0.0; 12]; 9];
        aspect_virupas[Graha::Guru.index() as usize][0] = 30.0;
        aspect_virupas[Graha::Surya.index() as usize][0] = 20.0;

        let mut graha_bhava_numbers = [0u8; 9];
        graha_bhava_numbers[Graha::Guru.index() as usize] = 1;

        let mut house_lord_strengths = [0.0; 12];
        house_lord_strengths[0] = 300.0;

        let result = calculate_bhava_bala(&BhavaBalaInputs {
            cusp_sidereal_lons: [65.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
            ascendant_sidereal_lon: 15.0,
            meridian_sidereal_lon: 105.0,
            graha_bhava_numbers,
            house_lord_strengths,
            aspect_virupas,
            include_node_aspects: false,
            include_special_rules: true,
            birth_period: BhavaBalaBirthPeriod::Day,
        });

        let entry = result.entries[0];
        assert_eq!(entry.bhava_number, 1);
        assert_eq!(entry.rashi_index, 2);
        assert_eq!(entry.lord, Graha::Buddh);
        assert!((entry.bhavadhipati - 300.0).abs() < EPS);
        assert!((entry.drishti - 25.0).abs() < EPS);
        assert!((entry.occupation_bonus - 60.0).abs() < EPS);
        assert!((entry.rising_bonus - 15.0).abs() < EPS);
        assert!(
            (entry.total_virupas
                - (entry.bhavadhipati
                    + entry.dig
                    + entry.drishti
                    + entry.occupation_bonus
                    + entry.rising_bonus))
                .abs()
                < EPS
        );
        assert!((entry.total_rupas * 60.0 - entry.total_virupas).abs() < EPS);
    }

    #[test]
    fn calculate_bhava_bala_can_exclude_special_rules_from_total() {
        let mut graha_bhava_numbers = [0u8; 9];
        graha_bhava_numbers[Graha::Guru.index() as usize] = 1;

        let mut house_lord_strengths = [0.0; 12];
        house_lord_strengths[0] = 300.0;

        let result = calculate_bhava_bala(&BhavaBalaInputs {
            cusp_sidereal_lons: [65.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
            ascendant_sidereal_lon: 15.0,
            meridian_sidereal_lon: 105.0,
            graha_bhava_numbers,
            house_lord_strengths,
            aspect_virupas: [[0.0; 12]; 9],
            include_node_aspects: false,
            include_special_rules: false,
            birth_period: BhavaBalaBirthPeriod::Day,
        });

        let entry = result.entries[0];
        assert!((entry.occupation_bonus - 60.0).abs() < EPS);
        assert!((entry.rising_bonus - 15.0).abs() < EPS);
        assert!(
            (entry.total_virupas - (entry.bhavadhipati + entry.dig + entry.drishti)).abs() < EPS
        );
    }
}
