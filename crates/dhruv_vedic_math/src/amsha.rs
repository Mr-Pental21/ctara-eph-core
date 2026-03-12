//! Amsha (divisional chart / varga chart) calculations.
//!
//! Transforms a sidereal longitude through a divisional mapping to produce
//! a new longitude in the amsha chart. Each amsha divides the 30-degree rashi
//! span into N equal parts and maps each part to a target rashi.
//!
//! Clean-room implementation from BPHS Shodashavarga definitions.
//! See `docs/clean_room_amsha.md`.

use crate::rashi::{RashiInfo, rashi_from_longitude};
use crate::util::normalize_360;

// ---------------------------------------------------------------------------
// Rashi element classification
// ---------------------------------------------------------------------------

/// Rashi element classification (for FEAW-based starting rashi).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RashiElement {
    Fire,
    Earth,
    Air,
    Water,
}

/// Determine the element of a rashi by 0-based index.
///
/// Fire: 0,4,8 (Mesha, Simha, Dhanu)
/// Earth: 1,5,9 (Vrishabha, Kanya, Makara)
/// Air: 2,6,10 (Mithuna, Tula, Kumbha)
/// Water: 3,7,11 (Karka, Vrischika, Meena)
pub fn rashi_element(rashi_index: u8) -> RashiElement {
    match rashi_index % 4 {
        0 => RashiElement::Fire,
        1 => RashiElement::Earth,
        2 => RashiElement::Air,
        3 => RashiElement::Water,
        _ => unreachable!(),
    }
}

// ---------------------------------------------------------------------------
// Amsha enum (34 variants)
// ---------------------------------------------------------------------------

/// 34 supported divisional charts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Amsha {
    D1,
    D2,
    D3,
    D4,
    D5,
    D6,
    D7,
    D8,
    D9,
    D10,
    D11,
    D12,
    D15,
    D16,
    D18,
    D20,
    D21,
    D22,
    D24,
    D25,
    D27,
    D28,
    D30,
    D36,
    D40,
    D45,
    D48,
    D50,
    D54,
    D60,
    D72,
    D81,
    D108,
    D144,
}

/// All 34 amshas in order.
pub const ALL_AMSHAS: [Amsha; 34] = [
    Amsha::D1,
    Amsha::D2,
    Amsha::D3,
    Amsha::D4,
    Amsha::D5,
    Amsha::D6,
    Amsha::D7,
    Amsha::D8,
    Amsha::D9,
    Amsha::D10,
    Amsha::D11,
    Amsha::D12,
    Amsha::D15,
    Amsha::D16,
    Amsha::D18,
    Amsha::D20,
    Amsha::D21,
    Amsha::D22,
    Amsha::D24,
    Amsha::D25,
    Amsha::D27,
    Amsha::D28,
    Amsha::D30,
    Amsha::D36,
    Amsha::D40,
    Amsha::D45,
    Amsha::D48,
    Amsha::D50,
    Amsha::D54,
    Amsha::D60,
    Amsha::D72,
    Amsha::D81,
    Amsha::D108,
    Amsha::D144,
];

/// Standard 16 Shodashavarga charts from BPHS.
pub const SHODASHAVARGA: [Amsha; 16] = [
    Amsha::D1,
    Amsha::D2,
    Amsha::D3,
    Amsha::D4,
    Amsha::D7,
    Amsha::D9,
    Amsha::D10,
    Amsha::D12,
    Amsha::D16,
    Amsha::D20,
    Amsha::D24,
    Amsha::D27,
    Amsha::D30,
    Amsha::D40,
    Amsha::D45,
    Amsha::D60,
];

impl Amsha {
    /// Number of divisions per rashi.
    pub const fn divisions(self) -> u16 {
        match self {
            Self::D1 => 1,
            Self::D2 => 2,
            Self::D3 => 3,
            Self::D4 => 4,
            Self::D5 => 5,
            Self::D6 => 6,
            Self::D7 => 7,
            Self::D8 => 8,
            Self::D9 => 9,
            Self::D10 => 10,
            Self::D11 => 11,
            Self::D12 => 12,
            Self::D15 => 15,
            Self::D16 => 16,
            Self::D18 => 18,
            Self::D20 => 20,
            Self::D21 => 21,
            Self::D22 => 22,
            Self::D24 => 24,
            Self::D25 => 25,
            Self::D27 => 27,
            Self::D28 => 28,
            Self::D30 => 30,
            Self::D36 => 36,
            Self::D40 => 40,
            Self::D45 => 45,
            Self::D48 => 48,
            Self::D50 => 50,
            Self::D54 => 54,
            Self::D60 => 60,
            Self::D72 => 72,
            Self::D81 => 81,
            Self::D108 => 108,
            Self::D144 => 144,
        }
    }

    /// Numeric D-number code.
    pub const fn code(self) -> u16 {
        self.divisions()
    }

    /// Display name.
    pub const fn name(self) -> &'static str {
        match self {
            Self::D1 => "D1_Rashi",
            Self::D2 => "D2_Hora",
            Self::D3 => "D3_Drekkana",
            Self::D4 => "D4_Chaturthamsha",
            Self::D5 => "D5_Panchamsha",
            Self::D6 => "D6_Shashthamsha",
            Self::D7 => "D7_Saptamsha",
            Self::D8 => "D8_Ashtamsha",
            Self::D9 => "D9_Navamsha",
            Self::D10 => "D10_Dashamsha",
            Self::D11 => "D11_Rudramsha",
            Self::D12 => "D12_Dwadashamsha",
            Self::D15 => "D15_Panchadashamsha",
            Self::D16 => "D16_Shodashamsha",
            Self::D18 => "D18_Ashtadashamsha",
            Self::D20 => "D20_Vimshamsha",
            Self::D21 => "D21_Ekavimshamsha",
            Self::D22 => "D22_Dwavimshamsha",
            Self::D24 => "D24_Chaturvimshamsha",
            Self::D25 => "D25_Panchavimshamsha",
            Self::D27 => "D27_Bhamsha",
            Self::D28 => "D28_Ashtavimshamsha",
            Self::D30 => "D30_Trimshamsha",
            Self::D36 => "D36_Shattrimshamsha",
            Self::D40 => "D40_Khavedamsha",
            Self::D45 => "D45_Akshavedamsha",
            Self::D48 => "D48_Ashtachatvarimsha",
            Self::D50 => "D50_Panchashatamsha",
            Self::D54 => "D54_Chatushpanchashamsha",
            Self::D60 => "D60_Shashtiamsha",
            Self::D72 => "D72_Dvasaptatiamsha",
            Self::D81 => "D81_Navamsha81",
            Self::D108 => "D108_Ashtottaramsha",
            Self::D144 => "D144_Dwadashashtottaramsha",
        }
    }

    /// Sanskrit name.
    pub const fn sanskrit_name(self) -> &'static str {
        match self {
            Self::D1 => "Rashi",
            Self::D2 => "Hora",
            Self::D3 => "Drekkana",
            Self::D4 => "Chaturthamsha",
            Self::D5 => "Panchamsha",
            Self::D6 => "Shashthamsha",
            Self::D7 => "Saptamsha",
            Self::D8 => "Ashtamsha",
            Self::D9 => "Navamsha",
            Self::D10 => "Dashamsha",
            Self::D11 => "Rudramsha",
            Self::D12 => "Dwadashamsha",
            Self::D15 => "Panchadashamsha",
            Self::D16 => "Shodashamsha",
            Self::D18 => "Ashtadashamsha",
            Self::D20 => "Vimshamsha",
            Self::D21 => "Ekavimshamsha",
            Self::D22 => "Dwavimshamsha",
            Self::D24 => "Chaturvimshamsha",
            Self::D25 => "Panchavimshamsha",
            Self::D27 => "Bhamsha",
            Self::D28 => "Ashtavimshamsha",
            Self::D30 => "Trimshamsha",
            Self::D36 => "Shattrimshamsha",
            Self::D40 => "Khavedamsha",
            Self::D45 => "Akshavedamsha",
            Self::D48 => "Ashtachatvarimsha",
            Self::D50 => "Panchashatamsha",
            Self::D54 => "Chatushpanchashamsha",
            Self::D60 => "Shashtiamsha",
            Self::D72 => "Dvasaptatiamsha",
            Self::D81 => "Navamsha81",
            Self::D108 => "Ashtottaramsha",
            Self::D144 => "Dwadashashtottaramsha",
        }
    }

    /// 0-based index into ALL_AMSHAS (0-33).
    pub const fn index(self) -> u8 {
        match self {
            Self::D1 => 0,
            Self::D2 => 1,
            Self::D3 => 2,
            Self::D4 => 3,
            Self::D5 => 4,
            Self::D6 => 5,
            Self::D7 => 6,
            Self::D8 => 7,
            Self::D9 => 8,
            Self::D10 => 9,
            Self::D11 => 10,
            Self::D12 => 11,
            Self::D15 => 12,
            Self::D16 => 13,
            Self::D18 => 14,
            Self::D20 => 15,
            Self::D21 => 16,
            Self::D22 => 17,
            Self::D24 => 18,
            Self::D25 => 19,
            Self::D27 => 20,
            Self::D28 => 21,
            Self::D30 => 22,
            Self::D36 => 23,
            Self::D40 => 24,
            Self::D45 => 25,
            Self::D48 => 26,
            Self::D50 => 27,
            Self::D54 => 28,
            Self::D60 => 29,
            Self::D72 => 30,
            Self::D81 => 31,
            Self::D108 => 32,
            Self::D144 => 33,
        }
    }

    /// Reverse lookup from D-number code.
    pub fn from_code(code: u16) -> Option<Amsha> {
        match code {
            1 => Some(Self::D1),
            2 => Some(Self::D2),
            3 => Some(Self::D3),
            4 => Some(Self::D4),
            5 => Some(Self::D5),
            6 => Some(Self::D6),
            7 => Some(Self::D7),
            8 => Some(Self::D8),
            9 => Some(Self::D9),
            10 => Some(Self::D10),
            11 => Some(Self::D11),
            12 => Some(Self::D12),
            15 => Some(Self::D15),
            16 => Some(Self::D16),
            18 => Some(Self::D18),
            20 => Some(Self::D20),
            21 => Some(Self::D21),
            22 => Some(Self::D22),
            24 => Some(Self::D24),
            25 => Some(Self::D25),
            27 => Some(Self::D27),
            28 => Some(Self::D28),
            30 => Some(Self::D30),
            36 => Some(Self::D36),
            40 => Some(Self::D40),
            45 => Some(Self::D45),
            48 => Some(Self::D48),
            50 => Some(Self::D50),
            54 => Some(Self::D54),
            60 => Some(Self::D60),
            72 => Some(Self::D72),
            81 => Some(Self::D81),
            108 => Some(Self::D108),
            144 => Some(Self::D144),
            _ => None,
        }
    }
}

// ---------------------------------------------------------------------------
// Variation
// ---------------------------------------------------------------------------

/// Variation selection for amsha computation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum AmshaVariation {
    /// Traditional Parashari method (default for all amshas).
    #[default]
    TraditionalParashari,
    /// D2 only: odd rashi -> Cancer/Leo, even rashi -> Leo/Cancer.
    HoraCancerLeoOnly,
}

impl AmshaVariation {
    /// Reverse lookup from variation code.
    pub fn from_code(code: u8) -> Option<AmshaVariation> {
        match code {
            0 => Some(Self::TraditionalParashari),
            1 => Some(Self::HoraCancerLeoOnly),
            _ => None,
        }
    }

    /// Check if this variation is applicable to the given amsha.
    pub fn is_applicable_to(self, amsha: Amsha) -> bool {
        match self {
            Self::TraditionalParashari => true,
            Self::HoraCancerLeoOnly => amsha == Amsha::D2,
        }
    }
}

// ---------------------------------------------------------------------------
// AmshaRequest
// ---------------------------------------------------------------------------

/// Request for batch amsha operations.
#[derive(Debug, Clone, Copy)]
pub struct AmshaRequest {
    pub amsha: Amsha,
    pub variation: Option<AmshaVariation>,
}

impl AmshaRequest {
    /// Create a request with the default variation.
    pub fn new(amsha: Amsha) -> Self {
        Self {
            amsha,
            variation: None,
        }
    }

    /// Create a request with a specific variation.
    pub fn with_variation(amsha: Amsha, variation: AmshaVariation) -> Self {
        Self {
            amsha,
            variation: Some(variation),
        }
    }

    /// Resolve None to the default variation.
    pub fn effective_variation(&self) -> AmshaVariation {
        self.variation
            .unwrap_or(AmshaVariation::TraditionalParashari)
    }
}

// ---------------------------------------------------------------------------
// Sequence table (internal)
// ---------------------------------------------------------------------------

/// Determine the target rashi index for a given amsha division.
///
/// # Arguments
/// * `amsha` - The divisional chart type
/// * `variation` - Which variation to use
/// * `natal_rashi_idx` - 0-based rashi index of the natal position (0=Mesha..11=Meena)
/// * `div_idx` - 0-based division index within the rashi
fn amsha_target_rashi(
    amsha: Amsha,
    variation: AmshaVariation,
    natal_rashi_idx: u8,
    div_idx: u16,
) -> u8 {
    match amsha {
        // D1: identity
        Amsha::D1 => natal_rashi_idx,

        // D2: two variations
        Amsha::D2 => match variation {
            AmshaVariation::HoraCancerLeoOnly => {
                // Odd rashi (1-based 1,3,5,7,9,11 = 0-based 0,2,4,6,8,10)
                let is_odd_rashi = natal_rashi_idx.is_multiple_of(2);
                if is_odd_rashi {
                    if div_idx == 0 { 3 } else { 4 } // Cancer, Leo
                } else if div_idx == 0 {
                    4 // Leo
                } else {
                    3 // Cancer
                }
            }
            AmshaVariation::TraditionalParashari => {
                // Zodiac cycling: start = (rashi * 2) % 12
                let start = (natal_rashi_idx as u16 * 2) % 12;
                ((start + div_idx) % 12) as u8
            }
        },

        // D3: trine progression (+4 step)
        Amsha::D3 => {
            let start = natal_rashi_idx as u16;
            ((start + div_idx * 4) % 12) as u8
        }

        // INCREMENT amshas: odd rashi = natal start, even rashi = natal + offset
        Amsha::D7 => increment_start(natal_rashi_idx, div_idx, 6),
        Amsha::D10 => increment_start(natal_rashi_idx, div_idx, 8),
        Amsha::D24 => increment_start(natal_rashi_idx, div_idx, 4),
        Amsha::D40 => increment_start(natal_rashi_idx, div_idx, 6),

        // FEAW amshas: element-based fixed starting rashi
        Amsha::D9 | Amsha::D60 => {
            let start = match rashi_element(natal_rashi_idx) {
                RashiElement::Fire => 0,  // Mesha
                RashiElement::Earth => 9, // Makara
                RashiElement::Air => 6,   // Tula
                RashiElement::Water => 3, // Karka
            };
            ((start + div_idx) % 12) as u8
        }
        Amsha::D16 => {
            let start: u16 = match rashi_element(natal_rashi_idx) {
                RashiElement::Fire => 0,  // Mesha
                RashiElement::Earth => 4, // Simha
                RashiElement::Air => 8,   // Dhanu
                RashiElement::Water => 0, // Mesha
            };
            ((start + div_idx) % 12) as u8
        }
        Amsha::D20 => {
            let start: u16 = match rashi_element(natal_rashi_idx) {
                RashiElement::Fire => 0,  // Mesha
                RashiElement::Earth => 8, // Dhanu
                RashiElement::Air => 4,   // Simha
                RashiElement::Water => 0, // Mesha
            };
            ((start + div_idx) % 12) as u8
        }

        // D30: odd rashi starts from Mesha(0), even rashi from Meena(11)
        Amsha::D30 => {
            let is_odd = natal_rashi_idx.is_multiple_of(2); // 0-indexed: 0,2,4.. are odd (1-based)
            let start: u16 = if is_odd { 0 } else { 11 };
            ((start + div_idx) % 12) as u8
        }

        // FIXED(0): start from natal rashi, step +1
        Amsha::D4
        | Amsha::D5
        | Amsha::D6
        | Amsha::D8
        | Amsha::D11
        | Amsha::D12
        | Amsha::D15
        | Amsha::D18
        | Amsha::D21
        | Amsha::D22
        | Amsha::D25
        | Amsha::D27
        | Amsha::D28
        | Amsha::D36
        | Amsha::D45
        | Amsha::D48
        | Amsha::D50
        | Amsha::D54
        | Amsha::D72
        | Amsha::D81
        | Amsha::D108
        | Amsha::D144 => {
            let start = natal_rashi_idx as u16;
            ((start + div_idx) % 12) as u8
        }
    }
}

/// Helper for INCREMENT amshas: odd rashi starts from natal, even from natal+offset.
fn increment_start(natal_rashi_idx: u8, div_idx: u16, even_offset: u16) -> u8 {
    // 0-indexed: 0,2,4,6,8,10 are odd rashis (1-based 1,3,5,7,9,11)
    let is_odd = natal_rashi_idx.is_multiple_of(2);
    let start = if is_odd {
        natal_rashi_idx as u16
    } else {
        (natal_rashi_idx as u16 + even_offset) % 12
    };
    ((start + div_idx) % 12) as u8
}

// ---------------------------------------------------------------------------
// Core transformation
// ---------------------------------------------------------------------------

/// Transform a sidereal longitude through an amsha division.
///
/// Returns amsha-transformed sidereal longitude in [0, 360).
pub fn amsha_longitude(sidereal_lon: f64, amsha: Amsha, variation: Option<AmshaVariation>) -> f64 {
    let variation = variation.unwrap_or(AmshaVariation::TraditionalParashari);

    // D1: identity
    if amsha == Amsha::D1 {
        return normalize_360(sidereal_lon);
    }

    let lon = normalize_360(sidereal_lon);
    let rashi_idx = (lon / 30.0).floor().min(11.0) as u8;
    let pos_in_rashi = lon - rashi_idx as f64 * 30.0;
    let total_divisions = amsha.divisions();
    let deg_per_div = 30.0 / total_divisions as f64;

    // Division index (clamped to valid range)
    let div_idx = ((pos_in_rashi / deg_per_div).floor() as u16).min(total_divisions - 1);

    // Target rashi from sequence table
    let target_rashi_idx = amsha_target_rashi(amsha, variation, rashi_idx, div_idx);

    // Scale position within division to 0-30 range
    let pos_in_div = pos_in_rashi - div_idx as f64 * deg_per_div;
    let scaled_pos = pos_in_div / deg_per_div * 30.0;

    (target_rashi_idx as f64 * 30.0 + scaled_pos) % 360.0
}

/// Batch: one longitude through multiple amshas.
pub fn amsha_longitudes(sidereal_lon: f64, requests: &[AmshaRequest]) -> Vec<f64> {
    requests
        .iter()
        .map(|req| amsha_longitude(sidereal_lon, req.amsha, req.variation))
        .collect()
}

// ---------------------------------------------------------------------------
// DMS-first APIs
// ---------------------------------------------------------------------------

/// Convert rashi index + degrees-in-rashi to absolute sidereal longitude.
pub fn rashi_position_to_longitude(rashi_index: u8, degrees_in_rashi: f64) -> f64 {
    normalize_360(rashi_index as f64 * 30.0 + degrees_in_rashi)
}

/// Transform from rashi position input, return full RashiInfo output.
pub fn amsha_from_rashi_position(
    rashi_index: u8,
    degrees_in_rashi: f64,
    amsha: Amsha,
    variation: Option<AmshaVariation>,
) -> RashiInfo {
    let lon = rashi_position_to_longitude(rashi_index, degrees_in_rashi);
    amsha_rashi_info(lon, amsha, variation)
}

/// Transform from sidereal longitude, return full RashiInfo.
pub fn amsha_rashi_info(
    sidereal_lon: f64,
    amsha: Amsha,
    variation: Option<AmshaVariation>,
) -> RashiInfo {
    let amsha_lon = amsha_longitude(sidereal_lon, amsha, variation);
    rashi_from_longitude(amsha_lon)
}

/// Batch: one longitude through multiple amshas, returning RashiInfo for each.
pub fn amsha_rashi_infos(sidereal_lon: f64, requests: &[AmshaRequest]) -> Vec<RashiInfo> {
    requests
        .iter()
        .map(|req| amsha_rashi_info(sidereal_lon, req.amsha, req.variation))
        .collect()
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::deg_to_dms;

    #[test]
    fn all_amshas_count() {
        assert_eq!(ALL_AMSHAS.len(), 34);
    }

    #[test]
    fn shodashavarga_count() {
        assert_eq!(SHODASHAVARGA.len(), 16);
    }

    #[test]
    fn d1_identity() {
        for i in 0..12 {
            let lon = i as f64 * 30.0 + 15.0;
            let result = amsha_longitude(lon, Amsha::D1, None);
            assert!(
                (result - lon).abs() < 1e-10,
                "D1 identity failed for lon={lon}"
            );
        }
    }

    #[test]
    fn d9_navamsha_fire_rashi() {
        // Mesha (0, Fire) at 5.0 deg: fire→start Mesha(0)
        // div_idx = floor(5.0 / 3.333...) = 1
        // target = (0+1) % 12 = 1 (Vrishabha)
        // scaled = (5.0 - 3.333...) / 3.333... * 30 = 15.0
        // result = 30 + 15 = 45.0 (Vrishabha 15°)
        let result = amsha_longitude(5.0, Amsha::D9, None);
        assert!((result - 45.0).abs() < 0.01, "D9 fire: got {result}");
    }

    #[test]
    fn d9_navamsha_earth_rashi() {
        // Vrishabha (1, Earth) at 45.5 deg: pos_in_rashi=15.5
        // earth → start Makara(9)
        // div_idx = floor(15.5 / 3.333...) = 4
        // target = (9+4) % 12 = 1 (Vrishabha)
        // scaled = (15.5 - 13.333...) / 3.333... * 30 = 19.5
        // result = 30 + 19.5 = 49.5
        let result = amsha_longitude(45.5, Amsha::D9, None);
        assert!((result - 49.5).abs() < 0.01, "D9 earth: got {result}");
    }

    #[test]
    fn d9_navamsha_air_rashi() {
        // Mithuna (2, Air) at 60.0 deg: pos_in_rashi=0
        // air → start Tula(6)
        // div_idx = 0, target = 6 (Tula)
        // scaled = 0
        // result = 180.0 (Tula 0°)
        let result = amsha_longitude(60.0, Amsha::D9, None);
        assert!((result - 180.0).abs() < 0.01, "D9 air: got {result}");
    }

    #[test]
    fn d9_navamsha_water_rashi() {
        // Karka (3, Water) at 90.0 deg: pos_in_rashi=0
        // water → start Karka(3)
        // div_idx = 0, target = 3 (Karka)
        // result = 90.0
        let result = amsha_longitude(90.0, Amsha::D9, None);
        assert!((result - 90.0).abs() < 0.01, "D9 water: got {result}");
    }

    #[test]
    fn d2_standard() {
        // Vrishabha (1) at 45.5 deg: pos_in_rashi=15.5
        // start = (1*2)%12 = 2, deg_per_div=15
        // div_idx = floor(15.5/15) = 1
        // target = (2+1)%12 = 3 (Karka)
        // scaled = (15.5-15)/15 * 30 = 1.0
        // result = 90 + 1 = 91.0
        let result = amsha_longitude(45.5, Amsha::D2, None);
        assert!((result - 91.0).abs() < 0.01, "D2 standard: got {result}");
    }

    #[test]
    fn d2_cancer_leo_only() {
        // Mesha (0, odd 1-based) at 10.0: pos_in_rashi=10
        // odd → first half Cancer(3), second half Leo(4)
        // div_idx = floor(10/15) = 0
        // target = 3 (Karka)
        // scaled = 10/15 * 30 = 20.0
        // result = 90 + 20 = 110.0
        let result = amsha_longitude(10.0, Amsha::D2, Some(AmshaVariation::HoraCancerLeoOnly));
        assert!(
            (result - 110.0).abs() < 0.01,
            "D2 cancer-leo odd: got {result}"
        );

        // Vrishabha (1, even 1-based) at 40.0: pos_in_rashi=10
        // even → first half Leo(4), second half Cancer(3)
        // div_idx = 0, target = 4 (Simha)
        // scaled = 10/15 * 30 = 20.0
        // result = 120 + 20 = 140.0
        let result2 = amsha_longitude(40.0, Amsha::D2, Some(AmshaVariation::HoraCancerLeoOnly));
        assert!(
            (result2 - 140.0).abs() < 0.01,
            "D2 cancer-leo even: got {result2}"
        );
    }

    #[test]
    fn d3_trine_progression() {
        // Vrishabha (1) at 45.5 deg: pos_in_rashi=15.5
        // start = 1, step = 4, deg_per_div = 10
        // div_idx = floor(15.5/10) = 1
        // target = (1 + 1*4) % 12 = 5 (Kanya)
        // scaled = (15.5-10)/10 * 30 = 16.5
        // result = 150 + 16.5 = 166.5
        let result = amsha_longitude(45.5, Amsha::D3, None);
        assert!((result - 166.5).abs() < 0.01, "D3 trine: got {result}");
    }

    #[test]
    fn d30_odd_even() {
        // Mesha (0, odd) at 1.5: pos_in_rashi=1.5
        // start=0 (Mesha), deg_per_div=1.0, div_idx=1
        // target = (0+1)%12 = 1 (Vrishabha)
        // scaled = (1.5-1)/1 * 30 = 15.0
        // result = 30 + 15 = 45.0
        let result = amsha_longitude(1.5, Amsha::D30, None);
        assert!((result - 45.0).abs() < 0.01, "D30 odd: got {result}");

        // Vrishabha (1, even) at 31.5: pos_in_rashi=1.5
        // start=11 (Meena), deg_per_div=1.0, div_idx=1
        // target = (11+1)%12 = 0 (Mesha)
        // scaled = 15.0
        // result = 0 + 15 = 15.0
        let result2 = amsha_longitude(31.5, Amsha::D30, None);
        assert!((result2 - 15.0).abs() < 0.01, "D30 even: got {result2}");
    }

    #[test]
    fn all_amshas_output_in_range() {
        let test_lons = [0.0, 15.0, 29.999, 45.5, 90.0, 180.0, 270.0, 359.999];
        for &lon in &test_lons {
            for &amsha in &ALL_AMSHAS {
                let result = amsha_longitude(lon, amsha, None);
                assert!(
                    (0.0..360.0).contains(&result),
                    "Out of range: amsha={:?}, lon={lon}, result={result}",
                    amsha
                );
            }
        }
    }

    #[test]
    fn boundary_zero() {
        let result = amsha_longitude(0.0, Amsha::D9, None);
        assert!(result >= 0.0 && result < 360.0);
    }

    #[test]
    fn boundary_near_30() {
        let result = amsha_longitude(29.999, Amsha::D9, None);
        assert!(result >= 0.0 && result < 360.0);
    }

    #[test]
    fn boundary_exactly_30() {
        let result = amsha_longitude(30.0, Amsha::D9, None);
        assert!(result >= 0.0 && result < 360.0);
    }

    #[test]
    fn boundary_near_360() {
        let result = amsha_longitude(359.999, Amsha::D9, None);
        assert!(result >= 0.0 && result < 360.0);
    }

    #[test]
    fn boundary_negative() {
        let result = amsha_longitude(-10.0, Amsha::D9, None);
        assert!(result >= 0.0 && result < 360.0);
    }

    #[test]
    fn dms_round_trip() {
        let test_vals = [0.0, 5.123, 15.5, 23.853, 29.999];
        for &val in &test_vals {
            let dms = deg_to_dms(val);
            let back = crate::rashi::dms_to_deg(&dms);
            assert!(
                (back - val).abs() < 0.001,
                "DMS round-trip failed: {val} -> {:?} -> {back}",
                dms
            );
        }
    }

    #[test]
    fn amsha_from_rashi_position_matches_rashi_info() {
        let lon = 45.5;
        let rashi_idx = 1u8; // Vrishabha
        let deg_in_rashi = 15.5;

        let from_pos = amsha_from_rashi_position(rashi_idx, deg_in_rashi, Amsha::D9, None);
        let from_lon = amsha_rashi_info(lon, Amsha::D9, None);

        assert_eq!(from_pos.rashi, from_lon.rashi);
        assert!((from_pos.degrees_in_rashi - from_lon.degrees_in_rashi).abs() < 0.01);
    }

    #[test]
    fn batch_matches_individual() {
        let lon = 100.0;
        let requests = vec![
            AmshaRequest::new(Amsha::D9),
            AmshaRequest::new(Amsha::D10),
            AmshaRequest::new(Amsha::D12),
        ];
        let batch = amsha_longitudes(lon, &requests);
        for (i, req) in requests.iter().enumerate() {
            let individual = amsha_longitude(lon, req.amsha, req.variation);
            assert!(
                (batch[i] - individual).abs() < 1e-10,
                "Batch mismatch at index {i}"
            );
        }
    }

    #[test]
    fn amsha_request_default_variation() {
        let req = AmshaRequest::new(Amsha::D9);
        assert_eq!(
            req.effective_variation(),
            AmshaVariation::TraditionalParashari
        );
    }

    #[test]
    fn amsha_from_code_valid() {
        assert_eq!(Amsha::from_code(9), Some(Amsha::D9));
        assert_eq!(Amsha::from_code(1), Some(Amsha::D1));
        assert_eq!(Amsha::from_code(144), Some(Amsha::D144));
    }

    #[test]
    fn amsha_from_code_invalid() {
        assert_eq!(Amsha::from_code(0), None);
        assert_eq!(Amsha::from_code(999), None);
        assert_eq!(Amsha::from_code(13), None);
    }

    #[test]
    fn variation_from_code_valid() {
        assert_eq!(
            AmshaVariation::from_code(0),
            Some(AmshaVariation::TraditionalParashari)
        );
        assert_eq!(
            AmshaVariation::from_code(1),
            Some(AmshaVariation::HoraCancerLeoOnly)
        );
    }

    #[test]
    fn variation_from_code_invalid() {
        assert_eq!(AmshaVariation::from_code(99), None);
    }

    #[test]
    fn variation_applicability() {
        assert!(AmshaVariation::TraditionalParashari.is_applicable_to(Amsha::D9));
        assert!(AmshaVariation::HoraCancerLeoOnly.is_applicable_to(Amsha::D2));
        assert!(!AmshaVariation::HoraCancerLeoOnly.is_applicable_to(Amsha::D9));
    }

    #[test]
    fn rashi_element_all_12() {
        assert_eq!(rashi_element(0), RashiElement::Fire); // Mesha
        assert_eq!(rashi_element(1), RashiElement::Earth); // Vrishabha
        assert_eq!(rashi_element(2), RashiElement::Air); // Mithuna
        assert_eq!(rashi_element(3), RashiElement::Water); // Karka
        assert_eq!(rashi_element(4), RashiElement::Fire); // Simha
        assert_eq!(rashi_element(5), RashiElement::Earth); // Kanya
        assert_eq!(rashi_element(6), RashiElement::Air); // Tula
        assert_eq!(rashi_element(7), RashiElement::Water); // Vrischika
        assert_eq!(rashi_element(8), RashiElement::Fire); // Dhanu
        assert_eq!(rashi_element(9), RashiElement::Earth); // Makara
        assert_eq!(rashi_element(10), RashiElement::Air); // Kumbha
        assert_eq!(rashi_element(11), RashiElement::Water); // Meena
    }

    #[test]
    fn amsha_index_sequential() {
        for (i, &amsha) in ALL_AMSHAS.iter().enumerate() {
            assert_eq!(amsha.index() as usize, i);
        }
    }

    #[test]
    fn amsha_code_roundtrip() {
        for &amsha in &ALL_AMSHAS {
            let code = amsha.code();
            assert_eq!(Amsha::from_code(code), Some(amsha));
        }
    }

    #[test]
    fn rashi_position_to_longitude_basic() {
        assert!((rashi_position_to_longitude(0, 0.0) - 0.0).abs() < 1e-10);
        assert!((rashi_position_to_longitude(1, 15.5) - 45.5).abs() < 1e-10);
        assert!((rashi_position_to_longitude(11, 29.0) - 359.0).abs() < 1e-10);
    }

    #[test]
    fn amsha_rashi_infos_batch() {
        let lon = 100.0;
        let requests = vec![AmshaRequest::new(Amsha::D9), AmshaRequest::new(Amsha::D12)];
        let results = amsha_rashi_infos(lon, &requests);
        assert_eq!(results.len(), 2);
        let individual_d9 = amsha_rashi_info(lon, Amsha::D9, None);
        assert_eq!(results[0].rashi, individual_d9.rashi);
    }
}
