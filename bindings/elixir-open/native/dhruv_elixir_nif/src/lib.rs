use std::fmt::Debug;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

use dhruv_config::{ConfigResolver, DefaultsMode, load_from_path};
use dhruv_core::{Body, Engine, EngineConfig, Frame, Observer, Query, StateVector};
use dhruv_frames::{
    ReferencePlane, cartesian_state_to_spherical_state, cartesian_to_spherical, nutation_iau2000b,
};
use dhruv_search::ConjunctionConfig;
use dhruv_search::operations::{
    AyanamshaMode, AyanamshaOperation, ConjunctionOperation, ConjunctionQuery, ConjunctionResult,
    GrahanKind, GrahanOperation, GrahanQuery, GrahanResult, LunarPhaseKind, LunarPhaseOperation,
    LunarPhaseQuery, LunarPhaseResult, MotionKind, MotionOperation, MotionQuery, MotionResult,
    NodeBackend, NodeOperation, PanchangOperation, PanchangResult, SankrantiOperation,
    SankrantiQuery, SankrantiResult, SankrantiTarget, TaraOperation, TaraOutputKind, TaraResult,
};
use dhruv_search::{
    PANCHANG_INCLUDE_ALL, PANCHANG_INCLUDE_AYANA, PANCHANG_INCLUDE_MASA, PANCHANG_INCLUDE_VARSHA,
    SankrantiConfig, StationaryConfig, ayanamsha, conjunction, dasha_hierarchy_for_birth,
    dasha_snapshot_at, full_kundali_for_date, graha_sidereal_longitudes, lunar_node, motion,
    panchang, set_time_conversion_policy, tara as tara_op,
};
use dhruv_search::{
    all_upagrahas_for_date, all_upagrahas_for_date_with_config, amsha_charts_for_date,
    arudha_padas_for_date, ashtakavarga_for_date, avastha_for_date, balas_for_date,
    bhavabala_for_date, charakaraka_for_date, core_bindus, drishti_for_date,
    graha_positions as graha_positions_fn, shadbala_for_date, sidereal_bhavas_for_date,
    sidereal_lagna_for_date, sidereal_mc_for_date, special_lagnas_for_date, vimsopaka_for_date,
};
use dhruv_tara::{TaraAccuracy, TaraCatalog, TaraConfig, TaraId};
use dhruv_time::{
    DeltaTModel, EopKernel, FutureDeltaTTransition, SmhFutureParabolaFamily, TimeConversionOptions,
    TimeConversionPolicy, TimeDiagnostics, TimeWarning, TtUtcSource, UtcTime, jd_to_tdb_seconds,
    tdb_seconds_to_jd,
};
use dhruv_vedic_base::bhava_types::ALL_BHAVA_SYSTEMS;
use dhruv_vedic_base::dasha::{
    ALL_DASHA_SYSTEMS, DashaEntity, DashaHierarchy, DashaLevel, DashaPeriod, DashaSnapshot,
    DashaSystem, DashaVariationConfig, SubPeriodMethod, YoginiScheme,
};
use dhruv_vedic_base::riseset::{approximate_local_noon_jd, compute_all_events, compute_rise_set};
use dhruv_vedic_base::riseset_types::{
    GeoLocation, RiseSetConfig, RiseSetEvent, RiseSetResult, SunLimb,
};
use dhruv_vedic_base::{
    ALL_GRAHAS, AyanamshaSystem, BhavaConfig, BhavaReferenceMode, BhavaResult, BhavaStartingPoint,
    BhavaSystem, CharakarakaResult, CharakarakaScheme, Graha, GulikaMaandiPlanet, LunarNode,
    NodeDignityPolicy, NodeMode, TimeUpagrahaConfig, TimeUpagrahaPoint, compute_bhavas,
    lagna_longitude_rad, mc_longitude_rad, ramc_rad,
};
use rustler::{Encoder, Env, ResourceArc, Term};
use serde::Deserialize;
use serde_json::{Value, json};

mod atoms {
    rustler::atoms! {
        ok,
        error
    }
}

type JsonResult = Result<Value, Value>;

#[derive(Debug)]
struct EngineResource {
    state: RwLock<EngineState>,
}

impl rustler::Resource for EngineResource {}

#[derive(Debug)]
struct EngineState {
    engine: Option<Engine>,
    resolver: Option<ConfigResolver>,
    eop: Option<EopKernel>,
    time_policy: TimeConversionPolicy,
    tara_catalog: Arc<TaraCatalog>,
}

impl EngineState {
    fn new(engine: Engine, time_policy: TimeConversionPolicy) -> Self {
        Self {
            engine: Some(engine),
            resolver: None,
            eop: None,
            time_policy,
            tara_catalog: Arc::new(TaraCatalog::embedded().clone()),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
enum EnumInput {
    Int(i64),
    Str(String),
}

#[derive(Debug, Clone, Deserialize)]
struct EngineConfigInput {
    spk_paths: Vec<String>,
    lsk_path: String,
    cache_capacity: Option<usize>,
    strict_validation: Option<bool>,
    time_policy: Option<TimePolicyInput>,
}

#[derive(Debug, Clone, Deserialize)]
struct PathInput {
    path: String,
}

#[derive(Debug, Clone, Copy, Deserialize)]
struct UtcInput {
    year: i32,
    month: u32,
    day: u32,
    hour: u32,
    minute: u32,
    second: f64,
}

#[derive(Debug, Clone, Copy, Deserialize)]
struct GeoLocationInput {
    latitude_deg: f64,
    longitude_deg: f64,
    altitude_m: Option<f64>,
}

#[derive(Debug, Clone, Deserialize)]
struct QueryInput {
    target: EnumInput,
    observer: EnumInput,
    frame: Option<EnumInput>,
    epoch_tdb_jd: Option<f64>,
    utc: Option<UtcInput>,
    output: Option<EnumInput>,
}

#[derive(Debug, Clone, Deserialize)]
struct BodyLonLatInput {
    body: EnumInput,
    jd_tdb: f64,
}

#[derive(Debug, Clone, Deserialize)]
struct CartesianInput {
    x: f64,
    y: f64,
    z: f64,
}

#[derive(Debug, Clone, Deserialize)]
struct TimeRunInput {
    op: String,
    utc: Option<UtcInput>,
    jd_tdb: Option<f64>,
    time_policy: Option<TimePolicyInput>,
}

#[derive(Debug, Clone, Deserialize)]
struct TimePolicyInput {
    mode: EnumInput,
    options: Option<TimePolicyOptionsInput>,
}

#[derive(Debug, Clone, Deserialize)]
struct TimePolicyOptionsInput {
    warn_on_fallback: Option<bool>,
    delta_t_model: Option<EnumInput>,
    freeze_future_dut1: Option<bool>,
    pre_range_dut1: Option<f64>,
    future_delta_t_transition: Option<EnumInput>,
    future_transition_years: Option<f64>,
    smh_future_family: Option<EnumInput>,
}

#[derive(Debug, Clone, Deserialize)]
struct VedicRequest {
    op: String,
    jd_tdb: Option<f64>,
    utc: Option<UtcInput>,
    location: Option<GeoLocationInput>,
    event: Option<EnumInput>,
    system: Option<EnumInput>,
    mode: Option<EnumInput>,
    backend: Option<EnumInput>,
    config: Option<RiseSetConfigInput>,
    bhava_config: Option<BhavaConfigInput>,
    sankranti_config: Option<SankrantiConfigInput>,
}

#[derive(Debug, Clone, Deserialize)]
struct PanchangRequest {
    op: String,
    utc: UtcInput,
    location: Option<GeoLocationInput>,
    include_calendar: Option<bool>,
    riseset_config: Option<RiseSetConfigInput>,
    sankranti_config: Option<SankrantiConfigInput>,
}

#[derive(Debug, Clone, Deserialize)]
struct SearchRequest {
    op: String,
    mode: EnumInput,
    body1: Option<EnumInput>,
    body2: Option<EnumInput>,
    body: Option<EnumInput>,
    kind: Option<EnumInput>,
    target: Option<EnumInput>,
    at_jd_tdb: Option<f64>,
    start_jd_tdb: Option<f64>,
    end_jd_tdb: Option<f64>,
    config: Option<SearchConfigInput>,
    sankranti_config: Option<SankrantiConfigInput>,
}

#[derive(Debug, Clone, Deserialize)]
struct JyotishRequest {
    op: String,
    jd_tdb: Option<f64>,
    utc: Option<UtcInput>,
    location: Option<GeoLocationInput>,
    system: Option<EnumInput>,
    scheme: Option<EnumInput>,
    node_dignity_policy: Option<EnumInput>,
    upagraha_config: Option<TimeUpagrahaConfigInput>,
    graha_positions_config: Option<GrahaPositionsConfigInput>,
    bindus_config: Option<BindusConfigInput>,
    drishti_config: Option<DrishtiConfigInput>,
    bhava_config: Option<BhavaConfigInput>,
    riseset_config: Option<RiseSetConfigInput>,
    sankranti_config: Option<SankrantiConfigInput>,
    full_kundali_config: Option<FullKundaliConfigInput>,
    amsha_requests: Option<Vec<AmshaRequestInput>>,
    amsha_scope: Option<AmshaChartScopeInput>,
}

#[derive(Debug, Clone, Deserialize)]
struct DashaRequest {
    op: String,
    birth_utc: UtcInput,
    query_utc: Option<UtcInput>,
    location: GeoLocationInput,
    system: EnumInput,
    max_level: Option<u8>,
    bhava_config: Option<BhavaConfigInput>,
    riseset_config: Option<RiseSetConfigInput>,
    sankranti_config: Option<SankrantiConfigInput>,
    variation: Option<DashaVariationInput>,
}

#[derive(Debug, Clone, Deserialize)]
struct TaraRequest {
    op: String,
    star: Option<EnumInput>,
    output: Option<EnumInput>,
    jd_tdb: Option<f64>,
    ayanamsha_deg: Option<f64>,
    config: Option<TaraConfigInput>,
}

#[derive(Debug, Clone, Deserialize)]
struct RiseSetConfigInput {
    use_refraction: Option<bool>,
    altitude_correction: Option<bool>,
    sun_limb: Option<EnumInput>,
}

#[derive(Debug, Clone, Deserialize)]
struct BhavaConfigInput {
    system: Option<EnumInput>,
    reference_mode: Option<EnumInput>,
}

#[derive(Debug, Clone, Deserialize)]
struct SankrantiConfigInput {
    ayanamsha_system: Option<EnumInput>,
    use_nutation: Option<bool>,
    reference_plane: Option<EnumInput>,
    step_size_days: Option<f64>,
    max_iterations: Option<u32>,
    convergence_days: Option<f64>,
}

#[derive(Debug, Clone, Deserialize)]
struct SearchConfigInput {
    target_separation_deg: Option<f64>,
    step_size_days: Option<f64>,
    max_iterations: Option<u32>,
    convergence_days: Option<f64>,
    include_penumbral: Option<bool>,
    include_peak_details: Option<bool>,
    numerical_step_days: Option<f64>,
}

#[derive(Debug, Clone, Deserialize)]
struct GrahaPositionsConfigInput {
    include_nakshatra: Option<bool>,
    include_lagna: Option<bool>,
    include_outer_planets: Option<bool>,
    include_bhava: Option<bool>,
}

#[derive(Debug, Clone, Deserialize)]
struct BindusConfigInput {
    include_nakshatra: Option<bool>,
    include_bhava: Option<bool>,
    upagraha_config: Option<TimeUpagrahaConfigInput>,
}

#[derive(Debug, Clone, Deserialize)]
struct TimeUpagrahaConfigInput {
    gulika_point: Option<EnumInput>,
    maandi_point: Option<EnumInput>,
    other_point: Option<EnumInput>,
    gulika_planet: Option<EnumInput>,
    maandi_planet: Option<EnumInput>,
}

#[derive(Debug, Clone, Deserialize)]
struct DrishtiConfigInput {
    include_bhava: Option<bool>,
    include_lagna: Option<bool>,
    include_bindus: Option<bool>,
}

#[derive(Debug, Clone, Deserialize)]
struct FullKundaliConfigInput {
    include_bhava_cusps: Option<bool>,
    include_graha_positions: Option<bool>,
    include_bindus: Option<bool>,
    include_drishti: Option<bool>,
    include_ashtakavarga: Option<bool>,
    include_upagrahas: Option<bool>,
    include_sphutas: Option<bool>,
    include_special_lagnas: Option<bool>,
    include_amshas: Option<bool>,
    include_shadbala: Option<bool>,
    include_bhavabala: Option<bool>,
    include_vimsopaka: Option<bool>,
    include_avastha: Option<bool>,
    include_charakaraka: Option<bool>,
    include_panchang: Option<bool>,
    include_calendar: Option<bool>,
    include_dasha: Option<bool>,
    charakaraka_scheme: Option<EnumInput>,
    node_dignity_policy: Option<EnumInput>,
    upagraha_config: Option<TimeUpagrahaConfigInput>,
    graha_positions_config: Option<GrahaPositionsConfigInput>,
    bindus_config: Option<BindusConfigInput>,
    drishti_config: Option<DrishtiConfigInput>,
    amsha_scope: Option<AmshaChartScopeInput>,
    amsha_selection: Option<Vec<AmshaRequestInput>>,
    dasha_config: Option<DashaSelectionConfigInput>,
}

#[derive(Debug, Clone, Deserialize)]
struct AmshaChartScopeInput {
    include_bhava_cusps: Option<bool>,
    include_arudha_padas: Option<bool>,
    include_upagrahas: Option<bool>,
    include_sphutas: Option<bool>,
    include_special_lagnas: Option<bool>,
}

#[derive(Debug, Clone, Deserialize)]
struct DashaSelectionConfigInput {
    systems: Option<Vec<EnumInput>>,
    max_level: Option<u8>,
    max_levels: Option<Vec<u8>>,
    snapshot_jd: Option<f64>,
}

#[derive(Debug, Clone, Deserialize)]
struct AmshaRequestInput {
    code: u16,
    variation: Option<u8>,
}

#[derive(Debug, Clone, Deserialize)]
struct DashaVariationInput {
    level_methods: Option<Vec<EnumInput>>,
    yogini_scheme: Option<EnumInput>,
    use_abhijit: Option<bool>,
}

#[derive(Debug, Clone, Deserialize)]
struct TaraConfigInput {
    accuracy: Option<EnumInput>,
    apply_parallax: Option<bool>,
}

const BODY_VARIANTS: [Body; 11] = [
    Body::Sun,
    Body::Mercury,
    Body::Venus,
    Body::Earth,
    Body::Moon,
    Body::Mars,
    Body::Jupiter,
    Body::Saturn,
    Body::Uranus,
    Body::Neptune,
    Body::Pluto,
];
const FRAME_VARIANTS: [Frame; 2] = [Frame::IcrfJ2000, Frame::EclipticJ2000];
const LUNAR_NODE_VARIANTS: [LunarNode; 2] = [LunarNode::Rahu, LunarNode::Ketu];
const NODE_MODE_VARIANTS: [NodeMode; 2] = [NodeMode::Mean, NodeMode::True];
const NODE_BACKEND_VARIANTS: [NodeBackend; 2] = [NodeBackend::Analytic, NodeBackend::Engine];
const AYANAMSHA_MODE_VARIANTS: [AyanamshaMode; 3] = [
    AyanamshaMode::Mean,
    AyanamshaMode::True,
    AyanamshaMode::Unified,
];
const RISESET_EVENT_VARIANTS: [RiseSetEvent; 8] = [
    RiseSetEvent::Sunrise,
    RiseSetEvent::Sunset,
    RiseSetEvent::CivilDawn,
    RiseSetEvent::CivilDusk,
    RiseSetEvent::NauticalDawn,
    RiseSetEvent::NauticalDusk,
    RiseSetEvent::AstronomicalDawn,
    RiseSetEvent::AstronomicalDusk,
];
const SUN_LIMB_VARIANTS: [SunLimb; 3] = [SunLimb::UpperLimb, SunLimb::Center, SunLimb::LowerLimb];
const REFERENCE_PLANE_VARIANTS: [ReferencePlane; 2] =
    [ReferencePlane::Ecliptic, ReferencePlane::Invariable];
const BHAVA_REFERENCE_MODE_VARIANTS: [BhavaReferenceMode; 2] = [
    BhavaReferenceMode::StartOfFirst,
    BhavaReferenceMode::MiddleOfFirst,
];
const GRAHAN_KIND_VARIANTS: [GrahanKind; 2] = [GrahanKind::Chandra, GrahanKind::Surya];
const MOTION_KIND_VARIANTS: [MotionKind; 2] = [MotionKind::Stationary, MotionKind::MaxSpeed];
const LUNAR_PHASE_KIND_VARIANTS: [LunarPhaseKind; 2] =
    [LunarPhaseKind::Amavasya, LunarPhaseKind::Purnima];
const TARA_OUTPUT_VARIANTS: [TaraOutputKind; 3] = [
    TaraOutputKind::Equatorial,
    TaraOutputKind::Ecliptic,
    TaraOutputKind::Sidereal,
];
const TARA_ACCURACY_VARIANTS: [TaraAccuracy; 2] =
    [TaraAccuracy::Astrometric, TaraAccuracy::Apparent];
const CHARAKARAKA_SCHEME_VARIANTS: [CharakarakaScheme; 4] = [
    CharakarakaScheme::Eight,
    CharakarakaScheme::SevenNoPitri,
    CharakarakaScheme::SevenPkMergedMk,
    CharakarakaScheme::MixedParashara,
];
const NODE_DIGNITY_POLICY_VARIANTS: [NodeDignityPolicy; 2] = [
    NodeDignityPolicy::SignLordBased,
    NodeDignityPolicy::AlwaysSama,
];
const TIME_UPAGRAHA_POINT_VARIANTS: [TimeUpagrahaPoint; 3] = [
    TimeUpagrahaPoint::Start,
    TimeUpagrahaPoint::Middle,
    TimeUpagrahaPoint::End,
];
const GULIKA_MAANDI_PLANET_VARIANTS: [GulikaMaandiPlanet; 2] =
    [GulikaMaandiPlanet::Rahu, GulikaMaandiPlanet::Saturn];
const YOGINI_SCHEME_VARIANTS: [YoginiScheme; 2] =
    [YoginiScheme::Default, YoginiScheme::LaDeepanshuGiri];
const SUB_PERIOD_METHOD_VARIANTS: [SubPeriodMethod; 4] = [
    SubPeriodMethod::ProportionalFromParent,
    SubPeriodMethod::EqualFromNext,
    SubPeriodMethod::EqualFromSame,
    SubPeriodMethod::ProportionalFromNext,
];

fn load(env: Env, _info: Term) -> bool {
    env.register::<EngineResource>().is_ok()
}

fn decode_term<T: for<'de> Deserialize<'de>>(term: Term<'_>) -> Result<T, rustler::Error> {
    let value: Value = rustler::serde::from_term(term).map_err(rustler::Error::from)?;
    serde_json::from_value(value).map_err(|_| rustler::Error::BadArg)
}

fn encode_json<'a>(env: Env<'a>, result: JsonResult) -> Result<Term<'a>, rustler::Error> {
    rustler::serde::to_term(env, &result).map_err(Into::into)
}

fn read_state(
    resource: &ResourceArc<EngineResource>,
    f: impl FnOnce(&EngineState) -> JsonResult,
) -> JsonResult {
    let guard = resource
        .state
        .read()
        .map_err(|_| error_payload("internal_error", "engine lock poisoned"))?;
    f(&guard)
}

fn write_state(
    resource: &ResourceArc<EngineResource>,
    f: impl FnOnce(&mut EngineState) -> JsonResult,
) -> JsonResult {
    let mut guard = resource
        .state
        .write()
        .map_err(|_| error_payload("internal_error", "engine lock poisoned"))?;
    f(&mut guard)
}

fn require_engine(state: &EngineState) -> Result<&Engine, Value> {
    state
        .engine
        .as_ref()
        .ok_or_else(|| error_payload("closed_engine", "engine is closed"))
}

fn error_payload(kind: &str, message: impl Into<String>) -> Value {
    json!({
        "kind": kind,
        "message": message.into(),
        "details": {}
    })
}

fn map_error(kind: &str, err: impl ToString) -> Value {
    error_payload(kind, err.to_string())
}

fn camel_to_snake(input: &str) -> String {
    let mut out = String::with_capacity(input.len() * 2);
    for (idx, ch) in input.chars().enumerate() {
        if ch.is_uppercase() && idx > 0 {
            out.push('_');
        }
        out.push(ch.to_ascii_lowercase());
    }
    out
}

fn debug_name<T: Debug>(value: T) -> String {
    camel_to_snake(&format!("{value:?}"))
}

const DELTA_T_MODEL_VARIANTS: [DeltaTModel; 2] = [
    DeltaTModel::LegacyEspenakMeeus2006,
    DeltaTModel::Smh2016WithPre720Quadratic,
];

const FUTURE_DELTA_T_TRANSITION_VARIANTS: [FutureDeltaTTransition; 2] = [
    FutureDeltaTTransition::LegacyTtUtcBlend,
    FutureDeltaTTransition::BridgeFromModernEndpoint,
];

const SMH_FUTURE_FAMILY_VARIANTS: [SmhFutureParabolaFamily; 6] = [
    SmhFutureParabolaFamily::Addendum2020Piecewise,
    SmhFutureParabolaFamily::ConstantCMinus20,
    SmhFutureParabolaFamily::ConstantCMinus17p52,
    SmhFutureParabolaFamily::ConstantCMinus15p32,
    SmhFutureParabolaFamily::Stephenson1997,
    SmhFutureParabolaFamily::Stephenson2016,
];

fn parse_named<T: Copy + Debug>(value: &str, variants: &[T]) -> Option<T> {
    let normalized = value.trim().to_ascii_lowercase();
    variants
        .iter()
        .copied()
        .find(|variant| debug_name(*variant) == normalized)
}

fn parse_delta_t_model(input: &EnumInput) -> Result<DeltaTModel, Value> {
    match input {
        EnumInput::Int(index) => DELTA_T_MODEL_VARIANTS
            .get(*index as usize)
            .copied()
            .ok_or_else(|| error_payload("invalid_request", "unknown delta_t_model")),
        EnumInput::Str(value) => parse_named(value, &DELTA_T_MODEL_VARIANTS)
            .ok_or_else(|| error_payload("invalid_request", "unknown delta_t_model")),
    }
}

fn parse_future_delta_t_transition(input: &EnumInput) -> Result<FutureDeltaTTransition, Value> {
    match input {
        EnumInput::Int(index) => FUTURE_DELTA_T_TRANSITION_VARIANTS
            .get(*index as usize)
            .copied()
            .ok_or_else(|| error_payload("invalid_request", "unknown future_delta_t_transition")),
        EnumInput::Str(value) => parse_named(value, &FUTURE_DELTA_T_TRANSITION_VARIANTS)
            .ok_or_else(|| error_payload("invalid_request", "unknown future_delta_t_transition")),
    }
}

fn parse_smh_future_family(input: &EnumInput) -> Result<SmhFutureParabolaFamily, Value> {
    match input {
        EnumInput::Int(index) => SMH_FUTURE_FAMILY_VARIANTS
            .get(*index as usize)
            .copied()
            .ok_or_else(|| error_payload("invalid_request", "unknown smh_future_family")),
        EnumInput::Str(value) => parse_named(value, &SMH_FUTURE_FAMILY_VARIANTS)
            .ok_or_else(|| error_payload("invalid_request", "unknown smh_future_family")),
    }
}

fn parse_time_policy_input(input: Option<TimePolicyInput>) -> Result<TimeConversionPolicy, Value> {
    let request = input.unwrap_or(TimePolicyInput {
        mode: EnumInput::Str("hybrid_delta_t".to_string()),
        options: None,
    });
    match request.mode {
        EnumInput::Str(ref value) if value == "strict_lsk" => Ok(TimeConversionPolicy::StrictLsk),
        EnumInput::Int(0) => Ok(TimeConversionPolicy::StrictLsk),
        _ => {
            let options = request.options.unwrap_or(TimePolicyOptionsInput {
                warn_on_fallback: None,
                delta_t_model: None,
                freeze_future_dut1: None,
                pre_range_dut1: None,
                future_delta_t_transition: None,
                future_transition_years: None,
                smh_future_family: None,
            });
            Ok(TimeConversionPolicy::HybridDeltaT(TimeConversionOptions {
                warn_on_fallback: options.warn_on_fallback.unwrap_or(true),
                delta_t_model: options
                    .delta_t_model
                    .as_ref()
                    .map(parse_delta_t_model)
                    .transpose()?
                    .unwrap_or_default(),
                freeze_future_dut1: options.freeze_future_dut1.unwrap_or(true),
                pre_range_dut1: options.pre_range_dut1.unwrap_or(0.0),
                future_delta_t_transition: options
                    .future_delta_t_transition
                    .as_ref()
                    .map(parse_future_delta_t_transition)
                    .transpose()?
                    .unwrap_or_default(),
                future_transition_years: options.future_transition_years.unwrap_or(100.0),
                smh_future_family: options
                    .smh_future_family
                    .as_ref()
                    .map(parse_smh_future_family)
                    .transpose()?
                    .unwrap_or_default(),
            }))
        }
    }
}

fn time_warning_json(warning: &TimeWarning) -> Value {
    match warning {
        TimeWarning::LskFutureFrozen {
            utc_seconds,
            last_entry_utc_seconds,
            used_delta_at_seconds,
        } => json!({
            "kind": "lsk_future_frozen",
            "utc_seconds": utc_seconds,
            "last_entry_utc_seconds": last_entry_utc_seconds,
            "used_delta_at_seconds": used_delta_at_seconds
        }),
        TimeWarning::LskPreRangeFallback {
            utc_seconds,
            first_entry_utc_seconds,
        } => json!({
            "kind": "lsk_pre_range_fallback",
            "utc_seconds": utc_seconds,
            "first_entry_utc_seconds": first_entry_utc_seconds
        }),
        TimeWarning::EopFutureFrozen {
            mjd,
            last_entry_mjd,
            used_dut1_seconds,
        } => json!({
            "kind": "eop_future_frozen",
            "mjd": mjd,
            "last_entry_mjd": last_entry_mjd,
            "used_dut1_seconds": used_dut1_seconds
        }),
        TimeWarning::EopPreRangeFallback {
            mjd,
            first_entry_mjd,
            used_dut1_seconds,
        } => json!({
            "kind": "eop_pre_range_fallback",
            "mjd": mjd,
            "first_entry_mjd": first_entry_mjd,
            "used_dut1_seconds": used_dut1_seconds
        }),
        TimeWarning::DeltaTModelUsed {
            model,
            segment,
            assumed_dut1_seconds,
        } => json!({
            "kind": "delta_t_model_used",
            "delta_t_model": debug_name(*model),
            "delta_t_segment": debug_name(*segment),
            "used_dut1_seconds": assumed_dut1_seconds
        }),
    }
}

fn time_diagnostics_json(diagnostics: &TimeDiagnostics) -> Value {
    json!({
        "source": match diagnostics.source {
            TtUtcSource::LskDeltaAt => "lsk_delta_at",
            TtUtcSource::DeltaTModel => "delta_t_model"
        },
        "tt_minus_utc_s": diagnostics.tt_minus_utc_s,
        "warnings": diagnostics.warnings.iter().map(time_warning_json).collect::<Vec<_>>()
    })
}

fn parse_body(input: &EnumInput) -> Result<Body, Value> {
    match input {
        EnumInput::Int(code) => Body::from_code(*code as i32)
            .ok_or_else(|| error_payload("invalid_request", "unknown body code")),
        EnumInput::Str(value) => parse_named(value, &BODY_VARIANTS)
            .ok_or_else(|| error_payload("invalid_request", "unknown body name")),
    }
}

fn parse_observer(input: &EnumInput) -> Result<Observer, Value> {
    match input {
        EnumInput::Int(0) => Ok(Observer::SolarSystemBarycenter),
        EnumInput::Int(code) => Body::from_code(*code as i32)
            .map(Observer::Body)
            .ok_or_else(|| error_payload("invalid_request", "unknown observer code")),
        EnumInput::Str(value) => {
            if value.eq_ignore_ascii_case("solar_system_barycenter")
                || value.eq_ignore_ascii_case("ssb")
            {
                Ok(Observer::SolarSystemBarycenter)
            } else {
                parse_named(value, &BODY_VARIANTS)
                    .map(Observer::Body)
                    .ok_or_else(|| error_payload("invalid_request", "unknown observer"))
            }
        }
    }
}

fn parse_frame(input: &EnumInput) -> Result<Frame, Value> {
    match input {
        EnumInput::Int(code) => Frame::from_code(*code as i32)
            .ok_or_else(|| error_payload("invalid_request", "unknown frame")),
        EnumInput::Str(value) => parse_named(value, &FRAME_VARIANTS)
            .ok_or_else(|| error_payload("invalid_request", "unknown frame")),
    }
}

fn parse_ayanamsha_system(input: Option<&EnumInput>) -> Result<AyanamshaSystem, Value> {
    match input {
        None => Ok(AyanamshaSystem::Lahiri),
        Some(EnumInput::Int(value)) => AyanamshaSystem::all()
            .get(*value as usize)
            .copied()
            .ok_or_else(|| error_payload("invalid_request", "unknown ayanamsha system")),
        Some(EnumInput::Str(value)) => parse_named(value, AyanamshaSystem::all())
            .ok_or_else(|| error_payload("invalid_request", "unknown ayanamsha system")),
    }
}

fn parse_charakaraka_scheme(input: Option<&EnumInput>) -> Result<CharakarakaScheme, Value> {
    match input {
        None => Ok(CharakarakaScheme::default()),
        Some(EnumInput::Int(value)) => CharakarakaScheme::from_u8(*value as u8)
            .ok_or_else(|| error_payload("invalid_request", "unknown charakaraka scheme")),
        Some(EnumInput::Str(value)) => parse_named(value, &CHARAKARAKA_SCHEME_VARIANTS)
            .ok_or_else(|| error_payload("invalid_request", "unknown charakaraka scheme")),
    }
}

fn parse_node_dignity_policy(input: Option<&EnumInput>) -> Result<NodeDignityPolicy, Value> {
    match input {
        None => Ok(NodeDignityPolicy::default()),
        Some(EnumInput::Int(value)) => NODE_DIGNITY_POLICY_VARIANTS
            .get(*value as usize)
            .copied()
            .ok_or_else(|| error_payload("invalid_request", "unknown node dignity policy")),
        Some(EnumInput::Str(value)) => parse_named(value, &NODE_DIGNITY_POLICY_VARIANTS)
            .ok_or_else(|| error_payload("invalid_request", "unknown node dignity policy")),
    }
}

fn parse_time_upagraha_point(
    input: Option<&EnumInput>,
    default: TimeUpagrahaPoint,
) -> Result<TimeUpagrahaPoint, Value> {
    match input {
        None => Ok(default),
        Some(EnumInput::Int(value)) => TIME_UPAGRAHA_POINT_VARIANTS
            .get(*value as usize)
            .copied()
            .ok_or_else(|| error_payload("invalid_request", "unknown time upagraha point")),
        Some(EnumInput::Str(value)) => parse_named(value, &TIME_UPAGRAHA_POINT_VARIANTS)
            .ok_or_else(|| error_payload("invalid_request", "unknown time upagraha point")),
    }
}

fn parse_gulika_maandi_planet(
    input: Option<&EnumInput>,
    default: GulikaMaandiPlanet,
) -> Result<GulikaMaandiPlanet, Value> {
    match input {
        None => Ok(default),
        Some(EnumInput::Int(value)) => GULIKA_MAANDI_PLANET_VARIANTS
            .get(*value as usize)
            .copied()
            .ok_or_else(|| error_payload("invalid_request", "unknown Gulika/Maandi planet")),
        Some(EnumInput::Str(value)) => parse_named(value, &GULIKA_MAANDI_PLANET_VARIANTS)
            .ok_or_else(|| error_payload("invalid_request", "unknown Gulika/Maandi planet")),
    }
}

fn apply_time_upagraha_config(
    config: &mut TimeUpagrahaConfig,
    input: Option<&TimeUpagrahaConfigInput>,
) -> Result<(), Value> {
    let Some(input) = input else {
        return Ok(());
    };
    config.gulika_point =
        parse_time_upagraha_point(input.gulika_point.as_ref(), config.gulika_point)?;
    config.maandi_point =
        parse_time_upagraha_point(input.maandi_point.as_ref(), config.maandi_point)?;
    config.other_point = parse_time_upagraha_point(input.other_point.as_ref(), config.other_point)?;
    config.gulika_planet =
        parse_gulika_maandi_planet(input.gulika_planet.as_ref(), config.gulika_planet)?;
    config.maandi_planet =
        parse_gulika_maandi_planet(input.maandi_planet.as_ref(), config.maandi_planet)?;
    Ok(())
}

fn parse_lunar_node(input: &EnumInput) -> Result<LunarNode, Value> {
    match input {
        EnumInput::Int(0) => Ok(LunarNode::Rahu),
        EnumInput::Int(1) => Ok(LunarNode::Ketu),
        EnumInput::Int(_) => Err(error_payload("invalid_request", "unknown lunar node")),
        EnumInput::Str(value) => parse_named(value, &LUNAR_NODE_VARIANTS)
            .ok_or_else(|| error_payload("invalid_request", "unknown lunar node")),
    }
}

fn parse_node_mode(input: Option<&EnumInput>) -> Result<NodeMode, Value> {
    match input {
        None => Ok(NodeMode::True),
        Some(EnumInput::Int(0)) => Ok(NodeMode::Mean),
        Some(EnumInput::Int(1)) => Ok(NodeMode::True),
        Some(EnumInput::Int(_)) => Err(error_payload("invalid_request", "unknown node mode")),
        Some(EnumInput::Str(value)) => parse_named(value, &NODE_MODE_VARIANTS)
            .ok_or_else(|| error_payload("invalid_request", "unknown node mode")),
    }
}

fn parse_node_backend(input: Option<&EnumInput>) -> Result<NodeBackend, Value> {
    match input {
        None => Ok(NodeBackend::Engine),
        Some(EnumInput::Int(value)) => NODE_BACKEND_VARIANTS
            .get(*value as usize)
            .copied()
            .ok_or_else(|| error_payload("invalid_request", "unknown node backend")),
        Some(EnumInput::Str(value)) => parse_named(value, &NODE_BACKEND_VARIANTS)
            .ok_or_else(|| error_payload("invalid_request", "unknown node backend")),
    }
}

fn parse_riseset_event(input: &EnumInput) -> Result<RiseSetEvent, Value> {
    match input {
        EnumInput::Int(value) => RISESET_EVENT_VARIANTS
            .get(*value as usize)
            .copied()
            .ok_or_else(|| error_payload("invalid_request", "unknown rise/set event")),
        EnumInput::Str(value) => parse_named(value, &RISESET_EVENT_VARIANTS)
            .ok_or_else(|| error_payload("invalid_request", "unknown rise/set event")),
    }
}

fn parse_reference_plane(input: Option<&EnumInput>) -> Result<Option<ReferencePlane>, Value> {
    match input {
        None => Ok(None),
        Some(EnumInput::Int(value)) => REFERENCE_PLANE_VARIANTS
            .get(*value as usize)
            .copied()
            .map(Some)
            .ok_or_else(|| error_payload("invalid_request", "unknown reference plane")),
        Some(EnumInput::Str(value)) => parse_named(value, &REFERENCE_PLANE_VARIANTS)
            .map(Some)
            .ok_or_else(|| error_payload("invalid_request", "unknown reference plane")),
    }
}

fn parse_bhava_system(input: Option<&EnumInput>) -> Result<BhavaSystem, Value> {
    match input {
        None => Ok(BhavaConfig::default().system),
        Some(EnumInput::Int(value)) => ALL_BHAVA_SYSTEMS
            .get(*value as usize)
            .copied()
            .ok_or_else(|| error_payload("invalid_request", "unknown bhava system")),
        Some(EnumInput::Str(value)) => parse_named(value, &ALL_BHAVA_SYSTEMS)
            .ok_or_else(|| error_payload("invalid_request", "unknown bhava system")),
    }
}

fn parse_bhava_reference_mode(input: Option<&EnumInput>) -> Result<BhavaReferenceMode, Value> {
    match input {
        None => Ok(BhavaReferenceMode::default()),
        Some(EnumInput::Int(value)) => BHAVA_REFERENCE_MODE_VARIANTS
            .get(*value as usize)
            .copied()
            .ok_or_else(|| error_payload("invalid_request", "unknown bhava reference mode")),
        Some(EnumInput::Str(value)) => parse_named(value, &BHAVA_REFERENCE_MODE_VARIANTS)
            .ok_or_else(|| error_payload("invalid_request", "unknown bhava reference mode")),
    }
}

fn parse_motion_kind(input: &EnumInput) -> Result<MotionKind, Value> {
    match input {
        EnumInput::Int(value) => MOTION_KIND_VARIANTS
            .get(*value as usize)
            .copied()
            .ok_or_else(|| error_payload("invalid_request", "unknown motion kind")),
        EnumInput::Str(value) => parse_named(value, &MOTION_KIND_VARIANTS)
            .ok_or_else(|| error_payload("invalid_request", "unknown motion kind")),
    }
}

fn parse_grahan_kind(input: &EnumInput) -> Result<GrahanKind, Value> {
    match input {
        EnumInput::Int(value) => GRAHAN_KIND_VARIANTS
            .get(*value as usize)
            .copied()
            .ok_or_else(|| error_payload("invalid_request", "unknown grahan kind")),
        EnumInput::Str(value) => parse_named(value, &GRAHAN_KIND_VARIANTS)
            .ok_or_else(|| error_payload("invalid_request", "unknown grahan kind")),
    }
}

fn parse_lunar_phase_kind(input: &EnumInput) -> Result<LunarPhaseKind, Value> {
    match input {
        EnumInput::Int(value) => LUNAR_PHASE_KIND_VARIANTS
            .get(*value as usize)
            .copied()
            .ok_or_else(|| error_payload("invalid_request", "unknown lunar phase kind")),
        EnumInput::Str(value) => parse_named(value, &LUNAR_PHASE_KIND_VARIANTS)
            .ok_or_else(|| error_payload("invalid_request", "unknown lunar phase kind")),
    }
}

fn parse_dasha_system(input: &EnumInput) -> Result<DashaSystem, Value> {
    match input {
        EnumInput::Int(value) => DashaSystem::from_u8(*value as u8)
            .ok_or_else(|| error_payload("invalid_request", "unknown dasha system")),
        EnumInput::Str(value) => parse_named(value, &ALL_DASHA_SYSTEMS)
            .ok_or_else(|| error_payload("invalid_request", "unknown dasha system")),
    }
}

fn parse_tara_id(input: &EnumInput) -> Result<TaraId, Value> {
    match input {
        EnumInput::Int(_) => Err(error_payload(
            "invalid_request",
            "tara star ids must be string names",
        )),
        EnumInput::Str(value) => TaraId::from_str(value)
            .ok_or_else(|| error_payload("invalid_request", "unknown tara id")),
    }
}

fn parse_tara_output(input: Option<&EnumInput>) -> Result<TaraOutputKind, Value> {
    match input {
        None => Ok(TaraOutputKind::Ecliptic),
        Some(EnumInput::Int(value)) => TARA_OUTPUT_VARIANTS
            .get(*value as usize)
            .copied()
            .ok_or_else(|| error_payload("invalid_request", "unknown tara output")),
        Some(EnumInput::Str(value)) => parse_named(value, &TARA_OUTPUT_VARIANTS)
            .ok_or_else(|| error_payload("invalid_request", "unknown tara output")),
    }
}

fn parse_tara_accuracy(input: Option<&EnumInput>) -> Result<TaraAccuracy, Value> {
    match input {
        None => Ok(TaraAccuracy::Astrometric),
        Some(EnumInput::Int(value)) => TARA_ACCURACY_VARIANTS
            .get(*value as usize)
            .copied()
            .ok_or_else(|| error_payload("invalid_request", "unknown tara accuracy")),
        Some(EnumInput::Str(value)) => parse_named(value, &TARA_ACCURACY_VARIANTS)
            .ok_or_else(|| error_payload("invalid_request", "unknown tara accuracy")),
    }
}

fn parse_utc(input: UtcInput) -> Result<UtcTime, Value> {
    UtcTime::try_new(
        input.year,
        input.month,
        input.day,
        input.hour,
        input.minute,
        input.second,
        None,
    )
    .map_err(|err| map_error("time_error", err))
}

fn parse_location(input: GeoLocationInput) -> GeoLocation {
    GeoLocation {
        latitude_deg: input.latitude_deg,
        longitude_deg: input.longitude_deg,
        altitude_m: input.altitude_m.unwrap_or(0.0),
    }
}

fn to_riseset_config(
    state: &EngineState,
    input: Option<&RiseSetConfigInput>,
) -> Result<RiseSetConfig, Value> {
    let mut config = state
        .resolver
        .as_ref()
        .and_then(|resolver| {
            resolver
                .resolve_riseset(None)
                .ok()
                .map(|effective| effective.value)
        })
        .unwrap_or_default();
    if let Some(input) = input {
        config.use_refraction = input.use_refraction.unwrap_or(config.use_refraction);
        config.altitude_correction = input
            .altitude_correction
            .unwrap_or(config.altitude_correction);
        if let Some(limb) = input.sun_limb.as_ref() {
            config.sun_limb = match limb {
                EnumInput::Int(value) => SUN_LIMB_VARIANTS
                    .get(*value as usize)
                    .copied()
                    .ok_or_else(|| error_payload("invalid_request", "unknown sun limb"))?,
                EnumInput::Str(value) => parse_named(value, &SUN_LIMB_VARIANTS)
                    .ok_or_else(|| error_payload("invalid_request", "unknown sun limb"))?,
            };
        }
    }
    Ok(config)
}

fn to_bhava_config(
    state: &EngineState,
    input: Option<&BhavaConfigInput>,
) -> Result<BhavaConfig, Value> {
    let mut config = state
        .resolver
        .as_ref()
        .and_then(|resolver| {
            resolver
                .resolve_bhava(None)
                .ok()
                .map(|effective| effective.value)
        })
        .unwrap_or_default();
    if let Some(input) = input {
        config.system = parse_bhava_system(input.system.as_ref())?;
        config.reference_mode = parse_bhava_reference_mode(input.reference_mode.as_ref())?;
        config.starting_point = BhavaStartingPoint::Lagna;
    }
    Ok(config)
}

fn to_sankranti_config(
    state: &EngineState,
    input: Option<&SankrantiConfigInput>,
) -> Result<SankrantiConfig, Value> {
    let mut config = state
        .resolver
        .as_ref()
        .and_then(|resolver| {
            resolver
                .resolve_sankranti(None)
                .ok()
                .map(|effective| effective.value)
        })
        .unwrap_or_else(SankrantiConfig::default_lahiri);
    if let Some(input) = input {
        if input.ayanamsha_system.is_some() {
            config.ayanamsha_system = parse_ayanamsha_system(input.ayanamsha_system.as_ref())?;
        }
        if let Some(use_nutation) = input.use_nutation {
            config.use_nutation = use_nutation;
        }
        if let Some(plane) = parse_reference_plane(input.reference_plane.as_ref())? {
            config.reference_plane = plane;
        }
        if let Some(step) = input.step_size_days {
            config.step_size_days = step;
        }
        if let Some(iterations) = input.max_iterations {
            config.max_iterations = iterations;
        }
        if let Some(convergence) = input.convergence_days {
            config.convergence_days = convergence;
        }
    }
    Ok(config)
}

fn to_conjunction_config(
    state: &EngineState,
    input: Option<&SearchConfigInput>,
) -> ConjunctionConfig {
    let mut config = state
        .resolver
        .as_ref()
        .and_then(|resolver| {
            resolver
                .resolve_conjunction(None)
                .ok()
                .map(|effective| effective.value)
        })
        .unwrap_or_else(|| ConjunctionConfig::conjunction(0.5));
    if let Some(input) = input {
        if let Some(target) = input.target_separation_deg {
            config.target_separation_deg = target;
        }
        if let Some(step) = input.step_size_days {
            config.step_size_days = step;
        }
        if let Some(iterations) = input.max_iterations {
            config.max_iterations = iterations;
        }
        if let Some(convergence) = input.convergence_days {
            config.convergence_days = convergence;
        }
    }
    config
}

fn to_stationary_config(
    state: &EngineState,
    input: Option<&SearchConfigInput>,
) -> StationaryConfig {
    let mut config = state
        .resolver
        .as_ref()
        .and_then(|resolver| {
            resolver
                .resolve_stationary(None)
                .ok()
                .map(|effective| effective.value)
        })
        .unwrap_or_else(StationaryConfig::inner_planet);
    if let Some(input) = input {
        if let Some(step) = input.step_size_days {
            config.step_size_days = step;
        }
        if let Some(iterations) = input.max_iterations {
            config.max_iterations = iterations;
        }
        if let Some(convergence) = input.convergence_days {
            config.convergence_days = convergence;
        }
        if let Some(numerical) = input.numerical_step_days {
            config.numerical_step_days = numerical;
        }
    }
    config
}

fn to_grahan_config(
    state: &EngineState,
    input: Option<&SearchConfigInput>,
) -> dhruv_search::GrahanConfig {
    let mut config = state
        .resolver
        .as_ref()
        .and_then(|resolver| {
            resolver
                .resolve_grahan(None)
                .ok()
                .map(|effective| effective.value)
        })
        .unwrap_or_default();
    if let Some(input) = input {
        if let Some(include_penumbral) = input.include_penumbral {
            config.include_penumbral = include_penumbral;
        }
        if let Some(include_peak_details) = input.include_peak_details {
            config.include_peak_details = include_peak_details;
        }
    }
    config
}

fn to_graha_positions_config(
    state: &EngineState,
    input: Option<&GrahaPositionsConfigInput>,
) -> Result<dhruv_search::GrahaPositionsConfig, Value> {
    let mut config = state
        .resolver
        .as_ref()
        .and_then(|resolver| {
            resolver
                .resolve_graha_positions(None)
                .ok()
                .map(|effective| effective.value)
        })
        .unwrap_or_default();
    if let Some(input) = input {
        if let Some(include_nakshatra) = input.include_nakshatra {
            config.include_nakshatra = include_nakshatra;
        }
        if let Some(include_lagna) = input.include_lagna {
            config.include_lagna = include_lagna;
        }
        if let Some(include_outer_planets) = input.include_outer_planets {
            config.include_outer_planets = include_outer_planets;
        }
        if let Some(include_bhava) = input.include_bhava {
            config.include_bhava = include_bhava;
        }
    }
    Ok(config)
}

fn to_bindus_config(
    state: &EngineState,
    input: Option<&BindusConfigInput>,
) -> Result<dhruv_search::BindusConfig, Value> {
    let mut config = state
        .resolver
        .as_ref()
        .and_then(|resolver| {
            resolver
                .resolve_bindus(None)
                .ok()
                .map(|effective| effective.value)
        })
        .unwrap_or_default();
    if let Some(input) = input {
        if let Some(include_nakshatra) = input.include_nakshatra {
            config.include_nakshatra = include_nakshatra;
        }
        if let Some(include_bhava) = input.include_bhava {
            config.include_bhava = include_bhava;
        }
        apply_time_upagraha_config(&mut config.upagraha_config, input.upagraha_config.as_ref())?;
    }
    Ok(config)
}

fn to_drishti_config(
    state: &EngineState,
    input: Option<&DrishtiConfigInput>,
) -> dhruv_search::DrishtiConfig {
    let mut config = state
        .resolver
        .as_ref()
        .and_then(|resolver| {
            resolver
                .resolve_drishti(None)
                .ok()
                .map(|effective| effective.value)
        })
        .unwrap_or_default();
    if let Some(input) = input {
        if let Some(include_bhava) = input.include_bhava {
            config.include_bhava = include_bhava;
        }
        if let Some(include_lagna) = input.include_lagna {
            config.include_lagna = include_lagna;
        }
        if let Some(include_bindus) = input.include_bindus {
            config.include_bindus = include_bindus;
        }
    }
    config
}

fn to_amsha_scope(input: Option<&AmshaChartScopeInput>) -> dhruv_search::AmshaChartScope {
    let mut scope = dhruv_search::AmshaChartScope::default();
    if let Some(input) = input {
        if let Some(value) = input.include_bhava_cusps {
            scope.include_bhava_cusps = value;
        }
        if let Some(value) = input.include_arudha_padas {
            scope.include_arudha_padas = value;
        }
        if let Some(value) = input.include_upagrahas {
            scope.include_upagrahas = value;
        }
        if let Some(value) = input.include_sphutas {
            scope.include_sphutas = value;
        }
        if let Some(value) = input.include_special_lagnas {
            scope.include_special_lagnas = value;
        }
    }
    scope
}

fn to_amsha_selection(
    input: Option<&[AmshaRequestInput]>,
) -> Result<dhruv_search::AmshaSelectionConfig, Value> {
    let mut selection = dhruv_search::AmshaSelectionConfig::default();
    let Some(input) = input else {
        return Ok(selection);
    };
    if input.len() > dhruv_search::MAX_AMSHA_REQUESTS {
        return Err(error_payload("invalid_request", "too many amsha requests"));
    }
    selection.count = input.len() as u8;
    for (index, request) in input.iter().enumerate() {
        let amsha = dhruv_vedic_base::Amsha::from_code(request.code)
            .ok_or_else(|| error_payload("invalid_request", "unknown amsha code"))?;
        let variation = match request.variation {
            Some(code) => {
                let variation = dhruv_vedic_base::AmshaVariation::from_code(code)
                    .ok_or_else(|| error_payload("invalid_request", "unknown amsha variation"))?;
                if !variation.is_applicable_to(amsha) {
                    return Err(error_payload(
                        "invalid_request",
                        "amsha variation not applicable to amsha code",
                    ));
                }
                variation
            }
            None => dhruv_vedic_base::AmshaVariation::TraditionalParashari,
        };
        selection.codes[index] = amsha.code();
        selection.variations[index] = match variation {
            dhruv_vedic_base::AmshaVariation::TraditionalParashari => 0,
            dhruv_vedic_base::AmshaVariation::HoraCancerLeoOnly => 1,
        };
    }
    Ok(selection)
}

fn to_full_kundali_config(
    state: &EngineState,
    input: Option<&FullKundaliConfigInput>,
) -> Result<dhruv_search::FullKundaliConfig, Value> {
    let mut config = state
        .resolver
        .as_ref()
        .and_then(|resolver| {
            resolver
                .resolve_full_kundali(None)
                .ok()
                .map(|effective| effective.value)
        })
        .unwrap_or_default();
    if let Some(input) = input {
        if let Some(value) = input.include_bhava_cusps {
            config.include_bhava_cusps = value;
        }
        if let Some(value) = input.include_graha_positions {
            config.include_graha_positions = value;
        }
        if let Some(value) = input.include_bindus {
            config.include_bindus = value;
        }
        if let Some(value) = input.include_drishti {
            config.include_drishti = value;
        }
        if let Some(value) = input.include_ashtakavarga {
            config.include_ashtakavarga = value;
        }
        if let Some(value) = input.include_upagrahas {
            config.include_upagrahas = value;
        }
        if let Some(value) = input.include_sphutas {
            config.include_sphutas = value;
        }
        if let Some(value) = input.include_special_lagnas {
            config.include_special_lagnas = value;
        }
        if let Some(value) = input.include_amshas {
            config.include_amshas = value;
        }
        if let Some(value) = input.include_shadbala {
            config.include_shadbala = value;
        }
        if let Some(value) = input.include_bhavabala {
            config.include_bhavabala = value;
        }
        if let Some(value) = input.include_vimsopaka {
            config.include_vimsopaka = value;
        }
        if let Some(value) = input.include_avastha {
            config.include_avastha = value;
        }
        if let Some(value) = input.include_charakaraka {
            config.include_charakaraka = value;
        }
        if let Some(value) = input.include_panchang {
            config.include_panchang = value;
        }
        if let Some(value) = input.include_calendar {
            config.include_calendar = value;
        }
        if let Some(value) = input.include_dasha {
            config.include_dasha = value;
        }
        if input.charakaraka_scheme.is_some() {
            config.charakaraka_scheme =
                parse_charakaraka_scheme(input.charakaraka_scheme.as_ref())?;
        }
        if input.node_dignity_policy.is_some() {
            config.node_dignity_policy =
                parse_node_dignity_policy(input.node_dignity_policy.as_ref())?;
        }
        if let Some(graha_positions_config) = input.graha_positions_config.as_ref() {
            config.graha_positions_config =
                to_graha_positions_config(state, Some(graha_positions_config))?;
        }
        if let Some(bindus_config) = input.bindus_config.as_ref() {
            config.bindus_config = to_bindus_config(state, Some(bindus_config))?;
        }
        // Apply upagraha_config AFTER bindus_config so the copy to
        // bindus_config.upagraha_config is not overwritten.
        if let Some(upagraha_config) = input.upagraha_config.as_ref() {
            apply_time_upagraha_config(&mut config.upagraha_config, Some(upagraha_config))?;
            config.bindus_config.upagraha_config = config.upagraha_config;
        }
        if let Some(drishti_config) = input.drishti_config.as_ref() {
            config.drishti_config = to_drishti_config(state, Some(drishti_config));
        }
        let amsha_scope = to_amsha_scope(input.amsha_scope.as_ref());
        let amsha_selection = to_amsha_selection(input.amsha_selection.as_deref())?;
        if input.amsha_scope.is_some() {
            config.amsha_scope = amsha_scope;
        }
        if input.amsha_selection.is_some() {
            config.amsha_selection = amsha_selection;
            config.include_amshas = true;
        }
        if config.include_amshas {
            config.graha_positions_config.include_lagna = true;
            if config.amsha_scope.include_bhava_cusps {
                config.include_bhava_cusps = true;
            }
            if config.amsha_scope.include_arudha_padas {
                config.include_bindus = true;
            }
            if config.amsha_scope.include_upagrahas {
                config.include_upagrahas = true;
            }
            if config.amsha_scope.include_sphutas {
                config.include_sphutas = true;
            }
            if config.amsha_scope.include_special_lagnas {
                config.include_special_lagnas = true;
            }
        }
        if let Some(dasha_config) = input.dasha_config.as_ref() {
            if let Some(snapshot_jd) = dasha_config.snapshot_jd {
                config.dasha_config.snapshot_jd = Some(snapshot_jd);
            }
            if let Some(max_level) = dasha_config.max_level {
                config.dasha_config.max_level = max_level;
            }
            if let Some(max_levels) = dasha_config.max_levels.as_ref() {
                for (index, max_level) in max_levels
                    .iter()
                    .copied()
                    .enumerate()
                    .take(config.dasha_config.max_levels.len())
                {
                    config.dasha_config.max_levels[index] = max_level;
                }
            }
            if let Some(systems) = dasha_config.systems.as_ref() {
                config.dasha_config.count = systems.len() as u8;
                for (index, system) in systems.iter().enumerate() {
                    config.dasha_config.systems[index] = parse_dasha_system(system)? as u8;
                }
            }
        }
    }
    Ok(config)
}

fn to_dasha_variation(input: Option<&DashaVariationInput>) -> Result<DashaVariationConfig, Value> {
    let mut config = DashaVariationConfig::default();
    if let Some(input) = input {
        if let Some(use_abhijit) = input.use_abhijit {
            config.use_abhijit = use_abhijit;
        }
        if let Some(scheme) = input.yogini_scheme.as_ref() {
            config.yogini_scheme = match scheme {
                EnumInput::Int(value) => YOGINI_SCHEME_VARIANTS
                    .get(*value as usize)
                    .copied()
                    .ok_or_else(|| error_payload("invalid_request", "unknown yogini scheme"))?,
                EnumInput::Str(value) => parse_named(value, &YOGINI_SCHEME_VARIANTS)
                    .ok_or_else(|| error_payload("invalid_request", "unknown yogini scheme"))?,
            };
        }
        if let Some(level_methods) = input.level_methods.as_ref() {
            for (index, method) in level_methods
                .iter()
                .enumerate()
                .take(config.level_methods.len())
            {
                config.level_methods[index] = Some(match method {
                    EnumInput::Int(value) => SUB_PERIOD_METHOD_VARIANTS
                        .get(*value as usize)
                        .copied()
                        .ok_or_else(|| {
                            error_payload("invalid_request", "unknown sub-period method")
                        })?,
                    EnumInput::Str(value) => parse_named(value, &SUB_PERIOD_METHOD_VARIANTS)
                        .ok_or_else(|| {
                            error_payload("invalid_request", "unknown sub-period method")
                        })?,
                });
            }
        }
    }
    Ok(config)
}

fn to_tara_config(
    state: &EngineState,
    input: Option<&TaraConfigInput>,
) -> Result<TaraConfig, Value> {
    let mut config = state
        .resolver
        .as_ref()
        .and_then(|resolver| {
            resolver
                .resolve_tara(None)
                .ok()
                .map(|effective| effective.value)
        })
        .unwrap_or_default();
    if let Some(input) = input {
        config.apply_parallax = input.apply_parallax.unwrap_or(config.apply_parallax);
        if input.accuracy.is_some() {
            config.accuracy = parse_tara_accuracy(input.accuracy.as_ref())?;
        }
    }
    Ok(config)
}

fn apply_time_policy(state: &EngineState) {
    set_time_conversion_policy(state.time_policy);
}

fn utc_json(utc: UtcTime) -> Value {
    json!({
        "year": utc.year,
        "month": utc.month,
        "day": utc.day,
        "hour": utc.hour,
        "minute": utc.minute,
        "second": utc.second
    })
}

fn state_vector_json(state: StateVector) -> Value {
    json!({
        "position_km": state.position_km,
        "velocity_km_s": state.velocity_km_s
    })
}

fn spherical_json(coords: dhruv_frames::SphericalCoords) -> Value {
    json!({
        "lon_deg": coords.lon_deg,
        "lat_deg": coords.lat_deg,
        "distance_km": coords.distance_km
    })
}

fn spherical_state_json(state: dhruv_frames::SphericalState) -> Value {
    json!({
        "lon_deg": state.lon_deg,
        "lat_deg": state.lat_deg,
        "distance_km": state.distance_km,
        "lon_speed": state.lon_speed,
        "lat_speed": state.lat_speed,
        "distance_speed": state.distance_speed
    })
}

fn parse_query_output(input: Option<&EnumInput>) -> Result<i32, Value> {
    match input {
        None => Ok(0),
        Some(EnumInput::Int(0)) => Ok(0),
        Some(EnumInput::Int(1)) => Ok(1),
        Some(EnumInput::Int(2)) => Ok(2),
        Some(EnumInput::Str(value)) => match value.as_str() {
            "cartesian" => Ok(0),
            "spherical" => Ok(1),
            "both" => Ok(2),
            _ => Err(error_payload("invalid_request", "unknown query output")),
        },
        _ => Err(error_payload("invalid_request", "unknown query output")),
    }
}

fn query_epoch_tdb_jd(state: &EngineState, request: &QueryInput) -> Result<f64, Value> {
    match (request.epoch_tdb_jd, request.utc.as_ref()) {
        (Some(epoch_tdb_jd), None) => Ok(epoch_tdb_jd),
        (None, Some(utc_input)) => {
            let utc = parse_utc(utc_input.clone())?;
            let jd_utc = dhruv_time::calendar_to_jd(
                utc.year,
                utc.month,
                utc.day as f64
                    + utc.hour as f64 / 24.0
                    + utc.minute as f64 / 1440.0
                    + utc.second / 86_400.0,
            );
            let utc_seconds = jd_to_tdb_seconds(jd_utc);
            let tdb_seconds = state
                .engine
                .as_ref()
                .ok_or_else(|| error_payload("engine_error", "engine not initialized"))?
                .lsk()
                .utc_to_tdb_with_policy_and_eop(utc_seconds, state.eop.as_ref(), state.time_policy)
                .tdb_seconds;
            Ok(tdb_seconds_to_jd(tdb_seconds))
        }
        _ => Err(error_payload(
            "invalid_request",
            "provide exactly one of epoch_tdb_jd or utc",
        )),
    }
}

fn ephemeris_query_result_json(state_vector: StateVector, output_mode: i32) -> Value {
    let spherical_state =
        cartesian_state_to_spherical_state(&state_vector.position_km, &state_vector.velocity_km_s);
    let output_name = match output_mode {
        1 => "spherical",
        2 => "both",
        _ => "cartesian",
    };
    let state_value = if output_mode == 1 {
        Value::Null
    } else {
        state_vector_json(state_vector)
    };
    let spherical_value = if output_mode == 0 {
        Value::Null
    } else {
        spherical_state_json(spherical_state)
    };
    json!({
        "state": state_value,
        "spherical_state": spherical_value,
        "output": output_name
    })
}

fn rise_set_result_json(result: RiseSetResult) -> Value {
    match result {
        RiseSetResult::Event { jd_tdb, event } => json!({
            "status": "event",
            "jd_tdb": jd_tdb,
            "event": debug_name(event)
        }),
        RiseSetResult::NeverRises => json!({ "status": "never_rises" }),
        RiseSetResult::NeverSets => json!({ "status": "never_sets" }),
    }
}

fn bhava_result_json(result: BhavaResult) -> Value {
    json!({
        "lagna_deg": result.lagna_deg,
        "mc_deg": result.mc_deg,
        "bhavas": result.bhavas.iter().map(|bhava| json!({
            "number": bhava.number,
            "cusp_deg": bhava.cusp_deg,
            "start_deg": bhava.start_deg,
            "end_deg": bhava.end_deg
        })).collect::<Vec<_>>()
    })
}

fn panchang_value_json(result: &PanchangResult) -> Value {
    json!({
        "tithi": result.tithi.map(tithi_json),
        "karana": result.karana.map(karana_json),
        "yoga": result.yoga.map(yoga_json),
        "vaar": result.vaar.map(vaar_json),
        "hora": result.hora.map(hora_json),
        "ghatika": result.ghatika.map(ghatika_json),
        "nakshatra": result.nakshatra.map(nakshatra_json),
        "masa": result.masa.map(masa_json),
        "ayana": result.ayana.map(ayana_json),
        "varsha": result.varsha.map(varsha_json)
    })
}

fn tithi_json(info: dhruv_search::TithiInfo) -> Value {
    json!({
        "tithi_index": info.tithi_index,
        "paksha": debug_name(info.paksha),
        "tithi_in_paksha": info.tithi_in_paksha,
        "start": utc_json(info.start),
        "end": utc_json(info.end)
    })
}

fn karana_json(info: dhruv_search::KaranaInfo) -> Value {
    json!({
        "karana_index": info.karana_index,
        "karana": debug_name(info.karana),
        "start": utc_json(info.start),
        "end": utc_json(info.end)
    })
}

fn yoga_json(info: dhruv_search::YogaInfo) -> Value {
    json!({
        "yoga_index": info.yoga_index,
        "yoga": debug_name(info.yoga),
        "start": utc_json(info.start),
        "end": utc_json(info.end)
    })
}

fn vaar_json(info: dhruv_search::VaarInfo) -> Value {
    json!({
        "vaar": debug_name(info.vaar),
        "start": utc_json(info.start),
        "end": utc_json(info.end)
    })
}

fn hora_json(info: dhruv_search::HoraInfo) -> Value {
    json!({
        "hora_index": info.hora_index,
        "hora": debug_name(info.hora),
        "start": utc_json(info.start),
        "end": utc_json(info.end)
    })
}

fn ghatika_json(info: dhruv_search::GhatikaInfo) -> Value {
    json!({
        "value": info.value,
        "start": utc_json(info.start),
        "end": utc_json(info.end)
    })
}

fn nakshatra_json(info: dhruv_search::PanchangNakshatraInfo) -> Value {
    json!({
        "nakshatra_index": info.nakshatra_index,
        "nakshatra": debug_name(info.nakshatra),
        "pada": info.pada,
        "start": utc_json(info.start),
        "end": utc_json(info.end)
    })
}

fn masa_json(info: dhruv_search::MasaInfo) -> Value {
    json!({
        "masa": debug_name(info.masa),
        "adhika": info.adhika,
        "start": utc_json(info.start),
        "end": utc_json(info.end)
    })
}

fn ayana_json(info: dhruv_search::AyanaInfo) -> Value {
    json!({
        "ayana": debug_name(info.ayana),
        "start": utc_json(info.start),
        "end": utc_json(info.end)
    })
}

fn varsha_json(info: dhruv_search::VarshaInfo) -> Value {
    json!({
        "samvatsara": debug_name(info.samvatsara),
        "order": info.order,
        "start": utc_json(info.start),
        "end": utc_json(info.end)
    })
}

fn conjunction_result_json(result: ConjunctionResult) -> Value {
    match result {
        ConjunctionResult::Single(event) => json!({ "events": event.map(conjunction_event_json) }),
        ConjunctionResult::Many(events) => {
            json!({ "events": events.into_iter().map(conjunction_event_json).collect::<Vec<_>>() })
        }
    }
}

fn conjunction_event_json(event: dhruv_search::ConjunctionEvent) -> Value {
    json!({
        "jd_tdb": event.jd_tdb,
        "actual_separation_deg": event.actual_separation_deg,
        "body1_longitude_deg": event.body1_longitude_deg,
        "body2_longitude_deg": event.body2_longitude_deg,
        "body1_latitude_deg": event.body1_latitude_deg,
        "body2_latitude_deg": event.body2_latitude_deg,
        "body1": debug_name(event.body1),
        "body2": debug_name(event.body2)
    })
}

fn grahan_result_json(result: GrahanResult) -> Value {
    match result {
        GrahanResult::ChandraSingle(event) => {
            json!({ "kind": "chandra", "events": event.map(chandra_grahan_json) })
        }
        GrahanResult::ChandraMany(events) => {
            json!({ "kind": "chandra", "events": events.into_iter().map(chandra_grahan_json).collect::<Vec<_>>() })
        }
        GrahanResult::SuryaSingle(event) => {
            json!({ "kind": "surya", "events": event.map(surya_grahan_json) })
        }
        GrahanResult::SuryaMany(events) => {
            json!({ "kind": "surya", "events": events.into_iter().map(surya_grahan_json).collect::<Vec<_>>() })
        }
    }
}

fn chandra_grahan_json(event: dhruv_search::ChandraGrahan) -> Value {
    json!({
        "grahan_type": debug_name(event.grahan_type),
        "magnitude": event.magnitude,
        "penumbral_magnitude": event.penumbral_magnitude,
        "greatest_grahan_jd": event.greatest_grahan_jd,
        "p1_jd": event.p1_jd,
        "u1_jd": event.u1_jd,
        "u2_jd": event.u2_jd,
        "u3_jd": event.u3_jd,
        "u4_jd": event.u4_jd,
        "p4_jd": event.p4_jd
    })
}

fn surya_grahan_json(event: dhruv_search::SuryaGrahan) -> Value {
    json!({
        "grahan_type": debug_name(event.grahan_type),
        "magnitude": event.magnitude,
        "greatest_grahan_jd": event.greatest_grahan_jd,
        "c1_jd": event.c1_jd,
        "c2_jd": event.c2_jd,
        "c3_jd": event.c3_jd,
        "c4_jd": event.c4_jd
    })
}

fn lunar_phase_result_json(result: LunarPhaseResult) -> Value {
    match result {
        LunarPhaseResult::Single(event) => json!({ "events": event.map(lunar_phase_event_json) }),
        LunarPhaseResult::Many(events) => {
            json!({ "events": events.into_iter().map(lunar_phase_event_json).collect::<Vec<_>>() })
        }
    }
}

fn lunar_phase_event_json(event: dhruv_search::LunarPhaseEvent) -> Value {
    json!({
        "utc": utc_json(event.utc),
        "phase": debug_name(event.phase),
        "moon_longitude_deg": event.moon_longitude_deg,
        "sun_longitude_deg": event.sun_longitude_deg
    })
}

fn sankranti_result_json(result: SankrantiResult) -> Value {
    match result {
        SankrantiResult::Single(event) => json!({ "events": event.map(sankranti_event_json) }),
        SankrantiResult::Many(events) => {
            json!({ "events": events.into_iter().map(sankranti_event_json).collect::<Vec<_>>() })
        }
    }
}

fn sankranti_event_json(event: dhruv_search::SankrantiEvent) -> Value {
    json!({
        "utc": utc_json(event.utc),
        "rashi": debug_name(event.rashi),
        "rashi_index": event.rashi_index,
        "sun_sidereal_longitude_deg": event.sun_sidereal_longitude_deg,
        "sun_tropical_longitude_deg": event.sun_tropical_longitude_deg
    })
}

fn motion_result_json(result: MotionResult) -> Value {
    match result {
        MotionResult::StationarySingle(event) => {
            json!({ "kind": "stationary", "events": event.map(stationary_event_json) })
        }
        MotionResult::StationaryMany(events) => {
            json!({ "kind": "stationary", "events": events.into_iter().map(stationary_event_json).collect::<Vec<_>>() })
        }
        MotionResult::MaxSpeedSingle(event) => {
            json!({ "kind": "max_speed", "events": event.map(max_speed_event_json) })
        }
        MotionResult::MaxSpeedMany(events) => {
            json!({ "kind": "max_speed", "events": events.into_iter().map(max_speed_event_json).collect::<Vec<_>>() })
        }
    }
}

fn stationary_event_json(event: dhruv_search::StationaryEvent) -> Value {
    json!({
        "jd_tdb": event.jd_tdb,
        "body": debug_name(event.body),
        "longitude_deg": event.longitude_deg,
        "latitude_deg": event.latitude_deg,
        "station_type": debug_name(event.station_type)
    })
}

fn max_speed_event_json(event: dhruv_search::MaxSpeedEvent) -> Value {
    json!({
        "jd_tdb": event.jd_tdb,
        "body": debug_name(event.body),
        "longitude_deg": event.longitude_deg,
        "latitude_deg": event.latitude_deg,
        "speed_deg_per_day": event.speed_deg_per_day,
        "speed_type": debug_name(event.speed_type)
    })
}

fn graha_entry_json(entry: dhruv_search::GrahaEntry, graha: Option<Graha>) -> Value {
    json!({
        "graha": graha.map(debug_name),
        "sidereal_longitude": entry.sidereal_longitude,
        "rashi": debug_name(entry.rashi),
        "rashi_index": entry.rashi_index,
        "nakshatra": debug_name(entry.nakshatra),
        "nakshatra_index": entry.nakshatra_index,
        "pada": entry.pada,
        "bhava_number": entry.bhava_number
    })
}

fn graha_longitudes_json(result: dhruv_search::GrahaLongitudes) -> Value {
    let grahas = ALL_GRAHAS
        .iter()
        .enumerate()
        .map(|(idx, graha)| {
            json!({
                "graha": debug_name(*graha),
                "longitude": result.longitudes[idx]
            })
        })
        .collect::<Vec<_>>();
    json!({ "grahas": grahas, "longitudes": result.longitudes })
}

fn graha_positions_json(result: dhruv_search::GrahaPositions) -> Value {
    json!({
        "grahas": ALL_GRAHAS.iter().enumerate().map(|(idx, graha)| graha_entry_json(result.grahas[idx], Some(*graha))).collect::<Vec<_>>(),
        "lagna": graha_entry_json(result.lagna, None),
        "outer_planets": [
            graha_entry_json(result.outer_planets[0], None),
            graha_entry_json(result.outer_planets[1], None),
            graha_entry_json(result.outer_planets[2], None)
        ]
    })
}

fn special_lagnas_json(result: dhruv_vedic_base::AllSpecialLagnas) -> Value {
    json!({
        "bhava_lagna": result.bhava_lagna,
        "hora_lagna": result.hora_lagna,
        "ghati_lagna": result.ghati_lagna,
        "vighati_lagna": result.vighati_lagna,
        "varnada_lagna": result.varnada_lagna,
        "sree_lagna": result.sree_lagna,
        "pranapada_lagna": result.pranapada_lagna,
        "indu_lagna": result.indu_lagna
    })
}

fn arudha_json(result: [dhruv_vedic_base::ArudhaResult; 12]) -> Value {
    json!({
        "entries": result.into_iter().enumerate().map(|(idx, entry)| json!({
            "index": idx + 1,
            "longitude_deg": entry.longitude_deg,
            "rashi_index": entry.rashi_index
        })).collect::<Vec<_>>()
    })
}

fn upagrahas_json(result: dhruv_vedic_base::AllUpagrahas) -> Value {
    json!({
        "gulika": result.gulika,
        "maandi": result.maandi,
        "kaala": result.kaala,
        "mrityu": result.mrityu,
        "artha_prahara": result.artha_prahara,
        "yama_ghantaka": result.yama_ghantaka,
        "dhooma": result.dhooma,
        "vyatipata": result.vyatipata,
        "parivesha": result.parivesha,
        "indra_chapa": result.indra_chapa,
        "upaketu": result.upaketu
    })
}

fn ashtakavarga_json(result: dhruv_vedic_base::AshtakavargaResult) -> Value {
    json!({
        "bhinna": result.bavs.iter().map(|bav| json!({
            "graha_index": bav.graha_index,
            "points": bav.points,
            "contributors": bav.contributors
        })).collect::<Vec<_>>(),
        "sarva": {
            "total_points": result.sav.total_points,
            "after_trikona": result.sav.after_trikona,
            "after_ekadhipatya": result.sav.after_ekadhipatya
        }
    })
}

fn drishti_json(result: dhruv_search::DrishtiResult) -> Value {
    let graha_to_graha = result
        .graha_to_graha
        .entries
        .iter()
        .map(|row| {
            row.iter()
                .map(|entry| entry.total_virupa)
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    json!({
        "graha_to_graha": graha_to_graha,
        "graha_to_lagna": result.graha_to_lagna.iter().map(|entry| entry.total_virupa).collect::<Vec<_>>()
    })
}

fn charakaraka_json(result: CharakarakaResult) -> Value {
    json!({
        "scheme": debug_name(result.scheme),
        "used_eight_karakas": result.used_eight_karakas,
        "entries": result.entries.into_iter().map(|entry| json!({
            "role": debug_name(entry.role),
            "graha": debug_name(entry.graha),
            "rank": entry.rank,
            "longitude_deg": entry.longitude_deg,
            "degrees_in_rashi": entry.degrees_in_rashi,
            "effective_degrees_in_rashi": entry.effective_degrees_in_rashi
        })).collect::<Vec<_>>()
    })
}

fn shadbala_json(result: dhruv_search::ShadbalaResult) -> Value {
    json!({
        "entries": result.entries.into_iter().map(|entry| json!({
            "graha": debug_name(entry.graha),
            "sthana": {
                "uchcha": entry.sthana.uchcha,
                "saptavargaja": entry.sthana.saptavargaja,
                "ojhayugma": entry.sthana.ojhayugma,
                "kendradi": entry.sthana.kendradi,
                "drekkana": entry.sthana.drekkana,
                "total": entry.sthana.total
            },
            "dig": entry.dig,
            "kala": {
                "nathonnatha": entry.kala.nathonnatha,
                "paksha": entry.kala.paksha,
                "tribhaga": entry.kala.tribhaga,
                "abda": entry.kala.abda,
                "masa": entry.kala.masa,
                "vara": entry.kala.vara,
                "hora": entry.kala.hora,
                "ayana": entry.kala.ayana,
                "yuddha": entry.kala.yuddha,
                "total": entry.kala.total
            },
            "cheshta": entry.cheshta,
            "naisargika": entry.naisargika,
            "drik": entry.drik,
            "total_shashtiamsas": entry.total_shashtiamsas,
            "total_rupas": entry.total_rupas,
            "required_strength": entry.required_strength,
            "is_strong": entry.is_strong
        })).collect::<Vec<_>>()
    })
}

fn bhavabala_json(result: dhruv_vedic_base::BhavaBalaResult) -> Value {
    json!({
        "entries": result.entries.into_iter().map(|entry| json!({
            "bhava_number": entry.bhava_number,
            "cusp_sidereal_lon": entry.cusp_sidereal_lon,
            "rashi_index": entry.rashi_index,
            "rashi": debug_name(dhruv_vedic_base::ALL_RASHIS[entry.rashi_index as usize]),
            "lord": debug_name(entry.lord),
            "bhavadhipati": entry.bhavadhipati,
            "dig": entry.dig,
            "drishti": entry.drishti,
            "occupation_bonus": entry.occupation_bonus,
            "rising_bonus": entry.rising_bonus,
            "total_virupas": entry.total_virupas,
            "total_rupas": entry.total_rupas
        })).collect::<Vec<_>>()
    })
}

fn vimsopaka_json(result: dhruv_search::VimsopakaResult) -> Value {
    json!({
        "entries": result.entries.into_iter().map(|entry| json!({
            "graha": debug_name(entry.graha),
            "shadvarga": entry.shadvarga,
            "saptavarga": entry.saptavarga,
            "dashavarga": entry.dashavarga,
            "shodasavarga": entry.shodasavarga
        })).collect::<Vec<_>>()
    })
}

fn avastha_json(result: dhruv_vedic_base::AllGrahaAvasthas) -> Value {
    json!({
        "entries": result.entries.into_iter().enumerate().map(|(idx, entry)| json!({
            "graha": debug_name(ALL_GRAHAS[idx]),
            "baladi": debug_name(entry.baladi),
            "jagradadi": debug_name(entry.jagradadi),
            "deeptadi": debug_name(entry.deeptadi),
            "lajjitadi": debug_name(entry.lajjitadi),
            "sayanadi": {
                "avastha": debug_name(entry.sayanadi.avastha),
                "sub_states": entry.sayanadi.sub_states.iter().map(|sub_state| debug_name(*sub_state)).collect::<Vec<_>>()
            }
        })).collect::<Vec<_>>()
    })
}

fn amsha_result_json(result: dhruv_search::AmshaResult) -> Value {
    json!({
        "charts": result.charts.into_iter().map(|chart| json!({
            "amsha": debug_name(chart.amsha),
            "variation": debug_name(chart.variation),
            "grahas": chart.grahas.into_iter().map(amsha_entry_json).collect::<Vec<_>>(),
            "lagna": amsha_entry_json(chart.lagna),
            "bhava_cusps": chart
                .bhava_cusps
                .map(|entries| entries.into_iter().map(amsha_entry_json).collect::<Vec<_>>()),
            "arudha_padas": chart
                .arudha_padas
                .map(|entries| entries.into_iter().map(amsha_entry_json).collect::<Vec<_>>()),
            "upagrahas": chart
                .upagrahas
                .map(|entries| entries.into_iter().map(amsha_entry_json).collect::<Vec<_>>()),
            "sphutas": chart
                .sphutas
                .map(|entries| entries.into_iter().map(amsha_entry_json).collect::<Vec<_>>()),
            "special_lagnas": chart
                .special_lagnas
                .map(|entries| entries.into_iter().map(amsha_entry_json).collect::<Vec<_>>())
        })).collect::<Vec<_>>()
    })
}

fn amsha_entry_json(entry: dhruv_search::AmshaEntry) -> Value {
    json!({
        "sidereal_longitude": entry.sidereal_longitude,
        "rashi": debug_name(entry.rashi),
        "rashi_index": entry.rashi_index,
        "degrees_in_rashi": entry.degrees_in_rashi,
        "dms": {
            "degrees": entry.dms.degrees,
            "minutes": entry.dms.minutes,
            "seconds": entry.dms.seconds
        }
    })
}

fn bindus_json(bindus: dhruv_search::BindusResult) -> Value {
    json!({
        "arudha_padas": bindus.arudha_padas.into_iter().map(|entry| graha_entry_json(entry, None)).collect::<Vec<_>>(),
        "bhrigu_bindu": graha_entry_json(bindus.bhrigu_bindu, None),
        "pranapada_lagna": graha_entry_json(bindus.pranapada_lagna, None),
        "gulika": graha_entry_json(bindus.gulika, None),
        "maandi": graha_entry_json(bindus.maandi, None),
        "hora_lagna": graha_entry_json(bindus.hora_lagna, None),
        "ghati_lagna": graha_entry_json(bindus.ghati_lagna, None),
        "sree_lagna": graha_entry_json(bindus.sree_lagna, None)
    })
}

fn full_kundali_json(result: dhruv_search::FullKundaliResult) -> Value {
    json!({
        "ayanamsha_deg": result.ayanamsha_deg,
        "bhava_cusps": result.bhava_cusps.map(bhava_result_json),
        "graha_positions": result.graha_positions.map(graha_positions_json),
        "bindus": result.bindus.map(bindus_json),
        "drishti": result.drishti.map(drishti_json),
        "ashtakavarga": result.ashtakavarga.map(ashtakavarga_json),
        "upagrahas": result.upagrahas.map(upagrahas_json),
        "sphutas": result.sphutas.map(|sphutas| json!({ "longitudes": sphutas.longitudes })),
        "special_lagnas": result.special_lagnas.map(special_lagnas_json),
        "amshas": result.amshas.map(amsha_result_json),
        "shadbala": result.shadbala.map(shadbala_json),
        "bhavabala": result.bhavabala.map(bhavabala_json),
        "vimsopaka": result.vimsopaka.map(vimsopaka_json),
        "avastha": result.avastha.map(avastha_json),
        "charakaraka": result.charakaraka.map(charakaraka_json),
        "panchang": result.panchang.map(|panchang| panchang_value_json(&PanchangResult {
            tithi: Some(panchang.tithi),
            karana: Some(panchang.karana),
            yoga: Some(panchang.yoga),
            vaar: Some(panchang.vaar),
            hora: Some(panchang.hora),
            ghatika: Some(panchang.ghatika),
            nakshatra: Some(panchang.nakshatra),
            masa: panchang.masa,
            ayana: panchang.ayana,
            varsha: panchang.varsha
        })),
        "dasha": result.dasha.map(|items| items.into_iter().map(dasha_hierarchy_json).collect::<Vec<_>>()),
        "dasha_snapshots": result.dasha_snapshots.map(|items| items.into_iter().map(dasha_snapshot_json).collect::<Vec<_>>())
    })
}

fn dasha_hierarchy_json(result: DashaHierarchy) -> Value {
    json!({
        "system": debug_name(result.system),
        "birth_jd": result.birth_jd,
        "levels": result.levels.into_iter().enumerate().map(|(idx, level)| json!({
            "level": idx,
            "name": debug_name(DashaLevel::from_u8(idx as u8).unwrap_or(DashaLevel::Mahadasha)),
            "periods": level.into_iter().map(dasha_period_json).collect::<Vec<_>>()
        })).collect::<Vec<_>>()
    })
}

fn dasha_snapshot_json(result: DashaSnapshot) -> Value {
    json!({
        "system": debug_name(result.system),
        "query_jd": result.query_jd,
        "periods": result.periods.into_iter().map(dasha_period_json).collect::<Vec<_>>()
    })
}

fn dasha_period_json(period: DashaPeriod) -> Value {
    json!({
        "entity": dasha_entity_json(period.entity),
        "start_jd": period.start_jd,
        "end_jd": period.end_jd,
        "level": debug_name(period.level),
        "order": period.order,
        "parent_idx": period.parent_idx
    })
}

fn dasha_entity_json(entity: DashaEntity) -> Value {
    match entity {
        DashaEntity::Graha(graha) => {
            json!({ "kind": "graha", "index": graha.index(), "name": graha.name() })
        }
        DashaEntity::Rashi(index) => {
            let name = dhruv_vedic_base::ALL_RASHIS
                .get(index as usize)
                .map(|rashi| rashi.name())
                .unwrap_or("Unknown");
            json!({ "kind": "rashi", "index": index, "name": name })
        }
        DashaEntity::Yogini(index) => {
            let name = DashaEntity::Yogini(index).name();
            json!({ "kind": "yogini", "index": index, "name": name })
        }
    }
}

fn tara_result_json(result: TaraResult) -> Value {
    match result {
        TaraResult::Equatorial(position) => json!({
            "output": "equatorial",
            "value": {
                "ra_deg": position.ra_deg,
                "dec_deg": position.dec_deg,
                "distance_au": position.distance_au
            }
        }),
        TaraResult::Ecliptic(coords) => json!({
            "output": "ecliptic",
            "value": {
                "lon_deg": coords.lon_deg,
                "lat_deg": coords.lat_deg,
                "distance_km": coords.distance_km
            }
        }),
        TaraResult::Sidereal(longitude) => json!({
            "output": "sidereal",
            "value": {
                "longitude_deg": longitude
            }
        }),
    }
}

fn handle_ephemeris(resource: &ResourceArc<EngineResource>, request: QueryInput) -> JsonResult {
    read_state(resource, |state| {
        let engine = require_engine(state)?;
        let output_mode = parse_query_output(request.output.as_ref())?;
        let query = Query {
            target: parse_body(&request.target)?,
            observer: parse_observer(&request.observer)?,
            frame: parse_frame(request.frame.as_ref().unwrap_or(&EnumInput::Int(0)))?,
            epoch_tdb_jd: query_epoch_tdb_jd(state, &request)?,
        };
        engine
            .query(query)
            .map(|state_vector| ephemeris_query_result_json(state_vector, output_mode))
            .map_err(|err| map_error("engine_error", err))
    })
}

fn handle_body_ecliptic_lon_lat(
    resource: &ResourceArc<EngineResource>,
    request: BodyLonLatInput,
) -> JsonResult {
    read_state(resource, |state| {
        let engine = require_engine(state)?;
        dhruv_search::body_ecliptic_lon_lat(engine, parse_body(&request.body)?, request.jd_tdb)
            .map(|(lon_deg, lat_deg)| json!({ "lon_deg": lon_deg, "lat_deg": lat_deg }))
            .map_err(|err| map_error("search_error", err))
    })
}

fn handle_time(resource: &ResourceArc<EngineResource>, request: TimeRunInput) -> JsonResult {
    read_state(resource, |state| {
        let engine = require_engine(state)?;
        match request.op.as_str() {
            "utc_to_jd_tdb" => {
                let utc = parse_utc(
                    request
                        .utc
                        .ok_or_else(|| error_payload("invalid_request", "utc is required"))?,
                )?;
                let policy = parse_time_policy_input(request.time_policy)?;
                let utc_seconds = jd_to_tdb_seconds(dhruv_time::calendar_to_jd(
                    utc.year,
                    utc.month,
                    utc.day as f64
                        + utc.hour as f64 / 24.0
                        + utc.minute as f64 / 1440.0
                        + utc.second / 86_400.0,
                ));
                let result = engine.lsk().utc_to_tdb_with_policy_and_eop(
                    utc_seconds,
                    state.eop.as_ref(),
                    policy,
                );
                Ok(json!({
                    "jd_tdb": tdb_seconds_to_jd(result.tdb_seconds),
                    "diagnostics": time_diagnostics_json(&result.diagnostics)
                }))
            }
            "jd_tdb_to_utc" => {
                let jd_tdb = request
                    .jd_tdb
                    .ok_or_else(|| error_payload("invalid_request", "jd_tdb is required"))?;
                Ok(utc_json(UtcTime::from_jd_tdb(jd_tdb, engine.lsk())))
            }
            _ => Err(error_payload("invalid_request", "unknown time operation")),
        }
    })
}

fn handle_vedic(resource: &ResourceArc<EngineResource>, request: VedicRequest) -> JsonResult {
    read_state(resource, |state| {
        let engine = require_engine(state)?;
        apply_time_policy(state);
        match request.op.as_str() {
            "ayanamsha" => {
                let op = AyanamshaOperation {
                    system: parse_ayanamsha_system(request.system.as_ref())?,
                    mode: request
                        .mode
                        .as_ref()
                        .map(|input| match input {
                            EnumInput::Int(value) => AYANAMSHA_MODE_VARIANTS
                                .get(*value as usize)
                                .copied()
                                .ok_or_else(|| {
                                    error_payload("invalid_request", "unknown ayanamsha mode")
                                }),
                            EnumInput::Str(value) => parse_named(value, &AYANAMSHA_MODE_VARIANTS)
                                .ok_or_else(|| {
                                    error_payload("invalid_request", "unknown ayanamsha mode")
                                }),
                        })
                        .transpose()?
                        .unwrap_or(AyanamshaMode::Unified),
                    at_jd_tdb: request
                        .jd_tdb
                        .ok_or_else(|| error_payload("invalid_request", "jd_tdb is required"))?,
                    use_nutation: request
                        .sankranti_config
                        .as_ref()
                        .and_then(|config| config.use_nutation)
                        .unwrap_or(false),
                    delta_psi_arcsec: 0.0,
                };
                ayanamsha(&op)
                    .map(|value| json!({ "ayanamsha_deg": value }))
                    .map_err(|err| map_error("search_error", err))
            }
            "lunar_node" => {
                let op =
                    NodeOperation {
                        node: parse_lunar_node(request.system.as_ref().ok_or_else(|| {
                            error_payload("invalid_request", "node is required")
                        })?)?,
                        mode: parse_node_mode(request.mode.as_ref())?,
                        backend: parse_node_backend(request.backend.as_ref())?,
                        at_jd_tdb: request.jd_tdb.ok_or_else(|| {
                            error_payload("invalid_request", "jd_tdb is required")
                        })?,
                    };
                lunar_node(engine, &op)
                    .map(|value| json!({ "longitude_deg": value }))
                    .map_err(|err| map_error("search_error", err))
            }
            "rise_set" => {
                let eop = state.eop.as_ref().ok_or_else(|| {
                    error_payload("missing_eop", "rise/set requires loaded EOP data")
                })?;
                let utc = parse_utc(
                    request
                        .utc
                        .ok_or_else(|| error_payload("invalid_request", "utc is required"))?,
                )?;
                let location = parse_location(
                    request
                        .location
                        .ok_or_else(|| error_payload("invalid_request", "location is required"))?,
                );
                let config = to_riseset_config(state, request.config.as_ref())?;
                let event = parse_riseset_event(
                    request
                        .event
                        .as_ref()
                        .ok_or_else(|| error_payload("invalid_request", "event is required"))?,
                )?;
                let jd_utc_noon = approximate_local_noon_jd(
                    dhruv_time::calendar_to_jd(utc.year, utc.month, utc.day as f64),
                    location.longitude_deg,
                );
                compute_rise_set(
                    engine,
                    engine.lsk(),
                    eop,
                    &location,
                    event,
                    jd_utc_noon,
                    &config,
                )
                .map(rise_set_result_json)
                .map_err(|err| map_error("vedic_error", err))
            }
            "all_events" => {
                let eop = state.eop.as_ref().ok_or_else(|| {
                    error_payload("missing_eop", "rise/set requires loaded EOP data")
                })?;
                let utc = parse_utc(
                    request
                        .utc
                        .ok_or_else(|| error_payload("invalid_request", "utc is required"))?,
                )?;
                let location = parse_location(
                    request
                        .location
                        .ok_or_else(|| error_payload("invalid_request", "location is required"))?,
                );
                let config = to_riseset_config(state, request.config.as_ref())?;
                let jd_utc_noon = approximate_local_noon_jd(
                    dhruv_time::calendar_to_jd(utc.year, utc.month, utc.day as f64),
                    location.longitude_deg,
                );
                compute_all_events(engine, engine.lsk(), eop, &location, jd_utc_noon, &config)
                    .map(|events| json!({ "events": events.into_iter().map(rise_set_result_json).collect::<Vec<_>>() }))
                    .map_err(|err| map_error("vedic_error", err))
            }
            "lagna" | "mc" | "ramc" | "bhavas" => {
                let eop = state.eop.as_ref().ok_or_else(|| {
                    error_payload("missing_eop", "operation requires loaded EOP data")
                })?;
                let utc = parse_utc(
                    request
                        .utc
                        .ok_or_else(|| error_payload("invalid_request", "utc is required"))?,
                )?;
                let location = parse_location(
                    request
                        .location
                        .ok_or_else(|| error_payload("invalid_request", "location is required"))?,
                );
                let bhava_config = to_bhava_config(state, request.bhava_config.as_ref())?;
                let sidereal_output = request.sankranti_config.is_some();
                let sankranti_config = if sidereal_output {
                    Some(to_sankranti_config(
                        state,
                        request.sankranti_config.as_ref(),
                    )?)
                } else {
                    None
                };
                let jd_utc = dhruv_time::calendar_to_jd(
                    utc.year,
                    utc.month,
                    utc.day as f64
                        + utc.hour as f64 / 24.0
                        + utc.minute as f64 / 1440.0
                        + utc.second / 86_400.0,
                );
                match request.op.as_str() {
                    "lagna" if sidereal_output => sidereal_lagna_for_date(
                        engine,
                        eop,
                        &utc,
                        &location,
                        sankranti_config.as_ref().expect("sidereal config set"),
                    )
                    .map(|value| json!({ "longitude_deg": value }))
                    .map_err(|err| map_error("vedic_error", err)),
                    "lagna" => lagna_longitude_rad(engine.lsk(), eop, &location, jd_utc)
                        .map(|value| json!({ "longitude_deg": value.to_degrees() }))
                        .map_err(|err| map_error("vedic_error", err)),
                    "mc" if sidereal_output => sidereal_mc_for_date(
                        engine,
                        eop,
                        &utc,
                        &location,
                        &bhava_config,
                        sankranti_config.as_ref().expect("sidereal config set"),
                    )
                    .map(|value| json!({ "longitude_deg": value }))
                    .map_err(|err| map_error("vedic_error", err)),
                    "mc" => mc_longitude_rad(engine.lsk(), eop, &location, jd_utc)
                        .map(|value| json!({ "longitude_deg": value.to_degrees() }))
                        .map_err(|err| map_error("vedic_error", err)),
                    "ramc" => ramc_rad(engine.lsk(), eop, &location, jd_utc)
                        .map(|value| json!({ "longitude_deg": value.to_degrees() }))
                        .map_err(|err| map_error("vedic_error", err)),
                    _ if sidereal_output => sidereal_bhavas_for_date(
                        engine,
                        eop,
                        &utc,
                        &location,
                        &bhava_config,
                        sankranti_config.as_ref().expect("sidereal config set"),
                    )
                    .map(bhava_result_json)
                    .map_err(|err| map_error("vedic_error", err)),
                    _ => {
                        compute_bhavas(engine, engine.lsk(), eop, &location, jd_utc, &bhava_config)
                            .map(bhava_result_json)
                            .map_err(|err| map_error("vedic_error", err))
                    }
                }
            }
            _ => Err(error_payload("invalid_request", "unknown vedic operation")),
        }
    })
}

fn handle_panchang(resource: &ResourceArc<EngineResource>, request: PanchangRequest) -> JsonResult {
    read_state(resource, |state| {
        let engine = require_engine(state)?;
        let eop = state
            .eop
            .as_ref()
            .ok_or_else(|| error_payload("missing_eop", "panchang requires loaded EOP data"))?;
        apply_time_policy(state);
        let utc = parse_utc(request.utc)?;
        let location = parse_location(request.location.unwrap_or(GeoLocationInput {
            latitude_deg: 0.0,
            longitude_deg: 0.0,
            altitude_m: Some(0.0),
        }));
        let riseset_config = to_riseset_config(state, request.riseset_config.as_ref())?;
        let sankranti_config = to_sankranti_config(state, request.sankranti_config.as_ref())?;
        let result = match request.op.as_str() {
            "tithi" => {
                json!({ "tithi": tithi_json(dhruv_search::tithi_for_date(engine, &utc).map_err(|err| map_error("search_error", err))?) })
            }
            "karana" => {
                json!({ "karana": karana_json(dhruv_search::karana_for_date(engine, &utc).map_err(|err| map_error("search_error", err))?) })
            }
            "yoga" => {
                json!({ "yoga": yoga_json(dhruv_search::yoga_for_date(engine, &utc, &sankranti_config).map_err(|err| map_error("search_error", err))?) })
            }
            "nakshatra" => {
                json!({ "nakshatra": nakshatra_json(dhruv_search::nakshatra_for_date(engine, &utc, &sankranti_config).map_err(|err| map_error("search_error", err))?) })
            }
            "vaar" => {
                json!({ "vaar": vaar_json(dhruv_search::vaar_for_date(engine, eop, &utc, &location, &riseset_config).map_err(|err| map_error("search_error", err))?) })
            }
            "hora" => {
                json!({ "hora": hora_json(dhruv_search::hora_for_date(engine, eop, &utc, &location, &riseset_config).map_err(|err| map_error("search_error", err))?) })
            }
            "ghatika" => {
                json!({ "ghatika": ghatika_json(dhruv_search::ghatika_for_date(engine, eop, &utc, &location, &riseset_config).map_err(|err| map_error("search_error", err))?) })
            }
            "masa" => {
                json!({ "masa": masa_json(dhruv_search::masa_for_date(engine, &utc, &sankranti_config).map_err(|err| map_error("search_error", err))?) })
            }
            "ayana" => {
                json!({ "ayana": ayana_json(dhruv_search::ayana_for_date(engine, &utc, &sankranti_config).map_err(|err| map_error("search_error", err))?) })
            }
            "varsha" => {
                json!({ "varsha": varsha_json(dhruv_search::varsha_for_date(engine, &utc, &sankranti_config).map_err(|err| map_error("search_error", err))?) })
            }
            "daily" => {
                let include_mask = PANCHANG_INCLUDE_ALL
                    - if request.include_calendar.unwrap_or(false) {
                        0
                    } else {
                        PANCHANG_INCLUDE_MASA | PANCHANG_INCLUDE_AYANA | PANCHANG_INCLUDE_VARSHA
                    };
                let op = PanchangOperation {
                    at_utc: utc,
                    location,
                    riseset_config,
                    sankranti_config,
                    include_mask,
                };
                let result =
                    panchang(engine, eop, &op).map_err(|err| map_error("search_error", err))?;
                panchang_value_json(&result)
            }
            _ => {
                return Err(error_payload(
                    "invalid_request",
                    "unknown panchang operation",
                ));
            }
        };
        Ok(result)
    })
}

fn handle_search(resource: &ResourceArc<EngineResource>, request: SearchRequest) -> JsonResult {
    read_state(resource, |state| {
        let engine = require_engine(state)?;
        apply_time_policy(state);
        match request.op.as_str() {
            "conjunction" => {
                let query = match request.mode {
                    EnumInput::Str(ref value) if value == "range" => ConjunctionQuery::Range {
                        start_jd_tdb: request.start_jd_tdb.ok_or_else(|| {
                            error_payload("invalid_request", "start_jd_tdb is required")
                        })?,
                        end_jd_tdb: request.end_jd_tdb.ok_or_else(|| {
                            error_payload("invalid_request", "end_jd_tdb is required")
                        })?,
                    },
                    EnumInput::Str(ref value) if value == "prev" => ConjunctionQuery::Prev {
                        at_jd_tdb: request.at_jd_tdb.ok_or_else(|| {
                            error_payload("invalid_request", "at_jd_tdb is required")
                        })?,
                    },
                    EnumInput::Int(1) => ConjunctionQuery::Prev {
                        at_jd_tdb: request.at_jd_tdb.ok_or_else(|| {
                            error_payload("invalid_request", "at_jd_tdb is required")
                        })?,
                    },
                    EnumInput::Int(2) => ConjunctionQuery::Range {
                        start_jd_tdb: request.start_jd_tdb.ok_or_else(|| {
                            error_payload("invalid_request", "start_jd_tdb is required")
                        })?,
                        end_jd_tdb: request.end_jd_tdb.ok_or_else(|| {
                            error_payload("invalid_request", "end_jd_tdb is required")
                        })?,
                    },
                    _ => ConjunctionQuery::Next {
                        at_jd_tdb: request.at_jd_tdb.ok_or_else(|| {
                            error_payload("invalid_request", "at_jd_tdb is required")
                        })?,
                    },
                };
                let op =
                    ConjunctionOperation {
                        body1: parse_body(request.body1.as_ref().ok_or_else(|| {
                            error_payload("invalid_request", "body1 is required")
                        })?)?,
                        body2: parse_body(request.body2.as_ref().ok_or_else(|| {
                            error_payload("invalid_request", "body2 is required")
                        })?)?,
                        config: to_conjunction_config(state, request.config.as_ref()),
                        query,
                    };
                conjunction(engine, &op)
                    .map(conjunction_result_json)
                    .map_err(|err| map_error("search_error", err))
            }
            "grahan" => {
                let query = match request.mode {
                    EnumInput::Str(ref value) if value == "range" => GrahanQuery::Range {
                        start_jd_tdb: request.start_jd_tdb.ok_or_else(|| {
                            error_payload("invalid_request", "start_jd_tdb is required")
                        })?,
                        end_jd_tdb: request.end_jd_tdb.ok_or_else(|| {
                            error_payload("invalid_request", "end_jd_tdb is required")
                        })?,
                    },
                    EnumInput::Str(ref value) if value == "prev" => GrahanQuery::Prev {
                        at_jd_tdb: request.at_jd_tdb.ok_or_else(|| {
                            error_payload("invalid_request", "at_jd_tdb is required")
                        })?,
                    },
                    _ => GrahanQuery::Next {
                        at_jd_tdb: request.at_jd_tdb.ok_or_else(|| {
                            error_payload("invalid_request", "at_jd_tdb is required")
                        })?,
                    },
                };
                let op =
                    GrahanOperation {
                        kind: parse_grahan_kind(request.kind.as_ref().ok_or_else(|| {
                            error_payload("invalid_request", "kind is required")
                        })?)?,
                        config: to_grahan_config(state, request.config.as_ref()),
                        query,
                    };
                dhruv_search::grahan(engine, &op)
                    .map(grahan_result_json)
                    .map_err(|err| map_error("search_error", err))
            }
            "lunar_phase" => {
                let query = match request.mode {
                    EnumInput::Str(ref value) if value == "range" => LunarPhaseQuery::Range {
                        start_jd_tdb: request.start_jd_tdb.ok_or_else(|| {
                            error_payload("invalid_request", "start_jd_tdb is required")
                        })?,
                        end_jd_tdb: request.end_jd_tdb.ok_or_else(|| {
                            error_payload("invalid_request", "end_jd_tdb is required")
                        })?,
                    },
                    EnumInput::Str(ref value) if value == "prev" => LunarPhaseQuery::Prev {
                        at_jd_tdb: request.at_jd_tdb.ok_or_else(|| {
                            error_payload("invalid_request", "at_jd_tdb is required")
                        })?,
                    },
                    _ => LunarPhaseQuery::Next {
                        at_jd_tdb: request.at_jd_tdb.ok_or_else(|| {
                            error_payload("invalid_request", "at_jd_tdb is required")
                        })?,
                    },
                };
                let op =
                    LunarPhaseOperation {
                        kind: parse_lunar_phase_kind(request.kind.as_ref().ok_or_else(|| {
                            error_payload("invalid_request", "kind is required")
                        })?)?,
                        query,
                    };
                dhruv_search::lunar_phase(engine, &op)
                    .map(lunar_phase_result_json)
                    .map_err(|err| map_error("search_error", err))
            }
            "sankranti" => {
                let query = match request.mode {
                    EnumInput::Str(ref value) if value == "range" => SankrantiQuery::Range {
                        start_jd_tdb: request.start_jd_tdb.ok_or_else(|| {
                            error_payload("invalid_request", "start_jd_tdb is required")
                        })?,
                        end_jd_tdb: request.end_jd_tdb.ok_or_else(|| {
                            error_payload("invalid_request", "end_jd_tdb is required")
                        })?,
                    },
                    EnumInput::Str(ref value) if value == "prev" => SankrantiQuery::Prev {
                        at_jd_tdb: request.at_jd_tdb.ok_or_else(|| {
                            error_payload("invalid_request", "at_jd_tdb is required")
                        })?,
                    },
                    _ => SankrantiQuery::Next {
                        at_jd_tdb: request.at_jd_tdb.ok_or_else(|| {
                            error_payload("invalid_request", "at_jd_tdb is required")
                        })?,
                    },
                };
                let target = match request.target.as_ref() {
                    None => SankrantiTarget::Any,
                    Some(EnumInput::Int(value)) => SankrantiTarget::SpecificRashi(
                        dhruv_vedic_base::ALL_RASHIS
                            .get(*value as usize)
                            .copied()
                            .ok_or_else(|| {
                                error_payload("invalid_request", "unknown rashi target")
                            })?,
                    ),
                    Some(EnumInput::Str(value)) => {
                        let rashi =
                            parse_named(value, &dhruv_vedic_base::ALL_RASHIS).ok_or_else(|| {
                                error_payload("invalid_request", "unknown rashi target")
                            })?;
                        SankrantiTarget::SpecificRashi(rashi)
                    }
                };
                let op = SankrantiOperation {
                    target,
                    config: to_sankranti_config(state, request.sankranti_config.as_ref())?,
                    query,
                };
                dhruv_search::sankranti(engine, &op)
                    .map(sankranti_result_json)
                    .map_err(|err| map_error("search_error", err))
            }
            "motion" => {
                let query = match request.mode {
                    EnumInput::Str(ref value) if value == "range" => MotionQuery::Range {
                        start_jd_tdb: request.start_jd_tdb.ok_or_else(|| {
                            error_payload("invalid_request", "start_jd_tdb is required")
                        })?,
                        end_jd_tdb: request.end_jd_tdb.ok_or_else(|| {
                            error_payload("invalid_request", "end_jd_tdb is required")
                        })?,
                    },
                    EnumInput::Str(ref value) if value == "prev" => MotionQuery::Prev {
                        at_jd_tdb: request.at_jd_tdb.ok_or_else(|| {
                            error_payload("invalid_request", "at_jd_tdb is required")
                        })?,
                    },
                    _ => MotionQuery::Next {
                        at_jd_tdb: request.at_jd_tdb.ok_or_else(|| {
                            error_payload("invalid_request", "at_jd_tdb is required")
                        })?,
                    },
                };
                let op =
                    MotionOperation {
                        body: parse_body(request.body.as_ref().ok_or_else(|| {
                            error_payload("invalid_request", "body is required")
                        })?)?,
                        kind: parse_motion_kind(request.kind.as_ref().ok_or_else(|| {
                            error_payload("invalid_request", "kind is required")
                        })?)?,
                        config: to_stationary_config(state, request.config.as_ref()),
                        query,
                    };
                motion(engine, &op)
                    .map(motion_result_json)
                    .map_err(|err| map_error("search_error", err))
            }
            _ => Err(error_payload("invalid_request", "unknown search operation")),
        }
    })
}

fn handle_jyotish(resource: &ResourceArc<EngineResource>, request: JyotishRequest) -> JsonResult {
    read_state(resource, |state| {
        let engine = require_engine(state)?;
        let eop = state.eop.as_ref().ok_or_else(|| {
            error_payload("missing_eop", "jyotish operations require loaded EOP data")
        })?;
        apply_time_policy(state);
        let utc = request.utc.map(parse_utc).transpose()?;
        let location = request.location.map(parse_location);
        let sankranti_config = to_sankranti_config(state, request.sankranti_config.as_ref())?;
        let bhava_config = to_bhava_config(state, request.bhava_config.as_ref())?;
        match request.op.as_str() {
            "graha_longitudes" => {
                let system = parse_ayanamsha_system(request.system.as_ref())?;
                graha_sidereal_longitudes(
                    engine,
                    request
                        .jd_tdb
                        .ok_or_else(|| error_payload("invalid_request", "jd_tdb is required"))?,
                    system,
                    sankranti_config.use_nutation,
                )
                .map(graha_longitudes_json)
                .map_err(|err| map_error("search_error", err))
            }
            "graha_positions" => {
                let positions = graha_positions_fn(
                    engine,
                    eop,
                    &utc.ok_or_else(|| error_payload("invalid_request", "utc is required"))?,
                    &location
                        .ok_or_else(|| error_payload("invalid_request", "location is required"))?,
                    &bhava_config,
                    &sankranti_config,
                    &to_graha_positions_config(state, request.graha_positions_config.as_ref())?,
                )
                .map_err(|err| map_error("search_error", err))?;
                Ok(graha_positions_json(positions))
            }
            "special_lagnas" => special_lagnas_for_date(
                engine,
                eop,
                &utc.ok_or_else(|| error_payload("invalid_request", "utc is required"))?,
                &location
                    .ok_or_else(|| error_payload("invalid_request", "location is required"))?,
                &to_riseset_config(state, request.riseset_config.as_ref())?,
                &sankranti_config,
            )
            .map(special_lagnas_json)
            .map_err(|err| map_error("search_error", err)),
            "arudha" => arudha_padas_for_date(
                engine,
                eop,
                &utc.ok_or_else(|| error_payload("invalid_request", "utc is required"))?,
                &location
                    .ok_or_else(|| error_payload("invalid_request", "location is required"))?,
                &bhava_config,
                &sankranti_config,
            )
            .map(arudha_json)
            .map_err(|err| map_error("search_error", err)),
            "upagrahas" => {
                let utc = utc.ok_or_else(|| error_payload("invalid_request", "utc is required"))?;
                let location = location
                    .ok_or_else(|| error_payload("invalid_request", "location is required"))?;
                let riseset_config = to_riseset_config(state, request.riseset_config.as_ref())?;
                let result = if let Some(input) = request.upagraha_config.as_ref() {
                    let mut upagraha_config = TimeUpagrahaConfig::default();
                    apply_time_upagraha_config(&mut upagraha_config, Some(input))?;
                    all_upagrahas_for_date_with_config(
                        engine,
                        eop,
                        &utc,
                        &location,
                        &riseset_config,
                        &sankranti_config,
                        &upagraha_config,
                    )
                } else {
                    all_upagrahas_for_date(
                        engine,
                        eop,
                        &utc,
                        &location,
                        &riseset_config,
                        &sankranti_config,
                    )
                };
                result
                    .map(upagrahas_json)
                    .map_err(|err| map_error("search_error", err))
            }
            "bindus" => core_bindus(
                engine,
                eop,
                &utc.ok_or_else(|| error_payload("invalid_request", "utc is required"))?,
                &location
                    .ok_or_else(|| error_payload("invalid_request", "location is required"))?,
                &bhava_config,
                &to_riseset_config(state, request.riseset_config.as_ref())?,
                &sankranti_config,
                &to_bindus_config(state, request.bindus_config.as_ref())?,
            )
            .map(bindus_json)
            .map_err(|err| map_error("search_error", err)),
            "ashtakavarga" => ashtakavarga_for_date(
                engine,
                eop,
                &utc.ok_or_else(|| error_payload("invalid_request", "utc is required"))?,
                &location
                    .ok_or_else(|| error_payload("invalid_request", "location is required"))?,
                &sankranti_config,
            )
            .map(ashtakavarga_json)
            .map_err(|err| map_error("search_error", err)),
            "drishti" => drishti_for_date(
                engine,
                eop,
                &utc.ok_or_else(|| error_payload("invalid_request", "utc is required"))?,
                &location
                    .ok_or_else(|| error_payload("invalid_request", "location is required"))?,
                &bhava_config,
                &to_riseset_config(state, request.riseset_config.as_ref())?,
                &sankranti_config,
                &to_drishti_config(state, request.drishti_config.as_ref()),
            )
            .map(drishti_json)
            .map_err(|err| map_error("search_error", err)),
            "charakaraka" => charakaraka_for_date(
                engine,
                eop,
                &utc.ok_or_else(|| error_payload("invalid_request", "utc is required"))?,
                &sankranti_config,
                parse_charakaraka_scheme(request.scheme.as_ref())?,
            )
            .map(charakaraka_json)
            .map_err(|err| map_error("search_error", err)),
            "shadbala" => shadbala_for_date(
                engine,
                eop,
                &utc.ok_or_else(|| error_payload("invalid_request", "utc is required"))?,
                &location
                    .ok_or_else(|| error_payload("invalid_request", "location is required"))?,
                &bhava_config,
                &to_riseset_config(state, request.riseset_config.as_ref())?,
                &sankranti_config,
            )
            .map(shadbala_json)
            .map_err(|err| map_error("search_error", err)),
            "bhavabala" => bhavabala_for_date(
                engine,
                eop,
                &utc.ok_or_else(|| error_payload("invalid_request", "utc is required"))?,
                &location
                    .ok_or_else(|| error_payload("invalid_request", "location is required"))?,
                &bhava_config,
                &to_riseset_config(state, request.riseset_config.as_ref())?,
                &sankranti_config,
            )
            .map(bhavabala_json)
            .map_err(|err| map_error("search_error", err)),
            "vimsopaka" => vimsopaka_for_date(
                engine,
                eop,
                &utc.ok_or_else(|| error_payload("invalid_request", "utc is required"))?,
                &location
                    .ok_or_else(|| error_payload("invalid_request", "location is required"))?,
                &sankranti_config,
                parse_node_dignity_policy(request.node_dignity_policy.as_ref())?,
            )
            .map(vimsopaka_json)
            .map_err(|err| map_error("search_error", err)),
            "balas" => balas_for_date(
                engine,
                eop,
                &utc.ok_or_else(|| error_payload("invalid_request", "utc is required"))?,
                &location
                    .ok_or_else(|| error_payload("invalid_request", "location is required"))?,
                &bhava_config,
                &to_riseset_config(state, request.riseset_config.as_ref())?,
                &sankranti_config,
                parse_node_dignity_policy(request.node_dignity_policy.as_ref())?,
            )
            .map(|result| {
                json!({
                    "shadbala": shadbala_json(result.shadbala),
                    "vimsopaka": vimsopaka_json(result.vimsopaka),
                    "ashtakavarga": ashtakavarga_json(result.ashtakavarga),
                    "bhavabala": bhavabala_json(result.bhavabala)
                })
            })
            .map_err(|err| map_error("search_error", err)),
            "avastha" => avastha_for_date(
                engine,
                eop,
                &location
                    .ok_or_else(|| error_payload("invalid_request", "location is required"))?,
                &utc.ok_or_else(|| error_payload("invalid_request", "utc is required"))?,
                &bhava_config,
                &to_riseset_config(state, request.riseset_config.as_ref())?,
                &sankranti_config,
                parse_node_dignity_policy(request.node_dignity_policy.as_ref())?,
            )
            .map(avastha_json)
            .map_err(|err| map_error("search_error", err)),
            "full_kundali" => full_kundali_for_date(
                engine,
                eop,
                &utc.ok_or_else(|| error_payload("invalid_request", "utc is required"))?,
                &location
                    .ok_or_else(|| error_payload("invalid_request", "location is required"))?,
                &bhava_config,
                &to_riseset_config(state, request.riseset_config.as_ref())?,
                &sankranti_config,
                &to_full_kundali_config(state, request.full_kundali_config.as_ref())?,
            )
            .map(full_kundali_json)
            .map_err(|err| map_error("search_error", err)),
            "amsha" => {
                let scope = to_amsha_scope(request.amsha_scope.as_ref());
                let requests = request
                    .amsha_requests
                    .unwrap_or_default()
                    .into_iter()
                    .map(|request| {
                        let amsha =
                            dhruv_vedic_base::Amsha::from_code(request.code).ok_or_else(|| {
                                error_payload("invalid_request", "unknown amsha code")
                            })?;
                        let variation = match request.variation {
                            Some(code) => Some(
                                dhruv_vedic_base::AmshaVariation::from_code(code).ok_or_else(
                                    || error_payload("invalid_request", "unknown amsha variation"),
                                )?,
                            ),
                            None => None,
                        };
                        Ok(dhruv_vedic_base::AmshaRequest { amsha, variation })
                    })
                    .collect::<Result<Vec<_>, Value>>()?;
                amsha_charts_for_date(
                    engine,
                    eop,
                    &utc.ok_or_else(|| error_payload("invalid_request", "utc is required"))?,
                    &location
                        .ok_or_else(|| error_payload("invalid_request", "location is required"))?,
                    &bhava_config,
                    &to_riseset_config(state, request.riseset_config.as_ref())?,
                    &sankranti_config,
                    &requests,
                    &scope,
                )
                .map(amsha_result_json)
                .map_err(|err| map_error("search_error", err))
            }
            _ => Err(error_payload(
                "invalid_request",
                "unknown jyotish operation",
            )),
        }
    })
}

fn handle_dasha(resource: &ResourceArc<EngineResource>, request: DashaRequest) -> JsonResult {
    read_state(resource, |state| {
        let engine = require_engine(state)?;
        let eop = state.eop.as_ref().ok_or_else(|| {
            error_payload("missing_eop", "dasha operations require loaded EOP data")
        })?;
        apply_time_policy(state);
        let birth_utc = parse_utc(request.birth_utc)?;
        let location = parse_location(request.location);
        let system = parse_dasha_system(&request.system)?;
        let bhava_config = to_bhava_config(state, request.bhava_config.as_ref())?;
        let riseset_config = to_riseset_config(state, request.riseset_config.as_ref())?;
        let sankranti_config = to_sankranti_config(state, request.sankranti_config.as_ref())?;
        let variation = to_dasha_variation(request.variation.as_ref())?;
        match request.op.as_str() {
            "hierarchy" => dasha_hierarchy_for_birth(
                engine,
                eop,
                &birth_utc,
                &location,
                system,
                request.max_level.unwrap_or(2),
                &bhava_config,
                &riseset_config,
                &sankranti_config,
                &variation,
            )
            .map(dasha_hierarchy_json)
            .map_err(|err| map_error("search_error", err)),
            "snapshot" => {
                dasha_snapshot_at(
                    engine,
                    eop,
                    &birth_utc,
                    &parse_utc(request.query_utc.ok_or_else(|| {
                        error_payload("invalid_request", "query_utc is required")
                    })?)?,
                    &location,
                    system,
                    request.max_level.unwrap_or(2),
                    &bhava_config,
                    &riseset_config,
                    &sankranti_config,
                    &variation,
                )
                .map(dasha_snapshot_json)
                .map_err(|err| map_error("search_error", err))
            }
            _ => Err(error_payload("invalid_request", "unknown dasha operation")),
        }
    })
}

fn handle_tara(resource: &ResourceArc<EngineResource>, request: TaraRequest) -> JsonResult {
    read_state(resource, |state| match request.op.as_str() {
        "catalog_info" => Ok(json!({
            "source": state.tara_catalog.source,
            "reference_epoch_jy": state.tara_catalog.reference_epoch_jy,
            "count": state.tara_catalog.len()
        })),
        "compute" => {
            let op = TaraOperation {
                star: parse_tara_id(
                    request
                        .star
                        .as_ref()
                        .ok_or_else(|| error_payload("invalid_request", "star is required"))?,
                )?,
                output: parse_tara_output(request.output.as_ref())?,
                at_jd_tdb: request
                    .jd_tdb
                    .ok_or_else(|| error_payload("invalid_request", "jd_tdb is required"))?,
                ayanamsha_deg: request.ayanamsha_deg.unwrap_or(0.0),
                config: to_tara_config(state, request.config.as_ref())?,
                earth_state: None,
            };
            tara_op(state.tara_catalog.as_ref(), &op)
                .map(tara_result_json)
                .map_err(|err| map_error("tara_error", err))
        }
        _ => Err(error_payload("invalid_request", "unknown tara operation")),
    })
}

#[rustler::nif(schedule = "DirtyCpu")]
fn engine_new<'a>(env: Env<'a>, config: Term<'a>) -> Result<Term<'a>, rustler::Error> {
    let config = decode_term::<EngineConfigInput>(config)?;
    let time_policy =
        parse_time_policy_input(config.time_policy).map_err(|_| rustler::Error::BadArg)?;
    let engine_config = EngineConfig {
        spk_paths: config.spk_paths.into_iter().map(PathBuf::from).collect(),
        lsk_path: PathBuf::from(config.lsk_path),
        cache_capacity: config.cache_capacity.unwrap_or(64),
        strict_validation: config.strict_validation.unwrap_or(false),
    };
    let engine = match Engine::new(engine_config) {
        Ok(engine) => engine,
        Err(err) => {
            let error_term =
                rustler::serde::to_term(env, &error_payload("engine_error", err.to_string()))?;
            return Ok((atoms::error(), error_term).encode(env));
        }
    };
    let resource = ResourceArc::new(EngineResource {
        state: RwLock::new(EngineState::new(engine, time_policy)),
    });
    Ok((atoms::ok(), resource).encode(env))
}

#[rustler::nif(schedule = "DirtyCpu")]
fn engine_close<'a>(
    env: Env<'a>,
    resource: ResourceArc<EngineResource>,
) -> Result<Term<'a>, rustler::Error> {
    let response = write_state(&resource, |state| {
        state.engine = None;
        Ok(json!({ "closed": true }))
    });
    encode_json(env, response)
}

#[rustler::nif(schedule = "DirtyIo")]
fn engine_load_config<'a>(
    env: Env<'a>,
    resource: ResourceArc<EngineResource>,
    request: Term<'a>,
) -> Result<Term<'a>, rustler::Error> {
    let request = decode_term::<PathInput>(request)?;
    let response = write_state(&resource, |state| {
        let loaded = load_from_path(PathBuf::from(&request.path).as_path())
            .map_err(|err| map_error("config_error", err))?;
        state.resolver = Some(ConfigResolver::new(loaded.file, DefaultsMode::Recommended));
        Ok(json!({ "loaded": true, "path": request.path }))
    });
    encode_json(env, response)
}

#[rustler::nif(schedule = "DirtyCpu")]
fn engine_clear_config<'a>(
    env: Env<'a>,
    resource: ResourceArc<EngineResource>,
) -> Result<Term<'a>, rustler::Error> {
    let response = write_state(&resource, |state| {
        state.resolver = None;
        Ok(json!({ "cleared": true }))
    });
    encode_json(env, response)
}

#[rustler::nif(schedule = "DirtyIo")]
fn engine_load_eop<'a>(
    env: Env<'a>,
    resource: ResourceArc<EngineResource>,
    request: Term<'a>,
) -> Result<Term<'a>, rustler::Error> {
    let request = decode_term::<PathInput>(request)?;
    let response = write_state(&resource, |state| {
        state.eop = Some(
            EopKernel::load(PathBuf::from(&request.path).as_path())
                .map_err(|err| map_error("time_error", err))?,
        );
        Ok(json!({ "loaded": true, "path": request.path }))
    });
    encode_json(env, response)
}

#[rustler::nif(schedule = "DirtyCpu")]
fn engine_clear_eop<'a>(
    env: Env<'a>,
    resource: ResourceArc<EngineResource>,
) -> Result<Term<'a>, rustler::Error> {
    let response = write_state(&resource, |state| {
        state.eop = None;
        Ok(json!({ "cleared": true }))
    });
    encode_json(env, response)
}

#[rustler::nif(schedule = "DirtyIo")]
fn engine_load_tara_catalog<'a>(
    env: Env<'a>,
    resource: ResourceArc<EngineResource>,
    request: Term<'a>,
) -> Result<Term<'a>, rustler::Error> {
    let request = decode_term::<PathInput>(request)?;
    let response = write_state(&resource, |state| {
        state.tara_catalog = Arc::new(
            TaraCatalog::load(PathBuf::from(&request.path).as_path())
                .map_err(|err| map_error("tara_error", err))?,
        );
        Ok(json!({ "loaded": true, "path": request.path }))
    });
    encode_json(env, response)
}

#[rustler::nif(schedule = "DirtyCpu")]
fn engine_reset_tara_catalog<'a>(
    env: Env<'a>,
    resource: ResourceArc<EngineResource>,
) -> Result<Term<'a>, rustler::Error> {
    let response = write_state(&resource, |state| {
        state.tara_catalog = Arc::new(TaraCatalog::embedded().clone());
        Ok(json!({ "reset": true }))
    });
    encode_json(env, response)
}

#[rustler::nif(schedule = "DirtyCpu")]
fn ephemeris_run<'a>(
    env: Env<'a>,
    resource: ResourceArc<EngineResource>,
    request: Term<'a>,
) -> Result<Term<'a>, rustler::Error> {
    let raw = decode_term::<Value>(request)?;
    let op = raw
        .get("op")
        .and_then(Value::as_str)
        .ok_or(rustler::Error::BadArg)?;
    let response = match op {
        "query" => handle_ephemeris(
            &resource,
            serde_json::from_value(raw).map_err(|_| rustler::Error::BadArg)?,
        ),
        "body_ecliptic_lon_lat" => handle_body_ecliptic_lon_lat(
            &resource,
            serde_json::from_value(raw).map_err(|_| rustler::Error::BadArg)?,
        ),
        _ => Err(error_payload(
            "invalid_request",
            "unknown ephemeris operation",
        )),
    };
    encode_json(env, response)
}

#[rustler::nif(schedule = "DirtyCpu")]
fn time_run<'a>(
    env: Env<'a>,
    resource: ResourceArc<EngineResource>,
    request: Term<'a>,
) -> Result<Term<'a>, rustler::Error> {
    let request = decode_term::<TimeRunInput>(request)?;
    encode_json(env, handle_time(&resource, request))
}

#[rustler::nif(schedule = "DirtyCpu")]
fn util_run<'a>(env: Env<'a>, request: Term<'a>) -> Result<Term<'a>, rustler::Error> {
    let raw = decode_term::<Value>(request)?;
    let response = match raw.get("op").and_then(Value::as_str).unwrap_or_default() {
        "cartesian_to_spherical" => {
            let input: CartesianInput =
                serde_json::from_value(raw).map_err(|_| rustler::Error::BadArg)?;
            Ok(spherical_json(cartesian_to_spherical(&[
                input.x, input.y, input.z,
            ])))
        }
        "nutation" => {
            let jd_tdb = raw
                .get("jd_tdb")
                .and_then(Value::as_f64)
                .ok_or(rustler::Error::BadArg)?;
            let t = dhruv_vedic_base::jd_tdb_to_centuries(jd_tdb);
            let (dpsi_arcsec, deps_arcsec) = nutation_iau2000b(t);
            Ok(json!({ "dpsi_arcsec": dpsi_arcsec, "deps_arcsec": deps_arcsec }))
        }
        _ => Err(error_payload(
            "invalid_request",
            "unknown utility operation",
        )),
    };
    encode_json(env, response)
}

#[rustler::nif(schedule = "DirtyCpu")]
fn vedic_run<'a>(
    env: Env<'a>,
    resource: ResourceArc<EngineResource>,
    request: Term<'a>,
) -> Result<Term<'a>, rustler::Error> {
    let request = decode_term::<VedicRequest>(request)?;
    encode_json(env, handle_vedic(&resource, request))
}

#[rustler::nif(schedule = "DirtyCpu")]
fn panchang_run<'a>(
    env: Env<'a>,
    resource: ResourceArc<EngineResource>,
    request: Term<'a>,
) -> Result<Term<'a>, rustler::Error> {
    let request = decode_term::<PanchangRequest>(request)?;
    encode_json(env, handle_panchang(&resource, request))
}

#[rustler::nif(schedule = "DirtyCpu")]
fn search_run<'a>(
    env: Env<'a>,
    resource: ResourceArc<EngineResource>,
    request: Term<'a>,
) -> Result<Term<'a>, rustler::Error> {
    let request = decode_term::<SearchRequest>(request)?;
    encode_json(env, handle_search(&resource, request))
}

#[rustler::nif(schedule = "DirtyCpu")]
fn jyotish_run<'a>(
    env: Env<'a>,
    resource: ResourceArc<EngineResource>,
    request: Term<'a>,
) -> Result<Term<'a>, rustler::Error> {
    let request = decode_term::<JyotishRequest>(request)?;
    encode_json(env, handle_jyotish(&resource, request))
}

#[rustler::nif(schedule = "DirtyCpu")]
fn dasha_run<'a>(
    env: Env<'a>,
    resource: ResourceArc<EngineResource>,
    request: Term<'a>,
) -> Result<Term<'a>, rustler::Error> {
    let request = decode_term::<DashaRequest>(request)?;
    encode_json(env, handle_dasha(&resource, request))
}

#[rustler::nif(schedule = "DirtyCpu")]
fn tara_run<'a>(
    env: Env<'a>,
    resource: ResourceArc<EngineResource>,
    request: Term<'a>,
) -> Result<Term<'a>, rustler::Error> {
    let request = decode_term::<TaraRequest>(request)?;
    encode_json(env, handle_tara(&resource, request))
}

rustler::init!("Elixir.CtaraDhruv.Native", load = load);

#[cfg(test)]
mod tests {
    use super::*;

    fn dummy_state() -> EngineState {
        EngineState {
            engine: None,
            resolver: None,
            eop: None,
            time_policy: TimeConversionPolicy::StrictLsk,
            tara_catalog: Arc::new(TaraCatalog::embedded().clone()),
        }
    }

    #[test]
    fn camel_to_snake_handles_mixed_case() {
        assert_eq!(camel_to_snake("MixedParashara"), "mixed_parashara");
        assert_eq!(camel_to_snake("StrictLsk"), "strict_lsk");
    }

    #[test]
    fn parse_named_matches_debug_form() {
        assert_eq!(
            parse_named("lahiri", AyanamshaSystem::all()),
            Some(AyanamshaSystem::Lahiri)
        );
        assert_eq!(
            parse_named("mixed_parashara", &CHARAKARAKA_SCHEME_VARIANTS),
            Some(CharakarakaScheme::MixedParashara)
        );
    }

    #[test]
    fn resource_mutation_updates_state() {
        let dummy = dummy_state();
        assert!(matches!(dummy.time_policy, TimeConversionPolicy::StrictLsk));
        assert!(dummy.tara_catalog.len() > 0);
    }

    #[test]
    fn error_payload_shape_is_stable() {
        let payload = error_payload("search_error", "boom");
        assert_eq!(payload["kind"], "search_error");
        assert_eq!(payload["message"], "boom");
    }

    #[test]
    fn amsha_selection_conversion_preserves_codes_and_variations() {
        let selection = to_amsha_selection(Some(&[
            AmshaRequestInput {
                code: 9,
                variation: None,
            },
            AmshaRequestInput {
                code: 2,
                variation: Some(1),
            },
        ]))
        .expect("amsha selection should parse");
        assert_eq!(selection.count, 2);
        assert_eq!(selection.codes[0], 9);
        assert_eq!(selection.variations[0], 0);
        assert_eq!(selection.codes[1], 2);
        assert_eq!(selection.variations[1], 1);
    }

    #[test]
    fn full_kundali_config_promotes_amsha_scope_dependencies() {
        let state = dummy_state();
        let config = to_full_kundali_config(
            &state,
            Some(&FullKundaliConfigInput {
                include_bhava_cusps: None,
                include_graha_positions: None,
                include_bindus: None,
                include_drishti: None,
                include_ashtakavarga: None,
                include_upagrahas: None,
                include_sphutas: None,
                include_special_lagnas: None,
                include_amshas: Some(true),
                include_shadbala: None,
                include_bhavabala: None,
                include_vimsopaka: None,
                include_avastha: None,
                include_charakaraka: None,
                include_panchang: None,
                include_calendar: None,
                include_dasha: None,
                charakaraka_scheme: None,
                node_dignity_policy: None,
                graha_positions_config: None,
                bindus_config: None,
                drishti_config: None,
                amsha_scope: Some(AmshaChartScopeInput {
                    include_bhava_cusps: Some(true),
                    include_arudha_padas: Some(true),
                    include_upagrahas: Some(true),
                    include_sphutas: Some(true),
                    include_special_lagnas: Some(true),
                }),
                amsha_selection: Some(vec![AmshaRequestInput {
                    code: 9,
                    variation: None,
                }]),
                dasha_config: None,
            }),
        )
        .expect("full kundali config should parse");
        assert!(config.include_amshas);
        assert!(config.graha_positions_config.include_lagna);
        assert!(config.include_bhava_cusps);
        assert!(config.include_bindus);
        assert!(config.include_upagrahas);
        assert!(config.include_sphutas);
        assert!(config.include_special_lagnas);
        assert_eq!(config.amsha_selection.count, 1);
        assert_eq!(config.amsha_selection.codes[0], 9);
        assert!(config.amsha_scope.include_bhava_cusps);
        assert!(config.amsha_scope.include_arudha_padas);
        assert!(config.amsha_scope.include_upagrahas);
        assert!(config.amsha_scope.include_sphutas);
        assert!(config.amsha_scope.include_special_lagnas);
    }
}
