//! Types for Vedic jyotish orchestration (graha longitudes, etc.).

use dhruv_vedic_base::{
    AllUpagrahas, AshtakavargaResult, DrishtiEntry, Graha, GrahaDrishtiMatrix, Nakshatra, Rashi,
};

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

/// Configuration flags for core_bindus computation.
#[derive(Debug, Clone, Copy)]
pub struct BindusConfig {
    /// Compute nakshatra + pada for each bindu point.
    pub include_nakshatra: bool,
    /// Compute bhava placement for each bindu point.
    pub include_bhava: bool,
}

impl Default for BindusConfig {
    fn default() -> Self {
        Self {
            include_nakshatra: false,
            include_bhava: false,
        }
    }
}

/// Curated sensitive points (bindus) with optional nakshatra/bhava enrichment.
#[derive(Debug, Clone, Copy)]
pub struct BindusResult {
    /// 12 arudha padas (A1 through A12).
    pub arudha_padas: [GrahaEntry; 12],
    /// Bhrigu Bindu (midpoint Rahu→Moon).
    pub bhrigu_bindu: GrahaEntry,
    /// Pranapada Lagna.
    pub pranapada_lagna: GrahaEntry,
    /// Gulika (lagna at Saturn's portion start).
    pub gulika: GrahaEntry,
    /// Maandi (lagna at Saturn's portion end).
    pub maandi: GrahaEntry,
    /// Hora Lagna.
    pub hora_lagna: GrahaEntry,
    /// Ghati Lagna.
    pub ghati_lagna: GrahaEntry,
    /// Sree Lagna.
    pub sree_lagna: GrahaEntry,
}

/// Configuration flags for drishti computation.
#[derive(Debug, Clone, Copy)]
pub struct DrishtiConfig {
    /// Compute 9×12 graha-to-bhava-cusp drishti.
    pub include_bhava: bool,
    /// Compute 9×1 graha-to-lagna drishti.
    pub include_lagna: bool,
    /// Compute 9×19 graha-to-core-bindus drishti.
    pub include_bindus: bool,
}

impl Default for DrishtiConfig {
    fn default() -> Self {
        Self {
            include_bhava: false,
            include_lagna: false,
            include_bindus: false,
        }
    }
}

/// Complete drishti result with graha matrix and optional extensions.
#[derive(Debug, Clone, Copy)]
pub struct DrishtiResult {
    /// 9×9 graha-to-graha drishti (always computed).
    pub graha_to_graha: GrahaDrishtiMatrix,
    /// 9×12 graha-to-bhava-cusp drishti (zeroed if flag off).
    pub graha_to_bhava: [[DrishtiEntry; 12]; 9],
    /// 9×1 graha-to-lagna drishti (zeroed if flag off).
    pub graha_to_lagna: [DrishtiEntry; 9],
    /// 9×19 graha-to-core-bindus drishti (zeroed if flag off).
    /// 19 bindus = 12 arudha padas + bhrigu_bindu + pranapada + gulika + maandi + hora_lagna + ghati_lagna + sree_lagna.
    pub graha_to_bindus: [[DrishtiEntry; 19]; 9],
}

/// Configuration for one-shot full kundali computation.
#[derive(Debug, Clone, Copy)]
pub struct FullKundaliConfig {
    /// Include comprehensive graha positions section.
    pub include_graha_positions: bool,
    /// Include core bindus section.
    pub include_bindus: bool,
    /// Include drishti section.
    pub include_drishti: bool,
    /// Include ashtakavarga section.
    pub include_ashtakavarga: bool,
    /// Include upagrahas section.
    pub include_upagrahas: bool,
    /// Config passed to graha positions computation.
    pub graha_positions_config: GrahaPositionsConfig,
    /// Config passed to bindus computation.
    pub bindus_config: BindusConfig,
    /// Config passed to drishti computation.
    pub drishti_config: DrishtiConfig,
}

impl Default for FullKundaliConfig {
    fn default() -> Self {
        Self {
            include_graha_positions: true,
            include_bindus: true,
            include_drishti: true,
            include_ashtakavarga: true,
            include_upagrahas: true,
            graha_positions_config: GrahaPositionsConfig::default(),
            bindus_config: BindusConfig::default(),
            drishti_config: DrishtiConfig::default(),
        }
    }
}

/// One-shot full kundali result.
#[derive(Debug, Clone)]
pub struct FullKundaliResult {
    /// Present when `FullKundaliConfig::include_graha_positions` is true.
    pub graha_positions: Option<GrahaPositions>,
    /// Present when `FullKundaliConfig::include_bindus` is true.
    pub bindus: Option<BindusResult>,
    /// Present when `FullKundaliConfig::include_drishti` is true.
    pub drishti: Option<DrishtiResult>,
    /// Present when `FullKundaliConfig::include_ashtakavarga` is true.
    pub ashtakavarga: Option<AshtakavargaResult>,
    /// Present when `FullKundaliConfig::include_upagrahas` is true.
    pub upagrahas: Option<AllUpagrahas>,
}
