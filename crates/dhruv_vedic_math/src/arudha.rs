//! Arudha Pada (bhava reflection) calculations.
//!
//! 12 Arudha Padas: A1 (Arudha Lagna) through A12 (Upapada).
//! Each reflects the influence of a bhava lord measured from the bhava cusp.
//!
//! Formula:
//! 1. arc = (lord_lon - cusp_lon) % 360
//! 2. arudha = (lord_lon + arc) % 360
//! 3. Exception: if result falls in bhava sign or 7th from it → take 10th
//!
//! Clean-room implementation from standard Vedic jyotish texts (BPHS, Jaimini Sutras).
//! See `docs/clean_room_arudha.md`.

use crate::util::normalize_360;

/// The 12 Arudha Padas.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ArudhaPada {
    ArudhaLagna, // A1/AL
    DhanaPada,   // A2
    VikramaPada, // A3
    MatriPada,   // A4
    MantraPada,  // A5
    RogaPada,    // A6
    DaraPada,    // A7
    MrityuPada,  // A8
    PitriPada,   // A9
    RajyaPada,   // A10
    LabhaPada,   // A11
    Upapada,     // A12/UL
}

/// All 12 arudha padas in order (A1 through A12).
pub const ALL_ARUDHA_PADAS: [ArudhaPada; 12] = [
    ArudhaPada::ArudhaLagna,
    ArudhaPada::DhanaPada,
    ArudhaPada::VikramaPada,
    ArudhaPada::MatriPada,
    ArudhaPada::MantraPada,
    ArudhaPada::RogaPada,
    ArudhaPada::DaraPada,
    ArudhaPada::MrityuPada,
    ArudhaPada::PitriPada,
    ArudhaPada::RajyaPada,
    ArudhaPada::LabhaPada,
    ArudhaPada::Upapada,
];

impl ArudhaPada {
    /// Name of the arudha pada.
    pub const fn name(self) -> &'static str {
        match self {
            Self::ArudhaLagna => "Arudha Lagna",
            Self::DhanaPada => "Dhana Pada",
            Self::VikramaPada => "Vikrama Pada",
            Self::MatriPada => "Matri Pada",
            Self::MantraPada => "Mantra Pada",
            Self::RogaPada => "Roga Pada",
            Self::DaraPada => "Dara Pada",
            Self::MrityuPada => "Mrityu Pada",
            Self::PitriPada => "Pitri Pada",
            Self::RajyaPada => "Rajya Pada",
            Self::LabhaPada => "Labha Pada",
            Self::Upapada => "Upapada",
        }
    }

    /// 0-based index (0=A1, 11=A12).
    pub const fn index(self) -> u8 {
        match self {
            Self::ArudhaLagna => 0,
            Self::DhanaPada => 1,
            Self::VikramaPada => 2,
            Self::MatriPada => 3,
            Self::MantraPada => 4,
            Self::RogaPada => 5,
            Self::DaraPada => 6,
            Self::MrityuPada => 7,
            Self::PitriPada => 8,
            Self::RajyaPada => 9,
            Self::LabhaPada => 10,
            Self::Upapada => 11,
        }
    }

    /// Bhava number (1-12).
    pub const fn bhava_number(self) -> u8 {
        self.index() + 1
    }
}

/// Result for a single arudha pada computation.
#[derive(Debug, Clone, Copy)]
pub struct ArudhaResult {
    pub pada: ArudhaPada,
    pub longitude_deg: f64,
    pub rashi_index: u8,
}

/// Compute the arudha pada for a single bhava.
///
/// Arguments:
/// - `bhava_cusp_lon`: sidereal longitude of the bhava cusp (0-360)
/// - `lord_lon`: sidereal longitude of the bhava's lord (0-360)
///
/// Returns `(arudha_longitude, arudha_rashi_index)`.
///
/// The exception rule: if the arudha falls in the same rashi as the bhava
/// cusp or its 7th, take the 10th from the result (add 270 deg).
pub fn arudha_pada(bhava_cusp_lon: f64, lord_lon: f64) -> (f64, u8) {
    // 1. Arc from cusp to lord
    let arc = normalize_360(lord_lon - bhava_cusp_lon);

    // 2. Project same arc forward from lord
    let mut arudha = normalize_360(lord_lon + arc);
    let mut arudha_rashi = (arudha / 30.0) as u8;

    // 3. Exception: same sign or 7th from bhava
    let bhava_rashi = (bhava_cusp_lon / 30.0) as u8;
    let seventh_from_bhava = (bhava_rashi + 6) % 12;

    if arudha_rashi == bhava_rashi || arudha_rashi == seventh_from_bhava {
        // Take 10th from result (add 9 signs = 270 deg)
        arudha = normalize_360(arudha + 270.0);
        arudha_rashi = (arudha / 30.0) as u8;
    }

    (arudha, arudha_rashi)
}

/// Compute all 12 arudha padas.
///
/// Arguments:
/// - `bhava_cusps`: 12 sidereal cusp longitudes (index 0 = 1st house)
/// - `lord_lons`: 12 sidereal lord longitudes (one per house, resolved from cusp rashi)
pub fn all_arudha_padas(bhava_cusps: &[f64; 12], lord_lons: &[f64; 12]) -> [ArudhaResult; 12] {
    let mut results = [ArudhaResult {
        pada: ArudhaPada::ArudhaLagna,
        longitude_deg: 0.0,
        rashi_index: 0,
    }; 12];

    for i in 0..12 {
        let (lon, rashi) = arudha_pada(bhava_cusps[i], lord_lons[i]);
        results[i] = ArudhaResult {
            pada: ALL_ARUDHA_PADAS[i],
            longitude_deg: lon,
            rashi_index: rashi,
        };
    }

    results
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_arudha_padas_count() {
        assert_eq!(ALL_ARUDHA_PADAS.len(), 12);
    }

    #[test]
    fn arudha_pada_indices_sequential() {
        for (i, ap) in ALL_ARUDHA_PADAS.iter().enumerate() {
            assert_eq!(ap.index() as usize, i);
            assert_eq!(ap.bhava_number() as usize, i + 1);
        }
    }

    #[test]
    fn arudha_pada_names_nonempty() {
        for ap in ALL_ARUDHA_PADAS {
            assert!(!ap.name().is_empty());
        }
    }

    #[test]
    fn basic_arudha_no_exception() {
        // Cusp at 10 deg (Mesha), Lord at 70 deg (Mithuna)
        // Arc = 60 deg, Arudha = 70+60 = 130 (Simha, rashi 4)
        // Bhava rashi = 0 (Mesha), 7th = 6 (Tula)
        // Arudha rashi = 4 (Simha) — no exception
        let (lon, rashi) = arudha_pada(10.0, 70.0);
        assert!((lon - 130.0).abs() < 1e-10);
        assert_eq!(rashi, 4); // Simha
    }

    #[test]
    fn arudha_exception_same_rashi() {
        // Cusp at 10 deg (Mesha), Lord at 15 deg (Mesha)
        // Arc = 5 deg, Arudha = 15+5 = 20 (Mesha, rashi 0)
        // Bhava rashi = 0 (Mesha) → same! → take 10th: 20+270 = 290 (Makara, rashi 9)
        let (lon, rashi) = arudha_pada(10.0, 15.0);
        assert!((lon - 290.0).abs() < 1e-10);
        assert_eq!(rashi, 9); // Makara
    }

    #[test]
    fn arudha_exception_seventh() {
        // Cusp at 10 deg (Mesha rashi 0), Lord at 100 deg (Karka rashi 3)
        // Arc = 90, Arudha = 100+90 = 190 (Tula, rashi 6)
        // 7th from Mesha = Tula (rashi 6) → exception! → 190+270 = 460 → 100 (Karka, rashi 3)
        let (lon, rashi) = arudha_pada(10.0, 100.0);
        assert!((lon - 100.0).abs() < 1e-10);
        assert_eq!(rashi, 3); // Karka
    }

    #[test]
    fn arudha_wraps_around() {
        // Cusp at 350 deg (Meena, rashi 11), Lord at 50 deg (Vrishabha, rashi 1)
        // Arc = (50-350)%360 = 60, Arudha = (50+60)%360 = 110 (Karka, rashi 3)
        // 7th from Meena = Kanya (rashi 5). Arudha rashi=3 — no exception
        let (lon, rashi) = arudha_pada(350.0, 50.0);
        assert!((lon - 110.0).abs() < 1e-10);
        assert_eq!(rashi, 3); // Karka
    }

    #[test]
    fn all_arudha_padas_basic() {
        let cusps = [
            0.0, 30.0, 60.0, 90.0, 120.0, 150.0, 180.0, 210.0, 240.0, 270.0, 300.0, 330.0,
        ];
        let lords = [
            45.0, 75.0, 105.0, 135.0, 165.0, 195.0, 225.0, 255.0, 285.0, 315.0, 345.0, 15.0,
        ];
        let results = all_arudha_padas(&cusps, &lords);

        for r in &results {
            assert!(r.longitude_deg >= 0.0 && r.longitude_deg < 360.0);
            assert!(r.rashi_index < 12);
        }
    }
}
