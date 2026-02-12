//! Types for Vedic jyotish orchestration (graha longitudes, etc.).

use dhruv_vedic_base::{Graha, Nakshatra, Rashi};

/// Sidereal longitudes of all 9 grahas.
#[derive(Debug, Clone, Copy)]
pub struct GrahaLongitudes {
    /// Sidereal longitudes indexed by `Graha::index()` (0-8).
    pub longitudes: [f64; 9],
}

impl GrahaLongitudes {
    /// Get the sidereal longitude for a specific graha.
    pub fn longitude(&self, graha: Graha) -> f64 {
        self.longitudes[graha.index() as usize]
    }

    /// Get the 0-based rashi index (0-11) for a specific graha.
    pub fn rashi_index(&self, graha: Graha) -> u8 {
        let lon = self.longitude(graha);
        ((lon / 30.0).floor() as u8).min(11)
    }

    /// Get rashi indices for all 9 grahas.
    pub fn all_rashi_indices(&self) -> [u8; 9] {
        let mut indices = [0u8; 9];
        for (i, &lon) in self.longitudes.iter().enumerate() {
            indices[i] = ((lon / 30.0).floor() as u8).min(11);
        }
        indices
    }
}

/// Configuration flags for graha_positions computation.
#[derive(Debug, Clone, Copy)]
pub struct GrahaPositionsConfig {
    /// Compute nakshatra + pada for each graha.
    pub include_nakshatra: bool,
    /// Compute lagna (ascendant) longitude.
    pub include_lagna: bool,
    /// Include Uranus, Neptune, Pluto.
    pub include_outer_planets: bool,
    /// Compute bhava placement for each graha.
    pub include_bhava: bool,
}

impl Default for GrahaPositionsConfig {
    fn default() -> Self {
        Self {
            include_nakshatra: false,
            include_lagna: false,
            include_outer_planets: false,
            include_bhava: false,
        }
    }
}

/// Position details for a single graha.
#[derive(Debug, Clone, Copy)]
pub struct GrahaEntry {
    /// Sidereal longitude in degrees [0, 360).
    pub sidereal_longitude: f64,
    /// Rashi (0-based, 0-11).
    pub rashi: Rashi,
    /// Rashi index (0-based, 0-11).
    pub rashi_index: u8,
    /// Nakshatra (sentinel Ashwini if not computed).
    pub nakshatra: Nakshatra,
    /// Nakshatra index (0-26), 255 if not computed.
    pub nakshatra_index: u8,
    /// Pada (1-4), 0 if not computed.
    pub pada: u8,
    /// Bhava number (1-12), 0 if not computed.
    pub bhava_number: u8,
}

impl GrahaEntry {
    /// Create a sentinel (zeroed) entry.
    pub fn sentinel() -> Self {
        Self {
            sidereal_longitude: 0.0,
            rashi: Rashi::Mesha,
            rashi_index: 0,
            nakshatra: Nakshatra::Ashwini,
            nakshatra_index: 255,
            pada: 0,
            bhava_number: 0,
        }
    }
}

/// Complete graha positions result.
#[derive(Debug, Clone, Copy)]
pub struct GrahaPositions {
    /// 9 Vedic grahas (Sun through Ketu), indexed by Graha::index().
    pub grahas: [GrahaEntry; 9],
    /// Lagna entry (sentinel zeros if not computed).
    pub lagna: GrahaEntry,
    /// Outer planets: [Uranus, Neptune, Pluto] (sentinel zeros if not computed).
    pub outer_planets: [GrahaEntry; 3],
}
