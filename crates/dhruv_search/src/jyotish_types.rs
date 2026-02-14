//! Types for Vedic jyotish orchestration (graha longitudes, etc.).

use dhruv_vedic_base::{
    AllGrahaAvasthas, Amsha, AmshaVariation, AllSpecialLagnas, AllUpagrahas, AshtakavargaResult,
    Dms, DrishtiEntry, Graha, GrahaDrishtiMatrix, KalaBalaBreakdown, Nakshatra,
    NodeDignityPolicy, Rashi, ShadbalaBreakdown, SthanaBalaBreakdown,
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

// ---------------------------------------------------------------------------
// Amsha (divisional chart) types
// ---------------------------------------------------------------------------

/// Maximum number of amsha requests in a single batch.
pub const MAX_AMSHA_REQUESTS: usize = 40;

/// Single entity's position in an amsha chart.
#[derive(Debug, Clone, Copy)]
pub struct AmshaEntry {
    /// Sidereal longitude in [0, 360).
    pub sidereal_longitude: f64,
    /// Rashi of the amsha position.
    pub rashi: Rashi,
    /// 0-based rashi index (0-11).
    pub rashi_index: u8,
    /// Degrees/minutes/seconds within rashi.
    pub dms: Dms,
    /// Decimal degrees within rashi [0, 30).
    pub degrees_in_rashi: f64,
}

/// Scope flags: which entity groups to include in amsha charts.
/// Grahas (9) + Lagna (1) always included.
#[derive(Debug, Clone, Copy)]
pub struct AmshaChartScope {
    pub include_bhava_cusps: bool,
    pub include_arudha_padas: bool,
    pub include_upagrahas: bool,
    pub include_sphutas: bool,
    pub include_special_lagnas: bool,
}

impl Default for AmshaChartScope {
    fn default() -> Self {
        Self {
            include_bhava_cusps: false,
            include_arudha_padas: false,
            include_upagrahas: false,
            include_sphutas: false,
            include_special_lagnas: false,
        }
    }
}

/// All entity positions in one amsha chart.
#[derive(Debug, Clone)]
pub struct AmshaChart {
    pub amsha: Amsha,
    pub variation: AmshaVariation,
    pub grahas: [AmshaEntry; 9],
    pub lagna: AmshaEntry,
    pub bhava_cusps: Option<[AmshaEntry; 12]>,
    pub arudha_padas: Option<[AmshaEntry; 12]>,
    pub upagrahas: Option<[AmshaEntry; 11]>,
    pub sphutas: Option<[AmshaEntry; 16]>,
    pub special_lagnas: Option<[AmshaEntry; 8]>,
}

/// Collection of amsha charts.
#[derive(Debug, Clone)]
pub struct AmshaResult {
    pub charts: Vec<AmshaChart>,
}

/// Fixed-size amsha selection for FullKundaliConfig (Copy-compatible).
#[derive(Debug, Clone, Copy)]
pub struct AmshaSelectionConfig {
    /// Number of valid entries (0..=MAX_AMSHA_REQUESTS).
    pub count: u8,
    /// D-numbers (1..144), 0=unused.
    pub codes: [u16; MAX_AMSHA_REQUESTS],
    /// Variation codes: 0=default, 1=HoraCancerLeoOnly.
    pub variations: [u8; MAX_AMSHA_REQUESTS],
}

impl Default for AmshaSelectionConfig {
    fn default() -> Self {
        Self {
            count: 0,
            codes: [0; MAX_AMSHA_REQUESTS],
            variations: [0; MAX_AMSHA_REQUESTS],
        }
    }
}

// ---------------------------------------------------------------------------
// Shadbala & Vimsopaka types
// ---------------------------------------------------------------------------

/// Shadbala entry for a single sapta graha.
#[derive(Debug, Clone, Copy)]
pub struct ShadbalaEntry {
    pub graha: Graha,
    pub sthana: SthanaBalaBreakdown,
    pub dig: f64,
    pub kala: KalaBalaBreakdown,
    pub cheshta: f64,
    pub naisargika: f64,
    pub drik: f64,
    pub total_shashtiamsas: f64,
    pub total_rupas: f64,
    pub required_strength: f64,
    pub is_strong: bool,
}

impl ShadbalaEntry {
    pub fn from_breakdown(graha: Graha, b: &ShadbalaBreakdown) -> Self {
        Self {
            graha,
            sthana: b.sthana,
            dig: b.dig,
            kala: b.kala,
            cheshta: b.cheshta,
            naisargika: b.naisargika,
            drik: b.drik,
            total_shashtiamsas: b.total_shashtiamsas,
            total_rupas: b.total_rupas,
            required_strength: b.required_strength,
            is_strong: b.is_strong,
        }
    }
}

/// Shadbala result for all 7 sapta grahas.
#[derive(Debug, Clone, Copy)]
pub struct ShadbalaResult {
    pub entries: [ShadbalaEntry; 7],
}

/// Vimsopaka entry for a single graha.
#[derive(Debug, Clone, Copy)]
pub struct VimsopakaEntry {
    pub graha: Graha,
    pub shadvarga: f64,
    pub saptavarga: f64,
    pub dashavarga: f64,
    pub shodasavarga: f64,
}

/// Vimsopaka result for all 9 navagrahas.
#[derive(Debug, Clone, Copy)]
pub struct VimsopakaResult {
    pub entries: [VimsopakaEntry; 9],
}

// ---------------------------------------------------------------------------
// Full kundali config/result
// ---------------------------------------------------------------------------

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
    /// Include special lagnas section.
    pub include_special_lagnas: bool,
    /// Include amsha (divisional chart) section.
    pub include_amshas: bool,
    /// Include shadbala section (sapta grahas only).
    pub include_shadbala: bool,
    /// Include vimsopaka bala section (navagraha).
    pub include_vimsopaka: bool,
    /// Include avastha (planetary state) section.
    pub include_avastha: bool,
    /// Node dignity policy for vimsopaka and avastha.
    pub node_dignity_policy: NodeDignityPolicy,
    /// Config passed to graha positions computation.
    pub graha_positions_config: GrahaPositionsConfig,
    /// Config passed to bindus computation.
    pub bindus_config: BindusConfig,
    /// Config passed to drishti computation.
    pub drishti_config: DrishtiConfig,
    /// Scope flags for amsha charts.
    pub amsha_scope: AmshaChartScope,
    /// Which amshas to compute.
    pub amsha_selection: AmshaSelectionConfig,
}

impl Default for FullKundaliConfig {
    fn default() -> Self {
        Self {
            include_graha_positions: true,
            include_bindus: true,
            include_drishti: true,
            include_ashtakavarga: true,
            include_upagrahas: true,
            include_special_lagnas: true,
            include_amshas: false,
            include_shadbala: false,
            include_vimsopaka: false,
            include_avastha: false,
            node_dignity_policy: NodeDignityPolicy::default(),
            graha_positions_config: GrahaPositionsConfig::default(),
            bindus_config: BindusConfig::default(),
            drishti_config: DrishtiConfig::default(),
            amsha_scope: AmshaChartScope::default(),
            amsha_selection: AmshaSelectionConfig::default(),
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
    /// Present when `FullKundaliConfig::include_special_lagnas` is true.
    pub special_lagnas: Option<AllSpecialLagnas>,
    /// Present when `FullKundaliConfig::include_amshas` is true.
    pub amshas: Option<AmshaResult>,
    /// Present when `FullKundaliConfig::include_shadbala` is true.
    pub shadbala: Option<ShadbalaResult>,
    /// Present when `FullKundaliConfig::include_vimsopaka` is true.
    pub vimsopaka: Option<VimsopakaResult>,
    /// Present when `FullKundaliConfig::include_avastha` is true.
    pub avastha: Option<AllGrahaAvasthas>,
}
