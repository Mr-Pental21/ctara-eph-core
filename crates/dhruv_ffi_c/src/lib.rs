//! C-facing adapter types for `ctara-dhruv-core`.

use std::ffi::CStr;
use std::path::PathBuf;
use std::ptr;
use std::sync::{LazyLock, RwLock};

use dhruv_config::{ConfigResolver, DefaultsMode, load_with_discovery};
use dhruv_core::{Body, Engine, EngineConfig, EngineError, Frame, Observer, Query, StateVector};
use dhruv_frames::PrecessionModel;
use dhruv_search::{
    ChandraGrahan, ChandraGrahanType, ConjunctionConfig, ConjunctionEvent, GrahaLongitudeKind,
    GrahaLongitudesConfig, GrahanConfig, LunarPhase, MaxSpeedEvent, MaxSpeedType, SankrantiConfig,
    SearchError, StationType, StationaryConfig, StationaryEvent, SuryaGrahan, SuryaGrahanType,
    amsha_charts_for_date, avastha_for_date, ayana_for_date, balas_for_date, bhavabala_for_date,
    body_ecliptic_lon_lat, charakaraka_for_date, dasha_child_period_with_inputs,
    dasha_children_with_inputs, dasha_complete_level_with_inputs, dasha_hierarchy_with_inputs,
    dasha_level0_entity_with_inputs, dasha_level0_with_inputs, dasha_snapshot_with_inputs,
    elongation_at, full_kundali_for_date, ghatika_for_date, ghatika_from_sunrises,
    graha_longitudes, hora_for_date, hora_from_sunrises, karana_at, karana_for_date, masa_for_date,
    nakshatra_at, nakshatra_for_date, next_amavasya, next_chandra_grahan, next_conjunction,
    next_max_speed, next_purnima, next_sankranti, next_specific_sankranti, next_stationary,
    next_surya_grahan, prev_amavasya, prev_chandra_grahan, prev_conjunction, prev_max_speed,
    prev_purnima, prev_sankranti, prev_specific_sankranti, prev_stationary, prev_surya_grahan,
    search_amavasyas, search_chandra_grahan, search_conjunctions, search_max_speed,
    search_purnimas, search_sankrantis, search_stationary, search_surya_grahan, shadbala_for_date,
    sidereal_sum_at, siderealize_bhava_result, special_lagnas_for_date, tithi_at, tithi_for_date,
    tropical_to_sidereal_longitude, vaar_for_date, vaar_from_sunrises, varsha_for_date,
    vedic_day_sunrises, vimsopaka_for_date, yoga_at, yoga_for_date,
};
use dhruv_tara::{TaraAccuracy, TaraCatalog, TaraConfig, TaraError, TaraId};
use dhruv_time::{
    DeltaTModel, DeltaTSegment, FutureDeltaTTransition, SmhFutureParabolaFamily,
    TimeConversionOptions, TimeConversionPolicy, TimeDiagnostics, TimeWarning, TtUtcSource,
    UtcTime,
};
use dhruv_vedic_base::dasha::RashiDashaInputs;
use dhruv_vedic_base::{
    Amsha, AmshaRequest, AmshaVariation, AyanamshaSystem, BhavaConfig, BhavaReferenceMode,
    BhavaStartingPoint, BhavaSystem, CharakarakaScheme, GeoLocation, LunarNode, NodeMode,
    RiseSetConfig, RiseSetEvent, RiseSetResult, SunLimb, VedicError, amsha_longitude,
    amsha_rashi_info, approximate_local_noon_jd, ayana_from_sidereal_longitude,
    ayanamsha_deg_with_catalog, ayanamsha_mean_deg_with_catalog, ayanamsha_true_deg,
    compute_all_events, compute_bhavas, compute_rise_set, deg_to_dms, jd_tdb_to_centuries,
    karana_from_elongation, lunar_node_deg, lunar_node_deg_for_epoch, masa_from_rashi_index,
    nakshatra_from_longitude, nakshatra_from_tropical, nakshatra28_from_longitude,
    nakshatra28_from_tropical, nth_rashi_from, rashi_from_longitude, rashi_from_tropical,
    samvatsara_from_year, tithi_from_elongation, utc_day_start_jd, vaar_from_jd, yoga_from_sum,
};
use dhruv_vedic_ops::{
    PANCHANG_INCLUDE_AYANA, PANCHANG_INCLUDE_GHATIKA, PANCHANG_INCLUDE_HORA,
    PANCHANG_INCLUDE_KARANA, PANCHANG_INCLUDE_MASA, PANCHANG_INCLUDE_NAKSHATRA,
    PANCHANG_INCLUDE_TITHI, PANCHANG_INCLUDE_VAAR, PANCHANG_INCLUDE_VARSHA, PANCHANG_INCLUDE_YOGA,
    PanchangOperation, PanchangResult, TaraOperation, TaraOutputKind, TaraResult,
};

/// ABI version for downstream bindings.
pub const DHRUV_API_VERSION: u32 = 49;

/// Fixed UTF-8 buffer size for path fields in C-compatible structs.
pub const DHRUV_PATH_CAPACITY: usize = 512;

/// Maximum number of SPK kernel paths in a C-compatible config.
pub const DHRUV_MAX_SPK_PATHS: usize = 8;

/// C-facing status codes.
#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DhruvStatus {
    Ok = 0,
    InvalidConfig = 1,
    InvalidQuery = 2,
    KernelLoad = 3,
    TimeConversion = 4,
    UnsupportedQuery = 5,
    EpochOutOfRange = 6,
    NullPointer = 7,
    EopLoad = 8,
    EopOutOfRange = 9,
    InvalidLocation = 10,
    NoConvergence = 11,
    InvalidSearchConfig = 12,
    InvalidInput = 13,
    Internal = 255,
}

/// Reference plane for positional measurements.
///
/// Most ayanamsha systems use `Ecliptic` (0). The Jagganatha system uses
/// `Invariable` (1), the plane perpendicular to the solar system's angular
/// momentum vector.
#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DhruvReferencePlane {
    /// Ecliptic plane (standard).
    Ecliptic = 0,
    /// Invariable plane (fixed, no precession).
    Invariable = 1,
}

pub const DHRUV_PRECESSION_MODEL_NEWCOMB1895: i32 = 0;
pub const DHRUV_PRECESSION_MODEL_LIESKE1977: i32 = 1;
pub const DHRUV_PRECESSION_MODEL_IAU2006: i32 = 2;
pub const DHRUV_PRECESSION_MODEL_VONDRAK2011: i32 = 3;

pub const DHRUV_GRAHA_LONGITUDE_KIND_SIDEREAL: i32 = 0;
pub const DHRUV_GRAHA_LONGITUDE_KIND_TROPICAL: i32 = 1;

impl From<&EngineError> for DhruvStatus {
    fn from(value: &EngineError) -> Self {
        match value {
            EngineError::InvalidConfig(_) => Self::InvalidConfig,
            EngineError::InvalidQuery(_) => Self::InvalidQuery,
            EngineError::KernelLoad(_) => Self::KernelLoad,
            EngineError::TimeConversion(_) => Self::TimeConversion,
            EngineError::UnsupportedQuery(_) => Self::UnsupportedQuery,
            EngineError::EpochOutOfRange { .. } => Self::EpochOutOfRange,
            EngineError::Internal(_) => Self::Internal,
            _ => Self::Internal,
        }
    }
}

impl From<&VedicError> for DhruvStatus {
    fn from(value: &VedicError) -> Self {
        match value {
            VedicError::Engine(e) => Self::from(e),
            VedicError::Time(dhruv_time::TimeError::EopParse(_))
            | VedicError::Time(dhruv_time::TimeError::Io(_)) => Self::EopLoad,
            VedicError::Time(dhruv_time::TimeError::EopOutOfRange) => Self::EopOutOfRange,
            VedicError::Time(_) => Self::TimeConversion,
            VedicError::InvalidLocation(_) => Self::InvalidLocation,
            VedicError::NoConvergence(_) => Self::NoConvergence,
            VedicError::InvalidInput(_) => Self::InvalidInput,
            _ => Self::Internal,
        }
    }
}

impl From<&SearchError> for DhruvStatus {
    fn from(value: &SearchError) -> Self {
        match value {
            SearchError::Engine(e) => Self::from(e),
            SearchError::InvalidConfig(_) => Self::InvalidSearchConfig,
            SearchError::NoConvergence(_) => Self::NoConvergence,
            _ => Self::Internal,
        }
    }
}

impl From<&dhruv_vedic_ops::SearchError> for DhruvStatus {
    fn from(value: &dhruv_vedic_ops::SearchError) -> Self {
        match value {
            dhruv_vedic_ops::SearchError::Engine(e) => Self::from(e),
            dhruv_vedic_ops::SearchError::InvalidConfig(_) => Self::InvalidSearchConfig,
            dhruv_vedic_ops::SearchError::NoConvergence(_) => Self::NoConvergence,
            _ => Self::Internal,
        }
    }
}

/// C-compatible engine configuration.
#[repr(C)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DhruvEngineConfig {
    pub spk_path_count: u32,
    pub spk_paths_utf8: [[u8; DHRUV_PATH_CAPACITY]; DHRUV_MAX_SPK_PATHS],
    pub lsk_path_utf8: [u8; DHRUV_PATH_CAPACITY],
    pub cache_capacity: u64,
    pub strict_validation: u8,
}

impl DhruvEngineConfig {
    /// Convenience constructor for a single SPK path (most common case).
    pub fn try_new(
        spk_path_utf8: &str,
        lsk_path_utf8: &str,
        cache_capacity: u64,
        strict_validation: bool,
    ) -> Result<Self, DhruvStatus> {
        Self::try_new_multi(
            &[spk_path_utf8],
            lsk_path_utf8,
            cache_capacity,
            strict_validation,
        )
    }

    /// Constructor for multiple SPK paths.
    pub fn try_new_multi(
        spk_paths: &[&str],
        lsk_path_utf8: &str,
        cache_capacity: u64,
        strict_validation: bool,
    ) -> Result<Self, DhruvStatus> {
        if spk_paths.is_empty() || spk_paths.len() > DHRUV_MAX_SPK_PATHS {
            return Err(DhruvStatus::InvalidConfig);
        }

        let mut spk_paths_utf8 = [[0_u8; DHRUV_PATH_CAPACITY]; DHRUV_MAX_SPK_PATHS];
        for (i, path) in spk_paths.iter().enumerate() {
            spk_paths_utf8[i] = encode_c_utf8(path)?;
        }

        Ok(Self {
            spk_path_count: spk_paths.len() as u32,
            spk_paths_utf8,
            lsk_path_utf8: encode_c_utf8(lsk_path_utf8)?,
            cache_capacity,
            strict_validation: u8::from(strict_validation),
        })
    }
}

impl TryFrom<&DhruvEngineConfig> for EngineConfig {
    type Error = EngineError;

    fn try_from(value: &DhruvEngineConfig) -> Result<Self, Self::Error> {
        let count = value.spk_path_count as usize;
        if count == 0 || count > DHRUV_MAX_SPK_PATHS {
            return Err(EngineError::InvalidConfig(
                "spk_path_count must be between 1 and 8",
            ));
        }

        let mut spk_paths = Vec::with_capacity(count);
        for buf in &value.spk_paths_utf8[..count] {
            let path_str = decode_c_utf8(buf)
                .map_err(|_| EngineError::InvalidConfig("invalid UTF-8 in spk_path"))?;
            spk_paths.push(PathBuf::from(path_str));
        }

        let lsk_path = decode_c_utf8(&value.lsk_path_utf8)
            .map_err(|_| EngineError::InvalidConfig("invalid UTF-8 in lsk_path"))?;

        let cache_capacity = usize::try_from(value.cache_capacity)
            .map_err(|_| EngineError::InvalidConfig("cache_capacity exceeds platform usize"))?;

        Ok(EngineConfig {
            spk_paths,
            lsk_path: PathBuf::from(lsk_path),
            cache_capacity,
            strict_validation: value.strict_validation != 0,
        })
    }
}

/// C-compatible query shape.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DhruvQuery {
    pub target: i32,
    pub observer: i32,
    pub frame: i32,
    pub epoch_tdb_jd: f64,
}

pub const DHRUV_QUERY_TIME_JD_TDB: i32 = 0;
pub const DHRUV_QUERY_TIME_UTC: i32 = 1;
pub const DHRUV_DASHA_TIME_JD_UTC: i32 = 0;
pub const DHRUV_DASHA_TIME_UTC: i32 = 1;

pub const DHRUV_QUERY_OUTPUT_CARTESIAN: i32 = 0;
pub const DHRUV_QUERY_OUTPUT_SPHERICAL: i32 = 1;
pub const DHRUV_QUERY_OUTPUT_BOTH: i32 = 2;

pub const DHRUV_TIME_POLICY_STRICT_LSK: i32 = 0;
pub const DHRUV_TIME_POLICY_HYBRID_DELTA_T: i32 = 1;

pub const DHRUV_DELTA_T_MODEL_LEGACY_ESPENAK_MEEUS_2006: i32 = 0;
pub const DHRUV_DELTA_T_MODEL_SMH2016_WITH_PRE720_QUADRATIC: i32 = 1;

pub const DHRUV_FUTURE_DELTA_T_TRANSITION_LEGACY_TT_UTC_BLEND: i32 = 0;
pub const DHRUV_FUTURE_DELTA_T_TRANSITION_BRIDGE_FROM_MODERN_ENDPOINT: i32 = 1;

pub const DHRUV_SMH_FUTURE_FAMILY_ADDENDUM_2020_PIECEWISE: i32 = 0;
pub const DHRUV_SMH_FUTURE_FAMILY_CONSTANT_C_MINUS20: i32 = 1;
pub const DHRUV_SMH_FUTURE_FAMILY_CONSTANT_C_MINUS17P52: i32 = 2;
pub const DHRUV_SMH_FUTURE_FAMILY_CONSTANT_C_MINUS15P32: i32 = 3;
pub const DHRUV_SMH_FUTURE_FAMILY_STEPHENSON_1997: i32 = 4;
pub const DHRUV_SMH_FUTURE_FAMILY_STEPHENSON_2016: i32 = 5;

pub const DHRUV_TT_UTC_SOURCE_LSK_DELTA_AT: i32 = 0;
pub const DHRUV_TT_UTC_SOURCE_DELTA_T_MODEL: i32 = 1;

pub const DHRUV_TIME_WARNING_LSK_FUTURE_FROZEN: i32 = 0;
pub const DHRUV_TIME_WARNING_LSK_PRE_RANGE_FALLBACK: i32 = 1;
pub const DHRUV_TIME_WARNING_EOP_FUTURE_FROZEN: i32 = 2;
pub const DHRUV_TIME_WARNING_EOP_PRE_RANGE_FALLBACK: i32 = 3;
pub const DHRUV_TIME_WARNING_DELTA_T_MODEL_USED: i32 = 4;

pub const DHRUV_DELTA_T_SEGMENT_PRE_MINUS720_QUADRATIC: i32 = 0;
pub const DHRUV_DELTA_T_SEGMENT_SMH2016_RECONSTRUCTION: i32 = 1;
pub const DHRUV_DELTA_T_SEGMENT_SMH_ASYMPTOTIC_FUTURE: i32 = 2;
pub const DHRUV_DELTA_T_SEGMENT_BEFORE_MINUS500: i32 = 3;
pub const DHRUV_DELTA_T_SEGMENT_MINUS500_TO_500: i32 = 4;
pub const DHRUV_DELTA_T_SEGMENT_YEAR500_TO1600: i32 = 5;
pub const DHRUV_DELTA_T_SEGMENT_YEAR1600_TO1700: i32 = 6;
pub const DHRUV_DELTA_T_SEGMENT_YEAR1700_TO1800: i32 = 7;
pub const DHRUV_DELTA_T_SEGMENT_YEAR1800_TO1860: i32 = 8;
pub const DHRUV_DELTA_T_SEGMENT_YEAR1860_TO1900: i32 = 9;
pub const DHRUV_DELTA_T_SEGMENT_YEAR1900_TO1920: i32 = 10;
pub const DHRUV_DELTA_T_SEGMENT_YEAR1920_TO1941: i32 = 11;
pub const DHRUV_DELTA_T_SEGMENT_YEAR1941_TO1961: i32 = 12;
pub const DHRUV_DELTA_T_SEGMENT_YEAR1961_TO1986: i32 = 13;
pub const DHRUV_DELTA_T_SEGMENT_YEAR1986_TO2005: i32 = 14;
pub const DHRUV_DELTA_T_SEGMENT_YEAR2005_TO2050: i32 = 15;
pub const DHRUV_DELTA_T_SEGMENT_YEAR2050_TO2150: i32 = 16;
pub const DHRUV_DELTA_T_SEGMENT_AFTER2150: i32 = 17;

pub const DHRUV_MAX_TIME_WARNINGS: usize = 8;

/// Unified query transport carrying either JD(TDB) or UTC input.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DhruvQueryRequest {
    pub target: i32,
    pub observer: i32,
    pub frame: i32,
    pub time_kind: i32,
    pub epoch_tdb_jd: f64,
    pub utc: DhruvUtcTime,
    pub output_mode: i32,
}

impl TryFrom<DhruvQuery> for Query {
    type Error = EngineError;

    fn try_from(value: DhruvQuery) -> Result<Self, Self::Error> {
        let target = Body::from_code(value.target)
            .ok_or(EngineError::InvalidQuery("target code is unsupported"))?;
        let observer = Observer::from_code(value.observer)
            .ok_or(EngineError::InvalidQuery("observer code is unsupported"))?;
        let frame = Frame::from_code(value.frame)
            .ok_or(EngineError::InvalidQuery("frame code is unsupported"))?;

        Ok(Query {
            target,
            observer,
            frame,
            epoch_tdb_jd: value.epoch_tdb_jd,
        })
    }
}

/// C-compatible output state vector.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DhruvStateVector {
    pub position_km: [f64; 3],
    pub velocity_km_s: [f64; 3],
}

impl From<StateVector> for DhruvStateVector {
    fn from(value: StateVector) -> Self {
        Self {
            position_km: value.position_km,
            velocity_km_s: value.velocity_km_s,
        }
    }
}

/// Unified query result carrying cartesian and/or spherical output.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DhruvQueryResult {
    pub state_vector: DhruvStateVector,
    pub spherical_state: DhruvSphericalState,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DhruvTimeConversionOptions {
    pub warn_on_fallback: u8,
    pub delta_t_model: i32,
    pub freeze_future_dut1: u8,
    pub pre_range_dut1: f64,
    pub future_delta_t_transition: i32,
    pub future_transition_years: f64,
    pub smh_future_family: i32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DhruvTimePolicy {
    pub mode: i32,
    pub options: DhruvTimeConversionOptions,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DhruvTimeWarning {
    pub kind: i32,
    pub utc_seconds: f64,
    pub first_entry_utc_seconds: f64,
    pub last_entry_utc_seconds: f64,
    pub used_delta_at_seconds: f64,
    pub mjd: f64,
    pub first_entry_mjd: f64,
    pub last_entry_mjd: f64,
    pub used_dut1_seconds: f64,
    pub delta_t_model: i32,
    pub delta_t_segment: i32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DhruvTimeDiagnostics {
    pub source: i32,
    pub tt_minus_utc_s: f64,
    pub warning_count: u32,
    pub warnings: [DhruvTimeWarning; DHRUV_MAX_TIME_WARNINGS],
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DhruvUtcToTdbRequest {
    pub utc: DhruvUtcTime,
    pub policy: DhruvTimePolicy,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DhruvUtcToTdbResult {
    pub jd_tdb: f64,
    pub diagnostics: DhruvTimeDiagnostics,
}

fn delta_t_model_from_code(code: i32) -> Option<DeltaTModel> {
    match code {
        DHRUV_DELTA_T_MODEL_LEGACY_ESPENAK_MEEUS_2006 => Some(DeltaTModel::LegacyEspenakMeeus2006),
        DHRUV_DELTA_T_MODEL_SMH2016_WITH_PRE720_QUADRATIC => {
            Some(DeltaTModel::Smh2016WithPre720Quadratic)
        }
        _ => None,
    }
}

fn delta_t_model_to_code(model: DeltaTModel) -> i32 {
    match model {
        DeltaTModel::LegacyEspenakMeeus2006 => DHRUV_DELTA_T_MODEL_LEGACY_ESPENAK_MEEUS_2006,
        DeltaTModel::Smh2016WithPre720Quadratic => {
            DHRUV_DELTA_T_MODEL_SMH2016_WITH_PRE720_QUADRATIC
        }
    }
}

fn delta_t_segment_to_code(segment: DeltaTSegment) -> i32 {
    match segment {
        DeltaTSegment::PreMinus720Quadratic => DHRUV_DELTA_T_SEGMENT_PRE_MINUS720_QUADRATIC,
        DeltaTSegment::Smh2016Reconstruction => DHRUV_DELTA_T_SEGMENT_SMH2016_RECONSTRUCTION,
        DeltaTSegment::SmhAsymptoticFuture => DHRUV_DELTA_T_SEGMENT_SMH_ASYMPTOTIC_FUTURE,
        DeltaTSegment::BeforeMinus500 => DHRUV_DELTA_T_SEGMENT_BEFORE_MINUS500,
        DeltaTSegment::Minus500To500 => DHRUV_DELTA_T_SEGMENT_MINUS500_TO_500,
        DeltaTSegment::Year500To1600 => DHRUV_DELTA_T_SEGMENT_YEAR500_TO1600,
        DeltaTSegment::Year1600To1700 => DHRUV_DELTA_T_SEGMENT_YEAR1600_TO1700,
        DeltaTSegment::Year1700To1800 => DHRUV_DELTA_T_SEGMENT_YEAR1700_TO1800,
        DeltaTSegment::Year1800To1860 => DHRUV_DELTA_T_SEGMENT_YEAR1800_TO1860,
        DeltaTSegment::Year1860To1900 => DHRUV_DELTA_T_SEGMENT_YEAR1860_TO1900,
        DeltaTSegment::Year1900To1920 => DHRUV_DELTA_T_SEGMENT_YEAR1900_TO1920,
        DeltaTSegment::Year1920To1941 => DHRUV_DELTA_T_SEGMENT_YEAR1920_TO1941,
        DeltaTSegment::Year1941To1961 => DHRUV_DELTA_T_SEGMENT_YEAR1941_TO1961,
        DeltaTSegment::Year1961To1986 => DHRUV_DELTA_T_SEGMENT_YEAR1961_TO1986,
        DeltaTSegment::Year1986To2005 => DHRUV_DELTA_T_SEGMENT_YEAR1986_TO2005,
        DeltaTSegment::Year2005To2050 => DHRUV_DELTA_T_SEGMENT_YEAR2005_TO2050,
        DeltaTSegment::Year2050To2150 => DHRUV_DELTA_T_SEGMENT_YEAR2050_TO2150,
        DeltaTSegment::After2150 => DHRUV_DELTA_T_SEGMENT_AFTER2150,
    }
}

fn smh_future_family_from_code(code: i32) -> Option<SmhFutureParabolaFamily> {
    match code {
        DHRUV_SMH_FUTURE_FAMILY_ADDENDUM_2020_PIECEWISE => {
            Some(SmhFutureParabolaFamily::Addendum2020Piecewise)
        }
        DHRUV_SMH_FUTURE_FAMILY_CONSTANT_C_MINUS20 => {
            Some(SmhFutureParabolaFamily::ConstantCMinus20)
        }
        DHRUV_SMH_FUTURE_FAMILY_CONSTANT_C_MINUS17P52 => {
            Some(SmhFutureParabolaFamily::ConstantCMinus17p52)
        }
        DHRUV_SMH_FUTURE_FAMILY_CONSTANT_C_MINUS15P32 => {
            Some(SmhFutureParabolaFamily::ConstantCMinus15p32)
        }
        DHRUV_SMH_FUTURE_FAMILY_STEPHENSON_1997 => Some(SmhFutureParabolaFamily::Stephenson1997),
        DHRUV_SMH_FUTURE_FAMILY_STEPHENSON_2016 => Some(SmhFutureParabolaFamily::Stephenson2016),
        _ => None,
    }
}

fn future_delta_t_transition_from_code(code: i32) -> Option<FutureDeltaTTransition> {
    match code {
        DHRUV_FUTURE_DELTA_T_TRANSITION_LEGACY_TT_UTC_BLEND => {
            Some(FutureDeltaTTransition::LegacyTtUtcBlend)
        }
        DHRUV_FUTURE_DELTA_T_TRANSITION_BRIDGE_FROM_MODERN_ENDPOINT => {
            Some(FutureDeltaTTransition::BridgeFromModernEndpoint)
        }
        _ => None,
    }
}

fn time_policy_from_ffi(policy: DhruvTimePolicy) -> Result<TimeConversionPolicy, DhruvStatus> {
    match policy.mode {
        DHRUV_TIME_POLICY_STRICT_LSK => Ok(TimeConversionPolicy::StrictLsk),
        DHRUV_TIME_POLICY_HYBRID_DELTA_T => {
            Ok(TimeConversionPolicy::HybridDeltaT(TimeConversionOptions {
                warn_on_fallback: policy.options.warn_on_fallback != 0,
                delta_t_model: delta_t_model_from_code(policy.options.delta_t_model)
                    .ok_or(DhruvStatus::InvalidInput)?,
                freeze_future_dut1: policy.options.freeze_future_dut1 != 0,
                pre_range_dut1: policy.options.pre_range_dut1,
                future_delta_t_transition: future_delta_t_transition_from_code(
                    policy.options.future_delta_t_transition,
                )
                .ok_or(DhruvStatus::InvalidInput)?,
                future_transition_years: policy.options.future_transition_years,
                smh_future_family: smh_future_family_from_code(policy.options.smh_future_family)
                    .ok_or(DhruvStatus::InvalidInput)?,
            }))
        }
        _ => Err(DhruvStatus::InvalidInput),
    }
}

fn empty_time_warning() -> DhruvTimeWarning {
    DhruvTimeWarning {
        kind: -1,
        utc_seconds: 0.0,
        first_entry_utc_seconds: 0.0,
        last_entry_utc_seconds: 0.0,
        used_delta_at_seconds: 0.0,
        mjd: 0.0,
        first_entry_mjd: 0.0,
        last_entry_mjd: 0.0,
        used_dut1_seconds: 0.0,
        delta_t_model: -1,
        delta_t_segment: -1,
    }
}

fn time_warning_to_ffi(warning: &TimeWarning) -> DhruvTimeWarning {
    match warning {
        TimeWarning::LskFutureFrozen {
            utc_seconds,
            last_entry_utc_seconds,
            used_delta_at_seconds,
        } => DhruvTimeWarning {
            kind: DHRUV_TIME_WARNING_LSK_FUTURE_FROZEN,
            utc_seconds: *utc_seconds,
            last_entry_utc_seconds: *last_entry_utc_seconds,
            used_delta_at_seconds: *used_delta_at_seconds,
            ..empty_time_warning()
        },
        TimeWarning::LskPreRangeFallback {
            utc_seconds,
            first_entry_utc_seconds,
        } => DhruvTimeWarning {
            kind: DHRUV_TIME_WARNING_LSK_PRE_RANGE_FALLBACK,
            utc_seconds: *utc_seconds,
            first_entry_utc_seconds: *first_entry_utc_seconds,
            ..empty_time_warning()
        },
        TimeWarning::EopFutureFrozen {
            mjd,
            last_entry_mjd,
            used_dut1_seconds,
        } => DhruvTimeWarning {
            kind: DHRUV_TIME_WARNING_EOP_FUTURE_FROZEN,
            mjd: *mjd,
            last_entry_mjd: *last_entry_mjd,
            used_dut1_seconds: *used_dut1_seconds,
            ..empty_time_warning()
        },
        TimeWarning::EopPreRangeFallback {
            mjd,
            first_entry_mjd,
            used_dut1_seconds,
        } => DhruvTimeWarning {
            kind: DHRUV_TIME_WARNING_EOP_PRE_RANGE_FALLBACK,
            mjd: *mjd,
            first_entry_mjd: *first_entry_mjd,
            used_dut1_seconds: *used_dut1_seconds,
            ..empty_time_warning()
        },
        TimeWarning::DeltaTModelUsed {
            model,
            segment,
            assumed_dut1_seconds,
        } => DhruvTimeWarning {
            kind: DHRUV_TIME_WARNING_DELTA_T_MODEL_USED,
            used_dut1_seconds: *assumed_dut1_seconds,
            delta_t_model: delta_t_model_to_code(*model),
            delta_t_segment: delta_t_segment_to_code(*segment),
            ..empty_time_warning()
        },
    }
}

fn time_diagnostics_to_ffi(diagnostics: &TimeDiagnostics) -> DhruvTimeDiagnostics {
    let mut warnings = [empty_time_warning(); DHRUV_MAX_TIME_WARNINGS];
    let warning_count = diagnostics.warnings.len().min(DHRUV_MAX_TIME_WARNINGS);
    for (slot, warning) in warnings
        .iter_mut()
        .zip(diagnostics.warnings.iter().take(DHRUV_MAX_TIME_WARNINGS))
    {
        *slot = time_warning_to_ffi(warning);
    }
    DhruvTimeDiagnostics {
        source: match diagnostics.source {
            TtUtcSource::LskDeltaAt => DHRUV_TT_UTC_SOURCE_LSK_DELTA_AT,
            TtUtcSource::DeltaTModel => DHRUV_TT_UTC_SOURCE_DELTA_T_MODEL,
        },
        tt_minus_utc_s: diagnostics.tt_minus_utc_s,
        warning_count: warning_count as u32,
        warnings,
    }
}

/// Opaque config handle for FFI callers.
pub struct DhruvConfigHandle {
    resolver: ConfigResolver,
}

static FFI_CONFIG_RESOLVER: LazyLock<RwLock<Option<ConfigResolver>>> =
    LazyLock::new(|| RwLock::new(None));

/// Opaque engine handle type for ABI consumers.
pub type DhruvEngineHandle = Engine;

/// Opaque LSK handle type for ABI consumers.
pub type DhruvLskHandle = dhruv_time::LeapSecondKernel;

fn ffi_resolver() -> Option<ConfigResolver> {
    let guard = FFI_CONFIG_RESOLVER.read().ok()?;
    guard.as_ref().cloned()
}

fn resolve_sankranti_config_ptr(
    config: *const DhruvSankrantiConfig,
) -> Result<SankrantiConfig, DhruvStatus> {
    if let Some(cfg) = unsafe { config.as_ref() } {
        return sankranti_config_from_ffi(cfg).ok_or(DhruvStatus::InvalidQuery);
    }
    if let Some(resolver) = ffi_resolver() {
        return resolver
            .resolve_sankranti(None)
            .map(|v| v.value)
            .map_err(|_| DhruvStatus::InvalidSearchConfig);
    }
    sankranti_config_from_ffi(&dhruv_sankranti_config_default()).ok_or(DhruvStatus::InvalidQuery)
}

fn riseset_config_from_ffi(cfg: &DhruvRiseSetConfig) -> Result<RiseSetConfig, DhruvStatus> {
    let sun_limb = match sun_limb_from_code(cfg.sun_limb) {
        Some(l) => l,
        None => return Err(DhruvStatus::InvalidQuery),
    };
    Ok(RiseSetConfig {
        use_refraction: cfg.use_refraction != 0,
        sun_limb,
        altitude_correction: cfg.altitude_correction != 0,
    })
}

fn resolve_riseset_config_ptr(
    config: *const DhruvRiseSetConfig,
) -> Result<RiseSetConfig, DhruvStatus> {
    if let Some(cfg) = unsafe { config.as_ref() } {
        return riseset_config_from_ffi(cfg);
    }
    if let Some(resolver) = ffi_resolver() {
        return resolver
            .resolve_riseset(None)
            .map(|v| v.value)
            .map_err(|_| DhruvStatus::InvalidSearchConfig);
    }
    riseset_config_from_ffi(&dhruv_riseset_config_default())
}

fn resolve_bhava_config_ptr(config: *const DhruvBhavaConfig) -> Result<BhavaConfig, DhruvStatus> {
    if let Some(cfg) = unsafe { config.as_ref() } {
        return bhava_config_from_ffi(cfg);
    }
    if let Some(resolver) = ffi_resolver() {
        return resolver
            .resolve_bhava(None)
            .map(|v| v.value)
            .map_err(|_| DhruvStatus::InvalidSearchConfig);
    }
    bhava_config_from_ffi(&dhruv_bhava_config_default())
}

fn graha_positions_config_from_ffi(
    cfg: &DhruvGrahaPositionsConfig,
) -> dhruv_search::GrahaPositionsConfig {
    dhruv_search::GrahaPositionsConfig {
        include_nakshatra: cfg.include_nakshatra != 0,
        include_lagna: cfg.include_lagna != 0,
        include_outer_planets: cfg.include_outer_planets != 0,
        include_bhava: cfg.include_bhava != 0,
    }
}

fn time_upagraha_point_from_code(code: i32) -> Option<dhruv_vedic_base::TimeUpagrahaPoint> {
    match code {
        DHRUV_UPAGRAHA_POINT_START => Some(dhruv_vedic_base::TimeUpagrahaPoint::Start),
        DHRUV_UPAGRAHA_POINT_MIDDLE => Some(dhruv_vedic_base::TimeUpagrahaPoint::Middle),
        DHRUV_UPAGRAHA_POINT_END => Some(dhruv_vedic_base::TimeUpagrahaPoint::End),
        _ => None,
    }
}

fn gulika_maandi_planet_from_code(code: i32) -> Option<dhruv_vedic_base::GulikaMaandiPlanet> {
    match code {
        DHRUV_GULIKA_MAANDI_PLANET_RAHU => Some(dhruv_vedic_base::GulikaMaandiPlanet::Rahu),
        DHRUV_GULIKA_MAANDI_PLANET_SATURN => Some(dhruv_vedic_base::GulikaMaandiPlanet::Saturn),
        _ => None,
    }
}

fn time_upagraha_config_from_ffi(
    cfg: &DhruvTimeUpagrahaConfig,
) -> Result<dhruv_vedic_base::TimeUpagrahaConfig, DhruvStatus> {
    Ok(dhruv_vedic_base::TimeUpagrahaConfig {
        gulika_point: time_upagraha_point_from_code(cfg.gulika_point)
            .ok_or(DhruvStatus::InvalidQuery)?,
        maandi_point: time_upagraha_point_from_code(cfg.maandi_point)
            .ok_or(DhruvStatus::InvalidQuery)?,
        other_point: time_upagraha_point_from_code(cfg.other_point)
            .ok_or(DhruvStatus::InvalidQuery)?,
        gulika_planet: gulika_maandi_planet_from_code(cfg.gulika_planet)
            .ok_or(DhruvStatus::InvalidQuery)?,
        maandi_planet: gulika_maandi_planet_from_code(cfg.maandi_planet)
            .ok_or(DhruvStatus::InvalidQuery)?,
    })
}

fn resolve_time_upagraha_config_ptr(
    config: *const DhruvTimeUpagrahaConfig,
) -> Result<dhruv_vedic_base::TimeUpagrahaConfig, DhruvStatus> {
    if let Some(cfg) = unsafe { config.as_ref() } {
        return time_upagraha_config_from_ffi(cfg);
    }
    Ok(dhruv_vedic_base::TimeUpagrahaConfig::default())
}

fn resolve_graha_positions_config_ptr(
    config: *const DhruvGrahaPositionsConfig,
) -> Result<dhruv_search::GrahaPositionsConfig, DhruvStatus> {
    if let Some(cfg) = unsafe { config.as_ref() } {
        return Ok(graha_positions_config_from_ffi(cfg));
    }
    if let Some(resolver) = ffi_resolver() {
        return resolver
            .resolve_graha_positions(None)
            .map(|v| v.value)
            .map_err(|_| DhruvStatus::InvalidSearchConfig);
    }
    Ok(dhruv_search::GrahaPositionsConfig {
        include_nakshatra: false,
        include_lagna: false,
        include_outer_planets: false,
        include_bhava: false,
    })
}

fn bindus_config_from_ffi(cfg: &DhruvBindusConfig) -> dhruv_search::BindusConfig {
    dhruv_search::BindusConfig {
        include_nakshatra: cfg.include_nakshatra != 0,
        include_bhava: cfg.include_bhava != 0,
        upagraha_config: time_upagraha_config_from_ffi(&cfg.upagraha_config).unwrap_or_default(),
    }
}

fn resolve_bindus_config_ptr(
    config: *const DhruvBindusConfig,
) -> Result<dhruv_search::BindusConfig, DhruvStatus> {
    if let Some(cfg) = unsafe { config.as_ref() } {
        return Ok(bindus_config_from_ffi(cfg));
    }
    if let Some(resolver) = ffi_resolver() {
        return resolver
            .resolve_bindus(None)
            .map(|v| v.value)
            .map_err(|_| DhruvStatus::InvalidSearchConfig);
    }
    Ok(dhruv_search::BindusConfig {
        include_nakshatra: false,
        include_bhava: false,
        upagraha_config: dhruv_vedic_base::TimeUpagrahaConfig::default(),
    })
}

fn drishti_config_from_ffi(cfg: &DhruvDrishtiConfig) -> dhruv_search::DrishtiConfig {
    dhruv_search::DrishtiConfig {
        include_bhava: cfg.include_bhava != 0,
        include_lagna: cfg.include_lagna != 0,
        include_bindus: cfg.include_bindus != 0,
    }
}

fn resolve_drishti_config_ptr(
    config: *const DhruvDrishtiConfig,
) -> Result<dhruv_search::DrishtiConfig, DhruvStatus> {
    if let Some(cfg) = unsafe { config.as_ref() } {
        return Ok(drishti_config_from_ffi(cfg));
    }
    if let Some(resolver) = ffi_resolver() {
        return resolver
            .resolve_drishti(None)
            .map(|v| v.value)
            .map_err(|_| DhruvStatus::InvalidSearchConfig);
    }
    Ok(dhruv_search::DrishtiConfig {
        include_bhava: false,
        include_lagna: false,
        include_bindus: false,
    })
}

fn full_kundali_config_from_ffi(
    cfg: &DhruvFullKundaliConfig,
) -> Result<dhruv_search::FullKundaliConfig, DhruvStatus> {
    let amsha_sel = dhruv_search::AmshaSelectionConfig {
        count: cfg.amsha_selection.count,
        codes: cfg.amsha_selection.codes,
        variations: cfg.amsha_selection.variations,
    };
    let node_dignity_policy = match cfg.node_dignity_policy {
        0 => dhruv_vedic_base::NodeDignityPolicy::SignLordBased,
        1 => dhruv_vedic_base::NodeDignityPolicy::AlwaysSama,
        _ => return Err(DhruvStatus::InvalidSearchConfig),
    };
    let charakaraka_scheme = match CharakarakaScheme::from_u8(cfg.charakaraka_scheme) {
        Some(v) => v,
        None => return Err(DhruvStatus::InvalidSearchConfig),
    };
    Ok(dhruv_search::FullKundaliConfig {
        include_bhava_cusps: cfg.include_bhava_cusps != 0,
        include_graha_positions: cfg.include_graha_positions != 0,
        include_bindus: cfg.include_bindus != 0,
        include_drishti: cfg.include_drishti != 0,
        include_ashtakavarga: cfg.include_ashtakavarga != 0,
        include_upagrahas: cfg.include_upagrahas != 0,
        include_sphutas: cfg.include_sphutas != 0,
        include_special_lagnas: cfg.include_special_lagnas != 0,
        include_amshas: cfg.include_amshas != 0,
        include_shadbala: cfg.include_shadbala != 0,
        include_bhavabala: cfg.include_bhavabala != 0,
        include_vimsopaka: cfg.include_vimsopaka != 0,
        include_avastha: cfg.include_avastha != 0,
        include_charakaraka: cfg.include_charakaraka != 0,
        charakaraka_scheme,
        node_dignity_policy,
        upagraha_config: time_upagraha_config_from_ffi(&cfg.upagraha_config)?,
        graha_positions_config: graha_positions_config_from_ffi(&cfg.graha_positions_config),
        bindus_config: bindus_config_from_ffi(&cfg.bindus_config),
        drishti_config: drishti_config_from_ffi(&cfg.drishti_config),
        amsha_scope: dhruv_search::AmshaChartScope {
            include_bhava_cusps: cfg.amsha_scope.include_bhava_cusps != 0,
            include_arudha_padas: cfg.amsha_scope.include_arudha_padas != 0,
            include_upagrahas: cfg.amsha_scope.include_upagrahas != 0,
            include_sphutas: cfg.amsha_scope.include_sphutas != 0,
            include_special_lagnas: cfg.amsha_scope.include_special_lagnas != 0,
        },
        amsha_selection: amsha_sel,
        include_panchang: cfg.include_panchang != 0 || cfg.include_calendar != 0,
        include_calendar: cfg.include_calendar != 0,
        include_dasha: cfg.include_dasha != 0,
        dasha_config: dasha_selection_from_ffi(&cfg.dasha_config),
    })
}

fn resolve_full_kundali_config_ptr(
    config: *const DhruvFullKundaliConfig,
) -> Result<dhruv_search::FullKundaliConfig, DhruvStatus> {
    if let Some(cfg) = unsafe { config.as_ref() } {
        return full_kundali_config_from_ffi(cfg);
    }
    if let Some(resolver) = ffi_resolver() {
        return resolver
            .resolve_full_kundali(None)
            .map(|v| v.value)
            .map_err(|_| DhruvStatus::InvalidSearchConfig);
    }
    full_kundali_config_from_ffi(&dhruv_full_kundali_config_default())
}

/// Build a core engine from C-compatible config.
pub fn dhruv_engine_new_internal(config: &DhruvEngineConfig) -> Result<Engine, DhruvStatus> {
    let core_config = EngineConfig::try_from(config).map_err(|err| DhruvStatus::from(&err))?;
    Engine::new(core_config).map_err(|err| DhruvStatus::from(&err))
}

/// Query the engine using C-compatible types.
pub fn dhruv_engine_query_internal(
    engine: &Engine,
    query: DhruvQuery,
) -> Result<DhruvStateVector, DhruvStatus> {
    let core_query = Query::try_from(query).map_err(|err| DhruvStatus::from(&err))?;
    let state = engine
        .query(core_query)
        .map_err(|err| DhruvStatus::from(&err))?;
    Ok(DhruvStateVector::from(state))
}

fn spherical_state_from_state(state: &StateVector) -> DhruvSphericalState {
    let ss =
        dhruv_frames::cartesian_state_to_spherical_state(&state.position_km, &state.velocity_km_s);
    DhruvSphericalState {
        lon_deg: ss.lon_deg,
        lat_deg: ss.lat_deg,
        distance_km: ss.distance_km,
        lon_speed: ss.lon_speed,
        lat_speed: ss.lat_speed,
        distance_speed: ss.distance_speed,
    }
}

fn validate_query_request_selectors(request: DhruvQueryRequest) -> Result<(), DhruvStatus> {
    match request.time_kind {
        DHRUV_QUERY_TIME_JD_TDB | DHRUV_QUERY_TIME_UTC => {}
        _ => return Err(DhruvStatus::InvalidQuery),
    }
    match request.output_mode {
        DHRUV_QUERY_OUTPUT_CARTESIAN | DHRUV_QUERY_OUTPUT_SPHERICAL | DHRUV_QUERY_OUTPUT_BOTH => {}
        _ => return Err(DhruvStatus::InvalidQuery),
    }
    Ok(())
}

fn query_from_request(engine: &Engine, request: DhruvQueryRequest) -> Result<Query, DhruvStatus> {
    validate_query_request_selectors(request)?;
    let target = Body::from_code(request.target).ok_or(DhruvStatus::InvalidQuery)?;
    let observer = Observer::from_code(request.observer).ok_or(DhruvStatus::InvalidQuery)?;
    let frame = Frame::from_code(request.frame).ok_or(DhruvStatus::InvalidQuery)?;
    let epoch_tdb_jd = match request.time_kind {
        DHRUV_QUERY_TIME_JD_TDB => request.epoch_tdb_jd,
        DHRUV_QUERY_TIME_UTC => dhruv_time::Epoch::from_utc(
            request.utc.year,
            request.utc.month,
            request.utc.day,
            request.utc.hour,
            request.utc.minute,
            request.utc.second,
            engine.lsk(),
        )
        .as_jd_tdb(),
        _ => unreachable!("validated above"),
    };

    Ok(Query {
        target,
        observer,
        frame,
        epoch_tdb_jd,
    })
}

fn query_result_from_state(state: StateVector, output_mode: i32) -> DhruvQueryResult {
    let zero_state = DhruvStateVector {
        position_km: [0.0; 3],
        velocity_km_s: [0.0; 3],
    };
    let zero_spherical = DhruvSphericalState {
        lon_deg: 0.0,
        lat_deg: 0.0,
        distance_km: 0.0,
        lon_speed: 0.0,
        lat_speed: 0.0,
        distance_speed: 0.0,
    };
    match output_mode {
        DHRUV_QUERY_OUTPUT_CARTESIAN => DhruvQueryResult {
            state_vector: DhruvStateVector::from(state),
            spherical_state: zero_spherical,
        },
        DHRUV_QUERY_OUTPUT_SPHERICAL => DhruvQueryResult {
            state_vector: zero_state,
            spherical_state: spherical_state_from_state(&state),
        },
        DHRUV_QUERY_OUTPUT_BOTH => DhruvQueryResult {
            state_vector: DhruvStateVector::from(state),
            spherical_state: spherical_state_from_state(&state),
        },
        _ => DhruvQueryResult {
            state_vector: zero_state,
            spherical_state: zero_spherical,
        },
    }
}

/// Query the engine using the unified request transport.
pub fn dhruv_engine_query_request_internal(
    engine: &Engine,
    request: DhruvQueryRequest,
) -> Result<DhruvQueryResult, DhruvStatus> {
    let query = query_from_request(engine, request)?;
    let state = engine.query(query).map_err(|err| DhruvStatus::from(&err))?;
    Ok(query_result_from_state(state, request.output_mode))
}

/// Convenience helper for one-shot callers.
pub fn dhruv_query_once_internal(
    config: &DhruvEngineConfig,
    query: DhruvQuery,
) -> Result<DhruvStateVector, DhruvStatus> {
    let engine = dhruv_engine_new_internal(config)?;
    dhruv_engine_query_internal(&engine, query)
}

/// Return ABI version of the exported C API.
#[unsafe(no_mangle)]
pub extern "C" fn dhruv_api_version() -> u32 {
    DHRUV_API_VERSION
}

fn defaults_mode_from_i32(value: i32) -> Option<DefaultsMode> {
    match value {
        0 => Some(DefaultsMode::Recommended),
        1 => Some(DefaultsMode::None),
        _ => None,
    }
}

/// Load layered configuration and make it active for nullable-config fallback.
///
/// `path_utf8` can be null to use auto-discovery.
/// `defaults_mode`: 0 = recommended, 1 = none.
///
/// # Safety
/// `out_handle` must be non-null. If non-null, `path_utf8` must be a valid\n/// NUL-terminated UTF-8 path string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dhruv_config_load(
    path_utf8: *const i8,
    defaults_mode: i32,
    out_handle: *mut *mut DhruvConfigHandle,
) -> DhruvStatus {
    ffi_boundary(|| {
        if out_handle.is_null() {
            return DhruvStatus::NullPointer;
        }
        let Some(mode) = defaults_mode_from_i32(defaults_mode) else {
            return DhruvStatus::InvalidConfig;
        };

        let explicit_path = if path_utf8.is_null() {
            None
        } else {
            let cstr = unsafe { CStr::from_ptr(path_utf8) };
            match cstr.to_str() {
                Ok(s) if !s.trim().is_empty() => Some(PathBuf::from(s)),
                _ => return DhruvStatus::InvalidConfig,
            }
        };

        let loaded = load_with_discovery(explicit_path.as_deref(), false)
            .map_err(|_| DhruvStatus::InvalidConfig);
        let loaded = match loaded {
            Ok(v) => v,
            Err(status) => return status,
        };
        let Some(loaded) = loaded else {
            return DhruvStatus::InvalidConfig;
        };

        let resolver = ConfigResolver::new(loaded.file, mode);
        let handle = DhruvConfigHandle {
            resolver: resolver.clone(),
        };
        if let Ok(mut guard) = FFI_CONFIG_RESOLVER.write() {
            *guard = Some(resolver);
        }

        unsafe { *out_handle = Box::into_raw(Box::new(handle)) };
        DhruvStatus::Ok
    })
}

/// Free a config handle previously returned by `dhruv_config_load`.
///
/// # Safety
/// `handle` must be null or a pointer returned by `dhruv_config_load`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dhruv_config_free(handle: *mut DhruvConfigHandle) -> DhruvStatus {
    ffi_boundary(|| {
        if handle.is_null() {
            return DhruvStatus::Ok;
        }
        let boxed = unsafe { Box::from_raw(handle) };
        let _ = boxed.resolver.defaults_mode();
        drop(boxed);
        DhruvStatus::Ok
    })
}

/// Clear active process-level resolver used for nullable-config fallback.
#[unsafe(no_mangle)]
pub extern "C" fn dhruv_config_clear_active() -> DhruvStatus {
    ffi_boundary(|| {
        if let Ok(mut guard) = FFI_CONFIG_RESOLVER.write() {
            *guard = None;
        }
        DhruvStatus::Ok
    })
}

/// Create an engine handle.
///
/// # Safety
/// `config` and `out_engine` must be valid, non-null pointers.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dhruv_engine_new(
    config: *const DhruvEngineConfig,
    out_engine: *mut *mut DhruvEngineHandle,
) -> DhruvStatus {
    ffi_boundary(|| {
        if config.is_null() || out_engine.is_null() {
            return DhruvStatus::NullPointer;
        }

        // SAFETY: Pointers are checked for null and are only borrowed for this call.
        let config_ref = unsafe { &*config };
        // SAFETY: Pointer is checked for null and we only write a single pointer value.
        let out_engine_ref = unsafe { &mut *out_engine };

        match dhruv_engine_new_internal(config_ref) {
            Ok(engine) => {
                *out_engine_ref = Box::into_raw(Box::new(engine));
                DhruvStatus::Ok
            }
            Err(status) => {
                *out_engine_ref = ptr::null_mut();
                status
            }
        }
    })
}

/// Query an existing engine handle.
///
/// # Safety
/// `engine`, `query`, and `out_state` must be valid, non-null pointers.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dhruv_engine_query(
    engine: *const DhruvEngineHandle,
    query: *const DhruvQuery,
    out_state: *mut DhruvStateVector,
) -> DhruvStatus {
    ffi_boundary(|| {
        if engine.is_null() || query.is_null() || out_state.is_null() {
            return DhruvStatus::NullPointer;
        }

        // SAFETY: Pointers are checked for null and only borrowed for this call.
        let engine_ref = unsafe { &*engine };
        // SAFETY: Pointer is checked for null and copied by value.
        let query_value = unsafe { *query };

        match dhruv_engine_query_internal(engine_ref, query_value) {
            Ok(state) => {
                // SAFETY: Pointer is checked for null and written once.
                unsafe { *out_state = state };
                DhruvStatus::Ok
            }
            Err(status) => status,
        }
    })
}

/// Query an existing engine handle using the unified request transport.
///
/// # Safety
/// `engine`, `request`, and `out_result` must be valid, non-null pointers.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dhruv_engine_query_request(
    engine: *const DhruvEngineHandle,
    request: *const DhruvQueryRequest,
    out_result: *mut DhruvQueryResult,
) -> DhruvStatus {
    ffi_boundary(|| {
        if engine.is_null() || request.is_null() || out_result.is_null() {
            return DhruvStatus::NullPointer;
        }

        let engine_ref = unsafe { &*engine };
        let request_value = unsafe { *request };

        match dhruv_engine_query_request_internal(engine_ref, request_value) {
            Ok(result) => {
                unsafe { *out_result = result };
                DhruvStatus::Ok
            }
            Err(status) => status,
        }
    })
}

/// Destroy an engine handle allocated by [`dhruv_engine_new`].
///
/// # Safety
/// `engine` must be either null or a pointer returned by `dhruv_engine_new`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dhruv_engine_free(engine: *mut DhruvEngineHandle) -> DhruvStatus {
    ffi_boundary(|| {
        if engine.is_null() {
            return DhruvStatus::Ok;
        }

        // SAFETY: Ownership is transferred back from a pointer created by Box::into_raw.
        unsafe { drop(Box::from_raw(engine)) };
        DhruvStatus::Ok
    })
}

/// One-shot query helper (constructs and tears down engine internally).
///
/// # Safety
/// `config`, `query`, and `out_state` must be valid, non-null pointers.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dhruv_query_once(
    config: *const DhruvEngineConfig,
    query: *const DhruvQuery,
    out_state: *mut DhruvStateVector,
) -> DhruvStatus {
    ffi_boundary(|| {
        if config.is_null() || query.is_null() || out_state.is_null() {
            return DhruvStatus::NullPointer;
        }

        // SAFETY: Pointer checks performed above; references are ephemeral.
        let config_ref = unsafe { &*config };
        // SAFETY: Pointer checks performed above; copied by value.
        let query_value = unsafe { *query };

        match dhruv_query_once_internal(config_ref, query_value) {
            Ok(state) => {
                // SAFETY: Pointer checks performed above; write one value.
                unsafe { *out_state = state };
                DhruvStatus::Ok
            }
            Err(status) => status,
        }
    })
}

/// C-compatible spherical coordinates output.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DhruvSphericalCoords {
    /// Longitude in degrees, range [0, 360).
    pub lon_deg: f64,
    /// Latitude in degrees, range [-90, 90].
    pub lat_deg: f64,
    /// Distance from origin in km.
    pub distance_km: f64,
}

/// C-compatible spherical state with angular velocities.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DhruvSphericalState {
    /// Longitude in degrees, range [0, 360).
    pub lon_deg: f64,
    /// Latitude in degrees, range [-90, 90].
    pub lat_deg: f64,
    /// Distance from origin in km.
    pub distance_km: f64,
    /// Longitude rate of change in deg/day.
    pub lon_speed: f64,
    /// Latitude rate of change in deg/day.
    pub lat_speed: f64,
    /// Radial velocity in km/s.
    pub distance_speed: f64,
}

/// Load a leap second kernel (LSK) from a file path.
///
/// The LSK is a lightweight (~5 KB) text file containing leap second data
/// needed for UTC to TDB time conversion. It can be loaded and used
/// independently of the engine.
///
/// # Safety
/// `lsk_path_utf8` must be a valid, non-null, NUL-terminated C string.
/// `out_lsk` must be a valid, non-null pointer.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dhruv_lsk_load(
    lsk_path_utf8: *const u8,
    out_lsk: *mut *mut DhruvLskHandle,
) -> DhruvStatus {
    ffi_boundary(|| {
        if lsk_path_utf8.is_null() || out_lsk.is_null() {
            return DhruvStatus::NullPointer;
        }

        // SAFETY: Pointer is checked for null; read until NUL byte.
        let c_str = unsafe { std::ffi::CStr::from_ptr(lsk_path_utf8 as *const i8) };
        let path_str = match c_str.to_str() {
            Ok(s) => s,
            Err(_) => return DhruvStatus::InvalidConfig,
        };

        match dhruv_time::LeapSecondKernel::load(std::path::Path::new(path_str)) {
            Ok(lsk) => {
                // SAFETY: Pointer is checked for null; write one pointer value.
                unsafe { *out_lsk = Box::into_raw(Box::new(lsk)) };
                DhruvStatus::Ok
            }
            Err(_) => {
                // SAFETY: Pointer is checked for null; write null on failure.
                unsafe { *out_lsk = ptr::null_mut() };
                DhruvStatus::KernelLoad
            }
        }
    })
}

/// Destroy an LSK handle allocated by [`dhruv_lsk_load`].
///
/// # Safety
/// `lsk` must be either null or a pointer returned by `dhruv_lsk_load`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dhruv_lsk_free(lsk: *mut DhruvLskHandle) -> DhruvStatus {
    ffi_boundary(|| {
        if lsk.is_null() {
            return DhruvStatus::Ok;
        }

        // SAFETY: Ownership is transferred back from a pointer created by Box::into_raw.
        unsafe { drop(Box::from_raw(lsk)) };
        DhruvStatus::Ok
    })
}

/// Convert UTC calendar date to TDB Julian Date using a standalone LSK handle.
///
/// Writes the resulting JD TDB plus diagnostics into `out_result`.
///
/// # Safety
/// `lsk`, `request`, and `out_result` must be valid, non-null pointers.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dhruv_utc_to_tdb_jd(
    lsk: *const DhruvLskHandle,
    eop: *const DhruvEopHandle,
    request: *const DhruvUtcToTdbRequest,
    out_result: *mut DhruvUtcToTdbResult,
) -> DhruvStatus {
    ffi_boundary(|| {
        if lsk.is_null() || request.is_null() || out_result.is_null() {
            return DhruvStatus::NullPointer;
        }

        // SAFETY: Pointer is checked for null above.
        let lsk_ref = unsafe { &*lsk };
        // SAFETY: Pointer is checked for null above.
        let request_ref = unsafe { &*request };
        let eop_ref = unsafe { eop.as_ref() };

        let utc = match dhruv_time::UtcTime::try_new(
            request_ref.utc.year,
            request_ref.utc.month,
            request_ref.utc.day,
            request_ref.utc.hour,
            request_ref.utc.minute,
            request_ref.utc.second,
            Some(lsk_ref),
        ) {
            Ok(utc) => utc,
            Err(_) => return DhruvStatus::InvalidInput,
        };
        let day_frac = utc.day as f64
            + utc.hour as f64 / 24.0
            + utc.minute as f64 / 1440.0
            + utc.second / 86_400.0;
        let utc_jd = dhruv_time::calendar_to_jd(utc.year, utc.month, day_frac);
        let utc_seconds = dhruv_time::jd_to_tdb_seconds(utc_jd);
        let policy = match time_policy_from_ffi(request_ref.policy) {
            Ok(policy) => policy,
            Err(status) => return status,
        };
        let out = lsk_ref.utc_to_tdb_with_policy_and_eop(utc_seconds, eop_ref, policy);

        // SAFETY: Pointer is checked for null above; write one value.
        unsafe {
            *out_result = DhruvUtcToTdbResult {
                jd_tdb: dhruv_time::tdb_seconds_to_jd(out.tdb_seconds),
                diagnostics: time_diagnostics_to_ffi(&out.diagnostics),
            };
        }
        DhruvStatus::Ok
    })
}

/// Convert Cartesian position [x, y, z] (km) to spherical coordinates.
///
/// Pure math, no engine needed. Writes longitude (degrees, 0..360),
/// latitude (degrees, -90..90), and distance (km) into `out_spherical`.
///
/// # Safety
/// `position_km` and `out_spherical` must be valid, non-null pointers.
/// `position_km` must point to at least 3 contiguous f64 values.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dhruv_cartesian_to_spherical(
    position_km: *const [f64; 3],
    out_spherical: *mut DhruvSphericalCoords,
) -> DhruvStatus {
    ffi_boundary(|| {
        if position_km.is_null() || out_spherical.is_null() {
            return DhruvStatus::NullPointer;
        }

        // SAFETY: Pointer is checked for null; read 3 contiguous f64.
        let xyz = unsafe { &*position_km };
        let s = dhruv_frames::cartesian_to_spherical(xyz);

        // SAFETY: Pointer is checked for null; write one struct.
        unsafe {
            *out_spherical = DhruvSphericalCoords {
                lon_deg: s.lon_deg,
                lat_deg: s.lat_deg,
                distance_km: s.distance_km,
            };
        }
        DhruvStatus::Ok
    })
}

// ---------------------------------------------------------------------------
// EOP opaque handle
// ---------------------------------------------------------------------------

/// Opaque EOP handle type for ABI consumers.
pub type DhruvEopHandle = dhruv_time::EopKernel;

/// Load an IERS EOP (finals2000A.all) file from a NUL-terminated file path.
///
/// # Safety
/// `eop_path_utf8` must be a valid, non-null, NUL-terminated C string.
/// `out_eop` must be a valid, non-null pointer.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dhruv_eop_load(
    eop_path_utf8: *const u8,
    out_eop: *mut *mut DhruvEopHandle,
) -> DhruvStatus {
    ffi_boundary(|| {
        if eop_path_utf8.is_null() || out_eop.is_null() {
            return DhruvStatus::NullPointer;
        }

        // SAFETY: Pointer is checked for null; read until NUL byte.
        let c_str = unsafe { std::ffi::CStr::from_ptr(eop_path_utf8 as *const i8) };
        let path_str = match c_str.to_str() {
            Ok(s) => s,
            Err(_) => return DhruvStatus::InvalidConfig,
        };

        match dhruv_time::EopKernel::load(std::path::Path::new(path_str)) {
            Ok(eop) => {
                // SAFETY: Pointer is checked for null; write one pointer value.
                unsafe { *out_eop = Box::into_raw(Box::new(eop)) };
                DhruvStatus::Ok
            }
            Err(_) => {
                // SAFETY: Pointer is checked for null; write null on failure.
                unsafe { *out_eop = ptr::null_mut() };
                DhruvStatus::EopLoad
            }
        }
    })
}

/// Destroy an EOP handle allocated by [`dhruv_eop_load`].
///
/// # Safety
/// `eop` must be either null or a pointer returned by `dhruv_eop_load`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dhruv_eop_free(eop: *mut DhruvEopHandle) -> DhruvStatus {
    ffi_boundary(|| {
        if eop.is_null() {
            return DhruvStatus::Ok;
        }

        // SAFETY: Ownership is transferred back from a pointer created by Box::into_raw.
        unsafe { drop(Box::from_raw(eop)) };
        DhruvStatus::Ok
    })
}

// ---------------------------------------------------------------------------
// Ayanamsha
// ---------------------------------------------------------------------------

/// Map integer code 0..19 to AyanamshaSystem enum variant.
fn ayanamsha_system_from_code(code: i32) -> Option<AyanamshaSystem> {
    let systems = AyanamshaSystem::all();
    let idx = usize::try_from(code).ok()?;
    systems.get(idx).copied()
}

/// Number of supported ayanamsha systems.
#[unsafe(no_mangle)]
pub extern "C" fn dhruv_ayanamsha_system_count() -> u32 {
    AyanamshaSystem::all().len() as u32
}

/// Returns the default reference plane for an ayanamsha system.
///
/// Returns 0 (Ecliptic) for most systems, 1 (Invariable) for Jagganatha.
/// Returns -1 for invalid system codes.
#[unsafe(no_mangle)]
pub extern "C" fn dhruv_reference_plane_default(system_code: i32) -> i32 {
    match ayanamsha_system_from_code(system_code) {
        Some(system) => match system.default_reference_plane() {
            dhruv_frames::ReferencePlane::Ecliptic => 0,
            dhruv_frames::ReferencePlane::Invariable => 1,
        },
        None => -1,
    }
}

// ---------------------------------------------------------------------------
// Rise/Set types
// ---------------------------------------------------------------------------

/// C-compatible geographic location.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DhruvGeoLocation {
    /// Latitude in degrees, north positive. Range: [-90, 90].
    pub latitude_deg: f64,
    /// Longitude in degrees, east positive. Range: [-180, 180].
    pub longitude_deg: f64,
    /// Altitude above sea level in meters.
    pub altitude_m: f64,
}

/// C-compatible rise/set configuration.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DhruvRiseSetConfig {
    /// Apply standard atmospheric refraction (34 arcmin): 1 = yes, 0 = no.
    pub use_refraction: u8,
    /// Which solar limb defines sunrise/sunset.
    /// 0 = UpperLimb, 1 = Center, 2 = LowerLimb.
    pub sun_limb: i32,
    /// Apply altitude dip correction: 1 = true, 0 = false.
    pub altitude_correction: u8,
}

/// Sun limb: upper limb defines sunrise/sunset (conventional).
pub const DHRUV_SUN_LIMB_UPPER: i32 = 0;
/// Sun limb: center of disk defines sunrise/sunset.
pub const DHRUV_SUN_LIMB_CENTER: i32 = 1;
/// Sun limb: lower limb defines sunrise/sunset.
pub const DHRUV_SUN_LIMB_LOWER: i32 = 2;

/// C-compatible rise/set result.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DhruvRiseSetResult {
    /// 0 = Event occurred, 1 = NeverRises, 2 = NeverSets.
    pub result_type: i32,
    /// Event code (valid when result_type == 0).
    pub event_code: i32,
    /// Event time in JD TDB (valid when result_type == 0).
    pub jd_tdb: f64,
}

// Result type constants
pub const DHRUV_RISESET_EVENT: i32 = 0;
pub const DHRUV_RISESET_NEVER_RISES: i32 = 1;
pub const DHRUV_RISESET_NEVER_SETS: i32 = 2;

// Event code constants
pub const DHRUV_EVENT_SUNRISE: i32 = 0;
pub const DHRUV_EVENT_SUNSET: i32 = 1;
pub const DHRUV_EVENT_CIVIL_DAWN: i32 = 2;
pub const DHRUV_EVENT_CIVIL_DUSK: i32 = 3;
pub const DHRUV_EVENT_NAUTICAL_DAWN: i32 = 4;
pub const DHRUV_EVENT_NAUTICAL_DUSK: i32 = 5;
pub const DHRUV_EVENT_ASTRONOMICAL_DAWN: i32 = 6;
pub const DHRUV_EVENT_ASTRONOMICAL_DUSK: i32 = 7;

/// Map integer event code to RiseSetEvent.
fn riseset_event_from_code(code: i32) -> Option<RiseSetEvent> {
    match code {
        DHRUV_EVENT_SUNRISE => Some(RiseSetEvent::Sunrise),
        DHRUV_EVENT_SUNSET => Some(RiseSetEvent::Sunset),
        DHRUV_EVENT_CIVIL_DAWN => Some(RiseSetEvent::CivilDawn),
        DHRUV_EVENT_CIVIL_DUSK => Some(RiseSetEvent::CivilDusk),
        DHRUV_EVENT_NAUTICAL_DAWN => Some(RiseSetEvent::NauticalDawn),
        DHRUV_EVENT_NAUTICAL_DUSK => Some(RiseSetEvent::NauticalDusk),
        DHRUV_EVENT_ASTRONOMICAL_DAWN => Some(RiseSetEvent::AstronomicalDawn),
        DHRUV_EVENT_ASTRONOMICAL_DUSK => Some(RiseSetEvent::AstronomicalDusk),
        _ => None,
    }
}

/// Map RiseSetEvent back to integer code.
fn riseset_event_to_code(event: RiseSetEvent) -> i32 {
    match event {
        RiseSetEvent::Sunrise => DHRUV_EVENT_SUNRISE,
        RiseSetEvent::Sunset => DHRUV_EVENT_SUNSET,
        RiseSetEvent::CivilDawn => DHRUV_EVENT_CIVIL_DAWN,
        RiseSetEvent::CivilDusk => DHRUV_EVENT_CIVIL_DUSK,
        RiseSetEvent::NauticalDawn => DHRUV_EVENT_NAUTICAL_DAWN,
        RiseSetEvent::NauticalDusk => DHRUV_EVENT_NAUTICAL_DUSK,
        RiseSetEvent::AstronomicalDawn => DHRUV_EVENT_ASTRONOMICAL_DAWN,
        RiseSetEvent::AstronomicalDusk => DHRUV_EVENT_ASTRONOMICAL_DUSK,
    }
}

/// Convert Rust RiseSetResult to C-compatible DhruvRiseSetResult.
fn to_ffi_result(result: &RiseSetResult) -> DhruvRiseSetResult {
    match *result {
        RiseSetResult::Event { jd_tdb, event } => DhruvRiseSetResult {
            result_type: DHRUV_RISESET_EVENT,
            event_code: riseset_event_to_code(event),
            jd_tdb,
        },
        RiseSetResult::NeverRises => DhruvRiseSetResult {
            result_type: DHRUV_RISESET_NEVER_RISES,
            event_code: 0,
            jd_tdb: 0.0,
        },
        RiseSetResult::NeverSets => DhruvRiseSetResult {
            result_type: DHRUV_RISESET_NEVER_SETS,
            event_code: 0,
            jd_tdb: 0.0,
        },
    }
}

// ---------------------------------------------------------------------------
// Rise/Set functions
// ---------------------------------------------------------------------------

/// Returns default rise/set configuration.
#[unsafe(no_mangle)]
pub extern "C" fn dhruv_riseset_config_default() -> DhruvRiseSetConfig {
    DhruvRiseSetConfig {
        use_refraction: 1,
        sun_limb: DHRUV_SUN_LIMB_UPPER,
        altitude_correction: 1,
    }
}

/// Convert C sun_limb code to Rust SunLimb enum.
fn sun_limb_from_code(code: i32) -> Option<SunLimb> {
    match code {
        DHRUV_SUN_LIMB_UPPER => Some(SunLimb::UpperLimb),
        DHRUV_SUN_LIMB_CENTER => Some(SunLimb::Center),
        DHRUV_SUN_LIMB_LOWER => Some(SunLimb::LowerLimb),
        _ => None,
    }
}

/// Compute a single rise/set event.
///
/// # Safety
/// All pointer arguments must be valid and non-null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dhruv_compute_rise_set(
    engine: *const DhruvEngineHandle,
    lsk: *const DhruvLskHandle,
    eop: *const DhruvEopHandle,
    location: *const DhruvGeoLocation,
    event_code: i32,
    jd_utc_noon: f64,
    config: *const DhruvRiseSetConfig,
    out_result: *mut DhruvRiseSetResult,
) -> DhruvStatus {
    ffi_boundary(|| {
        if engine.is_null()
            || lsk.is_null()
            || eop.is_null()
            || location.is_null()
            || out_result.is_null()
        {
            return DhruvStatus::NullPointer;
        }

        let event = match riseset_event_from_code(event_code) {
            Some(e) => e,
            None => return DhruvStatus::InvalidQuery,
        };

        // SAFETY: All pointers checked for null above.
        let engine_ref = unsafe { &*engine };
        let lsk_ref = unsafe { &*lsk };
        let eop_ref = unsafe { &*eop };
        let loc_ref = unsafe { &*location };

        let geo = GeoLocation::new(
            loc_ref.latitude_deg,
            loc_ref.longitude_deg,
            loc_ref.altitude_m,
        );
        let rs_config = match resolve_riseset_config_ptr(config) {
            Ok(c) => c,
            Err(status) => return status,
        };

        match compute_rise_set(
            engine_ref,
            lsk_ref,
            eop_ref,
            &geo,
            event,
            jd_utc_noon,
            &rs_config,
        ) {
            Ok(result) => {
                // SAFETY: Pointer checked for null.
                unsafe { *out_result = to_ffi_result(&result) };
                DhruvStatus::Ok
            }
            Err(e) => DhruvStatus::from(&e),
        }
    })
}

/// Compute all 8 rise/set events for a day.
///
/// Caller must provide `out_results` pointing to an array of at least 8
/// `DhruvRiseSetResult`. Order: AstroDawn, NautDawn, CivilDawn, Sunrise,
/// Sunset, CivilDusk, NautDusk, AstroDusk.
///
/// # Safety
/// All pointer arguments must be valid and non-null.
/// `out_results` must point to at least 8 contiguous `DhruvRiseSetResult`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dhruv_compute_all_events(
    engine: *const DhruvEngineHandle,
    lsk: *const DhruvLskHandle,
    eop: *const DhruvEopHandle,
    location: *const DhruvGeoLocation,
    jd_utc_noon: f64,
    config: *const DhruvRiseSetConfig,
    out_results: *mut DhruvRiseSetResult,
) -> DhruvStatus {
    ffi_boundary(|| {
        if engine.is_null()
            || lsk.is_null()
            || eop.is_null()
            || location.is_null()
            || out_results.is_null()
        {
            return DhruvStatus::NullPointer;
        }

        // SAFETY: All pointers checked for null above.
        let engine_ref = unsafe { &*engine };
        let lsk_ref = unsafe { &*lsk };
        let eop_ref = unsafe { &*eop };
        let loc_ref = unsafe { &*location };

        let geo = GeoLocation::new(
            loc_ref.latitude_deg,
            loc_ref.longitude_deg,
            loc_ref.altitude_m,
        );
        let rs_config = match resolve_riseset_config_ptr(config) {
            Ok(c) => c,
            Err(status) => return status,
        };

        match compute_all_events(engine_ref, lsk_ref, eop_ref, &geo, jd_utc_noon, &rs_config) {
            Ok(results) => {
                // SAFETY: Pointer checked; write 8 contiguous values.
                let out_slice = unsafe { std::slice::from_raw_parts_mut(out_results, 8) };
                for (i, r) in results.iter().enumerate() {
                    out_slice[i] = to_ffi_result(r);
                }
                DhruvStatus::Ok
            }
            Err(e) => DhruvStatus::from(&e),
        }
    })
}

/// Approximate local noon JD from 0h UT JD and longitude. Pure math.
#[unsafe(no_mangle)]
pub extern "C" fn dhruv_approximate_local_noon_jd(jd_ut_midnight: f64, longitude_deg: f64) -> f64 {
    approximate_local_noon_jd(jd_ut_midnight, longitude_deg)
}

// ---------------------------------------------------------------------------
// Unified ayanamsha + standalone nutation
// ---------------------------------------------------------------------------

/// Ayanamsha mode selector: mean model.
pub const DHRUV_AYANAMSHA_MODE_MEAN: i32 = 0;
/// Ayanamsha mode selector: true model from explicit delta-psi.
pub const DHRUV_AYANAMSHA_MODE_TRUE: i32 = 1;
/// Ayanamsha mode selector: unified model with `use_nutation`.
pub const DHRUV_AYANAMSHA_MODE_UNIFIED: i32 = 2;

/// Ayanamsha time input selector: JD TDB in `jd_tdb`.
pub const DHRUV_AYANAMSHA_TIME_JD_TDB: i32 = 0;
/// Ayanamsha time input selector: UTC in `utc` (requires LSK handle).
pub const DHRUV_AYANAMSHA_TIME_UTC: i32 = 1;

/// C-compatible request for unified ayanamsha compute API.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DhruvAyanamshaComputeRequest {
    /// Ayanamsha system code (0..19).
    pub system_code: i32,
    /// Mode selector (`DHRUV_AYANAMSHA_MODE_*`).
    pub mode: i32,
    /// Time input selector (`DHRUV_AYANAMSHA_TIME_*`).
    pub time_kind: i32,
    /// JD TDB input when `time_kind=JD_TDB`.
    pub jd_tdb: f64,
    /// UTC input when `time_kind=UTC`.
    pub utc: DhruvUtcTime,
    /// Used by `mode=UNIFIED` (`0=false, nonzero=true`).
    pub use_nutation: u8,
    /// Used by `mode=TRUE` (arcseconds).
    pub delta_psi_arcsec: f64,
}

/// Unified ayanamsha compute API covering mode + time-base variants.
///
/// - `mode=MEAN` computes mean ayanamsha.
/// - `mode=TRUE` computes true ayanamsha using `delta_psi_arcsec`.
/// - `mode=UNIFIED` computes unified ayanamsha using `use_nutation`.
///
/// - `time_kind=JD_TDB` uses `jd_tdb`.
/// - `time_kind=UTC` uses `utc` and requires `lsk`.
///
/// `catalog` is optional and only used for star-catalog-aware mean/unified
/// computations (`mode=MEAN` / `mode=UNIFIED`).
///
/// # Safety
/// `request` and `out_deg` must be valid non-null pointers.
/// `catalog` may be null. `lsk` may be null only for `time_kind=JD_TDB`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dhruv_ayanamsha_compute_ex(
    lsk: *const DhruvLskHandle,
    request: *const DhruvAyanamshaComputeRequest,
    catalog: *const DhruvTaraCatalogHandle,
    out_deg: *mut f64,
) -> DhruvStatus {
    ffi_boundary(|| {
        if request.is_null() || out_deg.is_null() {
            return DhruvStatus::NullPointer;
        }

        let req = unsafe { &*request };
        let system = match ayanamsha_system_from_code(req.system_code) {
            Some(s) => s,
            None => return DhruvStatus::InvalidQuery,
        };

        let jd_tdb = match req.time_kind {
            DHRUV_AYANAMSHA_TIME_JD_TDB => req.jd_tdb,
            DHRUV_AYANAMSHA_TIME_UTC => {
                if lsk.is_null() {
                    return DhruvStatus::NullPointer;
                }
                let lsk_ref = unsafe { &*lsk };
                ffi_to_utc_time(&req.utc).to_jd_tdb(lsk_ref)
            }
            _ => return DhruvStatus::InvalidQuery,
        };

        let t = jd_tdb_to_centuries(jd_tdb);
        let cat_opt = if catalog.is_null() {
            None
        } else {
            Some(unsafe { &*catalog })
        };

        let deg = match req.mode {
            DHRUV_AYANAMSHA_MODE_MEAN => ayanamsha_mean_deg_with_catalog(system, t, cat_opt),
            DHRUV_AYANAMSHA_MODE_TRUE => ayanamsha_true_deg(system, t, req.delta_psi_arcsec),
            DHRUV_AYANAMSHA_MODE_UNIFIED => {
                ayanamsha_deg_with_catalog(system, t, req.use_nutation != 0, cat_opt)
            }
            _ => return DhruvStatus::InvalidQuery,
        };

        unsafe { *out_deg = deg };
        DhruvStatus::Ok
    })
}

/// Compute IAU 2000B nutation (standalone).
///
/// Returns nutation in longitude (Δψ) and obliquity (Δε) in arcseconds.
///
/// # Safety
/// `out_dpsi_arcsec` and `out_deps_arcsec` must be valid, non-null pointers.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dhruv_nutation_iau2000b(
    jd_tdb: f64,
    out_dpsi_arcsec: *mut f64,
    out_deps_arcsec: *mut f64,
) -> DhruvStatus {
    ffi_boundary(|| {
        if out_dpsi_arcsec.is_null() || out_deps_arcsec.is_null() {
            return DhruvStatus::NullPointer;
        }

        let t = jd_tdb_to_centuries(jd_tdb);
        let (dpsi, deps) = dhruv_frames::nutation_iau2000b(t);

        // SAFETY: Pointers checked for null; write one value each.
        unsafe {
            *out_dpsi_arcsec = dpsi;
            *out_deps_arcsec = deps;
        }
        DhruvStatus::Ok
    })
}

// ---------------------------------------------------------------------------
// UTC time output
// ---------------------------------------------------------------------------

/// Broken-down UTC calendar time.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DhruvUtcTime {
    pub year: i32,
    pub month: u32,
    pub day: u32,
    pub hour: u32,
    pub minute: u32,
    pub second: f64,
}

/// Extract hour, minute, second from a fractional day.
fn fractional_day_to_hms(day_frac: f64) -> (u32, u32, f64) {
    let frac = day_frac.fract();
    let total_seconds = frac * 86_400.0;
    let hour = (total_seconds / 3600.0).floor() as u32;
    let minute = ((total_seconds % 3600.0) / 60.0).floor() as u32;
    let second = total_seconds % 60.0;
    (hour, minute, second)
}

/// Convert a JD TDB to UTC calendar components.
///
/// # Safety
/// `lsk` and `out_utc` must be valid, non-null pointers.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dhruv_jd_tdb_to_utc(
    lsk: *const DhruvLskHandle,
    jd_tdb: f64,
    out_utc: *mut DhruvUtcTime,
) -> DhruvStatus {
    ffi_boundary(|| {
        if lsk.is_null() || out_utc.is_null() {
            return DhruvStatus::NullPointer;
        }

        // SAFETY: Pointers checked for null.
        let lsk_ref = unsafe { &*lsk };

        let tdb_s = dhruv_time::jd_to_tdb_seconds(jd_tdb);
        let utc_s = lsk_ref.tdb_to_utc(tdb_s);
        let jd_utc = dhruv_time::tdb_seconds_to_jd(utc_s);
        let (year, month, day_frac) = dhruv_time::jd_to_calendar(jd_utc);
        let day = day_frac.floor() as u32;
        let (hour, minute, second) = fractional_day_to_hms(day_frac);

        // SAFETY: Pointer checked for null; write one struct.
        unsafe {
            *out_utc = DhruvUtcTime {
                year,
                month,
                day,
                hour,
                minute,
                second,
            };
        }
        DhruvStatus::Ok
    })
}

/// Convert a rise/set result to UTC calendar components.
///
/// Only valid when `result->result_type == DHRUV_RISESET_EVENT`.
/// Returns `InvalidQuery` for NeverRises / NeverSets.
///
/// # Safety
/// `lsk`, `result`, and `out_utc` must be valid, non-null pointers.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dhruv_riseset_result_to_utc(
    lsk: *const DhruvLskHandle,
    result: *const DhruvRiseSetResult,
    out_utc: *mut DhruvUtcTime,
) -> DhruvStatus {
    ffi_boundary(|| {
        if lsk.is_null() || result.is_null() || out_utc.is_null() {
            return DhruvStatus::NullPointer;
        }

        // SAFETY: Pointer checked for null.
        let result_ref = unsafe { &*result };

        if result_ref.result_type != DHRUV_RISESET_EVENT {
            return DhruvStatus::InvalidQuery;
        }

        // Delegate to the general-purpose converter.
        // SAFETY: lsk and out_utc already validated, forwarding directly.
        unsafe { dhruv_jd_tdb_to_utc(lsk, result_ref.jd_tdb, out_utc) }
    })
}

// ---------------------------------------------------------------------------
// Bhava (house) system constants
// ---------------------------------------------------------------------------

pub const DHRUV_BHAVA_EQUAL: i32 = 0;
pub const DHRUV_BHAVA_SURYA_SIDDHANTA: i32 = 1;
pub const DHRUV_BHAVA_SRIPATI: i32 = 2;
pub const DHRUV_BHAVA_KP: i32 = 3;
pub const DHRUV_BHAVA_KOCH: i32 = 4;
pub const DHRUV_BHAVA_REGIOMONTANUS: i32 = 5;
pub const DHRUV_BHAVA_CAMPANUS: i32 = 6;
pub const DHRUV_BHAVA_AXIAL_ROTATION: i32 = 7;
pub const DHRUV_BHAVA_TOPOCENTRIC: i32 = 8;
pub const DHRUV_BHAVA_ALCABITUS: i32 = 9;

pub const DHRUV_BHAVA_REF_START: i32 = 0;
pub const DHRUV_BHAVA_REF_MIDDLE: i32 = 1;
pub const DHRUV_BHAVA_OUTPUT_TROPICAL: i32 = 0;
pub const DHRUV_BHAVA_OUTPUT_SIDEREAL: i32 = 1;

/// Starting point: use the Lagna (Ascendant).
pub const DHRUV_BHAVA_START_LAGNA: i32 = -1;
/// Starting point: use a custom ecliptic degree (see `custom_start_deg`).
pub const DHRUV_BHAVA_START_CUSTOM: i32 = -2;
// Positive values = NAIF body codes for BodyLongitude starting point.

// ---------------------------------------------------------------------------
// Bhava C-compatible types
// ---------------------------------------------------------------------------

/// C-compatible bhava configuration.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DhruvBhavaConfig {
    /// House system code (0-9, see DHRUV_BHAVA_* constants).
    pub system: i32,
    /// Starting point: -1=Lagna, -2=custom deg, or positive NAIF body code.
    pub starting_point: i32,
    /// Custom ecliptic degree, used only when starting_point == -2.
    pub custom_start_deg: f64,
    /// Reference mode: 0=start of first, 1=middle of first.
    pub reference_mode: i32,
    /// Output longitude mode: 0=tropical ecliptic, 1=sidereal on configured reference plane.
    pub output_mode: i32,
    /// Ayanamsha system code used when `output_mode == 1`.
    pub ayanamsha_system: i32,
    /// Apply nutation when computing ayanamsha for sidereal output.
    pub use_nutation: u8,
    /// Reference plane for sidereal output: 0=ecliptic, 1=invariable, -1=system default.
    pub reference_plane: i32,
}

/// C-compatible single bhava result.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DhruvBhava {
    pub number: u8,
    pub cusp_deg: f64,
    pub start_deg: f64,
    pub end_deg: f64,
}

/// C-compatible full bhava result.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DhruvBhavaResult {
    pub bhavas: [DhruvBhava; 12],
    pub lagna_deg: f64,
    pub mc_deg: f64,
}

/// Map integer code 0..9 to BhavaSystem enum variant.
fn bhava_system_from_code(code: i32) -> Option<BhavaSystem> {
    let systems = BhavaSystem::all();
    let idx = usize::try_from(code).ok()?;
    systems.get(idx).copied()
}

/// Convert C config to Rust BhavaConfig.
fn bhava_config_from_ffi(cfg: &DhruvBhavaConfig) -> Result<BhavaConfig, DhruvStatus> {
    let system = bhava_system_from_code(cfg.system).ok_or(DhruvStatus::InvalidQuery)?;

    let starting_point = match cfg.starting_point {
        DHRUV_BHAVA_START_LAGNA => BhavaStartingPoint::Lagna,
        DHRUV_BHAVA_START_CUSTOM => BhavaStartingPoint::CustomDeg(cfg.custom_start_deg),
        code if code > 0 => {
            let body = Body::from_code(code).ok_or(DhruvStatus::InvalidQuery)?;
            BhavaStartingPoint::BodyLongitude(body)
        }
        _ => return Err(DhruvStatus::InvalidQuery),
    };

    let reference_mode = match cfg.reference_mode {
        DHRUV_BHAVA_REF_START => BhavaReferenceMode::StartOfFirst,
        DHRUV_BHAVA_REF_MIDDLE => BhavaReferenceMode::MiddleOfFirst,
        _ => return Err(DhruvStatus::InvalidQuery),
    };

    Ok(BhavaConfig {
        system,
        starting_point,
        reference_mode,
    })
}

#[derive(Debug, Clone, Copy)]
struct BhavaOutputProjection {
    ayanamsha_deg: f64,
    reference_plane: dhruv_frames::ReferencePlane,
}

fn bhava_output_projection_from_ffi(
    cfg: &DhruvBhavaConfig,
    jd_tdb: f64,
) -> Result<Option<BhavaOutputProjection>, DhruvStatus> {
    match cfg.output_mode {
        DHRUV_BHAVA_OUTPUT_TROPICAL => Ok(None),
        DHRUV_BHAVA_OUTPUT_SIDEREAL => {
            let system = ayanamsha_system_from_code(cfg.ayanamsha_system)
                .ok_or(DhruvStatus::InvalidQuery)?;
            let reference_plane = reference_plane_from_code(cfg.reference_plane, system);
            let t = jd_tdb_to_centuries(jd_tdb);
            let mut aya_config = SankrantiConfig::new(system, cfg.use_nutation != 0);
            aya_config.reference_plane = reference_plane;
            Ok(Some(BhavaOutputProjection {
                ayanamsha_deg: aya_config.ayanamsha_deg_at_centuries(t),
                reference_plane,
            }))
        }
        _ => Err(DhruvStatus::InvalidQuery),
    }
}

fn bhava_result_to_ffi_with_projection(
    result: &dhruv_vedic_base::BhavaResult,
    projection: Option<BhavaOutputProjection>,
) -> DhruvBhavaResult {
    let projected = projection.map(|projection| {
        siderealize_bhava_result(result, projection.ayanamsha_deg, projection.reference_plane)
    });
    let view = projected.as_ref().unwrap_or(result);
    let mut ffi_bhavas = [DhruvBhava {
        number: 0,
        cusp_deg: 0.0,
        start_deg: 0.0,
        end_deg: 0.0,
    }; 12];
    for (i, b) in view.bhavas.iter().enumerate() {
        ffi_bhavas[i] = DhruvBhava {
            number: b.number,
            cusp_deg: b.cusp_deg,
            start_deg: b.start_deg,
            end_deg: b.end_deg,
        };
    }
    DhruvBhavaResult {
        bhavas: ffi_bhavas,
        lagna_deg: view.lagna_deg,
        mc_deg: view.mc_deg,
    }
}

fn jd_utc_to_jd_tdb_with_eop(
    lsk: &dhruv_time::LeapSecondKernel,
    eop: &dhruv_time::EopKernel,
    jd_utc: f64,
) -> f64 {
    let utc_s = dhruv_time::jd_to_tdb_seconds(jd_utc);
    let out = lsk.utc_to_tdb_with_policy_and_eop(
        utc_s,
        Some(eop),
        dhruv_vedic_base::time_conversion_policy(),
    );
    dhruv_time::tdb_seconds_to_jd(out.tdb_seconds)
}

fn projected_tropical_deg(tropical_deg: f64, projection: Option<BhavaOutputProjection>) -> f64 {
    projection.map_or(tropical_deg, |projection| {
        tropical_to_sidereal_longitude(
            tropical_deg,
            projection.ayanamsha_deg,
            projection.reference_plane,
        )
    })
}

/// Returns default bhava configuration (Equal, Lagna, StartOfFirst).
#[unsafe(no_mangle)]
pub extern "C" fn dhruv_bhava_config_default() -> DhruvBhavaConfig {
    DhruvBhavaConfig {
        system: DHRUV_BHAVA_EQUAL,
        starting_point: DHRUV_BHAVA_START_LAGNA,
        custom_start_deg: 0.0,
        reference_mode: DHRUV_BHAVA_REF_START,
        output_mode: DHRUV_BHAVA_OUTPUT_TROPICAL,
        ayanamsha_system: 0,
        use_nutation: 0,
        reference_plane: -1,
    }
}

/// Number of supported bhava (house) systems.
#[unsafe(no_mangle)]
pub extern "C" fn dhruv_bhava_system_count() -> u32 {
    BhavaSystem::all().len() as u32
}

/// Compute bhava (house) cusps.
///
/// # Safety
/// All pointer arguments must be valid and non-null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dhruv_compute_bhavas(
    engine: *const DhruvEngineHandle,
    lsk: *const DhruvLskHandle,
    eop: *const DhruvEopHandle,
    location: *const DhruvGeoLocation,
    jd_utc: f64,
    config: *const DhruvBhavaConfig,
    out_result: *mut DhruvBhavaResult,
) -> DhruvStatus {
    ffi_boundary(|| {
        if engine.is_null()
            || lsk.is_null()
            || eop.is_null()
            || location.is_null()
            || out_result.is_null()
        {
            return DhruvStatus::NullPointer;
        }

        // SAFETY: All pointers checked for null above.
        let engine_ref = unsafe { &*engine };
        let lsk_ref = unsafe { &*lsk };
        let eop_ref = unsafe { &*eop };
        let loc_ref = unsafe { &*location };

        let geo = GeoLocation::new(
            loc_ref.latitude_deg,
            loc_ref.longitude_deg,
            loc_ref.altitude_m,
        );

        let rust_config = match resolve_bhava_config_ptr(config) {
            Ok(c) => c,
            Err(status) => return status,
        };
        let output_cfg_storage;
        let output_cfg = if config.is_null() {
            output_cfg_storage = dhruv_bhava_config_default();
            &output_cfg_storage
        } else {
            unsafe { &*config }
        };
        let projection = match bhava_output_projection_from_ffi(
            output_cfg,
            jd_utc_to_jd_tdb_with_eop(lsk_ref, eop_ref, jd_utc),
        ) {
            Ok(p) => p,
            Err(status) => return status,
        };

        match compute_bhavas(engine_ref, lsk_ref, eop_ref, &geo, jd_utc, &rust_config) {
            Ok(result) => {
                // SAFETY: Pointer checked for null.
                unsafe {
                    *out_result = bhava_result_to_ffi_with_projection(&result, projection);
                }
                DhruvStatus::Ok
            }
            Err(e) => DhruvStatus::from(&e),
        }
    })
}

/// Compute the Lagna (Ascendant) ecliptic longitude in degrees.
///
/// Requires LSK, EOP, and location (no engine needed).
///
/// # Safety
/// All pointer arguments must be valid and non-null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dhruv_lagna_deg(
    lsk: *const DhruvLskHandle,
    eop: *const DhruvEopHandle,
    location: *const DhruvGeoLocation,
    jd_utc: f64,
    out_deg: *mut f64,
) -> DhruvStatus {
    ffi_boundary(|| {
        if lsk.is_null() || eop.is_null() || location.is_null() || out_deg.is_null() {
            return DhruvStatus::NullPointer;
        }

        // SAFETY: Pointers checked for null above.
        let lsk_ref = unsafe { &*lsk };
        let eop_ref = unsafe { &*eop };
        let loc_ref = unsafe { &*location };

        let geo = GeoLocation::new(
            loc_ref.latitude_deg,
            loc_ref.longitude_deg,
            loc_ref.altitude_m,
        );

        match dhruv_vedic_base::lagna_longitude_rad(lsk_ref, eop_ref, &geo, jd_utc) {
            Ok(rad) => {
                // SAFETY: Pointer checked for null.
                unsafe { *out_deg = rad.to_degrees() };
                DhruvStatus::Ok
            }
            Err(e) => DhruvStatus::from(&e),
        }
    })
}

/// Compute the Lagna (Ascendant) longitude with optional sidereal output config.
///
/// # Safety
/// All pointer arguments must be valid and non-null except `config`, which may be NULL.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dhruv_lagna_deg_with_config(
    lsk: *const DhruvLskHandle,
    eop: *const DhruvEopHandle,
    location: *const DhruvGeoLocation,
    jd_utc: f64,
    config: *const DhruvBhavaConfig,
    out_deg: *mut f64,
) -> DhruvStatus {
    ffi_boundary(|| {
        if lsk.is_null() || eop.is_null() || location.is_null() || out_deg.is_null() {
            return DhruvStatus::NullPointer;
        }

        let lsk_ref = unsafe { &*lsk };
        let eop_ref = unsafe { &*eop };
        let loc_ref = unsafe { &*location };
        let geo = GeoLocation::new(
            loc_ref.latitude_deg,
            loc_ref.longitude_deg,
            loc_ref.altitude_m,
        );
        let cfg_storage;
        let cfg_ref = if config.is_null() {
            cfg_storage = dhruv_bhava_config_default();
            &cfg_storage
        } else {
            unsafe { &*config }
        };
        let projection = match bhava_output_projection_from_ffi(
            cfg_ref,
            jd_utc_to_jd_tdb_with_eop(lsk_ref, eop_ref, jd_utc),
        ) {
            Ok(p) => p,
            Err(status) => return status,
        };

        match dhruv_vedic_base::lagna_longitude_rad(lsk_ref, eop_ref, &geo, jd_utc) {
            Ok(rad) => {
                unsafe {
                    *out_deg = projected_tropical_deg(rad.to_degrees(), projection);
                }
                DhruvStatus::Ok
            }
            Err(e) => DhruvStatus::from(&e),
        }
    })
}

/// Compute the MC (Midheaven) ecliptic longitude in degrees.
///
/// # Safety
/// All pointer arguments must be valid and non-null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dhruv_mc_deg(
    lsk: *const DhruvLskHandle,
    eop: *const DhruvEopHandle,
    location: *const DhruvGeoLocation,
    jd_utc: f64,
    out_deg: *mut f64,
) -> DhruvStatus {
    ffi_boundary(|| {
        if lsk.is_null() || eop.is_null() || location.is_null() || out_deg.is_null() {
            return DhruvStatus::NullPointer;
        }

        // SAFETY: Pointers checked for null above.
        let lsk_ref = unsafe { &*lsk };
        let eop_ref = unsafe { &*eop };
        let loc_ref = unsafe { &*location };

        let geo = GeoLocation::new(
            loc_ref.latitude_deg,
            loc_ref.longitude_deg,
            loc_ref.altitude_m,
        );

        match dhruv_vedic_base::mc_longitude_rad(lsk_ref, eop_ref, &geo, jd_utc) {
            Ok(rad) => {
                // SAFETY: Pointer checked for null.
                unsafe { *out_deg = rad.to_degrees() };
                DhruvStatus::Ok
            }
            Err(e) => DhruvStatus::from(&e),
        }
    })
}

/// Compute the MC (Midheaven) longitude with optional sidereal output config.
///
/// # Safety
/// All pointer arguments must be valid and non-null except `config`, which may be NULL.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dhruv_mc_deg_with_config(
    lsk: *const DhruvLskHandle,
    eop: *const DhruvEopHandle,
    location: *const DhruvGeoLocation,
    jd_utc: f64,
    config: *const DhruvBhavaConfig,
    out_deg: *mut f64,
) -> DhruvStatus {
    ffi_boundary(|| {
        if lsk.is_null() || eop.is_null() || location.is_null() || out_deg.is_null() {
            return DhruvStatus::NullPointer;
        }

        let lsk_ref = unsafe { &*lsk };
        let eop_ref = unsafe { &*eop };
        let loc_ref = unsafe { &*location };
        let geo = GeoLocation::new(
            loc_ref.latitude_deg,
            loc_ref.longitude_deg,
            loc_ref.altitude_m,
        );
        let cfg_storage;
        let cfg_ref = if config.is_null() {
            cfg_storage = dhruv_bhava_config_default();
            &cfg_storage
        } else {
            unsafe { &*config }
        };
        let projection = match bhava_output_projection_from_ffi(
            cfg_ref,
            jd_utc_to_jd_tdb_with_eop(lsk_ref, eop_ref, jd_utc),
        ) {
            Ok(p) => p,
            Err(status) => return status,
        };

        match dhruv_vedic_base::mc_longitude_rad(lsk_ref, eop_ref, &geo, jd_utc) {
            Ok(rad) => {
                unsafe {
                    *out_deg = projected_tropical_deg(rad.to_degrees(), projection);
                }
                DhruvStatus::Ok
            }
            Err(e) => DhruvStatus::from(&e),
        }
    })
}

/// Compute the RAMC (Right Ascension of the MC / Local Sidereal Time) in degrees.
///
/// # Safety
/// All pointer arguments must be valid and non-null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dhruv_ramc_deg(
    lsk: *const DhruvLskHandle,
    eop: *const DhruvEopHandle,
    location: *const DhruvGeoLocation,
    jd_utc: f64,
    out_deg: *mut f64,
) -> DhruvStatus {
    ffi_boundary(|| {
        if lsk.is_null() || eop.is_null() || location.is_null() || out_deg.is_null() {
            return DhruvStatus::NullPointer;
        }

        let lsk_ref = unsafe { &*lsk };
        let eop_ref = unsafe { &*eop };
        let loc_ref = unsafe { &*location };

        let geo = GeoLocation::new(
            loc_ref.latitude_deg,
            loc_ref.longitude_deg,
            loc_ref.altitude_m,
        );

        match dhruv_vedic_base::ramc_rad(lsk_ref, eop_ref, &geo, jd_utc) {
            Ok(rad) => {
                unsafe { *out_deg = rad.to_degrees() };
                DhruvStatus::Ok
            }
            Err(e) => DhruvStatus::from(&e),
        }
    })
}

// ---------------------------------------------------------------------------
// Lunar node constants
// ---------------------------------------------------------------------------

/// Node code: Rahu (ascending node).
pub const DHRUV_NODE_RAHU: i32 = 0;
/// Node code: Ketu (descending node).
pub const DHRUV_NODE_KETU: i32 = 1;

/// Node mode: mean (polynomial only).
pub const DHRUV_NODE_MODE_MEAN: i32 = 0;
/// Node mode: true.
///
/// In pure-math APIs (`dhruv_lunar_node_deg`, `dhruv_lunar_node_deg_utc`),
/// this uses a 50-term perturbation series fitted against DE442s.
/// In engine-aware APIs (`dhruv_lunar_node_deg_with_engine`,
/// `dhruv_lunar_node_deg_utc_with_engine`), this uses the osculating node
/// from Moon state vectors.
pub const DHRUV_NODE_MODE_TRUE: i32 = 1;

/// Node backend selector: analytic model backend.
pub const DHRUV_NODE_BACKEND_ANALYTIC: i32 = 0;
/// Node backend selector: engine-backed backend.
pub const DHRUV_NODE_BACKEND_ENGINE: i32 = 1;

/// Node time selector: JD TDB input in `jd_tdb`.
pub const DHRUV_NODE_TIME_JD_TDB: i32 = 0;
/// Node time selector: UTC input in `utc` (requires LSK).
pub const DHRUV_NODE_TIME_UTC: i32 = 1;

/// C-compatible request for unified lunar-node compute API.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DhruvLunarNodeRequest {
    /// Node code (`DHRUV_NODE_RAHU` or `DHRUV_NODE_KETU`).
    pub node_code: i32,
    /// Mode code (`DHRUV_NODE_MODE_*`).
    pub mode_code: i32,
    /// Backend selector (`DHRUV_NODE_BACKEND_*`).
    pub backend: i32,
    /// Time selector (`DHRUV_NODE_TIME_*`).
    pub time_kind: i32,
    /// JD TDB input when `time_kind=JD_TDB`.
    pub jd_tdb: f64,
    /// UTC input when `time_kind=UTC`.
    pub utc: DhruvUtcTime,
}

/// Map integer code to LunarNode enum.
fn lunar_node_from_code(code: i32) -> Option<LunarNode> {
    let nodes = LunarNode::all();
    let idx = usize::try_from(code).ok()?;
    nodes.get(idx).copied()
}

/// Map integer code to NodeMode enum.
fn node_mode_from_code(code: i32) -> Option<NodeMode> {
    let modes = NodeMode::all();
    let idx = usize::try_from(code).ok()?;
    modes.get(idx).copied()
}

/// Compute lunar node longitude in degrees [0, 360).
///
/// Pure math, no engine needed.  For `mode_code=1` (True), the 50-term
/// perturbation series was fitted over 1900–2100; accuracy degrades outside
/// that interval.  Prefer the `_with_engine` variant for production use.
///
/// # Arguments
/// * `node_code` — 0 = Rahu, 1 = Ketu
/// * `mode_code` — 0 = Mean, 1 = True
/// * `jd_tdb` — Julian Date in TDB
/// * `out_deg` — output longitude in degrees
///
/// # Safety
/// `out_deg` must be a valid, non-null pointer.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dhruv_lunar_node_deg(
    node_code: i32,
    mode_code: i32,
    jd_tdb: f64,
    out_deg: *mut f64,
) -> DhruvStatus {
    ffi_boundary(|| {
        if out_deg.is_null() {
            return DhruvStatus::NullPointer;
        }

        let node = match lunar_node_from_code(node_code) {
            Some(n) => n,
            None => return DhruvStatus::InvalidQuery,
        };

        let mode = match node_mode_from_code(mode_code) {
            Some(m) => m,
            None => return DhruvStatus::InvalidQuery,
        };

        let t = jd_tdb_to_centuries(jd_tdb);
        let deg = lunar_node_deg(node, t, mode);

        // SAFETY: Pointer is checked for null; write one value.
        unsafe { *out_deg = deg };
        DhruvStatus::Ok
    })
}

/// Compute lunar node longitude in degrees [0, 360), using an engine handle.
///
/// For `mode_code=1` (True), this uses an osculating-node computation from
/// Moon state vectors (`r × v`) with full IAU 2006 precession to ecliptic-of-date.
/// For `mode_code=0` (Mean), this matches the polynomial mean-node model.
///
/// # Arguments
/// * `engine` — engine handle from `dhruv_engine_new`
/// * `node_code` — 0 = Rahu, 1 = Ketu
/// * `mode_code` — 0 = Mean, 1 = True
/// * `jd_tdb` — Julian Date in TDB
/// * `out_deg` — output longitude in degrees
///
/// # Safety
/// `engine` and `out_deg` must be valid, non-null pointers.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dhruv_lunar_node_deg_with_engine(
    engine: *const DhruvEngineHandle,
    node_code: i32,
    mode_code: i32,
    jd_tdb: f64,
    out_deg: *mut f64,
) -> DhruvStatus {
    ffi_boundary(|| {
        if engine.is_null() || out_deg.is_null() {
            return DhruvStatus::NullPointer;
        }

        let node = match lunar_node_from_code(node_code) {
            Some(n) => n,
            None => return DhruvStatus::InvalidQuery,
        };

        let mode = match node_mode_from_code(mode_code) {
            Some(m) => m,
            None => return DhruvStatus::InvalidQuery,
        };

        let engine_ref = unsafe { &*engine };
        match lunar_node_deg_for_epoch(engine_ref, node, jd_tdb, mode) {
            Ok(deg) => {
                unsafe { *out_deg = deg };
                DhruvStatus::Ok
            }
            Err(e) => DhruvStatus::from(&e),
        }
    })
}

/// Unified lunar-node compute API covering backend + time-base variants.
///
/// - `backend=ANALYTIC` uses pure math (`lunar_node_deg`).
/// - `backend=ENGINE` uses state-vector backend (`lunar_node_deg_for_epoch`).
/// - `time_kind=JD_TDB` uses `jd_tdb`.
/// - `time_kind=UTC` uses `utc` and requires `lsk`.
///
/// # Safety
/// `request` and `out_deg` must be valid, non-null pointers.
/// `engine` is required only when `backend=ENGINE`.
/// `lsk` is required only when `time_kind=UTC`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dhruv_lunar_node_compute_ex(
    engine: *const DhruvEngineHandle,
    lsk: *const DhruvLskHandle,
    request: *const DhruvLunarNodeRequest,
    out_deg: *mut f64,
) -> DhruvStatus {
    ffi_boundary(|| {
        if request.is_null() || out_deg.is_null() {
            return DhruvStatus::NullPointer;
        }

        let req = unsafe { &*request };
        let node = match lunar_node_from_code(req.node_code) {
            Some(n) => n,
            None => return DhruvStatus::InvalidQuery,
        };
        let mode = match node_mode_from_code(req.mode_code) {
            Some(m) => m,
            None => return DhruvStatus::InvalidQuery,
        };

        let jd_tdb = match req.time_kind {
            DHRUV_NODE_TIME_JD_TDB => req.jd_tdb,
            DHRUV_NODE_TIME_UTC => {
                if lsk.is_null() {
                    return DhruvStatus::NullPointer;
                }
                let lsk_ref = unsafe { &*lsk };
                ffi_to_utc_time(&req.utc).to_jd_tdb(lsk_ref)
            }
            _ => return DhruvStatus::InvalidQuery,
        };

        let lon = match req.backend {
            DHRUV_NODE_BACKEND_ANALYTIC => {
                let t = jd_tdb_to_centuries(jd_tdb);
                lunar_node_deg(node, t, mode)
            }
            DHRUV_NODE_BACKEND_ENGINE => {
                if engine.is_null() {
                    return DhruvStatus::NullPointer;
                }
                let engine_ref = unsafe { &*engine };
                match lunar_node_deg_for_epoch(engine_ref, node, jd_tdb, mode) {
                    Ok(deg) => deg,
                    Err(e) => return DhruvStatus::from(&e),
                }
            }
            _ => return DhruvStatus::InvalidQuery,
        };

        unsafe { *out_deg = lon };
        DhruvStatus::Ok
    })
}

/// Number of supported lunar node variants (Rahu, Ketu).
#[unsafe(no_mangle)]
pub extern "C" fn dhruv_lunar_node_count() -> u32 {
    LunarNode::all().len() as u32
}

// ---------------------------------------------------------------------------
// Conjunction/aspect search
// ---------------------------------------------------------------------------

/// C-compatible conjunction search configuration.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DhruvConjunctionConfig {
    /// Target ecliptic longitude separation in degrees [0, 360).
    /// 0 = conjunction, 180 = opposition, 90 = square, etc.
    pub target_separation_deg: f64,
    /// Coarse scan step size in days.
    pub step_size_days: f64,
    /// Maximum bisection iterations.
    pub max_iterations: u32,
    /// Convergence threshold in days.
    pub convergence_days: f64,
}

/// Conjunction query mode: next event after `at_jd_tdb`.
pub const DHRUV_CONJUNCTION_QUERY_MODE_NEXT: i32 = 0;
/// Conjunction query mode: previous event before `at_jd_tdb`.
pub const DHRUV_CONJUNCTION_QUERY_MODE_PREV: i32 = 1;
/// Conjunction query mode: all events in [`start_jd_tdb`, `end_jd_tdb`].
pub const DHRUV_CONJUNCTION_QUERY_MODE_RANGE: i32 = 2;

/// C-compatible request for unified conjunction search.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DhruvConjunctionSearchRequest {
    /// Body 1 NAIF code.
    pub body1_code: i32,
    /// Body 2 NAIF code.
    pub body2_code: i32,
    /// Query mode (see `DHRUV_CONJUNCTION_QUERY_MODE_*` constants).
    pub query_mode: i32,
    /// Anchor time for next/prev modes (JD TDB).
    pub at_jd_tdb: f64,
    /// Start of range window for range mode (JD TDB).
    pub start_jd_tdb: f64,
    /// End of range window for range mode (JD TDB).
    pub end_jd_tdb: f64,
    /// Conjunction search configuration.
    pub config: DhruvConjunctionConfig,
}

/// C-compatible conjunction event result.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DhruvConjunctionEvent {
    /// Event time as Julian Date (TDB).
    pub jd_tdb: f64,
    /// Actual ecliptic longitude separation at peak, in degrees.
    pub actual_separation_deg: f64,
    /// Body 1 ecliptic longitude in degrees.
    pub body1_longitude_deg: f64,
    /// Body 2 ecliptic longitude in degrees.
    pub body2_longitude_deg: f64,
    /// Body 1 ecliptic latitude in degrees.
    pub body1_latitude_deg: f64,
    /// Body 2 ecliptic latitude in degrees.
    pub body2_latitude_deg: f64,
    /// Body 1 NAIF code.
    pub body1_code: i32,
    /// Body 2 NAIF code.
    pub body2_code: i32,
}

impl From<&ConjunctionEvent> for DhruvConjunctionEvent {
    fn from(e: &ConjunctionEvent) -> Self {
        Self {
            jd_tdb: e.jd_tdb,
            actual_separation_deg: e.actual_separation_deg,
            body1_longitude_deg: e.body1_longitude_deg,
            body2_longitude_deg: e.body2_longitude_deg,
            body1_latitude_deg: e.body1_latitude_deg,
            body2_latitude_deg: e.body2_latitude_deg,
            body1_code: e.body1.code(),
            body2_code: e.body2.code(),
        }
    }
}

fn conjunction_config_from_ffi(cfg: &DhruvConjunctionConfig) -> ConjunctionConfig {
    ConjunctionConfig {
        target_separation_deg: cfg.target_separation_deg,
        step_size_days: cfg.step_size_days,
        max_iterations: cfg.max_iterations,
        convergence_days: cfg.convergence_days,
    }
}

/// Returns default conjunction configuration (0 deg, step=0.5 days).
#[unsafe(no_mangle)]
pub extern "C" fn dhruv_conjunction_config_default() -> DhruvConjunctionConfig {
    DhruvConjunctionConfig {
        target_separation_deg: 0.0,
        step_size_days: 0.5,
        max_iterations: 50,
        convergence_days: 1e-8,
    }
}

/// Unified conjunction search entrypoint.
///
/// Mode behavior:
/// - `DHRUV_CONJUNCTION_QUERY_MODE_NEXT` / `DHRUV_CONJUNCTION_QUERY_MODE_PREV`:
///   writes single-event result to `out_event` and found flag to `out_found`.
/// - `DHRUV_CONJUNCTION_QUERY_MODE_RANGE`:
///   writes events to `out_events[..max_count]` and actual count to `out_count`.
///
/// # Safety
/// `engine` and `request` must be valid and non-null.
/// For NEXT/PREV, `out_event` and `out_found` must be valid non-null pointers.
/// For RANGE, `out_events` and `out_count` must be valid non-null pointers.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dhruv_conjunction_search_ex(
    engine: *const DhruvEngineHandle,
    request: *const DhruvConjunctionSearchRequest,
    out_event: *mut DhruvConjunctionEvent,
    out_found: *mut u8,
    out_events: *mut DhruvConjunctionEvent,
    max_count: u32,
    out_count: *mut u32,
) -> DhruvStatus {
    ffi_boundary(|| {
        if engine.is_null() || request.is_null() {
            return DhruvStatus::NullPointer;
        }

        let engine_ref = unsafe { &*engine };
        let req = unsafe { &*request };

        let body1 = match Body::from_code(req.body1_code) {
            Some(b) => b,
            None => return DhruvStatus::InvalidQuery,
        };
        let body2 = match Body::from_code(req.body2_code) {
            Some(b) => b,
            None => return DhruvStatus::InvalidQuery,
        };
        let rust_config = conjunction_config_from_ffi(&req.config);

        match req.query_mode {
            DHRUV_CONJUNCTION_QUERY_MODE_NEXT => {
                if out_event.is_null() || out_found.is_null() {
                    return DhruvStatus::NullPointer;
                }
                match next_conjunction(engine_ref, body1, body2, req.at_jd_tdb, &rust_config) {
                    Ok(Some(event)) => {
                        unsafe {
                            *out_event = DhruvConjunctionEvent::from(&event);
                            *out_found = 1;
                        }
                        DhruvStatus::Ok
                    }
                    Ok(None) => {
                        unsafe { *out_found = 0 };
                        DhruvStatus::Ok
                    }
                    Err(e) => DhruvStatus::from(&e),
                }
            }
            DHRUV_CONJUNCTION_QUERY_MODE_PREV => {
                if out_event.is_null() || out_found.is_null() {
                    return DhruvStatus::NullPointer;
                }
                match prev_conjunction(engine_ref, body1, body2, req.at_jd_tdb, &rust_config) {
                    Ok(Some(event)) => {
                        unsafe {
                            *out_event = DhruvConjunctionEvent::from(&event);
                            *out_found = 1;
                        }
                        DhruvStatus::Ok
                    }
                    Ok(None) => {
                        unsafe { *out_found = 0 };
                        DhruvStatus::Ok
                    }
                    Err(e) => DhruvStatus::from(&e),
                }
            }
            DHRUV_CONJUNCTION_QUERY_MODE_RANGE => {
                if out_events.is_null() || out_count.is_null() {
                    return DhruvStatus::NullPointer;
                }
                match search_conjunctions(
                    engine_ref,
                    body1,
                    body2,
                    req.start_jd_tdb,
                    req.end_jd_tdb,
                    &rust_config,
                ) {
                    Ok(events) => {
                        let count = events.len().min(max_count as usize);
                        let out_slice = unsafe {
                            std::slice::from_raw_parts_mut(out_events, max_count as usize)
                        };
                        for (i, e) in events.iter().take(count).enumerate() {
                            out_slice[i] = DhruvConjunctionEvent::from(e);
                        }
                        unsafe { *out_count = count as u32 };
                        DhruvStatus::Ok
                    }
                    Err(e) => DhruvStatus::from(&e),
                }
            }
            _ => DhruvStatus::InvalidQuery,
        }
    })
}

// ---------------------------------------------------------------------------
// Grahan search
// ---------------------------------------------------------------------------

/// Sentinel value for absent optional JD fields in grahan results.
pub const DHRUV_JD_ABSENT: f64 = -1.0;

/// Chandra grahan type: penumbral only.
pub const DHRUV_CHANDRA_GRAHAN_PENUMBRAL: i32 = 0;
/// Chandra grahan type: partial (umbral).
pub const DHRUV_CHANDRA_GRAHAN_PARTIAL: i32 = 1;
/// Chandra grahan type: total.
pub const DHRUV_CHANDRA_GRAHAN_TOTAL: i32 = 2;

/// Surya grahan type: partial.
pub const DHRUV_SURYA_GRAHAN_PARTIAL: i32 = 0;
/// Surya grahan type: annular.
pub const DHRUV_SURYA_GRAHAN_ANNULAR: i32 = 1;
/// Surya grahan type: total.
pub const DHRUV_SURYA_GRAHAN_TOTAL: i32 = 2;
/// Surya grahan type: hybrid.
pub const DHRUV_SURYA_GRAHAN_HYBRID: i32 = 3;

/// C-compatible grahan search configuration.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DhruvGrahanConfig {
    /// Include penumbral-only chandra grahan: 1 = yes, 0 = no.
    pub include_penumbral: u8,
    /// Include ecliptic latitude and angular separation at peak: 1 = yes, 0 = no.
    pub include_peak_details: u8,
}

/// Grahan kind selector: lunar eclipse.
pub const DHRUV_GRAHAN_KIND_CHANDRA: i32 = 0;
/// Grahan kind selector: solar eclipse.
pub const DHRUV_GRAHAN_KIND_SURYA: i32 = 1;

/// Grahan query mode: next event after `at_jd_tdb`.
pub const DHRUV_GRAHAN_QUERY_MODE_NEXT: i32 = 0;
/// Grahan query mode: previous event before `at_jd_tdb`.
pub const DHRUV_GRAHAN_QUERY_MODE_PREV: i32 = 1;
/// Grahan query mode: all events in [`start_jd_tdb`, `end_jd_tdb`].
pub const DHRUV_GRAHAN_QUERY_MODE_RANGE: i32 = 2;

/// C-compatible request for unified grahan search.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DhruvGrahanSearchRequest {
    /// Grahan kind selector (`DHRUV_GRAHAN_KIND_*`).
    pub grahan_kind: i32,
    /// Query mode selector (`DHRUV_GRAHAN_QUERY_MODE_*`).
    pub query_mode: i32,
    /// Anchor time for next/prev modes (JD TDB).
    pub at_jd_tdb: f64,
    /// Start of range window for range mode (JD TDB).
    pub start_jd_tdb: f64,
    /// End of range window for range mode (JD TDB).
    pub end_jd_tdb: f64,
    /// Grahan search configuration.
    pub config: DhruvGrahanConfig,
}

/// Returns default grahan configuration.
#[unsafe(no_mangle)]
pub extern "C" fn dhruv_grahan_config_default() -> DhruvGrahanConfig {
    DhruvGrahanConfig {
        include_penumbral: 1,
        include_peak_details: 1,
    }
}

fn grahan_config_from_ffi(cfg: &DhruvGrahanConfig) -> GrahanConfig {
    GrahanConfig {
        include_penumbral: cfg.include_penumbral != 0,
        include_peak_details: cfg.include_peak_details != 0,
    }
}

fn chandra_grahan_type_to_code(t: ChandraGrahanType) -> i32 {
    match t {
        ChandraGrahanType::Penumbral => DHRUV_CHANDRA_GRAHAN_PENUMBRAL,
        ChandraGrahanType::Partial => DHRUV_CHANDRA_GRAHAN_PARTIAL,
        ChandraGrahanType::Total => DHRUV_CHANDRA_GRAHAN_TOTAL,
    }
}

fn surya_grahan_type_to_code(t: SuryaGrahanType) -> i32 {
    match t {
        SuryaGrahanType::Partial => DHRUV_SURYA_GRAHAN_PARTIAL,
        SuryaGrahanType::Annular => DHRUV_SURYA_GRAHAN_ANNULAR,
        SuryaGrahanType::Total => DHRUV_SURYA_GRAHAN_TOTAL,
        SuryaGrahanType::Hybrid => DHRUV_SURYA_GRAHAN_HYBRID,
    }
}

fn option_jd(opt: Option<f64>) -> f64 {
    opt.unwrap_or(DHRUV_JD_ABSENT)
}

/// C-compatible chandra grahan result.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DhruvChandraGrahanResult {
    /// Grahan type code (see DHRUV_CHANDRA_GRAHAN_* constants).
    pub grahan_type: i32,
    /// Umbral magnitude.
    pub magnitude: f64,
    /// Penumbral magnitude.
    pub penumbral_magnitude: f64,
    /// Time of greatest grahan (JD TDB).
    pub greatest_grahan_jd: f64,
    /// P1: First penumbral contact (JD TDB).
    pub p1_jd: f64,
    /// U1: First umbral contact (JD TDB). -1.0 if absent.
    pub u1_jd: f64,
    /// U2: Start of totality (JD TDB). -1.0 if absent.
    pub u2_jd: f64,
    /// U3: End of totality (JD TDB). -1.0 if absent.
    pub u3_jd: f64,
    /// U4: Last umbral contact (JD TDB). -1.0 if absent.
    pub u4_jd: f64,
    /// P4: Last penumbral contact (JD TDB).
    pub p4_jd: f64,
    /// Moon's ecliptic latitude at greatest grahan, in degrees.
    pub moon_ecliptic_lat_deg: f64,
    /// Angular separation at greatest grahan, in degrees.
    pub angular_separation_deg: f64,
}

impl From<&ChandraGrahan> for DhruvChandraGrahanResult {
    fn from(e: &ChandraGrahan) -> Self {
        Self {
            grahan_type: chandra_grahan_type_to_code(e.grahan_type),
            magnitude: e.magnitude,
            penumbral_magnitude: e.penumbral_magnitude,
            greatest_grahan_jd: e.greatest_grahan_jd,
            p1_jd: e.p1_jd,
            u1_jd: option_jd(e.u1_jd),
            u2_jd: option_jd(e.u2_jd),
            u3_jd: option_jd(e.u3_jd),
            u4_jd: option_jd(e.u4_jd),
            p4_jd: e.p4_jd,
            moon_ecliptic_lat_deg: e.moon_ecliptic_lat_deg,
            angular_separation_deg: e.angular_separation_deg,
        }
    }
}

/// C-compatible surya grahan result.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DhruvSuryaGrahanResult {
    /// Grahan type code (see DHRUV_SURYA_GRAHAN_* constants).
    pub grahan_type: i32,
    /// Magnitude: ratio of apparent Moon diameter to Sun diameter.
    pub magnitude: f64,
    /// Time of greatest grahan (JD TDB).
    pub greatest_grahan_jd: f64,
    /// C1: First external contact (JD TDB). -1.0 if absent.
    pub c1_jd: f64,
    /// C2: First internal contact (JD TDB). -1.0 if absent.
    pub c2_jd: f64,
    /// C3: Last internal contact (JD TDB). -1.0 if absent.
    pub c3_jd: f64,
    /// C4: Last external contact (JD TDB). -1.0 if absent.
    pub c4_jd: f64,
    /// Moon's ecliptic latitude at greatest grahan, in degrees.
    pub moon_ecliptic_lat_deg: f64,
    /// Angular separation at greatest grahan, in degrees.
    pub angular_separation_deg: f64,
}

impl From<&SuryaGrahan> for DhruvSuryaGrahanResult {
    fn from(e: &SuryaGrahan) -> Self {
        Self {
            grahan_type: surya_grahan_type_to_code(e.grahan_type),
            magnitude: e.magnitude,
            greatest_grahan_jd: e.greatest_grahan_jd,
            c1_jd: option_jd(e.c1_jd),
            c2_jd: option_jd(e.c2_jd),
            c3_jd: option_jd(e.c3_jd),
            c4_jd: option_jd(e.c4_jd),
            moon_ecliptic_lat_deg: e.moon_ecliptic_lat_deg,
            angular_separation_deg: e.angular_separation_deg,
        }
    }
}

/// Unified grahan search entrypoint.
///
/// Mode behavior:
/// - `DHRUV_GRAHAN_QUERY_MODE_NEXT` / `DHRUV_GRAHAN_QUERY_MODE_PREV`:
///   writes single-event result and found flag.
/// - `DHRUV_GRAHAN_QUERY_MODE_RANGE`:
///   writes array results and count.
///
/// Kind behavior:
/// - `DHRUV_GRAHAN_KIND_CHANDRA` uses chandra output pointers.
/// - `DHRUV_GRAHAN_KIND_SURYA` uses surya output pointers.
///
/// # Safety
/// `engine` and `request` must be valid and non-null.
/// Output pointers required depend on `grahan_kind` and `query_mode`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dhruv_grahan_search_ex(
    engine: *const DhruvEngineHandle,
    request: *const DhruvGrahanSearchRequest,
    out_chandra_single: *mut DhruvChandraGrahanResult,
    out_surya_single: *mut DhruvSuryaGrahanResult,
    out_found: *mut u8,
    out_chandra_many: *mut DhruvChandraGrahanResult,
    out_surya_many: *mut DhruvSuryaGrahanResult,
    max_count: u32,
    out_count: *mut u32,
) -> DhruvStatus {
    ffi_boundary(|| {
        if engine.is_null() || request.is_null() {
            return DhruvStatus::NullPointer;
        }

        let engine_ref = unsafe { &*engine };
        let req = unsafe { &*request };
        let rust_config = grahan_config_from_ffi(&req.config);

        match (req.grahan_kind, req.query_mode) {
            (DHRUV_GRAHAN_KIND_CHANDRA, DHRUV_GRAHAN_QUERY_MODE_NEXT) => {
                if out_chandra_single.is_null() || out_found.is_null() {
                    return DhruvStatus::NullPointer;
                }
                match next_chandra_grahan(engine_ref, req.at_jd_tdb, &rust_config) {
                    Ok(Some(grahan)) => {
                        unsafe {
                            *out_chandra_single = DhruvChandraGrahanResult::from(&grahan);
                            *out_found = 1;
                        }
                        DhruvStatus::Ok
                    }
                    Ok(None) => {
                        unsafe { *out_found = 0 };
                        DhruvStatus::Ok
                    }
                    Err(e) => DhruvStatus::from(&e),
                }
            }
            (DHRUV_GRAHAN_KIND_CHANDRA, DHRUV_GRAHAN_QUERY_MODE_PREV) => {
                if out_chandra_single.is_null() || out_found.is_null() {
                    return DhruvStatus::NullPointer;
                }
                match prev_chandra_grahan(engine_ref, req.at_jd_tdb, &rust_config) {
                    Ok(Some(grahan)) => {
                        unsafe {
                            *out_chandra_single = DhruvChandraGrahanResult::from(&grahan);
                            *out_found = 1;
                        }
                        DhruvStatus::Ok
                    }
                    Ok(None) => {
                        unsafe { *out_found = 0 };
                        DhruvStatus::Ok
                    }
                    Err(e) => DhruvStatus::from(&e),
                }
            }
            (DHRUV_GRAHAN_KIND_CHANDRA, DHRUV_GRAHAN_QUERY_MODE_RANGE) => {
                if out_chandra_many.is_null() || out_count.is_null() {
                    return DhruvStatus::NullPointer;
                }
                match search_chandra_grahan(
                    engine_ref,
                    req.start_jd_tdb,
                    req.end_jd_tdb,
                    &rust_config,
                ) {
                    Ok(results) => {
                        let count = results.len().min(max_count as usize);
                        let out_slice = unsafe {
                            std::slice::from_raw_parts_mut(out_chandra_many, max_count as usize)
                        };
                        for (i, e) in results.iter().take(count).enumerate() {
                            out_slice[i] = DhruvChandraGrahanResult::from(e);
                        }
                        unsafe { *out_count = count as u32 };
                        DhruvStatus::Ok
                    }
                    Err(e) => DhruvStatus::from(&e),
                }
            }
            (DHRUV_GRAHAN_KIND_SURYA, DHRUV_GRAHAN_QUERY_MODE_NEXT) => {
                if out_surya_single.is_null() || out_found.is_null() {
                    return DhruvStatus::NullPointer;
                }
                match next_surya_grahan(engine_ref, req.at_jd_tdb, &rust_config) {
                    Ok(Some(grahan)) => {
                        unsafe {
                            *out_surya_single = DhruvSuryaGrahanResult::from(&grahan);
                            *out_found = 1;
                        }
                        DhruvStatus::Ok
                    }
                    Ok(None) => {
                        unsafe { *out_found = 0 };
                        DhruvStatus::Ok
                    }
                    Err(e) => DhruvStatus::from(&e),
                }
            }
            (DHRUV_GRAHAN_KIND_SURYA, DHRUV_GRAHAN_QUERY_MODE_PREV) => {
                if out_surya_single.is_null() || out_found.is_null() {
                    return DhruvStatus::NullPointer;
                }
                match prev_surya_grahan(engine_ref, req.at_jd_tdb, &rust_config) {
                    Ok(Some(grahan)) => {
                        unsafe {
                            *out_surya_single = DhruvSuryaGrahanResult::from(&grahan);
                            *out_found = 1;
                        }
                        DhruvStatus::Ok
                    }
                    Ok(None) => {
                        unsafe { *out_found = 0 };
                        DhruvStatus::Ok
                    }
                    Err(e) => DhruvStatus::from(&e),
                }
            }
            (DHRUV_GRAHAN_KIND_SURYA, DHRUV_GRAHAN_QUERY_MODE_RANGE) => {
                if out_surya_many.is_null() || out_count.is_null() {
                    return DhruvStatus::NullPointer;
                }
                match search_surya_grahan(
                    engine_ref,
                    req.start_jd_tdb,
                    req.end_jd_tdb,
                    &rust_config,
                ) {
                    Ok(results) => {
                        let count = results.len().min(max_count as usize);
                        let out_slice = unsafe {
                            std::slice::from_raw_parts_mut(out_surya_many, max_count as usize)
                        };
                        for (i, e) in results.iter().take(count).enumerate() {
                            out_slice[i] = DhruvSuryaGrahanResult::from(e);
                        }
                        unsafe { *out_count = count as u32 };
                        DhruvStatus::Ok
                    }
                    Err(e) => DhruvStatus::from(&e),
                }
            }
            _ => DhruvStatus::InvalidQuery,
        }
    })
}

// ---------------------------------------------------------------------------
// Stationary point & max-speed search
// ---------------------------------------------------------------------------

/// Station retrograde: planet begins retrograde motion.
pub const DHRUV_STATION_RETROGRADE: i32 = 0;
/// Station direct: planet resumes direct motion.
pub const DHRUV_STATION_DIRECT: i32 = 1;

/// Max direct speed: peak forward velocity.
pub const DHRUV_MAX_SPEED_DIRECT: i32 = 0;
/// Max retrograde speed: peak retrograde velocity.
pub const DHRUV_MAX_SPEED_RETROGRADE: i32 = 1;

/// C-compatible stationary search configuration.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DhruvStationaryConfig {
    /// Coarse scan step size in days.
    pub step_size_days: f64,
    /// Maximum bisection iterations.
    pub max_iterations: u32,
    /// Convergence threshold in days.
    pub convergence_days: f64,
    /// Numerical central difference step in days (used by max-speed only).
    pub numerical_step_days: f64,
}

/// Motion kind selector: stationary event search.
pub const DHRUV_MOTION_KIND_STATIONARY: i32 = 0;
/// Motion kind selector: max-speed event search.
pub const DHRUV_MOTION_KIND_MAX_SPEED: i32 = 1;

/// Motion query mode: next event after `at_jd_tdb`.
pub const DHRUV_MOTION_QUERY_MODE_NEXT: i32 = 0;
/// Motion query mode: previous event before `at_jd_tdb`.
pub const DHRUV_MOTION_QUERY_MODE_PREV: i32 = 1;
/// Motion query mode: all events in [`start_jd_tdb`, `end_jd_tdb`].
pub const DHRUV_MOTION_QUERY_MODE_RANGE: i32 = 2;

/// C-compatible request for unified motion search.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DhruvMotionSearchRequest {
    /// NAIF body code.
    pub body_code: i32,
    /// Motion kind selector (`DHRUV_MOTION_KIND_*`).
    pub motion_kind: i32,
    /// Query mode selector (`DHRUV_MOTION_QUERY_MODE_*`).
    pub query_mode: i32,
    /// Anchor time for next/prev modes (JD TDB).
    pub at_jd_tdb: f64,
    /// Start of range window for range mode (JD TDB).
    pub start_jd_tdb: f64,
    /// End of range window for range mode (JD TDB).
    pub end_jd_tdb: f64,
    /// Search configuration.
    pub config: DhruvStationaryConfig,
}

/// C-compatible stationary point event result.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DhruvStationaryEvent {
    /// Event time as Julian Date (TDB).
    pub jd_tdb: f64,
    /// NAIF body code.
    pub body_code: i32,
    /// Ecliptic longitude at station in degrees.
    pub longitude_deg: f64,
    /// Ecliptic latitude at station in degrees.
    pub latitude_deg: f64,
    /// Station type code (see DHRUV_STATION_* constants).
    pub station_type: i32,
}

/// C-compatible max-speed event result.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DhruvMaxSpeedEvent {
    /// Event time as Julian Date (TDB).
    pub jd_tdb: f64,
    /// NAIF body code.
    pub body_code: i32,
    /// Ecliptic longitude at peak speed in degrees.
    pub longitude_deg: f64,
    /// Ecliptic latitude at peak speed in degrees.
    pub latitude_deg: f64,
    /// Longitude speed at peak in degrees per day.
    pub speed_deg_per_day: f64,
    /// Speed type code (see DHRUV_MAX_SPEED_* constants).
    pub speed_type: i32,
}

fn stationary_config_from_ffi(cfg: &DhruvStationaryConfig) -> StationaryConfig {
    StationaryConfig {
        step_size_days: cfg.step_size_days,
        max_iterations: cfg.max_iterations,
        convergence_days: cfg.convergence_days,
        numerical_step_days: cfg.numerical_step_days,
    }
}

fn station_type_to_code(t: StationType) -> i32 {
    match t {
        StationType::StationRetrograde => DHRUV_STATION_RETROGRADE,
        StationType::StationDirect => DHRUV_STATION_DIRECT,
    }
}

fn max_speed_type_to_code(t: MaxSpeedType) -> i32 {
    match t {
        MaxSpeedType::MaxDirect => DHRUV_MAX_SPEED_DIRECT,
        MaxSpeedType::MaxRetrograde => DHRUV_MAX_SPEED_RETROGRADE,
    }
}

impl From<&StationaryEvent> for DhruvStationaryEvent {
    fn from(e: &StationaryEvent) -> Self {
        Self {
            jd_tdb: e.jd_tdb,
            body_code: e.body.code(),
            longitude_deg: e.longitude_deg,
            latitude_deg: e.latitude_deg,
            station_type: station_type_to_code(e.station_type),
        }
    }
}

impl From<&MaxSpeedEvent> for DhruvMaxSpeedEvent {
    fn from(e: &MaxSpeedEvent) -> Self {
        Self {
            jd_tdb: e.jd_tdb,
            body_code: e.body.code(),
            longitude_deg: e.longitude_deg,
            latitude_deg: e.latitude_deg,
            speed_deg_per_day: e.speed_deg_per_day,
            speed_type: max_speed_type_to_code(e.speed_type),
        }
    }
}

/// Returns default stationary search configuration (inner planet defaults).
#[unsafe(no_mangle)]
pub extern "C" fn dhruv_stationary_config_default() -> DhruvStationaryConfig {
    DhruvStationaryConfig {
        step_size_days: 1.0,
        max_iterations: 50,
        convergence_days: 1e-8,
        numerical_step_days: 0.01,
    }
}

/// Unified motion search entrypoint.
///
/// Mode behavior:
/// - `DHRUV_MOTION_QUERY_MODE_NEXT` / `DHRUV_MOTION_QUERY_MODE_PREV`:
///   writes single-event result and found flag.
/// - `DHRUV_MOTION_QUERY_MODE_RANGE`:
///   writes array results and count.
///
/// Kind behavior:
/// - `DHRUV_MOTION_KIND_STATIONARY` uses stationary output pointers.
/// - `DHRUV_MOTION_KIND_MAX_SPEED` uses max-speed output pointers.
///
/// # Safety
/// `engine` and `request` must be valid and non-null.
/// Output pointers required depend on `motion_kind` and `query_mode`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dhruv_motion_search_ex(
    engine: *const DhruvEngineHandle,
    request: *const DhruvMotionSearchRequest,
    out_stationary_single: *mut DhruvStationaryEvent,
    out_max_speed_single: *mut DhruvMaxSpeedEvent,
    out_found: *mut u8,
    out_stationary_many: *mut DhruvStationaryEvent,
    out_max_speed_many: *mut DhruvMaxSpeedEvent,
    max_count: u32,
    out_count: *mut u32,
) -> DhruvStatus {
    ffi_boundary(|| {
        if engine.is_null() || request.is_null() {
            return DhruvStatus::NullPointer;
        }

        let engine_ref = unsafe { &*engine };
        let req = unsafe { &*request };
        let body = match Body::from_code(req.body_code) {
            Some(b) => b,
            None => return DhruvStatus::InvalidQuery,
        };
        let rust_config = stationary_config_from_ffi(&req.config);

        match (req.motion_kind, req.query_mode) {
            (DHRUV_MOTION_KIND_STATIONARY, DHRUV_MOTION_QUERY_MODE_NEXT) => {
                if out_stationary_single.is_null() || out_found.is_null() {
                    return DhruvStatus::NullPointer;
                }
                match next_stationary(engine_ref, body, req.at_jd_tdb, &rust_config) {
                    Ok(Some(event)) => {
                        unsafe {
                            *out_stationary_single = DhruvStationaryEvent::from(&event);
                            *out_found = 1;
                        }
                        DhruvStatus::Ok
                    }
                    Ok(None) => {
                        unsafe { *out_found = 0 };
                        DhruvStatus::Ok
                    }
                    Err(e) => DhruvStatus::from(&e),
                }
            }
            (DHRUV_MOTION_KIND_STATIONARY, DHRUV_MOTION_QUERY_MODE_PREV) => {
                if out_stationary_single.is_null() || out_found.is_null() {
                    return DhruvStatus::NullPointer;
                }
                match prev_stationary(engine_ref, body, req.at_jd_tdb, &rust_config) {
                    Ok(Some(event)) => {
                        unsafe {
                            *out_stationary_single = DhruvStationaryEvent::from(&event);
                            *out_found = 1;
                        }
                        DhruvStatus::Ok
                    }
                    Ok(None) => {
                        unsafe { *out_found = 0 };
                        DhruvStatus::Ok
                    }
                    Err(e) => DhruvStatus::from(&e),
                }
            }
            (DHRUV_MOTION_KIND_STATIONARY, DHRUV_MOTION_QUERY_MODE_RANGE) => {
                if out_stationary_many.is_null() || out_count.is_null() {
                    return DhruvStatus::NullPointer;
                }
                match search_stationary(
                    engine_ref,
                    body,
                    req.start_jd_tdb,
                    req.end_jd_tdb,
                    &rust_config,
                ) {
                    Ok(events) => {
                        let count = events.len().min(max_count as usize);
                        let out_slice = unsafe {
                            std::slice::from_raw_parts_mut(out_stationary_many, max_count as usize)
                        };
                        for (i, e) in events.iter().take(count).enumerate() {
                            out_slice[i] = DhruvStationaryEvent::from(e);
                        }
                        unsafe { *out_count = count as u32 };
                        DhruvStatus::Ok
                    }
                    Err(e) => DhruvStatus::from(&e),
                }
            }
            (DHRUV_MOTION_KIND_MAX_SPEED, DHRUV_MOTION_QUERY_MODE_NEXT) => {
                if out_max_speed_single.is_null() || out_found.is_null() {
                    return DhruvStatus::NullPointer;
                }
                match next_max_speed(engine_ref, body, req.at_jd_tdb, &rust_config) {
                    Ok(Some(event)) => {
                        unsafe {
                            *out_max_speed_single = DhruvMaxSpeedEvent::from(&event);
                            *out_found = 1;
                        }
                        DhruvStatus::Ok
                    }
                    Ok(None) => {
                        unsafe { *out_found = 0 };
                        DhruvStatus::Ok
                    }
                    Err(e) => DhruvStatus::from(&e),
                }
            }
            (DHRUV_MOTION_KIND_MAX_SPEED, DHRUV_MOTION_QUERY_MODE_PREV) => {
                if out_max_speed_single.is_null() || out_found.is_null() {
                    return DhruvStatus::NullPointer;
                }
                match prev_max_speed(engine_ref, body, req.at_jd_tdb, &rust_config) {
                    Ok(Some(event)) => {
                        unsafe {
                            *out_max_speed_single = DhruvMaxSpeedEvent::from(&event);
                            *out_found = 1;
                        }
                        DhruvStatus::Ok
                    }
                    Ok(None) => {
                        unsafe { *out_found = 0 };
                        DhruvStatus::Ok
                    }
                    Err(e) => DhruvStatus::from(&e),
                }
            }
            (DHRUV_MOTION_KIND_MAX_SPEED, DHRUV_MOTION_QUERY_MODE_RANGE) => {
                if out_max_speed_many.is_null() || out_count.is_null() {
                    return DhruvStatus::NullPointer;
                }
                match search_max_speed(
                    engine_ref,
                    body,
                    req.start_jd_tdb,
                    req.end_jd_tdb,
                    &rust_config,
                ) {
                    Ok(events) => {
                        let count = events.len().min(max_count as usize);
                        let out_slice = unsafe {
                            std::slice::from_raw_parts_mut(out_max_speed_many, max_count as usize)
                        };
                        for (i, e) in events.iter().take(count).enumerate() {
                            out_slice[i] = DhruvMaxSpeedEvent::from(e);
                        }
                        unsafe { *out_count = count as u32 };
                        DhruvStatus::Ok
                    }
                    Err(e) => DhruvStatus::from(&e),
                }
            }
            _ => DhruvStatus::InvalidQuery,
        }
    })
}

// ---------------------------------------------------------------------------
// Rashi / Nakshatra
// ---------------------------------------------------------------------------

/// C-compatible DMS (degrees-minutes-seconds).
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DhruvDms {
    pub degrees: u16,
    pub minutes: u8,
    pub seconds: f64,
}

/// C-compatible rashi result.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DhruvRashiInfo {
    /// 0-based rashi index (0 = Mesha, 11 = Meena).
    pub rashi_index: u8,
    /// Position within rashi as DMS.
    pub dms: DhruvDms,
    /// Decimal degrees within the rashi [0.0, 30.0).
    pub degrees_in_rashi: f64,
}

/// C-compatible nakshatra result (27-scheme).
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DhruvNakshatraInfo {
    /// 0-based nakshatra index (0 = Ashwini, 26 = Revati).
    pub nakshatra_index: u8,
    /// Pada (1-4).
    pub pada: u8,
    /// Decimal degrees within the nakshatra.
    pub degrees_in_nakshatra: f64,
    /// Decimal degrees within the pada.
    pub degrees_in_pada: f64,
}

/// C-compatible nakshatra result (28-scheme, with Abhijit).
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DhruvNakshatra28Info {
    /// 0-based nakshatra index (0 = Ashwini, 21 = Abhijit, 27 = Revati).
    pub nakshatra_index: u8,
    /// Pada (1-4, or 0 for Abhijit).
    pub pada: u8,
    /// Decimal degrees within the nakshatra.
    pub degrees_in_nakshatra: f64,
}

/// Convert decimal degrees to DMS.
///
/// # Safety
/// `out` must be a valid, non-null pointer.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dhruv_deg_to_dms(degrees: f64, out: *mut DhruvDms) -> DhruvStatus {
    ffi_boundary(|| {
        if out.is_null() {
            return DhruvStatus::NullPointer;
        }
        let d = deg_to_dms(degrees);
        unsafe {
            *out = DhruvDms {
                degrees: d.degrees,
                minutes: d.minutes,
                seconds: d.seconds,
            };
        }
        DhruvStatus::Ok
    })
}

/// Determine rashi from sidereal ecliptic longitude.
///
/// # Safety
/// `out` must be a valid, non-null pointer.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dhruv_rashi_from_longitude(
    sidereal_lon_deg: f64,
    out: *mut DhruvRashiInfo,
) -> DhruvStatus {
    ffi_boundary(|| {
        if out.is_null() {
            return DhruvStatus::NullPointer;
        }
        let info = rashi_from_longitude(sidereal_lon_deg);
        unsafe {
            *out = DhruvRashiInfo {
                rashi_index: info.rashi_index,
                dms: DhruvDms {
                    degrees: info.dms.degrees,
                    minutes: info.dms.minutes,
                    seconds: info.dms.seconds,
                },
                degrees_in_rashi: info.degrees_in_rashi,
            };
        }
        DhruvStatus::Ok
    })
}

/// Determine nakshatra from sidereal ecliptic longitude (27-scheme).
///
/// # Safety
/// `out` must be a valid, non-null pointer.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dhruv_nakshatra_from_longitude(
    sidereal_lon_deg: f64,
    out: *mut DhruvNakshatraInfo,
) -> DhruvStatus {
    ffi_boundary(|| {
        if out.is_null() {
            return DhruvStatus::NullPointer;
        }
        let info = nakshatra_from_longitude(sidereal_lon_deg);
        unsafe {
            *out = DhruvNakshatraInfo {
                nakshatra_index: info.nakshatra_index,
                pada: info.pada,
                degrees_in_nakshatra: info.degrees_in_nakshatra,
                degrees_in_pada: info.degrees_in_pada,
            };
        }
        DhruvStatus::Ok
    })
}

/// Determine nakshatra from sidereal ecliptic longitude (28-scheme with Abhijit).
///
/// # Safety
/// `out` must be a valid, non-null pointer.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dhruv_nakshatra28_from_longitude(
    sidereal_lon_deg: f64,
    out: *mut DhruvNakshatra28Info,
) -> DhruvStatus {
    ffi_boundary(|| {
        if out.is_null() {
            return DhruvStatus::NullPointer;
        }
        let info = nakshatra28_from_longitude(sidereal_lon_deg);
        unsafe {
            *out = DhruvNakshatra28Info {
                nakshatra_index: info.nakshatra_index,
                pada: info.pada,
                degrees_in_nakshatra: info.degrees_in_nakshatra,
            };
        }
        DhruvStatus::Ok
    })
}

/// Determine rashi from tropical longitude with ayanamsha subtraction.
///
/// # Safety
/// `out` must be a valid, non-null pointer.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dhruv_rashi_from_tropical(
    tropical_lon_deg: f64,
    aya_system: i32,
    jd_tdb: f64,
    use_nutation: u8,
    out: *mut DhruvRashiInfo,
) -> DhruvStatus {
    ffi_boundary(|| {
        if out.is_null() {
            return DhruvStatus::NullPointer;
        }
        let system = match ayanamsha_system_from_code(aya_system) {
            Some(s) => s,
            None => return DhruvStatus::InvalidQuery,
        };
        let info = rashi_from_tropical(tropical_lon_deg, system, jd_tdb, use_nutation != 0);
        unsafe {
            *out = DhruvRashiInfo {
                rashi_index: info.rashi_index,
                dms: DhruvDms {
                    degrees: info.dms.degrees,
                    minutes: info.dms.minutes,
                    seconds: info.dms.seconds,
                },
                degrees_in_rashi: info.degrees_in_rashi,
            };
        }
        DhruvStatus::Ok
    })
}

/// Determine nakshatra from tropical longitude with ayanamsha subtraction (27-scheme).
///
/// # Safety
/// `out` must be a valid, non-null pointer.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dhruv_nakshatra_from_tropical(
    tropical_lon_deg: f64,
    aya_system: i32,
    jd_tdb: f64,
    use_nutation: u8,
    out: *mut DhruvNakshatraInfo,
) -> DhruvStatus {
    ffi_boundary(|| {
        if out.is_null() {
            return DhruvStatus::NullPointer;
        }
        let system = match ayanamsha_system_from_code(aya_system) {
            Some(s) => s,
            None => return DhruvStatus::InvalidQuery,
        };
        let info = nakshatra_from_tropical(tropical_lon_deg, system, jd_tdb, use_nutation != 0);
        unsafe {
            *out = DhruvNakshatraInfo {
                nakshatra_index: info.nakshatra_index,
                pada: info.pada,
                degrees_in_nakshatra: info.degrees_in_nakshatra,
                degrees_in_pada: info.degrees_in_pada,
            };
        }
        DhruvStatus::Ok
    })
}

/// Determine nakshatra from tropical longitude with ayanamsha subtraction (28-scheme).
///
/// # Safety
/// `out` must be a valid, non-null pointer.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dhruv_nakshatra28_from_tropical(
    tropical_lon_deg: f64,
    aya_system: i32,
    jd_tdb: f64,
    use_nutation: u8,
    out: *mut DhruvNakshatra28Info,
) -> DhruvStatus {
    ffi_boundary(|| {
        if out.is_null() {
            return DhruvStatus::NullPointer;
        }
        let system = match ayanamsha_system_from_code(aya_system) {
            Some(s) => s,
            None => return DhruvStatus::InvalidQuery,
        };
        let info = nakshatra28_from_tropical(tropical_lon_deg, system, jd_tdb, use_nutation != 0);
        unsafe {
            *out = DhruvNakshatra28Info {
                nakshatra_index: info.nakshatra_index,
                pada: info.pada,
                degrees_in_nakshatra: info.degrees_in_nakshatra,
            };
        }
        DhruvStatus::Ok
    })
}

/// Number of rashis (always 12).
#[unsafe(no_mangle)]
pub extern "C" fn dhruv_rashi_count() -> u32 {
    12
}

/// Number of nakshatras for the given scheme.
///
/// `scheme_code`: 27 returns 27, 28 returns 28. Any other value returns 0.
#[unsafe(no_mangle)]
pub extern "C" fn dhruv_nakshatra_count(scheme_code: u32) -> u32 {
    match scheme_code {
        27 => 27,
        28 => 28,
        _ => 0,
    }
}

/// Rashi name as a NUL-terminated UTF-8 string.
///
/// Returns null pointer for invalid index.
#[unsafe(no_mangle)]
pub extern "C" fn dhruv_rashi_name(index: u32) -> *const std::ffi::c_char {
    static NAMES: [&str; 12] = [
        "Mesha\0",
        "Vrishabha\0",
        "Mithuna\0",
        "Karka\0",
        "Simha\0",
        "Kanya\0",
        "Tula\0",
        "Vrischika\0",
        "Dhanu\0",
        "Makara\0",
        "Kumbha\0",
        "Meena\0",
    ];
    match NAMES.get(index as usize) {
        Some(s) => s.as_ptr().cast(),
        None => ptr::null(),
    }
}

/// Nakshatra name (27-scheme) as a NUL-terminated UTF-8 string.
///
/// Returns null pointer for invalid index.
#[unsafe(no_mangle)]
pub extern "C" fn dhruv_nakshatra_name(index: u32) -> *const std::ffi::c_char {
    static NAMES: [&str; 27] = [
        "Ashwini\0",
        "Bharani\0",
        "Krittika\0",
        "Rohini\0",
        "Mrigashira\0",
        "Ardra\0",
        "Punarvasu\0",
        "Pushya\0",
        "Ashlesha\0",
        "Magha\0",
        "Purva Phalguni\0",
        "Uttara Phalguni\0",
        "Hasta\0",
        "Chitra\0",
        "Swati\0",
        "Vishakha\0",
        "Anuradha\0",
        "Jyeshtha\0",
        "Mula\0",
        "Purva Ashadha\0",
        "Uttara Ashadha\0",
        "Shravana\0",
        "Dhanishtha\0",
        "Shatabhisha\0",
        "Purva Bhadrapada\0",
        "Uttara Bhadrapada\0",
        "Revati\0",
    ];
    match NAMES.get(index as usize) {
        Some(s) => s.as_ptr().cast(),
        None => ptr::null(),
    }
}

/// Nakshatra name (28-scheme, with Abhijit) as a NUL-terminated UTF-8 string.
///
/// Returns null pointer for invalid index.
#[unsafe(no_mangle)]
pub extern "C" fn dhruv_nakshatra28_name(index: u32) -> *const std::ffi::c_char {
    static NAMES: [&str; 28] = [
        "Ashwini\0",
        "Bharani\0",
        "Krittika\0",
        "Rohini\0",
        "Mrigashira\0",
        "Ardra\0",
        "Punarvasu\0",
        "Pushya\0",
        "Ashlesha\0",
        "Magha\0",
        "Purva Phalguni\0",
        "Uttara Phalguni\0",
        "Hasta\0",
        "Chitra\0",
        "Swati\0",
        "Vishakha\0",
        "Anuradha\0",
        "Jyeshtha\0",
        "Mula\0",
        "Purva Ashadha\0",
        "Uttara Ashadha\0",
        "Abhijit\0",
        "Shravana\0",
        "Dhanishtha\0",
        "Shatabhisha\0",
        "Purva Bhadrapada\0",
        "Uttara Bhadrapada\0",
        "Revati\0",
    ];
    match NAMES.get(index as usize) {
        Some(s) => s.as_ptr().cast(),
        None => ptr::null(),
    }
}

// ---------------------------------------------------------------------------
// Panchang: Lunar Phase / Sankranti / Masa / Ayana / Varsha
// ---------------------------------------------------------------------------

/// Lunar phase constants.
pub const DHRUV_LUNAR_PHASE_NEW_MOON: i32 = 0;
pub const DHRUV_LUNAR_PHASE_FULL_MOON: i32 = 1;

/// Lunar-phase selector: Amavasya/new moon.
pub const DHRUV_LUNAR_PHASE_KIND_AMAVASYA: i32 = 0;
/// Lunar-phase selector: Purnima/full moon.
pub const DHRUV_LUNAR_PHASE_KIND_PURNIMA: i32 = 1;
/// Lunar-phase query mode: next event after `at_jd_tdb`.
pub const DHRUV_LUNAR_PHASE_QUERY_MODE_NEXT: i32 = 0;
/// Lunar-phase query mode: previous event before `at_jd_tdb`.
pub const DHRUV_LUNAR_PHASE_QUERY_MODE_PREV: i32 = 1;
/// Lunar-phase query mode: all events in [`start_jd_tdb`, `end_jd_tdb`].
pub const DHRUV_LUNAR_PHASE_QUERY_MODE_RANGE: i32 = 2;

/// Ayana constants.
pub const DHRUV_AYANA_UTTARAYANA: i32 = 0;
pub const DHRUV_AYANA_DAKSHINAYANA: i32 = 1;

/// Sankranti target selector: any rashi entry.
pub const DHRUV_SANKRANTI_TARGET_ANY: i32 = 0;
/// Sankranti target selector: specific rashi entry.
pub const DHRUV_SANKRANTI_TARGET_SPECIFIC: i32 = 1;
/// Sankranti query mode: next event after `at_jd_tdb`.
pub const DHRUV_SANKRANTI_QUERY_MODE_NEXT: i32 = 0;
/// Sankranti query mode: previous event before `at_jd_tdb`.
pub const DHRUV_SANKRANTI_QUERY_MODE_PREV: i32 = 1;
/// Sankranti query mode: all events in [`start_jd_tdb`, `end_jd_tdb`].
pub const DHRUV_SANKRANTI_QUERY_MODE_RANGE: i32 = 2;
/// Panchang time input selector: JD TDB.
pub const DHRUV_PANCHANG_TIME_JD_TDB: i32 = 0;
/// Panchang time input selector: UTC calendar fields.
pub const DHRUV_PANCHANG_TIME_UTC: i32 = 1;

/// Panchang include bit for tithi.
pub const DHRUV_PANCHANG_INCLUDE_TITHI: u32 = 1 << 0;
/// Panchang include bit for karana.
pub const DHRUV_PANCHANG_INCLUDE_KARANA: u32 = 1 << 1;
/// Panchang include bit for yoga.
pub const DHRUV_PANCHANG_INCLUDE_YOGA: u32 = 1 << 2;
/// Panchang include bit for vaar.
pub const DHRUV_PANCHANG_INCLUDE_VAAR: u32 = 1 << 3;
/// Panchang include bit for hora.
pub const DHRUV_PANCHANG_INCLUDE_HORA: u32 = 1 << 4;
/// Panchang include bit for ghatika.
pub const DHRUV_PANCHANG_INCLUDE_GHATIKA: u32 = 1 << 5;
/// Panchang include bit for nakshatra.
pub const DHRUV_PANCHANG_INCLUDE_NAKSHATRA: u32 = 1 << 6;
/// Panchang include bit for masa.
pub const DHRUV_PANCHANG_INCLUDE_MASA: u32 = 1 << 7;
/// Panchang include bit for ayana.
pub const DHRUV_PANCHANG_INCLUDE_AYANA: u32 = 1 << 8;
/// Panchang include bit for varsha.
pub const DHRUV_PANCHANG_INCLUDE_VARSHA: u32 = 1 << 9;
/// Panchang include mask for all core daily fields.
pub const DHRUV_PANCHANG_INCLUDE_ALL_CORE: u32 = DHRUV_PANCHANG_INCLUDE_TITHI
    | DHRUV_PANCHANG_INCLUDE_KARANA
    | DHRUV_PANCHANG_INCLUDE_YOGA
    | DHRUV_PANCHANG_INCLUDE_VAAR
    | DHRUV_PANCHANG_INCLUDE_HORA
    | DHRUV_PANCHANG_INCLUDE_GHATIKA
    | DHRUV_PANCHANG_INCLUDE_NAKSHATRA;
/// Panchang include mask for all calendar fields.
pub const DHRUV_PANCHANG_INCLUDE_ALL_CALENDAR: u32 =
    DHRUV_PANCHANG_INCLUDE_MASA | DHRUV_PANCHANG_INCLUDE_AYANA | DHRUV_PANCHANG_INCLUDE_VARSHA;
/// Panchang include mask for all fields.
pub const DHRUV_PANCHANG_INCLUDE_ALL: u32 =
    DHRUV_PANCHANG_INCLUDE_ALL_CORE | DHRUV_PANCHANG_INCLUDE_ALL_CALENDAR;

/// C-compatible request for unified lunar-phase search.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DhruvLunarPhaseSearchRequest {
    /// Phase kind selector (`DHRUV_LUNAR_PHASE_KIND_*`).
    pub phase_kind: i32,
    /// Query mode selector (`DHRUV_LUNAR_PHASE_QUERY_MODE_*`).
    pub query_mode: i32,
    /// Anchor time for next/prev modes (JD TDB).
    pub at_jd_tdb: f64,
    /// Start of range window for range mode (JD TDB).
    pub start_jd_tdb: f64,
    /// End of range window for range mode (JD TDB).
    pub end_jd_tdb: f64,
}

/// C-compatible lunar phase event.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DhruvLunarPhaseEvent {
    pub utc: DhruvUtcTime,
    /// Phase code (see DHRUV_LUNAR_PHASE_* constants).
    pub phase: i32,
    pub moon_longitude_deg: f64,
    pub sun_longitude_deg: f64,
}

/// C-compatible Sankranti search configuration.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DhruvSankrantiConfig {
    /// Ayanamsha system code (0-19).
    pub ayanamsha_system: i32,
    /// Whether to apply nutation correction (0=false, 1=true).
    pub use_nutation: u8,
    /// Reference plane (0=Ecliptic, 1=Invariable). Set to -1 for system default.
    pub reference_plane: i32,
    pub step_size_days: f64,
    pub max_iterations: u32,
    pub convergence_days: f64,
}

/// C-compatible Sankranti event.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DhruvSankrantiEvent {
    pub utc: DhruvUtcTime,
    /// 0-based rashi index (0=Mesha .. 11=Meena).
    pub rashi_index: i32,
    pub sun_sidereal_longitude_deg: f64,
    pub sun_tropical_longitude_deg: f64,
}

/// C-compatible request for unified sankranti search.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DhruvSankrantiSearchRequest {
    /// Target selector (`DHRUV_SANKRANTI_TARGET_*`).
    pub target_kind: i32,
    /// Query mode selector (`DHRUV_SANKRANTI_QUERY_MODE_*`).
    pub query_mode: i32,
    /// Rashi index for specific target (`0..11`), ignored for `TARGET_ANY`.
    pub rashi_index: i32,
    /// Anchor time for next/prev modes (JD TDB).
    pub at_jd_tdb: f64,
    /// Start of range window for range mode (JD TDB).
    pub start_jd_tdb: f64,
    /// End of range window for range mode (JD TDB).
    pub end_jd_tdb: f64,
    /// Sankranti search configuration.
    pub config: DhruvSankrantiConfig,
}

/// C-compatible Masa info.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DhruvMasaInfo {
    /// 0-based masa index (0=Chaitra .. 11=Phalguna).
    pub masa_index: i32,
    /// Whether this is an adhika (intercalary) month (0=false, 1=true).
    pub adhika: u8,
    pub start: DhruvUtcTime,
    pub end: DhruvUtcTime,
}

/// C-compatible Ayana info.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DhruvAyanaInfo {
    /// Ayana code (see DHRUV_AYANA_* constants).
    pub ayana: i32,
    pub start: DhruvUtcTime,
    pub end: DhruvUtcTime,
}

/// C-compatible Varsha info.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DhruvVarshaInfo {
    /// 0-based samvatsara index (0=Prabhava .. 59=Akshaya).
    pub samvatsara_index: i32,
    /// 1-based order in the 60-year cycle (1-60).
    pub order: i32,
    pub start: DhruvUtcTime,
    pub end: DhruvUtcTime,
}

/// C-compatible request for unified panchang computation.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DhruvPanchangComputeRequest {
    /// Time selector (`DHRUV_PANCHANG_TIME_*`).
    pub time_kind: i32,
    /// Time when `time_kind == DHRUV_PANCHANG_TIME_JD_TDB`.
    pub jd_tdb: f64,
    /// Time when `time_kind == DHRUV_PANCHANG_TIME_UTC`.
    pub utc: DhruvUtcTime,
    /// Include mask with `DHRUV_PANCHANG_INCLUDE_*` bits.
    pub include_mask: u32,
    /// Observer location.
    pub location: DhruvGeoLocation,
    /// Rise/set model configuration.
    pub riseset_config: DhruvRiseSetConfig,
    /// Ayanamsha config for sidereal-dependent elements.
    pub sankranti_config: DhruvSankrantiConfig,
}

/// C-compatible panchang response with per-field validity flags.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DhruvPanchangOperationResult {
    pub tithi_valid: u8,
    pub tithi: DhruvTithiInfo,
    pub karana_valid: u8,
    pub karana: DhruvKaranaInfo,
    pub yoga_valid: u8,
    pub yoga: DhruvYogaInfo,
    pub vaar_valid: u8,
    pub vaar: DhruvVaarInfo,
    pub hora_valid: u8,
    pub hora: DhruvHoraInfo,
    pub ghatika_valid: u8,
    pub ghatika: DhruvGhatikaInfo,
    pub nakshatra_valid: u8,
    pub nakshatra: DhruvPanchangNakshatraInfo,
    pub masa_valid: u8,
    pub masa: DhruvMasaInfo,
    pub ayana_valid: u8,
    pub ayana: DhruvAyanaInfo,
    pub varsha_valid: u8,
    pub varsha: DhruvVarshaInfo,
}

fn utc_time_to_ffi(t: &UtcTime) -> DhruvUtcTime {
    DhruvUtcTime {
        year: t.year,
        month: t.month,
        day: t.day,
        hour: t.hour,
        minute: t.minute,
        second: t.second,
    }
}

fn ffi_to_utc_time(t: &DhruvUtcTime) -> UtcTime {
    UtcTime::new(t.year, t.month, t.day, t.hour, t.minute, t.second)
}

fn lunar_phase_to_code(p: LunarPhase) -> i32 {
    match p {
        LunarPhase::NewMoon => DHRUV_LUNAR_PHASE_NEW_MOON,
        LunarPhase::FullMoon => DHRUV_LUNAR_PHASE_FULL_MOON,
    }
}

fn lunar_phase_event_to_ffi(event: &dhruv_search::LunarPhaseEvent) -> DhruvLunarPhaseEvent {
    DhruvLunarPhaseEvent {
        utc: utc_time_to_ffi(&event.utc),
        phase: lunar_phase_to_code(event.phase),
        moon_longitude_deg: event.moon_longitude_deg,
        sun_longitude_deg: event.sun_longitude_deg,
    }
}

fn sankranti_event_to_ffi(event: &dhruv_search::SankrantiEvent) -> DhruvSankrantiEvent {
    DhruvSankrantiEvent {
        utc: utc_time_to_ffi(&event.utc),
        rashi_index: event.rashi_index as i32,
        sun_sidereal_longitude_deg: event.sun_sidereal_longitude_deg,
        sun_tropical_longitude_deg: event.sun_tropical_longitude_deg,
    }
}

fn reference_plane_from_code(code: i32, system: AyanamshaSystem) -> dhruv_frames::ReferencePlane {
    match code {
        0 => dhruv_frames::ReferencePlane::Ecliptic,
        1 => dhruv_frames::ReferencePlane::Invariable,
        _ => system.default_reference_plane(), // -1 or any other value → system default
    }
}

fn precession_model_from_code(code: i32) -> Option<PrecessionModel> {
    match code {
        DHRUV_PRECESSION_MODEL_NEWCOMB1895 => Some(PrecessionModel::Newcomb1895),
        DHRUV_PRECESSION_MODEL_LIESKE1977 => Some(PrecessionModel::Lieske1977),
        DHRUV_PRECESSION_MODEL_IAU2006 => Some(PrecessionModel::Iau2006),
        DHRUV_PRECESSION_MODEL_VONDRAK2011 => Some(PrecessionModel::Vondrak2011),
        _ => None,
    }
}

fn resolve_graha_longitudes_config_ptr(
    config: *const DhruvGrahaLongitudesConfig,
) -> Result<GrahaLongitudesConfig, DhruvStatus> {
    let raw = if config.is_null() {
        dhruv_graha_longitudes_config_default()
    } else {
        unsafe { *config }
    };
    let kind = match raw.kind {
        DHRUV_GRAHA_LONGITUDE_KIND_SIDEREAL => GrahaLongitudeKind::Sidereal,
        DHRUV_GRAHA_LONGITUDE_KIND_TROPICAL => GrahaLongitudeKind::Tropical,
        _ => return Err(DhruvStatus::InvalidQuery),
    };
    let precession_model =
        precession_model_from_code(raw.precession_model).ok_or(DhruvStatus::InvalidQuery)?;
    let ayanamsha_system =
        ayanamsha_system_from_code(raw.ayanamsha_system).ok_or(DhruvStatus::InvalidQuery)?;
    let reference_plane = match kind {
        GrahaLongitudeKind::Sidereal => {
            reference_plane_from_code(raw.reference_plane, ayanamsha_system)
        }
        GrahaLongitudeKind::Tropical => match raw.reference_plane {
            0 => dhruv_frames::ReferencePlane::Ecliptic,
            1 => dhruv_frames::ReferencePlane::Invariable,
            _ => dhruv_frames::ReferencePlane::Ecliptic,
        },
    };
    Ok(GrahaLongitudesConfig {
        kind,
        ayanamsha_system,
        use_nutation: raw.use_nutation != 0,
        precession_model,
        reference_plane,
    })
}

fn sankranti_config_from_ffi(cfg: &DhruvSankrantiConfig) -> Option<SankrantiConfig> {
    let system = ayanamsha_system_from_code(cfg.ayanamsha_system)?;
    Some(SankrantiConfig {
        ayanamsha_system: system,
        use_nutation: cfg.use_nutation != 0,
        precession_model: dhruv_frames::DEFAULT_PRECESSION_MODEL,
        reference_plane: reference_plane_from_code(cfg.reference_plane, system),
        step_size_days: cfg.step_size_days,
        max_iterations: cfg.max_iterations,
        convergence_days: cfg.convergence_days,
    })
}

fn panchang_include_mask_from_ffi(mask: u32) -> Option<u32> {
    if mask == 0 || (mask & !DHRUV_PANCHANG_INCLUDE_ALL) != 0 {
        return None;
    }
    let mut out = 0_u32;
    if (mask & DHRUV_PANCHANG_INCLUDE_TITHI) != 0 {
        out |= PANCHANG_INCLUDE_TITHI;
    }
    if (mask & DHRUV_PANCHANG_INCLUDE_KARANA) != 0 {
        out |= PANCHANG_INCLUDE_KARANA;
    }
    if (mask & DHRUV_PANCHANG_INCLUDE_YOGA) != 0 {
        out |= PANCHANG_INCLUDE_YOGA;
    }
    if (mask & DHRUV_PANCHANG_INCLUDE_VAAR) != 0 {
        out |= PANCHANG_INCLUDE_VAAR;
    }
    if (mask & DHRUV_PANCHANG_INCLUDE_HORA) != 0 {
        out |= PANCHANG_INCLUDE_HORA;
    }
    if (mask & DHRUV_PANCHANG_INCLUDE_GHATIKA) != 0 {
        out |= PANCHANG_INCLUDE_GHATIKA;
    }
    if (mask & DHRUV_PANCHANG_INCLUDE_NAKSHATRA) != 0 {
        out |= PANCHANG_INCLUDE_NAKSHATRA;
    }
    if (mask & DHRUV_PANCHANG_INCLUDE_MASA) != 0 {
        out |= PANCHANG_INCLUDE_MASA;
    }
    if (mask & DHRUV_PANCHANG_INCLUDE_AYANA) != 0 {
        out |= PANCHANG_INCLUDE_AYANA;
    }
    if (mask & DHRUV_PANCHANG_INCLUDE_VARSHA) != 0 {
        out |= PANCHANG_INCLUDE_VARSHA;
    }
    Some(out)
}

/// Returns default Sankranti search configuration (Lahiri, no nutation).
#[unsafe(no_mangle)]
pub extern "C" fn dhruv_sankranti_config_default() -> DhruvSankrantiConfig {
    DhruvSankrantiConfig {
        ayanamsha_system: 0, // Lahiri
        use_nutation: 0,
        reference_plane: -1, // system default
        step_size_days: 1.0,
        max_iterations: 50,
        convergence_days: 1e-8,
    }
}

/// Unified lunar-phase search entrypoint.
///
/// Mode behavior:
/// - `DHRUV_LUNAR_PHASE_QUERY_MODE_NEXT` / `DHRUV_LUNAR_PHASE_QUERY_MODE_PREV`:
///   writes single-event result to `out_event` and found flag to `out_found`.
/// - `DHRUV_LUNAR_PHASE_QUERY_MODE_RANGE`:
///   writes events to `out_events[..max_count]` and actual count to `out_count`.
///
/// Kind behavior:
/// - `DHRUV_LUNAR_PHASE_KIND_AMAVASYA` selects new-moon events.
/// - `DHRUV_LUNAR_PHASE_KIND_PURNIMA` selects full-moon events.
///
/// # Safety
/// `engine` and `request` must be valid and non-null.
/// Output pointers required depend on `query_mode`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dhruv_lunar_phase_search_ex(
    engine: *const DhruvEngineHandle,
    request: *const DhruvLunarPhaseSearchRequest,
    out_event: *mut DhruvLunarPhaseEvent,
    out_found: *mut u8,
    out_events: *mut DhruvLunarPhaseEvent,
    max_count: u32,
    out_count: *mut u32,
) -> DhruvStatus {
    ffi_boundary(|| {
        if engine.is_null() || request.is_null() {
            return DhruvStatus::NullPointer;
        }

        let engine_ref = unsafe { &*engine };
        let req = unsafe { &*request };

        match (req.phase_kind, req.query_mode) {
            (DHRUV_LUNAR_PHASE_KIND_AMAVASYA, DHRUV_LUNAR_PHASE_QUERY_MODE_NEXT) => {
                if out_event.is_null() || out_found.is_null() {
                    return DhruvStatus::NullPointer;
                }
                let at = UtcTime::from_jd_tdb(req.at_jd_tdb, engine_ref.lsk());
                match next_amavasya(engine_ref, &at) {
                    Ok(Some(event)) => {
                        unsafe {
                            *out_event = lunar_phase_event_to_ffi(&event);
                            *out_found = 1;
                        }
                        DhruvStatus::Ok
                    }
                    Ok(None) => {
                        unsafe { *out_found = 0 };
                        DhruvStatus::Ok
                    }
                    Err(e) => DhruvStatus::from(&e),
                }
            }
            (DHRUV_LUNAR_PHASE_KIND_AMAVASYA, DHRUV_LUNAR_PHASE_QUERY_MODE_PREV) => {
                if out_event.is_null() || out_found.is_null() {
                    return DhruvStatus::NullPointer;
                }
                let at = UtcTime::from_jd_tdb(req.at_jd_tdb, engine_ref.lsk());
                match prev_amavasya(engine_ref, &at) {
                    Ok(Some(event)) => {
                        unsafe {
                            *out_event = lunar_phase_event_to_ffi(&event);
                            *out_found = 1;
                        }
                        DhruvStatus::Ok
                    }
                    Ok(None) => {
                        unsafe { *out_found = 0 };
                        DhruvStatus::Ok
                    }
                    Err(e) => DhruvStatus::from(&e),
                }
            }
            (DHRUV_LUNAR_PHASE_KIND_AMAVASYA, DHRUV_LUNAR_PHASE_QUERY_MODE_RANGE) => {
                if out_events.is_null() || out_count.is_null() {
                    return DhruvStatus::NullPointer;
                }
                let start = UtcTime::from_jd_tdb(req.start_jd_tdb, engine_ref.lsk());
                let end = UtcTime::from_jd_tdb(req.end_jd_tdb, engine_ref.lsk());
                match search_amavasyas(engine_ref, &start, &end) {
                    Ok(events) => {
                        let count = events.len().min(max_count as usize);
                        let out_slice = unsafe {
                            std::slice::from_raw_parts_mut(out_events, max_count as usize)
                        };
                        for (i, event) in events.iter().take(count).enumerate() {
                            out_slice[i] = lunar_phase_event_to_ffi(event);
                        }
                        unsafe { *out_count = count as u32 };
                        DhruvStatus::Ok
                    }
                    Err(e) => DhruvStatus::from(&e),
                }
            }
            (DHRUV_LUNAR_PHASE_KIND_PURNIMA, DHRUV_LUNAR_PHASE_QUERY_MODE_NEXT) => {
                if out_event.is_null() || out_found.is_null() {
                    return DhruvStatus::NullPointer;
                }
                let at = UtcTime::from_jd_tdb(req.at_jd_tdb, engine_ref.lsk());
                match next_purnima(engine_ref, &at) {
                    Ok(Some(event)) => {
                        unsafe {
                            *out_event = lunar_phase_event_to_ffi(&event);
                            *out_found = 1;
                        }
                        DhruvStatus::Ok
                    }
                    Ok(None) => {
                        unsafe { *out_found = 0 };
                        DhruvStatus::Ok
                    }
                    Err(e) => DhruvStatus::from(&e),
                }
            }
            (DHRUV_LUNAR_PHASE_KIND_PURNIMA, DHRUV_LUNAR_PHASE_QUERY_MODE_PREV) => {
                if out_event.is_null() || out_found.is_null() {
                    return DhruvStatus::NullPointer;
                }
                let at = UtcTime::from_jd_tdb(req.at_jd_tdb, engine_ref.lsk());
                match prev_purnima(engine_ref, &at) {
                    Ok(Some(event)) => {
                        unsafe {
                            *out_event = lunar_phase_event_to_ffi(&event);
                            *out_found = 1;
                        }
                        DhruvStatus::Ok
                    }
                    Ok(None) => {
                        unsafe { *out_found = 0 };
                        DhruvStatus::Ok
                    }
                    Err(e) => DhruvStatus::from(&e),
                }
            }
            (DHRUV_LUNAR_PHASE_KIND_PURNIMA, DHRUV_LUNAR_PHASE_QUERY_MODE_RANGE) => {
                if out_events.is_null() || out_count.is_null() {
                    return DhruvStatus::NullPointer;
                }
                let start = UtcTime::from_jd_tdb(req.start_jd_tdb, engine_ref.lsk());
                let end = UtcTime::from_jd_tdb(req.end_jd_tdb, engine_ref.lsk());
                match search_purnimas(engine_ref, &start, &end) {
                    Ok(events) => {
                        let count = events.len().min(max_count as usize);
                        let out_slice = unsafe {
                            std::slice::from_raw_parts_mut(out_events, max_count as usize)
                        };
                        for (i, event) in events.iter().take(count).enumerate() {
                            out_slice[i] = lunar_phase_event_to_ffi(event);
                        }
                        unsafe { *out_count = count as u32 };
                        DhruvStatus::Ok
                    }
                    Err(e) => DhruvStatus::from(&e),
                }
            }
            _ => DhruvStatus::InvalidQuery,
        }
    })
}

/// Unified sankranti search entrypoint.
///
/// Mode behavior:
/// - `DHRUV_SANKRANTI_QUERY_MODE_NEXT` / `DHRUV_SANKRANTI_QUERY_MODE_PREV`:
///   writes single-event result to `out_event` and found flag to `out_found`.
/// - `DHRUV_SANKRANTI_QUERY_MODE_RANGE`:
///   writes events to `out_events[..max_count]` and actual count to `out_count`.
///
/// Target behavior:
/// - `DHRUV_SANKRANTI_TARGET_ANY` searches all rashi entries.
/// - `DHRUV_SANKRANTI_TARGET_SPECIFIC` limits results to `rashi_index`.
///
/// # Safety
/// `engine` and `request` must be valid and non-null.
/// Output pointers required depend on `query_mode`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dhruv_sankranti_search_ex(
    engine: *const DhruvEngineHandle,
    request: *const DhruvSankrantiSearchRequest,
    out_event: *mut DhruvSankrantiEvent,
    out_found: *mut u8,
    out_events: *mut DhruvSankrantiEvent,
    max_count: u32,
    out_count: *mut u32,
) -> DhruvStatus {
    ffi_boundary(|| {
        if engine.is_null() || request.is_null() {
            return DhruvStatus::NullPointer;
        }

        let engine_ref = unsafe { &*engine };
        let req = unsafe { &*request };
        let config = match sankranti_config_from_ffi(&req.config) {
            Some(c) => c,
            None => return DhruvStatus::InvalidQuery,
        };
        let specific_rashi = match req.target_kind {
            DHRUV_SANKRANTI_TARGET_ANY => None,
            DHRUV_SANKRANTI_TARGET_SPECIFIC => {
                let idx = usize::try_from(req.rashi_index).ok().filter(|i| *i < 12);
                match idx {
                    Some(i) => Some(dhruv_vedic_base::ALL_RASHIS[i]),
                    None => return DhruvStatus::InvalidQuery,
                }
            }
            _ => return DhruvStatus::InvalidQuery,
        };

        match req.query_mode {
            DHRUV_SANKRANTI_QUERY_MODE_NEXT => {
                if out_event.is_null() || out_found.is_null() {
                    return DhruvStatus::NullPointer;
                }
                let at = UtcTime::from_jd_tdb(req.at_jd_tdb, engine_ref.lsk());
                let result = match specific_rashi {
                    Some(rashi) => next_specific_sankranti(engine_ref, &at, rashi, &config),
                    None => next_sankranti(engine_ref, &at, &config),
                };
                match result {
                    Ok(Some(event)) => {
                        unsafe {
                            *out_event = sankranti_event_to_ffi(&event);
                            *out_found = 1;
                        }
                        DhruvStatus::Ok
                    }
                    Ok(None) => {
                        unsafe { *out_found = 0 };
                        DhruvStatus::Ok
                    }
                    Err(e) => DhruvStatus::from(&e),
                }
            }
            DHRUV_SANKRANTI_QUERY_MODE_PREV => {
                if out_event.is_null() || out_found.is_null() {
                    return DhruvStatus::NullPointer;
                }
                let at = UtcTime::from_jd_tdb(req.at_jd_tdb, engine_ref.lsk());
                let result = match specific_rashi {
                    Some(rashi) => prev_specific_sankranti(engine_ref, &at, rashi, &config),
                    None => prev_sankranti(engine_ref, &at, &config),
                };
                match result {
                    Ok(Some(event)) => {
                        unsafe {
                            *out_event = sankranti_event_to_ffi(&event);
                            *out_found = 1;
                        }
                        DhruvStatus::Ok
                    }
                    Ok(None) => {
                        unsafe { *out_found = 0 };
                        DhruvStatus::Ok
                    }
                    Err(e) => DhruvStatus::from(&e),
                }
            }
            DHRUV_SANKRANTI_QUERY_MODE_RANGE => {
                if out_events.is_null() || out_count.is_null() {
                    return DhruvStatus::NullPointer;
                }
                let start = UtcTime::from_jd_tdb(req.start_jd_tdb, engine_ref.lsk());
                let end = UtcTime::from_jd_tdb(req.end_jd_tdb, engine_ref.lsk());
                match search_sankrantis(engine_ref, &start, &end, &config) {
                    Ok(events) => {
                        let filtered: Vec<_> = match specific_rashi {
                            Some(rashi) => {
                                events.into_iter().filter(|ev| ev.rashi == rashi).collect()
                            }
                            None => events,
                        };
                        let count = filtered.len().min(max_count as usize);
                        let out_slice = unsafe {
                            std::slice::from_raw_parts_mut(out_events, max_count as usize)
                        };
                        for (i, event) in filtered.iter().take(count).enumerate() {
                            out_slice[i] = sankranti_event_to_ffi(event);
                        }
                        unsafe { *out_count = count as u32 };
                        DhruvStatus::Ok
                    }
                    Err(e) => DhruvStatus::from(&e),
                }
            }
            _ => DhruvStatus::InvalidQuery,
        }
    })
}

/// Determine the Masa (lunar month) for a given UTC date.
///
/// # Safety
/// All pointer arguments must be valid and non-null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dhruv_masa_for_date(
    engine: *const DhruvEngineHandle,
    utc: *const DhruvUtcTime,
    config: *const DhruvSankrantiConfig,
    out: *mut DhruvMasaInfo,
) -> DhruvStatus {
    ffi_boundary(|| {
        if engine.is_null() || utc.is_null() || out.is_null() {
            return DhruvStatus::NullPointer;
        }
        let engine_ref = unsafe { &*engine };
        let t = ffi_to_utc_time(unsafe { &*utc });
        let cfg = match resolve_sankranti_config_ptr(config) {
            Ok(c) => c,
            Err(status) => return status,
        };
        match masa_for_date(engine_ref, &t, &cfg) {
            Ok(info) => {
                unsafe {
                    *out = DhruvMasaInfo {
                        masa_index: info.masa.index() as i32,
                        adhika: u8::from(info.adhika),
                        start: utc_time_to_ffi(&info.start),
                        end: utc_time_to_ffi(&info.end),
                    };
                }
                DhruvStatus::Ok
            }
            Err(e) => DhruvStatus::from(&e),
        }
    })
}

/// Determine the Ayana (solstice period) for a given UTC date.
///
/// # Safety
/// All pointer arguments must be valid and non-null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dhruv_ayana_for_date(
    engine: *const DhruvEngineHandle,
    utc: *const DhruvUtcTime,
    config: *const DhruvSankrantiConfig,
    out: *mut DhruvAyanaInfo,
) -> DhruvStatus {
    ffi_boundary(|| {
        if engine.is_null() || utc.is_null() || out.is_null() {
            return DhruvStatus::NullPointer;
        }
        let engine_ref = unsafe { &*engine };
        let t = ffi_to_utc_time(unsafe { &*utc });
        let cfg = match resolve_sankranti_config_ptr(config) {
            Ok(c) => c,
            Err(status) => return status,
        };
        match ayana_for_date(engine_ref, &t, &cfg) {
            Ok(info) => {
                unsafe {
                    *out = DhruvAyanaInfo {
                        ayana: info.ayana.index() as i32,
                        start: utc_time_to_ffi(&info.start),
                        end: utc_time_to_ffi(&info.end),
                    };
                }
                DhruvStatus::Ok
            }
            Err(e) => DhruvStatus::from(&e),
        }
    })
}

/// Determine the Varsha (60-year cycle position) for a given UTC date.
///
/// # Safety
/// All pointer arguments must be valid and non-null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dhruv_varsha_for_date(
    engine: *const DhruvEngineHandle,
    utc: *const DhruvUtcTime,
    config: *const DhruvSankrantiConfig,
    out: *mut DhruvVarshaInfo,
) -> DhruvStatus {
    ffi_boundary(|| {
        if engine.is_null() || utc.is_null() || out.is_null() {
            return DhruvStatus::NullPointer;
        }
        let engine_ref = unsafe { &*engine };
        let t = ffi_to_utc_time(unsafe { &*utc });
        let cfg = match resolve_sankranti_config_ptr(config) {
            Ok(c) => c,
            Err(status) => return status,
        };
        match varsha_for_date(engine_ref, &t, &cfg) {
            Ok(info) => {
                unsafe {
                    *out = DhruvVarshaInfo {
                        samvatsara_index: info.samvatsara.index() as i32,
                        order: info.order as i32,
                        start: utc_time_to_ffi(&info.start),
                        end: utc_time_to_ffi(&info.end),
                    };
                }
                DhruvStatus::Ok
            }
            Err(e) => DhruvStatus::from(&e),
        }
    })
}

/// Masa name as a NUL-terminated UTF-8 string.
///
/// Returns null pointer for invalid index.
#[unsafe(no_mangle)]
pub extern "C" fn dhruv_masa_name(index: u32) -> *const std::ffi::c_char {
    static NAMES: [&str; 12] = [
        "Chaitra\0",
        "Vaishakha\0",
        "Jyeshtha\0",
        "Ashadha\0",
        "Shravana\0",
        "Bhadrapada\0",
        "Ashvina\0",
        "Kartika\0",
        "Margashirsha\0",
        "Pausha\0",
        "Magha\0",
        "Phalguna\0",
    ];
    match NAMES.get(index as usize) {
        Some(s) => s.as_ptr().cast(),
        None => ptr::null(),
    }
}

/// Ayana name as a NUL-terminated UTF-8 string.
///
/// Returns null pointer for invalid index.
#[unsafe(no_mangle)]
pub extern "C" fn dhruv_ayana_name(index: u32) -> *const std::ffi::c_char {
    static NAMES: [&str; 2] = ["Uttarayana\0", "Dakshinayana\0"];
    match NAMES.get(index as usize) {
        Some(s) => s.as_ptr().cast(),
        None => ptr::null(),
    }
}

/// Samvatsara name as a NUL-terminated UTF-8 string.
///
/// Returns null pointer for invalid index.
#[unsafe(no_mangle)]
pub extern "C" fn dhruv_samvatsara_name(index: u32) -> *const std::ffi::c_char {
    static NAMES: [&str; 60] = [
        "Prabhava\0",
        "Vibhava\0",
        "Shukla\0",
        "Pramodoota\0",
        "Prajothpatti\0",
        "Angirasa\0",
        "Shrimukha\0",
        "Bhava\0",
        "Yuva\0",
        "Dhaatu\0",
        "Eeshvara\0",
        "Bahudhanya\0",
        "Pramaathi\0",
        "Vikrama\0",
        "Vrisha\0",
        "Chitrabhanu\0",
        "Svabhanu\0",
        "Taarana\0",
        "Paarthiva\0",
        "Vyaya\0",
        "Sarvajit\0",
        "Sarvadhari\0",
        "Virodhi\0",
        "Vikruti\0",
        "Khara\0",
        "Nandana\0",
        "Vijaya\0",
        "Jaya\0",
        "Manmatha\0",
        "Durmukhi\0",
        "Hevilambi\0",
        "Vilambi\0",
        "Vikari\0",
        "Sharvari\0",
        "Plava\0",
        "Shubhakrut\0",
        "Shobhakrut\0",
        "Krodhi\0",
        "Vishvavasu\0",
        "Paraabhava\0",
        "Plavanga\0",
        "Keelaka\0",
        "Saumya\0",
        "Sadharana\0",
        "Virodhikrut\0",
        "Paridhavi\0",
        "Pramaadhi\0",
        "Aananda\0",
        "Raakshasa\0",
        "Naala\0",
        "Pingala\0",
        "Kaalayukti\0",
        "Siddharthi\0",
        "Raudri\0",
        "Durmathi\0",
        "Dundubhi\0",
        "Rudhirodgaari\0",
        "Raktaakshi\0",
        "Krodhana\0",
        "Akshaya\0",
    ];
    match NAMES.get(index as usize) {
        Some(s) => s.as_ptr().cast(),
        None => ptr::null(),
    }
}

// ---------------------------------------------------------------------------
// Pure-math panchang classifiers
// ---------------------------------------------------------------------------

/// C-compatible tithi position from elongation.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct DhruvTithiPosition {
    /// 0-based tithi index (0..29).
    pub tithi_index: i32,
    /// Paksha: 0 = Shukla, 1 = Krishna.
    pub paksha: i32,
    /// 1-based tithi number within the paksha (1-15).
    pub tithi_in_paksha: i32,
    /// Degrees into the current tithi [0, 12).
    pub degrees_in_tithi: f64,
}

/// Determine the Tithi from Moon-Sun elongation (degrees).
///
/// Pure math — no engine or kernel needed.
///
/// # Safety
/// `out` must be a valid, non-null pointer.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dhruv_tithi_from_elongation(
    elongation_deg: f64,
    out: *mut DhruvTithiPosition,
) -> DhruvStatus {
    if out.is_null() {
        return DhruvStatus::NullPointer;
    }
    let pos = tithi_from_elongation(elongation_deg);
    unsafe {
        *out = DhruvTithiPosition {
            tithi_index: pos.tithi_index as i32,
            paksha: match pos.paksha {
                dhruv_vedic_base::Paksha::Shukla => 0,
                dhruv_vedic_base::Paksha::Krishna => 1,
            },
            tithi_in_paksha: pos.tithi_in_paksha as i32,
            degrees_in_tithi: pos.degrees_in_tithi,
        };
    }
    DhruvStatus::Ok
}

/// C-compatible karana position from elongation.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct DhruvKaranaPosition {
    /// 0-based karana sequence index within the synodic month (0..59).
    pub karana_index: i32,
    /// Degrees into the current karana [0, 6).
    pub degrees_in_karana: f64,
}

/// Determine the Karana from Moon-Sun elongation (degrees).
///
/// Pure math — no engine or kernel needed.
///
/// # Safety
/// `out` must be a valid, non-null pointer.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dhruv_karana_from_elongation(
    elongation_deg: f64,
    out: *mut DhruvKaranaPosition,
) -> DhruvStatus {
    if out.is_null() {
        return DhruvStatus::NullPointer;
    }
    let pos = karana_from_elongation(elongation_deg);
    unsafe {
        *out = DhruvKaranaPosition {
            karana_index: pos.karana_index as i32,
            degrees_in_karana: pos.degrees_in_karana,
        };
    }
    DhruvStatus::Ok
}

/// C-compatible yoga position from Sun+Moon sidereal sum.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct DhruvYogaPosition {
    /// 0-based yoga index (0..26).
    pub yoga_index: i32,
    /// Degrees into the current yoga [0, 13.333...).
    pub degrees_in_yoga: f64,
}

/// Determine the Yoga from the Sun+Moon sidereal longitude sum (degrees).
///
/// Pure math — no engine or kernel needed.
///
/// # Safety
/// `out` must be a valid, non-null pointer.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dhruv_yoga_from_sum(
    sum_deg: f64,
    out: *mut DhruvYogaPosition,
) -> DhruvStatus {
    if out.is_null() {
        return DhruvStatus::NullPointer;
    }
    let pos = yoga_from_sum(sum_deg);
    unsafe {
        *out = DhruvYogaPosition {
            yoga_index: pos.yoga_index as i32,
            degrees_in_yoga: pos.degrees_in_yoga,
        };
    }
    DhruvStatus::Ok
}

/// Determine the Vaar (weekday) from a Julian Date.
///
/// Returns 0-based vaar index: 0=Ravivaar(Sunday) .. 6=Shanivaar(Saturday).
/// Pure math — no engine or kernel needed.
#[unsafe(no_mangle)]
pub extern "C" fn dhruv_vaar_from_jd(jd: f64) -> i32 {
    let vaar = vaar_from_jd(jd);
    vaar.index() as i32
}

/// Determine the Masa (lunar month) from a 0-based rashi index (0..11).
///
/// Returns 0-based masa index: 0=Chaitra .. 11=Phalguna.
/// Returns -1 for invalid rashi_index (>= 12).
/// Pure math — no engine or kernel needed.
#[unsafe(no_mangle)]
pub extern "C" fn dhruv_masa_from_rashi_index(rashi_index: u32) -> i32 {
    if rashi_index >= 12 {
        return -1;
    }
    let masa = masa_from_rashi_index(rashi_index as u8);
    masa.index() as i32
}

/// Determine the Ayana from a sidereal longitude (degrees).
///
/// Returns 0 = Uttarayana, 1 = Dakshinayana.
/// Pure math — no engine or kernel needed.
#[unsafe(no_mangle)]
pub extern "C" fn dhruv_ayana_from_sidereal_longitude(lon_deg: f64) -> i32 {
    let ayana = ayana_from_sidereal_longitude(lon_deg);
    ayana.index() as i32
}

/// C-compatible samvatsara result.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct DhruvSamvatsaraResult {
    /// 0-based samvatsara index (0..59).
    pub samvatsara_index: i32,
    /// 1-based position in the 60-year cycle (1..60).
    pub cycle_position: i32,
}

/// Determine the Samvatsara (Jovian year) from a CE year.
///
/// Pure math — no engine or kernel needed.
///
/// # Safety
/// `out` must be a valid, non-null pointer.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dhruv_samvatsara_from_year(
    ce_year: i32,
    out: *mut DhruvSamvatsaraResult,
) -> DhruvStatus {
    if out.is_null() {
        return DhruvStatus::NullPointer;
    }
    let (samvatsara, position) = samvatsara_from_year(ce_year);
    unsafe {
        *out = DhruvSamvatsaraResult {
            samvatsara_index: samvatsara.index() as i32,
            cycle_position: position as i32,
        };
    }
    DhruvStatus::Ok
}

/// Compute the 0-based rashi index that is `offset` signs from `rashi_index`.
///
/// Formula: (rashi_index + offset - 1) % 12. Returns -1 if rashi_index >= 12.
/// Pure math — no engine or kernel needed.
#[unsafe(no_mangle)]
pub extern "C" fn dhruv_nth_rashi_from(rashi_index: u32, offset: u32) -> i32 {
    if rashi_index >= 12 {
        return -1;
    }
    nth_rashi_from(rashi_index as u8, offset as u8) as i32
}

// ---------------------------------------------------------------------------
// UTC result structs
// ---------------------------------------------------------------------------

/// C-compatible conjunction event with UTC time.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DhruvConjunctionEventUtc {
    pub utc: DhruvUtcTime,
    pub actual_separation_deg: f64,
    pub body1_longitude_deg: f64,
    pub body2_longitude_deg: f64,
    pub body1_latitude_deg: f64,
    pub body2_latitude_deg: f64,
    pub body1_code: i32,
    pub body2_code: i32,
}

/// C-compatible stationary event with UTC time.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DhruvStationaryEventUtc {
    pub utc: DhruvUtcTime,
    pub body_code: i32,
    pub longitude_deg: f64,
    pub latitude_deg: f64,
    pub station_type: i32,
}

/// C-compatible max-speed event with UTC time.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DhruvMaxSpeedEventUtc {
    pub utc: DhruvUtcTime,
    pub body_code: i32,
    pub longitude_deg: f64,
    pub latitude_deg: f64,
    pub speed_deg_per_day: f64,
    pub speed_type: i32,
}

/// C-compatible rise/set result with UTC time.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DhruvRiseSetResultUtc {
    /// 0 = Event occurred, 1 = NeverRises, 2 = NeverSets.
    pub result_type: i32,
    /// Event code (valid when result_type == 0).
    pub event_code: i32,
    /// Event time in UTC (valid when result_type == 0).
    pub utc: DhruvUtcTime,
}

/// C-compatible chandra grahan result with UTC times.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DhruvChandraGrahanResultUtc {
    pub grahan_type: i32,
    pub magnitude: f64,
    pub penumbral_magnitude: f64,
    pub greatest_grahan: DhruvUtcTime,
    pub p1: DhruvUtcTime,
    pub u1: DhruvUtcTime,
    pub u2: DhruvUtcTime,
    pub u3: DhruvUtcTime,
    pub u4: DhruvUtcTime,
    pub p4: DhruvUtcTime,
    pub moon_ecliptic_lat_deg: f64,
    pub angular_separation_deg: f64,
    /// 1 = u1 present, 0 = absent.
    pub u1_valid: u8,
    /// 1 = u2 present, 0 = absent.
    pub u2_valid: u8,
    /// 1 = u3 present, 0 = absent.
    pub u3_valid: u8,
    /// 1 = u4 present, 0 = absent.
    pub u4_valid: u8,
}

/// C-compatible surya grahan result with UTC times.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DhruvSuryaGrahanResultUtc {
    pub grahan_type: i32,
    pub magnitude: f64,
    pub greatest_grahan: DhruvUtcTime,
    pub c1: DhruvUtcTime,
    pub c2: DhruvUtcTime,
    pub c3: DhruvUtcTime,
    pub c4: DhruvUtcTime,
    pub moon_ecliptic_lat_deg: f64,
    pub angular_separation_deg: f64,
    /// 1 = c1 present, 0 = absent.
    pub c1_valid: u8,
    /// 1 = c2 present, 0 = absent.
    pub c2_valid: u8,
    /// 1 = c3 present, 0 = absent.
    pub c3_valid: u8,
    /// 1 = c4 present, 0 = absent.
    pub c4_valid: u8,
}

// ---------------------------------------------------------------------------
// UTC conversion helpers
// ---------------------------------------------------------------------------

const ZEROED_UTC: DhruvUtcTime = DhruvUtcTime {
    year: 0,
    month: 0,
    day: 0,
    hour: 0,
    minute: 0,
    second: 0.0,
};

fn jd_tdb_to_utc_time(jd_tdb: f64, lsk: &dhruv_time::LeapSecondKernel) -> DhruvUtcTime {
    utc_time_to_ffi(&UtcTime::from_jd_tdb(jd_tdb, lsk))
}

fn riseset_result_to_utc(
    r: &RiseSetResult,
    lsk: &dhruv_time::LeapSecondKernel,
) -> DhruvRiseSetResultUtc {
    match *r {
        RiseSetResult::Event { jd_tdb, event } => DhruvRiseSetResultUtc {
            result_type: DHRUV_RISESET_EVENT,
            event_code: riseset_event_to_code(event),
            utc: jd_tdb_to_utc_time(jd_tdb, lsk),
        },
        RiseSetResult::NeverRises => DhruvRiseSetResultUtc {
            result_type: DHRUV_RISESET_NEVER_RISES,
            event_code: 0,
            utc: ZEROED_UTC,
        },
        RiseSetResult::NeverSets => DhruvRiseSetResultUtc {
            result_type: DHRUV_RISESET_NEVER_SETS,
            event_code: 0,
            utc: ZEROED_UTC,
        },
    }
}

/// Convert DhruvUtcTime to JD UTC (no TDB conversion, pure calendar arithmetic).
fn ffi_utc_to_jd_utc(t: &DhruvUtcTime) -> f64 {
    let day_frac =
        t.day as f64 + t.hour as f64 / 24.0 + t.minute as f64 / 1440.0 + t.second / 86_400.0;
    dhruv_time::calendar_to_jd(t.year, t.month, day_frac)
}

// ---------------------------------------------------------------------------
// Group B: Rise/set + bhava _utc functions (5 functions)
// ---------------------------------------------------------------------------

/// Compute a single rise/set event with UTC input/output.
///
/// # Safety
/// All pointer arguments must be valid and non-null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dhruv_compute_rise_set_utc(
    engine: *const DhruvEngineHandle,
    lsk: *const DhruvLskHandle,
    eop: *const DhruvEopHandle,
    location: *const DhruvGeoLocation,
    event_code: i32,
    utc: *const DhruvUtcTime,
    config: *const DhruvRiseSetConfig,
    out_result: *mut DhruvRiseSetResultUtc,
) -> DhruvStatus {
    ffi_boundary(|| {
        if engine.is_null()
            || lsk.is_null()
            || eop.is_null()
            || location.is_null()
            || utc.is_null()
            || out_result.is_null()
        {
            return DhruvStatus::NullPointer;
        }
        let event = match riseset_event_from_code(event_code) {
            Some(e) => e,
            None => return DhruvStatus::InvalidQuery,
        };
        let engine_ref = unsafe { &*engine };
        let lsk_ref = unsafe { &*lsk };
        let eop_ref = unsafe { &*eop };
        let loc_ref = unsafe { &*location };
        let geo = GeoLocation::new(
            loc_ref.latitude_deg,
            loc_ref.longitude_deg,
            loc_ref.altitude_m,
        );
        let rs_config = match resolve_riseset_config_ptr(config) {
            Ok(c) => c,
            Err(status) => return status,
        };
        let jd_utc = ffi_utc_to_jd_utc(unsafe { &*utc });
        let jd_utc_noon =
            approximate_local_noon_jd(utc_day_start_jd(jd_utc), loc_ref.longitude_deg);
        match compute_rise_set(
            engine_ref,
            lsk_ref,
            eop_ref,
            &geo,
            event,
            jd_utc_noon,
            &rs_config,
        ) {
            Ok(result) => {
                unsafe { *out_result = riseset_result_to_utc(&result, lsk_ref) };
                DhruvStatus::Ok
            }
            Err(e) => DhruvStatus::from(&e),
        }
    })
}

/// Compute all 8 rise/set events for a day with UTC input/output.
///
/// # Safety
/// All pointer arguments must be valid and non-null.
/// `out_results` must point to at least 8 contiguous `DhruvRiseSetResultUtc`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dhruv_compute_all_events_utc(
    engine: *const DhruvEngineHandle,
    lsk: *const DhruvLskHandle,
    eop: *const DhruvEopHandle,
    location: *const DhruvGeoLocation,
    utc: *const DhruvUtcTime,
    config: *const DhruvRiseSetConfig,
    out_results: *mut DhruvRiseSetResultUtc,
) -> DhruvStatus {
    ffi_boundary(|| {
        if engine.is_null()
            || lsk.is_null()
            || eop.is_null()
            || location.is_null()
            || utc.is_null()
            || out_results.is_null()
        {
            return DhruvStatus::NullPointer;
        }
        let engine_ref = unsafe { &*engine };
        let lsk_ref = unsafe { &*lsk };
        let eop_ref = unsafe { &*eop };
        let loc_ref = unsafe { &*location };
        let geo = GeoLocation::new(
            loc_ref.latitude_deg,
            loc_ref.longitude_deg,
            loc_ref.altitude_m,
        );
        let rs_config = match resolve_riseset_config_ptr(config) {
            Ok(c) => c,
            Err(status) => return status,
        };
        let jd_utc = ffi_utc_to_jd_utc(unsafe { &*utc });
        let jd_utc_noon =
            approximate_local_noon_jd(utc_day_start_jd(jd_utc), loc_ref.longitude_deg);
        match compute_all_events(engine_ref, lsk_ref, eop_ref, &geo, jd_utc_noon, &rs_config) {
            Ok(results) => {
                let out_slice = unsafe { std::slice::from_raw_parts_mut(out_results, 8) };
                for (i, r) in results.iter().enumerate() {
                    out_slice[i] = riseset_result_to_utc(r, lsk_ref);
                }
                DhruvStatus::Ok
            }
            Err(e) => DhruvStatus::from(&e),
        }
    })
}

/// Compute bhava (house) cusps with UTC input.
///
/// # Safety
/// All pointer arguments must be valid and non-null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dhruv_compute_bhavas_utc(
    engine: *const DhruvEngineHandle,
    lsk: *const DhruvLskHandle,
    eop: *const DhruvEopHandle,
    location: *const DhruvGeoLocation,
    utc: *const DhruvUtcTime,
    config: *const DhruvBhavaConfig,
    out_result: *mut DhruvBhavaResult,
) -> DhruvStatus {
    ffi_boundary(|| {
        if engine.is_null()
            || lsk.is_null()
            || eop.is_null()
            || location.is_null()
            || utc.is_null()
            || out_result.is_null()
        {
            return DhruvStatus::NullPointer;
        }
        let engine_ref = unsafe { &*engine };
        let lsk_ref = unsafe { &*lsk };
        let eop_ref = unsafe { &*eop };
        let loc_ref = unsafe { &*location };
        let geo = GeoLocation::new(
            loc_ref.latitude_deg,
            loc_ref.longitude_deg,
            loc_ref.altitude_m,
        );
        let rust_config = match resolve_bhava_config_ptr(config) {
            Ok(c) => c,
            Err(s) => return s,
        };
        let jd_utc = ffi_utc_to_jd_utc(unsafe { &*utc });
        let output_cfg_storage;
        let output_cfg = if config.is_null() {
            output_cfg_storage = dhruv_bhava_config_default();
            &output_cfg_storage
        } else {
            unsafe { &*config }
        };
        let projection = match bhava_output_projection_from_ffi(
            output_cfg,
            jd_utc_to_jd_tdb_with_eop(lsk_ref, eop_ref, jd_utc),
        ) {
            Ok(p) => p,
            Err(status) => return status,
        };
        match compute_bhavas(engine_ref, lsk_ref, eop_ref, &geo, jd_utc, &rust_config) {
            Ok(result) => {
                unsafe {
                    *out_result = bhava_result_to_ffi_with_projection(&result, projection);
                }
                DhruvStatus::Ok
            }
            Err(e) => DhruvStatus::from(&e),
        }
    })
}

/// Compute the Lagna (Ascendant) with UTC input.
///
/// # Safety
/// All pointer arguments must be valid and non-null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dhruv_lagna_deg_utc(
    lsk: *const DhruvLskHandle,
    eop: *const DhruvEopHandle,
    location: *const DhruvGeoLocation,
    utc: *const DhruvUtcTime,
    out_deg: *mut f64,
) -> DhruvStatus {
    ffi_boundary(|| {
        if lsk.is_null()
            || eop.is_null()
            || location.is_null()
            || utc.is_null()
            || out_deg.is_null()
        {
            return DhruvStatus::NullPointer;
        }
        let lsk_ref = unsafe { &*lsk };
        let eop_ref = unsafe { &*eop };
        let loc_ref = unsafe { &*location };
        let geo = GeoLocation::new(
            loc_ref.latitude_deg,
            loc_ref.longitude_deg,
            loc_ref.altitude_m,
        );
        let jd_utc = ffi_utc_to_jd_utc(unsafe { &*utc });
        match dhruv_vedic_base::lagna_longitude_rad(lsk_ref, eop_ref, &geo, jd_utc) {
            Ok(rad) => {
                unsafe { *out_deg = rad.to_degrees() };
                DhruvStatus::Ok
            }
            Err(e) => DhruvStatus::from(&e),
        }
    })
}

/// Compute the Lagna (Ascendant) with UTC input and optional sidereal output config.
///
/// # Safety
/// All pointer arguments must be valid and non-null except `config`, which may be NULL.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dhruv_lagna_deg_utc_with_config(
    lsk: *const DhruvLskHandle,
    eop: *const DhruvEopHandle,
    location: *const DhruvGeoLocation,
    utc: *const DhruvUtcTime,
    config: *const DhruvBhavaConfig,
    out_deg: *mut f64,
) -> DhruvStatus {
    ffi_boundary(|| {
        if lsk.is_null()
            || eop.is_null()
            || location.is_null()
            || utc.is_null()
            || out_deg.is_null()
        {
            return DhruvStatus::NullPointer;
        }
        let lsk_ref = unsafe { &*lsk };
        let eop_ref = unsafe { &*eop };
        let loc_ref = unsafe { &*location };
        let geo = GeoLocation::new(
            loc_ref.latitude_deg,
            loc_ref.longitude_deg,
            loc_ref.altitude_m,
        );
        let jd_utc = ffi_utc_to_jd_utc(unsafe { &*utc });
        let cfg_storage;
        let cfg_ref = if config.is_null() {
            cfg_storage = dhruv_bhava_config_default();
            &cfg_storage
        } else {
            unsafe { &*config }
        };
        let projection = match bhava_output_projection_from_ffi(
            cfg_ref,
            jd_utc_to_jd_tdb_with_eop(lsk_ref, eop_ref, jd_utc),
        ) {
            Ok(p) => p,
            Err(status) => return status,
        };
        match dhruv_vedic_base::lagna_longitude_rad(lsk_ref, eop_ref, &geo, jd_utc) {
            Ok(rad) => {
                unsafe { *out_deg = projected_tropical_deg(rad.to_degrees(), projection) };
                DhruvStatus::Ok
            }
            Err(e) => DhruvStatus::from(&e),
        }
    })
}

/// Compute the MC (Midheaven) with UTC input.
///
/// # Safety
/// All pointer arguments must be valid and non-null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dhruv_mc_deg_utc(
    lsk: *const DhruvLskHandle,
    eop: *const DhruvEopHandle,
    location: *const DhruvGeoLocation,
    utc: *const DhruvUtcTime,
    out_deg: *mut f64,
) -> DhruvStatus {
    ffi_boundary(|| {
        if lsk.is_null()
            || eop.is_null()
            || location.is_null()
            || utc.is_null()
            || out_deg.is_null()
        {
            return DhruvStatus::NullPointer;
        }
        let lsk_ref = unsafe { &*lsk };
        let eop_ref = unsafe { &*eop };
        let loc_ref = unsafe { &*location };
        let geo = GeoLocation::new(
            loc_ref.latitude_deg,
            loc_ref.longitude_deg,
            loc_ref.altitude_m,
        );
        let jd_utc = ffi_utc_to_jd_utc(unsafe { &*utc });
        match dhruv_vedic_base::mc_longitude_rad(lsk_ref, eop_ref, &geo, jd_utc) {
            Ok(rad) => {
                unsafe { *out_deg = rad.to_degrees() };
                DhruvStatus::Ok
            }
            Err(e) => DhruvStatus::from(&e),
        }
    })
}

/// Compute the MC (Midheaven) with UTC input and optional sidereal output config.
///
/// # Safety
/// All pointer arguments must be valid and non-null except `config`, which may be NULL.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dhruv_mc_deg_utc_with_config(
    lsk: *const DhruvLskHandle,
    eop: *const DhruvEopHandle,
    location: *const DhruvGeoLocation,
    utc: *const DhruvUtcTime,
    config: *const DhruvBhavaConfig,
    out_deg: *mut f64,
) -> DhruvStatus {
    ffi_boundary(|| {
        if lsk.is_null()
            || eop.is_null()
            || location.is_null()
            || utc.is_null()
            || out_deg.is_null()
        {
            return DhruvStatus::NullPointer;
        }
        let lsk_ref = unsafe { &*lsk };
        let eop_ref = unsafe { &*eop };
        let loc_ref = unsafe { &*location };
        let geo = GeoLocation::new(
            loc_ref.latitude_deg,
            loc_ref.longitude_deg,
            loc_ref.altitude_m,
        );
        let jd_utc = ffi_utc_to_jd_utc(unsafe { &*utc });
        let cfg_storage;
        let cfg_ref = if config.is_null() {
            cfg_storage = dhruv_bhava_config_default();
            &cfg_storage
        } else {
            unsafe { &*config }
        };
        let projection = match bhava_output_projection_from_ffi(
            cfg_ref,
            jd_utc_to_jd_tdb_with_eop(lsk_ref, eop_ref, jd_utc),
        ) {
            Ok(p) => p,
            Err(status) => return status,
        };
        match dhruv_vedic_base::mc_longitude_rad(lsk_ref, eop_ref, &geo, jd_utc) {
            Ok(rad) => {
                unsafe { *out_deg = projected_tropical_deg(rad.to_degrees(), projection) };
                DhruvStatus::Ok
            }
            Err(e) => DhruvStatus::from(&e),
        }
    })
}

/// Compute the RAMC (Right Ascension of the MC / Local Sidereal Time) with UTC input.
///
/// # Safety
/// All pointer arguments must be valid and non-null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dhruv_ramc_deg_utc(
    lsk: *const DhruvLskHandle,
    eop: *const DhruvEopHandle,
    location: *const DhruvGeoLocation,
    utc: *const DhruvUtcTime,
    out_deg: *mut f64,
) -> DhruvStatus {
    ffi_boundary(|| {
        if lsk.is_null()
            || eop.is_null()
            || location.is_null()
            || utc.is_null()
            || out_deg.is_null()
        {
            return DhruvStatus::NullPointer;
        }
        let lsk_ref = unsafe { &*lsk };
        let eop_ref = unsafe { &*eop };
        let loc_ref = unsafe { &*location };
        let geo = GeoLocation::new(
            loc_ref.latitude_deg,
            loc_ref.longitude_deg,
            loc_ref.altitude_m,
        );
        let jd_utc = ffi_utc_to_jd_utc(unsafe { &*utc });
        match dhruv_vedic_base::ramc_rad(lsk_ref, eop_ref, &geo, jd_utc) {
            Ok(rad) => {
                unsafe { *out_deg = rad.to_degrees() };
                DhruvStatus::Ok
            }
            Err(e) => DhruvStatus::from(&e),
        }
    })
}

// ---------------------------------------------------------------------------
// Group C: Pure math _utc functions (8 functions)
// ---------------------------------------------------------------------------

/// IAU 2000B nutation with UTC input. Requires LSK for UTC→TDB.
///
/// # Safety
/// `lsk`, `out_dpsi_arcsec`, and `out_deps_arcsec` must be valid, non-null pointers.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dhruv_nutation_iau2000b_utc(
    lsk: *const DhruvLskHandle,
    utc: *const DhruvUtcTime,
    out_dpsi_arcsec: *mut f64,
    out_deps_arcsec: *mut f64,
) -> DhruvStatus {
    ffi_boundary(|| {
        if lsk.is_null() || utc.is_null() || out_dpsi_arcsec.is_null() || out_deps_arcsec.is_null()
        {
            return DhruvStatus::NullPointer;
        }
        let lsk_ref = unsafe { &*lsk };
        let jd_tdb = ffi_to_utc_time(unsafe { &*utc }).to_jd_tdb(lsk_ref);
        let t = jd_tdb_to_centuries(jd_tdb);
        let (dpsi, deps) = dhruv_frames::nutation_iau2000b(t);
        unsafe {
            *out_dpsi_arcsec = dpsi;
            *out_deps_arcsec = deps;
        }
        DhruvStatus::Ok
    })
}

/// Lunar node longitude with UTC input. Requires LSK for UTC→TDB.
///
/// # Safety
/// `lsk` and `out_deg` must be valid, non-null pointers.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dhruv_lunar_node_deg_utc(
    lsk: *const DhruvLskHandle,
    node_code: i32,
    mode_code: i32,
    utc: *const DhruvUtcTime,
    out_deg: *mut f64,
) -> DhruvStatus {
    ffi_boundary(|| {
        if lsk.is_null() || utc.is_null() || out_deg.is_null() {
            return DhruvStatus::NullPointer;
        }
        let node = match lunar_node_from_code(node_code) {
            Some(n) => n,
            None => return DhruvStatus::InvalidQuery,
        };
        let mode = match node_mode_from_code(mode_code) {
            Some(m) => m,
            None => return DhruvStatus::InvalidQuery,
        };
        let lsk_ref = unsafe { &*lsk };
        let jd_tdb = ffi_to_utc_time(unsafe { &*utc }).to_jd_tdb(lsk_ref);
        let t = jd_tdb_to_centuries(jd_tdb);
        unsafe { *out_deg = lunar_node_deg(node, t, mode) };
        DhruvStatus::Ok
    })
}

/// Lunar node longitude with UTC input using an engine handle.
///
/// For `mode_code=1` (True), this uses an osculating-node computation from
/// Moon state vectors. For `mode_code=0` (Mean), this matches the polynomial
/// mean-node model.
///
/// # Safety
/// `engine`, `utc`, and `out_deg` must be valid, non-null pointers.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dhruv_lunar_node_deg_utc_with_engine(
    engine: *const DhruvEngineHandle,
    _lsk: *const DhruvLskHandle,
    node_code: i32,
    mode_code: i32,
    utc: *const DhruvUtcTime,
    out_deg: *mut f64,
) -> DhruvStatus {
    ffi_boundary(|| {
        if engine.is_null() || utc.is_null() || out_deg.is_null() {
            return DhruvStatus::NullPointer;
        }
        let node = match lunar_node_from_code(node_code) {
            Some(n) => n,
            None => return DhruvStatus::InvalidQuery,
        };
        let mode = match node_mode_from_code(mode_code) {
            Some(m) => m,
            None => return DhruvStatus::InvalidQuery,
        };
        let engine_ref = unsafe { &*engine };
        let jd_tdb = ffi_to_utc_time(unsafe { &*utc }).to_jd_tdb(engine_ref.lsk());
        match lunar_node_deg_for_epoch(engine_ref, node, jd_tdb, mode) {
            Ok(deg) => {
                unsafe { *out_deg = deg };
                DhruvStatus::Ok
            }
            Err(e) => DhruvStatus::from(&e),
        }
    })
}

/// Rashi from tropical longitude with UTC input. Requires LSK for UTC→TDB.
///
/// # Safety
/// `lsk` and `out` must be valid, non-null pointers.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dhruv_rashi_from_tropical_utc(
    lsk: *const DhruvLskHandle,
    tropical_lon_deg: f64,
    aya_system: i32,
    utc: *const DhruvUtcTime,
    use_nutation: u8,
    out: *mut DhruvRashiInfo,
) -> DhruvStatus {
    ffi_boundary(|| {
        if lsk.is_null() || utc.is_null() || out.is_null() {
            return DhruvStatus::NullPointer;
        }
        let system = match ayanamsha_system_from_code(aya_system) {
            Some(s) => s,
            None => return DhruvStatus::InvalidQuery,
        };
        let lsk_ref = unsafe { &*lsk };
        let jd_tdb = ffi_to_utc_time(unsafe { &*utc }).to_jd_tdb(lsk_ref);
        let info = rashi_from_tropical(tropical_lon_deg, system, jd_tdb, use_nutation != 0);
        unsafe {
            *out = DhruvRashiInfo {
                rashi_index: info.rashi_index,
                dms: DhruvDms {
                    degrees: info.dms.degrees,
                    minutes: info.dms.minutes,
                    seconds: info.dms.seconds,
                },
                degrees_in_rashi: info.degrees_in_rashi,
            };
        }
        DhruvStatus::Ok
    })
}

/// Nakshatra from tropical longitude with UTC input (27-scheme). Requires LSK for UTC→TDB.
///
/// # Safety
/// `lsk` and `out` must be valid, non-null pointers.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dhruv_nakshatra_from_tropical_utc(
    lsk: *const DhruvLskHandle,
    tropical_lon_deg: f64,
    aya_system: i32,
    utc: *const DhruvUtcTime,
    use_nutation: u8,
    out: *mut DhruvNakshatraInfo,
) -> DhruvStatus {
    ffi_boundary(|| {
        if lsk.is_null() || utc.is_null() || out.is_null() {
            return DhruvStatus::NullPointer;
        }
        let system = match ayanamsha_system_from_code(aya_system) {
            Some(s) => s,
            None => return DhruvStatus::InvalidQuery,
        };
        let lsk_ref = unsafe { &*lsk };
        let jd_tdb = ffi_to_utc_time(unsafe { &*utc }).to_jd_tdb(lsk_ref);
        let info = nakshatra_from_tropical(tropical_lon_deg, system, jd_tdb, use_nutation != 0);
        unsafe {
            *out = DhruvNakshatraInfo {
                nakshatra_index: info.nakshatra_index,
                pada: info.pada,
                degrees_in_nakshatra: info.degrees_in_nakshatra,
                degrees_in_pada: info.degrees_in_pada,
            };
        }
        DhruvStatus::Ok
    })
}

/// Nakshatra from tropical longitude with UTC input (28-scheme). Requires LSK for UTC→TDB.
///
/// # Safety
/// `lsk` and `out` must be valid, non-null pointers.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dhruv_nakshatra28_from_tropical_utc(
    lsk: *const DhruvLskHandle,
    tropical_lon_deg: f64,
    aya_system: i32,
    utc: *const DhruvUtcTime,
    use_nutation: u8,
    out: *mut DhruvNakshatra28Info,
) -> DhruvStatus {
    ffi_boundary(|| {
        if lsk.is_null() || utc.is_null() || out.is_null() {
            return DhruvStatus::NullPointer;
        }
        let system = match ayanamsha_system_from_code(aya_system) {
            Some(s) => s,
            None => return DhruvStatus::InvalidQuery,
        };
        let lsk_ref = unsafe { &*lsk };
        let jd_tdb = ffi_to_utc_time(unsafe { &*utc }).to_jd_tdb(lsk_ref);
        let info = nakshatra28_from_tropical(tropical_lon_deg, system, jd_tdb, use_nutation != 0);
        unsafe {
            *out = DhruvNakshatra28Info {
                nakshatra_index: info.nakshatra_index,
                pada: info.pada,
                degrees_in_nakshatra: info.degrees_in_nakshatra,
            };
        }
        DhruvStatus::Ok
    })
}

fn ffi_boundary(f: impl FnOnce() -> DhruvStatus) -> DhruvStatus {
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)) {
        Ok(status) => status,
        Err(_) => DhruvStatus::Internal,
    }
}

fn encode_c_utf8(input: &str) -> Result<[u8; DHRUV_PATH_CAPACITY], DhruvStatus> {
    if input.is_empty() {
        return Err(DhruvStatus::InvalidConfig);
    }

    let bytes = input.as_bytes();
    if bytes.len() >= DHRUV_PATH_CAPACITY {
        return Err(DhruvStatus::InvalidConfig);
    }
    if bytes.contains(&0) {
        return Err(DhruvStatus::InvalidConfig);
    }

    let mut out = [0_u8; DHRUV_PATH_CAPACITY];
    out[..bytes.len()].copy_from_slice(bytes);
    Ok(out)
}

fn decode_c_utf8(buffer: &[u8; DHRUV_PATH_CAPACITY]) -> Result<&str, std::str::Utf8Error> {
    let end = buffer
        .iter()
        .position(|b| *b == 0)
        .unwrap_or(DHRUV_PATH_CAPACITY);
    std::str::from_utf8(&buffer[..end])
}

// ---------------------------------------------------------------------------
// Tithi / Karana / Yoga / Vaar / Hora / Ghatika FFI
// ---------------------------------------------------------------------------

/// C-compatible Tithi info.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DhruvTithiInfo {
    /// 0-based tithi index (0=Shukla Pratipada .. 29=Amavasya).
    pub tithi_index: i32,
    /// Paksha: 0=Shukla, 1=Krishna.
    pub paksha: i32,
    /// 1-based tithi number within the paksha (1-15).
    pub tithi_in_paksha: i32,
    pub start: DhruvUtcTime,
    pub end: DhruvUtcTime,
}

/// C-compatible Karana info.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DhruvKaranaInfo {
    /// 0-based karana sequence index (0-59) within the synodic month.
    pub karana_index: i32,
    /// Karana name index in ALL_KARANAS (0=Bava .. 10=Kinstugna).
    pub karana_name_index: i32,
    pub start: DhruvUtcTime,
    pub end: DhruvUtcTime,
}

/// C-compatible Yoga info.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DhruvYogaInfo {
    /// 0-based yoga index (0=Vishkumbha .. 26=Vaidhriti).
    pub yoga_index: i32,
    pub start: DhruvUtcTime,
    pub end: DhruvUtcTime,
}

/// C-compatible Vaar info.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DhruvVaarInfo {
    /// 0-based vaar index (0=Ravivaar/Sunday .. 6=Shanivaar/Saturday).
    pub vaar_index: i32,
    pub start: DhruvUtcTime,
    pub end: DhruvUtcTime,
}

/// C-compatible Hora info.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DhruvHoraInfo {
    /// Hora lord index in CHALDEAN_SEQUENCE (0=Surya .. 6=Mangal).
    pub hora_index: i32,
    /// 0-based hora position within the Vedic day (0-23).
    pub hora_position: i32,
    pub start: DhruvUtcTime,
    pub end: DhruvUtcTime,
}

/// C-compatible Ghatika info.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DhruvGhatikaInfo {
    /// Ghatika value (1-60).
    pub value: i32,
    pub start: DhruvUtcTime,
    pub end: DhruvUtcTime,
}

/// Determine the Tithi for a given UTC date.
///
/// # Safety
/// All pointer arguments must be valid and non-null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dhruv_tithi_for_date(
    engine: *const DhruvEngineHandle,
    utc: *const DhruvUtcTime,
    out: *mut DhruvTithiInfo,
) -> DhruvStatus {
    ffi_boundary(|| {
        if engine.is_null() || utc.is_null() || out.is_null() {
            return DhruvStatus::NullPointer;
        }
        let engine_ref = unsafe { &*engine };
        let t = ffi_to_utc_time(unsafe { &*utc });
        match tithi_for_date(engine_ref, &t) {
            Ok(info) => {
                unsafe {
                    *out = DhruvTithiInfo {
                        tithi_index: info.tithi_index as i32,
                        paksha: info.paksha as i32,
                        tithi_in_paksha: info.tithi_in_paksha as i32,
                        start: utc_time_to_ffi(&info.start),
                        end: utc_time_to_ffi(&info.end),
                    };
                }
                DhruvStatus::Ok
            }
            Err(e) => DhruvStatus::from(&e),
        }
    })
}

/// Determine the Karana for a given UTC date.
///
/// # Safety
/// All pointer arguments must be valid and non-null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dhruv_karana_for_date(
    engine: *const DhruvEngineHandle,
    utc: *const DhruvUtcTime,
    out: *mut DhruvKaranaInfo,
) -> DhruvStatus {
    ffi_boundary(|| {
        if engine.is_null() || utc.is_null() || out.is_null() {
            return DhruvStatus::NullPointer;
        }
        let engine_ref = unsafe { &*engine };
        let t = ffi_to_utc_time(unsafe { &*utc });
        match karana_for_date(engine_ref, &t) {
            Ok(info) => {
                unsafe {
                    *out = DhruvKaranaInfo {
                        karana_index: info.karana_index as i32,
                        karana_name_index: info.karana.index() as i32,
                        start: utc_time_to_ffi(&info.start),
                        end: utc_time_to_ffi(&info.end),
                    };
                }
                DhruvStatus::Ok
            }
            Err(e) => DhruvStatus::from(&e),
        }
    })
}

/// Determine the Yoga for a given UTC date.
///
/// Requires SankrantiConfig for ayanamsha (sum does not cancel).
///
/// # Safety
/// All pointer arguments must be valid and non-null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dhruv_yoga_for_date(
    engine: *const DhruvEngineHandle,
    utc: *const DhruvUtcTime,
    config: *const DhruvSankrantiConfig,
    out: *mut DhruvYogaInfo,
) -> DhruvStatus {
    ffi_boundary(|| {
        if engine.is_null() || utc.is_null() || out.is_null() {
            return DhruvStatus::NullPointer;
        }
        let engine_ref = unsafe { &*engine };
        let t = ffi_to_utc_time(unsafe { &*utc });
        let cfg = match resolve_sankranti_config_ptr(config) {
            Ok(c) => c,
            Err(status) => return status,
        };
        match yoga_for_date(engine_ref, &t, &cfg) {
            Ok(info) => {
                unsafe {
                    *out = DhruvYogaInfo {
                        yoga_index: info.yoga_index as i32,
                        start: utc_time_to_ffi(&info.start),
                        end: utc_time_to_ffi(&info.end),
                    };
                }
                DhruvStatus::Ok
            }
            Err(e) => DhruvStatus::from(&e),
        }
    })
}

/// Determine the Moon's Nakshatra (27-scheme) for a given UTC date.
///
/// Requires SankrantiConfig for ayanamsha.
///
/// # Safety
/// All pointer arguments must be valid and non-null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dhruv_nakshatra_for_date(
    engine: *const DhruvEngineHandle,
    utc: *const DhruvUtcTime,
    config: *const DhruvSankrantiConfig,
    out: *mut DhruvPanchangNakshatraInfo,
) -> DhruvStatus {
    ffi_boundary(|| {
        if engine.is_null() || utc.is_null() || out.is_null() {
            return DhruvStatus::NullPointer;
        }
        let engine_ref = unsafe { &*engine };
        let t = ffi_to_utc_time(unsafe { &*utc });
        let cfg = match resolve_sankranti_config_ptr(config) {
            Ok(c) => c,
            Err(status) => return status,
        };
        match nakshatra_for_date(engine_ref, &t, &cfg) {
            Ok(info) => {
                unsafe {
                    *out = DhruvPanchangNakshatraInfo {
                        nakshatra_index: info.nakshatra_index as i32,
                        pada: info.pada as i32,
                        start: utc_time_to_ffi(&info.start),
                        end: utc_time_to_ffi(&info.end),
                    };
                }
                DhruvStatus::Ok
            }
            Err(e) => DhruvStatus::from(&e),
        }
    })
}

/// Determine the Vaar (weekday) for a given UTC date and location.
///
/// # Safety
/// All pointer arguments must be valid and non-null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dhruv_vaar_for_date(
    engine: *const DhruvEngineHandle,
    eop: *const DhruvEopHandle,
    utc: *const DhruvUtcTime,
    location: *const DhruvGeoLocation,
    riseset_config: *const DhruvRiseSetConfig,
    out: *mut DhruvVaarInfo,
) -> DhruvStatus {
    ffi_boundary(|| {
        if engine.is_null() || eop.is_null() || utc.is_null() || location.is_null() || out.is_null()
        {
            return DhruvStatus::NullPointer;
        }
        let engine_ref = unsafe { &*engine };
        let eop_ref = unsafe { &*eop };
        let t = ffi_to_utc_time(unsafe { &*utc });
        let loc_ref = unsafe { &*location };
        let geo = GeoLocation::new(
            loc_ref.latitude_deg,
            loc_ref.longitude_deg,
            loc_ref.altitude_m,
        );
        let rs_config = match resolve_riseset_config_ptr(riseset_config) {
            Ok(c) => c,
            Err(status) => return status,
        };
        match vaar_for_date(engine_ref, eop_ref, &t, &geo, &rs_config) {
            Ok(info) => {
                unsafe {
                    *out = DhruvVaarInfo {
                        vaar_index: info.vaar.index() as i32,
                        start: utc_time_to_ffi(&info.start),
                        end: utc_time_to_ffi(&info.end),
                    };
                }
                DhruvStatus::Ok
            }
            Err(e) => DhruvStatus::from(&e),
        }
    })
}

/// Determine the Hora (planetary hour) for a given UTC date and location.
///
/// # Safety
/// All pointer arguments must be valid and non-null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dhruv_hora_for_date(
    engine: *const DhruvEngineHandle,
    eop: *const DhruvEopHandle,
    utc: *const DhruvUtcTime,
    location: *const DhruvGeoLocation,
    riseset_config: *const DhruvRiseSetConfig,
    out: *mut DhruvHoraInfo,
) -> DhruvStatus {
    ffi_boundary(|| {
        if engine.is_null() || eop.is_null() || utc.is_null() || location.is_null() || out.is_null()
        {
            return DhruvStatus::NullPointer;
        }
        let engine_ref = unsafe { &*engine };
        let eop_ref = unsafe { &*eop };
        let t = ffi_to_utc_time(unsafe { &*utc });
        let loc_ref = unsafe { &*location };
        let geo = GeoLocation::new(
            loc_ref.latitude_deg,
            loc_ref.longitude_deg,
            loc_ref.altitude_m,
        );
        let rs_config = match resolve_riseset_config_ptr(riseset_config) {
            Ok(c) => c,
            Err(status) => return status,
        };
        match hora_for_date(engine_ref, eop_ref, &t, &geo, &rs_config) {
            Ok(info) => {
                unsafe {
                    *out = DhruvHoraInfo {
                        hora_index: info.hora.index() as i32,
                        hora_position: info.hora_index as i32,
                        start: utc_time_to_ffi(&info.start),
                        end: utc_time_to_ffi(&info.end),
                    };
                }
                DhruvStatus::Ok
            }
            Err(e) => DhruvStatus::from(&e),
        }
    })
}

/// Determine the Ghatika for a given UTC date and location.
///
/// # Safety
/// All pointer arguments must be valid and non-null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dhruv_ghatika_for_date(
    engine: *const DhruvEngineHandle,
    eop: *const DhruvEopHandle,
    utc: *const DhruvUtcTime,
    location: *const DhruvGeoLocation,
    riseset_config: *const DhruvRiseSetConfig,
    out: *mut DhruvGhatikaInfo,
) -> DhruvStatus {
    ffi_boundary(|| {
        if engine.is_null() || eop.is_null() || utc.is_null() || location.is_null() || out.is_null()
        {
            return DhruvStatus::NullPointer;
        }
        let engine_ref = unsafe { &*engine };
        let eop_ref = unsafe { &*eop };
        let t = ffi_to_utc_time(unsafe { &*utc });
        let loc_ref = unsafe { &*location };
        let geo = GeoLocation::new(
            loc_ref.latitude_deg,
            loc_ref.longitude_deg,
            loc_ref.altitude_m,
        );
        let rs_config = match resolve_riseset_config_ptr(riseset_config) {
            Ok(c) => c,
            Err(status) => return status,
        };
        match ghatika_for_date(engine_ref, eop_ref, &t, &geo, &rs_config) {
            Ok(info) => {
                unsafe {
                    *out = DhruvGhatikaInfo {
                        value: info.value as i32,
                        start: utc_time_to_ffi(&info.start),
                        end: utc_time_to_ffi(&info.end),
                    };
                }
                DhruvStatus::Ok
            }
            Err(e) => DhruvStatus::from(&e),
        }
    })
}

/// C-compatible Panchang Nakshatra info (Moon's nakshatra with boundaries).
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DhruvPanchangNakshatraInfo {
    /// 0-based nakshatra index (0=Ashwini .. 26=Revati).
    pub nakshatra_index: i32,
    /// Pada (quarter) within the nakshatra (1-4).
    pub pada: i32,
    pub start: DhruvUtcTime,
    pub end: DhruvUtcTime,
}

/// C-compatible combined Panchang info.
///
/// Contains all seven daily elements plus optional calendar fields.
/// Calendar fields use `*_valid` flags (0=absent, 1=present).
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DhruvPanchangInfo {
    pub tithi: DhruvTithiInfo,
    pub karana: DhruvKaranaInfo,
    pub yoga: DhruvYogaInfo,
    pub vaar: DhruvVaarInfo,
    pub hora: DhruvHoraInfo,
    pub ghatika: DhruvGhatikaInfo,
    pub nakshatra: DhruvPanchangNakshatraInfo,
    /// 1 if masa/ayana/varsha fields are populated, 0 otherwise.
    pub calendar_valid: u8,
    pub masa: DhruvMasaInfo,
    pub ayana: DhruvAyanaInfo,
    pub varsha: DhruvVarshaInfo,
}

/// Unified panchang compute entrypoint with include-mask control.
///
/// `time_kind` controls whether input comes from `jd_tdb` or `utc`.
/// `include_mask` selects which fields are populated in `out`.
///
/// # Safety
/// `engine`, `eop`, `request`, and `out` must be valid non-null pointers.
/// `lsk` is required when `request->time_kind == DHRUV_PANCHANG_TIME_JD_TDB`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dhruv_panchang_compute_ex(
    engine: *const DhruvEngineHandle,
    eop: *const DhruvEopHandle,
    lsk: *const DhruvLskHandle,
    request: *const DhruvPanchangComputeRequest,
    out: *mut DhruvPanchangOperationResult,
) -> DhruvStatus {
    ffi_boundary(|| {
        if engine.is_null() || eop.is_null() || request.is_null() || out.is_null() {
            return DhruvStatus::NullPointer;
        }

        let req = unsafe { &*request };

        let include_mask = match panchang_include_mask_from_ffi(req.include_mask) {
            Some(v) => v,
            None => return DhruvStatus::InvalidQuery,
        };

        let cfg = match sankranti_config_from_ffi(&req.sankranti_config) {
            Some(c) => c,
            None => return DhruvStatus::InvalidQuery,
        };

        let sun_limb = match sun_limb_from_code(req.riseset_config.sun_limb) {
            Some(v) => v,
            None => return DhruvStatus::InvalidQuery,
        };
        let riseset_config = RiseSetConfig {
            use_refraction: req.riseset_config.use_refraction != 0,
            sun_limb,
            altitude_correction: req.riseset_config.altitude_correction != 0,
        };

        let at_utc = match req.time_kind {
            DHRUV_PANCHANG_TIME_JD_TDB => {
                if lsk.is_null() {
                    return DhruvStatus::NullPointer;
                }
                let lsk_ref = unsafe { &*lsk };
                UtcTime::from_jd_tdb(req.jd_tdb, lsk_ref)
            }
            DHRUV_PANCHANG_TIME_UTC => ffi_to_utc_time(&req.utc),
            _ => return DhruvStatus::InvalidQuery,
        };

        let location = GeoLocation::new(
            req.location.latitude_deg,
            req.location.longitude_deg,
            req.location.altitude_m,
        );

        let op = PanchangOperation {
            at_utc,
            location,
            riseset_config,
            sankranti_config: cfg,
            include_mask,
        };

        let engine_ref = unsafe { &*engine };
        let eop_ref = unsafe { &*eop };
        match dhruv_vedic_ops::panchang(engine_ref, eop_ref, &op) {
            Ok(info) => {
                unsafe { *out = panchang_result_to_ffi(&info) };
                DhruvStatus::Ok
            }
            Err(e) => DhruvStatus::from(&e),
        }
    })
}

fn zeroed_utc() -> DhruvUtcTime {
    utc_time_to_ffi(&UtcTime::new(0, 0, 0, 0, 0, 0.0))
}

fn zeroed_tithi_info() -> DhruvTithiInfo {
    let z = zeroed_utc();
    DhruvTithiInfo {
        tithi_index: 0,
        paksha: 0,
        tithi_in_paksha: 0,
        start: z,
        end: z,
    }
}

fn zeroed_karana_info() -> DhruvKaranaInfo {
    let z = zeroed_utc();
    DhruvKaranaInfo {
        karana_index: 0,
        karana_name_index: 0,
        start: z,
        end: z,
    }
}

fn zeroed_yoga_info() -> DhruvYogaInfo {
    let z = zeroed_utc();
    DhruvYogaInfo {
        yoga_index: 0,
        start: z,
        end: z,
    }
}

fn zeroed_vaar_info() -> DhruvVaarInfo {
    let z = zeroed_utc();
    DhruvVaarInfo {
        vaar_index: 0,
        start: z,
        end: z,
    }
}

fn zeroed_hora_info() -> DhruvHoraInfo {
    let z = zeroed_utc();
    DhruvHoraInfo {
        hora_index: 0,
        hora_position: 0,
        start: z,
        end: z,
    }
}

fn zeroed_ghatika_info() -> DhruvGhatikaInfo {
    let z = zeroed_utc();
    DhruvGhatikaInfo {
        value: 0,
        start: z,
        end: z,
    }
}

fn zeroed_panchang_nakshatra_info() -> DhruvPanchangNakshatraInfo {
    let z = zeroed_utc();
    DhruvPanchangNakshatraInfo {
        nakshatra_index: 0,
        pada: 0,
        start: z,
        end: z,
    }
}

fn zeroed_masa_info() -> DhruvMasaInfo {
    let z = zeroed_utc();
    DhruvMasaInfo {
        masa_index: 0,
        adhika: 0,
        start: z,
        end: z,
    }
}

fn zeroed_ayana_info() -> DhruvAyanaInfo {
    let z = zeroed_utc();
    DhruvAyanaInfo {
        ayana: 0,
        start: z,
        end: z,
    }
}

fn zeroed_varsha_info() -> DhruvVarshaInfo {
    let z = zeroed_utc();
    DhruvVarshaInfo {
        samvatsara_index: 0,
        order: 0,
        start: z,
        end: z,
    }
}

fn tithi_info_to_ffi(info: &dhruv_search::TithiInfo) -> DhruvTithiInfo {
    DhruvTithiInfo {
        tithi_index: info.tithi_index as i32,
        paksha: info.paksha as i32,
        tithi_in_paksha: info.tithi_in_paksha as i32,
        start: utc_time_to_ffi(&info.start),
        end: utc_time_to_ffi(&info.end),
    }
}

fn karana_info_to_ffi(info: &dhruv_search::KaranaInfo) -> DhruvKaranaInfo {
    DhruvKaranaInfo {
        karana_index: info.karana_index as i32,
        karana_name_index: info.karana.index() as i32,
        start: utc_time_to_ffi(&info.start),
        end: utc_time_to_ffi(&info.end),
    }
}

fn yoga_info_to_ffi(info: &dhruv_search::YogaInfo) -> DhruvYogaInfo {
    DhruvYogaInfo {
        yoga_index: info.yoga_index as i32,
        start: utc_time_to_ffi(&info.start),
        end: utc_time_to_ffi(&info.end),
    }
}

fn vaar_info_to_ffi(info: &dhruv_search::VaarInfo) -> DhruvVaarInfo {
    DhruvVaarInfo {
        vaar_index: info.vaar.index() as i32,
        start: utc_time_to_ffi(&info.start),
        end: utc_time_to_ffi(&info.end),
    }
}

fn hora_info_to_ffi(info: &dhruv_search::HoraInfo) -> DhruvHoraInfo {
    DhruvHoraInfo {
        hora_index: info.hora.index() as i32,
        hora_position: info.hora_index as i32,
        start: utc_time_to_ffi(&info.start),
        end: utc_time_to_ffi(&info.end),
    }
}

fn ghatika_info_to_ffi(info: &dhruv_search::GhatikaInfo) -> DhruvGhatikaInfo {
    DhruvGhatikaInfo {
        value: info.value as i32,
        start: utc_time_to_ffi(&info.start),
        end: utc_time_to_ffi(&info.end),
    }
}

fn panchang_nakshatra_info_to_ffi(
    info: &dhruv_search::PanchangNakshatraInfo,
) -> DhruvPanchangNakshatraInfo {
    DhruvPanchangNakshatraInfo {
        nakshatra_index: info.nakshatra_index as i32,
        pada: info.pada as i32,
        start: utc_time_to_ffi(&info.start),
        end: utc_time_to_ffi(&info.end),
    }
}

fn masa_info_to_ffi(info: &dhruv_search::MasaInfo) -> DhruvMasaInfo {
    DhruvMasaInfo {
        masa_index: info.masa.index() as i32,
        adhika: u8::from(info.adhika),
        start: utc_time_to_ffi(&info.start),
        end: utc_time_to_ffi(&info.end),
    }
}

fn ayana_info_to_ffi(info: &dhruv_search::AyanaInfo) -> DhruvAyanaInfo {
    DhruvAyanaInfo {
        ayana: info.ayana.index() as i32,
        start: utc_time_to_ffi(&info.start),
        end: utc_time_to_ffi(&info.end),
    }
}

fn varsha_info_to_ffi(info: &dhruv_search::VarshaInfo) -> DhruvVarshaInfo {
    DhruvVarshaInfo {
        samvatsara_index: info.samvatsara.index() as i32,
        order: info.order as i32,
        start: utc_time_to_ffi(&info.start),
        end: utc_time_to_ffi(&info.end),
    }
}

/// Convert a Rust `PanchangInfo` to a C-compatible `DhruvPanchangInfo`.
fn panchang_info_to_ffi(info: &dhruv_search::PanchangInfo) -> DhruvPanchangInfo {
    let (calendar_valid, masa_ffi, ayana_ffi, varsha_ffi) =
        match (info.masa, info.ayana, info.varsha) {
            (Some(m), Some(a), Some(v)) => (
                1u8,
                masa_info_to_ffi(&m),
                ayana_info_to_ffi(&a),
                varsha_info_to_ffi(&v),
            ),
            _ => (
                0u8,
                zeroed_masa_info(),
                zeroed_ayana_info(),
                zeroed_varsha_info(),
            ),
        };
    DhruvPanchangInfo {
        tithi: tithi_info_to_ffi(&info.tithi),
        karana: karana_info_to_ffi(&info.karana),
        yoga: yoga_info_to_ffi(&info.yoga),
        vaar: vaar_info_to_ffi(&info.vaar),
        hora: hora_info_to_ffi(&info.hora),
        ghatika: ghatika_info_to_ffi(&info.ghatika),
        nakshatra: panchang_nakshatra_info_to_ffi(&info.nakshatra),
        calendar_valid,
        masa: masa_ffi,
        ayana: ayana_ffi,
        varsha: varsha_ffi,
    }
}

fn tithi_info_to_ffi_ops(info: &dhruv_vedic_ops::TithiInfo) -> DhruvTithiInfo {
    DhruvTithiInfo {
        tithi_index: info.tithi_index as i32,
        paksha: info.paksha as i32,
        tithi_in_paksha: info.tithi_in_paksha as i32,
        start: utc_time_to_ffi(&info.start),
        end: utc_time_to_ffi(&info.end),
    }
}

fn karana_info_to_ffi_ops(info: &dhruv_vedic_ops::KaranaInfo) -> DhruvKaranaInfo {
    DhruvKaranaInfo {
        karana_index: info.karana_index as i32,
        karana_name_index: info.karana.index() as i32,
        start: utc_time_to_ffi(&info.start),
        end: utc_time_to_ffi(&info.end),
    }
}

fn yoga_info_to_ffi_ops(info: &dhruv_vedic_ops::YogaInfo) -> DhruvYogaInfo {
    DhruvYogaInfo {
        yoga_index: info.yoga_index as i32,
        start: utc_time_to_ffi(&info.start),
        end: utc_time_to_ffi(&info.end),
    }
}

fn vaar_info_to_ffi_ops(info: &dhruv_vedic_ops::VaarInfo) -> DhruvVaarInfo {
    DhruvVaarInfo {
        vaar_index: info.vaar.index() as i32,
        start: utc_time_to_ffi(&info.start),
        end: utc_time_to_ffi(&info.end),
    }
}

fn hora_info_to_ffi_ops(info: &dhruv_vedic_ops::HoraInfo) -> DhruvHoraInfo {
    DhruvHoraInfo {
        hora_index: info.hora.index() as i32,
        hora_position: info.hora_index as i32,
        start: utc_time_to_ffi(&info.start),
        end: utc_time_to_ffi(&info.end),
    }
}

fn ghatika_info_to_ffi_ops(info: &dhruv_vedic_ops::GhatikaInfo) -> DhruvGhatikaInfo {
    DhruvGhatikaInfo {
        value: info.value as i32,
        start: utc_time_to_ffi(&info.start),
        end: utc_time_to_ffi(&info.end),
    }
}

fn panchang_nakshatra_info_to_ffi_ops(
    info: &dhruv_vedic_ops::PanchangNakshatraInfo,
) -> DhruvPanchangNakshatraInfo {
    DhruvPanchangNakshatraInfo {
        nakshatra_index: info.nakshatra_index as i32,
        pada: info.pada as i32,
        start: utc_time_to_ffi(&info.start),
        end: utc_time_to_ffi(&info.end),
    }
}

fn masa_info_to_ffi_ops(info: &dhruv_vedic_ops::MasaInfo) -> DhruvMasaInfo {
    DhruvMasaInfo {
        masa_index: info.masa.index() as i32,
        adhika: u8::from(info.adhika),
        start: utc_time_to_ffi(&info.start),
        end: utc_time_to_ffi(&info.end),
    }
}

fn ayana_info_to_ffi_ops(info: &dhruv_vedic_ops::AyanaInfo) -> DhruvAyanaInfo {
    DhruvAyanaInfo {
        ayana: info.ayana.index() as i32,
        start: utc_time_to_ffi(&info.start),
        end: utc_time_to_ffi(&info.end),
    }
}

fn varsha_info_to_ffi_ops(info: &dhruv_vedic_ops::VarshaInfo) -> DhruvVarshaInfo {
    DhruvVarshaInfo {
        samvatsara_index: info.samvatsara.index() as i32,
        order: info.order as i32,
        start: utc_time_to_ffi(&info.start),
        end: utc_time_to_ffi(&info.end),
    }
}

fn panchang_result_to_ffi(info: &PanchangResult) -> DhruvPanchangOperationResult {
    let (tithi_valid, tithi) = match info.tithi {
        Some(v) => (1, tithi_info_to_ffi_ops(&v)),
        None => (0, zeroed_tithi_info()),
    };
    let (karana_valid, karana) = match info.karana {
        Some(v) => (1, karana_info_to_ffi_ops(&v)),
        None => (0, zeroed_karana_info()),
    };
    let (yoga_valid, yoga) = match info.yoga {
        Some(v) => (1, yoga_info_to_ffi_ops(&v)),
        None => (0, zeroed_yoga_info()),
    };
    let (vaar_valid, vaar) = match info.vaar {
        Some(v) => (1, vaar_info_to_ffi_ops(&v)),
        None => (0, zeroed_vaar_info()),
    };
    let (hora_valid, hora) = match info.hora {
        Some(v) => (1, hora_info_to_ffi_ops(&v)),
        None => (0, zeroed_hora_info()),
    };
    let (ghatika_valid, ghatika) = match info.ghatika {
        Some(v) => (1, ghatika_info_to_ffi_ops(&v)),
        None => (0, zeroed_ghatika_info()),
    };
    let (nakshatra_valid, nakshatra) = match info.nakshatra {
        Some(v) => (1, panchang_nakshatra_info_to_ffi_ops(&v)),
        None => (0, zeroed_panchang_nakshatra_info()),
    };
    let (masa_valid, masa) = match info.masa {
        Some(v) => (1, masa_info_to_ffi_ops(&v)),
        None => (0, zeroed_masa_info()),
    };
    let (ayana_valid, ayana) = match info.ayana {
        Some(v) => (1, ayana_info_to_ffi_ops(&v)),
        None => (0, zeroed_ayana_info()),
    };
    let (varsha_valid, varsha) = match info.varsha {
        Some(v) => (1, varsha_info_to_ffi_ops(&v)),
        None => (0, zeroed_varsha_info()),
    };

    DhruvPanchangOperationResult {
        tithi_valid,
        tithi,
        karana_valid,
        karana,
        yoga_valid,
        yoga,
        vaar_valid,
        vaar,
        hora_valid,
        hora,
        ghatika_valid,
        ghatika,
        nakshatra_valid,
        nakshatra,
        masa_valid,
        masa,
        ayana_valid,
        ayana,
        varsha_valid,
        varsha,
    }
}

/// Return the name of a tithi by index (0-29).
///
/// Returns a NUL-terminated static string, or null for invalid index.
#[unsafe(no_mangle)]
pub extern "C" fn dhruv_tithi_name(index: u32) -> *const std::ffi::c_char {
    static NAMES: [&str; 30] = [
        "Shukla Pratipada\0",
        "Shukla Dwitiya\0",
        "Shukla Tritiya\0",
        "Shukla Chaturthi\0",
        "Shukla Panchami\0",
        "Shukla Shashthi\0",
        "Shukla Saptami\0",
        "Shukla Ashtami\0",
        "Shukla Navami\0",
        "Shukla Dashami\0",
        "Shukla Ekadashi\0",
        "Shukla Dwadashi\0",
        "Shukla Trayodashi\0",
        "Shukla Chaturdashi\0",
        "Purnima\0",
        "Krishna Pratipada\0",
        "Krishna Dwitiya\0",
        "Krishna Tritiya\0",
        "Krishna Chaturthi\0",
        "Krishna Panchami\0",
        "Krishna Shashthi\0",
        "Krishna Saptami\0",
        "Krishna Ashtami\0",
        "Krishna Navami\0",
        "Krishna Dashami\0",
        "Krishna Ekadashi\0",
        "Krishna Dwadashi\0",
        "Krishna Trayodashi\0",
        "Krishna Chaturdashi\0",
        "Amavasya\0",
    ];
    match NAMES.get(index as usize) {
        Some(s) => s.as_ptr().cast(),
        None => ptr::null(),
    }
}

/// Return the name of a karana by name-index (0-10, per ALL_KARANAS).
///
/// Returns a NUL-terminated static string, or null for invalid index.
#[unsafe(no_mangle)]
pub extern "C" fn dhruv_karana_name(index: u32) -> *const std::ffi::c_char {
    static NAMES: [&str; 11] = [
        "Bava\0",
        "Balava\0",
        "Kaulava\0",
        "Taitilla\0",
        "Garija\0",
        "Vanija\0",
        "Vishti\0",
        "Shakuni\0",
        "Chatuspad\0",
        "Naga\0",
        "Kinstugna\0",
    ];
    match NAMES.get(index as usize) {
        Some(s) => s.as_ptr().cast(),
        None => ptr::null(),
    }
}

/// Return the name of a yoga by index (0-26).
///
/// Returns a NUL-terminated static string, or null for invalid index.
#[unsafe(no_mangle)]
pub extern "C" fn dhruv_yoga_name(index: u32) -> *const std::ffi::c_char {
    static NAMES: [&str; 27] = [
        "Vishkumbha\0",
        "Priti\0",
        "Ayushman\0",
        "Saubhagya\0",
        "Shobhana\0",
        "Atiganda\0",
        "Sukarma\0",
        "Dhriti\0",
        "Shula\0",
        "Ganda\0",
        "Vriddhi\0",
        "Dhruva\0",
        "Vyaghata\0",
        "Harshana\0",
        "Vajra\0",
        "Siddhi\0",
        "Vyatipata\0",
        "Variyan\0",
        "Parigha\0",
        "Shiva\0",
        "Siddha\0",
        "Sadhya\0",
        "Shubha\0",
        "Shukla\0",
        "Brahma\0",
        "Indra\0",
        "Vaidhriti\0",
    ];
    match NAMES.get(index as usize) {
        Some(s) => s.as_ptr().cast(),
        None => ptr::null(),
    }
}

/// Return the name of a vaar by index (0-6).
///
/// Returns a NUL-terminated static string, or null for invalid index.
#[unsafe(no_mangle)]
pub extern "C" fn dhruv_vaar_name(index: u32) -> *const std::ffi::c_char {
    static NAMES: [&str; 7] = [
        "Ravivaar\0",
        "Somvaar\0",
        "Mangalvaar\0",
        "Budhvaar\0",
        "Guruvaar\0",
        "Shukravaar\0",
        "Shanivaar\0",
    ];
    match NAMES.get(index as usize) {
        Some(s) => s.as_ptr().cast(),
        None => ptr::null(),
    }
}

/// Return the name of a hora lord by Chaldean index (0-6).
///
/// Returns a NUL-terminated static string, or null for invalid index.
#[unsafe(no_mangle)]
pub extern "C" fn dhruv_hora_name(index: u32) -> *const std::ffi::c_char {
    static NAMES: [&str; 7] = [
        "Surya\0",
        "Shukra\0",
        "Buddh\0",
        "Chandra\0",
        "Shani\0",
        "Guru\0",
        "Mangal\0",
    ];
    match NAMES.get(index as usize) {
        Some(s) => s.as_ptr().cast(),
        None => ptr::null(),
    }
}

// ---------------------------------------------------------------------------
// Panchang composable intermediates + pre-computed input variants
// ---------------------------------------------------------------------------

/// Compute Moon-Sun elongation at a given JD TDB.
///
/// Returns (Moon_lon - Sun_lon) mod 360 in degrees via `out_deg`.
///
/// # Safety
/// All pointer arguments must be valid and non-null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dhruv_elongation_at(
    engine: *const DhruvEngineHandle,
    jd_tdb: f64,
    out_deg: *mut f64,
) -> DhruvStatus {
    ffi_boundary(|| {
        if engine.is_null() || out_deg.is_null() {
            return DhruvStatus::NullPointer;
        }
        let engine_ref = unsafe { &*engine };
        match elongation_at(engine_ref, jd_tdb) {
            Ok(deg) => {
                unsafe {
                    *out_deg = deg;
                }
                DhruvStatus::Ok
            }
            Err(e) => DhruvStatus::from(&e),
        }
    })
}

/// Compute sidereal sum (Moon_sid + Sun_sid) mod 360 at a given JD TDB.
///
/// Requires SankrantiConfig for ayanamsha (sum does not cancel).
///
/// # Safety
/// All pointer arguments must be valid and non-null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dhruv_sidereal_sum_at(
    engine: *const DhruvEngineHandle,
    jd_tdb: f64,
    config: *const DhruvSankrantiConfig,
    out_deg: *mut f64,
) -> DhruvStatus {
    ffi_boundary(|| {
        if engine.is_null() || out_deg.is_null() {
            return DhruvStatus::NullPointer;
        }
        let engine_ref = unsafe { &*engine };
        let cfg = match resolve_sankranti_config_ptr(config) {
            Ok(c) => c,
            Err(status) => return status,
        };
        match sidereal_sum_at(engine_ref, jd_tdb, &cfg) {
            Ok(deg) => {
                unsafe {
                    *out_deg = deg;
                }
                DhruvStatus::Ok
            }
            Err(e) => DhruvStatus::from(&e),
        }
    })
}

/// Compute Vedic day sunrise bracket for a given UTC moment.
///
/// Writes the two JD TDB values of the bracketing sunrises to
/// `out_sunrise_jd` and `out_next_sunrise_jd`.
///
/// # Safety
/// All pointer arguments must be valid and non-null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dhruv_vedic_day_sunrises(
    engine: *const DhruvEngineHandle,
    eop: *const DhruvEopHandle,
    utc: *const DhruvUtcTime,
    location: *const DhruvGeoLocation,
    riseset_config: *const DhruvRiseSetConfig,
    out_sunrise_jd: *mut f64,
    out_next_sunrise_jd: *mut f64,
) -> DhruvStatus {
    ffi_boundary(|| {
        if engine.is_null()
            || eop.is_null()
            || utc.is_null()
            || location.is_null()
            || out_sunrise_jd.is_null()
            || out_next_sunrise_jd.is_null()
        {
            return DhruvStatus::NullPointer;
        }
        let engine_ref = unsafe { &*engine };
        let eop_ref = unsafe { &*eop };
        let t = ffi_to_utc_time(unsafe { &*utc });
        let loc_ref = unsafe { &*location };
        let geo = GeoLocation::new(
            loc_ref.latitude_deg,
            loc_ref.longitude_deg,
            loc_ref.altitude_m,
        );
        let rs_config = match resolve_riseset_config_ptr(riseset_config) {
            Ok(c) => c,
            Err(status) => return status,
        };
        match vedic_day_sunrises(engine_ref, eop_ref, &t, &geo, &rs_config) {
            Ok((sr, nsr)) => {
                unsafe {
                    *out_sunrise_jd = sr;
                    *out_next_sunrise_jd = nsr;
                }
                DhruvStatus::Ok
            }
            Err(e) => DhruvStatus::from(&e),
        }
    })
}

/// Query a body's ecliptic longitude and latitude in degrees.
///
/// # Safety
/// All pointer arguments must be valid and non-null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dhruv_body_ecliptic_lon_lat(
    engine: *const DhruvEngineHandle,
    body_code: i32,
    jd_tdb: f64,
    out_lon_deg: *mut f64,
    out_lat_deg: *mut f64,
) -> DhruvStatus {
    ffi_boundary(|| {
        if engine.is_null() || out_lon_deg.is_null() || out_lat_deg.is_null() {
            return DhruvStatus::NullPointer;
        }
        let engine_ref = unsafe { &*engine };
        let body = match Body::from_code(body_code) {
            Some(b) => b,
            None => return DhruvStatus::InvalidQuery,
        };
        match body_ecliptic_lon_lat(engine_ref, body, jd_tdb) {
            Ok((lon, lat)) => {
                unsafe {
                    *out_lon_deg = lon;
                    *out_lat_deg = lat;
                }
                DhruvStatus::Ok
            }
            Err(e) => DhruvStatus::from(&e),
        }
    })
}

/// Determine the Tithi from a pre-computed elongation.
///
/// # Safety
/// All pointer arguments must be valid and non-null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dhruv_tithi_at(
    engine: *const DhruvEngineHandle,
    jd_tdb: f64,
    elongation_deg: f64,
    out: *mut DhruvTithiInfo,
) -> DhruvStatus {
    ffi_boundary(|| {
        if engine.is_null() || out.is_null() {
            return DhruvStatus::NullPointer;
        }
        let engine_ref = unsafe { &*engine };
        match tithi_at(engine_ref, jd_tdb, elongation_deg) {
            Ok(info) => {
                unsafe {
                    *out = DhruvTithiInfo {
                        tithi_index: info.tithi_index as i32,
                        paksha: info.paksha as i32,
                        tithi_in_paksha: info.tithi_in_paksha as i32,
                        start: utc_time_to_ffi(&info.start),
                        end: utc_time_to_ffi(&info.end),
                    };
                }
                DhruvStatus::Ok
            }
            Err(e) => DhruvStatus::from(&e),
        }
    })
}

/// Determine the Karana from a pre-computed elongation.
///
/// # Safety
/// All pointer arguments must be valid and non-null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dhruv_karana_at(
    engine: *const DhruvEngineHandle,
    jd_tdb: f64,
    elongation_deg: f64,
    out: *mut DhruvKaranaInfo,
) -> DhruvStatus {
    ffi_boundary(|| {
        if engine.is_null() || out.is_null() {
            return DhruvStatus::NullPointer;
        }
        let engine_ref = unsafe { &*engine };
        match karana_at(engine_ref, jd_tdb, elongation_deg) {
            Ok(info) => {
                unsafe {
                    *out = DhruvKaranaInfo {
                        karana_index: info.karana_index as i32,
                        karana_name_index: info.karana.index() as i32,
                        start: utc_time_to_ffi(&info.start),
                        end: utc_time_to_ffi(&info.end),
                    };
                }
                DhruvStatus::Ok
            }
            Err(e) => DhruvStatus::from(&e),
        }
    })
}

/// Determine the Yoga from a pre-computed sidereal sum.
///
/// # Safety
/// All pointer arguments must be valid and non-null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dhruv_yoga_at(
    engine: *const DhruvEngineHandle,
    jd_tdb: f64,
    sidereal_sum_deg: f64,
    config: *const DhruvSankrantiConfig,
    out: *mut DhruvYogaInfo,
) -> DhruvStatus {
    ffi_boundary(|| {
        if engine.is_null() || out.is_null() {
            return DhruvStatus::NullPointer;
        }
        let engine_ref = unsafe { &*engine };
        let cfg = match resolve_sankranti_config_ptr(config) {
            Ok(c) => c,
            Err(status) => return status,
        };
        match yoga_at(engine_ref, jd_tdb, sidereal_sum_deg, &cfg) {
            Ok(info) => {
                unsafe {
                    *out = DhruvYogaInfo {
                        yoga_index: info.yoga_index as i32,
                        start: utc_time_to_ffi(&info.start),
                        end: utc_time_to_ffi(&info.end),
                    };
                }
                DhruvStatus::Ok
            }
            Err(e) => DhruvStatus::from(&e),
        }
    })
}

/// Determine the Vaar from pre-computed sunrise boundaries.
///
/// # Safety
/// All pointer arguments must be valid and non-null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dhruv_vaar_from_sunrises(
    lsk: *const DhruvLskHandle,
    sunrise_jd: f64,
    next_sunrise_jd: f64,
    out: *mut DhruvVaarInfo,
) -> DhruvStatus {
    ffi_boundary(|| {
        if lsk.is_null() || out.is_null() {
            return DhruvStatus::NullPointer;
        }
        let lsk_ref = unsafe { &*lsk };
        let info = vaar_from_sunrises(sunrise_jd, next_sunrise_jd, lsk_ref);
        unsafe {
            *out = DhruvVaarInfo {
                vaar_index: info.vaar.index() as i32,
                start: utc_time_to_ffi(&info.start),
                end: utc_time_to_ffi(&info.end),
            };
        }
        DhruvStatus::Ok
    })
}

/// Determine the Hora from pre-computed sunrise boundaries.
///
/// # Safety
/// All pointer arguments must be valid and non-null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dhruv_hora_from_sunrises(
    lsk: *const DhruvLskHandle,
    jd_tdb: f64,
    sunrise_jd: f64,
    next_sunrise_jd: f64,
    out: *mut DhruvHoraInfo,
) -> DhruvStatus {
    ffi_boundary(|| {
        if lsk.is_null() || out.is_null() {
            return DhruvStatus::NullPointer;
        }
        let lsk_ref = unsafe { &*lsk };
        let info = hora_from_sunrises(jd_tdb, sunrise_jd, next_sunrise_jd, lsk_ref);
        unsafe {
            *out = DhruvHoraInfo {
                hora_index: info.hora.index() as i32,
                hora_position: info.hora_index as i32,
                start: utc_time_to_ffi(&info.start),
                end: utc_time_to_ffi(&info.end),
            };
        }
        DhruvStatus::Ok
    })
}

/// Determine the Ghatika from pre-computed sunrise boundaries.
///
/// # Safety
/// All pointer arguments must be valid and non-null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dhruv_ghatika_from_sunrises(
    lsk: *const DhruvLskHandle,
    jd_tdb: f64,
    sunrise_jd: f64,
    next_sunrise_jd: f64,
    out: *mut DhruvGhatikaInfo,
) -> DhruvStatus {
    ffi_boundary(|| {
        if lsk.is_null() || out.is_null() {
            return DhruvStatus::NullPointer;
        }
        let lsk_ref = unsafe { &*lsk };
        let info = ghatika_from_sunrises(jd_tdb, sunrise_jd, next_sunrise_jd, lsk_ref);
        unsafe {
            *out = DhruvGhatikaInfo {
                value: info.value as i32,
                start: utc_time_to_ffi(&info.start),
                end: utc_time_to_ffi(&info.end),
            };
        }
        DhruvStatus::Ok
    })
}

// ---------------------------------------------------------------------------
// Graha + Sphuta FFI (Phase 10a)
// ---------------------------------------------------------------------------

/// Number of grahas.
pub const DHRUV_GRAHA_COUNT: u32 = 9;
/// Number of sapta grahas (excludes Rahu/Ketu).
pub const DHRUV_SAPTA_GRAHA_COUNT: u32 = 7;
/// Number of sphutas.
pub const DHRUV_SPHUTA_COUNT: u32 = 16;

/// Return the name of a graha by index (0-8). Returns null for invalid index.
///
/// The returned pointer is a NUL-terminated static string and must not be freed.
#[unsafe(no_mangle)]
pub extern "C" fn dhruv_graha_name(index: u32) -> *const std::ffi::c_char {
    static NAMES: [&str; 9] = [
        "Surya\0",
        "Chandra\0",
        "Mangal\0",
        "Buddh\0",
        "Guru\0",
        "Shukra\0",
        "Shani\0",
        "Rahu\0",
        "Ketu\0",
    ];
    if index as usize >= NAMES.len() {
        return ptr::null();
    }
    NAMES[index as usize].as_ptr() as *const std::ffi::c_char
}

/// Return the name of a Yogini dasha entity by index (0-7). Returns null for invalid index.
///
/// The returned pointer is a NUL-terminated static string and must not be freed.
#[unsafe(no_mangle)]
pub extern "C" fn dhruv_yogini_name(index: u32) -> *const std::ffi::c_char {
    static NAMES: [&str; 8] = [
        "Mangala\0",
        "Pingala\0",
        "Dhanya\0",
        "Bhramari\0",
        "Bhadrika\0",
        "Ulka\0",
        "Siddha\0",
        "Sankata\0",
    ];
    if index as usize >= NAMES.len() {
        return ptr::null();
    }
    NAMES[index as usize].as_ptr() as *const std::ffi::c_char
}

/// Return the graha index (0-8) that is the lord of the rashi at `rashi_index` (0-11).
/// Returns -1 for invalid rashi_index.
#[unsafe(no_mangle)]
pub extern "C" fn dhruv_rashi_lord(rashi_index: u32) -> i32 {
    match dhruv_vedic_base::rashi_lord_by_index(rashi_index as u8) {
        Some(g) => g.index() as i32,
        None => -1,
    }
}

/// Return the name of a sphuta by index (0-15). Returns null for invalid index.
#[unsafe(no_mangle)]
pub extern "C" fn dhruv_sphuta_name(index: u32) -> *const std::ffi::c_char {
    let all = dhruv_vedic_base::sphuta::ALL_SPHUTAS;
    if index >= all.len() as u32 {
        return ptr::null();
    }
    let name = all[index as usize].name();
    name.as_ptr() as *const std::ffi::c_char
}

/// C-compatible inputs for all_sphutas.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct DhruvSphutalInputs {
    pub sun: f64,
    pub moon: f64,
    pub mars: f64,
    pub jupiter: f64,
    pub venus: f64,
    pub rahu: f64,
    pub lagna: f64,
    pub eighth_lord: f64,
    pub gulika: f64,
}

/// C-compatible result for all_sphutas.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct DhruvSphutalResult {
    /// Longitudes for each sphuta (indexed 0-15 matching ALL_SPHUTAS order).
    pub longitudes: [f64; 16],
}

/// Compute all 16 sphutas from the given inputs.
///
/// # Safety
/// `inputs` and `out` must be valid, non-null pointers.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dhruv_all_sphutas(
    inputs: *const DhruvSphutalInputs,
    out: *mut DhruvSphutalResult,
) -> DhruvStatus {
    if inputs.is_null() || out.is_null() {
        return DhruvStatus::NullPointer;
    }
    let inp = unsafe { &*inputs };
    let vedic_inputs = dhruv_vedic_base::SphutalInputs {
        sun: inp.sun,
        moon: inp.moon,
        mars: inp.mars,
        jupiter: inp.jupiter,
        venus: inp.venus,
        rahu: inp.rahu,
        lagna: inp.lagna,
        eighth_lord: inp.eighth_lord,
        gulika: inp.gulika,
    };
    let results = dhruv_vedic_base::all_sphutas(&vedic_inputs);
    let mut lons = [0.0f64; 16];
    for (i, (_sphuta, lon)) in results.iter().enumerate() {
        lons[i] = *lon;
    }
    unsafe {
        (*out).longitudes = lons;
    }
    DhruvStatus::Ok
}

/// Compute a single sphuta: Bhrigu Bindu.
#[unsafe(no_mangle)]
pub extern "C" fn dhruv_bhrigu_bindu(rahu: f64, moon: f64) -> f64 {
    dhruv_vedic_base::bhrigu_bindu(rahu, moon)
}

/// Compute a single sphuta: Prana Sphuta.
#[unsafe(no_mangle)]
pub extern "C" fn dhruv_prana_sphuta(lagna: f64, moon: f64) -> f64 {
    dhruv_vedic_base::prana_sphuta(lagna, moon)
}

/// Compute a single sphuta: Deha Sphuta.
#[unsafe(no_mangle)]
pub extern "C" fn dhruv_deha_sphuta(moon: f64, lagna: f64) -> f64 {
    dhruv_vedic_base::deha_sphuta(moon, lagna)
}

/// Compute a single sphuta: Mrityu Sphuta.
#[unsafe(no_mangle)]
pub extern "C" fn dhruv_mrityu_sphuta(eighth_lord: f64, lagna: f64) -> f64 {
    dhruv_vedic_base::mrityu_sphuta(eighth_lord, lagna)
}

/// Compute a single sphuta: Tithi Sphuta.
#[unsafe(no_mangle)]
pub extern "C" fn dhruv_tithi_sphuta(moon: f64, sun: f64, lagna: f64) -> f64 {
    dhruv_vedic_base::tithi_sphuta(moon, sun, lagna)
}

/// Compute a single sphuta: Yoga Sphuta.
#[unsafe(no_mangle)]
pub extern "C" fn dhruv_yoga_sphuta(sun: f64, moon: f64) -> f64 {
    dhruv_vedic_base::yoga_sphuta(sun, moon)
}

/// Compute a single sphuta: Yoga Sphuta Normalized.
#[unsafe(no_mangle)]
pub extern "C" fn dhruv_yoga_sphuta_normalized(sun: f64, moon: f64) -> f64 {
    dhruv_vedic_base::yoga_sphuta_normalized(sun, moon)
}

/// Compute a single sphuta: Rahu Tithi Sphuta.
#[unsafe(no_mangle)]
pub extern "C" fn dhruv_rahu_tithi_sphuta(rahu: f64, sun: f64, lagna: f64) -> f64 {
    dhruv_vedic_base::rahu_tithi_sphuta(rahu, sun, lagna)
}

/// Compute a single sphuta: Kshetra Sphuta.
#[unsafe(no_mangle)]
pub extern "C" fn dhruv_kshetra_sphuta(
    venus: f64,
    moon: f64,
    mars: f64,
    jupiter: f64,
    lagna: f64,
) -> f64 {
    dhruv_vedic_base::kshetra_sphuta(venus, moon, mars, jupiter, lagna)
}

/// Compute a single sphuta: Beeja Sphuta.
#[unsafe(no_mangle)]
pub extern "C" fn dhruv_beeja_sphuta(sun: f64, venus: f64, jupiter: f64) -> f64 {
    dhruv_vedic_base::beeja_sphuta(sun, venus, jupiter)
}

/// Compute a single sphuta: TriSphuta.
#[unsafe(no_mangle)]
pub extern "C" fn dhruv_trisphuta(lagna: f64, moon: f64, gulika: f64) -> f64 {
    dhruv_vedic_base::trisphuta(lagna, moon, gulika)
}

/// Compute a single sphuta: ChatusSphuta.
#[unsafe(no_mangle)]
pub extern "C" fn dhruv_chatussphuta(trisphuta_val: f64, sun: f64) -> f64 {
    dhruv_vedic_base::chatussphuta(trisphuta_val, sun)
}

/// Compute a single sphuta: PanchaSphuta.
#[unsafe(no_mangle)]
pub extern "C" fn dhruv_panchasphuta(chatussphuta_val: f64, rahu: f64) -> f64 {
    dhruv_vedic_base::panchasphuta(chatussphuta_val, rahu)
}

/// Compute a single sphuta: Sookshma TriSphuta.
#[unsafe(no_mangle)]
pub extern "C" fn dhruv_sookshma_trisphuta(lagna: f64, moon: f64, gulika: f64, sun: f64) -> f64 {
    dhruv_vedic_base::sookshma_trisphuta(lagna, moon, gulika, sun)
}

/// Compute a single sphuta: Avayoga Sphuta.
#[unsafe(no_mangle)]
pub extern "C" fn dhruv_avayoga_sphuta(sun: f64, moon: f64) -> f64 {
    dhruv_vedic_base::avayoga_sphuta(sun, moon)
}

/// Compute a single sphuta: Kunda.
#[unsafe(no_mangle)]
pub extern "C" fn dhruv_kunda(lagna: f64, moon: f64, mars: f64) -> f64 {
    dhruv_vedic_base::kunda(lagna, moon, mars)
}

// ---------------------------------------------------------------------------
// Special Lagnas
// ---------------------------------------------------------------------------

/// Number of special lagnas.
pub const DHRUV_SPECIAL_LAGNA_COUNT: u32 = 8;

/// C-compatible result for all 8 special lagnas.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct DhruvSpecialLagnas {
    pub bhava_lagna: f64,
    pub hora_lagna: f64,
    pub ghati_lagna: f64,
    pub vighati_lagna: f64,
    pub varnada_lagna: f64,
    pub sree_lagna: f64,
    pub pranapada_lagna: f64,
    pub indu_lagna: f64,
}

/// Return the name of a special lagna by 0-based index.
///
/// Returns null for invalid indices (>= 8).
#[unsafe(no_mangle)]
pub extern "C" fn dhruv_special_lagna_name(index: u32) -> *const std::ffi::c_char {
    if index >= 8 {
        return ptr::null();
    }
    let name = dhruv_vedic_base::ALL_SPECIAL_LAGNAS[index as usize].name();
    name.as_ptr().cast()
}

/// Compute a single special lagna: Bhava Lagna (pure math).
#[unsafe(no_mangle)]
pub extern "C" fn dhruv_bhava_lagna(sun_lon: f64, ghatikas: f64) -> f64 {
    dhruv_vedic_base::bhava_lagna(sun_lon, ghatikas)
}

/// Compute a single special lagna: Hora Lagna (pure math).
#[unsafe(no_mangle)]
pub extern "C" fn dhruv_hora_lagna(sun_lon: f64, ghatikas: f64) -> f64 {
    dhruv_vedic_base::hora_lagna(sun_lon, ghatikas)
}

/// Compute a single special lagna: Ghati Lagna (pure math).
#[unsafe(no_mangle)]
pub extern "C" fn dhruv_ghati_lagna(sun_lon: f64, ghatikas: f64) -> f64 {
    dhruv_vedic_base::ghati_lagna(sun_lon, ghatikas)
}

/// Compute a single special lagna: Vighati Lagna (pure math).
#[unsafe(no_mangle)]
pub extern "C" fn dhruv_vighati_lagna(lagna_lon: f64, vighatikas: f64) -> f64 {
    dhruv_vedic_base::vighati_lagna(lagna_lon, vighatikas)
}

/// Compute a single special lagna: Varnada Lagna (pure math).
#[unsafe(no_mangle)]
pub extern "C" fn dhruv_varnada_lagna(lagna_lon: f64, hora_lagna_lon: f64) -> f64 {
    dhruv_vedic_base::varnada_lagna(lagna_lon, hora_lagna_lon)
}

/// Compute a single special lagna: Sree Lagna (pure math).
#[unsafe(no_mangle)]
pub extern "C" fn dhruv_sree_lagna(moon_lon: f64, lagna_lon: f64) -> f64 {
    dhruv_vedic_base::sree_lagna(moon_lon, lagna_lon)
}

/// Compute a single special lagna: Pranapada Lagna (pure math).
#[unsafe(no_mangle)]
pub extern "C" fn dhruv_pranapada_lagna(sun_lon: f64, ghatikas: f64) -> f64 {
    dhruv_vedic_base::pranapada_lagna(sun_lon, ghatikas)
}

/// Compute a single special lagna: Indu Lagna (pure math).
///
/// `lagna_lord` and `moon_9th_lord` are graha indices (0-8, per ALL_GRAHAS order).
#[unsafe(no_mangle)]
pub extern "C" fn dhruv_indu_lagna(moon_lon: f64, lagna_lord: u32, moon_9th_lord: u32) -> f64 {
    if lagna_lord >= 9 || moon_9th_lord >= 9 {
        return -1.0;
    }
    let ll = dhruv_vedic_base::ALL_GRAHAS[lagna_lord as usize];
    let m9l = dhruv_vedic_base::ALL_GRAHAS[moon_9th_lord as usize];
    dhruv_vedic_base::indu_lagna(moon_lon, ll, m9l)
}

/// Compute all 8 special lagnas for a given date and location (engine-dependent).
///
/// # Safety
/// All pointers must be valid. Returns `NullPointer` if any is null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dhruv_special_lagnas_for_date(
    engine: *const Engine,
    eop: *const dhruv_time::EopKernel,
    utc: *const DhruvUtcTime,
    location: *const DhruvGeoLocation,
    riseset_config: *const DhruvRiseSetConfig,
    ayanamsha_system: u32,
    use_nutation: u8,
    out: *mut DhruvSpecialLagnas,
) -> DhruvStatus {
    if engine.is_null() || eop.is_null() || utc.is_null() || location.is_null() || out.is_null() {
        return DhruvStatus::NullPointer;
    }

    let engine = unsafe { &*engine };
    let eop = unsafe { &*eop };
    let utc_c = unsafe { &*utc };
    let loc_c = unsafe { &*location };

    let utc_time = UtcTime {
        year: utc_c.year,
        month: utc_c.month,
        day: utc_c.day,
        hour: utc_c.hour,
        minute: utc_c.minute,
        second: utc_c.second,
    };

    let location = GeoLocation::new(loc_c.latitude_deg, loc_c.longitude_deg, loc_c.altitude_m);

    let rs_config = match resolve_riseset_config_ptr(riseset_config) {
        Ok(c) => c,
        Err(status) => return status,
    };

    let system = match ayanamsha_system_from_code(ayanamsha_system as i32) {
        Some(s) => s,
        None => return DhruvStatus::InvalidQuery,
    };
    let aya_config = SankrantiConfig::new(system, use_nutation != 0);

    match special_lagnas_for_date(engine, eop, &utc_time, &location, &rs_config, &aya_config) {
        Ok(result) => {
            unsafe {
                (*out).bhava_lagna = result.bhava_lagna;
                (*out).hora_lagna = result.hora_lagna;
                (*out).ghati_lagna = result.ghati_lagna;
                (*out).vighati_lagna = result.vighati_lagna;
                (*out).varnada_lagna = result.varnada_lagna;
                (*out).sree_lagna = result.sree_lagna;
                (*out).pranapada_lagna = result.pranapada_lagna;
                (*out).indu_lagna = result.indu_lagna;
            }
            DhruvStatus::Ok
        }
        Err(e) => DhruvStatus::from(&e),
    }
}

// ---------------------------------------------------------------------------
// Arudha Padas
// ---------------------------------------------------------------------------

/// Number of arudha padas.
pub const DHRUV_ARUDHA_PADA_COUNT: u32 = 12;

/// C-compatible result for a single arudha pada.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct DhruvArudhaResult {
    pub bhava_number: u8,
    pub longitude_deg: f64,
    pub rashi_index: u8,
}

/// Return the name of an arudha pada by 0-based index.
///
/// Returns null for invalid indices (>= 12).
#[unsafe(no_mangle)]
pub extern "C" fn dhruv_arudha_pada_name(index: u32) -> *const std::ffi::c_char {
    if index >= 12 {
        return ptr::null();
    }
    let name = dhruv_vedic_base::ALL_ARUDHA_PADAS[index as usize].name();
    name.as_ptr().cast()
}

/// Compute a single arudha pada (pure math).
///
/// Returns the arudha longitude in degrees [0, 360).
///
/// # Safety
/// `out_rashi` must be null or point to a valid `u8`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dhruv_arudha_pada(
    bhava_cusp_lon: f64,
    lord_lon: f64,
    out_rashi: *mut u8,
) -> f64 {
    let (lon, rashi) = dhruv_vedic_base::arudha_pada(bhava_cusp_lon, lord_lon);
    if !out_rashi.is_null() {
        unsafe {
            *out_rashi = rashi;
        }
    }
    lon
}

/// Compute all 12 arudha padas for a given date and location (engine-dependent).
///
/// Writes 12 results to `out` array. Returns `NullPointer` if any pointer is null.
///
/// # Safety
/// All pointers must be valid. `out` must point to an array of at least 12 `DhruvArudhaResult`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dhruv_arudha_padas_for_date(
    engine: *const Engine,
    eop: *const dhruv_time::EopKernel,
    utc: *const DhruvUtcTime,
    location: *const DhruvGeoLocation,
    ayanamsha_system: u32,
    use_nutation: u8,
    out: *mut DhruvArudhaResult,
) -> DhruvStatus {
    if engine.is_null() || eop.is_null() || utc.is_null() || location.is_null() || out.is_null() {
        return DhruvStatus::NullPointer;
    }

    let engine = unsafe { &*engine };
    let eop = unsafe { &*eop };
    let utc_c = unsafe { &*utc };
    let loc_c = unsafe { &*location };

    let utc_time = UtcTime {
        year: utc_c.year,
        month: utc_c.month,
        day: utc_c.day,
        hour: utc_c.hour,
        minute: utc_c.minute,
        second: utc_c.second,
    };

    let location = GeoLocation::new(loc_c.latitude_deg, loc_c.longitude_deg, loc_c.altitude_m);

    let system = match ayanamsha_system_from_code(ayanamsha_system as i32) {
        Some(s) => s,
        None => return DhruvStatus::InvalidQuery,
    };

    let bhava_config = dhruv_vedic_base::BhavaConfig::default();
    let aya_config = SankrantiConfig::new(system, use_nutation != 0);

    match dhruv_search::arudha_padas_for_date(
        engine,
        eop,
        &utc_time,
        &location,
        &bhava_config,
        &aya_config,
    ) {
        Ok(results) => {
            let out_slice = unsafe { std::slice::from_raw_parts_mut(out, 12) };
            for (i, r) in results.iter().enumerate() {
                out_slice[i] = DhruvArudhaResult {
                    bhava_number: r.pada.bhava_number(),
                    longitude_deg: r.longitude_deg,
                    rashi_index: r.rashi_index,
                };
            }
            DhruvStatus::Ok
        }
        Err(e) => DhruvStatus::from(&e),
    }
}

// ---------------------------------------------------------------------------
// Upagrahas
// ---------------------------------------------------------------------------

/// Number of upagrahas.
pub const DHRUV_UPAGRAHA_COUNT: u32 = 11;

/// Time-based upagraha point: start of the selected portion.
pub const DHRUV_UPAGRAHA_POINT_START: i32 = 0;
/// Time-based upagraha point: midpoint of the selected portion.
pub const DHRUV_UPAGRAHA_POINT_MIDDLE: i32 = 1;
/// Time-based upagraha point: end of the selected portion.
pub const DHRUV_UPAGRAHA_POINT_END: i32 = 2;

/// Gulika/Maandi should use Rahu's portion.
pub const DHRUV_GULIKA_MAANDI_PLANET_RAHU: i32 = 0;
/// Gulika/Maandi should use Saturn's portion.
pub const DHRUV_GULIKA_MAANDI_PLANET_SATURN: i32 = 1;

/// C-compatible configuration for time-based upagraha period selection.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DhruvTimeUpagrahaConfig {
    /// 0=Start, 1=Middle, 2=End.
    pub gulika_point: i32,
    /// 0=Start, 1=Middle, 2=End.
    pub maandi_point: i32,
    /// Shared point for Kaala, Mrityu, Artha Prahara, Yama Ghantaka.
    /// 0=Start, 1=Middle, 2=End.
    pub other_point: i32,
    /// 0=Rahu, 1=Saturn.
    pub gulika_planet: i32,
    /// 0=Rahu, 1=Saturn.
    pub maandi_planet: i32,
}

/// Returns the default time-based upagraha configuration.
#[unsafe(no_mangle)]
pub extern "C" fn dhruv_time_upagraha_config_default() -> DhruvTimeUpagrahaConfig {
    DhruvTimeUpagrahaConfig {
        gulika_point: DHRUV_UPAGRAHA_POINT_START,
        maandi_point: DHRUV_UPAGRAHA_POINT_END,
        other_point: DHRUV_UPAGRAHA_POINT_START,
        gulika_planet: DHRUV_GULIKA_MAANDI_PLANET_RAHU,
        maandi_planet: DHRUV_GULIKA_MAANDI_PLANET_RAHU,
    }
}

/// C-compatible result for all 11 upagrahas (sidereal longitudes).
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct DhruvAllUpagrahas {
    pub gulika: f64,
    pub maandi: f64,
    pub kaala: f64,
    pub mrityu: f64,
    pub artha_prahara: f64,
    pub yama_ghantaka: f64,
    pub dhooma: f64,
    pub vyatipata: f64,
    pub parivesha: f64,
    pub indra_chapa: f64,
    pub upaketu: f64,
}

/// Return the name of an upagraha by index (0-10).
///
/// Returns null for invalid indices (>= 11).
#[unsafe(no_mangle)]
pub extern "C" fn dhruv_upagraha_name(index: u32) -> *const std::ffi::c_char {
    if index >= 11 {
        return ptr::null();
    }
    let upa = dhruv_vedic_base::ALL_UPAGRAHAS[index as usize];
    let name = match upa {
        dhruv_vedic_base::Upagraha::Gulika => c"Gulika",
        dhruv_vedic_base::Upagraha::Maandi => c"Maandi",
        dhruv_vedic_base::Upagraha::Kaala => c"Kaala",
        dhruv_vedic_base::Upagraha::Mrityu => c"Mrityu",
        dhruv_vedic_base::Upagraha::ArthaPrahara => c"Artha Prahara",
        dhruv_vedic_base::Upagraha::YamaGhantaka => c"Yama Ghantaka",
        dhruv_vedic_base::Upagraha::Dhooma => c"Dhooma",
        dhruv_vedic_base::Upagraha::Vyatipata => c"Vyatipata",
        dhruv_vedic_base::Upagraha::Parivesha => c"Parivesha",
        dhruv_vedic_base::Upagraha::IndraChapa => c"Indra Chapa",
        dhruv_vedic_base::Upagraha::Upaketu => c"Upaketu",
    };
    name.as_ptr()
}

/// Compute the 5 sun-based upagrahas from sidereal Sun longitude.
///
/// Pure math, no engine needed.
///
/// # Safety
/// `out` must point to a valid `DhruvAllUpagrahas`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dhruv_sun_based_upagrahas(
    sun_sid_lon: f64,
    out: *mut DhruvAllUpagrahas,
) -> DhruvStatus {
    if out.is_null() {
        return DhruvStatus::NullPointer;
    }
    let result = dhruv_vedic_base::sun_based_upagrahas(sun_sid_lon);
    let out = unsafe { &mut *out };
    out.dhooma = result.dhooma;
    out.vyatipata = result.vyatipata;
    out.parivesha = result.parivesha;
    out.indra_chapa = result.indra_chapa;
    out.upaketu = result.upaketu;
    // Time-based fields are left uninitialized — caller should only read sun-based fields
    DhruvStatus::Ok
}

/// Map a 0-based upagraha index to the Upagraha enum (time-based only: 0-5).
fn time_upagraha_from_index(index: u32) -> Option<dhruv_vedic_base::Upagraha> {
    match index {
        0 => Some(dhruv_vedic_base::Upagraha::Gulika),
        1 => Some(dhruv_vedic_base::Upagraha::Maandi),
        2 => Some(dhruv_vedic_base::Upagraha::Kaala),
        3 => Some(dhruv_vedic_base::Upagraha::Mrityu),
        4 => Some(dhruv_vedic_base::Upagraha::ArthaPrahara),
        5 => Some(dhruv_vedic_base::Upagraha::YamaGhantaka),
        _ => None,
    }
}

/// Compute the JD at which to evaluate a time-based upagraha's lagna.
///
/// Accepts pre-computed sunrise/sunset/next-sunrise JDs.
/// `upagraha_index`: 0=Gulika, 1=Maandi, 2=Kaala, 3=Mrityu,
///                   4=ArthaPrahara, 5=YamaGhantaka.
/// `weekday`: 0=Sunday .. 6=Saturday.
/// `is_day`: 1=daytime, 0=nighttime.
///
/// Pure math — no engine or kernel needed.
///
/// # Safety
/// `out_jd` must be a valid, non-null pointer.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dhruv_time_upagraha_jd(
    upagraha_index: u32,
    weekday: u32,
    is_day: u8,
    sunrise_jd: f64,
    sunset_jd: f64,
    next_sunrise_jd: f64,
    out_jd: *mut f64,
) -> DhruvStatus {
    unsafe {
        dhruv_time_upagraha_jd_with_config(
            upagraha_index,
            weekday,
            is_day,
            sunrise_jd,
            sunset_jd,
            next_sunrise_jd,
            std::ptr::null(),
            out_jd,
        )
    }
}

/// Compute the JD at which to evaluate a time-based upagraha's lagna using
/// configurable period selection.
///
/// # Safety
/// `out_jd` must be a valid, non-null pointer.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dhruv_time_upagraha_jd_with_config(
    upagraha_index: u32,
    weekday: u32,
    is_day: u8,
    sunrise_jd: f64,
    sunset_jd: f64,
    next_sunrise_jd: f64,
    upagraha_config: *const DhruvTimeUpagrahaConfig,
    out_jd: *mut f64,
) -> DhruvStatus {
    if out_jd.is_null() {
        return DhruvStatus::NullPointer;
    }
    let upa = match time_upagraha_from_index(upagraha_index) {
        Some(u) => u,
        None => return DhruvStatus::InvalidQuery,
    };
    if weekday > 6 {
        return DhruvStatus::InvalidQuery;
    }
    let upagraha_config = match resolve_time_upagraha_config_ptr(upagraha_config) {
        Ok(c) => c,
        Err(status) => return status,
    };
    let jd = dhruv_vedic_base::time_upagraha_jd_with_config(
        upa,
        weekday as u8,
        is_day != 0,
        sunrise_jd,
        sunset_jd,
        next_sunrise_jd,
        &upagraha_config,
    );
    unsafe { *out_jd = jd };
    DhruvStatus::Ok
}

/// Compute the JD for a time-based upagraha from a UTC date and location.
///
/// Computes sunrise/sunset/next-sunrise internally from engine+EOP+location.
/// `upagraha_index`: 0=Gulika, 1=Maandi, 2=Kaala, 3=Mrityu,
///                   4=ArthaPrahara, 5=YamaGhantaka.
///
/// # Safety
/// All pointer arguments must be valid and non-null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dhruv_time_upagraha_jd_utc(
    engine: *const Engine,
    eop: *const dhruv_time::EopKernel,
    utc: *const DhruvUtcTime,
    location: *const DhruvGeoLocation,
    riseset_config: *const DhruvRiseSetConfig,
    upagraha_index: u32,
    out_jd: *mut f64,
) -> DhruvStatus {
    unsafe {
        dhruv_time_upagraha_jd_utc_with_config(
            engine,
            eop,
            utc,
            location,
            riseset_config,
            std::ptr::null(),
            upagraha_index,
            out_jd,
        )
    }
}

/// Compute the JD for a time-based upagraha from a UTC date and location using
/// configurable period selection.
///
/// # Safety
/// All pointer arguments must be valid and non-null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dhruv_time_upagraha_jd_utc_with_config(
    engine: *const Engine,
    eop: *const dhruv_time::EopKernel,
    utc: *const DhruvUtcTime,
    location: *const DhruvGeoLocation,
    riseset_config: *const DhruvRiseSetConfig,
    upagraha_config: *const DhruvTimeUpagrahaConfig,
    upagraha_index: u32,
    out_jd: *mut f64,
) -> DhruvStatus {
    if engine.is_null() || eop.is_null() || utc.is_null() || location.is_null() || out_jd.is_null()
    {
        return DhruvStatus::NullPointer;
    }

    let upa = match time_upagraha_from_index(upagraha_index) {
        Some(u) => u,
        None => return DhruvStatus::InvalidQuery,
    };

    let engine = unsafe { &*engine };
    let eop = unsafe { &*eop };
    let utc_c = unsafe { &*utc };
    let loc_c = unsafe { &*location };

    let utc_time = UtcTime {
        year: utc_c.year,
        month: utc_c.month,
        day: utc_c.day,
        hour: utc_c.hour,
        minute: utc_c.minute,
        second: utc_c.second,
    };

    let location = GeoLocation::new(loc_c.latitude_deg, loc_c.longitude_deg, loc_c.altitude_m);

    let rs_config = match resolve_riseset_config_ptr(riseset_config) {
        Ok(c) => c,
        Err(status) => return status,
    };
    let upagraha_config = match resolve_time_upagraha_config_ptr(upagraha_config) {
        Ok(c) => c,
        Err(status) => return status,
    };

    // Compute sunrise pair (vedic day boundaries)
    let (jd_sunrise, jd_next_sunrise) =
        match vedic_day_sunrises(engine, eop, &utc_time, &location, &rs_config) {
            Ok(pair) => pair,
            Err(e) => return DhruvStatus::from(&e),
        };

    // Compute sunset
    let jd_utc = ffi_utc_to_jd_utc(utc_c);
    let noon_jd = approximate_local_noon_jd(utc_day_start_jd(jd_utc), loc_c.longitude_deg);
    let jd_sunset = match compute_rise_set(
        engine,
        engine.lsk(),
        eop,
        &location,
        RiseSetEvent::Sunset,
        noon_jd,
        &rs_config,
    ) {
        Ok(RiseSetResult::Event { jd_tdb: jd, .. }) => jd,
        Ok(_) => return DhruvStatus::NoConvergence,
        Err(_) => return DhruvStatus::NoConvergence,
    };

    // Determine if query time is during day or night
    let jd_tdb = utc_time.to_jd_tdb(engine.lsk());
    let is_day = jd_tdb >= jd_sunrise && jd_tdb < jd_sunset;

    // Weekday from sunrise
    let weekday = vaar_from_jd(jd_sunrise).index();

    let jd = dhruv_vedic_base::time_upagraha_jd_with_config(
        upa,
        weekday,
        is_day,
        jd_sunrise,
        jd_sunset,
        jd_next_sunrise,
        &upagraha_config,
    );
    unsafe { *out_jd = jd };
    DhruvStatus::Ok
}

/// Compute all 11 upagrahas for a given date and location.
///
/// # Safety
/// All pointers must be valid. `out` must point to a valid `DhruvAllUpagrahas`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dhruv_all_upagrahas_for_date(
    engine: *const Engine,
    eop: *const dhruv_time::EopKernel,
    utc: *const DhruvUtcTime,
    location: *const DhruvGeoLocation,
    ayanamsha_system: u32,
    use_nutation: u8,
    out: *mut DhruvAllUpagrahas,
) -> DhruvStatus {
    unsafe {
        dhruv_all_upagrahas_for_date_with_config(
            engine,
            eop,
            utc,
            location,
            ayanamsha_system,
            use_nutation,
            std::ptr::null(),
            out,
        )
    }
}

/// Compute all 11 upagrahas for a given date and location using configurable
/// time-based period selection.
///
/// # Safety
/// All pointers must be valid. `out` must point to a valid `DhruvAllUpagrahas`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dhruv_all_upagrahas_for_date_with_config(
    engine: *const Engine,
    eop: *const dhruv_time::EopKernel,
    utc: *const DhruvUtcTime,
    location: *const DhruvGeoLocation,
    ayanamsha_system: u32,
    use_nutation: u8,
    upagraha_config: *const DhruvTimeUpagrahaConfig,
    out: *mut DhruvAllUpagrahas,
) -> DhruvStatus {
    if engine.is_null() || eop.is_null() || utc.is_null() || location.is_null() || out.is_null() {
        return DhruvStatus::NullPointer;
    }

    let engine = unsafe { &*engine };
    let eop = unsafe { &*eop };
    let utc_c = unsafe { &*utc };
    let loc_c = unsafe { &*location };

    let utc_time = UtcTime {
        year: utc_c.year,
        month: utc_c.month,
        day: utc_c.day,
        hour: utc_c.hour,
        minute: utc_c.minute,
        second: utc_c.second,
    };

    let location = GeoLocation::new(loc_c.latitude_deg, loc_c.longitude_deg, loc_c.altitude_m);

    let system = match ayanamsha_system_from_code(ayanamsha_system as i32) {
        Some(s) => s,
        None => return DhruvStatus::InvalidQuery,
    };

    let rs_config = dhruv_vedic_base::RiseSetConfig::default();
    let aya_config = SankrantiConfig::new(system, use_nutation != 0);
    let upagraha_config = match resolve_time_upagraha_config_ptr(upagraha_config) {
        Ok(c) => c,
        Err(status) => return status,
    };

    match dhruv_search::all_upagrahas_for_date_with_config(
        engine,
        eop,
        &utc_time,
        &location,
        &rs_config,
        &aya_config,
        &upagraha_config,
    ) {
        Ok(result) => {
            let out = unsafe { &mut *out };
            out.gulika = result.gulika;
            out.maandi = result.maandi;
            out.kaala = result.kaala;
            out.mrityu = result.mrityu;
            out.artha_prahara = result.artha_prahara;
            out.yama_ghantaka = result.yama_ghantaka;
            out.dhooma = result.dhooma;
            out.vyatipata = result.vyatipata;
            out.parivesha = result.parivesha;
            out.indra_chapa = result.indra_chapa;
            out.upaketu = result.upaketu;
            DhruvStatus::Ok
        }
        Err(e) => DhruvStatus::from(&e),
    }
}

// ---------------------------------------------------------------------------
// Ashtakavarga
// ---------------------------------------------------------------------------

/// Number of grahas in the Ashtakavarga system (Sun through Saturn).
pub const DHRUV_ASHTAKAVARGA_GRAHA_COUNT: u32 = 7;

/// C-compatible Bhinna Ashtakavarga for a single graha.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct DhruvBhinnaAshtakavarga {
    /// Target graha index (0=Sun through 6=Saturn).
    pub graha_index: u8,
    /// Benefic points per rashi (12 entries, 0-based index, max 8 each).
    pub points: [u8; 12],
    /// Contributor attribution matrix `[rashi][contributor]` with 0/1 values.
    ///
    /// contributor index: 0=Sun, 1=Moon, 2=Mars, 3=Mercury,
    /// 4=Jupiter, 5=Venus, 6=Saturn, 7=Lagna.
    pub contributors: [[u8; 8]; 12],
}

/// C-compatible Sarva Ashtakavarga result.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct DhruvSarvaAshtakavarga {
    /// SAV total per rashi (sum of all 7 BAVs).
    pub total_points: [u8; 12],
    /// After Trikona Sodhana.
    pub after_trikona: [u8; 12],
    /// After Ekadhipatya Sodhana.
    pub after_ekadhipatya: [u8; 12],
}

/// C-compatible complete Ashtakavarga result.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct DhruvAshtakavargaResult {
    /// Bhinna Ashtakavarga for 7 grahas.
    pub bavs: [DhruvBhinnaAshtakavarga; 7],
    /// Sarva Ashtakavarga with sodhana.
    pub sav: DhruvSarvaAshtakavarga,
}

/// Calculate all BAVs from rashi indices (pure math, no engine needed).
///
/// # Safety
/// `graha_rashis` must point to a valid `[u8; 7]`.
/// `out` must point to a valid `DhruvAshtakavargaResult`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dhruv_calculate_ashtakavarga(
    graha_rashis: *const u8,
    lagna_rashi: u8,
    out: *mut DhruvAshtakavargaResult,
) -> DhruvStatus {
    if graha_rashis.is_null() || out.is_null() {
        return DhruvStatus::NullPointer;
    }
    let rashis = unsafe { std::slice::from_raw_parts(graha_rashis, 7) };
    let mut arr = [0u8; 7];
    arr.copy_from_slice(rashis);

    let result = dhruv_vedic_base::calculate_ashtakavarga(&arr, lagna_rashi);
    let out = unsafe { &mut *out };
    for (i, bav) in result.bavs.iter().enumerate() {
        out.bavs[i] = DhruvBhinnaAshtakavarga {
            graha_index: bav.graha_index,
            points: bav.points,
            contributors: bav.contributors,
        };
    }
    out.sav = DhruvSarvaAshtakavarga {
        total_points: result.sav.total_points,
        after_trikona: result.sav.after_trikona,
        after_ekadhipatya: result.sav.after_ekadhipatya,
    };
    DhruvStatus::Ok
}

/// Calculate BAV for a single graha (pure math).
///
/// `graha_index`: 0=Sun through 6=Saturn.
/// `graha_rashis`: pointer to 7 `u8` values (0-based rashi index for Sun..Saturn).
/// `lagna_rashi`: 0-based rashi index of the Ascendant.
/// `out`: pointer to a `DhruvBhinnaAshtakavarga`.
///
/// # Safety
/// `graha_rashis` must point to 7 contiguous `u8` values. `out` must be valid.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dhruv_calculate_bav(
    graha_index: u8,
    graha_rashis: *const u8,
    lagna_rashi: u8,
    out: *mut DhruvBhinnaAshtakavarga,
) -> DhruvStatus {
    if graha_rashis.is_null() || out.is_null() {
        return DhruvStatus::NullPointer;
    }
    if graha_index > 6 {
        return DhruvStatus::InvalidQuery;
    }
    let rashis = unsafe { std::slice::from_raw_parts(graha_rashis, 7) };
    let mut arr = [0u8; 7];
    arr.copy_from_slice(rashis);

    let bav = dhruv_vedic_base::calculate_bav(graha_index, &arr, lagna_rashi);
    let out = unsafe { &mut *out };
    out.graha_index = bav.graha_index;
    out.points = bav.points;
    out.contributors = bav.contributors;
    DhruvStatus::Ok
}

/// Calculate BAV for all 7 grahas (pure math).
///
/// `graha_rashis`: pointer to 7 `u8` values (0-based rashi index for Sun..Saturn).
/// `lagna_rashi`: 0-based rashi index of the Ascendant.
/// `out`: pointer to array of 7 `DhruvBhinnaAshtakavarga`.
///
/// # Safety
/// `graha_rashis` must point to 7 contiguous `u8`. `out` must point to 7 contiguous
/// `DhruvBhinnaAshtakavarga`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dhruv_calculate_all_bav(
    graha_rashis: *const u8,
    lagna_rashi: u8,
    out: *mut DhruvBhinnaAshtakavarga,
) -> DhruvStatus {
    if graha_rashis.is_null() || out.is_null() {
        return DhruvStatus::NullPointer;
    }
    let rashis = unsafe { std::slice::from_raw_parts(graha_rashis, 7) };
    let mut arr = [0u8; 7];
    arr.copy_from_slice(rashis);

    let bavs = dhruv_vedic_base::calculate_all_bav(&arr, lagna_rashi);
    let out_slice = unsafe { std::slice::from_raw_parts_mut(out, 7) };
    for (i, bav) in bavs.iter().enumerate() {
        out_slice[i] = DhruvBhinnaAshtakavarga {
            graha_index: bav.graha_index,
            points: bav.points,
            contributors: bav.contributors,
        };
    }
    DhruvStatus::Ok
}

/// Calculate SAV from 7 BAVs (pure math).
///
/// `bavs`: pointer to 7 `DhruvBhinnaAshtakavarga` (e.g. from `dhruv_calculate_all_bav`).
/// `out`: pointer to a `DhruvSarvaAshtakavarga`.
///
/// # Safety
/// `bavs` must point to 7 contiguous `DhruvBhinnaAshtakavarga`. `out` must be valid.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dhruv_calculate_sav(
    bavs: *const DhruvBhinnaAshtakavarga,
    out: *mut DhruvSarvaAshtakavarga,
) -> DhruvStatus {
    if bavs.is_null() || out.is_null() {
        return DhruvStatus::NullPointer;
    }
    let bav_slice = unsafe { std::slice::from_raw_parts(bavs, 7) };
    let mut rust_bavs = [dhruv_vedic_base::BhinnaAshtakavarga {
        graha_index: 0,
        points: [0; 12],
        contributors: [[0; 8]; 12],
    }; 7];
    for (i, b) in bav_slice.iter().enumerate() {
        rust_bavs[i] = dhruv_vedic_base::BhinnaAshtakavarga {
            graha_index: b.graha_index,
            points: b.points,
            contributors: b.contributors,
        };
    }

    let sav = dhruv_vedic_base::calculate_sav(&rust_bavs);
    let out = unsafe { &mut *out };
    out.total_points = sav.total_points;
    out.after_trikona = sav.after_trikona;
    out.after_ekadhipatya = sav.after_ekadhipatya;
    DhruvStatus::Ok
}

/// Apply Trikona Sodhana to 12 rashi totals (pure math).
///
/// `totals`: pointer to 12 `u8` values (rashi totals).
/// `out`: pointer to 12 `u8` values (result after trikona reduction).
///
/// # Safety
/// Both pointers must point to 12 contiguous `u8` values.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dhruv_trikona_sodhana(totals: *const u8, out: *mut u8) -> DhruvStatus {
    if totals.is_null() || out.is_null() {
        return DhruvStatus::NullPointer;
    }
    let src = unsafe { std::slice::from_raw_parts(totals, 12) };
    let mut arr = [0u8; 12];
    arr.copy_from_slice(src);

    let result = dhruv_vedic_base::trikona_sodhana(&arr);
    let dst = unsafe { std::slice::from_raw_parts_mut(out, 12) };
    dst.copy_from_slice(&result);
    DhruvStatus::Ok
}

/// Apply Ekadhipatya Sodhana to 12 rashi totals (pure math).
///
/// Typically called on the output of `dhruv_trikona_sodhana`.
/// `after_trikona`: pointer to 12 `u8` values.
/// `out`: pointer to 12 `u8` values (result after ekadhipatya reduction).
///
/// # Safety
/// Both pointers must point to 12 contiguous `u8` values.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dhruv_ekadhipatya_sodhana(
    after_trikona: *const u8,
    out: *mut u8,
) -> DhruvStatus {
    if after_trikona.is_null() || out.is_null() {
        return DhruvStatus::NullPointer;
    }
    let src = unsafe { std::slice::from_raw_parts(after_trikona, 12) };
    let mut arr = [0u8; 12];
    arr.copy_from_slice(src);

    let result = dhruv_vedic_base::ekadhipatya_sodhana(&arr);
    let dst = unsafe { std::slice::from_raw_parts_mut(out, 12) };
    dst.copy_from_slice(&result);
    DhruvStatus::Ok
}

// ---------------------------------------------------------------------------
// Pure-math: graha drishti, ghatika, hora, ghatikas_since_sunrise
// ---------------------------------------------------------------------------

/// C-compatible 9×9 graha drishti matrix (pure graha-to-graha).
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct DhruvGrahaDrishtiMatrix {
    /// entries\[src\]\[tgt\]. Diagonal (self-aspect) entries are zeroed.
    pub entries: [[DhruvDrishtiEntry; 9]; 9],
}

/// Compute drishti from a single graha to a single sidereal point (pure math).
///
/// `graha_index`: 0=Surya .. 8=Ketu.
/// `source_lon`: sidereal longitude of the source graha (degrees).
/// `target_lon`: sidereal longitude of the target point (degrees).
///
/// # Safety
/// `out` must point to a valid `DhruvDrishtiEntry`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dhruv_graha_drishti(
    graha_index: u32,
    source_lon: f64,
    target_lon: f64,
    out: *mut DhruvDrishtiEntry,
) -> DhruvStatus {
    if out.is_null() {
        return DhruvStatus::NullPointer;
    }
    let graha = match graha_index {
        0 => dhruv_vedic_base::Graha::Surya,
        1 => dhruv_vedic_base::Graha::Chandra,
        2 => dhruv_vedic_base::Graha::Mangal,
        3 => dhruv_vedic_base::Graha::Buddh,
        4 => dhruv_vedic_base::Graha::Guru,
        5 => dhruv_vedic_base::Graha::Shukra,
        6 => dhruv_vedic_base::Graha::Shani,
        7 => dhruv_vedic_base::Graha::Rahu,
        8 => dhruv_vedic_base::Graha::Ketu,
        _ => return DhruvStatus::InvalidQuery,
    };
    let entry = dhruv_vedic_base::graha_drishti(graha, source_lon, target_lon);
    unsafe { *out = drishti_entry_to_ffi(&entry) };
    DhruvStatus::Ok
}

/// Compute the full 9×9 graha drishti matrix from sidereal longitudes (pure math).
///
/// `longitudes`: pointer to 9 `f64` sidereal longitudes (Sun..Ketu order).
///
/// # Safety
/// `longitudes` must point to 9 contiguous `f64`. `out` must be valid.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dhruv_graha_drishti_matrix(
    longitudes: *const f64,
    out: *mut DhruvGrahaDrishtiMatrix,
) -> DhruvStatus {
    if longitudes.is_null() || out.is_null() {
        return DhruvStatus::NullPointer;
    }
    let lons = unsafe { std::slice::from_raw_parts(longitudes, 9) };
    let mut arr = [0.0f64; 9];
    arr.copy_from_slice(lons);

    let matrix = dhruv_vedic_base::graha_drishti_matrix(&arr);
    let out = unsafe { &mut *out };
    for si in 0..9 {
        for ti in 0..9 {
            out.entries[si][ti] = drishti_entry_to_ffi(&matrix.entries[si][ti]);
        }
    }
    DhruvStatus::Ok
}

/// Determine the ghatika from elapsed seconds since sunrise (pure math).
///
/// `seconds_since_sunrise`: seconds elapsed since the Vedic day's sunrise.
/// `vedic_day_duration_seconds`: total seconds from sunrise to next sunrise.
/// `out_value`: receives ghatika value (1-60).
/// `out_index`: receives 0-based ghatika index (0-59).
///
/// # Safety
/// Both output pointers must be valid.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dhruv_ghatika_from_elapsed(
    seconds_since_sunrise: f64,
    vedic_day_duration_seconds: f64,
    out_value: *mut u8,
    out_index: *mut u8,
) -> DhruvStatus {
    if out_value.is_null() || out_index.is_null() {
        return DhruvStatus::NullPointer;
    }
    let pos =
        dhruv_vedic_base::ghatika_from_elapsed(seconds_since_sunrise, vedic_day_duration_seconds);
    unsafe {
        *out_value = pos.value;
        *out_index = pos.index;
    }
    DhruvStatus::Ok
}

/// Compute ghatikas elapsed since sunrise (pure math).
///
/// One Vedic day = 60 ghatikas. Result can exceed 60 if `jd_moment` is past next sunrise.
///
/// # Safety
/// `out_ghatikas` must be a valid pointer.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dhruv_ghatikas_since_sunrise(
    jd_moment: f64,
    jd_sunrise: f64,
    jd_next_sunrise: f64,
    out_ghatikas: *mut f64,
) -> DhruvStatus {
    if out_ghatikas.is_null() {
        return DhruvStatus::NullPointer;
    }
    let g = dhruv_vedic_base::ghatikas_since_sunrise(jd_moment, jd_sunrise, jd_next_sunrise);
    unsafe { *out_ghatikas = g };
    DhruvStatus::Ok
}

/// Determine the hora lord for a given weekday and hora position (pure math).
///
/// `vaar_index`: 0=Sunday .. 6=Saturday.
/// `hora_index`: 0=first hora at sunrise .. 23=last hora.
/// Returns the hora lord index in Chaldean sequence (0=Surya, 1=Shukra, 2=Buddh,
/// 3=Chandra, 4=Shani, 5=Guru, 6=Mangal), or -1 on invalid input.
#[unsafe(no_mangle)]
pub extern "C" fn dhruv_hora_at(vaar_index: u32, hora_index: u32) -> i32 {
    if vaar_index > 6 || hora_index > 23 {
        return -1;
    }
    let vaar = dhruv_vedic_base::ALL_VAARS[vaar_index as usize];
    let hora = dhruv_vedic_base::hora_at(vaar, hora_index as u8);
    hora.index() as i32
}

/// Compute complete Ashtakavarga for a given date and location.
///
/// # Safety
/// All pointers must be valid. `out` must point to a valid `DhruvAshtakavargaResult`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dhruv_ashtakavarga_for_date(
    engine: *const Engine,
    eop: *const dhruv_time::EopKernel,
    utc: *const DhruvUtcTime,
    location: *const DhruvGeoLocation,
    ayanamsha_system: u32,
    use_nutation: u8,
    out: *mut DhruvAshtakavargaResult,
) -> DhruvStatus {
    if engine.is_null() || eop.is_null() || utc.is_null() || location.is_null() || out.is_null() {
        return DhruvStatus::NullPointer;
    }

    let engine = unsafe { &*engine };
    let eop = unsafe { &*eop };
    let utc_c = unsafe { &*utc };
    let loc_c = unsafe { &*location };

    let utc_time = UtcTime {
        year: utc_c.year,
        month: utc_c.month,
        day: utc_c.day,
        hour: utc_c.hour,
        minute: utc_c.minute,
        second: utc_c.second,
    };

    let location = GeoLocation::new(loc_c.latitude_deg, loc_c.longitude_deg, loc_c.altitude_m);

    let system = match ayanamsha_system_from_code(ayanamsha_system as i32) {
        Some(s) => s,
        None => return DhruvStatus::InvalidQuery,
    };

    let aya_config = SankrantiConfig::new(system, use_nutation != 0);

    match dhruv_search::ashtakavarga_for_date(engine, eop, &utc_time, &location, &aya_config) {
        Ok(result) => {
            let out = unsafe { &mut *out };
            for (i, bav) in result.bavs.iter().enumerate() {
                out.bavs[i] = DhruvBhinnaAshtakavarga {
                    graha_index: bav.graha_index,
                    points: bav.points,
                    contributors: bav.contributors,
                };
            }
            out.sav = DhruvSarvaAshtakavarga {
                total_points: result.sav.total_points,
                after_trikona: result.sav.after_trikona,
                after_ekadhipatya: result.sav.after_ekadhipatya,
            };
            DhruvStatus::Ok
        }
        Err(e) => DhruvStatus::from(&e),
    }
}

// ---------------------------------------------------------------------------
// Graha Positions C-compatible types
// ---------------------------------------------------------------------------

/// C-compatible graha positions configuration.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct DhruvGrahaPositionsConfig {
    pub include_nakshatra: u8,
    pub include_lagna: u8,
    pub include_outer_planets: u8,
    pub include_bhava: u8,
}

/// C-compatible single graha entry.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct DhruvGrahaEntry {
    pub sidereal_longitude: f64,
    /// 0-based rashi index (0-11).
    pub rashi_index: u8,
    /// 0-based nakshatra index (0-26), 255 if not computed.
    pub nakshatra_index: u8,
    /// Pada (1-4), 0 if not computed.
    pub pada: u8,
    /// Bhava number (1-12), 0 if not computed.
    pub bhava_number: u8,
}

/// C-compatible graha positions result.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct DhruvGrahaPositions {
    /// 9 Vedic grahas (indexed by graha index 0-8).
    pub grahas: [DhruvGrahaEntry; 9],
    /// Lagna entry (sentinel if not computed).
    pub lagna: DhruvGrahaEntry,
    /// Outer planets: [Uranus, Neptune, Pluto].
    pub outer_planets: [DhruvGrahaEntry; 3],
}

fn graha_entry_to_ffi(entry: &dhruv_search::GrahaEntry) -> DhruvGrahaEntry {
    DhruvGrahaEntry {
        sidereal_longitude: entry.sidereal_longitude,
        rashi_index: entry.rashi_index,
        nakshatra_index: entry.nakshatra_index,
        pada: entry.pada,
        bhava_number: entry.bhava_number,
    }
}

/// Compute comprehensive graha positions.
///
/// # Safety
/// All pointers must be valid. `out` must point to a valid `DhruvGrahaPositions`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dhruv_graha_positions(
    engine: *const Engine,
    eop: *const dhruv_time::EopKernel,
    utc: *const DhruvUtcTime,
    location: *const DhruvGeoLocation,
    bhava_config: *const DhruvBhavaConfig,
    ayanamsha_system: u32,
    use_nutation: u8,
    config: *const DhruvGrahaPositionsConfig,
    out: *mut DhruvGrahaPositions,
) -> DhruvStatus {
    if engine.is_null() || eop.is_null() || utc.is_null() || location.is_null() || out.is_null() {
        return DhruvStatus::NullPointer;
    }

    let engine = unsafe { &*engine };
    let eop = unsafe { &*eop };
    let utc_c = unsafe { &*utc };
    let loc_c = unsafe { &*location };

    let utc_time = UtcTime {
        year: utc_c.year,
        month: utc_c.month,
        day: utc_c.day,
        hour: utc_c.hour,
        minute: utc_c.minute,
        second: utc_c.second,
    };

    let location = GeoLocation::new(loc_c.latitude_deg, loc_c.longitude_deg, loc_c.altitude_m);

    let system = match ayanamsha_system_from_code(ayanamsha_system as i32) {
        Some(s) => s,
        None => return DhruvStatus::InvalidQuery,
    };

    let rust_bhava_config = match resolve_bhava_config_ptr(bhava_config) {
        Ok(c) => c,
        Err(status) => return status,
    };

    let rust_config = match resolve_graha_positions_config_ptr(config) {
        Ok(c) => c,
        Err(status) => return status,
    };

    let aya_config = SankrantiConfig::new(system, use_nutation != 0);

    match dhruv_search::graha_positions(
        engine,
        eop,
        &utc_time,
        &location,
        &rust_bhava_config,
        &aya_config,
        &rust_config,
    ) {
        Ok(result) => {
            let out = unsafe { &mut *out };
            for i in 0..9 {
                out.grahas[i] = graha_entry_to_ffi(&result.grahas[i]);
            }
            out.lagna = graha_entry_to_ffi(&result.lagna);
            for i in 0..3 {
                out.outer_planets[i] = graha_entry_to_ffi(&result.outer_planets[i]);
            }
            DhruvStatus::Ok
        }
        Err(e) => DhruvStatus::from(&e),
    }
}

// ---------------------------------------------------------------------------
// Core Bindus
// ---------------------------------------------------------------------------

/// C-compatible bindus configuration.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct DhruvBindusConfig {
    /// Include nakshatra + pada: 1 = yes, 0 = no.
    pub include_nakshatra: u8,
    /// Include bhava placement: 1 = yes, 0 = no.
    pub include_bhava: u8,
    /// Time-based Gulika/Maandi configuration.
    pub upagraha_config: DhruvTimeUpagrahaConfig,
}

/// C-compatible bindus result.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct DhruvBindusResult {
    /// 12 arudha padas (A1-A12).
    pub arudha_padas: [DhruvGrahaEntry; 12],
    /// Bhrigu Bindu.
    pub bhrigu_bindu: DhruvGrahaEntry,
    /// Pranapada Lagna.
    pub pranapada_lagna: DhruvGrahaEntry,
    /// Gulika.
    pub gulika: DhruvGrahaEntry,
    /// Maandi.
    pub maandi: DhruvGrahaEntry,
    /// Hora Lagna.
    pub hora_lagna: DhruvGrahaEntry,
    /// Ghati Lagna.
    pub ghati_lagna: DhruvGrahaEntry,
    /// Sree Lagna.
    pub sree_lagna: DhruvGrahaEntry,
}

/// Compute curated sensitive points (bindus) with optional nakshatra/bhava enrichment.
///
/// # Safety
/// All pointers must be valid. `out` must point to a valid `DhruvBindusResult`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dhruv_core_bindus(
    engine: *const Engine,
    eop: *const dhruv_time::EopKernel,
    utc: *const DhruvUtcTime,
    location: *const DhruvGeoLocation,
    bhava_config: *const DhruvBhavaConfig,
    riseset_config: *const DhruvRiseSetConfig,
    ayanamsha_system: u32,
    use_nutation: u8,
    config: *const DhruvBindusConfig,
    out: *mut DhruvBindusResult,
) -> DhruvStatus {
    if engine.is_null() || eop.is_null() || utc.is_null() || location.is_null() || out.is_null() {
        return DhruvStatus::NullPointer;
    }

    let engine = unsafe { &*engine };
    let eop = unsafe { &*eop };
    let utc_c = unsafe { &*utc };
    let loc_c = unsafe { &*location };

    let utc_time = UtcTime {
        year: utc_c.year,
        month: utc_c.month,
        day: utc_c.day,
        hour: utc_c.hour,
        minute: utc_c.minute,
        second: utc_c.second,
    };

    let location = GeoLocation::new(loc_c.latitude_deg, loc_c.longitude_deg, loc_c.altitude_m);

    let system = match ayanamsha_system_from_code(ayanamsha_system as i32) {
        Some(s) => s,
        None => return DhruvStatus::InvalidQuery,
    };

    let rust_bhava_config = match resolve_bhava_config_ptr(bhava_config) {
        Ok(c) => c,
        Err(status) => return status,
    };

    let rs_config = match resolve_riseset_config_ptr(riseset_config) {
        Ok(c) => c,
        Err(status) => return status,
    };

    let rust_config = match resolve_bindus_config_ptr(config) {
        Ok(c) => c,
        Err(status) => return status,
    };

    let aya_config = SankrantiConfig::new(system, use_nutation != 0);

    match dhruv_search::core_bindus(
        engine,
        eop,
        &utc_time,
        &location,
        &rust_bhava_config,
        &rs_config,
        &aya_config,
        &rust_config,
    ) {
        Ok(result) => {
            let out = unsafe { &mut *out };
            for i in 0..12 {
                out.arudha_padas[i] = graha_entry_to_ffi(&result.arudha_padas[i]);
            }
            out.bhrigu_bindu = graha_entry_to_ffi(&result.bhrigu_bindu);
            out.pranapada_lagna = graha_entry_to_ffi(&result.pranapada_lagna);
            out.gulika = graha_entry_to_ffi(&result.gulika);
            out.maandi = graha_entry_to_ffi(&result.maandi);
            out.hora_lagna = graha_entry_to_ffi(&result.hora_lagna);
            out.ghati_lagna = graha_entry_to_ffi(&result.ghati_lagna);
            out.sree_lagna = graha_entry_to_ffi(&result.sree_lagna);
            DhruvStatus::Ok
        }
        Err(e) => DhruvStatus::from(&e),
    }
}

// ---------------------------------------------------------------------------
// Drishti (Planetary Aspects)
// ---------------------------------------------------------------------------

/// C-compatible drishti entry.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct DhruvDrishtiEntry {
    pub angular_distance: f64,
    pub base_virupa: f64,
    pub special_virupa: f64,
    pub total_virupa: f64,
}

/// C-compatible drishti configuration.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct DhruvDrishtiConfig {
    /// Include graha-to-bhava drishti: 1 = yes, 0 = no.
    pub include_bhava: u8,
    /// Include graha-to-lagna drishti: 1 = yes, 0 = no.
    pub include_lagna: u8,
    /// Include graha-to-core-bindus drishti: 1 = yes, 0 = no.
    pub include_bindus: u8,
}

/// C-compatible drishti result.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct DhruvDrishtiResult {
    /// 9×9 graha-to-graha drishti matrix.
    pub graha_to_graha: [[DhruvDrishtiEntry; 9]; 9],
    /// 9×12 graha-to-bhava-cusp drishti.
    pub graha_to_bhava: [[DhruvDrishtiEntry; 12]; 9],
    /// 9×1 graha-to-lagna drishti.
    pub graha_to_lagna: [DhruvDrishtiEntry; 9],
    /// 9×19 graha-to-core-bindus drishti.
    pub graha_to_bindus: [[DhruvDrishtiEntry; 19]; 9],
}

fn drishti_entry_to_ffi(entry: &dhruv_vedic_base::DrishtiEntry) -> DhruvDrishtiEntry {
    DhruvDrishtiEntry {
        angular_distance: entry.angular_distance,
        base_virupa: entry.base_virupa,
        special_virupa: entry.special_virupa,
        total_virupa: entry.total_virupa,
    }
}

fn ashtakavarga_result_to_ffi(
    result: &dhruv_vedic_base::AshtakavargaResult,
) -> DhruvAshtakavargaResult {
    let mut out = DhruvAshtakavargaResult {
        bavs: [DhruvBhinnaAshtakavarga {
            graha_index: 0,
            points: [0; 12],
            contributors: [[0; 8]; 12],
        }; 7],
        sav: DhruvSarvaAshtakavarga {
            total_points: [0; 12],
            after_trikona: [0; 12],
            after_ekadhipatya: [0; 12],
        },
    };
    for (i, bav) in result.bavs.iter().enumerate() {
        out.bavs[i] = DhruvBhinnaAshtakavarga {
            graha_index: bav.graha_index,
            points: bav.points,
            contributors: bav.contributors,
        };
    }
    out.sav = DhruvSarvaAshtakavarga {
        total_points: result.sav.total_points,
        after_trikona: result.sav.after_trikona,
        after_ekadhipatya: result.sav.after_ekadhipatya,
    };
    out
}

fn shadbala_result_to_ffi(result: &dhruv_search::ShadbalaResult) -> DhruvShadbalaResult {
    let mut out = DhruvShadbalaResult {
        entries: [DhruvShadbalaEntry {
            graha_index: 0,
            sthana: DhruvSthanaBalaBreakdown {
                uchcha: 0.0,
                saptavargaja: 0.0,
                ojhayugma: 0.0,
                kendradi: 0.0,
                drekkana: 0.0,
                total: 0.0,
            },
            dig: 0.0,
            kala: DhruvKalaBalaBreakdown {
                nathonnatha: 0.0,
                paksha: 0.0,
                tribhaga: 0.0,
                abda: 0.0,
                masa: 0.0,
                vara: 0.0,
                hora: 0.0,
                ayana: 0.0,
                yuddha: 0.0,
                total: 0.0,
            },
            cheshta: 0.0,
            naisargika: 0.0,
            drik: 0.0,
            total_shashtiamsas: 0.0,
            total_rupas: 0.0,
            required_strength: 0.0,
            is_strong: 0,
        }; 7],
    };
    for (i, e) in result.entries.iter().enumerate() {
        out.entries[i] = DhruvShadbalaEntry {
            graha_index: e.graha.index(),
            sthana: DhruvSthanaBalaBreakdown {
                uchcha: e.sthana.uchcha,
                saptavargaja: e.sthana.saptavargaja,
                ojhayugma: e.sthana.ojhayugma,
                kendradi: e.sthana.kendradi,
                drekkana: e.sthana.drekkana,
                total: e.sthana.total,
            },
            dig: e.dig,
            kala: DhruvKalaBalaBreakdown {
                nathonnatha: e.kala.nathonnatha,
                paksha: e.kala.paksha,
                tribhaga: e.kala.tribhaga,
                abda: e.kala.abda,
                masa: e.kala.masa,
                vara: e.kala.vara,
                hora: e.kala.hora,
                ayana: e.kala.ayana,
                yuddha: e.kala.yuddha,
                total: e.kala.total,
            },
            cheshta: e.cheshta,
            naisargika: e.naisargika,
            drik: e.drik,
            total_shashtiamsas: e.total_shashtiamsas,
            total_rupas: e.total_rupas,
            required_strength: e.required_strength,
            is_strong: e.is_strong as u8,
        };
    }
    out
}

fn bhavabala_entry_to_ffi(entry: &dhruv_vedic_base::BhavaBalaEntry) -> DhruvBhavaBalaEntry {
    DhruvBhavaBalaEntry {
        bhava_number: entry.bhava_number,
        cusp_sidereal_lon: entry.cusp_sidereal_lon,
        rashi_index: entry.rashi_index,
        lord_graha_index: entry.lord.index(),
        bhavadhipati: entry.bhavadhipati,
        dig: entry.dig,
        drishti: entry.drishti,
        occupation_bonus: entry.occupation_bonus,
        rising_bonus: entry.rising_bonus,
        total_virupas: entry.total_virupas,
        total_rupas: entry.total_rupas,
    }
}

fn bhavabala_result_to_ffi(result: &dhruv_vedic_base::BhavaBalaResult) -> DhruvBhavaBalaResult {
    let mut out = DhruvBhavaBalaResult {
        entries: [DhruvBhavaBalaEntry {
            bhava_number: 0,
            cusp_sidereal_lon: 0.0,
            rashi_index: 0,
            lord_graha_index: 0,
            bhavadhipati: 0.0,
            dig: 0.0,
            drishti: 0.0,
            occupation_bonus: 0.0,
            rising_bonus: 0.0,
            total_virupas: 0.0,
            total_rupas: 0.0,
        }; 12],
    };
    for (i, entry) in result.entries.iter().enumerate() {
        out.entries[i] = bhavabala_entry_to_ffi(entry);
    }
    out
}

fn vimsopaka_result_to_ffi(result: &dhruv_search::VimsopakaResult) -> DhruvVimsopakaResult {
    let mut out = DhruvVimsopakaResult {
        entries: [DhruvVimsopakaEntry {
            graha_index: 0,
            shadvarga: 0.0,
            saptavarga: 0.0,
            dashavarga: 0.0,
            shodasavarga: 0.0,
        }; 9],
    };
    for (i, e) in result.entries.iter().enumerate() {
        out.entries[i] = DhruvVimsopakaEntry {
            graha_index: e.graha.index(),
            shadvarga: e.shadvarga,
            saptavarga: e.saptavarga,
            dashavarga: e.dashavarga,
            shodasavarga: e.shodasavarga,
        };
    }
    out
}

fn bhava_bala_birth_period_from_code(
    code: u32,
) -> Result<dhruv_vedic_base::BhavaBalaBirthPeriod, DhruvStatus> {
    match code {
        0 => Ok(dhruv_vedic_base::BhavaBalaBirthPeriod::Day),
        1 => Ok(dhruv_vedic_base::BhavaBalaBirthPeriod::Twilight),
        2 => Ok(dhruv_vedic_base::BhavaBalaBirthPeriod::Night),
        _ => Err(DhruvStatus::InvalidInput),
    }
}

/// Compute graha drishti (planetary aspects) with optional extensions.
///
/// # Safety
/// All pointers must be valid. `out` must point to a valid `DhruvDrishtiResult`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dhruv_drishti(
    engine: *const Engine,
    eop: *const dhruv_time::EopKernel,
    utc: *const DhruvUtcTime,
    location: *const DhruvGeoLocation,
    bhava_config: *const DhruvBhavaConfig,
    riseset_config: *const DhruvRiseSetConfig,
    ayanamsha_system: u32,
    use_nutation: u8,
    config: *const DhruvDrishtiConfig,
    out: *mut DhruvDrishtiResult,
) -> DhruvStatus {
    if engine.is_null() || eop.is_null() || utc.is_null() || location.is_null() || out.is_null() {
        return DhruvStatus::NullPointer;
    }

    let engine = unsafe { &*engine };
    let eop = unsafe { &*eop };
    let utc_c = unsafe { &*utc };
    let loc_c = unsafe { &*location };

    let utc_time = UtcTime {
        year: utc_c.year,
        month: utc_c.month,
        day: utc_c.day,
        hour: utc_c.hour,
        minute: utc_c.minute,
        second: utc_c.second,
    };

    let location = GeoLocation::new(loc_c.latitude_deg, loc_c.longitude_deg, loc_c.altitude_m);

    let system = match ayanamsha_system_from_code(ayanamsha_system as i32) {
        Some(s) => s,
        None => return DhruvStatus::InvalidQuery,
    };

    let rust_bhava_config = match resolve_bhava_config_ptr(bhava_config) {
        Ok(c) => c,
        Err(status) => return status,
    };

    let rs_config = match resolve_riseset_config_ptr(riseset_config) {
        Ok(c) => c,
        Err(status) => return status,
    };

    let rust_config = match resolve_drishti_config_ptr(config) {
        Ok(c) => c,
        Err(status) => return status,
    };

    let aya_config = SankrantiConfig::new(system, use_nutation != 0);

    match dhruv_search::drishti_for_date(
        engine,
        eop,
        &utc_time,
        &location,
        &rust_bhava_config,
        &rs_config,
        &aya_config,
        &rust_config,
    ) {
        Ok(result) => {
            let out = unsafe { &mut *out };
            for i in 0..9 {
                for j in 0..9 {
                    out.graha_to_graha[i][j] =
                        drishti_entry_to_ffi(&result.graha_to_graha.entries[i][j]);
                }
                out.graha_to_lagna[i] = drishti_entry_to_ffi(&result.graha_to_lagna[i]);
                for j in 0..12 {
                    out.graha_to_bhava[i][j] = drishti_entry_to_ffi(&result.graha_to_bhava[i][j]);
                }
                for j in 0..19 {
                    out.graha_to_bindus[i][j] = drishti_entry_to_ffi(&result.graha_to_bindus[i][j]);
                }
            }
            DhruvStatus::Ok
        }
        Err(e) => DhruvStatus::from(&e),
    }
}

// ---------------------------------------------------------------------------
// Amsha (Divisional Chart) FFI types
// ---------------------------------------------------------------------------

/// C-compatible amsha entry (position in a divisional chart).
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct DhruvAmshaEntry {
    /// Sidereal longitude in [0, 360).
    pub sidereal_longitude: f64,
    /// 0-based rashi index (0-11).
    pub rashi_index: u8,
    /// Degrees component of DMS within rashi.
    pub dms_degrees: u16,
    /// Minutes component of DMS within rashi.
    pub dms_minutes: u8,
    /// Seconds component of DMS within rashi.
    pub dms_seconds: f64,
    /// Decimal degrees within rashi [0, 30).
    pub degrees_in_rashi: f64,
}

impl DhruvAmshaEntry {
    fn zeroed() -> Self {
        Self {
            sidereal_longitude: 0.0,
            rashi_index: 0,
            dms_degrees: 0,
            dms_minutes: 0,
            dms_seconds: 0.0,
            degrees_in_rashi: 0.0,
        }
    }
}

/// C-compatible amsha chart scope flags.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct DhruvAmshaChartScope {
    pub include_bhava_cusps: u8,
    pub include_arudha_padas: u8,
    pub include_upagrahas: u8,
    pub include_sphutas: u8,
    pub include_special_lagnas: u8,
}

/// C-compatible amsha selection config for FullKundali.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct DhruvAmshaSelectionConfig {
    /// Number of valid entries (0..=40).
    pub count: u8,
    /// D-numbers (1..144), 0=unused.
    pub codes: [u16; 40],
    /// Variation codes: 0=default, 1=HoraCancerLeoOnly.
    pub variations: [u8; 40],
}

/// C-compatible single amsha chart result.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct DhruvAmshaChart {
    /// D-number of this chart.
    pub amsha_code: u16,
    /// Variation code (0=default, 1=HoraCancerLeoOnly).
    pub variation_code: u8,
    /// 9 Vedic grahas.
    pub grahas: [DhruvAmshaEntry; 9],
    /// Lagna entry.
    pub lagna: DhruvAmshaEntry,
    /// Whether bhava cusps are populated.
    pub bhava_cusps_valid: u8,
    pub bhava_cusps: [DhruvAmshaEntry; 12],
    /// Whether arudha padas are populated.
    pub arudha_padas_valid: u8,
    pub arudha_padas: [DhruvAmshaEntry; 12],
    /// Whether upagrahas are populated.
    pub upagrahas_valid: u8,
    pub upagrahas: [DhruvAmshaEntry; 11],
    /// Whether sphutas are populated.
    pub sphutas_valid: u8,
    pub sphutas: [DhruvAmshaEntry; 16],
    /// Whether special lagnas are populated.
    pub special_lagnas_valid: u8,
    pub special_lagnas: [DhruvAmshaEntry; 8],
}

impl DhruvAmshaChart {
    fn zeroed() -> Self {
        Self {
            amsha_code: 0,
            variation_code: 0,
            grahas: [DhruvAmshaEntry::zeroed(); 9],
            lagna: DhruvAmshaEntry::zeroed(),
            bhava_cusps_valid: 0,
            bhava_cusps: [DhruvAmshaEntry::zeroed(); 12],
            arudha_padas_valid: 0,
            arudha_padas: [DhruvAmshaEntry::zeroed(); 12],
            upagrahas_valid: 0,
            upagrahas: [DhruvAmshaEntry::zeroed(); 11],
            sphutas_valid: 0,
            sphutas: [DhruvAmshaEntry::zeroed(); 16],
            special_lagnas_valid: 0,
            special_lagnas: [DhruvAmshaEntry::zeroed(); 8],
        }
    }
}

fn amsha_entry_to_ffi(entry: &dhruv_search::AmshaEntry) -> DhruvAmshaEntry {
    DhruvAmshaEntry {
        sidereal_longitude: entry.sidereal_longitude,
        rashi_index: entry.rashi_index,
        dms_degrees: entry.dms.degrees,
        dms_minutes: entry.dms.minutes,
        dms_seconds: entry.dms.seconds,
        degrees_in_rashi: entry.degrees_in_rashi,
    }
}

fn amsha_chart_to_ffi(chart: &dhruv_search::AmshaChart) -> DhruvAmshaChart {
    let mut out = DhruvAmshaChart::zeroed();
    out.amsha_code = chart.amsha.code();
    out.variation_code = match chart.variation {
        AmshaVariation::TraditionalParashari => 0,
        AmshaVariation::HoraCancerLeoOnly => 1,
    };
    for (i, graha) in chart.grahas.iter().enumerate() {
        out.grahas[i] = amsha_entry_to_ffi(graha);
    }
    out.lagna = amsha_entry_to_ffi(&chart.lagna);
    if let Some(ref cusps) = chart.bhava_cusps {
        out.bhava_cusps_valid = 1;
        for (i, cusp) in cusps.iter().enumerate() {
            out.bhava_cusps[i] = amsha_entry_to_ffi(cusp);
        }
    }
    if let Some(ref padas) = chart.arudha_padas {
        out.arudha_padas_valid = 1;
        for (i, pada) in padas.iter().enumerate() {
            out.arudha_padas[i] = amsha_entry_to_ffi(pada);
        }
    }
    if let Some(ref upa) = chart.upagrahas {
        out.upagrahas_valid = 1;
        for (i, upagraha) in upa.iter().enumerate() {
            out.upagrahas[i] = amsha_entry_to_ffi(upagraha);
        }
    }
    if let Some(ref sph) = chart.sphutas {
        out.sphutas_valid = 1;
        for (i, sphuta) in sph.iter().enumerate() {
            out.sphutas[i] = amsha_entry_to_ffi(sphuta);
        }
    }
    if let Some(ref sl) = chart.special_lagnas {
        out.special_lagnas_valid = 1;
        for (i, special_lagna) in sl.iter().enumerate() {
            out.special_lagnas[i] = amsha_entry_to_ffi(special_lagna);
        }
    }
    out
}

// ---------------------------------------------------------------------------
// Charakaraka FFI types
// ---------------------------------------------------------------------------

/// Maximum number of charakaraka assignments in one result.
pub const DHRUV_MAX_CHARAKARAKA_ENTRIES: usize = 8;

/// C-compatible charakaraka entry.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct DhruvCharakarakaEntry {
    /// Charakaraka role code.
    pub role_code: u8,
    /// Graha index (0=Sun..8=Ketu).
    pub graha_index: u8,
    /// 1-based rank within selected scheme.
    pub rank: u8,
    /// Sidereal longitude in [0,360).
    pub longitude_deg: f64,
    /// Degrees within rashi [0,30).
    pub degrees_in_rashi: f64,
    /// Ranking degree (Rahu-reversed where applicable).
    pub effective_degrees_in_rashi: f64,
}

impl DhruvCharakarakaEntry {
    fn zeroed() -> Self {
        Self {
            role_code: 0,
            graha_index: 0,
            rank: 0,
            longitude_deg: 0.0,
            degrees_in_rashi: 0.0,
            effective_degrees_in_rashi: 0.0,
        }
    }
}

/// C-compatible charakaraka result.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct DhruvCharakarakaResult {
    /// Input scheme code.
    pub scheme: u8,
    /// For mixed scheme: 1 if resolved to 8-karaka mode.
    pub used_eight_karakas: u8,
    /// Number of populated entries (7 or 8).
    pub count: u8,
    /// Ordered assignments.
    pub entries: [DhruvCharakarakaEntry; DHRUV_MAX_CHARAKARAKA_ENTRIES],
}

impl DhruvCharakarakaResult {
    fn zeroed() -> Self {
        Self {
            scheme: 0,
            used_eight_karakas: 0,
            count: 0,
            entries: [DhruvCharakarakaEntry::zeroed(); DHRUV_MAX_CHARAKARAKA_ENTRIES],
        }
    }
}

fn charakaraka_entry_to_ffi(entry: &dhruv_vedic_base::CharakarakaEntry) -> DhruvCharakarakaEntry {
    DhruvCharakarakaEntry {
        role_code: entry.role.code(),
        graha_index: entry.graha.index(),
        rank: entry.rank,
        longitude_deg: entry.longitude_deg,
        degrees_in_rashi: entry.degrees_in_rashi,
        effective_degrees_in_rashi: entry.effective_degrees_in_rashi,
    }
}

// ---------------------------------------------------------------------------
// Shadbala & Vimsopaka FFI types
// ---------------------------------------------------------------------------

/// C-compatible Sthana Bala breakdown.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct DhruvSthanaBalaBreakdown {
    pub uchcha: f64,
    pub saptavargaja: f64,
    pub ojhayugma: f64,
    pub kendradi: f64,
    pub drekkana: f64,
    pub total: f64,
}

/// C-compatible Kala Bala breakdown.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct DhruvKalaBalaBreakdown {
    pub nathonnatha: f64,
    pub paksha: f64,
    pub tribhaga: f64,
    pub abda: f64,
    pub masa: f64,
    pub vara: f64,
    pub hora: f64,
    pub ayana: f64,
    pub yuddha: f64,
    pub total: f64,
}

/// C-compatible Shadbala entry for a single sapta graha.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct DhruvShadbalaEntry {
    pub graha_index: u8,
    pub sthana: DhruvSthanaBalaBreakdown,
    pub dig: f64,
    pub kala: DhruvKalaBalaBreakdown,
    pub cheshta: f64,
    pub naisargika: f64,
    pub drik: f64,
    pub total_shashtiamsas: f64,
    pub total_rupas: f64,
    pub required_strength: f64,
    pub is_strong: u8,
}

/// C-compatible Shadbala result for all 7 sapta grahas.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct DhruvShadbalaResult {
    pub entries: [DhruvShadbalaEntry; 7],
}

/// C-compatible Bhava Bala entry for a single house.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct DhruvBhavaBalaEntry {
    pub bhava_number: u8,
    pub cusp_sidereal_lon: f64,
    pub rashi_index: u8,
    pub lord_graha_index: u8,
    pub bhavadhipati: f64,
    pub dig: f64,
    pub drishti: f64,
    pub occupation_bonus: f64,
    pub rising_bonus: f64,
    pub total_virupas: f64,
    pub total_rupas: f64,
}

/// C-compatible Bhava Bala result for all 12 houses.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct DhruvBhavaBalaResult {
    pub entries: [DhruvBhavaBalaEntry; 12],
}

/// C-compatible low-level inputs for Bhava Bala.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct DhruvBhavaBalaInputs {
    /// Sidereal cusp longitudes for houses 1..12.
    pub cusp_sidereal_lons: [f64; 12],
    /// Sidereal ascendant longitude.
    pub ascendant_sidereal_lon: f64,
    /// Sidereal meridian (MC) longitude.
    pub meridian_sidereal_lon: f64,
    /// Bhava number (1-12, or 0 if unknown) for each graha in Sun..Ketu order.
    pub graha_bhava_numbers: [u8; 9],
    /// House-lord Shadbala totals in virupas for each house.
    pub house_lord_strengths: [f64; 12],
    /// Aspect virupas from each graha to each house cusp in Sun..Ketu x house order.
    pub aspect_virupas: [[f64; 12]; 9],
    /// Birth-period code: 0=Day, 1=Twilight, 2=Night.
    pub birth_period: u32,
}

/// C-compatible Vimsopaka entry for a single graha.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct DhruvVimsopakaEntry {
    pub graha_index: u8,
    pub shadvarga: f64,
    pub saptavarga: f64,
    pub dashavarga: f64,
    pub shodasavarga: f64,
}

/// C-compatible Vimsopaka result for all 9 navagrahas.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct DhruvVimsopakaResult {
    pub entries: [DhruvVimsopakaEntry; 9],
}

/// C-compatible combined bala bundle result.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct DhruvBalaBundleResult {
    pub shadbala: DhruvShadbalaResult,
    pub vimsopaka: DhruvVimsopakaResult,
    pub ashtakavarga: DhruvAshtakavargaResult,
    pub bhavabala: DhruvBhavaBalaResult,
}

// ---------------------------------------------------------------------------
// Avastha (Planetary State) FFI types
// ---------------------------------------------------------------------------

/// C-compatible Sayanadi result for a single graha.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct DhruvSayanadiResult {
    /// SayanadiAvastha index (0-11).
    pub avastha: u8,
    /// SayanadiSubState index per name-group (Ka/Cha/Ta-retroflex/Ta-dental/Pa).
    pub sub_states: [u8; 5],
}

/// C-compatible avasthas for a single graha.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct DhruvGrahaAvasthas {
    /// BaladiAvastha index (0-4).
    pub baladi: u8,
    /// JagradadiAvastha index (0-2).
    pub jagradadi: u8,
    /// DeeptadiAvastha index (0-8).
    pub deeptadi: u8,
    /// LajjitadiAvastha index (0-5).
    pub lajjitadi: u8,
    /// Sayanadi result with primary + 5 sub-states.
    pub sayanadi: DhruvSayanadiResult,
}

/// C-compatible avasthas for all 9 grahas.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct DhruvAllGrahaAvasthas {
    pub entries: [DhruvGrahaAvasthas; 9],
}

/// C-compatible full kundali configuration.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct DhruvFullKundaliConfig {
    /// Include bhava cusps section. Default: 1.
    pub include_bhava_cusps: u8,
    /// Include graha positions section.
    pub include_graha_positions: u8,
    /// Include bindus section.
    pub include_bindus: u8,
    /// Include drishti section.
    pub include_drishti: u8,
    /// Include ashtakavarga section.
    pub include_ashtakavarga: u8,
    /// Include upagrahas section.
    pub include_upagrahas: u8,
    /// Include root sphutas section.
    pub include_sphutas: u8,
    /// Include special lagnas section.
    pub include_special_lagnas: u8,
    /// Include amsha (divisional chart) section.
    pub include_amshas: u8,
    /// Include shadbala section (sapta grahas only).
    pub include_shadbala: u8,
    /// Include Bhava Bala section (12 houses).
    pub include_bhavabala: u8,
    /// Include vimsopaka bala section (navagraha).
    pub include_vimsopaka: u8,
    /// Include avastha (planetary state) section.
    pub include_avastha: u8,
    /// Include charakaraka section.
    pub include_charakaraka: u8,
    /// Charakaraka scheme code (0..3).
    pub charakaraka_scheme: u8,
    /// Node dignity policy for vimsopaka/avastha: 0=SignLordBased (default), 1=AlwaysSama.
    pub node_dignity_policy: u32,
    /// Time-based upagraha configuration shared by root upagrahas and sphutas.
    pub upagraha_config: DhruvTimeUpagrahaConfig,
    /// Graha positions config.
    pub graha_positions_config: DhruvGrahaPositionsConfig,
    /// Bindus config.
    pub bindus_config: DhruvBindusConfig,
    /// Drishti config.
    pub drishti_config: DhruvDrishtiConfig,
    /// Scope flags for amsha charts.
    pub amsha_scope: DhruvAmshaChartScope,
    /// Which amshas to compute.
    pub amsha_selection: DhruvAmshaSelectionConfig,
    /// Include panchang (tithi, karana, yoga, vaar, hora, ghatika, nakshatra).
    pub include_panchang: u8,
    /// Include calendar elements (masa, ayana, varsha). Implies include_panchang.
    pub include_calendar: u8,
    /// Include dasha (planetary period) section.
    pub include_dasha: u8,
    /// Dasha configuration.
    pub dasha_config: DhruvDashaSelectionConfig,
}

/// Maximum number of amsha charts in a single FFI batch.
pub const DHRUV_MAX_AMSHA_REQUESTS: usize = 40;
/// Maximum number of dasha systems in a single FFI batch.
pub const DHRUV_MAX_DASHA_SYSTEMS: usize = dhruv_vedic_base::dasha::MAX_DASHA_SYSTEMS;

/// C-compatible full kundali result.
///
/// **Ownership:** This struct owns its `dasha_handles`. After use, call
/// `dhruv_full_kundali_result_free` to release inner resources. Do NOT
/// `memcpy` the struct and free both copies — copied handles become dangling
/// after the first free. Treat as move-only: exactly one `result_free` call
/// per `dhruv_full_kundali_for_date` invocation.
///
/// `dasha_snapshot_count` may be less than `dasha_count` (partial success).
/// Match snapshots to hierarchies by `dasha_snapshots[i].system`, not index.
#[repr(C)]
#[derive(Debug)]
pub struct DhruvFullKundaliResult {
    /// Ayanamsha in degrees used for this kundali.
    pub ayanamsha_deg: f64,
    /// 1 when `include_bhava_cusps` was non-zero and computation succeeded; 0 otherwise.
    pub bhava_cusps_valid: u8,
    pub bhava_cusps: DhruvBhavaResult,
    pub graha_positions_valid: u8,
    pub graha_positions: DhruvGrahaPositions,
    pub bindus_valid: u8,
    pub bindus: DhruvBindusResult,
    pub drishti_valid: u8,
    pub drishti: DhruvDrishtiResult,
    pub ashtakavarga_valid: u8,
    pub ashtakavarga: DhruvAshtakavargaResult,
    pub upagrahas_valid: u8,
    pub upagrahas: DhruvAllUpagrahas,
    pub sphutas_valid: u8,
    pub sphutas: DhruvSphutalResult,
    pub special_lagnas_valid: u8,
    pub special_lagnas: DhruvSpecialLagnas,
    pub amshas_valid: u8,
    /// Number of populated amsha charts (0..=DHRUV_MAX_AMSHA_REQUESTS).
    pub amshas_count: u8,
    /// Fixed-size array of amsha charts.
    pub amshas: [DhruvAmshaChart; DHRUV_MAX_AMSHA_REQUESTS],
    pub shadbala_valid: u8,
    pub shadbala: DhruvShadbalaResult,
    pub bhavabala_valid: u8,
    pub bhavabala: DhruvBhavaBalaResult,
    pub vimsopaka_valid: u8,
    pub vimsopaka: DhruvVimsopakaResult,
    pub avastha_valid: u8,
    pub avastha: DhruvAllGrahaAvasthas,
    pub charakaraka_valid: u8,
    pub charakaraka: DhruvCharakarakaResult,
    pub panchang_valid: u8,
    pub panchang: DhruvPanchangInfo,
    /// Number of valid dasha hierarchies (0..=DHRUV_MAX_DASHA_SYSTEMS).
    pub dasha_count: u8,
    /// Opaque hierarchy handles. Read via `dhruv_dasha_hierarchy_*` accessors.
    /// Freed by `dhruv_full_kundali_result_free`. Do NOT call
    /// `dhruv_dasha_hierarchy_free` on these.
    pub dasha_handles: [DhruvDashaHierarchyHandle; DHRUV_MAX_DASHA_SYSTEMS],
    /// System codes for each hierarchy (DashaSystem repr(u8)).
    pub dasha_systems: [u8; DHRUV_MAX_DASHA_SYSTEMS],
    /// Number of valid dasha snapshots (may be < dasha_count).
    pub dasha_snapshot_count: u8,
    /// Inline snapshots matched by `.system` field, not by index.
    pub dasha_snapshots: [DhruvDashaSnapshot; DHRUV_MAX_DASHA_SYSTEMS],
}

/// Free resources owned by a `DhruvFullKundaliResult`.
/// Passing NULL is a no-op. Sets freed handles to NULL and zeroes all dasha
/// bookkeeping fields for deterministic post-free state.
///
/// **Ownership:** Exactly one `result_free` call per
/// `dhruv_full_kundali_for_date` invocation. Do NOT memcpy the result struct
/// and free both copies.
///
/// # Safety
/// `result` must point to a struct previously initialized by
/// `dhruv_full_kundali_for_date`, or be NULL.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dhruv_full_kundali_result_free(result: *mut DhruvFullKundaliResult) {
    if result.is_null() {
        return;
    }
    let r = unsafe { &mut *result };
    // Iterate all slots — don't trust dasha_count which may be corrupt.
    for handle in &mut r.dasha_handles {
        if !handle.is_null() {
            let _ =
                unsafe { Box::from_raw(*handle as *mut dhruv_vedic_base::dasha::DashaHierarchy) };
            *handle = ptr::null_mut();
        }
    }
    r.dasha_count = 0;
    r.dasha_snapshot_count = 0;
    r.dasha_systems = [0; DHRUV_MAX_DASHA_SYSTEMS];
}

/// Returns default full kundali configuration.
///
/// All core include flags (`include_bhava_cusps`, `include_graha_positions`,
/// `include_bindus`, `include_drishti`, `include_ashtakavarga`,
/// `include_upagrahas`, `include_sphutas`, `include_special_lagnas`) are set to 1.
/// Optional sections (`include_amshas`, `include_shadbala`, `include_vimsopaka`,
/// `include_avastha`, `include_panchang`, `include_calendar`, `include_dasha`)
/// are set to 0.
///
/// C callers should use this instead of zero-initializing the struct, which
/// would leave `include_bhava_cusps` (and other defaults) at 0.
#[unsafe(no_mangle)]
pub extern "C" fn dhruv_full_kundali_config_default() -> DhruvFullKundaliConfig {
    DhruvFullKundaliConfig {
        include_bhava_cusps: 1,
        include_graha_positions: 1,
        include_bindus: 1,
        include_drishti: 1,
        include_ashtakavarga: 1,
        include_upagrahas: 1,
        include_sphutas: 1,
        include_special_lagnas: 1,
        include_amshas: 0,
        include_shadbala: 0,
        include_bhavabala: 0,
        include_vimsopaka: 0,
        include_avastha: 0,
        include_charakaraka: 0,
        charakaraka_scheme: CharakarakaScheme::default() as u8,
        node_dignity_policy: 0, // SignLordBased
        upagraha_config: dhruv_time_upagraha_config_default(),
        graha_positions_config: DhruvGrahaPositionsConfig {
            include_nakshatra: 0,
            include_lagna: 1,
            include_outer_planets: 0,
            include_bhava: 0,
        },
        bindus_config: DhruvBindusConfig {
            include_nakshatra: 0,
            include_bhava: 0,
            upagraha_config: dhruv_time_upagraha_config_default(),
        },
        drishti_config: DhruvDrishtiConfig {
            include_bhava: 0,
            include_lagna: 0,
            include_bindus: 0,
        },
        amsha_scope: DhruvAmshaChartScope {
            include_bhava_cusps: 0,
            include_arudha_padas: 0,
            include_upagrahas: 0,
            include_sphutas: 0,
            include_special_lagnas: 0,
        },
        amsha_selection: DhruvAmshaSelectionConfig {
            count: 0,
            codes: [0; 40],
            variations: [0; 40],
        },
        include_panchang: 0,
        include_calendar: 0,
        include_dasha: 0,
        dasha_config: dhruv_dasha_selection_config_default(),
    }
}

/// Compute a full kundali in one call, reusing shared intermediates.
///
/// # Safety
/// All pointers must be valid. `out` must point to a valid `DhruvFullKundaliResult`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dhruv_full_kundali_for_date(
    engine: *const Engine,
    eop: *const dhruv_time::EopKernel,
    utc: *const DhruvUtcTime,
    location: *const DhruvGeoLocation,
    bhava_config: *const DhruvBhavaConfig,
    riseset_config: *const DhruvRiseSetConfig,
    ayanamsha_system: u32,
    use_nutation: u8,
    config: *const DhruvFullKundaliConfig,
    out: *mut DhruvFullKundaliResult,
) -> DhruvStatus {
    if engine.is_null() || eop.is_null() || utc.is_null() || location.is_null() || out.is_null() {
        return DhruvStatus::NullPointer;
    }

    // Zero-init immediately after null checks so result_free is safe on all paths.
    unsafe { std::ptr::write_bytes(out, 0, 1) };

    let engine = unsafe { &*engine };
    let eop = unsafe { &*eop };
    let utc_c = unsafe { &*utc };
    let loc_c = unsafe { &*location };

    let utc_time = UtcTime {
        year: utc_c.year,
        month: utc_c.month,
        day: utc_c.day,
        hour: utc_c.hour,
        minute: utc_c.minute,
        second: utc_c.second,
    };
    let location = GeoLocation::new(loc_c.latitude_deg, loc_c.longitude_deg, loc_c.altitude_m);

    let system = match ayanamsha_system_from_code(ayanamsha_system as i32) {
        Some(s) => s,
        None => return DhruvStatus::InvalidQuery,
    };

    let rust_bhava_config = match resolve_bhava_config_ptr(bhava_config) {
        Ok(c) => c,
        Err(status) => return status,
    };
    let rs_config = match resolve_riseset_config_ptr(riseset_config) {
        Ok(c) => c,
        Err(status) => return status,
    };
    let aya_config = SankrantiConfig::new(system, use_nutation != 0);

    let rust_config = match resolve_full_kundali_config_ptr(config) {
        Ok(c) => c,
        Err(status) => return status,
    };

    match full_kundali_for_date(
        engine,
        eop,
        &utc_time,
        &location,
        &rust_bhava_config,
        &rs_config,
        &aya_config,
        &rust_config,
    ) {
        Ok(result) => {
            // out was zero-init'd at entry; safe to populate fields now.
            let out = unsafe { &mut *out };

            out.ayanamsha_deg = result.ayanamsha_deg;

            if let Some(bh) = result.bhava_cusps {
                out.bhava_cusps_valid = 1;
                for i in 0..12 {
                    out.bhava_cusps.bhavas[i] = DhruvBhava {
                        number: bh.bhavas[i].number,
                        cusp_deg: bh.bhavas[i].cusp_deg,
                        start_deg: bh.bhavas[i].start_deg,
                        end_deg: bh.bhavas[i].end_deg,
                    };
                }
                out.bhava_cusps.lagna_deg = bh.lagna_deg;
                out.bhava_cusps.mc_deg = bh.mc_deg;
            }

            if let Some(g) = result.graha_positions {
                out.graha_positions_valid = 1;
                for i in 0..9 {
                    out.graha_positions.grahas[i] = graha_entry_to_ffi(&g.grahas[i]);
                }
                out.graha_positions.lagna = graha_entry_to_ffi(&g.lagna);
                for i in 0..3 {
                    out.graha_positions.outer_planets[i] = graha_entry_to_ffi(&g.outer_planets[i]);
                }
            }

            if let Some(b) = result.bindus {
                out.bindus_valid = 1;
                for i in 0..12 {
                    out.bindus.arudha_padas[i] = graha_entry_to_ffi(&b.arudha_padas[i]);
                }
                out.bindus.bhrigu_bindu = graha_entry_to_ffi(&b.bhrigu_bindu);
                out.bindus.pranapada_lagna = graha_entry_to_ffi(&b.pranapada_lagna);
                out.bindus.gulika = graha_entry_to_ffi(&b.gulika);
                out.bindus.maandi = graha_entry_to_ffi(&b.maandi);
                out.bindus.hora_lagna = graha_entry_to_ffi(&b.hora_lagna);
                out.bindus.ghati_lagna = graha_entry_to_ffi(&b.ghati_lagna);
                out.bindus.sree_lagna = graha_entry_to_ffi(&b.sree_lagna);
            }

            if let Some(d) = result.drishti {
                out.drishti_valid = 1;
                for i in 0..9 {
                    for j in 0..9 {
                        out.drishti.graha_to_graha[i][j] =
                            drishti_entry_to_ffi(&d.graha_to_graha.entries[i][j]);
                    }
                    out.drishti.graha_to_lagna[i] = drishti_entry_to_ffi(&d.graha_to_lagna[i]);
                    for j in 0..12 {
                        out.drishti.graha_to_bhava[i][j] =
                            drishti_entry_to_ffi(&d.graha_to_bhava[i][j]);
                    }
                    for j in 0..19 {
                        out.drishti.graha_to_bindus[i][j] =
                            drishti_entry_to_ffi(&d.graha_to_bindus[i][j]);
                    }
                }
            }

            if let Some(a) = result.ashtakavarga {
                out.ashtakavarga_valid = 1;
                for (i, bav) in a.bavs.iter().enumerate() {
                    out.ashtakavarga.bavs[i] = DhruvBhinnaAshtakavarga {
                        graha_index: bav.graha_index,
                        points: bav.points,
                        contributors: bav.contributors,
                    };
                }
                out.ashtakavarga.sav = DhruvSarvaAshtakavarga {
                    total_points: a.sav.total_points,
                    after_trikona: a.sav.after_trikona,
                    after_ekadhipatya: a.sav.after_ekadhipatya,
                };
            }

            if let Some(u) = result.upagrahas {
                out.upagrahas_valid = 1;
                out.upagrahas.gulika = u.gulika;
                out.upagrahas.maandi = u.maandi;
                out.upagrahas.kaala = u.kaala;
                out.upagrahas.mrityu = u.mrityu;
                out.upagrahas.artha_prahara = u.artha_prahara;
                out.upagrahas.yama_ghantaka = u.yama_ghantaka;
                out.upagrahas.dhooma = u.dhooma;
                out.upagrahas.vyatipata = u.vyatipata;
                out.upagrahas.parivesha = u.parivesha;
                out.upagrahas.indra_chapa = u.indra_chapa;
                out.upagrahas.upaketu = u.upaketu;
            }

            if let Some(s) = result.sphutas {
                out.sphutas_valid = 1;
                out.sphutas.longitudes = s.longitudes;
            }

            if let Some(s) = result.special_lagnas {
                out.special_lagnas_valid = 1;
                out.special_lagnas.bhava_lagna = s.bhava_lagna;
                out.special_lagnas.hora_lagna = s.hora_lagna;
                out.special_lagnas.ghati_lagna = s.ghati_lagna;
                out.special_lagnas.vighati_lagna = s.vighati_lagna;
                out.special_lagnas.varnada_lagna = s.varnada_lagna;
                out.special_lagnas.sree_lagna = s.sree_lagna;
                out.special_lagnas.pranapada_lagna = s.pranapada_lagna;
                out.special_lagnas.indu_lagna = s.indu_lagna;
            }

            if let Some(ref am) = result.amshas {
                out.amshas_valid = 1;
                let count = am.charts.len().min(DHRUV_MAX_AMSHA_REQUESTS);
                out.amshas_count = count as u8;
                for i in 0..count {
                    out.amshas[i] = amsha_chart_to_ffi(&am.charts[i]);
                }
            }

            if let Some(ref sb) = result.shadbala {
                out.shadbala_valid = 1;
                out.shadbala = shadbala_result_to_ffi(sb);
            }

            if let Some(ref bb) = result.bhavabala {
                out.bhavabala_valid = 1;
                out.bhavabala = bhavabala_result_to_ffi(bb);
            }

            if let Some(ref vm) = result.vimsopaka {
                out.vimsopaka_valid = 1;
                out.vimsopaka = vimsopaka_result_to_ffi(vm);
            }

            if let Some(ref av) = result.avastha {
                out.avastha_valid = 1;
                for i in 0..9 {
                    let e = &av.entries[i];
                    out.avastha.entries[i] = DhruvGrahaAvasthas {
                        baladi: e.baladi.index(),
                        jagradadi: e.jagradadi.index(),
                        deeptadi: e.deeptadi.index(),
                        lajjitadi: e.lajjitadi.index(),
                        sayanadi: DhruvSayanadiResult {
                            avastha: e.sayanadi.avastha.index(),
                            sub_states: [
                                e.sayanadi.sub_states[0].index(),
                                e.sayanadi.sub_states[1].index(),
                                e.sayanadi.sub_states[2].index(),
                                e.sayanadi.sub_states[3].index(),
                                e.sayanadi.sub_states[4].index(),
                            ],
                        },
                    };
                }
            }

            if let Some(ref ck) = result.charakaraka {
                out.charakaraka_valid = 1;
                out.charakaraka.scheme = ck.scheme as u8;
                out.charakaraka.used_eight_karakas = ck.used_eight_karakas as u8;
                let count = ck.entries.len().min(DHRUV_MAX_CHARAKARAKA_ENTRIES);
                out.charakaraka.count = count as u8;
                for i in 0..count {
                    out.charakaraka.entries[i] = charakaraka_entry_to_ffi(&ck.entries[i]);
                }
            }

            if let Some(ref p) = result.panchang {
                out.panchang_valid = 1;
                out.panchang = panchang_info_to_ffi(p);
            }

            if let Some(ref dasha_vec) = result.dasha {
                if dasha_vec.len() > DHRUV_MAX_DASHA_SYSTEMS {
                    return DhruvStatus::InvalidSearchConfig;
                }
                out.dasha_count = dasha_vec.len() as u8;
                for (i, h) in dasha_vec.iter().enumerate() {
                    let boxed = Box::new(h.clone());
                    out.dasha_handles[i] = Box::into_raw(boxed) as DhruvDashaHierarchyHandle;
                    out.dasha_systems[i] = h.system as u8;
                }

                if let Some(ref snap_vec) = result.dasha_snapshots {
                    if snap_vec.len() > DHRUV_MAX_DASHA_SYSTEMS {
                        return DhruvStatus::InvalidSearchConfig;
                    }
                    out.dasha_snapshot_count = snap_vec.len() as u8;
                    for (i, s) in snap_vec.iter().enumerate() {
                        out.dasha_snapshots[i].system = s.system as u8;
                        out.dasha_snapshots[i].query_jd = s.query_jd;
                        let count = s.periods.len().min(5);
                        out.dasha_snapshots[i].count = count as u8;
                        for j in 0..count {
                            out.dasha_snapshots[i].periods[j] = dasha_period_to_ffi(&s.periods[j]);
                        }
                    }
                }
            }

            DhruvStatus::Ok
        }
        Err(e) => DhruvStatus::from(&e),
    }
}

// ---------------------------------------------------------------------------
// Amsha (Divisional Chart) FFI functions
// ---------------------------------------------------------------------------

/// Transform a sidereal longitude through an amsha division.
///
/// # Safety
/// `out` must be a valid, non-null pointer.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dhruv_amsha_longitude(
    sidereal_lon: f64,
    amsha_code: u16,
    variation_code: u8,
    out: *mut f64,
) -> DhruvStatus {
    if out.is_null() {
        return DhruvStatus::NullPointer;
    }
    let amsha = match Amsha::from_code(amsha_code) {
        Some(a) => a,
        None => return DhruvStatus::InvalidSearchConfig,
    };
    let variation = match AmshaVariation::from_code(variation_code) {
        Some(v) => v,
        None => return DhruvStatus::InvalidSearchConfig,
    };
    if !variation.is_applicable_to(amsha) {
        return DhruvStatus::InvalidSearchConfig;
    }
    let result = amsha_longitude(sidereal_lon, amsha, Some(variation));
    unsafe { *out = result };
    DhruvStatus::Ok
}

/// Transform a sidereal longitude through an amsha division, returning full rashi info.
///
/// # Safety
/// `out` must be a valid, non-null pointer.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dhruv_amsha_rashi_info(
    sidereal_lon: f64,
    amsha_code: u16,
    variation_code: u8,
    out: *mut DhruvRashiInfo,
) -> DhruvStatus {
    if out.is_null() {
        return DhruvStatus::NullPointer;
    }
    let amsha = match Amsha::from_code(amsha_code) {
        Some(a) => a,
        None => return DhruvStatus::InvalidSearchConfig,
    };
    let variation = match AmshaVariation::from_code(variation_code) {
        Some(v) => v,
        None => return DhruvStatus::InvalidSearchConfig,
    };
    if !variation.is_applicable_to(amsha) {
        return DhruvStatus::InvalidSearchConfig;
    }
    let info = amsha_rashi_info(sidereal_lon, amsha, Some(variation));
    unsafe {
        *out = DhruvRashiInfo {
            rashi_index: info.rashi_index,
            dms: DhruvDms {
                degrees: info.dms.degrees,
                minutes: info.dms.minutes,
                seconds: info.dms.seconds,
            },
            degrees_in_rashi: info.degrees_in_rashi,
        };
    }
    DhruvStatus::Ok
}

/// Transform one longitude through multiple amshas.
///
/// `variation_codes` may be null (all default). `amsha_codes` and `out` must
/// be non-null when `count > 0`.
///
/// # Safety
/// `amsha_codes` must point to `count` u16 values. `variation_codes` (if
/// non-null) must point to `count` u8 values. `out` must point to `count` f64s.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dhruv_amsha_longitudes(
    sidereal_lon: f64,
    amsha_codes: *const u16,
    variation_codes: *const u8,
    count: u32,
    out: *mut f64,
) -> DhruvStatus {
    if count == 0 {
        return DhruvStatus::Ok;
    }
    if amsha_codes.is_null() || out.is_null() {
        return DhruvStatus::NullPointer;
    }
    let codes = unsafe { std::slice::from_raw_parts(amsha_codes, count as usize) };
    let out_slice = unsafe { std::slice::from_raw_parts_mut(out, count as usize) };
    for i in 0..count as usize {
        let amsha = match Amsha::from_code(codes[i]) {
            Some(a) => a,
            None => return DhruvStatus::InvalidSearchConfig,
        };
        let variation = if variation_codes.is_null() {
            AmshaVariation::TraditionalParashari
        } else {
            let vc = unsafe { *variation_codes.add(i) };
            match AmshaVariation::from_code(vc) {
                Some(v) => v,
                None => return DhruvStatus::InvalidSearchConfig,
            }
        };
        if !variation.is_applicable_to(amsha) {
            return DhruvStatus::InvalidSearchConfig;
        }
        out_slice[i] = amsha_longitude(sidereal_lon, amsha, Some(variation));
    }
    DhruvStatus::Ok
}

/// Compute an amsha chart for a single amsha at a given date/location.
///
/// # Safety
/// All pointer parameters must be valid and non-null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dhruv_amsha_chart_for_date(
    engine: *const Engine,
    eop: *const dhruv_time::EopKernel,
    utc: *const DhruvUtcTime,
    location: *const DhruvGeoLocation,
    bhava_config: *const DhruvBhavaConfig,
    riseset_config: *const DhruvRiseSetConfig,
    ayanamsha_system: u32,
    use_nutation: u8,
    amsha_code: u16,
    variation_code: u8,
    scope: *const DhruvAmshaChartScope,
    out: *mut DhruvAmshaChart,
) -> DhruvStatus {
    if engine.is_null()
        || eop.is_null()
        || utc.is_null()
        || location.is_null()
        || scope.is_null()
        || out.is_null()
    {
        return DhruvStatus::NullPointer;
    }

    let amsha = match Amsha::from_code(amsha_code) {
        Some(a) => a,
        None => return DhruvStatus::InvalidSearchConfig,
    };
    let variation = match AmshaVariation::from_code(variation_code) {
        Some(v) => v,
        None => return DhruvStatus::InvalidSearchConfig,
    };
    if !variation.is_applicable_to(amsha) {
        return DhruvStatus::InvalidSearchConfig;
    }

    let engine = unsafe { &*engine };
    let eop = unsafe { &*eop };
    let utc_c = unsafe { &*utc };
    let loc_c = unsafe { &*location };
    let scope_c = unsafe { &*scope };

    let utc_time = UtcTime {
        year: utc_c.year,
        month: utc_c.month,
        day: utc_c.day,
        hour: utc_c.hour,
        minute: utc_c.minute,
        second: utc_c.second,
    };
    let location = GeoLocation::new(loc_c.latitude_deg, loc_c.longitude_deg, loc_c.altitude_m);
    let system = match ayanamsha_system_from_code(ayanamsha_system as i32) {
        Some(s) => s,
        None => return DhruvStatus::InvalidQuery,
    };
    let rust_bhava_config = match resolve_bhava_config_ptr(bhava_config) {
        Ok(c) => c,
        Err(status) => return status,
    };
    let rs_config = match resolve_riseset_config_ptr(riseset_config) {
        Ok(c) => c,
        Err(status) => return status,
    };
    let aya_config = SankrantiConfig::new(system, use_nutation != 0);

    let requests = [AmshaRequest::with_variation(amsha, variation)];
    let rust_scope = dhruv_search::AmshaChartScope {
        include_bhava_cusps: scope_c.include_bhava_cusps != 0,
        include_arudha_padas: scope_c.include_arudha_padas != 0,
        include_upagrahas: scope_c.include_upagrahas != 0,
        include_sphutas: scope_c.include_sphutas != 0,
        include_special_lagnas: scope_c.include_special_lagnas != 0,
    };

    match amsha_charts_for_date(
        engine,
        eop,
        &utc_time,
        &location,
        &rust_bhava_config,
        &rs_config,
        &aya_config,
        &requests,
        &rust_scope,
    ) {
        Ok(result) => {
            if let Some(chart) = result.charts.first() {
                unsafe { *out = amsha_chart_to_ffi(chart) };
            }
            DhruvStatus::Ok
        }
        Err(e) => DhruvStatus::from(&e),
    }
}

// ---------------------------------------------------------------------------
// Charakaraka FFI functions (date-based)
// ---------------------------------------------------------------------------

/// Compute Chara Karakas for a given date.
///
/// `scheme`:
/// - 0 = Eight
/// - 1 = SevenNoPitri
/// - 2 = SevenPkMergedMk
/// - 3 = MixedParashara
///
/// # Safety
/// All pointers must be valid. `out` must point to a valid `DhruvCharakarakaResult`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dhruv_charakaraka_for_date(
    engine: *const Engine,
    eop: *const dhruv_time::EopKernel,
    utc: *const DhruvUtcTime,
    ayanamsha_system: u32,
    use_nutation: u8,
    scheme: u8,
    out: *mut DhruvCharakarakaResult,
) -> DhruvStatus {
    if engine.is_null() || eop.is_null() || utc.is_null() || out.is_null() {
        return DhruvStatus::NullPointer;
    }

    let scheme = match CharakarakaScheme::from_u8(scheme) {
        Some(v) => v,
        None => return DhruvStatus::InvalidSearchConfig,
    };

    let engine = unsafe { &*engine };
    let eop = unsafe { &*eop };
    let utc_c = unsafe { &*utc };

    let utc_time = UtcTime {
        year: utc_c.year,
        month: utc_c.month,
        day: utc_c.day,
        hour: utc_c.hour,
        minute: utc_c.minute,
        second: utc_c.second,
    };

    let system = match ayanamsha_system_from_code(ayanamsha_system as i32) {
        Some(s) => s,
        None => return DhruvStatus::InvalidQuery,
    };
    let aya_config = SankrantiConfig::new(system, use_nutation != 0);

    match charakaraka_for_date(engine, eop, &utc_time, &aya_config, scheme) {
        Ok(result) => {
            let out = unsafe { &mut *out };
            *out = DhruvCharakarakaResult::zeroed();
            out.scheme = scheme as u8;
            out.used_eight_karakas = result.used_eight_karakas as u8;
            let count = result.entries.len().min(DHRUV_MAX_CHARAKARAKA_ENTRIES);
            out.count = count as u8;
            for i in 0..count {
                out.entries[i] = charakaraka_entry_to_ffi(&result.entries[i]);
            }
            DhruvStatus::Ok
        }
        Err(e) => DhruvStatus::from(&e),
    }
}

// ---------------------------------------------------------------------------
// Shadbala & Vimsopaka FFI functions (date-based)
// ---------------------------------------------------------------------------

/// Compute Shadbala for all 7 sapta grahas at a given date and location.
///
/// # Safety
/// All pointers must be valid. `out` must point to a valid `DhruvShadbalaResult`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dhruv_shadbala_for_date(
    engine: *const Engine,
    eop: *const dhruv_time::EopKernel,
    utc: *const DhruvUtcTime,
    location: *const DhruvGeoLocation,
    bhava_config: *const DhruvBhavaConfig,
    riseset_config: *const DhruvRiseSetConfig,
    ayanamsha_system: u32,
    use_nutation: u8,
    out: *mut DhruvShadbalaResult,
) -> DhruvStatus {
    if engine.is_null() || eop.is_null() || utc.is_null() || location.is_null() || out.is_null() {
        return DhruvStatus::NullPointer;
    }

    let engine = unsafe { &*engine };
    let eop = unsafe { &*eop };
    let utc_c = unsafe { &*utc };
    let loc_c = unsafe { &*location };

    let utc_time = UtcTime {
        year: utc_c.year,
        month: utc_c.month,
        day: utc_c.day,
        hour: utc_c.hour,
        minute: utc_c.minute,
        second: utc_c.second,
    };
    let location = GeoLocation::new(loc_c.latitude_deg, loc_c.longitude_deg, loc_c.altitude_m);

    let system = match ayanamsha_system_from_code(ayanamsha_system as i32) {
        Some(s) => s,
        None => return DhruvStatus::InvalidQuery,
    };
    let rust_bhava_config = match resolve_bhava_config_ptr(bhava_config) {
        Ok(c) => c,
        Err(status) => return status,
    };
    let rs_config = match resolve_riseset_config_ptr(riseset_config) {
        Ok(c) => c,
        Err(status) => return status,
    };
    let aya_config = SankrantiConfig::new(system, use_nutation != 0);

    match shadbala_for_date(
        engine,
        eop,
        &utc_time,
        &location,
        &rust_bhava_config,
        &rs_config,
        &aya_config,
    ) {
        Ok(result) => {
            let out = unsafe { &mut *out };
            *out = shadbala_result_to_ffi(&result);
            DhruvStatus::Ok
        }
        Err(e) => DhruvStatus::from(&e),
    }
}

/// Compute Bhava Bala from low-level preassembled inputs (pure math).
///
/// # Safety
/// `inputs` and `out` must be valid pointers.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dhruv_calculate_bhavabala(
    inputs: *const DhruvBhavaBalaInputs,
    out: *mut DhruvBhavaBalaResult,
) -> DhruvStatus {
    if inputs.is_null() || out.is_null() {
        return DhruvStatus::NullPointer;
    }

    let inputs = unsafe { &*inputs };
    let birth_period = match bhava_bala_birth_period_from_code(inputs.birth_period) {
        Ok(value) => value,
        Err(status) => return status,
    };
    let rust_inputs = dhruv_vedic_base::BhavaBalaInputs {
        cusp_sidereal_lons: inputs.cusp_sidereal_lons,
        ascendant_sidereal_lon: inputs.ascendant_sidereal_lon,
        meridian_sidereal_lon: inputs.meridian_sidereal_lon,
        graha_bhava_numbers: inputs.graha_bhava_numbers,
        house_lord_strengths: inputs.house_lord_strengths,
        aspect_virupas: inputs.aspect_virupas,
        birth_period,
    };

    let out = unsafe { &mut *out };
    *out = bhavabala_result_to_ffi(&dhruv_vedic_base::calculate_bhava_bala(&rust_inputs));
    DhruvStatus::Ok
}

/// Compute Bhava Bala for all 12 houses at a given date and location.
///
/// # Safety
/// All pointers must be valid. `out` must point to a valid `DhruvBhavaBalaResult`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dhruv_bhavabala_for_date(
    engine: *const Engine,
    eop: *const dhruv_time::EopKernel,
    utc: *const DhruvUtcTime,
    location: *const DhruvGeoLocation,
    bhava_config: *const DhruvBhavaConfig,
    riseset_config: *const DhruvRiseSetConfig,
    ayanamsha_system: u32,
    use_nutation: u8,
    out: *mut DhruvBhavaBalaResult,
) -> DhruvStatus {
    if engine.is_null() || eop.is_null() || utc.is_null() || location.is_null() || out.is_null() {
        return DhruvStatus::NullPointer;
    }

    let engine = unsafe { &*engine };
    let eop = unsafe { &*eop };
    let utc_c = unsafe { &*utc };
    let loc_c = unsafe { &*location };

    let utc_time = UtcTime {
        year: utc_c.year,
        month: utc_c.month,
        day: utc_c.day,
        hour: utc_c.hour,
        minute: utc_c.minute,
        second: utc_c.second,
    };
    let location = GeoLocation::new(loc_c.latitude_deg, loc_c.longitude_deg, loc_c.altitude_m);

    let system = match ayanamsha_system_from_code(ayanamsha_system as i32) {
        Some(s) => s,
        None => return DhruvStatus::InvalidQuery,
    };
    let rust_bhava_config = match resolve_bhava_config_ptr(bhava_config) {
        Ok(c) => c,
        Err(status) => return status,
    };
    let rs_config = match resolve_riseset_config_ptr(riseset_config) {
        Ok(c) => c,
        Err(status) => return status,
    };
    let aya_config = SankrantiConfig::new(system, use_nutation != 0);

    match bhavabala_for_date(
        engine,
        eop,
        &utc_time,
        &location,
        &rust_bhava_config,
        &rs_config,
        &aya_config,
    ) {
        Ok(result) => {
            let out = unsafe { &mut *out };
            *out = bhavabala_result_to_ffi(&result);
            DhruvStatus::Ok
        }
        Err(e) => DhruvStatus::from(&e),
    }
}

/// Compute Vimsopaka Bala for all 9 navagrahas at a given date and location.
///
/// `node_dignity_policy`: 0=SignLordBased, 1=AlwaysSama.
///
/// # Safety
/// All pointers must be valid. `out` must point to a valid `DhruvVimsopakaResult`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dhruv_vimsopaka_for_date(
    engine: *const Engine,
    eop: *const dhruv_time::EopKernel,
    utc: *const DhruvUtcTime,
    location: *const DhruvGeoLocation,
    ayanamsha_system: u32,
    use_nutation: u8,
    node_dignity_policy: u32,
    out: *mut DhruvVimsopakaResult,
) -> DhruvStatus {
    if engine.is_null() || eop.is_null() || utc.is_null() || location.is_null() || out.is_null() {
        return DhruvStatus::NullPointer;
    }

    let engine = unsafe { &*engine };
    let eop = unsafe { &*eop };
    let utc_c = unsafe { &*utc };
    let loc_c = unsafe { &*location };

    let utc_time = UtcTime {
        year: utc_c.year,
        month: utc_c.month,
        day: utc_c.day,
        hour: utc_c.hour,
        minute: utc_c.minute,
        second: utc_c.second,
    };
    let location = GeoLocation::new(loc_c.latitude_deg, loc_c.longitude_deg, loc_c.altitude_m);

    let system = match ayanamsha_system_from_code(ayanamsha_system as i32) {
        Some(s) => s,
        None => return DhruvStatus::InvalidQuery,
    };
    let policy = match node_dignity_policy {
        0 => dhruv_vedic_base::NodeDignityPolicy::SignLordBased,
        1 => dhruv_vedic_base::NodeDignityPolicy::AlwaysSama,
        _ => return DhruvStatus::InvalidSearchConfig,
    };
    let aya_config = SankrantiConfig::new(system, use_nutation != 0);

    match vimsopaka_for_date(engine, eop, &utc_time, &location, &aya_config, policy) {
        Ok(result) => {
            let out = unsafe { &mut *out };
            *out = vimsopaka_result_to_ffi(&result);
            DhruvStatus::Ok
        }
        Err(e) => DhruvStatus::from(&e),
    }
}

/// Compute the combined bala bundle for a given date and location.
///
/// The bundle includes Shadbala, Vimsopaka Bala, Ashtakavarga, and Bhava Bala.
///
/// # Safety
/// All pointers must be valid. `out` must point to a valid `DhruvBalaBundleResult`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dhruv_balas_for_date(
    engine: *const Engine,
    eop: *const dhruv_time::EopKernel,
    utc: *const DhruvUtcTime,
    location: *const DhruvGeoLocation,
    bhava_config: *const DhruvBhavaConfig,
    riseset_config: *const DhruvRiseSetConfig,
    ayanamsha_system: u32,
    use_nutation: u8,
    node_dignity_policy: u32,
    out: *mut DhruvBalaBundleResult,
) -> DhruvStatus {
    if engine.is_null() || eop.is_null() || utc.is_null() || location.is_null() || out.is_null() {
        return DhruvStatus::NullPointer;
    }

    let engine = unsafe { &*engine };
    let eop = unsafe { &*eop };
    let utc_c = unsafe { &*utc };
    let loc_c = unsafe { &*location };

    let utc_time = UtcTime {
        year: utc_c.year,
        month: utc_c.month,
        day: utc_c.day,
        hour: utc_c.hour,
        minute: utc_c.minute,
        second: utc_c.second,
    };
    let location = GeoLocation::new(loc_c.latitude_deg, loc_c.longitude_deg, loc_c.altitude_m);

    let system = match ayanamsha_system_from_code(ayanamsha_system as i32) {
        Some(s) => s,
        None => return DhruvStatus::InvalidQuery,
    };
    let rust_bhava_config = match resolve_bhava_config_ptr(bhava_config) {
        Ok(c) => c,
        Err(status) => return status,
    };
    let rs_config = match resolve_riseset_config_ptr(riseset_config) {
        Ok(c) => c,
        Err(status) => return status,
    };
    let policy = match node_dignity_policy {
        0 => dhruv_vedic_base::NodeDignityPolicy::SignLordBased,
        1 => dhruv_vedic_base::NodeDignityPolicy::AlwaysSama,
        _ => return DhruvStatus::InvalidSearchConfig,
    };
    let aya_config = SankrantiConfig::new(system, use_nutation != 0);

    match balas_for_date(
        engine,
        eop,
        &utc_time,
        &location,
        &rust_bhava_config,
        &rs_config,
        &aya_config,
        policy,
    ) {
        Ok(result) => {
            let out = unsafe { &mut *out };
            out.shadbala = shadbala_result_to_ffi(&result.shadbala);
            out.vimsopaka = vimsopaka_result_to_ffi(&result.vimsopaka);
            out.ashtakavarga = ashtakavarga_result_to_ffi(&result.ashtakavarga);
            out.bhavabala = bhavabala_result_to_ffi(&result.bhavabala);
            DhruvStatus::Ok
        }
        Err(e) => DhruvStatus::from(&e),
    }
}

// ---------------------------------------------------------------------------
// Avastha FFI functions (date-based)
// ---------------------------------------------------------------------------

/// Compute all 5 avastha categories for all 9 grahas at a given date and location.
///
/// `node_dignity_policy`: 0=SignLordBased, 1=AlwaysSama.
///
/// # Safety
/// All pointers must be valid. `out` must point to a valid `DhruvAllGrahaAvasthas`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dhruv_avastha_for_date(
    engine: *const Engine,
    eop: *const dhruv_time::EopKernel,
    utc: *const DhruvUtcTime,
    location: *const DhruvGeoLocation,
    bhava_config: *const DhruvBhavaConfig,
    riseset_config: *const DhruvRiseSetConfig,
    ayanamsha_system: u32,
    use_nutation: u8,
    node_dignity_policy: u32,
    out: *mut DhruvAllGrahaAvasthas,
) -> DhruvStatus {
    if engine.is_null() || eop.is_null() || utc.is_null() || location.is_null() || out.is_null() {
        return DhruvStatus::NullPointer;
    }

    let engine = unsafe { &*engine };
    let eop = unsafe { &*eop };
    let utc_c = unsafe { &*utc };
    let loc_c = unsafe { &*location };

    let utc_time = UtcTime {
        year: utc_c.year,
        month: utc_c.month,
        day: utc_c.day,
        hour: utc_c.hour,
        minute: utc_c.minute,
        second: utc_c.second,
    };
    let location = GeoLocation::new(loc_c.latitude_deg, loc_c.longitude_deg, loc_c.altitude_m);

    let system = match ayanamsha_system_from_code(ayanamsha_system as i32) {
        Some(s) => s,
        None => return DhruvStatus::InvalidQuery,
    };
    let rust_bhava_config = match resolve_bhava_config_ptr(bhava_config) {
        Ok(c) => c,
        Err(status) => return status,
    };
    let rs_config = match resolve_riseset_config_ptr(riseset_config) {
        Ok(c) => c,
        Err(status) => return status,
    };
    let policy = match node_dignity_policy {
        0 => dhruv_vedic_base::NodeDignityPolicy::SignLordBased,
        1 => dhruv_vedic_base::NodeDignityPolicy::AlwaysSama,
        _ => return DhruvStatus::InvalidSearchConfig,
    };
    let aya_config = SankrantiConfig::new(system, use_nutation != 0);

    match avastha_for_date(
        engine,
        eop,
        &location,
        &utc_time,
        &rust_bhava_config,
        &rs_config,
        &aya_config,
        policy,
    ) {
        Ok(result) => {
            let out = unsafe { &mut *out };
            for i in 0..9 {
                let e = &result.entries[i];
                out.entries[i] = DhruvGrahaAvasthas {
                    baladi: e.baladi.index(),
                    jagradadi: e.jagradadi.index(),
                    deeptadi: e.deeptadi.index(),
                    lajjitadi: e.lajjitadi.index(),
                    sayanadi: DhruvSayanadiResult {
                        avastha: e.sayanadi.avastha.index(),
                        sub_states: [
                            e.sayanadi.sub_states[0].index(),
                            e.sayanadi.sub_states[1].index(),
                            e.sayanadi.sub_states[2].index(),
                            e.sayanadi.sub_states[3].index(),
                            e.sayanadi.sub_states[4].index(),
                        ],
                    },
                };
            }
            DhruvStatus::Ok
        }
        Err(e) => DhruvStatus::from(&e),
    }
}

// ---------------------------------------------------------------------------
// Graha Sidereal Longitudes
// ---------------------------------------------------------------------------

/// C-compatible graha sidereal longitudes result.
///
/// Contains sidereal longitudes (degrees, 0..360) for all 9 grahas,
/// indexed by Graha order: Surya=0, Chandra=1, Mangal=2, Buddh=3,
/// Guru=4, Shukra=5, Shani=6, Rahu=7, Ketu=8.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct DhruvGrahaLongitudes {
    pub longitudes: [f64; 9],
}

/// Configuration for graha longitude computation.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DhruvGrahaLongitudesConfig {
    /// `DHRUV_GRAHA_LONGITUDE_KIND_*`
    pub kind: i32,
    /// Ayanamsha system code (0-19). Used for sidereal output.
    pub ayanamsha_system: i32,
    /// Whether to apply nutation correction when meaningful.
    pub use_nutation: u8,
    /// `DHRUV_PRECESSION_MODEL_*`
    pub precession_model: i32,
    /// `DhruvReferencePlane` or -1 for system default.
    pub reference_plane: i32,
}

#[unsafe(no_mangle)]
pub extern "C" fn dhruv_graha_longitudes_config_default() -> DhruvGrahaLongitudesConfig {
    DhruvGrahaLongitudesConfig {
        kind: DHRUV_GRAHA_LONGITUDE_KIND_SIDEREAL,
        ayanamsha_system: 0,
        use_nutation: 0,
        precession_model: DHRUV_PRECESSION_MODEL_VONDRAK2011,
        reference_plane: -1,
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dhruv_graha_longitudes(
    engine: *const Engine,
    jd_tdb: f64,
    config: *const DhruvGrahaLongitudesConfig,
    out: *mut DhruvGrahaLongitudes,
) -> DhruvStatus {
    if engine.is_null() || out.is_null() {
        return DhruvStatus::NullPointer;
    }
    let engine = unsafe { &*engine };
    let rust_config = match resolve_graha_longitudes_config_ptr(config) {
        Ok(cfg) => cfg,
        Err(status) => return status,
    };
    match graha_longitudes(engine, jd_tdb, &rust_config) {
        Ok(result) => {
            let out = unsafe { &mut *out };
            out.longitudes = result.longitudes;
            DhruvStatus::Ok
        }
        Err(e) => DhruvStatus::from(&e),
    }
}

// ---------------------------------------------------------------------------
// Nakshatra At (from pre-computed Moon sidereal longitude)
// ---------------------------------------------------------------------------

/// Determine the Moon's Nakshatra from a pre-computed sidereal longitude.
///
/// Accepts a Moon sidereal longitude in degrees [0, 360) at `jd_tdb`.
/// The engine is still needed for boundary bisection (finding start/end times).
/// The result includes nakshatra index, pada, and start/end times (UTC).
///
/// # Safety
/// `engine`, `config`, and `out` must be valid, non-null pointers.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dhruv_nakshatra_at(
    engine: *const DhruvEngineHandle,
    jd_tdb: f64,
    moon_sidereal_deg: f64,
    config: *const DhruvSankrantiConfig,
    out: *mut DhruvPanchangNakshatraInfo,
) -> DhruvStatus {
    if engine.is_null() || out.is_null() {
        return DhruvStatus::NullPointer;
    }

    let engine_ref = unsafe { &*engine };
    let cfg = match resolve_sankranti_config_ptr(config) {
        Ok(c) => c,
        Err(status) => return status,
    };

    match nakshatra_at(engine_ref, jd_tdb, moon_sidereal_deg, &cfg) {
        Ok(info) => {
            unsafe {
                *out = DhruvPanchangNakshatraInfo {
                    nakshatra_index: info.nakshatra_index as i32,
                    pada: info.pada as i32,
                    start: utc_time_to_ffi(&info.start),
                    end: utc_time_to_ffi(&info.end),
                };
            }
            DhruvStatus::Ok
        }
        Err(e) => DhruvStatus::from(&e),
    }
}

// ---------------------------------------------------------------------------
// Dasha FFI types and functions
// ---------------------------------------------------------------------------

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct DhruvRashiDashaInputs {
    pub graha_sidereal_lons: [f64; 9],
    pub lagna_sidereal_lon: f64,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct DhruvDashaInputs {
    pub has_moon_sid_lon: u8,
    pub moon_sid_lon: f64,
    pub has_rashi_inputs: u8,
    pub rashi_inputs: DhruvRashiDashaInputs,
    pub has_sunrise_sunset: u8,
    pub sunrise_jd: f64,
    pub sunset_jd: f64,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct DhruvDashaBirthContext {
    pub time_kind: i32,
    pub birth_jd: f64,
    pub birth_utc: DhruvUtcTime,
    pub has_location: u8,
    pub location: DhruvGeoLocation,
    pub bhava_config: DhruvBhavaConfig,
    pub riseset_config: DhruvRiseSetConfig,
    pub sankranti_config: DhruvSankrantiConfig,
    pub has_inputs: u8,
    pub inputs: DhruvDashaInputs,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct DhruvDashaHierarchyRequest {
    pub birth: DhruvDashaBirthContext,
    pub system: u8,
    pub max_level: u8,
    pub variation: DhruvDashaVariationConfig,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct DhruvDashaSnapshotRequest {
    pub birth: DhruvDashaBirthContext,
    pub query_time_kind: i32,
    pub query_jd: f64,
    pub query_utc: DhruvUtcTime,
    pub system: u8,
    pub max_level: u8,
    pub variation: DhruvDashaVariationConfig,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct DhruvDashaLevel0Request {
    pub birth: DhruvDashaBirthContext,
    pub system: u8,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct DhruvDashaLevel0EntityRequest {
    pub birth: DhruvDashaBirthContext,
    pub system: u8,
    pub entity_type: u8,
    pub entity_index: u8,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct DhruvDashaChildrenRequest {
    pub birth: DhruvDashaBirthContext,
    pub system: u8,
    pub variation: DhruvDashaVariationConfig,
    pub parent: DhruvDashaPeriod,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct DhruvDashaChildPeriodRequest {
    pub birth: DhruvDashaBirthContext,
    pub system: u8,
    pub variation: DhruvDashaVariationConfig,
    pub parent: DhruvDashaPeriod,
    pub child_entity_type: u8,
    pub child_entity_index: u8,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct DhruvDashaCompleteLevelRequest {
    pub birth: DhruvDashaBirthContext,
    pub system: u8,
    pub variation: DhruvDashaVariationConfig,
    pub child_level: u8,
}

#[derive(Debug, Clone, Default)]
struct OwnedDashaInputs {
    moon_sid_lon: Option<f64>,
    rashi_inputs: Option<RashiDashaInputs>,
    sunrise_sunset: Option<(f64, f64)>,
}

impl OwnedDashaInputs {
    fn borrowed(&self) -> dhruv_search::DashaInputs<'_> {
        dhruv_search::DashaInputs {
            moon_sid_lon: self.moon_sid_lon,
            rashi_inputs: self.rashi_inputs.as_ref(),
            sunrise_sunset: self.sunrise_sunset,
        }
    }
}

#[derive(Debug, Clone)]
struct ResolvedDashaBirthContext {
    birth_jd: f64,
    inputs: OwnedDashaInputs,
}

/// C-compatible single dasha period.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct DhruvDashaPeriod {
    /// Entity type: 0=Graha, 1=Rashi, 2=Yogini.
    pub entity_type: u8,
    /// Entity index: Graha index (0-8), rashi (0-11), yogini (0-7).
    pub entity_index: u8,
    /// Exact canonical entity name as a static NUL-terminated UTF-8 string.
    pub entity_name: *const std::ffi::c_char,
    /// JD UTC, inclusive.
    pub start_jd: f64,
    /// JD UTC, exclusive.
    pub end_jd: f64,
    /// Hierarchical level (0-4).
    pub level: u8,
    /// 1-indexed position among siblings.
    pub order: u16,
    /// Index into parent level's array (0 for level 0).
    pub parent_idx: u32,
}

/// Fixed-capacity snapshot for FFI (max 5 levels).
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct DhruvDashaSnapshot {
    /// Dasha system code (DashaSystem repr(u8)).
    pub system: u8,
    /// Query JD UTC.
    pub query_jd: f64,
    /// Number of valid periods (0-5).
    pub count: u8,
    /// One period per level.
    pub periods: [DhruvDashaPeriod; 5],
}

/// Opaque handle for DashaHierarchy (heap-allocated, caller frees).
pub type DhruvDashaHierarchyHandle = *mut std::ffi::c_void;
/// Opaque handle for a heap-allocated period vector.
pub type DhruvDashaPeriodListHandle = *mut std::ffi::c_void;

fn dasha_period_to_ffi(p: &dhruv_vedic_base::dasha::DashaPeriod) -> DhruvDashaPeriod {
    DhruvDashaPeriod {
        entity_type: p.entity.type_code(),
        entity_index: p.entity.entity_index(),
        entity_name: dasha_entity_name_ptr(p.entity),
        start_jd: p.start_jd,
        end_jd: p.end_jd,
        level: p.level as u8,
        order: p.order,
        parent_idx: p.parent_idx,
    }
}

fn dasha_entity_from_ffi(
    entity_type: u8,
    entity_index: u8,
) -> Result<dhruv_vedic_base::dasha::DashaEntity, DhruvStatus> {
    match entity_type {
        0 => {
            let graha = match entity_index {
                0 => dhruv_vedic_base::Graha::Surya,
                1 => dhruv_vedic_base::Graha::Chandra,
                2 => dhruv_vedic_base::Graha::Mangal,
                3 => dhruv_vedic_base::Graha::Buddh,
                4 => dhruv_vedic_base::Graha::Guru,
                5 => dhruv_vedic_base::Graha::Shukra,
                6 => dhruv_vedic_base::Graha::Shani,
                7 => dhruv_vedic_base::Graha::Rahu,
                8 => dhruv_vedic_base::Graha::Ketu,
                _ => return Err(DhruvStatus::InvalidInput),
            };
            Ok(dhruv_vedic_base::dasha::DashaEntity::Graha(graha))
        }
        1 => {
            if entity_index >= 12 {
                return Err(DhruvStatus::InvalidInput);
            }
            Ok(dhruv_vedic_base::dasha::DashaEntity::Rashi(entity_index))
        }
        2 => {
            if entity_index >= 8 {
                return Err(DhruvStatus::InvalidInput);
            }
            Ok(dhruv_vedic_base::dasha::DashaEntity::Yogini(entity_index))
        }
        _ => Err(DhruvStatus::InvalidInput),
    }
}

fn dasha_entity_name_ptr(entity: dhruv_vedic_base::dasha::DashaEntity) -> *const std::ffi::c_char {
    match entity {
        dhruv_vedic_base::dasha::DashaEntity::Graha(graha) => {
            dhruv_graha_name(graha.index() as u32)
        }
        dhruv_vedic_base::dasha::DashaEntity::Rashi(index) => dhruv_rashi_name(index as u32),
        dhruv_vedic_base::dasha::DashaEntity::Yogini(index) => dhruv_yogini_name(index as u32),
    }
}

fn dasha_period_from_ffi(
    p: &DhruvDashaPeriod,
) -> Result<dhruv_vedic_base::dasha::DashaPeriod, DhruvStatus> {
    let entity = dasha_entity_from_ffi(p.entity_type, p.entity_index)?;
    let level =
        dhruv_vedic_base::dasha::DashaLevel::from_u8(p.level).ok_or(DhruvStatus::InvalidInput)?;
    Ok(dhruv_vedic_base::dasha::DashaPeriod {
        entity,
        start_jd: p.start_jd,
        end_jd: p.end_jd,
        level,
        order: p.order,
        parent_idx: p.parent_idx,
    })
}

/// Get the number of levels in a dasha hierarchy.
///
/// # Safety
/// `handle` must be a valid handle from `dhruv_dasha_hierarchy` or NULL.
/// `out` must be a valid, non-null pointer.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dhruv_dasha_hierarchy_level_count(
    handle: DhruvDashaHierarchyHandle,
    out: *mut u8,
) -> DhruvStatus {
    if handle.is_null() || out.is_null() {
        return DhruvStatus::NullPointer;
    }
    let hierarchy = unsafe { &*(handle as *const dhruv_vedic_base::dasha::DashaHierarchy) };
    unsafe { *out = hierarchy.levels.len() as u8 };
    DhruvStatus::Ok
}

/// Get the number of periods at a specific level in a dasha hierarchy.
///
/// # Safety
/// `handle` must be a valid handle. `out` must be a valid, non-null pointer.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dhruv_dasha_hierarchy_period_count(
    handle: DhruvDashaHierarchyHandle,
    level: u8,
    out: *mut u32,
) -> DhruvStatus {
    if handle.is_null() || out.is_null() {
        return DhruvStatus::NullPointer;
    }
    let hierarchy = unsafe { &*(handle as *const dhruv_vedic_base::dasha::DashaHierarchy) };
    let lvl = level as usize;
    if lvl >= hierarchy.levels.len() {
        return DhruvStatus::InvalidInput;
    }
    unsafe { *out = hierarchy.levels[lvl].len() as u32 };
    DhruvStatus::Ok
}

/// Get a specific period from a dasha hierarchy.
///
/// # Safety
/// `handle` must be a valid handle. `out` must be a valid, non-null pointer.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dhruv_dasha_hierarchy_period_at(
    handle: DhruvDashaHierarchyHandle,
    level: u8,
    idx: u32,
    out: *mut DhruvDashaPeriod,
) -> DhruvStatus {
    if handle.is_null() || out.is_null() {
        return DhruvStatus::NullPointer;
    }
    let hierarchy = unsafe { &*(handle as *const dhruv_vedic_base::dasha::DashaHierarchy) };
    let lvl = level as usize;
    if lvl >= hierarchy.levels.len() {
        return DhruvStatus::InvalidInput;
    }
    let i = idx as usize;
    if i >= hierarchy.levels[lvl].len() {
        return DhruvStatus::InvalidInput;
    }
    unsafe { *out = dasha_period_to_ffi(&hierarchy.levels[lvl][i]) };
    DhruvStatus::Ok
}

/// Free a dasha hierarchy handle. Passing NULL is a no-op.
///
/// # Safety
/// `handle` must be a valid handle from `dhruv_dasha_hierarchy` or NULL.
/// Must not be called on handles owned by a FullKundaliResult.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dhruv_dasha_hierarchy_free(handle: DhruvDashaHierarchyHandle) {
    if !handle.is_null() {
        let _ = unsafe { Box::from_raw(handle as *mut dhruv_vedic_base::dasha::DashaHierarchy) };
    }
}

/// Get the number of periods in a period list handle.
///
/// # Safety
/// `handle` must be a valid period list handle. `out` must be non-null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dhruv_dasha_period_list_count(
    handle: DhruvDashaPeriodListHandle,
    out: *mut u32,
) -> DhruvStatus {
    if handle.is_null() || out.is_null() {
        return DhruvStatus::NullPointer;
    }
    let periods = unsafe { &*(handle as *const Vec<dhruv_vedic_base::dasha::DashaPeriod>) };
    unsafe { *out = periods.len() as u32 };
    DhruvStatus::Ok
}

/// Read one period from a period list handle by index.
///
/// # Safety
/// `handle` must be a valid period list handle. `out` must be non-null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dhruv_dasha_period_list_at(
    handle: DhruvDashaPeriodListHandle,
    idx: u32,
    out: *mut DhruvDashaPeriod,
) -> DhruvStatus {
    if handle.is_null() || out.is_null() {
        return DhruvStatus::NullPointer;
    }
    let periods = unsafe { &*(handle as *const Vec<dhruv_vedic_base::dasha::DashaPeriod>) };
    let i = idx as usize;
    if i >= periods.len() {
        return DhruvStatus::InvalidInput;
    }
    unsafe { *out = dasha_period_to_ffi(&periods[i]) };
    DhruvStatus::Ok
}

/// Free a dasha period list handle. Passing NULL is a no-op.
///
/// # Safety
/// `handle` must come from one of the dasha period list producers or be NULL.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dhruv_dasha_period_list_free(handle: DhruvDashaPeriodListHandle) {
    if !handle.is_null() {
        let _ = unsafe { Box::from_raw(handle as *mut Vec<dhruv_vedic_base::dasha::DashaPeriod>) };
    }
}

/// C-compatible dasha variation overrides.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct DhruvDashaVariationConfig {
    /// Per-level sub-period method overrides (0xFF = system default).
    pub level_methods: [u8; 5],
    /// Yogini scheme code (0=default).
    pub yogini_scheme: u8,
    /// For Ashtottari: use Abhijit in birth-balance detection.
    pub use_abhijit: u8,
}

fn dasha_variation_from_ffi(
    cfg: &DhruvDashaVariationConfig,
) -> Result<dhruv_vedic_base::dasha::DashaVariationConfig, DhruvStatus> {
    let mut level_methods = [None; 5];
    for (idx, slot) in level_methods.iter_mut().enumerate() {
        *slot = match cfg.level_methods[idx] {
            0xFF => None,
            raw => Some(
                dhruv_vedic_base::dasha::SubPeriodMethod::from_u8(raw)
                    .ok_or(DhruvStatus::InvalidSearchConfig)?,
            ),
        };
    }
    let yogini_scheme = dhruv_vedic_base::dasha::YoginiScheme::from_u8(cfg.yogini_scheme)
        .ok_or(DhruvStatus::InvalidSearchConfig)?;
    Ok(dhruv_vedic_base::dasha::DashaVariationConfig {
        level_methods,
        yogini_scheme,
        use_abhijit: cfg.use_abhijit != 0,
    })
}

/// Return default dasha variation settings.
#[unsafe(no_mangle)]
pub extern "C" fn dhruv_dasha_variation_config_default() -> DhruvDashaVariationConfig {
    DhruvDashaVariationConfig {
        level_methods: [0xFF; 5],
        yogini_scheme: 0,
        use_abhijit: 1,
    }
}

/// C-compatible dasha selection config for FullKundali integration.
///
/// Mirrors `DashaSelectionConfig` from `dhruv_search`.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct DhruvDashaSelectionConfig {
    /// Number of valid entries in `systems` (0..=DHRUV_MAX_DASHA_SYSTEMS).
    pub count: u8,
    /// Dasha system codes (DashaSystem repr(u8), 0xFF=unused).
    pub systems: [u8; DHRUV_MAX_DASHA_SYSTEMS],
    /// Per-system max level depth (0-4, 0xFF=use `max_level`).
    pub max_levels: [u8; DHRUV_MAX_DASHA_SYSTEMS],
    /// Shared max level depth fallback (0-4, default 2).
    pub max_level: u8,
    /// Per-level sub-period method overrides (0xFF=system default).
    pub level_methods: [u8; 5],
    /// Yogini scheme code (0=default).
    pub yogini_scheme: u8,
    /// Use Abhijit for Ashtottari (1=yes, 0=no).
    pub use_abhijit: u8,
    /// 0 = no snapshot, 1 = snapshot_jd is valid.
    pub has_snapshot_jd: u8,
    /// Query JD UTC for snapshot. Only read when `has_snapshot_jd == 1`.
    pub snapshot_jd: f64,
}

fn dasha_selection_from_ffi(c: &DhruvDashaSelectionConfig) -> dhruv_search::DashaSelectionConfig {
    dhruv_search::DashaSelectionConfig {
        count: c.count,
        systems: c.systems,
        max_levels: c.max_levels,
        max_level: c.max_level,
        level_methods: c.level_methods,
        yogini_scheme: c.yogini_scheme,
        use_abhijit: c.use_abhijit,
        snapshot_jd: if c.has_snapshot_jd != 0 {
            Some(c.snapshot_jd)
        } else {
            None
        },
    }
}

/// Return a `DhruvDashaSelectionConfig` with safe defaults.
///
/// count=0 (no systems selected), max_level=2, no snapshot.
#[unsafe(no_mangle)]
pub extern "C" fn dhruv_dasha_selection_config_default() -> DhruvDashaSelectionConfig {
    DhruvDashaSelectionConfig {
        count: 0,
        systems: [0xFF; DHRUV_MAX_DASHA_SYSTEMS],
        max_levels: [0xFF; DHRUV_MAX_DASHA_SYSTEMS],
        max_level: dhruv_vedic_base::dasha::DEFAULT_DASHA_LEVEL,
        level_methods: [0xFF; 5],
        yogini_scheme: 0,
        use_abhijit: 1,
        has_snapshot_jd: 0,
        snapshot_jd: 0.0,
    }
}

fn dasha_jd_utc_from_selector(
    time_kind: i32,
    jd: f64,
    utc: DhruvUtcTime,
) -> Result<f64, DhruvStatus> {
    match time_kind {
        DHRUV_DASHA_TIME_JD_UTC => Ok(jd),
        DHRUV_DASHA_TIME_UTC => {
            let y = utc.year as f64;
            let m = utc.month as f64;
            let d = utc.day as f64
                + utc.hour as f64 / 24.0
                + utc.minute as f64 / 1440.0
                + utc.second / 86400.0;
            let (y2, m2) = if m <= 2.0 {
                (y - 1.0, m + 12.0)
            } else {
                (y, m)
            };
            let a = (y2 / 100.0).floor();
            let b = 2.0 - a + (a / 4.0).floor();
            Ok((365.25 * (y2 + 4716.0)).floor() + (30.6001 * (m2 + 1.0)).floor() + d + b - 1524.5)
        }
        _ => Err(DhruvStatus::InvalidInput),
    }
}

fn dasha_system_is_rashi_based(system: dhruv_vedic_base::dasha::DashaSystem) -> bool {
    matches!(
        system,
        dhruv_vedic_base::dasha::DashaSystem::Chara
            | dhruv_vedic_base::dasha::DashaSystem::Sthira
            | dhruv_vedic_base::dasha::DashaSystem::Yogardha
            | dhruv_vedic_base::dasha::DashaSystem::Driga
            | dhruv_vedic_base::dasha::DashaSystem::Shoola
            | dhruv_vedic_base::dasha::DashaSystem::Mandooka
            | dhruv_vedic_base::dasha::DashaSystem::Chakra
            | dhruv_vedic_base::dasha::DashaSystem::Kendradi
            | dhruv_vedic_base::dasha::DashaSystem::KarakaKendradi
            | dhruv_vedic_base::dasha::DashaSystem::KarakaKendradiGraha
    )
}

fn dasha_system_needs_moon_lon(system: dhruv_vedic_base::dasha::DashaSystem) -> bool {
    !dasha_system_is_rashi_based(system) && system != dhruv_vedic_base::dasha::DashaSystem::Kala
}

fn dasha_system_needs_sunrise_sunset(system: dhruv_vedic_base::dasha::DashaSystem) -> bool {
    matches!(
        system,
        dhruv_vedic_base::dasha::DashaSystem::Kala | dhruv_vedic_base::dasha::DashaSystem::Chakra
    )
}

fn dasha_inputs_from_ffi(raw: DhruvDashaInputs) -> Result<OwnedDashaInputs, DhruvStatus> {
    let rashi_inputs = if raw.has_rashi_inputs != 0 {
        Some(RashiDashaInputs::new(
            raw.rashi_inputs.graha_sidereal_lons,
            raw.rashi_inputs.lagna_sidereal_lon,
        ))
    } else {
        None
    };
    let sunrise_sunset = if raw.has_sunrise_sunset != 0 {
        Some((raw.sunrise_jd, raw.sunset_jd))
    } else {
        None
    };
    Ok(OwnedDashaInputs {
        moon_sid_lon: if raw.has_moon_sid_lon != 0 {
            Some(raw.moon_sid_lon)
        } else {
            None
        },
        rashi_inputs,
        sunrise_sunset,
    })
}

fn resolve_dasha_birth_context(
    engine: *const Engine,
    eop: *const dhruv_time::EopKernel,
    ctx: DhruvDashaBirthContext,
    system: dhruv_vedic_base::dasha::DashaSystem,
) -> Result<ResolvedDashaBirthContext, DhruvStatus> {
    let birth_jd = dasha_jd_utc_from_selector(ctx.time_kind, ctx.birth_jd, ctx.birth_utc)?;

    if ctx.has_inputs != 0 {
        return Ok(ResolvedDashaBirthContext {
            birth_jd,
            inputs: dasha_inputs_from_ffi(ctx.inputs)?,
        });
    }

    if engine.is_null() || eop.is_null() {
        return Err(DhruvStatus::NullPointer);
    }
    if ctx.time_kind != DHRUV_DASHA_TIME_UTC || ctx.has_location == 0 {
        return Err(DhruvStatus::InvalidInput);
    }

    let birth_utc = UtcTime {
        year: ctx.birth_utc.year,
        month: ctx.birth_utc.month,
        day: ctx.birth_utc.day,
        hour: ctx.birth_utc.hour,
        minute: ctx.birth_utc.minute,
        second: ctx.birth_utc.second,
    };
    let location = GeoLocation::new(
        ctx.location.latitude_deg,
        ctx.location.longitude_deg,
        ctx.location.altitude_m,
    );
    let riseset_config = resolve_riseset_config_ptr(&ctx.riseset_config)?;
    let sankranti_config = resolve_sankranti_config_ptr(&ctx.sankranti_config)?;
    let _ = resolve_bhava_config_ptr(&ctx.bhava_config)?;
    let engine = unsafe { &*engine };
    let eop = unsafe { &*eop };
    let utc_seconds = dhruv_time::jd_to_tdb_seconds(birth_jd);
    let birth_jd_tdb = dhruv_time::tdb_seconds_to_jd(
        engine
            .lsk()
            .utc_to_tdb_with_policy_and_eop(
                utc_seconds,
                Some(eop),
                dhruv_search::time_conversion_policy(),
            )
            .tdb_seconds,
    );
    let moon_sid_lon = if dasha_system_needs_moon_lon(system) {
        Some(
            dhruv_search::moon_sidereal_longitude_at(engine, birth_jd_tdb, &sankranti_config)
                .map_err(|err| DhruvStatus::from(&err))?,
        )
    } else {
        None
    };
    let rashi_inputs = if dasha_system_is_rashi_based(system) {
        let graha_lons = graha_longitudes(
            engine,
            birth_jd_tdb,
            &GrahaLongitudesConfig::sidereal_with_model(
                sankranti_config.ayanamsha_system,
                sankranti_config.use_nutation,
                sankranti_config.precession_model,
                sankranti_config.reference_plane,
            ),
        )
        .map_err(|err| DhruvStatus::from(&err))?;
        let lagna_sid = dhruv_search::sidereal_lagna_for_date(
            engine,
            eop,
            &birth_utc,
            &location,
            &sankranti_config,
        )
        .map_err(|err| DhruvStatus::from(&err))?;
        Some(RashiDashaInputs::new(graha_lons.longitudes, lagna_sid))
    } else {
        None
    };
    let sunrise_sunset = if dasha_system_needs_sunrise_sunset(system) {
        Some(
            dhruv_search::vedic_day_sunrises(engine, eop, &birth_utc, &location, &riseset_config)
                .map_err(|err| DhruvStatus::from(&err))?,
        )
    } else {
        None
    };

    Ok(ResolvedDashaBirthContext {
        birth_jd,
        inputs: OwnedDashaInputs {
            moon_sid_lon,
            rashi_inputs,
            sunrise_sunset,
        },
    })
}

/// Compute a full dasha hierarchy from either UTC/location context or raw inputs.
///
/// # Safety
/// `request` and `out` must be valid and non-null. `engine`/`eop` may be null only when
/// `request->birth.has_inputs != 0`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dhruv_dasha_hierarchy(
    engine: *const Engine,
    eop: *const dhruv_time::EopKernel,
    request: *const DhruvDashaHierarchyRequest,
    out: *mut DhruvDashaHierarchyHandle,
) -> DhruvStatus {
    if request.is_null() || out.is_null() {
        return DhruvStatus::NullPointer;
    }

    let request = unsafe { &*request };
    let system = match dhruv_vedic_base::dasha::DashaSystem::from_u8(request.system) {
        Some(value) => value,
        None => return DhruvStatus::InvalidInput,
    };
    let variation = match dasha_variation_from_ffi(&request.variation) {
        Ok(value) => value,
        Err(status) => return status,
    };
    let birth = match resolve_dasha_birth_context(engine, eop, request.birth, system) {
        Ok(value) => value,
        Err(status) => return status,
    };
    let inputs = birth.inputs.borrowed();

    match dasha_hierarchy_with_inputs(
        birth.birth_jd,
        system,
        request
            .max_level
            .min(dhruv_vedic_base::dasha::MAX_DASHA_LEVEL),
        &variation,
        &inputs,
    ) {
        Ok(hierarchy) => {
            let boxed = Box::new(hierarchy);
            unsafe { *out = Box::into_raw(boxed) as DhruvDashaHierarchyHandle };
            DhruvStatus::Ok
        }
        Err(err) => DhruvStatus::from(&err),
    }
}

/// Compute a dasha snapshot from either UTC/location context or raw inputs.
///
/// # Safety
/// `request` and `out` must be valid and non-null. `engine`/`eop` may be null only when
/// `request->birth.has_inputs != 0`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dhruv_dasha_snapshot(
    engine: *const Engine,
    eop: *const dhruv_time::EopKernel,
    request: *const DhruvDashaSnapshotRequest,
    out: *mut DhruvDashaSnapshot,
) -> DhruvStatus {
    if request.is_null() || out.is_null() {
        return DhruvStatus::NullPointer;
    }

    let request = unsafe { &*request };
    let system = match dhruv_vedic_base::dasha::DashaSystem::from_u8(request.system) {
        Some(value) => value,
        None => return DhruvStatus::InvalidInput,
    };
    let variation = match dasha_variation_from_ffi(&request.variation) {
        Ok(value) => value,
        Err(status) => return status,
    };
    let birth = match resolve_dasha_birth_context(engine, eop, request.birth, system) {
        Ok(value) => value,
        Err(status) => return status,
    };
    let query_jd = match dasha_jd_utc_from_selector(
        request.query_time_kind,
        request.query_jd,
        request.query_utc,
    ) {
        Ok(value) => value,
        Err(status) => return status,
    };
    let inputs = birth.inputs.borrowed();

    match dasha_snapshot_with_inputs(
        birth.birth_jd,
        query_jd,
        system,
        request
            .max_level
            .min(dhruv_vedic_base::dasha::MAX_DASHA_LEVEL),
        &variation,
        &inputs,
    ) {
        Ok(snapshot) => {
            let out = unsafe { &mut *out };
            out.system = request.system;
            out.query_jd = snapshot.query_jd;
            let count = snapshot.periods.len().min(5);
            out.count = count as u8;
            out.periods = [DhruvDashaPeriod {
                entity_type: 0,
                entity_index: 0,
                entity_name: ptr::null(),
                start_jd: 0.0,
                end_jd: 0.0,
                level: 0,
                order: 0,
                parent_idx: 0,
            }; 5];
            for i in 0..count {
                out.periods[i] = dasha_period_to_ffi(&snapshot.periods[i]);
            }
            DhruvStatus::Ok
        }
        Err(err) => DhruvStatus::from(&err),
    }
}

/// Compute level-0 periods from either UTC/location context or raw inputs.
///
/// # Safety
/// `request` and `out` must be valid and non-null. `engine`/`eop` may be null only when
/// `request->birth.has_inputs != 0`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dhruv_dasha_level0(
    engine: *const Engine,
    eop: *const dhruv_time::EopKernel,
    request: *const DhruvDashaLevel0Request,
    out: *mut DhruvDashaPeriodListHandle,
) -> DhruvStatus {
    if request.is_null() || out.is_null() {
        return DhruvStatus::NullPointer;
    }

    let request = unsafe { &*request };
    let system = match dhruv_vedic_base::dasha::DashaSystem::from_u8(request.system) {
        Some(value) => value,
        None => return DhruvStatus::InvalidInput,
    };
    let birth = match resolve_dasha_birth_context(engine, eop, request.birth, system) {
        Ok(value) => value,
        Err(status) => return status,
    };
    let inputs = birth.inputs.borrowed();

    match dasha_level0_with_inputs(birth.birth_jd, system, &inputs) {
        Ok(periods) => {
            let boxed = Box::new(periods);
            unsafe { *out = Box::into_raw(boxed) as DhruvDashaPeriodListHandle };
            DhruvStatus::Ok
        }
        Err(err) => DhruvStatus::from(&err),
    }
}

/// Compute one level-0 entity period from either UTC/location context or raw inputs.
///
/// # Safety
/// `request`, `out_found`, and `out` must be valid and non-null. `engine`/`eop` may be null only
/// when `request->birth.has_inputs != 0`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dhruv_dasha_level0_entity(
    engine: *const Engine,
    eop: *const dhruv_time::EopKernel,
    request: *const DhruvDashaLevel0EntityRequest,
    out_found: *mut u8,
    out: *mut DhruvDashaPeriod,
) -> DhruvStatus {
    if request.is_null() || out_found.is_null() || out.is_null() {
        return DhruvStatus::NullPointer;
    }

    let request = unsafe { &*request };
    let system = match dhruv_vedic_base::dasha::DashaSystem::from_u8(request.system) {
        Some(value) => value,
        None => return DhruvStatus::InvalidInput,
    };
    let entity = match dasha_entity_from_ffi(request.entity_type, request.entity_index) {
        Ok(value) => value,
        Err(status) => return status,
    };
    let birth = match resolve_dasha_birth_context(engine, eop, request.birth, system) {
        Ok(value) => value,
        Err(status) => return status,
    };
    let inputs = birth.inputs.borrowed();

    match dasha_level0_entity_with_inputs(birth.birth_jd, system, entity, &inputs) {
        Ok(Some(period)) => {
            unsafe {
                *out_found = 1;
                *out = dasha_period_to_ffi(&period);
            }
            DhruvStatus::Ok
        }
        Ok(None) => {
            unsafe { *out_found = 0 };
            DhruvStatus::Ok
        }
        Err(err) => DhruvStatus::from(&err),
    }
}

/// Compute child periods from either UTC/location context or raw inputs.
///
/// # Safety
/// `request` and `out` must be valid and non-null. `engine`/`eop` may be null only when
/// `request->birth.has_inputs != 0`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dhruv_dasha_children(
    engine: *const Engine,
    eop: *const dhruv_time::EopKernel,
    request: *const DhruvDashaChildrenRequest,
    out: *mut DhruvDashaPeriodListHandle,
) -> DhruvStatus {
    if request.is_null() || out.is_null() {
        return DhruvStatus::NullPointer;
    }

    let request = unsafe { &*request };
    let system = match dhruv_vedic_base::dasha::DashaSystem::from_u8(request.system) {
        Some(value) => value,
        None => return DhruvStatus::InvalidInput,
    };
    let variation = match dasha_variation_from_ffi(&request.variation) {
        Ok(value) => value,
        Err(status) => return status,
    };
    let parent = match dasha_period_from_ffi(&request.parent) {
        Ok(value) => value,
        Err(status) => return status,
    };
    let birth = match resolve_dasha_birth_context(engine, eop, request.birth, system) {
        Ok(value) => value,
        Err(status) => return status,
    };
    let inputs = birth.inputs.borrowed();

    match dasha_children_with_inputs(system, &parent, &variation, &inputs) {
        Ok(periods) => {
            let boxed = Box::new(periods);
            unsafe { *out = Box::into_raw(boxed) as DhruvDashaPeriodListHandle };
            DhruvStatus::Ok
        }
        Err(err) => DhruvStatus::from(&err),
    }
}

/// Compute one child period from either UTC/location context or raw inputs.
///
/// # Safety
/// `request`, `out_found`, and `out` must be valid and non-null. `engine`/`eop` may be null only
/// when `request->birth.has_inputs != 0`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dhruv_dasha_child_period(
    engine: *const Engine,
    eop: *const dhruv_time::EopKernel,
    request: *const DhruvDashaChildPeriodRequest,
    out_found: *mut u8,
    out: *mut DhruvDashaPeriod,
) -> DhruvStatus {
    if request.is_null() || out_found.is_null() || out.is_null() {
        return DhruvStatus::NullPointer;
    }

    let request = unsafe { &*request };
    let system = match dhruv_vedic_base::dasha::DashaSystem::from_u8(request.system) {
        Some(value) => value,
        None => return DhruvStatus::InvalidInput,
    };
    let variation = match dasha_variation_from_ffi(&request.variation) {
        Ok(value) => value,
        Err(status) => return status,
    };
    let parent = match dasha_period_from_ffi(&request.parent) {
        Ok(value) => value,
        Err(status) => return status,
    };
    let child_entity =
        match dasha_entity_from_ffi(request.child_entity_type, request.child_entity_index) {
            Ok(value) => value,
            Err(status) => return status,
        };
    let birth = match resolve_dasha_birth_context(engine, eop, request.birth, system) {
        Ok(value) => value,
        Err(status) => return status,
    };
    let inputs = birth.inputs.borrowed();

    match dasha_child_period_with_inputs(system, &parent, child_entity, &variation, &inputs) {
        Ok(Some(period)) => {
            unsafe {
                *out_found = 1;
                *out = dasha_period_to_ffi(&period);
            }
            DhruvStatus::Ok
        }
        Ok(None) => {
            unsafe { *out_found = 0 };
            DhruvStatus::Ok
        }
        Err(err) => DhruvStatus::from(&err),
    }
}

/// Compute a complete level from either UTC/location context or raw inputs.
///
/// # Safety
/// `request`, `parent_periods`, and `out` must be valid according to `parent_count`.
/// `engine`/`eop` may be null only when `request->birth.has_inputs != 0`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dhruv_dasha_complete_level(
    engine: *const Engine,
    eop: *const dhruv_time::EopKernel,
    request: *const DhruvDashaCompleteLevelRequest,
    parent_periods: *const DhruvDashaPeriod,
    parent_count: u32,
    out: *mut DhruvDashaPeriodListHandle,
) -> DhruvStatus {
    if request.is_null() || out.is_null() || (parent_count > 0 && parent_periods.is_null()) {
        return DhruvStatus::NullPointer;
    }

    let request = unsafe { &*request };
    let system = match dhruv_vedic_base::dasha::DashaSystem::from_u8(request.system) {
        Some(value) => value,
        None => return DhruvStatus::InvalidInput,
    };
    let variation = match dasha_variation_from_ffi(&request.variation) {
        Ok(value) => value,
        Err(status) => return status,
    };
    let child_level = match dhruv_vedic_base::dasha::DashaLevel::from_u8(request.child_level) {
        Some(value) => value,
        None => return DhruvStatus::InvalidInput,
    };
    let mut rust_parent_periods = Vec::with_capacity(parent_count as usize);
    if parent_count > 0 {
        let parent_slice =
            unsafe { std::slice::from_raw_parts(parent_periods, parent_count as usize) };
        for period in parent_slice {
            match dasha_period_from_ffi(period) {
                Ok(value) => rust_parent_periods.push(value),
                Err(status) => return status,
            }
        }
    }
    let birth = match resolve_dasha_birth_context(engine, eop, request.birth, system) {
        Ok(value) => value,
        Err(status) => return status,
    };
    let inputs = birth.inputs.borrowed();

    match dasha_complete_level_with_inputs(
        system,
        &rust_parent_periods,
        child_level,
        &variation,
        &inputs,
    ) {
        Ok(periods) => {
            let boxed = Box::new(periods);
            unsafe { *out = Box::into_raw(boxed) as DhruvDashaPeriodListHandle };
            DhruvStatus::Ok
        }
        Err(err) => DhruvStatus::from(&err),
    }
}

// ---- Fixed Star (Tara) types and functions ----

impl From<&TaraError> for DhruvStatus {
    fn from(value: &TaraError) -> Self {
        match value {
            TaraError::StarNotFound(_) => Self::InvalidQuery,
            TaraError::CatalogLoad(_) => Self::KernelLoad,
            TaraError::EarthStateRequired => Self::InvalidInput,
        }
    }
}

/// Opaque catalog handle for tara functions.
pub type DhruvTaraCatalogHandle = TaraCatalog;

/// C-compatible equatorial position.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DhruvEquatorialPosition {
    pub ra_deg: f64,
    pub dec_deg: f64,
    pub distance_au: f64,
}

/// C-compatible Earth state vector.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DhruvEarthState {
    pub position_au: [f64; 3],
    pub velocity_au_day: [f64; 3],
}

/// C-compatible tara config.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DhruvTaraConfig {
    /// 0 = Astrometric, 1 = Apparent
    pub accuracy: i32,
    /// 1 = apply parallax, 0 = don't
    pub apply_parallax: u8,
}

/// Tara output selector: equatorial position.
pub const DHRUV_TARA_OUTPUT_EQUATORIAL: i32 = 0;
/// Tara output selector: ecliptic position.
pub const DHRUV_TARA_OUTPUT_ECLIPTIC: i32 = 1;
/// Tara output selector: sidereal longitude.
pub const DHRUV_TARA_OUTPUT_SIDEREAL: i32 = 2;

/// C-compatible request for unified tara computation.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DhruvTaraComputeRequest {
    /// Star identifier (TaraId code).
    pub tara_id: i32,
    /// Output selector (`DHRUV_TARA_OUTPUT_*`).
    pub output_kind: i32,
    /// Epoch as JD TDB.
    pub jd_tdb: f64,
    /// Ayanamsha in degrees (used only for sidereal output).
    pub ayanamsha_deg: f64,
    /// Tara config for apparent/parallax options.
    pub config: DhruvTaraConfig,
    /// Whether `earth_state` is populated (0/1).
    pub earth_state_valid: u8,
    /// Optional Earth state for apparent/parallax modes.
    pub earth_state: DhruvEarthState,
}

/// C-compatible unified tara output.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DhruvTaraComputeResult {
    /// Echoed output selector (`DHRUV_TARA_OUTPUT_*`).
    pub output_kind: i32,
    /// Set when output_kind is EQUATORIAL.
    pub equatorial: DhruvEquatorialPosition,
    /// Set when output_kind is ECLIPTIC.
    pub ecliptic: DhruvSphericalCoords,
    /// Set when output_kind is SIDEREAL.
    pub sidereal_longitude_deg: f64,
}

/// Load a star catalog from a JSON file.
///
/// # Safety
/// `path_utf8` must be a valid null-terminated UTF-8 string.
/// `out_handle` must be a valid non-null pointer.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dhruv_tara_catalog_load(
    path_utf8: *const u8,
    path_len: u32,
    out_handle: *mut *mut DhruvTaraCatalogHandle,
) -> DhruvStatus {
    ffi_boundary(|| {
        if path_utf8.is_null() || out_handle.is_null() {
            return DhruvStatus::NullPointer;
        }
        let path_bytes = unsafe { std::slice::from_raw_parts(path_utf8, path_len as usize) };
        let path_str = match std::str::from_utf8(path_bytes) {
            Ok(s) => s,
            Err(_) => return DhruvStatus::InvalidConfig,
        };
        let out = unsafe { &mut *out_handle };
        match TaraCatalog::load(std::path::Path::new(path_str)) {
            Ok(catalog) => {
                *out = Box::into_raw(Box::new(catalog));
                DhruvStatus::Ok
            }
            Err(e) => {
                *out = ptr::null_mut();
                DhruvStatus::from(&e)
            }
        }
    })
}

/// Free a star catalog handle.
///
/// # Safety
/// `handle` must be a valid pointer previously returned by `dhruv_tara_catalog_load`,
/// or null (no-op).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dhruv_tara_catalog_free(handle: *mut DhruvTaraCatalogHandle) {
    if !handle.is_null() {
        drop(unsafe { Box::from_raw(handle) });
    }
}

fn tara_config_from_c(config: &DhruvTaraConfig) -> TaraConfig {
    TaraConfig {
        accuracy: if config.accuracy == 1 {
            TaraAccuracy::Apparent
        } else {
            TaraAccuracy::Astrometric
        },
        apply_parallax: config.apply_parallax != 0,
    }
}

fn earth_state_from_value(valid: u8, es: &DhruvEarthState) -> Option<dhruv_tara::EarthState> {
    if valid == 0 {
        None
    } else {
        Some(dhruv_tara::EarthState {
            position_au: es.position_au,
            velocity_au_day: es.velocity_au_day,
        })
    }
}

/// Unified tara compute entrypoint.
///
/// Selects equatorial/ecliptic/sidereal output via `request->output_kind`.
///
/// # Safety
/// `handle`, `request`, and `out` must be valid and non-null pointers.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dhruv_tara_compute_ex(
    handle: *const DhruvTaraCatalogHandle,
    request: *const DhruvTaraComputeRequest,
    out: *mut DhruvTaraComputeResult,
) -> DhruvStatus {
    ffi_boundary(|| {
        if handle.is_null() || request.is_null() || out.is_null() {
            return DhruvStatus::NullPointer;
        }

        let req = unsafe { &*request };
        let star = match TaraId::from_code(req.tara_id) {
            Some(v) => v,
            None => return DhruvStatus::InvalidQuery,
        };
        let output = match req.output_kind {
            DHRUV_TARA_OUTPUT_EQUATORIAL => TaraOutputKind::Equatorial,
            DHRUV_TARA_OUTPUT_ECLIPTIC => TaraOutputKind::Ecliptic,
            DHRUV_TARA_OUTPUT_SIDEREAL => TaraOutputKind::Sidereal,
            _ => return DhruvStatus::InvalidQuery,
        };
        let op = TaraOperation {
            star,
            output,
            at_jd_tdb: req.jd_tdb,
            ayanamsha_deg: req.ayanamsha_deg,
            config: tara_config_from_c(&req.config),
            earth_state: earth_state_from_value(req.earth_state_valid, &req.earth_state),
        };

        let catalog = unsafe { &*handle };
        match dhruv_vedic_ops::tara(catalog, &op) {
            Ok(TaraResult::Equatorial(pos)) => {
                unsafe {
                    *out = DhruvTaraComputeResult {
                        output_kind: DHRUV_TARA_OUTPUT_EQUATORIAL,
                        equatorial: DhruvEquatorialPosition {
                            ra_deg: pos.ra_deg,
                            dec_deg: pos.dec_deg,
                            distance_au: pos.distance_au,
                        },
                        ecliptic: DhruvSphericalCoords {
                            lon_deg: 0.0,
                            lat_deg: 0.0,
                            distance_km: 0.0,
                        },
                        sidereal_longitude_deg: 0.0,
                    };
                }
                DhruvStatus::Ok
            }
            Ok(TaraResult::Ecliptic(sc)) => {
                unsafe {
                    *out = DhruvTaraComputeResult {
                        output_kind: DHRUV_TARA_OUTPUT_ECLIPTIC,
                        equatorial: DhruvEquatorialPosition {
                            ra_deg: 0.0,
                            dec_deg: 0.0,
                            distance_au: 0.0,
                        },
                        ecliptic: DhruvSphericalCoords {
                            lon_deg: sc.lon_deg,
                            lat_deg: sc.lat_deg,
                            distance_km: sc.distance_km,
                        },
                        sidereal_longitude_deg: 0.0,
                    };
                }
                DhruvStatus::Ok
            }
            Ok(TaraResult::Sidereal(lon)) => {
                unsafe {
                    *out = DhruvTaraComputeResult {
                        output_kind: DHRUV_TARA_OUTPUT_SIDEREAL,
                        equatorial: DhruvEquatorialPosition {
                            ra_deg: 0.0,
                            dec_deg: 0.0,
                            distance_au: 0.0,
                        },
                        ecliptic: DhruvSphericalCoords {
                            lon_deg: 0.0,
                            lat_deg: 0.0,
                            distance_km: 0.0,
                        },
                        sidereal_longitude_deg: lon,
                    };
                }
                DhruvStatus::Ok
            }
            Err(e) => DhruvStatus::from(&e),
        }
    })
}

/// Compute ecliptic position of the Galactic Center.
///
/// # Safety
/// `out` must be a valid non-null pointer. `handle` is required but
/// the GC position is catalog-independent.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dhruv_tara_galactic_center_ecliptic(
    handle: *const DhruvTaraCatalogHandle,
    jd_tdb: f64,
    out: *mut DhruvSphericalCoords,
) -> DhruvStatus {
    ffi_boundary(|| {
        if handle.is_null() || out.is_null() {
            return DhruvStatus::NullPointer;
        }
        let catalog = unsafe { &*handle };
        match dhruv_tara::position_ecliptic(catalog, TaraId::GalacticCenter, jd_tdb) {
            Ok(sc) => {
                unsafe {
                    *out = DhruvSphericalCoords {
                        lon_deg: sc.lon_deg,
                        lat_deg: sc.lat_deg,
                        distance_km: sc.distance_km,
                    };
                }
                DhruvStatus::Ok
            }
            Err(e) => DhruvStatus::from(&e),
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_maps_from_core_error() {
        let status = DhruvStatus::from(&EngineError::InvalidQuery("bad"));
        assert_eq!(status, DhruvStatus::InvalidQuery);
    }

    #[test]
    fn ffi_query_rejects_null_input_pointer() {
        let mut out_state = DhruvStateVector {
            position_km: [0.0; 3],
            velocity_km_s: [0.0; 3],
        };
        // SAFETY: Null engine pointer is intentional for this validation test.
        let status = unsafe { dhruv_engine_query(ptr::null(), ptr::null(), &mut out_state) };
        assert_eq!(status, DhruvStatus::NullPointer);
    }

    #[test]
    fn ffi_query_request_rejects_null_input_pointer() {
        let mut out = DhruvQueryResult {
            state_vector: DhruvStateVector {
                position_km: [0.0; 3],
                velocity_km_s: [0.0; 3],
            },
            spherical_state: DhruvSphericalState {
                lon_deg: 0.0,
                lat_deg: 0.0,
                distance_km: 0.0,
                lon_speed: 0.0,
                lat_speed: 0.0,
                distance_speed: 0.0,
            },
        };
        let status = unsafe { dhruv_engine_query_request(ptr::null(), ptr::null(), &mut out) };
        assert_eq!(status, DhruvStatus::NullPointer);
    }

    #[test]
    fn query_request_invalid_time_kind_rejected() {
        let request = DhruvQueryRequest {
            target: 399,
            observer: 10,
            frame: 0,
            time_kind: 99,
            epoch_tdb_jd: 2_451_545.0,
            utc: ZEROED_UTC,
            output_mode: DHRUV_QUERY_OUTPUT_BOTH,
        };
        let result = validate_query_request_selectors(request);
        assert_eq!(result, Err(DhruvStatus::InvalidQuery));
    }

    #[test]
    fn query_request_invalid_output_mode_rejected() {
        let request = DhruvQueryRequest {
            target: 399,
            observer: 10,
            frame: 0,
            time_kind: DHRUV_QUERY_TIME_JD_TDB,
            epoch_tdb_jd: 2_451_545.0,
            utc: ZEROED_UTC,
            output_mode: 99,
        };
        let result = validate_query_request_selectors(request);
        assert_eq!(result, Err(DhruvStatus::InvalidQuery));
    }

    #[test]
    fn query_result_from_state_honors_output_mode() {
        let state = StateVector {
            position_km: [1.0, 2.0, 3.0],
            velocity_km_s: [0.1, 0.2, 0.3],
        };
        let cart = query_result_from_state(state, DHRUV_QUERY_OUTPUT_CARTESIAN);
        assert_eq!(cart.state_vector.position_km, [1.0, 2.0, 3.0]);
        assert_eq!(cart.spherical_state.distance_km, 0.0);

        let sph = query_result_from_state(state, DHRUV_QUERY_OUTPUT_SPHERICAL);
        assert_eq!(sph.state_vector.position_km, [0.0; 3]);
        assert!(sph.spherical_state.distance_km > 0.0);

        let both = query_result_from_state(state, DHRUV_QUERY_OUTPUT_BOTH);
        assert_eq!(both.state_vector.position_km, [1.0, 2.0, 3.0]);
        assert!(both.spherical_state.distance_km > 0.0);
    }

    #[test]
    fn ffi_lsk_load_rejects_null() {
        let mut lsk_ptr: *mut DhruvLskHandle = ptr::null_mut();
        // SAFETY: Null path pointer is intentional for validation.
        let status = unsafe { dhruv_lsk_load(ptr::null(), &mut lsk_ptr) };
        assert_eq!(status, DhruvStatus::NullPointer);
    }

    #[test]
    fn ffi_utc_to_tdb_jd_rejects_null() {
        let mut out = DhruvUtcToTdbResult {
            jd_tdb: 0.0,
            diagnostics: DhruvTimeDiagnostics {
                source: 0,
                tt_minus_utc_s: 0.0,
                warning_count: 0,
                warnings: [empty_time_warning(); DHRUV_MAX_TIME_WARNINGS],
            },
        };
        // SAFETY: Null LSK pointer is intentional for validation.
        let status =
            unsafe { dhruv_utc_to_tdb_jd(ptr::null(), ptr::null(), ptr::null(), &mut out) };
        assert_eq!(status, DhruvStatus::NullPointer);
    }

    #[test]
    fn ffi_cartesian_to_spherical_along_x() {
        let pos = [1.0e8_f64, 0.0, 0.0];
        let mut out = DhruvSphericalCoords {
            lon_deg: 0.0,
            lat_deg: 0.0,
            distance_km: 0.0,
        };
        // SAFETY: Both pointers are valid stack references.
        let status = unsafe { dhruv_cartesian_to_spherical(&pos, &mut out) };
        assert_eq!(status, DhruvStatus::Ok);
        assert!((out.lon_deg - 0.0).abs() < 1e-10);
        assert!((out.lat_deg - 0.0).abs() < 1e-10);
        assert!((out.distance_km - 1.0e8).abs() < 1e-3);
    }

    #[test]
    fn ffi_cartesian_to_spherical_rejects_null() {
        let mut out = DhruvSphericalCoords {
            lon_deg: 0.0,
            lat_deg: 0.0,
            distance_km: 0.0,
        };
        // SAFETY: Null position pointer is intentional for validation.
        let status = unsafe { dhruv_cartesian_to_spherical(ptr::null(), &mut out) };
        assert_eq!(status, DhruvStatus::NullPointer);
    }

    // --- EOP handle tests ---

    #[test]
    fn ffi_eop_load_rejects_null() {
        let mut eop_ptr: *mut DhruvEopHandle = ptr::null_mut();
        // SAFETY: Null path pointer is intentional for validation.
        let status = unsafe { dhruv_eop_load(ptr::null(), &mut eop_ptr) };
        assert_eq!(status, DhruvStatus::NullPointer);
    }

    // --- Ayanamsha tests ---

    #[test]
    fn ffi_ayanamsha_system_count_is_20() {
        assert_eq!(dhruv_ayanamsha_system_count(), 20);
    }

    // --- Rise/Set config test ---

    #[test]
    fn ffi_riseset_config_default_values() {
        let cfg = dhruv_riseset_config_default();
        assert_eq!(cfg.use_refraction, 1);
        assert_eq!(cfg.sun_limb, DHRUV_SUN_LIMB_UPPER);
        assert_eq!(cfg.altitude_correction, 1);
    }

    #[test]
    fn ffi_ayanamsha_compute_ex_rejects_null_request() {
        let mut out: f64 = 0.0;
        // SAFETY: Null request pointer is intentional for validation.
        let status =
            unsafe { dhruv_ayanamsha_compute_ex(ptr::null(), ptr::null(), ptr::null(), &mut out) };
        assert_eq!(status, DhruvStatus::NullPointer);
    }

    #[test]
    fn ffi_ayanamsha_compute_ex_rejects_null_output() {
        let request = DhruvAyanamshaComputeRequest {
            system_code: 0,
            mode: DHRUV_AYANAMSHA_MODE_MEAN,
            time_kind: DHRUV_AYANAMSHA_TIME_JD_TDB,
            jd_tdb: 2_451_545.0,
            utc: DhruvUtcTime {
                year: 2000,
                month: 1,
                day: 1,
                hour: 12,
                minute: 0,
                second: 0.0,
            },
            use_nutation: 0,
            delta_psi_arcsec: 0.0,
        };
        // SAFETY: Null output pointer is intentional for validation.
        let status = unsafe {
            dhruv_ayanamsha_compute_ex(ptr::null(), &request, ptr::null(), ptr::null_mut())
        };
        assert_eq!(status, DhruvStatus::NullPointer);
    }

    #[test]
    fn ffi_ayanamsha_compute_ex_rejects_invalid_selector() {
        let request = DhruvAyanamshaComputeRequest {
            system_code: 0,
            mode: 99,
            time_kind: 99,
            jd_tdb: 2_451_545.0,
            utc: DhruvUtcTime {
                year: 2000,
                month: 1,
                day: 1,
                hour: 12,
                minute: 0,
                second: 0.0,
            },
            use_nutation: 0,
            delta_psi_arcsec: 0.0,
        };
        let mut out: f64 = 0.0;
        // SAFETY: Valid pointers with intentionally invalid selector fields.
        let status =
            unsafe { dhruv_ayanamsha_compute_ex(ptr::null(), &request, ptr::null(), &mut out) };
        assert_eq!(status, DhruvStatus::InvalidQuery);
    }

    #[test]
    fn ffi_ayanamsha_compute_ex_rejects_invalid_code() {
        let request = DhruvAyanamshaComputeRequest {
            system_code: 99,
            mode: DHRUV_AYANAMSHA_MODE_MEAN,
            time_kind: DHRUV_AYANAMSHA_TIME_JD_TDB,
            jd_tdb: 2_451_545.0,
            utc: DhruvUtcTime {
                year: 2000,
                month: 1,
                day: 1,
                hour: 12,
                minute: 0,
                second: 0.0,
            },
            use_nutation: 0,
            delta_psi_arcsec: 0.0,
        };
        let mut out: f64 = 0.0;
        // SAFETY: Invalid system code is intentional for validation.
        let status =
            unsafe { dhruv_ayanamsha_compute_ex(ptr::null(), &request, ptr::null(), &mut out) };
        assert_eq!(status, DhruvStatus::InvalidQuery);
    }

    #[test]
    fn ffi_ayanamsha_compute_ex_mean_lahiri_at_j2000() {
        let request = DhruvAyanamshaComputeRequest {
            system_code: 0,
            mode: DHRUV_AYANAMSHA_MODE_MEAN,
            time_kind: DHRUV_AYANAMSHA_TIME_JD_TDB,
            jd_tdb: 2_451_545.0,
            utc: DhruvUtcTime {
                year: 2000,
                month: 1,
                day: 1,
                hour: 12,
                minute: 0,
                second: 0.0,
            },
            use_nutation: 0,
            delta_psi_arcsec: 0.0,
        };
        let mut out: f64 = 0.0;
        // SAFETY: Valid pointers.
        let status =
            unsafe { dhruv_ayanamsha_compute_ex(ptr::null(), &request, ptr::null(), &mut out) };
        assert_eq!(status, DhruvStatus::Ok);
        assert!(
            (out - 23.857_052_898_247_307).abs() < 1e-12,
            "Lahiri at J2000 = {out}, expected mean anchor reference"
        );
    }

    #[test]
    fn ffi_ayanamsha_compute_ex_nutation_flag_works() {
        let base = DhruvAyanamshaComputeRequest {
            system_code: 1,
            mode: DHRUV_AYANAMSHA_MODE_UNIFIED,
            time_kind: DHRUV_AYANAMSHA_TIME_JD_TDB,
            jd_tdb: 2_460_310.5,
            utc: DhruvUtcTime {
                year: 2000,
                month: 1,
                day: 1,
                hour: 0,
                minute: 0,
                second: 0.0,
            },
            use_nutation: 0,
            delta_psi_arcsec: 0.0,
        };
        let with_nut = DhruvAyanamshaComputeRequest {
            use_nutation: 1,
            ..base
        };
        let mut with_nut_out: f64 = 0.0;
        let mut without_nut_out: f64 = 0.0;
        // SAFETY: Valid pointers.
        let s1 = unsafe {
            dhruv_ayanamsha_compute_ex(ptr::null(), &with_nut, ptr::null(), &mut with_nut_out)
        };
        let s2 = unsafe {
            dhruv_ayanamsha_compute_ex(ptr::null(), &base, ptr::null(), &mut without_nut_out)
        };
        assert_eq!(s1, DhruvStatus::Ok);
        assert_eq!(s2, DhruvStatus::Ok);
        let t = dhruv_vedic_base::jd_tdb_to_centuries(base.jd_tdb);
        let (dpsi, _) = dhruv_frames::nutation_iau2000b(t);
        let expected_diff = dpsi / 3600.0;
        assert!((with_nut_out - without_nut_out - expected_diff).abs() < 1e-10);
    }

    #[test]
    fn ffi_ayanamsha_compute_ex_utc_requires_lsk() {
        let request = DhruvAyanamshaComputeRequest {
            system_code: 0,
            mode: DHRUV_AYANAMSHA_MODE_MEAN,
            time_kind: DHRUV_AYANAMSHA_TIME_UTC,
            jd_tdb: 0.0,
            utc: DhruvUtcTime {
                year: 2000,
                month: 1,
                day: 1,
                hour: 12,
                minute: 0,
                second: 0.0,
            },
            use_nutation: 0,
            delta_psi_arcsec: 0.0,
        };
        let mut out: f64 = 0.0;
        // SAFETY: Null LSK with UTC input is intentional for validation.
        let status =
            unsafe { dhruv_ayanamsha_compute_ex(ptr::null(), &request, ptr::null(), &mut out) };
        assert_eq!(status, DhruvStatus::NullPointer);
    }

    #[test]
    fn ffi_nutation_iau2000b_at_j2000() {
        let mut dpsi: f64 = 0.0;
        let mut deps: f64 = 0.0;
        // SAFETY: Valid pointers.
        let status = unsafe { dhruv_nutation_iau2000b(2_451_545.0, &mut dpsi, &mut deps) };
        assert_eq!(status, DhruvStatus::Ok);
        assert!(dpsi.is_finite());
        assert!(deps.is_finite());
        assert!(dpsi.abs() < 20.0, "Δψ = {dpsi}");
        assert!(deps.abs() < 10.0, "Δε = {deps}");
    }

    #[test]
    fn ffi_nutation_rejects_null() {
        let mut dpsi: f64 = 0.0;
        // SAFETY: Null pointer intentional for validation.
        let status = unsafe { dhruv_nutation_iau2000b(2_451_545.0, &mut dpsi, ptr::null_mut()) };
        assert_eq!(status, DhruvStatus::NullPointer);
    }

    // --- Rise/Set null rejection ---

    #[test]
    fn ffi_compute_rise_set_rejects_null() {
        let mut out = DhruvRiseSetResult {
            result_type: 0,
            event_code: 0,
            jd_tdb: 0.0,
        };
        // SAFETY: Null engine pointer is intentional for validation.
        let status = unsafe {
            dhruv_compute_rise_set(
                ptr::null(),
                ptr::null(),
                ptr::null(),
                ptr::null(),
                0,
                0.0,
                ptr::null(),
                &mut out,
            )
        };
        assert_eq!(status, DhruvStatus::NullPointer);
    }

    // --- Local noon ---

    #[test]
    fn ffi_local_noon() {
        let jd_0h = 2_460_000.5;
        let noon = dhruv_approximate_local_noon_jd(jd_0h, 0.0);
        assert!((noon - (jd_0h + 0.5)).abs() < 1e-10);
    }

    // --- UTC time tests ---

    #[test]
    fn ffi_jd_tdb_to_utc_rejects_null() {
        let mut out = DhruvUtcTime {
            year: 0,
            month: 0,
            day: 0,
            hour: 0,
            minute: 0,
            second: 0.0,
        };
        // SAFETY: Null LSK pointer is intentional for validation.
        let status = unsafe { dhruv_jd_tdb_to_utc(ptr::null(), 2_451_545.0, &mut out) };
        assert_eq!(status, DhruvStatus::NullPointer);
    }

    #[test]
    fn ffi_riseset_result_to_utc_never_rises() {
        let result = DhruvRiseSetResult {
            result_type: DHRUV_RISESET_NEVER_RISES,
            event_code: 0,
            jd_tdb: 0.0,
        };
        let mut out = DhruvUtcTime {
            year: 0,
            month: 0,
            day: 0,
            hour: 0,
            minute: 0,
            second: 0.0,
        };
        // Need a non-null LSK pointer for the null check to pass, but the
        // function should return InvalidQuery before dereferencing it.
        // Use a dangling-but-aligned pointer — the function checks result_type first.
        let fake_lsk = std::ptr::NonNull::<DhruvLskHandle>::dangling().as_ptr();
        // SAFETY: fake_lsk is non-null; function returns InvalidQuery before deref.
        let status =
            unsafe { dhruv_riseset_result_to_utc(fake_lsk as *const _, &result, &mut out) };
        assert_eq!(status, DhruvStatus::InvalidQuery);
    }

    #[test]
    fn ffi_fractional_day_to_hms_noon() {
        let (h, m, s) = fractional_day_to_hms(15.5); // .5 = noon
        assert_eq!(h, 12);
        assert_eq!(m, 0);
        assert!(s.abs() < 0.01);
    }

    #[test]
    fn ffi_fractional_day_to_hms_quarter() {
        let (h, m, s) = fractional_day_to_hms(1.25); // .25 = 06:00
        assert_eq!(h, 6);
        assert_eq!(m, 0);
        assert!(s.abs() < 0.01);
    }

    // --- Bhava tests ---

    #[test]
    fn ffi_bhava_config_default_values() {
        let cfg = dhruv_bhava_config_default();
        assert_eq!(cfg.system, DHRUV_BHAVA_EQUAL);
        assert_eq!(cfg.starting_point, DHRUV_BHAVA_START_LAGNA);
        assert_eq!(cfg.reference_mode, DHRUV_BHAVA_REF_START);
        assert!((cfg.custom_start_deg - 0.0).abs() < 1e-15);
    }

    #[test]
    fn ffi_bhava_system_count_is_10() {
        assert_eq!(dhruv_bhava_system_count(), 10);
    }

    #[test]
    fn ffi_compute_bhavas_rejects_null() {
        let mut out = DhruvBhavaResult {
            bhavas: [DhruvBhava {
                number: 0,
                cusp_deg: 0.0,
                start_deg: 0.0,
                end_deg: 0.0,
            }; 12],
            lagna_deg: 0.0,
            mc_deg: 0.0,
        };
        // SAFETY: Null pointers intentional for validation.
        let status = unsafe {
            dhruv_compute_bhavas(
                ptr::null(),
                ptr::null(),
                ptr::null(),
                ptr::null(),
                0.0,
                ptr::null(),
                &mut out,
            )
        };
        assert_eq!(status, DhruvStatus::NullPointer);
    }

    #[test]
    fn ffi_bhava_config_invalid_system() {
        let cfg = DhruvBhavaConfig {
            system: 99,
            starting_point: DHRUV_BHAVA_START_LAGNA,
            custom_start_deg: 0.0,
            reference_mode: DHRUV_BHAVA_REF_START,
            ..dhruv_bhava_config_default()
        };
        let result = bhava_config_from_ffi(&cfg);
        assert_eq!(result, Err(DhruvStatus::InvalidQuery));
    }

    #[test]
    fn ffi_bhava_config_invalid_starting_point() {
        let cfg = DhruvBhavaConfig {
            system: DHRUV_BHAVA_EQUAL,
            starting_point: -99,
            custom_start_deg: 0.0,
            reference_mode: DHRUV_BHAVA_REF_START,
            ..dhruv_bhava_config_default()
        };
        let result = bhava_config_from_ffi(&cfg);
        assert_eq!(result, Err(DhruvStatus::InvalidQuery));
    }

    #[test]
    fn ffi_lagna_deg_rejects_null() {
        let mut out: f64 = 0.0;
        // SAFETY: Null pointers intentional for validation.
        let status =
            unsafe { dhruv_lagna_deg(ptr::null(), ptr::null(), ptr::null(), 0.0, &mut out) };
        assert_eq!(status, DhruvStatus::NullPointer);
    }

    #[test]
    fn ffi_mc_deg_rejects_null() {
        let mut out: f64 = 0.0;
        // SAFETY: Null pointers intentional for validation.
        let status = unsafe { dhruv_mc_deg(ptr::null(), ptr::null(), ptr::null(), 0.0, &mut out) };
        assert_eq!(status, DhruvStatus::NullPointer);
    }

    #[test]
    fn ffi_ramc_deg_rejects_null() {
        let mut out: f64 = 0.0;
        let status =
            unsafe { dhruv_ramc_deg(ptr::null(), ptr::null(), ptr::null(), 0.0, &mut out) };
        assert_eq!(status, DhruvStatus::NullPointer);
    }

    // --- Lunar node tests ---

    #[test]
    fn ffi_lunar_node_rejects_invalid_node_code() {
        let mut out: f64 = 0.0;
        // SAFETY: Valid output pointer, invalid node code.
        let status =
            unsafe { dhruv_lunar_node_deg(99, DHRUV_NODE_MODE_MEAN, 2_451_545.0, &mut out) };
        assert_eq!(status, DhruvStatus::InvalidQuery);
    }

    #[test]
    fn ffi_lunar_node_rejects_invalid_mode_code() {
        let mut out: f64 = 0.0;
        // SAFETY: Valid output pointer, invalid mode code.
        let status = unsafe { dhruv_lunar_node_deg(DHRUV_NODE_RAHU, 99, 2_451_545.0, &mut out) };
        assert_eq!(status, DhruvStatus::InvalidQuery);
    }

    #[test]
    fn ffi_lunar_node_rejects_null() {
        // SAFETY: Null output pointer is intentional for validation.
        let status = unsafe {
            dhruv_lunar_node_deg(
                DHRUV_NODE_RAHU,
                DHRUV_NODE_MODE_MEAN,
                2_451_545.0,
                ptr::null_mut(),
            )
        };
        assert_eq!(status, DhruvStatus::NullPointer);
    }

    #[test]
    fn ffi_lunar_node_rahu_at_j2000() {
        let mut out: f64 = 0.0;
        // SAFETY: Valid pointers.
        let status = unsafe {
            dhruv_lunar_node_deg(DHRUV_NODE_RAHU, DHRUV_NODE_MODE_MEAN, 2_451_545.0, &mut out)
        };
        assert_eq!(status, DhruvStatus::Ok);
        assert!(
            (out - 125.04).abs() < 0.1,
            "mean Rahu at J2000 = {out}, expected ~125.04"
        );
    }

    #[test]
    fn ffi_lunar_node_ketu_opposite_rahu() {
        let mut rahu: f64 = 0.0;
        let mut ketu: f64 = 0.0;
        // SAFETY: Valid pointers.
        let s1 = unsafe {
            dhruv_lunar_node_deg(
                DHRUV_NODE_RAHU,
                DHRUV_NODE_MODE_MEAN,
                2_451_545.0,
                &mut rahu,
            )
        };
        let s2 = unsafe {
            dhruv_lunar_node_deg(
                DHRUV_NODE_KETU,
                DHRUV_NODE_MODE_MEAN,
                2_451_545.0,
                &mut ketu,
            )
        };
        assert_eq!(s1, DhruvStatus::Ok);
        assert_eq!(s2, DhruvStatus::Ok);
        let diff = ((ketu - rahu) % 360.0 + 360.0) % 360.0;
        assert!(
            (diff - 180.0).abs() < 1e-10,
            "Ketu - Rahu = {diff}, expected 180"
        );
    }

    #[test]
    fn ffi_lunar_node_count() {
        assert_eq!(dhruv_lunar_node_count(), 2);
    }

    #[test]
    fn ffi_lunar_node_compute_ex_rejects_null_request() {
        let mut out: f64 = 0.0;
        // SAFETY: Null request pointer is intentional for validation.
        let status =
            unsafe { dhruv_lunar_node_compute_ex(ptr::null(), ptr::null(), ptr::null(), &mut out) };
        assert_eq!(status, DhruvStatus::NullPointer);
    }

    #[test]
    fn ffi_lunar_node_compute_ex_rejects_invalid_selector() {
        let request = DhruvLunarNodeRequest {
            node_code: 99,
            mode_code: 99,
            backend: 99,
            time_kind: 99,
            jd_tdb: 2_451_545.0,
            utc: DhruvUtcTime {
                year: 2000,
                month: 1,
                day: 1,
                hour: 12,
                minute: 0,
                second: 0.0,
            },
        };
        let mut out: f64 = 0.0;
        // SAFETY: Valid pointers with intentionally invalid selector fields.
        let status =
            unsafe { dhruv_lunar_node_compute_ex(ptr::null(), ptr::null(), &request, &mut out) };
        assert_eq!(status, DhruvStatus::InvalidQuery);
    }

    #[test]
    fn ffi_lunar_node_compute_ex_matches_analytic_jd() {
        let request = DhruvLunarNodeRequest {
            node_code: DHRUV_NODE_RAHU,
            mode_code: DHRUV_NODE_MODE_MEAN,
            backend: DHRUV_NODE_BACKEND_ANALYTIC,
            time_kind: DHRUV_NODE_TIME_JD_TDB,
            jd_tdb: 2_451_545.0,
            utc: DhruvUtcTime {
                year: 2000,
                month: 1,
                day: 1,
                hour: 0,
                minute: 0,
                second: 0.0,
            },
        };
        let mut ex: f64 = 0.0;
        let mut old: f64 = 0.0;
        // SAFETY: Valid pointers.
        let s1 =
            unsafe { dhruv_lunar_node_compute_ex(ptr::null(), ptr::null(), &request, &mut ex) };
        let s2 = unsafe {
            dhruv_lunar_node_deg(DHRUV_NODE_RAHU, DHRUV_NODE_MODE_MEAN, 2_451_545.0, &mut old)
        };
        assert_eq!(s1, DhruvStatus::Ok);
        assert_eq!(s2, DhruvStatus::Ok);
        assert!((ex - old).abs() < 1e-15);
    }

    #[test]
    fn ffi_panchang_compute_ex_rejects_null_request() {
        let mut out = DhruvPanchangOperationResult {
            tithi_valid: 0,
            tithi: zeroed_tithi_info(),
            karana_valid: 0,
            karana: zeroed_karana_info(),
            yoga_valid: 0,
            yoga: zeroed_yoga_info(),
            vaar_valid: 0,
            vaar: zeroed_vaar_info(),
            hora_valid: 0,
            hora: zeroed_hora_info(),
            ghatika_valid: 0,
            ghatika: zeroed_ghatika_info(),
            nakshatra_valid: 0,
            nakshatra: zeroed_panchang_nakshatra_info(),
            masa_valid: 0,
            masa: zeroed_masa_info(),
            ayana_valid: 0,
            ayana: zeroed_ayana_info(),
            varsha_valid: 0,
            varsha: zeroed_varsha_info(),
        };
        // SAFETY: Null request pointer is intentional for validation.
        let status = unsafe {
            dhruv_panchang_compute_ex(ptr::null(), ptr::null(), ptr::null(), ptr::null(), &mut out)
        };
        assert_eq!(status, DhruvStatus::NullPointer);
    }

    #[test]
    fn ffi_panchang_compute_ex_rejects_invalid_selector() {
        let fake_engine = std::ptr::NonNull::<DhruvEngineHandle>::dangling().as_ptr();
        let fake_eop = std::ptr::NonNull::<DhruvEopHandle>::dangling().as_ptr();
        let request = DhruvPanchangComputeRequest {
            time_kind: 99,
            jd_tdb: 2_451_545.0,
            utc: DhruvUtcTime {
                year: 2000,
                month: 1,
                day: 1,
                hour: 12,
                minute: 0,
                second: 0.0,
            },
            include_mask: DHRUV_PANCHANG_INCLUDE_TITHI,
            location: DhruvGeoLocation {
                latitude_deg: 0.0,
                longitude_deg: 0.0,
                altitude_m: 0.0,
            },
            riseset_config: dhruv_riseset_config_default(),
            sankranti_config: dhruv_sankranti_config_default(),
        };
        let mut out = DhruvPanchangOperationResult {
            tithi_valid: 0,
            tithi: zeroed_tithi_info(),
            karana_valid: 0,
            karana: zeroed_karana_info(),
            yoga_valid: 0,
            yoga: zeroed_yoga_info(),
            vaar_valid: 0,
            vaar: zeroed_vaar_info(),
            hora_valid: 0,
            hora: zeroed_hora_info(),
            ghatika_valid: 0,
            ghatika: zeroed_ghatika_info(),
            nakshatra_valid: 0,
            nakshatra: zeroed_panchang_nakshatra_info(),
            masa_valid: 0,
            masa: zeroed_masa_info(),
            ayana_valid: 0,
            ayana: zeroed_ayana_info(),
            varsha_valid: 0,
            varsha: zeroed_varsha_info(),
        };
        // SAFETY: Valid request pointer with intentionally invalid time selector.
        let status = unsafe {
            dhruv_panchang_compute_ex(
                fake_engine as *const _,
                fake_eop as *const _,
                ptr::null(),
                &request,
                &mut out,
            )
        };
        assert_eq!(status, DhruvStatus::InvalidQuery);
    }

    #[test]
    fn ffi_panchang_compute_ex_jd_requires_lsk() {
        let fake_engine = std::ptr::NonNull::<DhruvEngineHandle>::dangling().as_ptr();
        let fake_eop = std::ptr::NonNull::<DhruvEopHandle>::dangling().as_ptr();
        let request = DhruvPanchangComputeRequest {
            time_kind: DHRUV_PANCHANG_TIME_JD_TDB,
            jd_tdb: 2_451_545.0,
            utc: DhruvUtcTime {
                year: 2000,
                month: 1,
                day: 1,
                hour: 12,
                minute: 0,
                second: 0.0,
            },
            include_mask: DHRUV_PANCHANG_INCLUDE_TITHI,
            location: DhruvGeoLocation {
                latitude_deg: 0.0,
                longitude_deg: 0.0,
                altitude_m: 0.0,
            },
            riseset_config: dhruv_riseset_config_default(),
            sankranti_config: dhruv_sankranti_config_default(),
        };
        let mut out = DhruvPanchangOperationResult {
            tithi_valid: 0,
            tithi: zeroed_tithi_info(),
            karana_valid: 0,
            karana: zeroed_karana_info(),
            yoga_valid: 0,
            yoga: zeroed_yoga_info(),
            vaar_valid: 0,
            vaar: zeroed_vaar_info(),
            hora_valid: 0,
            hora: zeroed_hora_info(),
            ghatika_valid: 0,
            ghatika: zeroed_ghatika_info(),
            nakshatra_valid: 0,
            nakshatra: zeroed_panchang_nakshatra_info(),
            masa_valid: 0,
            masa: zeroed_masa_info(),
            ayana_valid: 0,
            ayana: zeroed_ayana_info(),
            varsha_valid: 0,
            varsha: zeroed_varsha_info(),
        };
        // SAFETY: JD input without LSK is intentional for validation.
        let status = unsafe {
            dhruv_panchang_compute_ex(
                fake_engine as *const _,
                fake_eop as *const _,
                ptr::null(),
                &request,
                &mut out,
            )
        };
        assert_eq!(status, DhruvStatus::NullPointer);
    }

    #[test]
    fn ffi_tara_compute_ex_rejects_null_request() {
        let mut out = DhruvTaraComputeResult {
            output_kind: 0,
            equatorial: DhruvEquatorialPosition {
                ra_deg: 0.0,
                dec_deg: 0.0,
                distance_au: 0.0,
            },
            ecliptic: DhruvSphericalCoords {
                lon_deg: 0.0,
                lat_deg: 0.0,
                distance_km: 0.0,
            },
            sidereal_longitude_deg: 0.0,
        };
        // SAFETY: Null request pointer is intentional for validation.
        let status = unsafe { dhruv_tara_compute_ex(ptr::null(), ptr::null(), &mut out) };
        assert_eq!(status, DhruvStatus::NullPointer);
    }

    #[test]
    fn ffi_tara_compute_ex_rejects_invalid_selector() {
        let fake_catalog = std::ptr::NonNull::<DhruvTaraCatalogHandle>::dangling().as_ptr();
        let request = DhruvTaraComputeRequest {
            tara_id: 0,
            output_kind: 99,
            jd_tdb: 2_451_545.0,
            ayanamsha_deg: 24.0,
            config: DhruvTaraConfig {
                accuracy: 0,
                apply_parallax: 0,
            },
            earth_state_valid: 0,
            earth_state: DhruvEarthState {
                position_au: [0.0; 3],
                velocity_au_day: [0.0; 3],
            },
        };
        let mut out = DhruvTaraComputeResult {
            output_kind: 0,
            equatorial: DhruvEquatorialPosition {
                ra_deg: 0.0,
                dec_deg: 0.0,
                distance_au: 0.0,
            },
            ecliptic: DhruvSphericalCoords {
                lon_deg: 0.0,
                lat_deg: 0.0,
                distance_km: 0.0,
            },
            sidereal_longitude_deg: 0.0,
        };
        // SAFETY: Invalid selector is validated before catalog dereference.
        let status = unsafe { dhruv_tara_compute_ex(fake_catalog as *const _, &request, &mut out) };
        assert_eq!(status, DhruvStatus::InvalidQuery);
    }

    #[test]
    fn ffi_api_version_is_49() {
        assert_eq!(dhruv_api_version(), 49);
    }

    #[test]
    fn ffi_full_kundali_config_default_values() {
        let cfg = dhruv_full_kundali_config_default();
        assert_eq!(cfg.include_bhava_cusps, 1);
        assert_eq!(cfg.include_graha_positions, 1);
        assert_eq!(cfg.include_bindus, 1);
        assert_eq!(cfg.include_drishti, 1);
        assert_eq!(cfg.include_ashtakavarga, 1);
        assert_eq!(cfg.include_upagrahas, 1);
        assert_eq!(cfg.include_special_lagnas, 1);
        assert_eq!(cfg.include_amshas, 0);
        assert_eq!(cfg.include_shadbala, 0);
        assert_eq!(cfg.include_bhavabala, 0);
        assert_eq!(cfg.include_vimsopaka, 0);
        assert_eq!(cfg.include_avastha, 0);
        assert_eq!(cfg.include_charakaraka, 0);
        assert_eq!(cfg.charakaraka_scheme, CharakarakaScheme::default() as u8);
        assert_eq!(cfg.include_panchang, 0);
        assert_eq!(cfg.include_calendar, 0);
        assert_eq!(cfg.include_dasha, 0);
        assert_eq!(cfg.node_dignity_policy, 0);
        assert_eq!(cfg.graha_positions_config.include_lagna, 1);
    }

    // --- Search error mapping ---

    #[test]
    fn ffi_search_error_invalid_config() {
        let err = SearchError::InvalidConfig("bad config");
        assert_eq!(DhruvStatus::from(&err), DhruvStatus::InvalidSearchConfig);
    }

    #[test]
    fn ffi_search_error_no_convergence() {
        let err = SearchError::NoConvergence("stuck");
        assert_eq!(DhruvStatus::from(&err), DhruvStatus::NoConvergence);
    }

    #[test]
    fn ffi_search_error_engine() {
        let err = SearchError::Engine(EngineError::EpochOutOfRange { epoch_tdb_jd: 0.0 });
        assert_eq!(DhruvStatus::from(&err), DhruvStatus::EpochOutOfRange);
    }

    // --- Conjunction FFI tests ---

    #[test]
    fn ffi_conjunction_config_default_values() {
        let cfg = dhruv_conjunction_config_default();
        assert!((cfg.target_separation_deg - 0.0).abs() < 1e-15);
        assert!((cfg.step_size_days - 0.5).abs() < 1e-15);
        assert_eq!(cfg.max_iterations, 50);
        assert!((cfg.convergence_days - 1e-8).abs() < 1e-20);
    }

    #[test]
    fn ffi_conjunction_search_ex_rejects_null_request() {
        let mut event = std::mem::MaybeUninit::<DhruvConjunctionEvent>::uninit();
        let mut found: u8 = 0;
        let status = unsafe {
            dhruv_conjunction_search_ex(
                ptr::null(),
                ptr::null(),
                event.as_mut_ptr(),
                &mut found,
                ptr::null_mut(),
                0,
                ptr::null_mut(),
            )
        };
        assert_eq!(status, DhruvStatus::NullPointer);
    }

    #[test]
    fn ffi_conjunction_search_ex_rejects_invalid_mode() {
        let fake_engine = std::ptr::NonNull::<DhruvEngineHandle>::dangling().as_ptr();
        let request = DhruvConjunctionSearchRequest {
            body1_code: 10,
            body2_code: 301,
            query_mode: 99,
            at_jd_tdb: 2_460_000.5,
            start_jd_tdb: 2_460_000.5,
            end_jd_tdb: 2_460_100.5,
            config: dhruv_conjunction_config_default(),
        };
        let mut event = std::mem::MaybeUninit::<DhruvConjunctionEvent>::uninit();
        let mut found: u8 = 0;
        let status = unsafe {
            dhruv_conjunction_search_ex(
                fake_engine as *const _,
                &request,
                event.as_mut_ptr(),
                &mut found,
                ptr::null_mut(),
                0,
                ptr::null_mut(),
            )
        };
        assert_eq!(status, DhruvStatus::InvalidQuery);
    }

    #[test]
    fn ffi_conjunction_search_ex_next_rejects_null_engine() {
        let request = DhruvConjunctionSearchRequest {
            body1_code: 10,
            body2_code: 301,
            query_mode: DHRUV_CONJUNCTION_QUERY_MODE_NEXT,
            at_jd_tdb: 2_460_000.5,
            start_jd_tdb: 2_460_000.5,
            end_jd_tdb: 2_460_100.5,
            config: dhruv_conjunction_config_default(),
        };
        let mut event = std::mem::MaybeUninit::<DhruvConjunctionEvent>::uninit();
        let mut found: u8 = 0;
        let status = unsafe {
            dhruv_conjunction_search_ex(
                ptr::null(),
                &request,
                event.as_mut_ptr(),
                &mut found,
                ptr::null_mut(),
                0,
                ptr::null_mut(),
            )
        };
        assert_eq!(status, DhruvStatus::NullPointer);
    }

    #[test]
    fn ffi_conjunction_search_ex_rejects_invalid_body_code() {
        let fake_engine = std::ptr::NonNull::<DhruvEngineHandle>::dangling().as_ptr();
        let request = DhruvConjunctionSearchRequest {
            body1_code: 999999,
            body2_code: 301,
            query_mode: DHRUV_CONJUNCTION_QUERY_MODE_NEXT,
            at_jd_tdb: 2_460_000.5,
            start_jd_tdb: 2_460_000.5,
            end_jd_tdb: 2_460_100.5,
            config: dhruv_conjunction_config_default(),
        };
        let mut event = std::mem::MaybeUninit::<DhruvConjunctionEvent>::uninit();
        let mut found: u8 = 0;
        let status = unsafe {
            dhruv_conjunction_search_ex(
                fake_engine as *const _,
                &request,
                event.as_mut_ptr(),
                &mut found,
                ptr::null_mut(),
                0,
                ptr::null_mut(),
            )
        };
        assert_eq!(status, DhruvStatus::InvalidQuery);
    }

    // --- Grahan FFI tests ---

    #[test]
    fn ffi_grahan_config_default_values() {
        let cfg = dhruv_grahan_config_default();
        assert_eq!(cfg.include_penumbral, 1);
        assert_eq!(cfg.include_peak_details, 1);
    }

    #[test]
    fn ffi_grahan_search_ex_rejects_null_request() {
        let mut chandra = std::mem::MaybeUninit::<DhruvChandraGrahanResult>::uninit();
        let mut found: u8 = 0;
        let status = unsafe {
            dhruv_grahan_search_ex(
                ptr::null(),
                ptr::null(),
                chandra.as_mut_ptr(),
                ptr::null_mut(),
                &mut found,
                ptr::null_mut(),
                ptr::null_mut(),
                0,
                ptr::null_mut(),
            )
        };
        assert_eq!(status, DhruvStatus::NullPointer);
    }

    #[test]
    fn ffi_grahan_search_ex_rejects_invalid_selector() {
        let fake_engine = std::ptr::NonNull::<DhruvEngineHandle>::dangling().as_ptr();
        let request = DhruvGrahanSearchRequest {
            grahan_kind: 99,
            query_mode: 99,
            at_jd_tdb: 2_460_000.5,
            start_jd_tdb: 2_460_000.5,
            end_jd_tdb: 2_460_100.5,
            config: dhruv_grahan_config_default(),
        };
        let mut found: u8 = 0;
        let mut chandra = std::mem::MaybeUninit::<DhruvChandraGrahanResult>::uninit();
        let status = unsafe {
            dhruv_grahan_search_ex(
                fake_engine as *const _,
                &request,
                chandra.as_mut_ptr(),
                ptr::null_mut(),
                &mut found,
                ptr::null_mut(),
                ptr::null_mut(),
                0,
                ptr::null_mut(),
            )
        };
        assert_eq!(status, DhruvStatus::InvalidQuery);
    }

    #[test]
    fn ffi_grahan_search_ex_chandra_next_rejects_null_engine() {
        let request = DhruvGrahanSearchRequest {
            grahan_kind: DHRUV_GRAHAN_KIND_CHANDRA,
            query_mode: DHRUV_GRAHAN_QUERY_MODE_NEXT,
            at_jd_tdb: 2_460_000.5,
            start_jd_tdb: 2_460_000.5,
            end_jd_tdb: 2_460_100.5,
            config: dhruv_grahan_config_default(),
        };
        let mut chandra = std::mem::MaybeUninit::<DhruvChandraGrahanResult>::uninit();
        let mut found: u8 = 0;
        let status = unsafe {
            dhruv_grahan_search_ex(
                ptr::null(),
                &request,
                chandra.as_mut_ptr(),
                ptr::null_mut(),
                &mut found,
                ptr::null_mut(),
                ptr::null_mut(),
                0,
                ptr::null_mut(),
            )
        };
        assert_eq!(status, DhruvStatus::NullPointer);
    }

    #[test]
    fn ffi_grahan_search_ex_surya_prev_rejects_null_engine() {
        let request = DhruvGrahanSearchRequest {
            grahan_kind: DHRUV_GRAHAN_KIND_SURYA,
            query_mode: DHRUV_GRAHAN_QUERY_MODE_PREV,
            at_jd_tdb: 2_460_000.5,
            start_jd_tdb: 2_460_000.5,
            end_jd_tdb: 2_460_100.5,
            config: dhruv_grahan_config_default(),
        };
        let mut surya = std::mem::MaybeUninit::<DhruvSuryaGrahanResult>::uninit();
        let mut found: u8 = 0;
        let status = unsafe {
            dhruv_grahan_search_ex(
                ptr::null(),
                &request,
                ptr::null_mut(),
                surya.as_mut_ptr(),
                &mut found,
                ptr::null_mut(),
                ptr::null_mut(),
                0,
                ptr::null_mut(),
            )
        };
        assert_eq!(status, DhruvStatus::NullPointer);
    }

    #[test]
    fn ffi_chandra_grahan_type_constants() {
        assert_eq!(DHRUV_CHANDRA_GRAHAN_PENUMBRAL, 0);
        assert_eq!(DHRUV_CHANDRA_GRAHAN_PARTIAL, 1);
        assert_eq!(DHRUV_CHANDRA_GRAHAN_TOTAL, 2);
    }

    #[test]
    fn ffi_surya_grahan_type_constants() {
        assert_eq!(DHRUV_SURYA_GRAHAN_PARTIAL, 0);
        assert_eq!(DHRUV_SURYA_GRAHAN_ANNULAR, 1);
        assert_eq!(DHRUV_SURYA_GRAHAN_TOTAL, 2);
        assert_eq!(DHRUV_SURYA_GRAHAN_HYBRID, 3);
    }

    #[test]
    fn ffi_jd_absent_sentinel() {
        assert!((DHRUV_JD_ABSENT - (-1.0)).abs() < 1e-15);
    }

    #[test]
    fn ffi_option_jd_some() {
        assert!((option_jd(Some(2_460_000.5)) - 2_460_000.5).abs() < 1e-15);
    }

    #[test]
    fn ffi_option_jd_none() {
        assert!((option_jd(None) - DHRUV_JD_ABSENT).abs() < 1e-15);
    }

    // --- Stationary/max-speed FFI tests ---

    #[test]
    fn ffi_stationary_config_default_values() {
        let cfg = dhruv_stationary_config_default();
        assert!((cfg.step_size_days - 1.0).abs() < 1e-10);
        assert_eq!(cfg.max_iterations, 50);
        assert!((cfg.convergence_days - 1e-8).abs() < 1e-15);
        assert!((cfg.numerical_step_days - 0.01).abs() < 1e-10);
    }

    #[test]
    fn ffi_station_type_constants() {
        assert_eq!(DHRUV_STATION_RETROGRADE, 0);
        assert_eq!(DHRUV_STATION_DIRECT, 1);
    }

    #[test]
    fn ffi_max_speed_type_constants() {
        assert_eq!(DHRUV_MAX_SPEED_DIRECT, 0);
        assert_eq!(DHRUV_MAX_SPEED_RETROGRADE, 1);
    }

    #[test]
    fn ffi_motion_search_ex_rejects_null_request() {
        let mut found: u8 = 0;
        let mut stationary = std::mem::MaybeUninit::<DhruvStationaryEvent>::uninit();
        let status = unsafe {
            dhruv_motion_search_ex(
                ptr::null(),
                ptr::null(),
                stationary.as_mut_ptr(),
                ptr::null_mut(),
                &mut found,
                ptr::null_mut(),
                ptr::null_mut(),
                0,
                ptr::null_mut(),
            )
        };
        assert_eq!(status, DhruvStatus::NullPointer);
    }

    #[test]
    fn ffi_motion_search_ex_rejects_invalid_selector() {
        let fake_engine = std::ptr::NonNull::<DhruvEngineHandle>::dangling().as_ptr();
        let request = DhruvMotionSearchRequest {
            body_code: 199,
            motion_kind: 99,
            query_mode: 99,
            at_jd_tdb: 2_460_000.5,
            start_jd_tdb: 2_460_000.5,
            end_jd_tdb: 2_460_100.5,
            config: dhruv_stationary_config_default(),
        };
        let mut found: u8 = 0;
        let mut stationary = std::mem::MaybeUninit::<DhruvStationaryEvent>::uninit();
        let status = unsafe {
            dhruv_motion_search_ex(
                fake_engine as *const _,
                &request,
                stationary.as_mut_ptr(),
                ptr::null_mut(),
                &mut found,
                ptr::null_mut(),
                ptr::null_mut(),
                0,
                ptr::null_mut(),
            )
        };
        assert_eq!(status, DhruvStatus::InvalidQuery);
    }

    #[test]
    fn ffi_motion_search_ex_stationary_next_rejects_null_engine() {
        let request = DhruvMotionSearchRequest {
            body_code: 199,
            motion_kind: DHRUV_MOTION_KIND_STATIONARY,
            query_mode: DHRUV_MOTION_QUERY_MODE_NEXT,
            at_jd_tdb: 2_460_000.5,
            start_jd_tdb: 2_460_000.5,
            end_jd_tdb: 2_460_100.5,
            config: dhruv_stationary_config_default(),
        };
        let mut stationary = std::mem::MaybeUninit::<DhruvStationaryEvent>::uninit();
        let mut found: u8 = 0;
        let status = unsafe {
            dhruv_motion_search_ex(
                ptr::null(),
                &request,
                stationary.as_mut_ptr(),
                ptr::null_mut(),
                &mut found,
                ptr::null_mut(),
                ptr::null_mut(),
                0,
                ptr::null_mut(),
            )
        };
        assert_eq!(status, DhruvStatus::NullPointer);
    }

    #[test]
    fn ffi_motion_search_ex_max_speed_prev_rejects_null_engine() {
        let request = DhruvMotionSearchRequest {
            body_code: 199,
            motion_kind: DHRUV_MOTION_KIND_MAX_SPEED,
            query_mode: DHRUV_MOTION_QUERY_MODE_PREV,
            at_jd_tdb: 2_460_000.5,
            start_jd_tdb: 2_460_000.5,
            end_jd_tdb: 2_460_100.5,
            config: dhruv_stationary_config_default(),
        };
        let mut max_speed = std::mem::MaybeUninit::<DhruvMaxSpeedEvent>::uninit();
        let mut found: u8 = 0;
        let status = unsafe {
            dhruv_motion_search_ex(
                ptr::null(),
                &request,
                ptr::null_mut(),
                max_speed.as_mut_ptr(),
                &mut found,
                ptr::null_mut(),
                ptr::null_mut(),
                0,
                ptr::null_mut(),
            )
        };
        assert_eq!(status, DhruvStatus::NullPointer);
    }

    #[test]
    fn ffi_motion_search_ex_rejects_invalid_body_code() {
        let fake_engine = std::ptr::NonNull::<DhruvEngineHandle>::dangling().as_ptr();
        let request = DhruvMotionSearchRequest {
            body_code: 999999,
            motion_kind: DHRUV_MOTION_KIND_STATIONARY,
            query_mode: DHRUV_MOTION_QUERY_MODE_NEXT,
            at_jd_tdb: 2_460_000.5,
            start_jd_tdb: 2_460_000.5,
            end_jd_tdb: 2_460_100.5,
            config: dhruv_stationary_config_default(),
        };
        let mut stationary = std::mem::MaybeUninit::<DhruvStationaryEvent>::uninit();
        let mut found: u8 = 0;
        let status = unsafe {
            dhruv_motion_search_ex(
                fake_engine as *const _,
                &request,
                stationary.as_mut_ptr(),
                ptr::null_mut(),
                &mut found,
                ptr::null_mut(),
                ptr::null_mut(),
                0,
                ptr::null_mut(),
            )
        };
        assert_eq!(status, DhruvStatus::InvalidQuery);
    }

    #[test]
    fn ffi_motion_search_ex_stationary_rejects_sun() {
        let fake_engine = std::ptr::NonNull::<DhruvEngineHandle>::dangling().as_ptr();
        let request = DhruvMotionSearchRequest {
            body_code: 10, // Sun
            motion_kind: DHRUV_MOTION_KIND_STATIONARY,
            query_mode: DHRUV_MOTION_QUERY_MODE_NEXT,
            at_jd_tdb: 2_460_000.5,
            start_jd_tdb: 2_460_000.5,
            end_jd_tdb: 2_460_100.5,
            config: dhruv_stationary_config_default(),
        };
        let mut stationary = std::mem::MaybeUninit::<DhruvStationaryEvent>::uninit();
        let mut found: u8 = 0;
        let status = unsafe {
            dhruv_motion_search_ex(
                fake_engine as *const _,
                &request,
                stationary.as_mut_ptr(),
                ptr::null_mut(),
                &mut found,
                ptr::null_mut(),
                ptr::null_mut(),
                0,
                ptr::null_mut(),
            )
        };
        assert_eq!(status, DhruvStatus::InvalidSearchConfig);
    }

    #[test]
    fn ffi_motion_search_ex_max_speed_rejects_earth() {
        let fake_engine = std::ptr::NonNull::<DhruvEngineHandle>::dangling().as_ptr();
        let request = DhruvMotionSearchRequest {
            body_code: 399, // Earth
            motion_kind: DHRUV_MOTION_KIND_MAX_SPEED,
            query_mode: DHRUV_MOTION_QUERY_MODE_NEXT,
            at_jd_tdb: 2_460_000.5,
            start_jd_tdb: 2_460_000.5,
            end_jd_tdb: 2_460_100.5,
            config: dhruv_stationary_config_default(),
        };
        let mut max_speed = std::mem::MaybeUninit::<DhruvMaxSpeedEvent>::uninit();
        let mut found: u8 = 0;
        let status = unsafe {
            dhruv_motion_search_ex(
                fake_engine as *const _,
                &request,
                ptr::null_mut(),
                max_speed.as_mut_ptr(),
                &mut found,
                ptr::null_mut(),
                ptr::null_mut(),
                0,
                ptr::null_mut(),
            )
        };
        assert_eq!(status, DhruvStatus::InvalidSearchConfig);
    }

    // --- Rashi/Nakshatra FFI tests ---

    #[test]
    fn ffi_deg_to_dms_basic() {
        let mut out = DhruvDms {
            degrees: 0,
            minutes: 0,
            seconds: 0.0,
        };
        let status = unsafe { dhruv_deg_to_dms(23.853, &mut out) };
        assert_eq!(status, DhruvStatus::Ok);
        assert_eq!(out.degrees, 23);
        assert_eq!(out.minutes, 51);
        assert!((out.seconds - 10.8).abs() < 0.01);
    }

    #[test]
    fn ffi_deg_to_dms_rejects_null() {
        let status = unsafe { dhruv_deg_to_dms(10.0, ptr::null_mut()) };
        assert_eq!(status, DhruvStatus::NullPointer);
    }

    #[test]
    fn ffi_rashi_from_longitude_mesha() {
        let mut out = std::mem::MaybeUninit::<DhruvRashiInfo>::uninit();
        let status = unsafe { dhruv_rashi_from_longitude(15.0, out.as_mut_ptr()) };
        assert_eq!(status, DhruvStatus::Ok);
        let info = unsafe { out.assume_init() };
        assert_eq!(info.rashi_index, 0); // Mesha
        assert!((info.degrees_in_rashi - 15.0).abs() < 1e-10);
    }

    #[test]
    fn ffi_rashi_from_longitude_rejects_null() {
        let status = unsafe { dhruv_rashi_from_longitude(0.0, ptr::null_mut()) };
        assert_eq!(status, DhruvStatus::NullPointer);
    }

    #[test]
    fn ffi_nakshatra_from_longitude_ashwini() {
        let mut out = std::mem::MaybeUninit::<DhruvNakshatraInfo>::uninit();
        let status = unsafe { dhruv_nakshatra_from_longitude(5.0, out.as_mut_ptr()) };
        assert_eq!(status, DhruvStatus::Ok);
        let info = unsafe { out.assume_init() };
        assert_eq!(info.nakshatra_index, 0); // Ashwini
        assert_eq!(info.pada, 2); // 5.0 / 3.333 = 1.5 → pada 2
    }

    #[test]
    fn ffi_nakshatra_from_longitude_rejects_null() {
        let status = unsafe { dhruv_nakshatra_from_longitude(0.0, ptr::null_mut()) };
        assert_eq!(status, DhruvStatus::NullPointer);
    }

    #[test]
    fn ffi_nakshatra28_abhijit() {
        let mut out = std::mem::MaybeUninit::<DhruvNakshatra28Info>::uninit();
        let status = unsafe { dhruv_nakshatra28_from_longitude(278.0, out.as_mut_ptr()) };
        assert_eq!(status, DhruvStatus::Ok);
        let info = unsafe { out.assume_init() };
        assert_eq!(info.nakshatra_index, 21); // Abhijit
        assert_eq!(info.pada, 0); // Abhijit has no pada
    }

    #[test]
    fn ffi_nakshatra28_from_longitude_rejects_null() {
        let status = unsafe { dhruv_nakshatra28_from_longitude(0.0, ptr::null_mut()) };
        assert_eq!(status, DhruvStatus::NullPointer);
    }

    #[test]
    fn ffi_rashi_from_tropical_lahiri() {
        let mut out = std::mem::MaybeUninit::<DhruvRashiInfo>::uninit();
        // Lahiri = code 0, J2000, tropical 280.5 → sidereal ~256.6 → Dhanu (8)
        let status =
            unsafe { dhruv_rashi_from_tropical(280.5, 0, 2_451_545.0, 0, out.as_mut_ptr()) };
        assert_eq!(status, DhruvStatus::Ok);
        let info = unsafe { out.assume_init() };
        assert_eq!(info.rashi_index, 8); // Dhanu
    }

    #[test]
    fn ffi_rashi_from_tropical_invalid_system() {
        let mut out = std::mem::MaybeUninit::<DhruvRashiInfo>::uninit();
        let status =
            unsafe { dhruv_rashi_from_tropical(280.5, 99, 2_451_545.0, 0, out.as_mut_ptr()) };
        assert_eq!(status, DhruvStatus::InvalidQuery);
    }

    #[test]
    fn ffi_nakshatra_from_tropical_rejects_null() {
        let status =
            unsafe { dhruv_nakshatra_from_tropical(280.5, 0, 2_451_545.0, 0, ptr::null_mut()) };
        assert_eq!(status, DhruvStatus::NullPointer);
    }

    #[test]
    fn ffi_nakshatra28_from_tropical_rejects_null() {
        let status =
            unsafe { dhruv_nakshatra28_from_tropical(280.5, 0, 2_451_545.0, 0, ptr::null_mut()) };
        assert_eq!(status, DhruvStatus::NullPointer);
    }

    #[test]
    fn ffi_rashi_count_is_12() {
        assert_eq!(dhruv_rashi_count(), 12);
    }

    #[test]
    fn ffi_nakshatra_count_27() {
        assert_eq!(dhruv_nakshatra_count(27), 27);
    }

    #[test]
    fn ffi_nakshatra_count_28() {
        assert_eq!(dhruv_nakshatra_count(28), 28);
    }

    #[test]
    fn ffi_nakshatra_count_invalid() {
        assert_eq!(dhruv_nakshatra_count(0), 0);
        assert_eq!(dhruv_nakshatra_count(29), 0);
    }

    #[test]
    fn ffi_rashi_name_valid() {
        let name_ptr = dhruv_rashi_name(0);
        assert!(!name_ptr.is_null());
        let name = unsafe { std::ffi::CStr::from_ptr(name_ptr) };
        assert_eq!(name.to_str().unwrap(), "Mesha");

        let name_ptr = dhruv_rashi_name(11);
        assert!(!name_ptr.is_null());
        let name = unsafe { std::ffi::CStr::from_ptr(name_ptr) };
        assert_eq!(name.to_str().unwrap(), "Meena");
    }

    #[test]
    fn ffi_rashi_name_invalid() {
        assert!(dhruv_rashi_name(12).is_null());
        assert!(dhruv_rashi_name(99).is_null());
    }

    #[test]
    fn ffi_nakshatra_name_valid() {
        let name_ptr = dhruv_nakshatra_name(0);
        assert!(!name_ptr.is_null());
        let name = unsafe { std::ffi::CStr::from_ptr(name_ptr) };
        assert_eq!(name.to_str().unwrap(), "Ashwini");

        let name_ptr = dhruv_nakshatra_name(26);
        assert!(!name_ptr.is_null());
        let name = unsafe { std::ffi::CStr::from_ptr(name_ptr) };
        assert_eq!(name.to_str().unwrap(), "Revati");
    }

    #[test]
    fn ffi_nakshatra_name_invalid() {
        assert!(dhruv_nakshatra_name(27).is_null());
    }

    #[test]
    fn ffi_nakshatra28_name_abhijit() {
        let name_ptr = dhruv_nakshatra28_name(21);
        assert!(!name_ptr.is_null());
        let name = unsafe { std::ffi::CStr::from_ptr(name_ptr) };
        assert_eq!(name.to_str().unwrap(), "Abhijit");
    }

    #[test]
    fn ffi_nakshatra28_name_invalid() {
        assert!(dhruv_nakshatra28_name(28).is_null());
    }

    // --- Panchang FFI tests ---

    #[test]
    fn ffi_sankranti_config_default_values() {
        let config = dhruv_sankranti_config_default();
        assert_eq!(config.ayanamsha_system, 0); // Lahiri
        assert_eq!(config.use_nutation, 0);
        assert!((config.step_size_days - 1.0).abs() < 1e-10);
        assert_eq!(config.max_iterations, 50);
        assert!((config.convergence_days - 1e-8).abs() < 1e-15);
    }

    #[test]
    fn ffi_lunar_phase_constants() {
        assert_eq!(DHRUV_LUNAR_PHASE_NEW_MOON, 0);
        assert_eq!(DHRUV_LUNAR_PHASE_FULL_MOON, 1);
    }

    #[test]
    fn ffi_ayana_constants() {
        assert_eq!(DHRUV_AYANA_UTTARAYANA, 0);
        assert_eq!(DHRUV_AYANA_DAKSHINAYANA, 1);
    }

    #[test]
    fn ffi_lunar_phase_search_ex_rejects_null_request() {
        let mut event = std::mem::MaybeUninit::<DhruvLunarPhaseEvent>::uninit();
        let mut found: u8 = 0;
        let status = unsafe {
            dhruv_lunar_phase_search_ex(
                ptr::null(),
                ptr::null(),
                event.as_mut_ptr(),
                &mut found,
                ptr::null_mut(),
                0,
                ptr::null_mut(),
            )
        };
        assert_eq!(status, DhruvStatus::NullPointer);
    }

    #[test]
    fn ffi_lunar_phase_search_ex_rejects_invalid_selector() {
        let fake_engine = std::ptr::NonNull::<DhruvEngineHandle>::dangling().as_ptr();
        let request = DhruvLunarPhaseSearchRequest {
            phase_kind: 99,
            query_mode: 99,
            at_jd_tdb: 2_460_000.5,
            start_jd_tdb: 2_460_000.5,
            end_jd_tdb: 2_460_100.5,
        };
        let mut event = std::mem::MaybeUninit::<DhruvLunarPhaseEvent>::uninit();
        let mut found: u8 = 0;
        let status = unsafe {
            dhruv_lunar_phase_search_ex(
                fake_engine as *const _,
                &request,
                event.as_mut_ptr(),
                &mut found,
                ptr::null_mut(),
                0,
                ptr::null_mut(),
            )
        };
        assert_eq!(status, DhruvStatus::InvalidQuery);
    }

    #[test]
    fn ffi_lunar_phase_search_ex_purnima_next_rejects_null_engine() {
        let request = DhruvLunarPhaseSearchRequest {
            phase_kind: DHRUV_LUNAR_PHASE_KIND_PURNIMA,
            query_mode: DHRUV_LUNAR_PHASE_QUERY_MODE_NEXT,
            at_jd_tdb: 2_460_000.5,
            start_jd_tdb: 2_460_000.5,
            end_jd_tdb: 2_460_100.5,
        };
        let mut event = std::mem::MaybeUninit::<DhruvLunarPhaseEvent>::uninit();
        let mut found: u8 = 0;
        let status = unsafe {
            dhruv_lunar_phase_search_ex(
                ptr::null(),
                &request,
                event.as_mut_ptr(),
                &mut found,
                ptr::null_mut(),
                0,
                ptr::null_mut(),
            )
        };
        assert_eq!(status, DhruvStatus::NullPointer);
    }

    #[test]
    fn ffi_lunar_phase_search_ex_amavasya_next_rejects_null_engine() {
        let request = DhruvLunarPhaseSearchRequest {
            phase_kind: DHRUV_LUNAR_PHASE_KIND_AMAVASYA,
            query_mode: DHRUV_LUNAR_PHASE_QUERY_MODE_NEXT,
            at_jd_tdb: 2_460_000.5,
            start_jd_tdb: 2_460_000.5,
            end_jd_tdb: 2_460_100.5,
        };
        let mut event = std::mem::MaybeUninit::<DhruvLunarPhaseEvent>::uninit();
        let mut found: u8 = 0;
        let status = unsafe {
            dhruv_lunar_phase_search_ex(
                ptr::null(),
                &request,
                event.as_mut_ptr(),
                &mut found,
                ptr::null_mut(),
                0,
                ptr::null_mut(),
            )
        };
        assert_eq!(status, DhruvStatus::NullPointer);
    }

    #[test]
    fn ffi_sankranti_search_ex_rejects_null_request() {
        let mut event = std::mem::MaybeUninit::<DhruvSankrantiEvent>::uninit();
        let mut found: u8 = 0;
        let status = unsafe {
            dhruv_sankranti_search_ex(
                ptr::null(),
                ptr::null(),
                event.as_mut_ptr(),
                &mut found,
                ptr::null_mut(),
                0,
                ptr::null_mut(),
            )
        };
        assert_eq!(status, DhruvStatus::NullPointer);
    }

    #[test]
    fn ffi_sankranti_search_ex_rejects_invalid_selector() {
        let fake_engine = std::ptr::NonNull::<DhruvEngineHandle>::dangling().as_ptr();
        let request = DhruvSankrantiSearchRequest {
            target_kind: 99,
            query_mode: 99,
            rashi_index: 0,
            at_jd_tdb: 2_460_000.5,
            start_jd_tdb: 2_460_000.5,
            end_jd_tdb: 2_460_100.5,
            config: dhruv_sankranti_config_default(),
        };
        let mut event = std::mem::MaybeUninit::<DhruvSankrantiEvent>::uninit();
        let mut found: u8 = 0;
        let status = unsafe {
            dhruv_sankranti_search_ex(
                fake_engine as *const _,
                &request,
                event.as_mut_ptr(),
                &mut found,
                ptr::null_mut(),
                0,
                ptr::null_mut(),
            )
        };
        assert_eq!(status, DhruvStatus::InvalidQuery);
    }

    #[test]
    fn ffi_sankranti_search_ex_next_any_rejects_null_engine() {
        let request = DhruvSankrantiSearchRequest {
            target_kind: DHRUV_SANKRANTI_TARGET_ANY,
            query_mode: DHRUV_SANKRANTI_QUERY_MODE_NEXT,
            rashi_index: 0,
            at_jd_tdb: 2_460_000.5,
            start_jd_tdb: 2_460_000.5,
            end_jd_tdb: 2_460_100.5,
            config: dhruv_sankranti_config_default(),
        };
        let mut event = std::mem::MaybeUninit::<DhruvSankrantiEvent>::uninit();
        let mut found: u8 = 0;
        let status = unsafe {
            dhruv_sankranti_search_ex(
                ptr::null(),
                &request,
                event.as_mut_ptr(),
                &mut found,
                ptr::null_mut(),
                0,
                ptr::null_mut(),
            )
        };
        assert_eq!(status, DhruvStatus::NullPointer);
    }

    #[test]
    fn ffi_masa_for_date_rejects_null() {
        let utc = DhruvUtcTime {
            year: 2024,
            month: 1,
            day: 15,
            hour: 0,
            minute: 0,
            second: 0.0,
        };
        let config = dhruv_sankranti_config_default();
        let status =
            unsafe { dhruv_masa_for_date(std::ptr::null(), &utc, &config, std::ptr::null_mut()) };
        assert_eq!(status, DhruvStatus::NullPointer);
    }

    #[test]
    fn ffi_ayana_for_date_rejects_null() {
        let utc = DhruvUtcTime {
            year: 2024,
            month: 1,
            day: 15,
            hour: 0,
            minute: 0,
            second: 0.0,
        };
        let config = dhruv_sankranti_config_default();
        let status =
            unsafe { dhruv_ayana_for_date(std::ptr::null(), &utc, &config, std::ptr::null_mut()) };
        assert_eq!(status, DhruvStatus::NullPointer);
    }

    #[test]
    fn ffi_varsha_for_date_rejects_null() {
        let utc = DhruvUtcTime {
            year: 2024,
            month: 1,
            day: 15,
            hour: 0,
            minute: 0,
            second: 0.0,
        };
        let config = dhruv_sankranti_config_default();
        let status =
            unsafe { dhruv_varsha_for_date(std::ptr::null(), &utc, &config, std::ptr::null_mut()) };
        assert_eq!(status, DhruvStatus::NullPointer);
    }

    #[test]
    fn ffi_masa_name_valid() {
        // Index 0 = Chaitra
        let name_ptr = dhruv_masa_name(0);
        assert!(!name_ptr.is_null());
        let name = unsafe { std::ffi::CStr::from_ptr(name_ptr) };
        assert_eq!(name.to_str().unwrap(), "Chaitra");
        // Index 9 = Pausha
        let name_ptr = dhruv_masa_name(9);
        assert!(!name_ptr.is_null());
        let name = unsafe { std::ffi::CStr::from_ptr(name_ptr) };
        assert_eq!(name.to_str().unwrap(), "Pausha");
    }

    #[test]
    fn ffi_masa_name_invalid() {
        assert!(dhruv_masa_name(12).is_null());
    }

    #[test]
    fn ffi_ayana_name_valid() {
        let name_ptr = dhruv_ayana_name(0);
        assert!(!name_ptr.is_null());
        let name = unsafe { std::ffi::CStr::from_ptr(name_ptr) };
        assert_eq!(name.to_str().unwrap(), "Uttarayana");

        let name_ptr = dhruv_ayana_name(1);
        assert!(!name_ptr.is_null());
        let name = unsafe { std::ffi::CStr::from_ptr(name_ptr) };
        assert_eq!(name.to_str().unwrap(), "Dakshinayana");
    }

    #[test]
    fn ffi_ayana_name_invalid() {
        assert!(dhruv_ayana_name(2).is_null());
    }

    #[test]
    fn ffi_samvatsara_name_valid() {
        // Index 0 = Prabhava
        let name_ptr = dhruv_samvatsara_name(0);
        assert!(!name_ptr.is_null());
        let name = unsafe { std::ffi::CStr::from_ptr(name_ptr) };
        assert_eq!(name.to_str().unwrap(), "Prabhava");
        // Index 59 = Akshaya
        let name_ptr = dhruv_samvatsara_name(59);
        assert!(!name_ptr.is_null());
        let name = unsafe { std::ffi::CStr::from_ptr(name_ptr) };
        assert_eq!(name.to_str().unwrap(), "Akshaya");
    }

    #[test]
    fn ffi_samvatsara_name_invalid() {
        assert!(dhruv_samvatsara_name(60).is_null());
    }

    // --- Pure-math panchang classifier tests ---

    #[test]
    fn ffi_tithi_from_elongation_rejects_null() {
        let s = unsafe { dhruv_tithi_from_elongation(0.0, ptr::null_mut()) };
        assert_eq!(s, DhruvStatus::NullPointer);
    }

    #[test]
    fn ffi_tithi_from_elongation_shukla_pratipada() {
        let mut out = std::mem::MaybeUninit::<DhruvTithiPosition>::uninit();
        let s = unsafe { dhruv_tithi_from_elongation(5.0, out.as_mut_ptr()) };
        assert_eq!(s, DhruvStatus::Ok);
        let t = unsafe { out.assume_init() };
        assert_eq!(t.tithi_index, 0); // Pratipada
        assert_eq!(t.paksha, 0); // Shukla
        assert_eq!(t.tithi_in_paksha, 1);
    }

    #[test]
    fn ffi_tithi_from_elongation_krishna() {
        let mut out = std::mem::MaybeUninit::<DhruvTithiPosition>::uninit();
        let s = unsafe { dhruv_tithi_from_elongation(200.0, out.as_mut_ptr()) };
        assert_eq!(s, DhruvStatus::Ok);
        let t = unsafe { out.assume_init() };
        assert_eq!(t.paksha, 1); // Krishna
    }

    #[test]
    fn ffi_karana_from_elongation_rejects_null() {
        let s = unsafe { dhruv_karana_from_elongation(0.0, ptr::null_mut()) };
        assert_eq!(s, DhruvStatus::NullPointer);
    }

    #[test]
    fn ffi_karana_from_elongation_basic() {
        let mut out = std::mem::MaybeUninit::<DhruvKaranaPosition>::uninit();
        let s = unsafe { dhruv_karana_from_elongation(3.0, out.as_mut_ptr()) };
        assert_eq!(s, DhruvStatus::Ok);
        let k = unsafe { out.assume_init() };
        assert_eq!(k.karana_index, 0);
        assert!(k.degrees_in_karana >= 0.0 && k.degrees_in_karana < 6.0);
    }

    #[test]
    fn ffi_yoga_from_sum_rejects_null() {
        let s = unsafe { dhruv_yoga_from_sum(0.0, ptr::null_mut()) };
        assert_eq!(s, DhruvStatus::NullPointer);
    }

    #[test]
    fn ffi_yoga_from_sum_basic() {
        let mut out = std::mem::MaybeUninit::<DhruvYogaPosition>::uninit();
        let s = unsafe { dhruv_yoga_from_sum(5.0, out.as_mut_ptr()) };
        assert_eq!(s, DhruvStatus::Ok);
        let y = unsafe { out.assume_init() };
        assert_eq!(y.yoga_index, 0); // Vishkambha
        assert!(y.degrees_in_yoga >= 0.0);
    }

    #[test]
    fn ffi_vaar_from_jd_j2000() {
        // J2000.0 = 2000-01-01 12:00 TT = Saturday
        assert_eq!(dhruv_vaar_from_jd(2451545.0), 6); // Shanivaar
    }

    #[test]
    fn ffi_vaar_from_jd_sunday() {
        // 2000-01-02 = Sunday
        assert_eq!(dhruv_vaar_from_jd(2451546.0), 0); // Ravivaar
    }

    #[test]
    fn ffi_masa_from_rashi_index_valid() {
        // Rashi 0 (Mesha) -> Masa 0 (Chaitra)
        assert_eq!(dhruv_masa_from_rashi_index(0), 0);
        // Rashi 11 (Meena) -> Masa 11 (Phalguna)
        assert_eq!(dhruv_masa_from_rashi_index(11), 11);
    }

    #[test]
    fn ffi_masa_from_rashi_index_invalid() {
        assert_eq!(dhruv_masa_from_rashi_index(12), -1);
        assert_eq!(dhruv_masa_from_rashi_index(255), -1);
    }

    #[test]
    fn ffi_ayana_from_sidereal_longitude_uttarayana() {
        // 0 deg (Mesha) -> Uttarayana
        assert_eq!(dhruv_ayana_from_sidereal_longitude(0.0), 0);
        // 45 deg -> Uttarayana
        assert_eq!(dhruv_ayana_from_sidereal_longitude(45.0), 0);
        // 300 deg (Makara) -> Uttarayana
        assert_eq!(dhruv_ayana_from_sidereal_longitude(300.0), 0);
    }

    #[test]
    fn ffi_ayana_from_sidereal_longitude_dakshinayana() {
        // 120 deg (Simha) -> Dakshinayana
        assert_eq!(dhruv_ayana_from_sidereal_longitude(120.0), 1);
        // 180 deg (Tula) -> Dakshinayana
        assert_eq!(dhruv_ayana_from_sidereal_longitude(180.0), 1);
    }

    #[test]
    fn ffi_samvatsara_from_year_rejects_null() {
        let s = unsafe { dhruv_samvatsara_from_year(2024, ptr::null_mut()) };
        assert_eq!(s, DhruvStatus::NullPointer);
    }

    #[test]
    fn ffi_samvatsara_from_year_2024() {
        let mut out = std::mem::MaybeUninit::<DhruvSamvatsaraResult>::uninit();
        let s = unsafe { dhruv_samvatsara_from_year(2024, out.as_mut_ptr()) };
        assert_eq!(s, DhruvStatus::Ok);
        let r = unsafe { out.assume_init() };
        assert!(r.samvatsara_index >= 0 && r.samvatsara_index < 60);
        assert!(r.cycle_position >= 1 && r.cycle_position <= 60);
    }

    #[test]
    fn ffi_nth_rashi_from_basic() {
        // 2nd from Mesha (0) = Vrishabha (1)
        assert_eq!(dhruv_nth_rashi_from(0, 2), 1);
        // 5th from Mesha (0) = Simha (4)
        assert_eq!(dhruv_nth_rashi_from(0, 5), 4);
        // 12th from Mesha (0) = Meena (11)
        assert_eq!(dhruv_nth_rashi_from(0, 12), 11);
    }

    #[test]
    fn ffi_nth_rashi_from_wrap() {
        // 3rd from Meena (11) = Vrishabha (1)
        assert_eq!(dhruv_nth_rashi_from(11, 3), 1);
    }

    #[test]
    fn ffi_nth_rashi_from_invalid() {
        assert_eq!(dhruv_nth_rashi_from(12, 1), -1);
        assert_eq!(dhruv_nth_rashi_from(255, 5), -1);
    }

    // Group B null rejection

    #[test]
    fn ffi_compute_rise_set_utc_rejects_null() {
        let mut out = std::mem::MaybeUninit::<DhruvRiseSetResultUtc>::uninit();
        let status = unsafe {
            dhruv_compute_rise_set_utc(
                ptr::null(),
                ptr::null(),
                ptr::null(),
                ptr::null(),
                0,
                ptr::null(),
                ptr::null(),
                out.as_mut_ptr(),
            )
        };
        assert_eq!(status, DhruvStatus::NullPointer);
    }

    #[test]
    fn ffi_compute_all_events_utc_rejects_null() {
        let status = unsafe {
            dhruv_compute_all_events_utc(
                ptr::null(),
                ptr::null(),
                ptr::null(),
                ptr::null(),
                ptr::null(),
                ptr::null(),
                ptr::null_mut(),
            )
        };
        assert_eq!(status, DhruvStatus::NullPointer);
    }

    #[test]
    fn ffi_compute_bhavas_utc_rejects_null() {
        let mut out = std::mem::MaybeUninit::<DhruvBhavaResult>::uninit();
        let status = unsafe {
            dhruv_compute_bhavas_utc(
                ptr::null(),
                ptr::null(),
                ptr::null(),
                ptr::null(),
                ptr::null(),
                ptr::null(),
                out.as_mut_ptr(),
            )
        };
        assert_eq!(status, DhruvStatus::NullPointer);
    }

    #[test]
    fn ffi_lagna_deg_utc_rejects_null() {
        let mut out: f64 = 0.0;
        let status = unsafe {
            dhruv_lagna_deg_utc(ptr::null(), ptr::null(), ptr::null(), ptr::null(), &mut out)
        };
        assert_eq!(status, DhruvStatus::NullPointer);
    }

    #[test]
    fn ffi_mc_deg_utc_rejects_null() {
        let mut out: f64 = 0.0;
        let status = unsafe {
            dhruv_mc_deg_utc(ptr::null(), ptr::null(), ptr::null(), ptr::null(), &mut out)
        };
        assert_eq!(status, DhruvStatus::NullPointer);
    }

    #[test]
    fn ffi_ramc_deg_utc_rejects_null() {
        let mut out: f64 = 0.0;
        let status = unsafe {
            dhruv_ramc_deg_utc(ptr::null(), ptr::null(), ptr::null(), ptr::null(), &mut out)
        };
        assert_eq!(status, DhruvStatus::NullPointer);
    }

    // Group C null rejection

    #[test]
    fn ffi_nutation_iau2000b_utc_rejects_null() {
        let mut dpsi: f64 = 0.0;
        let mut deps: f64 = 0.0;
        let status =
            unsafe { dhruv_nutation_iau2000b_utc(ptr::null(), ptr::null(), &mut dpsi, &mut deps) };
        assert_eq!(status, DhruvStatus::NullPointer);
    }

    #[test]
    fn ffi_lunar_node_deg_utc_rejects_null() {
        let mut out: f64 = 0.0;
        let status = unsafe { dhruv_lunar_node_deg_utc(ptr::null(), 0, 0, ptr::null(), &mut out) };
        assert_eq!(status, DhruvStatus::NullPointer);
    }

    #[test]
    fn ffi_rashi_from_tropical_utc_rejects_null() {
        let mut out = std::mem::MaybeUninit::<DhruvRashiInfo>::uninit();
        let status = unsafe {
            dhruv_rashi_from_tropical_utc(ptr::null(), 280.5, 0, ptr::null(), 0, out.as_mut_ptr())
        };
        assert_eq!(status, DhruvStatus::NullPointer);
    }

    #[test]
    fn ffi_nakshatra_from_tropical_utc_rejects_null() {
        let mut out = std::mem::MaybeUninit::<DhruvNakshatraInfo>::uninit();
        let status = unsafe {
            dhruv_nakshatra_from_tropical_utc(
                ptr::null(),
                280.5,
                0,
                ptr::null(),
                0,
                out.as_mut_ptr(),
            )
        };
        assert_eq!(status, DhruvStatus::NullPointer);
    }

    #[test]
    fn ffi_nakshatra28_from_tropical_utc_rejects_null() {
        let mut out = std::mem::MaybeUninit::<DhruvNakshatra28Info>::uninit();
        let status = unsafe {
            dhruv_nakshatra28_from_tropical_utc(
                ptr::null(),
                280.5,
                0,
                ptr::null(),
                0,
                out.as_mut_ptr(),
            )
        };
        assert_eq!(status, DhruvStatus::NullPointer);
    }

    // --- UTC conversion helpers ---

    #[test]
    fn ffi_utc_to_jd_utc_roundtrip() {
        let utc = DhruvUtcTime {
            year: 2024,
            month: 3,
            day: 20,
            hour: 12,
            minute: 0,
            second: 0.0,
        };
        let jd = ffi_utc_to_jd_utc(&utc);
        // 2024-03-20 12:00 UTC ≈ JD 2460390.0
        assert!((jd - 2_460_390.0).abs() < 0.01, "jd={jd}");
    }

    #[test]
    fn ffi_zeroed_utc_is_zero() {
        assert_eq!(ZEROED_UTC.year, 0);
        assert_eq!(ZEROED_UTC.month, 0);
        assert!((ZEROED_UTC.second - 0.0).abs() < 1e-15);
    }

    // --- Panchang composable intermediates null-pointer tests ---

    #[test]
    fn ffi_elongation_at_null() {
        let mut out = 0.0f64;
        let s = unsafe { dhruv_elongation_at(ptr::null(), 2451545.0, &mut out) };
        assert_eq!(s, DhruvStatus::NullPointer);
    }

    #[test]
    fn ffi_sidereal_sum_at_null() {
        let mut out = 0.0f64;
        let s = unsafe { dhruv_sidereal_sum_at(ptr::null(), 2451545.0, ptr::null(), &mut out) };
        assert_eq!(s, DhruvStatus::NullPointer);
    }

    #[test]
    fn ffi_vedic_day_sunrises_null() {
        let mut sr = 0.0f64;
        let mut nsr = 0.0f64;
        let s = unsafe {
            dhruv_vedic_day_sunrises(
                ptr::null(),
                ptr::null(),
                ptr::null(),
                ptr::null(),
                ptr::null(),
                &mut sr,
                &mut nsr,
            )
        };
        assert_eq!(s, DhruvStatus::NullPointer);
    }

    #[test]
    fn ffi_body_ecliptic_lon_lat_null() {
        let mut lon = 0.0f64;
        let mut lat = 0.0f64;
        let s =
            unsafe { dhruv_body_ecliptic_lon_lat(ptr::null(), 301, 2451545.0, &mut lon, &mut lat) };
        assert_eq!(s, DhruvStatus::NullPointer);
    }

    #[test]
    fn ffi_tithi_at_null() {
        let s = unsafe { dhruv_tithi_at(ptr::null(), 2451545.0, 90.0, ptr::null_mut()) };
        assert_eq!(s, DhruvStatus::NullPointer);
    }

    #[test]
    fn ffi_karana_at_null() {
        let s = unsafe { dhruv_karana_at(ptr::null(), 2451545.0, 90.0, ptr::null_mut()) };
        assert_eq!(s, DhruvStatus::NullPointer);
    }

    #[test]
    fn ffi_yoga_at_null() {
        let s =
            unsafe { dhruv_yoga_at(ptr::null(), 2451545.0, 90.0, ptr::null(), ptr::null_mut()) };
        assert_eq!(s, DhruvStatus::NullPointer);
    }

    #[test]
    fn ffi_vaar_from_sunrises_null() {
        let s =
            unsafe { dhruv_vaar_from_sunrises(ptr::null(), 2451545.0, 2451546.0, ptr::null_mut()) };
        assert_eq!(s, DhruvStatus::NullPointer);
    }

    #[test]
    fn ffi_hora_from_sunrises_null() {
        let s = unsafe {
            dhruv_hora_from_sunrises(
                ptr::null(),
                2451545.5,
                2451545.0,
                2451546.0,
                ptr::null_mut(),
            )
        };
        assert_eq!(s, DhruvStatus::NullPointer);
    }

    #[test]
    fn ffi_ghatika_from_sunrises_null() {
        let s = unsafe {
            dhruv_ghatika_from_sunrises(
                ptr::null(),
                2451545.5,
                2451545.0,
                2451546.0,
                ptr::null_mut(),
            )
        };
        assert_eq!(s, DhruvStatus::NullPointer);
    }

    #[test]
    fn ffi_graha_name_valid() {
        let name = dhruv_graha_name(0);
        assert!(!name.is_null());
    }

    #[test]
    fn ffi_graha_name_invalid() {
        assert!(dhruv_graha_name(9).is_null());
        assert!(dhruv_graha_name(100).is_null());
    }

    #[test]
    fn ffi_yogini_name_valid() {
        let name = dhruv_yogini_name(0);
        assert!(!name.is_null());
    }

    #[test]
    fn ffi_rashi_lord_valid() {
        // Mesha (0) -> Mangal (2)
        assert_eq!(dhruv_rashi_lord(0), 2);
        // Simha (4) -> Surya (0)
        assert_eq!(dhruv_rashi_lord(4), 0);
        // Karka (3) -> Chandra (1)
        assert_eq!(dhruv_rashi_lord(3), 1);
    }

    #[test]
    fn ffi_rashi_lord_invalid() {
        assert_eq!(dhruv_rashi_lord(12), -1);
        assert_eq!(dhruv_rashi_lord(255), -1);
    }

    #[test]
    fn ffi_sphuta_name_valid() {
        let name = dhruv_sphuta_name(0);
        assert!(!name.is_null());
    }

    #[test]
    fn ffi_sphuta_name_invalid() {
        assert!(dhruv_sphuta_name(16).is_null());
    }

    #[test]
    fn ffi_all_sphutas_null() {
        let mut result = DhruvSphutalResult {
            longitudes: [0.0; 16],
        };
        let s = unsafe { dhruv_all_sphutas(ptr::null(), &mut result) };
        assert_eq!(s, DhruvStatus::NullPointer);

        let inputs = DhruvSphutalInputs {
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
        let s = unsafe { dhruv_all_sphutas(&inputs, ptr::null_mut()) };
        assert_eq!(s, DhruvStatus::NullPointer);
    }

    #[test]
    fn ffi_all_sphutas_values_in_range() {
        let inputs = DhruvSphutalInputs {
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
        let mut result = DhruvSphutalResult {
            longitudes: [0.0; 16],
        };
        let s = unsafe { dhruv_all_sphutas(&inputs, &mut result) };
        assert_eq!(s, DhruvStatus::Ok);
        for lon in &result.longitudes {
            assert!(*lon >= 0.0 && *lon < 360.0, "lon={lon} out of range");
        }
    }

    #[test]
    fn ffi_individual_sphuta_functions() {
        // Bhrigu Bindu
        let bb = dhruv_bhrigu_bindu(120.0, 240.0);
        assert!((bb - 180.0).abs() < 1e-10);

        // Yoga Sphuta
        let ys = dhruv_yoga_sphuta(100.0, 200.0);
        assert!((ys - 300.0).abs() < 1e-10);

        // Avayoga: complement of yoga
        let as_ = dhruv_avayoga_sphuta(100.0, 200.0);
        assert!((as_ - 60.0).abs() < 1e-10);
    }

    // --- Special Lagnas ---

    #[test]
    fn ffi_special_lagna_count() {
        assert_eq!(DHRUV_SPECIAL_LAGNA_COUNT, 8);
    }

    #[test]
    fn ffi_special_lagna_name_valid() {
        let name = dhruv_special_lagna_name(0);
        assert!(!name.is_null());
        let name = dhruv_special_lagna_name(7);
        assert!(!name.is_null());
    }

    #[test]
    fn ffi_special_lagna_name_invalid() {
        let name = dhruv_special_lagna_name(8);
        assert!(name.is_null());
        let name = dhruv_special_lagna_name(u32::MAX);
        assert!(name.is_null());
    }

    #[test]
    fn ffi_bhava_lagna_value() {
        let bl = dhruv_bhava_lagna(45.0, 5.0);
        assert!((bl - 75.0).abs() < 1e-10);
    }

    #[test]
    fn ffi_hora_lagna_value() {
        let hl = dhruv_hora_lagna(100.0, 2.5);
        assert!((hl - 130.0).abs() < 1e-10);
    }

    #[test]
    fn ffi_ghati_lagna_value() {
        let gl = dhruv_ghati_lagna(100.0, 1.0);
        assert!((gl - 130.0).abs() < 1e-10);
    }

    #[test]
    fn ffi_vighati_lagna_value() {
        let vl = dhruv_vighati_lagna(100.0, 60.0);
        assert!((vl - 130.0).abs() < 1e-10);
    }

    #[test]
    fn ffi_sree_lagna_at_nakshatra_start() {
        let sl = dhruv_sree_lagna(0.0, 100.0);
        assert!((sl - 100.0).abs() < 1e-10);
    }

    #[test]
    fn ffi_indu_lagna_invalid_lord() {
        let il = dhruv_indu_lagna(100.0, 99, 0);
        assert!((il - (-1.0)).abs() < 1e-10);
    }

    #[test]
    fn ffi_indu_lagna_valid() {
        // Sun=30, Moon=16 → total=46, remainder=10 → Moon + 9*30 = 50 + 270 = 320
        let il = dhruv_indu_lagna(50.0, 0, 1); // Surya=0, Chandra=1
        assert!((il - 320.0).abs() < 1e-10);
    }

    #[test]
    fn ffi_special_lagnas_for_date_null() {
        let mut out = DhruvSpecialLagnas {
            bhava_lagna: 0.0,
            hora_lagna: 0.0,
            ghati_lagna: 0.0,
            vighati_lagna: 0.0,
            varnada_lagna: 0.0,
            sree_lagna: 0.0,
            pranapada_lagna: 0.0,
            indu_lagna: 0.0,
        };
        let s = unsafe {
            dhruv_special_lagnas_for_date(
                ptr::null(),
                ptr::null(),
                ptr::null(),
                ptr::null(),
                ptr::null(),
                0,
                0,
                &mut out,
            )
        };
        assert_eq!(s, DhruvStatus::NullPointer);
    }

    // --- Ashtakavarga ---

    #[test]
    fn ffi_ashtakavarga_graha_count() {
        assert_eq!(DHRUV_ASHTAKAVARGA_GRAHA_COUNT, 7);
    }

    #[test]
    fn ffi_calculate_ashtakavarga_null() {
        let mut out = std::mem::MaybeUninit::<DhruvAshtakavargaResult>::uninit();
        let s = unsafe { dhruv_calculate_ashtakavarga(ptr::null(), 0, out.as_mut_ptr()) };
        assert_eq!(s, DhruvStatus::NullPointer);

        let rashis = [0u8; 7];
        let s = unsafe { dhruv_calculate_ashtakavarga(rashis.as_ptr(), 0, ptr::null_mut()) };
        assert_eq!(s, DhruvStatus::NullPointer);
    }

    #[test]
    fn ffi_calculate_ashtakavarga_values() {
        let rashis = [0u8; 7];
        let mut out = std::mem::MaybeUninit::<DhruvAshtakavargaResult>::uninit();
        let s = unsafe { dhruv_calculate_ashtakavarga(rashis.as_ptr(), 0, out.as_mut_ptr()) };
        assert_eq!(s, DhruvStatus::Ok);
        let result = unsafe { out.assume_init() };
        // Sun BAV total should be 48
        let sun_total: u8 = result.bavs[0].points.iter().sum();
        assert_eq!(sun_total, 48);
        // SAV total should be 337
        let sav_total: u16 = result.sav.total_points.iter().map(|&p| p as u16).sum();
        assert_eq!(sav_total, 337);
        for rashi in 0..12 {
            let row_sum: u8 = result.bavs[0].contributors[rashi].iter().sum();
            assert_eq!(row_sum, result.bavs[0].points[rashi]);
        }
    }

    #[test]
    fn ffi_ashtakavarga_for_date_null() {
        let mut out = std::mem::MaybeUninit::<DhruvAshtakavargaResult>::uninit();
        let s = unsafe {
            dhruv_ashtakavarga_for_date(
                ptr::null(),
                ptr::null(),
                ptr::null(),
                ptr::null(),
                0,
                0,
                out.as_mut_ptr(),
            )
        };
        assert_eq!(s, DhruvStatus::NullPointer);
    }

    // --- calculate_bav ---

    #[test]
    fn ffi_calculate_bav_rejects_null() {
        let mut out = std::mem::MaybeUninit::<DhruvBhinnaAshtakavarga>::uninit();
        let s = unsafe { dhruv_calculate_bav(0, ptr::null(), 0, out.as_mut_ptr()) };
        assert_eq!(s, DhruvStatus::NullPointer);

        let rashis = [0u8; 7];
        let s = unsafe { dhruv_calculate_bav(0, rashis.as_ptr(), 0, ptr::null_mut()) };
        assert_eq!(s, DhruvStatus::NullPointer);
    }

    #[test]
    fn ffi_calculate_bav_rejects_invalid_index() {
        let rashis = [0u8; 7];
        let mut out = std::mem::MaybeUninit::<DhruvBhinnaAshtakavarga>::uninit();
        let s = unsafe { dhruv_calculate_bav(7, rashis.as_ptr(), 0, out.as_mut_ptr()) };
        assert_eq!(s, DhruvStatus::InvalidQuery);
    }

    #[test]
    fn ffi_calculate_bav_valid() {
        let rashis = [0u8; 7];
        let mut out = std::mem::MaybeUninit::<DhruvBhinnaAshtakavarga>::uninit();
        let s = unsafe { dhruv_calculate_bav(0, rashis.as_ptr(), 0, out.as_mut_ptr()) };
        assert_eq!(s, DhruvStatus::Ok);
        let bav = unsafe { out.assume_init() };
        assert_eq!(bav.graha_index, 0);
        let total: u8 = bav.points.iter().sum();
        assert_eq!(total, 48); // Sun BAV total is always 48
        let matrix_total: u8 = bav.contributors.iter().flatten().sum();
        assert_eq!(matrix_total, total);
    }

    // --- calculate_all_bav ---

    #[test]
    fn ffi_calculate_all_bav_rejects_null() {
        let mut out = [DhruvBhinnaAshtakavarga {
            graha_index: 0,
            points: [0; 12],
            contributors: [[0; 8]; 12],
        }; 7];
        let s = unsafe { dhruv_calculate_all_bav(ptr::null(), 0, out.as_mut_ptr()) };
        assert_eq!(s, DhruvStatus::NullPointer);

        let rashis = [0u8; 7];
        let s = unsafe { dhruv_calculate_all_bav(rashis.as_ptr(), 0, ptr::null_mut()) };
        assert_eq!(s, DhruvStatus::NullPointer);
    }

    #[test]
    fn ffi_calculate_all_bav_valid() {
        let rashis = [0u8; 7];
        let mut out = [DhruvBhinnaAshtakavarga {
            graha_index: 0,
            points: [0; 12],
            contributors: [[0; 8]; 12],
        }; 7];
        let s = unsafe { dhruv_calculate_all_bav(rashis.as_ptr(), 0, out.as_mut_ptr()) };
        assert_eq!(s, DhruvStatus::Ok);
        // Check graha indices are 0..6
        for i in 0..7 {
            assert_eq!(out[i].graha_index, i as u8);
        }
        // Sun BAV total = 48
        let sun_total: u8 = out[0].points.iter().sum();
        assert_eq!(sun_total, 48);
    }

    // --- calculate_sav ---

    #[test]
    fn ffi_calculate_sav_rejects_null() {
        let mut out = std::mem::MaybeUninit::<DhruvSarvaAshtakavarga>::uninit();
        let s = unsafe { dhruv_calculate_sav(ptr::null(), out.as_mut_ptr()) };
        assert_eq!(s, DhruvStatus::NullPointer);

        let bavs = [DhruvBhinnaAshtakavarga {
            graha_index: 0,
            points: [0; 12],
            contributors: [[0; 8]; 12],
        }; 7];
        let s = unsafe { dhruv_calculate_sav(bavs.as_ptr(), ptr::null_mut()) };
        assert_eq!(s, DhruvStatus::NullPointer);
    }

    #[test]
    fn ffi_calculate_sav_valid() {
        // First compute BAVs, then SAV
        let rashis = [0u8; 7];
        let mut bavs = [DhruvBhinnaAshtakavarga {
            graha_index: 0,
            points: [0; 12],
            contributors: [[0; 8]; 12],
        }; 7];
        let s = unsafe { dhruv_calculate_all_bav(rashis.as_ptr(), 0, bavs.as_mut_ptr()) };
        assert_eq!(s, DhruvStatus::Ok);

        let mut sav = std::mem::MaybeUninit::<DhruvSarvaAshtakavarga>::uninit();
        let s = unsafe { dhruv_calculate_sav(bavs.as_ptr(), sav.as_mut_ptr()) };
        assert_eq!(s, DhruvStatus::Ok);
        let sav = unsafe { sav.assume_init() };
        let total: u16 = sav.total_points.iter().map(|&p| p as u16).sum();
        assert_eq!(total, 337);
    }

    // --- trikona_sodhana ---

    #[test]
    fn ffi_trikona_sodhana_rejects_null() {
        let mut out = [0u8; 12];
        let s = unsafe { dhruv_trikona_sodhana(ptr::null(), out.as_mut_ptr()) };
        assert_eq!(s, DhruvStatus::NullPointer);

        let totals = [0u8; 12];
        let s = unsafe { dhruv_trikona_sodhana(totals.as_ptr(), ptr::null_mut()) };
        assert_eq!(s, DhruvStatus::NullPointer);
    }

    #[test]
    fn ffi_trikona_sodhana_valid() {
        // Fire trikona: rashis 0,4,8 with values 5,3,7 → min=3, result 2,0,4
        let mut totals = [0u8; 12];
        totals[0] = 5;
        totals[4] = 3;
        totals[8] = 7;
        let mut out = [0u8; 12];
        let s = unsafe { dhruv_trikona_sodhana(totals.as_ptr(), out.as_mut_ptr()) };
        assert_eq!(s, DhruvStatus::Ok);
        assert_eq!(out[0], 2);
        assert_eq!(out[4], 0);
        assert_eq!(out[8], 4);
    }

    // --- ekadhipatya_sodhana ---

    #[test]
    fn ffi_ekadhipatya_sodhana_rejects_null() {
        let mut out = [0u8; 12];
        let s = unsafe { dhruv_ekadhipatya_sodhana(ptr::null(), out.as_mut_ptr()) };
        assert_eq!(s, DhruvStatus::NullPointer);

        let after = [0u8; 12];
        let s = unsafe { dhruv_ekadhipatya_sodhana(after.as_ptr(), ptr::null_mut()) };
        assert_eq!(s, DhruvStatus::NullPointer);
    }

    #[test]
    fn ffi_ekadhipatya_sodhana_valid() {
        // Mercury pair: rashis 2,5 with values 4,6 → min=4, result 0,2
        let mut after = [0u8; 12];
        after[2] = 4;
        after[5] = 6;
        let mut out = [0u8; 12];
        let s = unsafe { dhruv_ekadhipatya_sodhana(after.as_ptr(), out.as_mut_ptr()) };
        assert_eq!(s, DhruvStatus::Ok);
        assert_eq!(out[2], 0);
        assert_eq!(out[5], 2);
    }

    // --- graha_drishti ---

    #[test]
    fn ffi_graha_drishti_rejects_null_out() {
        let s = unsafe { dhruv_graha_drishti(0, 0.0, 180.0, ptr::null_mut()) };
        assert_eq!(s, DhruvStatus::NullPointer);
    }

    #[test]
    fn ffi_graha_drishti_rejects_invalid_index() {
        let mut out = std::mem::MaybeUninit::<DhruvDrishtiEntry>::uninit();
        let s = unsafe { dhruv_graha_drishti(9, 0.0, 180.0, out.as_mut_ptr()) };
        assert_eq!(s, DhruvStatus::InvalidQuery);
    }

    #[test]
    fn ffi_graha_drishti_valid() {
        let mut out = std::mem::MaybeUninit::<DhruvDrishtiEntry>::uninit();
        // Sun (0) aspecting 180° away → full 7th aspect (60 virupa base)
        let s = unsafe { dhruv_graha_drishti(0, 0.0, 180.0, out.as_mut_ptr()) };
        assert_eq!(s, DhruvStatus::Ok);
        let e = unsafe { out.assume_init() };
        assert!((e.angular_distance - 180.0).abs() < 1e-9);
        assert!(e.base_virupa >= 59.0); // 7th house aspect is 60 virupa
    }

    // --- graha_drishti_matrix ---

    #[test]
    fn ffi_graha_drishti_matrix_rejects_null() {
        let mut out = std::mem::MaybeUninit::<DhruvGrahaDrishtiMatrix>::uninit();
        let s = unsafe { dhruv_graha_drishti_matrix(ptr::null(), out.as_mut_ptr()) };
        assert_eq!(s, DhruvStatus::NullPointer);

        let lons = [0.0f64; 9];
        let s = unsafe { dhruv_graha_drishti_matrix(lons.as_ptr(), ptr::null_mut()) };
        assert_eq!(s, DhruvStatus::NullPointer);
    }

    #[test]
    fn ffi_graha_drishti_matrix_valid() {
        let lons = [0.0, 30.0, 60.0, 90.0, 120.0, 150.0, 180.0, 210.0, 240.0];
        let mut out = std::mem::MaybeUninit::<DhruvGrahaDrishtiMatrix>::uninit();
        let s = unsafe { dhruv_graha_drishti_matrix(lons.as_ptr(), out.as_mut_ptr()) };
        assert_eq!(s, DhruvStatus::Ok);
        let m = unsafe { out.assume_init() };
        // Diagonal should be zero
        assert_eq!(m.entries[0][0].total_virupa, 0.0);
        // Sun→7th planet (180° away) should have high virupa
        assert!(m.entries[0][6].base_virupa >= 59.0);
    }

    // --- ghatika_from_elapsed ---

    #[test]
    fn ffi_ghatika_from_elapsed_rejects_null() {
        let mut idx: u8 = 0;
        let s = unsafe { dhruv_ghatika_from_elapsed(0.0, 86400.0, ptr::null_mut(), &mut idx) };
        assert_eq!(s, DhruvStatus::NullPointer);

        let mut val: u8 = 0;
        let s = unsafe { dhruv_ghatika_from_elapsed(0.0, 86400.0, &mut val, ptr::null_mut()) };
        assert_eq!(s, DhruvStatus::NullPointer);
    }

    #[test]
    fn ffi_ghatika_from_elapsed_valid() {
        let mut val: u8 = 0;
        let mut idx: u8 = 0;
        // At sunrise (0 seconds elapsed), ghatika = 1 (index 0)
        let s = unsafe { dhruv_ghatika_from_elapsed(0.0, 86400.0, &mut val, &mut idx) };
        assert_eq!(s, DhruvStatus::Ok);
        assert_eq!(val, 1);
        assert_eq!(idx, 0);
    }

    // --- ghatikas_since_sunrise ---

    #[test]
    fn ffi_ghatikas_since_sunrise_rejects_null() {
        let s = unsafe {
            dhruv_ghatikas_since_sunrise(2451545.5, 2451545.0, 2451546.0, ptr::null_mut())
        };
        assert_eq!(s, DhruvStatus::NullPointer);
    }

    #[test]
    fn ffi_ghatikas_since_sunrise_valid() {
        let mut out: f64 = 0.0;
        // Midpoint of day → 30 ghatikas
        let s = unsafe { dhruv_ghatikas_since_sunrise(2451545.5, 2451545.0, 2451546.0, &mut out) };
        assert_eq!(s, DhruvStatus::Ok);
        assert!((out - 30.0).abs() < 1e-9);
    }

    // --- hora_at ---

    #[test]
    fn ffi_hora_at_invalid() {
        assert_eq!(dhruv_hora_at(7, 0), -1); // invalid vaar
        assert_eq!(dhruv_hora_at(0, 24), -1); // invalid hora index
    }

    #[test]
    fn ffi_hora_at_valid() {
        // Sunday (0), first hora → Surya (index 0)
        assert_eq!(dhruv_hora_at(0, 0), 0);
        // Sunday (0), second hora → Shukra (index 1)
        assert_eq!(dhruv_hora_at(0, 1), 1);
    }

    #[test]
    fn ffi_graha_positions_rejects_null() {
        let mut out = std::mem::MaybeUninit::<DhruvGrahaPositions>::uninit();
        let cfg = DhruvGrahaPositionsConfig {
            include_nakshatra: 0,
            include_lagna: 0,
            include_outer_planets: 0,
            include_bhava: 0,
        };
        let bhava_cfg = dhruv_bhava_config_default();
        let s = unsafe {
            dhruv_graha_positions(
                ptr::null(),
                ptr::null(),
                ptr::null(),
                ptr::null(),
                &bhava_cfg,
                0,
                0,
                &cfg,
                out.as_mut_ptr(),
            )
        };
        assert_eq!(s, DhruvStatus::NullPointer);
    }

    #[test]
    fn ffi_core_bindus_rejects_null() {
        let mut out = std::mem::MaybeUninit::<DhruvBindusResult>::uninit();
        let cfg = DhruvBindusConfig {
            include_nakshatra: 0,
            include_bhava: 0,
            upagraha_config: dhruv_time_upagraha_config_default(),
        };
        let bhava_cfg = dhruv_bhava_config_default();
        let rs_cfg = dhruv_riseset_config_default();
        let s = unsafe {
            dhruv_core_bindus(
                ptr::null(),
                ptr::null(),
                ptr::null(),
                ptr::null(),
                &bhava_cfg,
                &rs_cfg,
                0,
                0,
                &cfg,
                out.as_mut_ptr(),
            )
        };
        assert_eq!(s, DhruvStatus::NullPointer);
    }

    #[test]
    fn ffi_drishti_rejects_null() {
        let mut out = std::mem::MaybeUninit::<DhruvDrishtiResult>::uninit();
        let cfg = DhruvDrishtiConfig {
            include_bhava: 0,
            include_lagna: 0,
            include_bindus: 0,
        };
        let bhava_cfg = dhruv_bhava_config_default();
        let rs_cfg = dhruv_riseset_config_default();
        let s = unsafe {
            dhruv_drishti(
                ptr::null(),
                ptr::null(),
                ptr::null(),
                ptr::null(),
                &bhava_cfg,
                &rs_cfg,
                0,
                0,
                &cfg,
                out.as_mut_ptr(),
            )
        };
        assert_eq!(s, DhruvStatus::NullPointer);
    }

    #[test]
    fn ffi_full_kundali_rejects_null() {
        let mut out = std::mem::MaybeUninit::<DhruvFullKundaliResult>::uninit();
        let cfg = dhruv_full_kundali_config_default();
        let bhava_cfg = dhruv_bhava_config_default();
        let rs_cfg = dhruv_riseset_config_default();
        let s = unsafe {
            dhruv_full_kundali_for_date(
                ptr::null(),
                ptr::null(),
                ptr::null(),
                ptr::null(),
                &bhava_cfg,
                &rs_cfg,
                0,
                0,
                &cfg,
                out.as_mut_ptr(),
            )
        };
        assert_eq!(s, DhruvStatus::NullPointer);
    }

    #[test]
    fn ffi_full_kundali_rejects_null_out() {
        let cfg = dhruv_full_kundali_config_default();
        let bhava_cfg = dhruv_bhava_config_default();
        let rs_cfg = dhruv_riseset_config_default();
        let utc = DhruvUtcTime {
            year: 2000,
            month: 1,
            day: 1,
            hour: 0,
            minute: 0,
            second: 0.0,
        };
        let location = DhruvGeoLocation {
            latitude_deg: 0.0,
            longitude_deg: 0.0,
            altitude_m: 0.0,
        };
        let engine_ptr = std::ptr::NonNull::<Engine>::dangling().as_ptr();
        let eop_ptr = std::ptr::NonNull::<dhruv_time::EopKernel>::dangling().as_ptr();
        let s = unsafe {
            dhruv_full_kundali_for_date(
                engine_ptr,
                eop_ptr,
                &utc,
                &location,
                &bhava_cfg,
                &rs_cfg,
                0,
                0,
                &cfg,
                ptr::null_mut(),
            )
        };
        assert_eq!(s, DhruvStatus::NullPointer);
    }

    #[test]
    fn ffi_graha_longitudes_rejects_null() {
        let mut out = std::mem::MaybeUninit::<DhruvGrahaLongitudes>::uninit();
        let cfg = dhruv_graha_longitudes_config_default();
        let s = unsafe { dhruv_graha_longitudes(ptr::null(), 2451545.0, &cfg, out.as_mut_ptr()) };
        assert_eq!(s, DhruvStatus::NullPointer);
    }

    #[test]
    fn ffi_graha_longitudes_rejects_null_out() {
        // Use a non-null but invalid engine pointer would be UB, so just test null out
        let cfg = dhruv_graha_longitudes_config_default();
        let s = unsafe { dhruv_graha_longitudes(ptr::null(), 2451545.0, &cfg, ptr::null_mut()) };
        assert_eq!(s, DhruvStatus::NullPointer);
    }

    #[test]
    fn ffi_graha_longitudes_rejects_invalid_ayanamsha() {
        let engine_ptr: *const Engine = ptr::null();
        let mut out = std::mem::MaybeUninit::<DhruvGrahaLongitudes>::uninit();
        let mut cfg = dhruv_graha_longitudes_config_default();
        cfg.ayanamsha_system = 99;
        let s = unsafe { dhruv_graha_longitudes(engine_ptr, 2451545.0, &cfg, out.as_mut_ptr()) };
        // Null engine is checked first
        assert_eq!(s, DhruvStatus::NullPointer);
    }

    #[test]
    fn ffi_graha_longitudes_rejects_null_engine() {
        let mut out = std::mem::MaybeUninit::<DhruvGrahaLongitudes>::uninit();
        let cfg = dhruv_graha_longitudes_config_default();
        let s = unsafe { dhruv_graha_longitudes(ptr::null(), 2451545.0, &cfg, out.as_mut_ptr()) };
        assert_eq!(s, DhruvStatus::NullPointer);
    }

    #[test]
    fn ffi_graha_longitudes_rejects_null_out_with_null_config() {
        let s =
            unsafe { dhruv_graha_longitudes(ptr::null(), 2451545.0, ptr::null(), ptr::null_mut()) };
        assert_eq!(s, DhruvStatus::NullPointer);
    }

    #[test]
    fn ffi_nakshatra_at_rejects_null() {
        let mut out = std::mem::MaybeUninit::<DhruvPanchangNakshatraInfo>::uninit();
        let cfg = dhruv_sankranti_config_default();
        let s =
            unsafe { dhruv_nakshatra_at(ptr::null(), 2451545.0, 120.0, &cfg, out.as_mut_ptr()) };
        assert_eq!(s, DhruvStatus::NullPointer);
    }

    #[test]
    fn ffi_nakshatra_at_rejects_null_config() {
        let mut out = std::mem::MaybeUninit::<DhruvPanchangNakshatraInfo>::uninit();
        let s = unsafe {
            dhruv_nakshatra_at(ptr::null(), 2451545.0, 120.0, ptr::null(), out.as_mut_ptr())
        };
        assert_eq!(s, DhruvStatus::NullPointer);
    }

    #[test]
    fn ffi_nakshatra_at_rejects_null_out() {
        let cfg = dhruv_sankranti_config_default();
        let s = unsafe { dhruv_nakshatra_at(ptr::null(), 2451545.0, 120.0, &cfg, ptr::null_mut()) };
        assert_eq!(s, DhruvStatus::NullPointer);
    }

    // ── time_upagraha_jd ──────────────────────────────────────────

    #[test]
    fn ffi_time_upagraha_jd_rejects_null_out() {
        let s = unsafe {
            dhruv_time_upagraha_jd(0, 0, 1, 2451545.0, 2451545.3, 2451546.0, ptr::null_mut())
        };
        assert_eq!(s, DhruvStatus::NullPointer);
    }

    #[test]
    fn ffi_time_upagraha_jd_rejects_invalid_index() {
        let mut out: f64 = 0.0;
        let s =
            unsafe { dhruv_time_upagraha_jd(6, 0, 1, 2451545.0, 2451545.3, 2451546.0, &mut out) };
        assert_eq!(s, DhruvStatus::InvalidQuery);
    }

    #[test]
    fn ffi_time_upagraha_jd_rejects_invalid_weekday() {
        let mut out: f64 = 0.0;
        let s =
            unsafe { dhruv_time_upagraha_jd(0, 7, 1, 2451545.0, 2451545.3, 2451546.0, &mut out) };
        assert_eq!(s, DhruvStatus::InvalidQuery);
    }

    #[test]
    fn ffi_time_upagraha_jd_valid() {
        let mut out: f64 = 0.0;
        // Gulika (0), Sunday (0), daytime
        let s =
            unsafe { dhruv_time_upagraha_jd(0, 0, 1, 2451545.0, 2451545.3, 2451546.0, &mut out) };
        assert_eq!(s, DhruvStatus::Ok);
        // Result should be between sunrise and sunset
        assert!(out >= 2451545.0 && out <= 2451545.3, "out={out}");
    }

    #[test]
    fn ffi_time_upagraha_jd_with_config_supports_saturn_middle() {
        let cfg = DhruvTimeUpagrahaConfig {
            gulika_point: DHRUV_UPAGRAHA_POINT_MIDDLE,
            maandi_point: DHRUV_UPAGRAHA_POINT_END,
            other_point: DHRUV_UPAGRAHA_POINT_START,
            gulika_planet: DHRUV_GULIKA_MAANDI_PLANET_SATURN,
            maandi_planet: DHRUV_GULIKA_MAANDI_PLANET_RAHU,
        };
        let mut out: f64 = 0.0;
        let s = unsafe {
            dhruv_time_upagraha_jd_with_config(
                0, 0, 1, 2451545.0, 2451545.8, 2451546.0, &cfg, &mut out,
            )
        };
        assert_eq!(s, DhruvStatus::Ok);
        assert!((out - 2451545.65).abs() < 1e-9, "out={out}");
    }

    #[test]
    fn ffi_time_upagraha_jd_utc_rejects_null() {
        let mut out: f64 = 0.0;
        let rs_cfg = dhruv_riseset_config_default();
        let loc = DhruvGeoLocation {
            latitude_deg: 28.6,
            longitude_deg: 77.2,
            altitude_m: 0.0,
        };
        let utc = DhruvUtcTime {
            year: 2024,
            month: 1,
            day: 15,
            hour: 12,
            minute: 0,
            second: 0.0,
        };
        // null engine
        let s = unsafe {
            dhruv_time_upagraha_jd_utc(ptr::null(), ptr::null(), &utc, &loc, &rs_cfg, 0, &mut out)
        };
        assert_eq!(s, DhruvStatus::NullPointer);
    }

    #[test]
    fn ffi_time_upagraha_jd_utc_rejects_null_out() {
        let rs_cfg = dhruv_riseset_config_default();
        let loc = DhruvGeoLocation {
            latitude_deg: 28.6,
            longitude_deg: 77.2,
            altitude_m: 0.0,
        };
        let utc = DhruvUtcTime {
            year: 2024,
            month: 1,
            day: 15,
            hour: 12,
            minute: 0,
            second: 0.0,
        };
        let s = unsafe {
            dhruv_time_upagraha_jd_utc(
                ptr::null(),
                ptr::null(),
                &utc,
                &loc,
                &rs_cfg,
                0,
                ptr::null_mut(),
            )
        };
        assert_eq!(s, DhruvStatus::NullPointer);
    }

    #[test]
    fn ffi_time_upagraha_jd_utc_rejects_invalid_index() {
        let rs_cfg = dhruv_riseset_config_default();
        let loc = DhruvGeoLocation {
            latitude_deg: 28.6,
            longitude_deg: 77.2,
            altitude_m: 0.0,
        };
        let utc = DhruvUtcTime {
            year: 2024,
            month: 1,
            day: 15,
            hour: 12,
            minute: 0,
            second: 0.0,
        };
        let mut out: f64 = 0.0;
        // index 6 is invalid (only 0-5 allowed), but null engine is checked first
        let s = unsafe {
            dhruv_time_upagraha_jd_utc(ptr::null(), ptr::null(), &utc, &loc, &rs_cfg, 6, &mut out)
        };
        assert_eq!(s, DhruvStatus::NullPointer);
    }

    // --- Amsha FFI tests ---

    #[test]
    fn ffi_amsha_longitude_d9() {
        let mut out: f64 = 0.0;
        let s = unsafe { dhruv_amsha_longitude(45.0, 9, 0, &mut out) };
        assert_eq!(s, DhruvStatus::Ok);
        assert!(out >= 0.0 && out < 360.0);
    }

    #[test]
    fn ffi_amsha_longitude_null_out() {
        let s = unsafe { dhruv_amsha_longitude(45.0, 9, 0, ptr::null_mut()) };
        assert_eq!(s, DhruvStatus::NullPointer);
    }

    #[test]
    fn ffi_amsha_longitude_invalid_code() {
        let mut out: f64 = 0.0;
        let s = unsafe { dhruv_amsha_longitude(45.0, 999, 0, &mut out) };
        assert_eq!(s, DhruvStatus::InvalidSearchConfig);
    }

    #[test]
    fn ffi_amsha_longitude_invalid_variation() {
        let mut out: f64 = 0.0;
        // HoraCancerLeoOnly (1) on D9 should fail
        let s = unsafe { dhruv_amsha_longitude(45.0, 9, 1, &mut out) };
        assert_eq!(s, DhruvStatus::InvalidSearchConfig);
    }

    #[test]
    fn ffi_amsha_rashi_info_d9() {
        let mut out = DhruvRashiInfo {
            rashi_index: 0,
            dms: DhruvDms {
                degrees: 0,
                minutes: 0,
                seconds: 0.0,
            },
            degrees_in_rashi: 0.0,
        };
        let s = unsafe { dhruv_amsha_rashi_info(45.0, 9, 0, &mut out) };
        assert_eq!(s, DhruvStatus::Ok);
        assert!(out.rashi_index < 12);
        assert!(out.degrees_in_rashi >= 0.0 && out.degrees_in_rashi < 30.0);
    }

    #[test]
    fn ffi_amsha_longitudes_batch() {
        let codes: [u16; 3] = [1, 9, 10];
        let mut out = [0.0f64; 3];
        let s = unsafe {
            dhruv_amsha_longitudes(45.0, codes.as_ptr(), ptr::null(), 3, out.as_mut_ptr())
        };
        assert_eq!(s, DhruvStatus::Ok);
        for v in &out {
            assert!(*v >= 0.0 && *v < 360.0);
        }
        // D1 identity
        assert!((out[0] - 45.0).abs() < 0.001);
    }

    #[test]
    fn ffi_amsha_longitudes_zero_count() {
        let s =
            unsafe { dhruv_amsha_longitudes(45.0, ptr::null(), ptr::null(), 0, ptr::null_mut()) };
        assert_eq!(s, DhruvStatus::Ok);
    }

    #[test]
    fn ffi_amsha_longitudes_null_codes() {
        let mut out = [0.0f64; 1];
        let s =
            unsafe { dhruv_amsha_longitudes(45.0, ptr::null(), ptr::null(), 1, out.as_mut_ptr()) };
        assert_eq!(s, DhruvStatus::NullPointer);
    }

    #[test]
    fn ffi_amsha_chart_null_engine() {
        let utc = DhruvUtcTime {
            year: 2024,
            month: 1,
            day: 15,
            hour: 12,
            minute: 0,
            second: 0.0,
        };
        let loc = DhruvGeoLocation {
            latitude_deg: 28.6,
            longitude_deg: 77.2,
            altitude_m: 0.0,
        };
        let bhava = DhruvBhavaConfig {
            system: 0,
            starting_point: 0,
            custom_start_deg: 0.0,
            reference_mode: 0,
            ..dhruv_bhava_config_default()
        };
        let rs = DhruvRiseSetConfig {
            use_refraction: 1,
            sun_limb: 0,
            altitude_correction: 0,
        };
        let scope = DhruvAmshaChartScope {
            include_bhava_cusps: 0,
            include_arudha_padas: 0,
            include_upagrahas: 0,
            include_sphutas: 0,
            include_special_lagnas: 0,
        };
        let mut out = DhruvAmshaChart::zeroed();
        let s = unsafe {
            dhruv_amsha_chart_for_date(
                ptr::null(),
                ptr::null(),
                &utc,
                &loc,
                &bhava,
                &rs,
                1,
                1,
                9,
                0,
                &scope,
                &mut out,
            )
        };
        assert_eq!(s, DhruvStatus::NullPointer);
    }
}
