//! Configuration data for the Yogini dasha system.
//!
//! 8 Yoginis, 36-year total cycle.
//! Provenance: BPHS chapter on Yogini dasha. See docs/clean_room_dasha.md.

use crate::graha::Graha;

use super::types::{DAYS_PER_YEAR, DashaEntity};
use super::variation::SubPeriodMethod;

/// Yogini names (0-indexed).
pub const YOGINI_NAMES: [&str; 8] = [
    "Mangala", "Pingala", "Dhanya", "Bhramari", "Bhadrika", "Ulka", "Siddha", "Sankata",
];

/// Graha lord for each Yogini (default scheme).
pub const YOGINI_GRAHAS: [Graha; 8] = [
    Graha::Chandra,
    Graha::Surya,
    Graha::Guru,
    Graha::Mangal,
    Graha::Buddh,
    Graha::Shani,
    Graha::Shukra,
    Graha::Rahu,
];

/// Periods in years (1..8).
const YOGINI_YEARS: [f64; 8] = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0];

/// Nakshatra (0-26) → Yogini index (0-7).
///
/// Formula: yogini_idx = ((nakshatra_1_indexed + 3) % 8), where 0 maps to 7.
/// Equivalently, the pattern repeats every 8 nakshatras starting from Ardra(5)=0.
pub const YOGINI_NAK_MAP: [u8; 8] = [3, 4, 5, 6, 7, 0, 1, 2];

/// Configuration for the Yogini dasha system.
#[derive(Debug, Clone)]
pub struct YoginiDashaConfig {
    /// Yogini sequence (8 entries).
    pub yogini_sequence: Vec<DashaEntity>,
    /// Period in days for each yogini.
    pub periods_days: Vec<f64>,
    /// Total period in days.
    pub total_period_days: f64,
    /// Nakshatra (0-26) → yogini index (0-7).
    pub nakshatra_to_yogini_idx: [u8; 27],
    /// Default sub-period method.
    pub default_method: SubPeriodMethod,
}

impl YoginiDashaConfig {
    /// Get the entity/period pairs as a flat sequence.
    pub fn entity_sequence(&self) -> Vec<(DashaEntity, f64)> {
        self.yogini_sequence
            .iter()
            .zip(self.periods_days.iter())
            .map(|(&e, &p)| (e, p))
            .collect()
    }

    /// Get the starting yogini index for a given nakshatra.
    pub fn starting_yogini_idx(&self, nakshatra_index: u8) -> u8 {
        self.nakshatra_to_yogini_idx[nakshatra_index.min(26) as usize]
    }

    /// Get the entry period in days for the starting yogini of a nakshatra.
    pub fn entry_period_days(&self, nakshatra_index: u8) -> f64 {
        let yi = self.starting_yogini_idx(nakshatra_index) as usize;
        self.periods_days[yi]
    }
}

/// Build the 27-nakshatra-to-yogini mapping.
fn build_nak_map() -> [u8; 27] {
    let mut map = [0u8; 27];
    for (i, slot) in map.iter_mut().enumerate() {
        // 1-indexed nakshatra: nak_1 = i + 1
        // remainder = (nak_1 + 3) % 8
        // yogini_idx = if remainder == 0 { 7 } else { remainder - 1 }
        let nak_1 = (i + 1) as u8;
        let remainder = (nak_1 + 3) % 8;
        *slot = if remainder == 0 { 7 } else { remainder - 1 };
    }
    map
}

/// Create the default Yogini dasha configuration.
pub fn yogini_config() -> YoginiDashaConfig {
    let periods_days: Vec<f64> = YOGINI_YEARS.iter().map(|&y| y * DAYS_PER_YEAR).collect();
    let total = periods_days.iter().sum();
    let yogini_sequence: Vec<DashaEntity> = (0u8..8).map(DashaEntity::Yogini).collect();

    YoginiDashaConfig {
        yogini_sequence,
        periods_days,
        total_period_days: total,
        nakshatra_to_yogini_idx: build_nak_map(),
        default_method: SubPeriodMethod::ProportionalFromParent,
    }
}

/// Get the Yogini name for a 0-based index.
pub fn yogini_name(idx: u8) -> &'static str {
    if (idx as usize) < YOGINI_NAMES.len() {
        YOGINI_NAMES[idx as usize]
    } else {
        "Unknown"
    }
}

/// Get the graha lord for a Yogini (default scheme).
pub fn yogini_graha(idx: u8) -> Option<Graha> {
    YOGINI_GRAHAS.get(idx as usize).copied()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn yogini_total_36_years() {
        let cfg = yogini_config();
        let total_years = cfg.total_period_days / DAYS_PER_YEAR;
        assert!((total_years - 36.0).abs() < 1e-10);
    }

    #[test]
    fn yogini_8_entities() {
        let cfg = yogini_config();
        assert_eq!(cfg.yogini_sequence.len(), 8);
        assert_eq!(cfg.periods_days.len(), 8);
    }

    #[test]
    fn yogini_ardra_maps_to_mangala() {
        let cfg = yogini_config();
        assert_eq!(cfg.starting_yogini_idx(5), 0); // Ardra → Mangala (index 0)
    }

    #[test]
    fn yogini_ashwini_maps_to_bhramari() {
        let cfg = yogini_config();
        assert_eq!(cfg.starting_yogini_idx(0), 3); // Ashwini → Bhramari (index 3)
    }

    #[test]
    fn yogini_mrigashira_maps_to_sankata() {
        let cfg = yogini_config();
        assert_eq!(cfg.starting_yogini_idx(4), 7); // Mrigashira → Sankata (index 7)
    }

    #[test]
    fn yogini_all_27_mapped_to_valid() {
        let cfg = yogini_config();
        for (i, &yi) in cfg.nakshatra_to_yogini_idx.iter().enumerate() {
            assert!(yi < 8, "Nakshatra {} mapped to invalid yogini {}", i, yi);
        }
    }

    #[test]
    fn yogini_mangala_1_year() {
        let cfg = yogini_config();
        let entry = cfg.entry_period_days(5); // Ardra → Mangala → 1 year
        let years = entry / DAYS_PER_YEAR;
        assert!((years - 1.0).abs() < 1e-10);
    }

    #[test]
    fn yogini_name_lookup() {
        assert_eq!(yogini_name(0), "Mangala");
        assert_eq!(yogini_name(7), "Sankata");
        assert_eq!(yogini_name(8), "Unknown");
    }
}
