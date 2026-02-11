//! Sphuta (sensitive point) calculations.
//!
//! 16 mathematical sensitive points derived from planetary positions.
//! All functions are pure math: sidereal longitudes in, sidereal longitude out.
//!
//! Clean-room implementation from standard Vedic jyotish texts (BPHS).
//! See `docs/clean_room_sphuta.md`.

use crate::util::normalize_360;

/// The 16 sphuta types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Sphuta {
    BhriguBindu,
    PranaSphuta,
    DehaSphuta,
    MrityuSphuta,
    TithiSphuta,
    YogaSphuta,
    YogaSphutaNormalized,
    RahuTithiSphuta,
    KshetraSphuta,
    BeejaSphuta,
    TriSphuta,
    ChatusSphuta,
    PanchaSphuta,
    SookshmaTrisphuta,
    AvayogaSphuta,
    Kunda,
}

/// All 16 sphutas in order.
pub const ALL_SPHUTAS: [Sphuta; 16] = [
    Sphuta::BhriguBindu,
    Sphuta::PranaSphuta,
    Sphuta::DehaSphuta,
    Sphuta::MrityuSphuta,
    Sphuta::TithiSphuta,
    Sphuta::YogaSphuta,
    Sphuta::YogaSphutaNormalized,
    Sphuta::RahuTithiSphuta,
    Sphuta::KshetraSphuta,
    Sphuta::BeejaSphuta,
    Sphuta::TriSphuta,
    Sphuta::ChatusSphuta,
    Sphuta::PanchaSphuta,
    Sphuta::SookshmaTrisphuta,
    Sphuta::AvayogaSphuta,
    Sphuta::Kunda,
];

impl Sphuta {
    /// Name of the sphuta.
    pub const fn name(self) -> &'static str {
        match self {
            Self::BhriguBindu => "Bhrigu Bindu",
            Self::PranaSphuta => "Prana Sphuta",
            Self::DehaSphuta => "Deha Sphuta",
            Self::MrityuSphuta => "Mrityu Sphuta",
            Self::TithiSphuta => "Tithi Sphuta",
            Self::YogaSphuta => "Yoga Sphuta",
            Self::YogaSphutaNormalized => "Yoga Sphuta Normalized",
            Self::RahuTithiSphuta => "Rahu Tithi Sphuta",
            Self::KshetraSphuta => "Kshetra Sphuta",
            Self::BeejaSphuta => "Beeja Sphuta",
            Self::TriSphuta => "TriSphuta",
            Self::ChatusSphuta => "ChatusSphuta",
            Self::PanchaSphuta => "PanchaSphuta",
            Self::SookshmaTrisphuta => "Sookshma TriSphuta",
            Self::AvayogaSphuta => "Avayoga Sphuta",
            Self::Kunda => "Kunda",
        }
    }

    /// 0-based index.
    pub const fn index(self) -> u8 {
        match self {
            Self::BhriguBindu => 0,
            Self::PranaSphuta => 1,
            Self::DehaSphuta => 2,
            Self::MrityuSphuta => 3,
            Self::TithiSphuta => 4,
            Self::YogaSphuta => 5,
            Self::YogaSphutaNormalized => 6,
            Self::RahuTithiSphuta => 7,
            Self::KshetraSphuta => 8,
            Self::BeejaSphuta => 9,
            Self::TriSphuta => 10,
            Self::ChatusSphuta => 11,
            Self::PanchaSphuta => 12,
            Self::SookshmaTrisphuta => 13,
            Self::AvayogaSphuta => 14,
            Self::Kunda => 15,
        }
    }
}

// ---------------------------------------------------------------------------
// Individual sphuta formulas (all pure math, f64 in → f64 out)
// ---------------------------------------------------------------------------

/// Bhrigu Bindu: midpoint from Rahu to Moon (forward arc).
///
/// Formula: `(rahu + ((moon + 360 - rahu) % 360) / 2) % 360`
pub fn bhrigu_bindu(rahu: f64, moon: f64) -> f64 {
    normalize_360(rahu + normalize_360(moon - rahu) / 2.0)
}

/// Prana Sphuta (Life Point).
///
/// Formula: `(lagna × 5 + moon) % 360`
pub fn prana_sphuta(lagna: f64, moon: f64) -> f64 {
    normalize_360(lagna * 5.0 + moon)
}

/// Deha Sphuta (Body Point).
///
/// Formula: `(moon × 8 + lagna) % 360`
pub fn deha_sphuta(moon: f64, lagna: f64) -> f64 {
    normalize_360(moon * 8.0 + lagna)
}

/// Mrityu Sphuta (Death Point).
///
/// Formula: `(eighth_lord × 8 + lagna) % 360`
pub fn mrityu_sphuta(eighth_lord: f64, lagna: f64) -> f64 {
    normalize_360(eighth_lord * 8.0 + lagna)
}

/// Tithi Sphuta.
///
/// Formula: `((moon - sun) % 360 / 12 + lagna) % 360`
pub fn tithi_sphuta(moon: f64, sun: f64, lagna: f64) -> f64 {
    let tithi_arc = normalize_360(moon - sun);
    normalize_360(tithi_arc / 12.0 + lagna)
}

/// Yoga Sphuta (Sun+Moon combination).
///
/// Formula: `(sun + moon) % 360`
pub fn yoga_sphuta(sun: f64, moon: f64) -> f64 {
    normalize_360(sun + moon)
}

/// Yoga Sphuta Normalized: position within the current yoga cycle.
///
/// Each yoga spans 13°20' (360/27). Returns the position within the current yoga.
pub fn yoga_sphuta_normalized(sun: f64, moon: f64) -> f64 {
    let yoga_total = normalize_360(sun + moon);
    let yoga_span = 360.0 / 27.0;
    let yoga_index = yoga_total / yoga_span;
    (yoga_index % 1.0) * yoga_span
}

/// Rahu Tithi Sphuta.
///
/// Formula: `((rahu - sun) % 360 / 12 + lagna) % 360`
pub fn rahu_tithi_sphuta(rahu: f64, sun: f64, lagna: f64) -> f64 {
    let arc = normalize_360(rahu - sun);
    normalize_360(arc / 12.0 + lagna)
}

/// Kshetra Sphuta (Female fertility point).
///
/// Formula: `(venus + moon + mars + jupiter + lagna) % 360`
pub fn kshetra_sphuta(venus: f64, moon: f64, mars: f64, jupiter: f64, lagna: f64) -> f64 {
    normalize_360(venus + moon + mars + jupiter + lagna)
}

/// Beeja Sphuta (Male fertility point).
///
/// Formula: `(sun + venus + jupiter) % 360`
pub fn beeja_sphuta(sun: f64, venus: f64, jupiter: f64) -> f64 {
    normalize_360(sun + venus + jupiter)
}

/// TriSphuta (Three-fold point).
///
/// Formula: `(lagna + moon + gulika) % 360`
pub fn trisphuta(lagna: f64, moon: f64, gulika: f64) -> f64 {
    normalize_360(lagna + moon + gulika)
}

/// ChatusSphuta (Four-fold point).
///
/// Formula: `(trisphuta_val + sun) % 360`
pub fn chatussphuta(trisphuta_val: f64, sun: f64) -> f64 {
    normalize_360(trisphuta_val + sun)
}

/// PanchaSphuta (Five-fold point).
///
/// Formula: `(chatussphuta_val + rahu) % 360`
pub fn panchasphuta(chatussphuta_val: f64, rahu: f64) -> f64 {
    normalize_360(chatussphuta_val + rahu)
}

/// Sookshma TriSphuta (Subtle three-fold point).
///
/// Formula: `((lagna + moon + gulika + sun) / 4) % 360`
pub fn sookshma_trisphuta(lagna: f64, moon: f64, gulika: f64, sun: f64) -> f64 {
    normalize_360((lagna + moon + gulika + sun) / 4.0)
}

/// Avayoga Sphuta (opposite of Yoga Sphuta).
///
/// Formula: `(360 - yoga_sphuta) % 360`
pub fn avayoga_sphuta(sun: f64, moon: f64) -> f64 {
    normalize_360(360.0 - yoga_sphuta(sun, moon))
}

/// Kunda.
///
/// Formula: `(lagna + moon + mars) % 360`
pub fn kunda(lagna: f64, moon: f64, mars: f64) -> f64 {
    normalize_360(lagna + moon + mars)
}

// ---------------------------------------------------------------------------
// Batch computation
// ---------------------------------------------------------------------------

/// Input longitudes for computing all 16 sphutas.
///
/// All values are sidereal ecliptic longitudes in degrees.
#[derive(Debug, Clone, Copy)]
pub struct SphutalInputs {
    pub sun: f64,
    pub moon: f64,
    pub mars: f64,
    pub jupiter: f64,
    pub venus: f64,
    pub rahu: f64,
    pub lagna: f64,
    /// Longitude of the 8th house lord (needed for Mrityu Sphuta).
    pub eighth_lord: f64,
    /// Gulika longitude (needed for TriSphuta and derivatives).
    pub gulika: f64,
}

/// Compute all 16 sphutas from the given inputs.
///
/// Returns an array of (Sphuta, longitude_deg) pairs.
pub fn all_sphutas(inputs: &SphutalInputs) -> [(Sphuta, f64); 16] {
    let tri = trisphuta(inputs.lagna, inputs.moon, inputs.gulika);
    let chatus = chatussphuta(tri, inputs.sun);

    [
        (Sphuta::BhriguBindu, bhrigu_bindu(inputs.rahu, inputs.moon)),
        (Sphuta::PranaSphuta, prana_sphuta(inputs.lagna, inputs.moon)),
        (Sphuta::DehaSphuta, deha_sphuta(inputs.moon, inputs.lagna)),
        (Sphuta::MrityuSphuta, mrityu_sphuta(inputs.eighth_lord, inputs.lagna)),
        (Sphuta::TithiSphuta, tithi_sphuta(inputs.moon, inputs.sun, inputs.lagna)),
        (Sphuta::YogaSphuta, yoga_sphuta(inputs.sun, inputs.moon)),
        (Sphuta::YogaSphutaNormalized, yoga_sphuta_normalized(inputs.sun, inputs.moon)),
        (Sphuta::RahuTithiSphuta, rahu_tithi_sphuta(inputs.rahu, inputs.sun, inputs.lagna)),
        (Sphuta::KshetraSphuta, kshetra_sphuta(inputs.venus, inputs.moon, inputs.mars, inputs.jupiter, inputs.lagna)),
        (Sphuta::BeejaSphuta, beeja_sphuta(inputs.sun, inputs.venus, inputs.jupiter)),
        (Sphuta::TriSphuta, tri),
        (Sphuta::ChatusSphuta, chatus),
        (Sphuta::PanchaSphuta, panchasphuta(chatus, inputs.rahu)),
        (Sphuta::SookshmaTrisphuta, sookshma_trisphuta(inputs.lagna, inputs.moon, inputs.gulika, inputs.sun)),
        (Sphuta::AvayogaSphuta, avayoga_sphuta(inputs.sun, inputs.moon)),
        (Sphuta::Kunda, kunda(inputs.lagna, inputs.moon, inputs.mars)),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_sphutas_count() {
        assert_eq!(ALL_SPHUTAS.len(), 16);
    }

    #[test]
    fn sphuta_indices_sequential() {
        for (i, s) in ALL_SPHUTAS.iter().enumerate() {
            assert_eq!(s.index() as usize, i);
        }
    }

    #[test]
    fn sphuta_names_nonempty() {
        for s in ALL_SPHUTAS {
            assert!(!s.name().is_empty());
        }
    }

    #[test]
    fn bhrigu_bindu_known() {
        // Rahu=120, Moon=240: forward arc from Rahu to Moon = 120
        // midpoint = 120 + 60 = 180
        let bb = bhrigu_bindu(120.0, 240.0);
        assert!((bb - 180.0).abs() < 1e-10, "bb={bb}");
    }

    #[test]
    fn bhrigu_bindu_same_position() {
        // Rahu=100, Moon=100: forward arc = 0, midpoint = 100
        let bb = bhrigu_bindu(100.0, 100.0);
        assert!((bb - 100.0).abs() < 1e-10, "bb={bb}");
    }

    #[test]
    fn bhrigu_bindu_wrap() {
        // Rahu=350, Moon=10: forward arc from 350 to 10 = 20
        // midpoint = 350 + 10 = 360 → 0
        let bb = bhrigu_bindu(350.0, 10.0);
        assert!((bb - 0.0).abs() < 1e-10, "bb={bb}");
    }

    #[test]
    fn prana_sphuta_known() {
        let ps = prana_sphuta(30.0, 60.0);
        // (30 × 5 + 60) % 360 = 210
        assert!((ps - 210.0).abs() < 1e-10, "ps={ps}");
    }

    #[test]
    fn deha_sphuta_known() {
        let ds = deha_sphuta(45.0, 10.0);
        // (45 × 8 + 10) % 360 = 370 % 360 = 10
        assert!((ds - 10.0).abs() < 1e-10, "ds={ds}");
    }

    #[test]
    fn mrityu_sphuta_known() {
        let ms = mrityu_sphuta(40.0, 10.0);
        // (40 × 8 + 10) % 360 = 330
        assert!((ms - 330.0).abs() < 1e-10, "ms={ms}");
    }

    #[test]
    fn tithi_sphuta_known() {
        let ts = tithi_sphuta(180.0, 0.0, 100.0);
        // arc = (180 - 0) % 360 = 180, normalized = 180/12 = 15
        // (15 + 100) % 360 = 115
        assert!((ts - 115.0).abs() < 1e-10, "ts={ts}");
    }

    #[test]
    fn yoga_sphuta_known() {
        let ys = yoga_sphuta(100.0, 200.0);
        assert!((ys - 300.0).abs() < 1e-10, "ys={ys}");
    }

    #[test]
    fn yoga_sphuta_wrap() {
        let ys = yoga_sphuta(200.0, 200.0);
        // (200 + 200) % 360 = 40
        assert!((ys - 40.0).abs() < 1e-10, "ys={ys}");
    }

    #[test]
    fn yoga_sphuta_normalized_range() {
        // Result should be in [0, 13.333...)
        let span = 360.0 / 27.0;
        for sun in [0.0, 45.0, 90.0, 180.0, 270.0, 350.0] {
            for moon in [0.0, 60.0, 120.0, 240.0, 359.0] {
                let ysn = yoga_sphuta_normalized(sun, moon);
                assert!(ysn >= 0.0 && ysn < span + 1e-10, "ysn={ysn}");
            }
        }
    }

    #[test]
    fn avayoga_complement() {
        let ys = yoga_sphuta(100.0, 200.0);
        let as_ = avayoga_sphuta(100.0, 200.0);
        // yoga + avayoga should sum to 360 (or 0 if both are 0/360)
        let sum = normalize_360(ys + as_);
        assert!(sum.abs() < 1e-10 || (sum - 360.0).abs() < 1e-10, "sum={sum}");
    }

    #[test]
    fn kshetra_sphuta_known() {
        let ks = kshetra_sphuta(10.0, 20.0, 30.0, 40.0, 50.0);
        // (10+20+30+40+50) % 360 = 150
        assert!((ks - 150.0).abs() < 1e-10, "ks={ks}");
    }

    #[test]
    fn beeja_sphuta_known() {
        let bs = beeja_sphuta(100.0, 200.0, 100.0);
        // (100+200+100) % 360 = 400 % 360 = 40
        assert!((bs - 40.0).abs() < 1e-10, "bs={bs}");
    }

    #[test]
    fn trisphuta_known() {
        let ts = trisphuta(10.0, 20.0, 30.0);
        assert!((ts - 60.0).abs() < 1e-10, "ts={ts}");
    }

    #[test]
    fn chatussphuta_from_trisphuta() {
        let tri = trisphuta(10.0, 20.0, 30.0);
        let cs = chatussphuta(tri, 40.0);
        // (60 + 40) % 360 = 100
        assert!((cs - 100.0).abs() < 1e-10, "cs={cs}");
    }

    #[test]
    fn panchasphuta_chain() {
        let tri = trisphuta(10.0, 20.0, 30.0);
        let cs = chatussphuta(tri, 40.0);
        let ps = panchasphuta(cs, 50.0);
        // (100 + 50) % 360 = 150
        assert!((ps - 150.0).abs() < 1e-10, "ps={ps}");
    }

    #[test]
    fn sookshma_trisphuta_known() {
        let st = sookshma_trisphuta(40.0, 80.0, 120.0, 160.0);
        // (40+80+120+160)/4 % 360 = 400/4 = 100
        assert!((st - 100.0).abs() < 1e-10, "st={st}");
    }

    #[test]
    fn kunda_known() {
        let k = kunda(50.0, 100.0, 150.0);
        // (50+100+150) % 360 = 300
        assert!((k - 300.0).abs() < 1e-10, "k={k}");
    }

    #[test]
    fn all_sphutas_output_in_range() {
        let inputs = SphutalInputs {
            sun: 100.0,
            moon: 200.0,
            mars: 150.0,
            jupiter: 250.0,
            venus: 300.0,
            rahu: 50.0,
            lagna: 120.0,
            eighth_lord: 180.0,
            gulika: 270.0,
        };
        let results = all_sphutas(&inputs);
        assert_eq!(results.len(), 16);
        for (sphuta, lon) in &results {
            assert!(
                *lon >= 0.0 && *lon < 360.0,
                "{}: lon={lon} out of range",
                sphuta.name()
            );
        }
    }

    #[test]
    fn all_sphutas_consistency() {
        // Verify that batch matches individual functions
        let inputs = SphutalInputs {
            sun: 100.0,
            moon: 200.0,
            mars: 150.0,
            jupiter: 250.0,
            venus: 300.0,
            rahu: 50.0,
            lagna: 120.0,
            eighth_lord: 180.0,
            gulika: 270.0,
        };
        let results = all_sphutas(&inputs);
        assert!((results[0].1 - bhrigu_bindu(50.0, 200.0)).abs() < 1e-10);
        assert!((results[1].1 - prana_sphuta(120.0, 200.0)).abs() < 1e-10);
        assert!((results[5].1 - yoga_sphuta(100.0, 200.0)).abs() < 1e-10);
        assert!((results[14].1 - avayoga_sphuta(100.0, 200.0)).abs() < 1e-10);
    }
}
