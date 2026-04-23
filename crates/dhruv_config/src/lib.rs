//! Layered configuration loader and resolver.
//!
//! Precedence (highest -> lowest): explicit overrides, operation config,
//! common config, recommended defaults.

use std::collections::BTreeMap;
use std::env;
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::fs;
use std::path::{Path, PathBuf};

use dhruv_core::{Body, EngineConfig};
use dhruv_frames::{PrecessionModel, ReferencePlane};
use dhruv_search::{
    AmshaSelectionConfig, BindusConfig, ConjunctionConfig, DashaSelectionConfig, DashaSnapshotTime,
    DrishtiConfig, FullKundaliConfig, GrahaPositionsConfig, GrahanConfig, SankrantiConfig,
    StationaryConfig,
};
use dhruv_tara::{TaraAccuracy, TaraConfig};
use dhruv_time::UtcTime;
use dhruv_vedic_base::bhava_types::ALL_BHAVA_SYSTEMS;
use dhruv_vedic_base::dasha::MAX_DASHA_SYSTEMS;
use dhruv_vedic_base::{
    AyanamshaSystem, BhavaConfig, BhavaReferenceMode, BhavaStartingPoint, ChandraBeneficRule,
    NodeDignityPolicy, RiseSetConfig, SunLimb,
};
use serde::Deserialize;

const CURRENT_CONFIG_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigFormat {
    Toml,
    Json,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigSource {
    Explicit,
    Operation,
    Common,
    RecommendedDefault,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EffectiveConfig<T> {
    pub value: T,
    pub source_by_field: BTreeMap<String, ConfigSource>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DefaultsMode {
    Recommended,
    None,
}

#[derive(Debug)]
pub enum ConfigError {
    Io(String),
    Parse(String),
    InvalidConfig(String),
    UnsupportedVersion(u32),
    MissingRequired(&'static str),
    InvalidEnumValue { field: &'static str, value: String },
    AmbiguousConfigFiles { toml: PathBuf, json: PathBuf },
}

impl Display for ConfigError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(msg) => write!(f, "io error: {msg}"),
            Self::Parse(msg) => write!(f, "parse error: {msg}"),
            Self::InvalidConfig(msg) => write!(f, "invalid config: {msg}"),
            Self::UnsupportedVersion(v) => write!(f, "unsupported config version: {v}"),
            Self::MissingRequired(field) => write!(f, "missing required field: {field}"),
            Self::InvalidEnumValue { field, value } => {
                write!(f, "invalid value '{value}' for field '{field}'")
            }
            Self::AmbiguousConfigFiles { toml, json } => write!(
                f,
                "ambiguous config files found (both TOML and JSON exist): {} and {}",
                toml.display(),
                json.display()
            ),
        }
    }
}

impl Error for ConfigError {}

#[derive(Debug, Clone)]
pub struct LoadedConfig {
    pub path: PathBuf,
    pub format: ConfigFormat,
    pub file: DhruvConfigFile,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DhruvConfigFile {
    #[serde(default = "default_config_version")]
    pub version: u32,
    #[serde(default)]
    pub common: CommonConfigPatch,
    #[serde(default)]
    pub operations: OperationConfigPatchSet,
}

fn default_config_version() -> u32 {
    CURRENT_CONFIG_VERSION
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CommonConfigPatch {
    pub engine: Option<EngineConfigPatch>,
    pub ayanamsha_system: Option<EnumInput>,
    pub use_nutation: Option<bool>,
    pub precession_model: Option<EnumInput>,
    pub reference_plane: Option<EnumInput>,
    pub step_size_days: Option<f64>,
    pub max_iterations: Option<u32>,
    pub convergence_days: Option<f64>,
    pub cache_capacity: Option<usize>,
    pub strict_validation: Option<bool>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OperationConfigPatchSet {
    #[serde(default)]
    pub conjunction: ConjunctionConfigPatch,
    #[serde(default)]
    pub grahan: GrahanConfigPatch,
    #[serde(default)]
    pub stationary: StationaryConfigPatch,
    #[serde(default)]
    pub sankranti: SankrantiConfigPatch,
    #[serde(default)]
    pub riseset: RiseSetConfigPatch,
    #[serde(default)]
    pub bhava: BhavaConfigPatch,
    #[serde(default)]
    pub tara: TaraConfigPatch,
    #[serde(default)]
    pub graha_positions: GrahaPositionsConfigPatch,
    #[serde(default)]
    pub bindus: BindusConfigPatch,
    #[serde(default)]
    pub drishti: DrishtiConfigPatch,
    #[serde(default)]
    pub full_kundali: FullKundaliConfigPatch,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EngineConfigPatch {
    pub spk_paths: Option<Vec<String>>,
    pub lsk_path: Option<String>,
    pub cache_capacity: Option<usize>,
    pub strict_validation: Option<bool>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConjunctionConfigPatch {
    pub target_separation_deg: Option<f64>,
    pub step_size_days: Option<f64>,
    pub max_iterations: Option<u32>,
    pub convergence_days: Option<f64>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GrahanConfigPatch {
    pub include_penumbral: Option<bool>,
    pub include_peak_details: Option<bool>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StationaryConfigPatch {
    pub step_size_days: Option<f64>,
    pub max_iterations: Option<u32>,
    pub convergence_days: Option<f64>,
    pub numerical_step_days: Option<f64>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SankrantiConfigPatch {
    pub ayanamsha_system: Option<EnumInput>,
    pub use_nutation: Option<bool>,
    pub precession_model: Option<EnumInput>,
    pub reference_plane: Option<EnumInput>,
    pub step_size_days: Option<f64>,
    pub max_iterations: Option<u32>,
    pub convergence_days: Option<f64>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RiseSetConfigPatch {
    pub use_refraction: Option<bool>,
    pub sun_limb: Option<EnumInput>,
    pub altitude_correction: Option<bool>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BhavaConfigPatch {
    pub system: Option<EnumInput>,
    pub starting_point_kind: Option<EnumInput>,
    pub starting_point_body_code: Option<i32>,
    pub starting_point_custom_deg: Option<f64>,
    pub reference_mode: Option<EnumInput>,
    pub use_rashi_bhava_for_bala_avastha: Option<bool>,
    pub include_node_aspects_for_drik_bala: Option<bool>,
    pub divide_guru_buddh_drishti_by_4_for_drik_bala: Option<bool>,
    pub chandra_benefic_rule: Option<EnumInput>,
    pub include_rashi_bhava_results: Option<bool>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TaraConfigPatch {
    pub accuracy: Option<EnumInput>,
    pub apply_parallax: Option<bool>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GrahaPositionsConfigPatch {
    pub include_nakshatra: Option<bool>,
    pub include_lagna: Option<bool>,
    pub include_outer_planets: Option<bool>,
    pub include_bhava: Option<bool>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BindusConfigPatch {
    pub include_nakshatra: Option<bool>,
    pub include_bhava: Option<bool>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DrishtiConfigPatch {
    pub include_bhava: Option<bool>,
    pub include_lagna: Option<bool>,
    pub include_bindus: Option<bool>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AmshaSelectionConfigPatch {
    pub count: Option<u8>,
    pub codes: Option<Vec<u16>>,
    pub variations: Option<Vec<u8>>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UtcTimeConfigValue {
    pub year: i32,
    pub month: u32,
    pub day: u32,
    pub hour: u32,
    pub minute: u32,
    pub second: f64,
}

impl TryFrom<UtcTimeConfigValue> for UtcTime {
    type Error = ConfigError;

    fn try_from(value: UtcTimeConfigValue) -> Result<Self, Self::Error> {
        UtcTime::try_new(
            value.year,
            value.month,
            value.day,
            value.hour,
            value.minute,
            value.second,
            None,
        )
        .map_err(|e| ConfigError::InvalidConfig(format!("invalid snapshot_utc: {e}")))
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DashaSelectionConfigPatch {
    pub count: Option<u8>,
    pub systems: Option<Vec<u8>>,
    pub max_levels: Option<Vec<u8>>,
    pub max_level: Option<u8>,
    pub level_methods: Option<Vec<u8>>,
    pub yogini_scheme: Option<u8>,
    pub use_abhijit: Option<u8>,
    pub snapshot_utc: Option<UtcTimeConfigValue>,
    pub snapshot_jd_utc: Option<f64>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FullKundaliConfigPatch {
    pub include_bhava_cusps: Option<bool>,
    pub include_graha_positions: Option<bool>,
    pub include_bindus: Option<bool>,
    pub include_drishti: Option<bool>,
    pub include_ashtakavarga: Option<bool>,
    pub include_upagrahas: Option<bool>,
    pub include_sphutas: Option<bool>,
    pub include_special_lagnas: Option<bool>,
    pub include_amshas: Option<bool>,
    pub include_shadbala: Option<bool>,
    pub include_bhavabala: Option<bool>,
    pub include_vimsopaka: Option<bool>,
    pub include_avastha: Option<bool>,
    pub include_charakaraka: Option<bool>,
    pub charakaraka_scheme: Option<EnumInput>,
    pub include_panchang: Option<bool>,
    pub include_calendar: Option<bool>,
    pub include_dasha: Option<bool>,
    pub node_dignity_policy: Option<EnumInput>,
    pub graha_positions: Option<GrahaPositionsConfigPatch>,
    pub bindus: Option<BindusConfigPatch>,
    pub drishti: Option<DrishtiConfigPatch>,
    pub amsha_selection: Option<AmshaSelectionConfigPatch>,
    pub dasha: Option<DashaSelectionConfigPatch>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum EnumInput {
    Int(i64),
    Str(String),
}

impl EnumInput {
    fn as_lower(&self) -> String {
        match self {
            Self::Int(v) => v.to_string(),
            Self::Str(s) => s.to_ascii_lowercase(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ConfigResolver {
    file: DhruvConfigFile,
    defaults_mode: DefaultsMode,
}

impl ConfigResolver {
    pub fn new(file: DhruvConfigFile, defaults_mode: DefaultsMode) -> Self {
        Self {
            file,
            defaults_mode,
        }
    }

    pub fn file(&self) -> &DhruvConfigFile {
        &self.file
    }

    pub fn defaults_mode(&self) -> DefaultsMode {
        self.defaults_mode
    }

    pub fn resolve_engine(
        &self,
        explicit: Option<EngineConfigPatch>,
    ) -> Result<EffectiveConfig<EngineConfig>, ConfigError> {
        let op = self.file.common.engine.clone().unwrap_or_default();
        let explicit = explicit.unwrap_or_default();

        let (spk_paths, spk_source) = choose_vec_string(
            explicit.spk_paths,
            op.spk_paths,
            None,
            if self.defaults_mode == DefaultsMode::Recommended {
                Some(Vec::new())
            } else {
                None
            },
            "engine.spk_paths",
        )?;
        if spk_paths.is_empty() {
            return Err(ConfigError::MissingRequired("engine.spk_paths"));
        }

        let (lsk_path, lsk_source) = choose_string(
            explicit.lsk_path,
            op.lsk_path,
            None,
            None,
            "engine.lsk_path",
        )?;

        let (cache_capacity, cache_source) = choose_copy(
            explicit.cache_capacity,
            op.cache_capacity,
            self.file.common.cache_capacity,
            if self.defaults_mode == DefaultsMode::Recommended {
                Some(256)
            } else {
                None
            },
            "engine.cache_capacity",
        )?;

        let (strict_validation, strict_source) = choose_copy(
            explicit.strict_validation,
            op.strict_validation,
            self.file.common.strict_validation,
            if self.defaults_mode == DefaultsMode::Recommended {
                Some(true)
            } else {
                None
            },
            "engine.strict_validation",
        )?;

        let mut source = BTreeMap::new();
        source.insert("spk_paths".to_string(), spk_source);
        source.insert("lsk_path".to_string(), lsk_source);
        source.insert("cache_capacity".to_string(), cache_source);
        source.insert("strict_validation".to_string(), strict_source);

        Ok(EffectiveConfig {
            value: EngineConfig {
                spk_paths: spk_paths.into_iter().map(PathBuf::from).collect(),
                lsk_path: PathBuf::from(lsk_path),
                cache_capacity,
                strict_validation,
            },
            source_by_field: source,
        })
    }

    pub fn resolve_conjunction(
        &self,
        explicit: Option<ConjunctionConfigPatch>,
    ) -> Result<EffectiveConfig<ConjunctionConfig>, ConfigError> {
        let explicit = explicit.unwrap_or_default();
        let op = &self.file.operations.conjunction;

        let (target_separation_deg, target_source) = choose_copy(
            explicit.target_separation_deg,
            op.target_separation_deg,
            None,
            recommended(self.defaults_mode, 0.0),
            "conjunction.target_separation_deg",
        )?;
        let (step_size_days, step_source) = choose_copy(
            explicit.step_size_days,
            op.step_size_days,
            self.file.common.step_size_days,
            recommended(self.defaults_mode, 1.0),
            "conjunction.step_size_days",
        )?;
        let (max_iterations, iter_source) = choose_copy(
            explicit.max_iterations,
            op.max_iterations,
            self.file.common.max_iterations,
            recommended(self.defaults_mode, 50),
            "conjunction.max_iterations",
        )?;
        let (convergence_days, conv_source) = choose_copy(
            explicit.convergence_days,
            op.convergence_days,
            self.file.common.convergence_days,
            recommended(self.defaults_mode, 1e-8),
            "conjunction.convergence_days",
        )?;

        let cfg = ConjunctionConfig {
            target_separation_deg,
            step_size_days,
            max_iterations,
            convergence_days,
        };

        let mut source = BTreeMap::new();
        source.insert("target_separation_deg".to_string(), target_source);
        source.insert("step_size_days".to_string(), step_source);
        source.insert("max_iterations".to_string(), iter_source);
        source.insert("convergence_days".to_string(), conv_source);

        Ok(EffectiveConfig {
            value: cfg,
            source_by_field: source,
        })
    }

    pub fn resolve_grahan(
        &self,
        explicit: Option<GrahanConfigPatch>,
    ) -> Result<EffectiveConfig<GrahanConfig>, ConfigError> {
        let explicit = explicit.unwrap_or_default();
        let op = &self.file.operations.grahan;

        let (include_penumbral, p_source) = choose_copy(
            explicit.include_penumbral,
            op.include_penumbral,
            None,
            recommended(self.defaults_mode, true),
            "grahan.include_penumbral",
        )?;
        let (include_peak_details, d_source) = choose_copy(
            explicit.include_peak_details,
            op.include_peak_details,
            None,
            recommended(self.defaults_mode, true),
            "grahan.include_peak_details",
        )?;

        let mut source = BTreeMap::new();
        source.insert("include_penumbral".to_string(), p_source);
        source.insert("include_peak_details".to_string(), d_source);

        Ok(EffectiveConfig {
            value: GrahanConfig {
                include_penumbral,
                include_peak_details,
            },
            source_by_field: source,
        })
    }

    pub fn resolve_stationary(
        &self,
        explicit: Option<StationaryConfigPatch>,
    ) -> Result<EffectiveConfig<StationaryConfig>, ConfigError> {
        let explicit = explicit.unwrap_or_default();
        let op = &self.file.operations.stationary;

        let (step_size_days, step_source) = choose_copy(
            explicit.step_size_days,
            op.step_size_days,
            self.file.common.step_size_days,
            recommended(self.defaults_mode, 1.0),
            "stationary.step_size_days",
        )?;
        let (max_iterations, iter_source) = choose_copy(
            explicit.max_iterations,
            op.max_iterations,
            self.file.common.max_iterations,
            recommended(self.defaults_mode, 50),
            "stationary.max_iterations",
        )?;
        let (convergence_days, conv_source) = choose_copy(
            explicit.convergence_days,
            op.convergence_days,
            self.file.common.convergence_days,
            recommended(self.defaults_mode, 1e-8),
            "stationary.convergence_days",
        )?;
        let (numerical_step_days, nstep_source) = choose_copy(
            explicit.numerical_step_days,
            op.numerical_step_days,
            None,
            recommended(self.defaults_mode, 0.01),
            "stationary.numerical_step_days",
        )?;

        let mut source = BTreeMap::new();
        source.insert("step_size_days".to_string(), step_source);
        source.insert("max_iterations".to_string(), iter_source);
        source.insert("convergence_days".to_string(), conv_source);
        source.insert("numerical_step_days".to_string(), nstep_source);

        Ok(EffectiveConfig {
            value: StationaryConfig {
                step_size_days,
                max_iterations,
                convergence_days,
                numerical_step_days,
            },
            source_by_field: source,
        })
    }

    pub fn resolve_sankranti(
        &self,
        explicit: Option<SankrantiConfigPatch>,
    ) -> Result<EffectiveConfig<SankrantiConfig>, ConfigError> {
        let explicit = explicit.unwrap_or_default();
        let op = &self.file.operations.sankranti;

        let (system_input, system_source) = choose_enum(
            explicit.ayanamsha_system,
            op.ayanamsha_system.clone(),
            self.file.common.ayanamsha_system.clone(),
            recommended_enum(self.defaults_mode, EnumInput::Int(0)),
            "sankranti.ayanamsha_system",
        )?;
        let ayanamsha_system = parse_ayanamsha_system(&system_input, "sankranti.ayanamsha_system")?;

        let (use_nutation, nut_source) = choose_copy(
            explicit.use_nutation,
            op.use_nutation,
            self.file.common.use_nutation,
            recommended(self.defaults_mode, false),
            "sankranti.use_nutation",
        )?;

        let (model_input, model_source) = choose_enum(
            explicit.precession_model,
            op.precession_model.clone(),
            self.file.common.precession_model.clone(),
            recommended_enum(
                self.defaults_mode,
                EnumInput::Str("vondrak2011".to_string()),
            ),
            "sankranti.precession_model",
        )?;
        let precession_model = parse_precession_model(&model_input, "sankranti.precession_model")?;

        let (plane_input, plane_source) = choose_enum(
            explicit.reference_plane,
            op.reference_plane.clone(),
            self.file.common.reference_plane.clone(),
            recommended_enum(
                self.defaults_mode,
                EnumInput::Str("default-from-ayanamsha".to_string()),
            ),
            "sankranti.reference_plane",
        )?;
        let reference_plane = if let EnumInput::Str(s) = &plane_input {
            if s.eq_ignore_ascii_case("default-from-ayanamsha") {
                ayanamsha_system.default_reference_plane()
            } else {
                parse_reference_plane(&plane_input, "sankranti.reference_plane")?
            }
        } else {
            parse_reference_plane(&plane_input, "sankranti.reference_plane")?
        };

        let (step_size_days, step_source) = choose_copy(
            explicit.step_size_days,
            op.step_size_days,
            self.file.common.step_size_days,
            recommended(self.defaults_mode, 1.0),
            "sankranti.step_size_days",
        )?;
        let (max_iterations, iter_source) = choose_copy(
            explicit.max_iterations,
            op.max_iterations,
            self.file.common.max_iterations,
            recommended(self.defaults_mode, 50),
            "sankranti.max_iterations",
        )?;
        let (convergence_days, conv_source) = choose_copy(
            explicit.convergence_days,
            op.convergence_days,
            self.file.common.convergence_days,
            recommended(self.defaults_mode, 1e-8),
            "sankranti.convergence_days",
        )?;

        let cfg = SankrantiConfig {
            ayanamsha_system,
            use_nutation,
            precession_model,
            reference_plane,
            step_size_days,
            max_iterations,
            convergence_days,
        };

        let mut source = BTreeMap::new();
        source.insert("ayanamsha_system".to_string(), system_source);
        source.insert("use_nutation".to_string(), nut_source);
        source.insert("precession_model".to_string(), model_source);
        source.insert("reference_plane".to_string(), plane_source);
        source.insert("step_size_days".to_string(), step_source);
        source.insert("max_iterations".to_string(), iter_source);
        source.insert("convergence_days".to_string(), conv_source);

        Ok(EffectiveConfig {
            value: cfg,
            source_by_field: source,
        })
    }

    pub fn resolve_riseset(
        &self,
        explicit: Option<RiseSetConfigPatch>,
    ) -> Result<EffectiveConfig<RiseSetConfig>, ConfigError> {
        let explicit = explicit.unwrap_or_default();
        let op = &self.file.operations.riseset;

        let (use_refraction, r_source) = choose_copy(
            explicit.use_refraction,
            op.use_refraction,
            None,
            recommended(self.defaults_mode, true),
            "riseset.use_refraction",
        )?;

        let (sun_limb_input, limb_source) = choose_enum(
            explicit.sun_limb,
            op.sun_limb.clone(),
            None,
            recommended_enum(self.defaults_mode, EnumInput::Str("upper-limb".to_string())),
            "riseset.sun_limb",
        )?;
        let sun_limb = parse_sun_limb(&sun_limb_input, "riseset.sun_limb")?;

        let (altitude_correction, a_source) = choose_copy(
            explicit.altitude_correction,
            op.altitude_correction,
            None,
            recommended(self.defaults_mode, true),
            "riseset.altitude_correction",
        )?;

        let mut source = BTreeMap::new();
        source.insert("use_refraction".to_string(), r_source);
        source.insert("sun_limb".to_string(), limb_source);
        source.insert("altitude_correction".to_string(), a_source);

        Ok(EffectiveConfig {
            value: RiseSetConfig {
                use_refraction,
                sun_limb,
                altitude_correction,
            },
            source_by_field: source,
        })
    }

    pub fn resolve_bhava(
        &self,
        explicit: Option<BhavaConfigPatch>,
    ) -> Result<EffectiveConfig<BhavaConfig>, ConfigError> {
        let explicit = explicit.unwrap_or_default();
        let op = &self.file.operations.bhava;

        let (system_input, system_source) = choose_enum(
            explicit.system,
            op.system.clone(),
            None,
            recommended_enum(self.defaults_mode, EnumInput::Int(0)),
            "bhava.system",
        )?;
        let system = parse_bhava_system(&system_input, "bhava.system")?;

        let (sp_kind_input, sp_kind_source) = choose_enum(
            explicit.starting_point_kind,
            op.starting_point_kind.clone(),
            None,
            recommended_enum(self.defaults_mode, EnumInput::Str("lagna".to_string())),
            "bhava.starting_point_kind",
        )?;

        let sp_body_code = explicit
            .starting_point_body_code
            .or(op.starting_point_body_code);
        let sp_custom_deg = explicit
            .starting_point_custom_deg
            .or(op.starting_point_custom_deg);

        let starting_point = parse_starting_point(
            &sp_kind_input,
            sp_body_code,
            sp_custom_deg,
            "bhava.starting_point_kind",
        )?;

        let (reference_mode_input, mode_source) = choose_enum(
            explicit.reference_mode,
            op.reference_mode.clone(),
            None,
            recommended_enum(
                self.defaults_mode,
                EnumInput::Str("start-of-first".to_string()),
            ),
            "bhava.reference_mode",
        )?;
        let reference_mode =
            parse_bhava_reference_mode(&reference_mode_input, "bhava.reference_mode")?;

        let mut source = BTreeMap::new();
        source.insert("system".to_string(), system_source);
        source.insert("starting_point_kind".to_string(), sp_kind_source);
        source.insert("reference_mode".to_string(), mode_source);
        let use_rashi_bhava_for_bala_avastha = explicit
            .use_rashi_bhava_for_bala_avastha
            .or(op.use_rashi_bhava_for_bala_avastha)
            .unwrap_or(true);
        let include_node_aspects_for_drik_bala = explicit
            .include_node_aspects_for_drik_bala
            .or(op.include_node_aspects_for_drik_bala)
            .unwrap_or(false);
        let divide_guru_buddh_drishti_by_4_for_drik_bala = explicit
            .divide_guru_buddh_drishti_by_4_for_drik_bala
            .or(op.divide_guru_buddh_drishti_by_4_for_drik_bala)
            .unwrap_or(true);
        let chandra_rule_input = explicit
            .chandra_benefic_rule
            .or_else(|| op.chandra_benefic_rule.clone())
            .unwrap_or(EnumInput::Str("brightness-72".to_string()));
        let chandra_benefic_rule =
            parse_chandra_benefic_rule(&chandra_rule_input, "bhava.chandra_benefic_rule")?;
        let include_rashi_bhava_results = explicit
            .include_rashi_bhava_results
            .or(op.include_rashi_bhava_results)
            .unwrap_or(true);

        Ok(EffectiveConfig {
            value: BhavaConfig {
                system,
                starting_point,
                reference_mode,
                use_rashi_bhava_for_bala_avastha,
                include_node_aspects_for_drik_bala,
                divide_guru_buddh_drishti_by_4_for_drik_bala,
                chandra_benefic_rule,
                include_rashi_bhava_results,
            },
            source_by_field: source,
        })
    }

    pub fn resolve_tara(
        &self,
        explicit: Option<TaraConfigPatch>,
    ) -> Result<EffectiveConfig<TaraConfig>, ConfigError> {
        let explicit = explicit.unwrap_or_default();
        let op = &self.file.operations.tara;

        let (accuracy_input, acc_source) = choose_enum(
            explicit.accuracy,
            op.accuracy.clone(),
            None,
            recommended_enum(
                self.defaults_mode,
                EnumInput::Str("astrometric".to_string()),
            ),
            "tara.accuracy",
        )?;
        let accuracy = parse_tara_accuracy(&accuracy_input, "tara.accuracy")?;

        let (apply_parallax, p_source) = choose_copy(
            explicit.apply_parallax,
            op.apply_parallax,
            None,
            recommended(self.defaults_mode, false),
            "tara.apply_parallax",
        )?;

        let mut source = BTreeMap::new();
        source.insert("accuracy".to_string(), acc_source);
        source.insert("apply_parallax".to_string(), p_source);

        Ok(EffectiveConfig {
            value: TaraConfig {
                accuracy,
                apply_parallax,
            },
            source_by_field: source,
        })
    }

    pub fn resolve_graha_positions(
        &self,
        explicit: Option<GrahaPositionsConfigPatch>,
    ) -> Result<EffectiveConfig<GrahaPositionsConfig>, ConfigError> {
        let explicit = explicit.unwrap_or_default();
        let op = &self.file.operations.graha_positions;

        let (include_nakshatra, s1) = choose_copy(
            explicit.include_nakshatra,
            op.include_nakshatra,
            None,
            recommended(self.defaults_mode, false),
            "graha_positions.include_nakshatra",
        )?;
        let (include_lagna, s2) = choose_copy(
            explicit.include_lagna,
            op.include_lagna,
            None,
            recommended(self.defaults_mode, false),
            "graha_positions.include_lagna",
        )?;
        let (include_outer_planets, s3) = choose_copy(
            explicit.include_outer_planets,
            op.include_outer_planets,
            None,
            recommended(self.defaults_mode, false),
            "graha_positions.include_outer_planets",
        )?;
        let (include_bhava, s4) = choose_copy(
            explicit.include_bhava,
            op.include_bhava,
            None,
            recommended(self.defaults_mode, false),
            "graha_positions.include_bhava",
        )?;

        let mut source = BTreeMap::new();
        source.insert("include_nakshatra".to_string(), s1);
        source.insert("include_lagna".to_string(), s2);
        source.insert("include_outer_planets".to_string(), s3);
        source.insert("include_bhava".to_string(), s4);

        Ok(EffectiveConfig {
            value: GrahaPositionsConfig {
                include_nakshatra,
                include_lagna,
                include_outer_planets,
                include_bhava,
            },
            source_by_field: source,
        })
    }

    pub fn resolve_bindus(
        &self,
        explicit: Option<BindusConfigPatch>,
    ) -> Result<EffectiveConfig<BindusConfig>, ConfigError> {
        let explicit = explicit.unwrap_or_default();
        let op = &self.file.operations.bindus;

        let (include_nakshatra, s1) = choose_copy(
            explicit.include_nakshatra,
            op.include_nakshatra,
            None,
            recommended(self.defaults_mode, false),
            "bindus.include_nakshatra",
        )?;
        let (include_bhava, s2) = choose_copy(
            explicit.include_bhava,
            op.include_bhava,
            None,
            recommended(self.defaults_mode, false),
            "bindus.include_bhava",
        )?;

        let mut source = BTreeMap::new();
        source.insert("include_nakshatra".to_string(), s1);
        source.insert("include_bhava".to_string(), s2);

        Ok(EffectiveConfig {
            value: BindusConfig {
                include_nakshatra,
                include_bhava,
                upagraha_config: dhruv_vedic_base::TimeUpagrahaConfig::default(),
            },
            source_by_field: source,
        })
    }

    pub fn resolve_drishti(
        &self,
        explicit: Option<DrishtiConfigPatch>,
    ) -> Result<EffectiveConfig<DrishtiConfig>, ConfigError> {
        let explicit = explicit.unwrap_or_default();
        let op = &self.file.operations.drishti;

        let (include_bhava, s1) = choose_copy(
            explicit.include_bhava,
            op.include_bhava,
            None,
            recommended(self.defaults_mode, false),
            "drishti.include_bhava",
        )?;
        let (include_lagna, s2) = choose_copy(
            explicit.include_lagna,
            op.include_lagna,
            None,
            recommended(self.defaults_mode, false),
            "drishti.include_lagna",
        )?;
        let (include_bindus, s3) = choose_copy(
            explicit.include_bindus,
            op.include_bindus,
            None,
            recommended(self.defaults_mode, false),
            "drishti.include_bindus",
        )?;

        let mut source = BTreeMap::new();
        source.insert("include_bhava".to_string(), s1);
        source.insert("include_lagna".to_string(), s2);
        source.insert("include_bindus".to_string(), s3);

        Ok(EffectiveConfig {
            value: DrishtiConfig {
                include_bhava,
                include_lagna,
                include_bindus,
            },
            source_by_field: source,
        })
    }

    pub fn resolve_full_kundali(
        &self,
        explicit: Option<FullKundaliConfigPatch>,
    ) -> Result<EffectiveConfig<FullKundaliConfig>, ConfigError> {
        let explicit = explicit.unwrap_or_default();
        let op = &self.file.operations.full_kundali;
        let defaults = FullKundaliConfig::default();

        let mut source = BTreeMap::new();

        let include_bhava_cusps = layered_bool(
            explicit.include_bhava_cusps,
            op.include_bhava_cusps,
            defaults.include_bhava_cusps,
            "include_bhava_cusps",
            &mut source,
        );
        let include_graha_positions = layered_bool(
            explicit.include_graha_positions,
            op.include_graha_positions,
            defaults.include_graha_positions,
            "include_graha_positions",
            &mut source,
        );
        let include_bindus = layered_bool(
            explicit.include_bindus,
            op.include_bindus,
            defaults.include_bindus,
            "include_bindus",
            &mut source,
        );
        let include_drishti = layered_bool(
            explicit.include_drishti,
            op.include_drishti,
            defaults.include_drishti,
            "include_drishti",
            &mut source,
        );
        let include_ashtakavarga = layered_bool(
            explicit.include_ashtakavarga,
            op.include_ashtakavarga,
            defaults.include_ashtakavarga,
            "include_ashtakavarga",
            &mut source,
        );
        let include_upagrahas = layered_bool(
            explicit.include_upagrahas,
            op.include_upagrahas,
            defaults.include_upagrahas,
            "include_upagrahas",
            &mut source,
        );
        let include_sphutas = layered_bool(
            explicit.include_sphutas,
            op.include_sphutas,
            defaults.include_sphutas,
            "include_sphutas",
            &mut source,
        );
        let include_special_lagnas = layered_bool(
            explicit.include_special_lagnas,
            op.include_special_lagnas,
            defaults.include_special_lagnas,
            "include_special_lagnas",
            &mut source,
        );
        let include_amshas = layered_bool(
            explicit.include_amshas,
            op.include_amshas,
            defaults.include_amshas,
            "include_amshas",
            &mut source,
        );
        let include_shadbala = layered_bool(
            explicit.include_shadbala,
            op.include_shadbala,
            defaults.include_shadbala,
            "include_shadbala",
            &mut source,
        );
        let include_bhavabala = layered_bool(
            explicit.include_bhavabala,
            op.include_bhavabala,
            defaults.include_bhavabala,
            "include_bhavabala",
            &mut source,
        );
        let include_vimsopaka = layered_bool(
            explicit.include_vimsopaka,
            op.include_vimsopaka,
            defaults.include_vimsopaka,
            "include_vimsopaka",
            &mut source,
        );
        let include_avastha = layered_bool(
            explicit.include_avastha,
            op.include_avastha,
            defaults.include_avastha,
            "include_avastha",
            &mut source,
        );
        let include_charakaraka = layered_bool(
            explicit.include_charakaraka,
            op.include_charakaraka,
            defaults.include_charakaraka,
            "include_charakaraka",
            &mut source,
        );
        let include_panchang = layered_bool(
            explicit.include_panchang,
            op.include_panchang,
            defaults.include_panchang,
            "include_panchang",
            &mut source,
        );
        let include_calendar = layered_bool(
            explicit.include_calendar,
            op.include_calendar,
            defaults.include_calendar,
            "include_calendar",
            &mut source,
        );
        let include_dasha = layered_bool(
            explicit.include_dasha,
            op.include_dasha,
            defaults.include_dasha,
            "include_dasha",
            &mut source,
        );

        let node_dignity_policy = if let Some(v) = explicit.node_dignity_policy {
            source.insert("node_dignity_policy".to_string(), ConfigSource::Explicit);
            parse_node_dignity_policy(&v, "full_kundali.node_dignity_policy")?
        } else if let Some(v) = &op.node_dignity_policy {
            source.insert("node_dignity_policy".to_string(), ConfigSource::Operation);
            parse_node_dignity_policy(v, "full_kundali.node_dignity_policy")?
        } else {
            source.insert(
                "node_dignity_policy".to_string(),
                ConfigSource::RecommendedDefault,
            );
            defaults.node_dignity_policy
        };

        let charakaraka_scheme = if let Some(v) = explicit.charakaraka_scheme {
            source.insert("charakaraka_scheme".to_string(), ConfigSource::Explicit);
            parse_charakaraka_scheme(&v, "full_kundali.charakaraka_scheme")?
        } else if let Some(v) = &op.charakaraka_scheme {
            source.insert("charakaraka_scheme".to_string(), ConfigSource::Operation);
            parse_charakaraka_scheme(v, "full_kundali.charakaraka_scheme")?
        } else {
            source.insert(
                "charakaraka_scheme".to_string(),
                ConfigSource::RecommendedDefault,
            );
            defaults.charakaraka_scheme
        };

        let explicit_full_graha_positions = explicit.graha_positions.clone();
        let op_full_graha_positions = op.graha_positions.clone();
        let mut graha_positions_cfg = self
            .resolve_graha_positions(merge_patch(
                explicit_full_graha_positions.clone(),
                op_full_graha_positions.clone(),
            ))?
            .value;
        if explicit_full_graha_positions
            .as_ref()
            .and_then(|patch| patch.include_lagna)
            .is_none()
            && op_full_graha_positions
                .as_ref()
                .and_then(|patch| patch.include_lagna)
                .is_none()
        {
            graha_positions_cfg.include_lagna = defaults.graha_positions_config.include_lagna;
        }
        let bindus_cfg = self
            .resolve_bindus(merge_patch(explicit.bindus, op.bindus.clone()))?
            .value;
        let drishti_cfg = self
            .resolve_drishti(merge_patch(explicit.drishti, op.drishti.clone()))?
            .value;

        let amsha_selection = apply_amsha_selection_patch(
            defaults.amsha_selection,
            merge_patch(explicit.amsha_selection, op.amsha_selection.clone()),
        )?;
        let dasha_config = apply_dasha_selection_patch(
            defaults.dasha_config,
            merge_patch(explicit.dasha, op.dasha.clone()),
        )?;

        Ok(EffectiveConfig {
            value: FullKundaliConfig {
                include_bhava_cusps,
                include_graha_positions,
                include_bindus,
                include_drishti,
                include_ashtakavarga,
                include_upagrahas,
                include_sphutas,
                include_special_lagnas,
                include_amshas,
                include_shadbala,
                include_bhavabala,
                include_vimsopaka,
                include_avastha,
                include_charakaraka,
                charakaraka_scheme,
                include_panchang,
                include_calendar,
                include_dasha,
                node_dignity_policy,
                upagraha_config: dhruv_vedic_base::TimeUpagrahaConfig::default(),
                graha_positions_config: graha_positions_cfg,
                bindus_config: bindus_cfg,
                drishti_config: drishti_cfg,
                amsha_scope: defaults.amsha_scope,
                amsha_selection,
                dasha_config,
            },
            source_by_field: source,
        })
    }
}

pub fn load_from_path(path: &Path) -> Result<LoadedConfig, ConfigError> {
    let content = fs::read_to_string(path)
        .map_err(|e| ConfigError::Io(format!("{}: {e}", path.display())))?;

    let ext = path
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();

    let (format, mut file): (ConfigFormat, DhruvConfigFile) = match ext.as_str() {
        "json" => (
            ConfigFormat::Json,
            serde_json::from_str(&content)
                .map_err(|e| ConfigError::Parse(format!("{}: {e}", path.display())))?,
        ),
        "toml" | "tml" => (
            ConfigFormat::Toml,
            toml::from_str(&content)
                .map_err(|e| ConfigError::Parse(format!("{}: {e}", path.display())))?,
        ),
        _ => {
            if let Ok(file) = toml::from_str::<DhruvConfigFile>(&content) {
                (ConfigFormat::Toml, file)
            } else {
                (
                    ConfigFormat::Json,
                    serde_json::from_str(&content)
                        .map_err(|e| ConfigError::Parse(format!("{}: {e}", path.display())))?,
                )
            }
        }
    };

    if file.version == 0 {
        file.version = CURRENT_CONFIG_VERSION;
    }
    if file.version != CURRENT_CONFIG_VERSION {
        return Err(ConfigError::UnsupportedVersion(file.version));
    }

    Ok(LoadedConfig {
        path: path.to_path_buf(),
        format,
        file,
    })
}

pub fn discover_config_path(
    explicit: Option<&Path>,
    disable: bool,
) -> Result<Option<PathBuf>, ConfigError> {
    if disable {
        return Ok(None);
    }

    if let Some(path) = explicit {
        if path.exists() {
            return Ok(Some(path.to_path_buf()));
        }
        return Err(ConfigError::Io(format!(
            "explicit config path does not exist: {}",
            path.display()
        )));
    }

    if let Ok(env_path) = env::var("DHRUV_CONFIG_FILE")
        && !env_path.trim().is_empty()
    {
        let path = PathBuf::from(env_path);
        if path.exists() {
            return Ok(Some(path));
        }
        return Err(ConfigError::Io(format!(
            "DHRUV_CONFIG_FILE path does not exist: {}",
            path.display()
        )));
    }

    for (toml_path, json_path) in discovery_pairs() {
        let toml_exists = toml_path.exists();
        let json_exists = json_path.exists();

        if toml_exists && json_exists {
            return Err(ConfigError::AmbiguousConfigFiles {
                toml: toml_path,
                json: json_path,
            });
        }
        if toml_exists {
            return Ok(Some(toml_path));
        }
        if json_exists {
            return Ok(Some(json_path));
        }
    }

    Ok(None)
}

pub fn load_with_discovery(
    explicit: Option<&Path>,
    disable: bool,
) -> Result<Option<LoadedConfig>, ConfigError> {
    let Some(path) = discover_config_path(explicit, disable)? else {
        return Ok(None);
    };
    Ok(Some(load_from_path(&path)?))
}

fn discovery_pairs() -> Vec<(PathBuf, PathBuf)> {
    let mut pairs = Vec::new();

    #[cfg(target_os = "windows")]
    {
        if let Ok(appdata) = env::var("APPDATA") {
            let dir = PathBuf::from(appdata).join("dhruv");
            pairs.push((dir.join("config.toml"), dir.join("config.json")));
        }
        if let Ok(program_data) = env::var("PROGRAMDATA") {
            let dir = PathBuf::from(program_data).join("dhruv");
            pairs.push((dir.join("config.toml"), dir.join("config.json")));
        }
    }

    #[cfg(target_os = "macos")]
    {
        if let Ok(home) = env::var("HOME") {
            let user_dir = PathBuf::from(home)
                .join("Library")
                .join("Application Support")
                .join("dhruv");
            pairs.push((user_dir.join("config.toml"), user_dir.join("config.json")));
        }

        let sys_dir = PathBuf::from("/Library/Application Support/dhruv");
        pairs.push((sys_dir.join("config.toml"), sys_dir.join("config.json")));
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    {
        if let Ok(xdg) = env::var("XDG_CONFIG_HOME")
            && !xdg.trim().is_empty()
        {
            let dir = PathBuf::from(xdg).join("dhruv");
            pairs.push((dir.join("config.toml"), dir.join("config.json")));
        } else if let Ok(home) = env::var("HOME") {
            let dir = PathBuf::from(home).join(".config").join("dhruv");
            pairs.push((dir.join("config.toml"), dir.join("config.json")));
        }

        let xdg_sys = PathBuf::from("/etc/xdg/dhruv");
        pairs.push((xdg_sys.join("config.toml"), xdg_sys.join("config.json")));

        let etc_sys = PathBuf::from("/etc/dhruv");
        pairs.push((etc_sys.join("config.toml"), etc_sys.join("config.json")));
    }

    pairs
}

fn parse_ayanamsha_system(
    input: &EnumInput,
    field: &'static str,
) -> Result<AyanamshaSystem, ConfigError> {
    match input {
        EnumInput::Int(idx) => {
            let idx = usize::try_from(*idx).map_err(|_| ConfigError::InvalidEnumValue {
                field,
                value: idx.to_string(),
            })?;
            AyanamshaSystem::all()
                .get(idx)
                .copied()
                .ok_or_else(|| ConfigError::InvalidEnumValue {
                    field,
                    value: idx.to_string(),
                })
        }
        EnumInput::Str(s) => {
            let key = s.to_ascii_lowercase();
            for v in AyanamshaSystem::all() {
                let name = format!("{v:?}").to_ascii_lowercase();
                if key == name || key.replace('-', "") == name.replace('_', "") {
                    return Ok(*v);
                }
            }
            Err(ConfigError::InvalidEnumValue {
                field,
                value: s.clone(),
            })
        }
    }
}

fn parse_precession_model(
    input: &EnumInput,
    field: &'static str,
) -> Result<PrecessionModel, ConfigError> {
    match input.as_lower().replace('_', "-").as_str() {
        "0" | "newcomb1895" | "newcomb-1895" => Ok(PrecessionModel::Newcomb1895),
        "1" | "lieske1977" | "lieske-1977" => Ok(PrecessionModel::Lieske1977),
        "2" | "iau2006" | "iau-2006" => Ok(PrecessionModel::Iau2006),
        "3" | "vondrak2011" | "vondrak-2011" => Ok(PrecessionModel::Vondrak2011),
        other => Err(ConfigError::InvalidEnumValue {
            field,
            value: other.to_string(),
        }),
    }
}

fn parse_reference_plane(
    input: &EnumInput,
    field: &'static str,
) -> Result<ReferencePlane, ConfigError> {
    match input.as_lower().replace('_', "-").as_str() {
        "0" | "ecliptic" => Ok(ReferencePlane::Ecliptic),
        "1" | "invariable" => Ok(ReferencePlane::Invariable),
        other => Err(ConfigError::InvalidEnumValue {
            field,
            value: other.to_string(),
        }),
    }
}

fn parse_sun_limb(input: &EnumInput, field: &'static str) -> Result<SunLimb, ConfigError> {
    match input.as_lower().replace('_', "-").as_str() {
        "0" | "upper-limb" => Ok(SunLimb::UpperLimb),
        "1" | "center" => Ok(SunLimb::Center),
        "2" | "lower-limb" => Ok(SunLimb::LowerLimb),
        other => Err(ConfigError::InvalidEnumValue {
            field,
            value: other.to_string(),
        }),
    }
}

fn parse_bhava_system(
    input: &EnumInput,
    field: &'static str,
) -> Result<dhruv_vedic_base::BhavaSystem, ConfigError> {
    match input {
        EnumInput::Int(idx) => {
            let idx = usize::try_from(*idx).map_err(|_| ConfigError::InvalidEnumValue {
                field,
                value: idx.to_string(),
            })?;
            ALL_BHAVA_SYSTEMS
                .get(idx)
                .copied()
                .ok_or_else(|| ConfigError::InvalidEnumValue {
                    field,
                    value: idx.to_string(),
                })
        }
        EnumInput::Str(s) => {
            let key = s.to_ascii_lowercase().replace('-', "").replace('_', "");
            ALL_BHAVA_SYSTEMS
                .iter()
                .copied()
                .find(|v| {
                    let name = format!("{v:?}").to_ascii_lowercase();
                    key == name.replace('_', "")
                })
                .ok_or_else(|| ConfigError::InvalidEnumValue {
                    field,
                    value: s.clone(),
                })
        }
    }
}

fn parse_bhava_reference_mode(
    input: &EnumInput,
    field: &'static str,
) -> Result<BhavaReferenceMode, ConfigError> {
    match input.as_lower().replace('_', "-").as_str() {
        "0" | "start-of-first" => Ok(BhavaReferenceMode::StartOfFirst),
        "1" | "middle-of-first" => Ok(BhavaReferenceMode::MiddleOfFirst),
        other => Err(ConfigError::InvalidEnumValue {
            field,
            value: other.to_string(),
        }),
    }
}

fn parse_chandra_benefic_rule(
    input: &EnumInput,
    field: &'static str,
) -> Result<ChandraBeneficRule, ConfigError> {
    match input.as_lower().replace('_', "-").as_str() {
        "0" | "brightness-72" | "brightness72" | "phase-72" | "old-72" => {
            Ok(ChandraBeneficRule::Brightness72)
        }
        "1" | "waxing-180" | "waxing180" | "current-180" => Ok(ChandraBeneficRule::Waxing180),
        other => Err(ConfigError::InvalidEnumValue {
            field,
            value: other.to_string(),
        }),
    }
}

fn parse_tara_accuracy(
    input: &EnumInput,
    field: &'static str,
) -> Result<TaraAccuracy, ConfigError> {
    match input.as_lower().replace('_', "-").as_str() {
        "0" | "astrometric" => Ok(TaraAccuracy::Astrometric),
        "1" | "apparent" => Ok(TaraAccuracy::Apparent),
        other => Err(ConfigError::InvalidEnumValue {
            field,
            value: other.to_string(),
        }),
    }
}

fn parse_node_dignity_policy(
    input: &EnumInput,
    field: &'static str,
) -> Result<NodeDignityPolicy, ConfigError> {
    match input.as_lower().replace('_', "-").as_str() {
        "0" | "sign-lord-based" => Ok(NodeDignityPolicy::SignLordBased),
        "1" | "always-sama" => Ok(NodeDignityPolicy::AlwaysSama),
        other => Err(ConfigError::InvalidEnumValue {
            field,
            value: other.to_string(),
        }),
    }
}

fn parse_charakaraka_scheme(
    input: &EnumInput,
    field: &'static str,
) -> Result<dhruv_vedic_base::CharakarakaScheme, ConfigError> {
    match input.as_lower().replace('_', "-").as_str() {
        "0" | "8" | "eight" => Ok(dhruv_vedic_base::CharakarakaScheme::Eight),
        "1" | "seven-no-pitri" | "7-no-pitri" => {
            Ok(dhruv_vedic_base::CharakarakaScheme::SevenNoPitri)
        }
        "2" | "seven-pk-merged-mk" | "7-pk-merged-mk" | "pk-merged-mk" => {
            Ok(dhruv_vedic_base::CharakarakaScheme::SevenPkMergedMk)
        }
        "3" | "mixed-parashara" | "mixed" | "7-8-parashara" => {
            Ok(dhruv_vedic_base::CharakarakaScheme::MixedParashara)
        }
        other => Err(ConfigError::InvalidEnumValue {
            field,
            value: other.to_string(),
        }),
    }
}

fn parse_starting_point(
    kind: &EnumInput,
    body_code: Option<i32>,
    custom_deg: Option<f64>,
    field: &'static str,
) -> Result<BhavaStartingPoint, ConfigError> {
    match kind.as_lower().replace('_', "-").as_str() {
        "0" | "lagna" => Ok(BhavaStartingPoint::Lagna),
        "1" | "body" | "body-longitude" => {
            let code = body_code.ok_or(ConfigError::MissingRequired(
                "bhava.starting_point_body_code",
            ))?;
            let body = Body::from_code(code).ok_or_else(|| ConfigError::InvalidEnumValue {
                field,
                value: code.to_string(),
            })?;
            Ok(BhavaStartingPoint::BodyLongitude(body))
        }
        "2" | "custom" | "custom-deg" => {
            let deg = custom_deg.ok_or(ConfigError::MissingRequired(
                "bhava.starting_point_custom_deg",
            ))?;
            Ok(BhavaStartingPoint::CustomDeg(deg))
        }
        other => Err(ConfigError::InvalidEnumValue {
            field,
            value: other.to_string(),
        }),
    }
}

fn apply_amsha_selection_patch(
    mut base: AmshaSelectionConfig,
    patch: Option<AmshaSelectionConfigPatch>,
) -> Result<AmshaSelectionConfig, ConfigError> {
    let Some(patch) = patch else {
        return Ok(base);
    };

    if let Some(v) = patch.count {
        base.count = v;
    }
    if let Some(codes) = patch.codes {
        if codes.len() > base.codes.len() {
            return Err(ConfigError::InvalidConfig(format!(
                "amsha_selection.codes length {} exceeds max {}",
                codes.len(),
                base.codes.len()
            )));
        }
        base.codes = [0; 40];
        for (i, code) in codes.into_iter().enumerate() {
            base.codes[i] = code;
        }
    }
    if let Some(variations) = patch.variations {
        if variations.len() > base.variations.len() {
            return Err(ConfigError::InvalidConfig(format!(
                "amsha_selection.variations length {} exceeds max {}",
                variations.len(),
                base.variations.len()
            )));
        }
        base.variations = [0; 40];
        for (i, v) in variations.into_iter().enumerate() {
            base.variations[i] = v;
        }
    }
    Ok(base)
}

fn apply_dasha_selection_patch(
    mut base: DashaSelectionConfig,
    patch: Option<DashaSelectionConfigPatch>,
) -> Result<DashaSelectionConfig, ConfigError> {
    let Some(patch) = patch else {
        return Ok(base);
    };

    if let Some(v) = patch.count {
        base.count = v;
    }
    if let Some(systems) = patch.systems {
        if systems.len() > MAX_DASHA_SYSTEMS {
            return Err(ConfigError::InvalidConfig(format!(
                "dasha.systems length {} exceeds max {}",
                systems.len(),
                MAX_DASHA_SYSTEMS
            )));
        }
        base.systems = [0xFF; MAX_DASHA_SYSTEMS];
        base.max_levels = [0xFF; MAX_DASHA_SYSTEMS];
        for (i, v) in systems.into_iter().enumerate() {
            base.systems[i] = v;
        }
    }
    if let Some(max_levels) = patch.max_levels {
        if max_levels.len() > MAX_DASHA_SYSTEMS {
            return Err(ConfigError::InvalidConfig(format!(
                "dasha.max_levels length {} exceeds max {}",
                max_levels.len(),
                MAX_DASHA_SYSTEMS
            )));
        }
        base.max_levels = [0xFF; MAX_DASHA_SYSTEMS];
        for (i, v) in max_levels.into_iter().enumerate() {
            base.max_levels[i] = v;
        }
    }
    if let Some(v) = patch.max_level {
        base.max_level = v;
    }
    if let Some(methods) = patch.level_methods {
        if methods.len() > base.level_methods.len() {
            return Err(ConfigError::InvalidConfig(format!(
                "dasha.level_methods length {} exceeds max {}",
                methods.len(),
                base.level_methods.len()
            )));
        }
        base.level_methods = [0xFF; 5];
        for (i, m) in methods.into_iter().enumerate() {
            base.level_methods[i] = m;
        }
    }
    if let Some(v) = patch.yogini_scheme {
        base.yogini_scheme = v;
    }
    if let Some(v) = patch.use_abhijit {
        base.use_abhijit = v;
    }
    if patch.snapshot_utc.is_some() && patch.snapshot_jd_utc.is_some() {
        return Err(ConfigError::InvalidConfig(
            "dasha snapshot config accepts only one of snapshot_utc or snapshot_jd_utc".to_string(),
        ));
    }
    if let Some(snapshot_utc) = patch.snapshot_utc {
        base.snapshot_time = Some(DashaSnapshotTime::Utc(UtcTime::try_from(snapshot_utc)?));
    }
    if let Some(snapshot_jd_utc) = patch.snapshot_jd_utc {
        base.snapshot_time = Some(DashaSnapshotTime::JdUtc(snapshot_jd_utc));
    }

    base.sanitize();
    base.validate()
        .map_err(|e| ConfigError::InvalidConfig(e.to_string()))?;
    Ok(base)
}

fn merge_patch<T>(explicit: Option<T>, operation: Option<T>) -> Option<T> {
    explicit.or(operation)
}

fn choose_copy<T: Copy>(
    explicit: Option<T>,
    op: Option<T>,
    common: Option<T>,
    default: Option<T>,
    field: &'static str,
) -> Result<(T, ConfigSource), ConfigError> {
    if let Some(v) = explicit {
        return Ok((v, ConfigSource::Explicit));
    }
    if let Some(v) = op {
        return Ok((v, ConfigSource::Operation));
    }
    if let Some(v) = common {
        return Ok((v, ConfigSource::Common));
    }
    if let Some(v) = default {
        return Ok((v, ConfigSource::RecommendedDefault));
    }
    Err(ConfigError::MissingRequired(field))
}

fn choose_string(
    explicit: Option<String>,
    op: Option<String>,
    common: Option<String>,
    default: Option<String>,
    field: &'static str,
) -> Result<(String, ConfigSource), ConfigError> {
    if let Some(v) = explicit {
        return Ok((v, ConfigSource::Explicit));
    }
    if let Some(v) = op {
        return Ok((v, ConfigSource::Operation));
    }
    if let Some(v) = common {
        return Ok((v, ConfigSource::Common));
    }
    if let Some(v) = default {
        return Ok((v, ConfigSource::RecommendedDefault));
    }
    Err(ConfigError::MissingRequired(field))
}

fn choose_vec_string(
    explicit: Option<Vec<String>>,
    op: Option<Vec<String>>,
    common: Option<Vec<String>>,
    default: Option<Vec<String>>,
    field: &'static str,
) -> Result<(Vec<String>, ConfigSource), ConfigError> {
    if let Some(v) = explicit {
        return Ok((v, ConfigSource::Explicit));
    }
    if let Some(v) = op {
        return Ok((v, ConfigSource::Operation));
    }
    if let Some(v) = common {
        return Ok((v, ConfigSource::Common));
    }
    if let Some(v) = default {
        return Ok((v, ConfigSource::RecommendedDefault));
    }
    Err(ConfigError::MissingRequired(field))
}

fn choose_enum(
    explicit: Option<EnumInput>,
    op: Option<EnumInput>,
    common: Option<EnumInput>,
    default: Option<EnumInput>,
    field: &'static str,
) -> Result<(EnumInput, ConfigSource), ConfigError> {
    if let Some(v) = explicit {
        return Ok((v, ConfigSource::Explicit));
    }
    if let Some(v) = op {
        return Ok((v, ConfigSource::Operation));
    }
    if let Some(v) = common {
        return Ok((v, ConfigSource::Common));
    }
    if let Some(v) = default {
        return Ok((v, ConfigSource::RecommendedDefault));
    }
    Err(ConfigError::MissingRequired(field))
}

fn recommended<T>(mode: DefaultsMode, value: T) -> Option<T> {
    match mode {
        DefaultsMode::Recommended => Some(value),
        DefaultsMode::None => None,
    }
}

fn recommended_enum(mode: DefaultsMode, value: EnumInput) -> Option<EnumInput> {
    match mode {
        DefaultsMode::Recommended => Some(value),
        DefaultsMode::None => None,
    }
}

fn layered_bool(
    explicit: Option<bool>,
    operation: Option<bool>,
    default: bool,
    name: &str,
    source: &mut BTreeMap<String, ConfigSource>,
) -> bool {
    if let Some(v) = explicit {
        source.insert(name.to_string(), ConfigSource::Explicit);
        v
    } else if let Some(v) = operation {
        source.insert(name.to_string(), ConfigSource::Operation);
        v
    } else {
        source.insert(name.to_string(), ConfigSource::RecommendedDefault);
        default
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_json_strict_unknown_key_fails() {
        let text = r#"{
            "version": 1,
            "common": {"unknown": 1}
        }"#;
        let err = serde_json::from_str::<DhruvConfigFile>(text).unwrap_err();
        assert!(err.to_string().contains("unknown field"));
    }

    #[test]
    fn parse_toml_and_resolve_defaults() {
        let text = r#"
version = 1
[operations.conjunction]
step_size_days = 2.0
"#;
        let file: DhruvConfigFile = toml::from_str(text).unwrap();
        let resolver = ConfigResolver::new(file, DefaultsMode::Recommended);
        let eff = resolver.resolve_conjunction(None).unwrap();
        assert_eq!(eff.value.step_size_days, 2.0);
        assert_eq!(eff.value.target_separation_deg, 0.0);
        assert_eq!(eff.value.max_iterations, 50);
    }

    #[test]
    fn resolve_sankranti_from_common() {
        let text = r#"
version = 1
[common]
ayanamsha_system = 0
use_nutation = true
precession_model = "iau2006"
reference_plane = "ecliptic"
max_iterations = 55
"#;
        let file: DhruvConfigFile = toml::from_str(text).unwrap();
        let resolver = ConfigResolver::new(file, DefaultsMode::Recommended);
        let eff = resolver.resolve_sankranti(None).unwrap();
        assert!(eff.value.use_nutation);
        assert_eq!(eff.value.max_iterations, 55);
    }

    #[test]
    fn resolve_engine_missing_paths_errors() {
        let file = DhruvConfigFile {
            version: 1,
            common: CommonConfigPatch::default(),
            operations: OperationConfigPatchSet::default(),
        };
        let resolver = ConfigResolver::new(file, DefaultsMode::Recommended);
        let err = resolver.resolve_engine(None).unwrap_err();
        assert!(matches!(err, ConfigError::MissingRequired(_)));
    }
}
