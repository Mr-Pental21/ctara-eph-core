//! Canonical operation-style APIs for `dhruv_rs`.
//!
//! Search/event requests stay backed by `dhruv_search`, while assembled Vedic
//! operations map to `dhruv_vedic_ops`.

use dhruv_core::Body;
use dhruv_search::{
    AmshaSelectionConfig, ConjunctionConfig, ConjunctionOperation, ConjunctionQuery,
    ConjunctionResult, GrahanConfig, GrahanKind, GrahanOperation, GrahanQuery, GrahanResult,
    LunarPhaseKind, LunarPhaseOperation, LunarPhaseQuery, LunarPhaseResult, MotionKind,
    MotionOperation, MotionQuery, MotionResult, SankrantiConfig, SankrantiOperation,
    SankrantiQuery, SankrantiResult, SankrantiTarget, StationaryConfig,
    all_upagrahas_for_date_with_config, avastha_for_date, avastha_for_graha, full_kundali_for_date,
};
use dhruv_search::{FullKundaliConfig, FullKundaliResult};
use dhruv_tara::{EarthState, TaraCatalog, TaraConfig, TaraId};
use dhruv_time::{EopKernel, UtcTime, calendar_to_jd, jd_to_tdb_seconds, tdb_seconds_to_jd};
use dhruv_vedic_base::{
    AllGrahaAvasthas, AllUpagrahas, AyanamshaSystem, BhavaConfig, CharakarakaResult,
    CharakarakaScheme, GeoLocation, Graha, GrahaAvasthas, LunarNode, NodeDignityPolicy, NodeMode,
    RiseSetConfig, TimeUpagrahaConfig,
};
use dhruv_vedic_ops::{
    AyanamshaMode, AyanamshaOperation, NodeBackend, NodeOperation, PanchangOperation,
    PanchangResult, TaraOperation, TaraOutputKind, TaraResult,
};

use crate::context::DhruvContext;
use crate::date::UtcDate;
use crate::error::DhruvError;

/// Time input accepted by operation requests.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TimeInput {
    /// UTC calendar timestamp.
    Utc(UtcDate),
    /// Julian Date in TDB.
    JdTdb(f64),
}

/// Query selector for conjunction operations.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ConjunctionRequestQuery {
    /// Find next event after `at`.
    Next { at: TimeInput },
    /// Find previous event before `at`.
    Prev { at: TimeInput },
    /// Find all events in `[start, end]`.
    Range { start: TimeInput, end: TimeInput },
}

/// Unified conjunction request.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ConjunctionRequest {
    pub body1: Body,
    pub body2: Body,
    pub config: Option<ConjunctionConfig>,
    pub query: ConjunctionRequestQuery,
}

fn resolve_conjunction_config(
    ctx: &DhruvContext,
    explicit: Option<ConjunctionConfig>,
) -> Result<ConjunctionConfig, DhruvError> {
    if let Some(cfg) = explicit {
        return Ok(cfg);
    }
    if let Some(resolver) = ctx.resolver() {
        return resolver
            .resolve_conjunction(None)
            .map(|v| v.value)
            .map_err(|e| DhruvError::Config(e.to_string()));
    }
    Ok(ConjunctionConfig::conjunction(0.5))
}

fn utc_to_jd_tdb_for_context(ctx: &DhruvContext, date: UtcDate) -> f64 {
    let eng = ctx.engine();
    let day_frac =
        date.day as f64 + date.hour as f64 / 24.0 + date.min as f64 / 1440.0 + date.sec / 86_400.0;
    let jd_utc = calendar_to_jd(date.year, date.month, day_frac);
    let utc_seconds = jd_to_tdb_seconds(jd_utc);
    let out =
        eng.lsk()
            .utc_to_tdb_with_policy_and_eop(utc_seconds, None, ctx.time_conversion_policy());
    tdb_seconds_to_jd(out.tdb_seconds)
}

fn time_input_to_jd_tdb(ctx: &DhruvContext, input: TimeInput) -> f64 {
    match input {
        TimeInput::Utc(date) => utc_to_jd_tdb_for_context(ctx, date),
        TimeInput::JdTdb(jd) => jd,
    }
}

fn time_input_to_utc_for_context(ctx: &DhruvContext, input: TimeInput) -> UtcTime {
    match input {
        TimeInput::Utc(date) => date.into(),
        TimeInput::JdTdb(jd) => UtcTime::from_jd_tdb(jd, ctx.engine().lsk()),
    }
}

/// Execute a unified conjunction operation.
pub fn conjunction(
    ctx: &DhruvContext,
    request: &ConjunctionRequest,
) -> Result<ConjunctionResult, DhruvError> {
    let eng = ctx.engine();
    let query = match request.query {
        ConjunctionRequestQuery::Next { at } => ConjunctionQuery::Next {
            at_jd_tdb: time_input_to_jd_tdb(ctx, at),
        },
        ConjunctionRequestQuery::Prev { at } => ConjunctionQuery::Prev {
            at_jd_tdb: time_input_to_jd_tdb(ctx, at),
        },
        ConjunctionRequestQuery::Range { start, end } => ConjunctionQuery::Range {
            start_jd_tdb: time_input_to_jd_tdb(ctx, start),
            end_jd_tdb: time_input_to_jd_tdb(ctx, end),
        },
    };
    let op = ConjunctionOperation {
        body1: request.body1,
        body2: request.body2,
        config: resolve_conjunction_config(ctx, request.config)?,
        query,
    };
    Ok(dhruv_search::conjunction(eng, &op)?)
}

/// Query selector for grahan operations.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GrahanRequestQuery {
    /// Find next event after `at`.
    Next { at: TimeInput },
    /// Find previous event before `at`.
    Prev { at: TimeInput },
    /// Find all events in `[start, end]`.
    Range { start: TimeInput, end: TimeInput },
}

/// Unified grahan request.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GrahanRequest {
    pub kind: GrahanKind,
    pub config: Option<GrahanConfig>,
    pub query: GrahanRequestQuery,
}

fn resolve_grahan_config(
    ctx: &DhruvContext,
    explicit: Option<GrahanConfig>,
) -> Result<GrahanConfig, DhruvError> {
    if let Some(cfg) = explicit {
        return Ok(cfg);
    }
    if let Some(resolver) = ctx.resolver() {
        return resolver
            .resolve_grahan(None)
            .map(|v| v.value)
            .map_err(|e| DhruvError::Config(e.to_string()));
    }
    Ok(GrahanConfig::default())
}

/// Execute a unified grahan operation.
pub fn grahan(ctx: &DhruvContext, request: &GrahanRequest) -> Result<GrahanResult, DhruvError> {
    let eng = ctx.engine();
    let query = match request.query {
        GrahanRequestQuery::Next { at } => GrahanQuery::Next {
            at_jd_tdb: time_input_to_jd_tdb(ctx, at),
        },
        GrahanRequestQuery::Prev { at } => GrahanQuery::Prev {
            at_jd_tdb: time_input_to_jd_tdb(ctx, at),
        },
        GrahanRequestQuery::Range { start, end } => GrahanQuery::Range {
            start_jd_tdb: time_input_to_jd_tdb(ctx, start),
            end_jd_tdb: time_input_to_jd_tdb(ctx, end),
        },
    };
    let op = GrahanOperation {
        kind: request.kind,
        config: resolve_grahan_config(ctx, request.config)?,
        query,
    };
    Ok(dhruv_search::grahan(eng, &op)?)
}

/// Query selector for motion operations.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MotionRequestQuery {
    /// Find next event after `at`.
    Next { at: TimeInput },
    /// Find previous event before `at`.
    Prev { at: TimeInput },
    /// Find all events in `[start, end]`.
    Range { start: TimeInput, end: TimeInput },
}

/// Unified motion request.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MotionRequest {
    pub body: Body,
    pub kind: MotionKind,
    pub config: Option<StationaryConfig>,
    pub query: MotionRequestQuery,
}

fn resolve_motion_config(
    ctx: &DhruvContext,
    explicit: Option<StationaryConfig>,
) -> Result<StationaryConfig, DhruvError> {
    if let Some(cfg) = explicit {
        return Ok(cfg);
    }
    if let Some(resolver) = ctx.resolver() {
        return resolver
            .resolve_stationary(None)
            .map(|v| v.value)
            .map_err(|e| DhruvError::Config(e.to_string()));
    }
    Ok(StationaryConfig::inner_planet())
}

fn resolve_bhava_config(
    ctx: &DhruvContext,
    explicit: Option<BhavaConfig>,
) -> Result<BhavaConfig, DhruvError> {
    if let Some(cfg) = explicit {
        return Ok(cfg);
    }
    if let Some(resolver) = ctx.resolver() {
        return resolver
            .resolve_bhava(None)
            .map(|v| v.value)
            .map_err(|e| DhruvError::Config(e.to_string()));
    }
    Ok(BhavaConfig::default())
}

/// Execute a unified motion operation.
pub fn motion(ctx: &DhruvContext, request: &MotionRequest) -> Result<MotionResult, DhruvError> {
    let eng = ctx.engine();
    let query = match request.query {
        MotionRequestQuery::Next { at } => MotionQuery::Next {
            at_jd_tdb: time_input_to_jd_tdb(ctx, at),
        },
        MotionRequestQuery::Prev { at } => MotionQuery::Prev {
            at_jd_tdb: time_input_to_jd_tdb(ctx, at),
        },
        MotionRequestQuery::Range { start, end } => MotionQuery::Range {
            start_jd_tdb: time_input_to_jd_tdb(ctx, start),
            end_jd_tdb: time_input_to_jd_tdb(ctx, end),
        },
    };
    let op = MotionOperation {
        body: request.body,
        kind: request.kind,
        config: resolve_motion_config(ctx, request.config)?,
        query,
    };
    Ok(dhruv_search::motion(eng, &op)?)
}

/// Query selector for lunar-phase operations.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LunarPhaseRequestQuery {
    /// Find next event after `at`.
    Next { at: TimeInput },
    /// Find previous event before `at`.
    Prev { at: TimeInput },
    /// Find all events in `[start, end]`.
    Range { start: TimeInput, end: TimeInput },
}

/// Unified lunar-phase request.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LunarPhaseRequest {
    pub kind: LunarPhaseKind,
    pub query: LunarPhaseRequestQuery,
}

/// Execute a unified lunar-phase operation.
pub fn lunar_phase(
    ctx: &DhruvContext,
    request: &LunarPhaseRequest,
) -> Result<LunarPhaseResult, DhruvError> {
    let eng = ctx.engine();
    let query = match request.query {
        LunarPhaseRequestQuery::Next { at } => LunarPhaseQuery::Next {
            at_jd_tdb: time_input_to_jd_tdb(ctx, at),
        },
        LunarPhaseRequestQuery::Prev { at } => LunarPhaseQuery::Prev {
            at_jd_tdb: time_input_to_jd_tdb(ctx, at),
        },
        LunarPhaseRequestQuery::Range { start, end } => LunarPhaseQuery::Range {
            start_jd_tdb: time_input_to_jd_tdb(ctx, start),
            end_jd_tdb: time_input_to_jd_tdb(ctx, end),
        },
    };
    let op = LunarPhaseOperation {
        kind: request.kind,
        query,
    };
    Ok(dhruv_search::lunar_phase(eng, &op)?)
}

/// Query selector for sankranti operations.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SankrantiRequestQuery {
    /// Find next event after `at`.
    Next { at: TimeInput },
    /// Find previous event before `at`.
    Prev { at: TimeInput },
    /// Find all events in `[start, end]`.
    Range { start: TimeInput, end: TimeInput },
}

/// Unified sankranti request.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SankrantiRequest {
    pub target: SankrantiTarget,
    pub config: Option<SankrantiConfig>,
    pub query: SankrantiRequestQuery,
}

fn resolve_sankranti_config(
    ctx: &DhruvContext,
    explicit: Option<SankrantiConfig>,
) -> Result<SankrantiConfig, DhruvError> {
    if let Some(cfg) = explicit {
        return Ok(cfg);
    }
    if let Some(resolver) = ctx.resolver() {
        return resolver
            .resolve_sankranti(None)
            .map(|v| v.value)
            .map_err(|e| DhruvError::Config(e.to_string()));
    }
    Ok(SankrantiConfig::default_lahiri())
}

/// Execute a unified sankranti operation.
pub fn sankranti(
    ctx: &DhruvContext,
    request: &SankrantiRequest,
) -> Result<SankrantiResult, DhruvError> {
    let eng = ctx.engine();
    let query = match request.query {
        SankrantiRequestQuery::Next { at } => SankrantiQuery::Next {
            at_jd_tdb: time_input_to_jd_tdb(ctx, at),
        },
        SankrantiRequestQuery::Prev { at } => SankrantiQuery::Prev {
            at_jd_tdb: time_input_to_jd_tdb(ctx, at),
        },
        SankrantiRequestQuery::Range { start, end } => SankrantiQuery::Range {
            start_jd_tdb: time_input_to_jd_tdb(ctx, start),
            end_jd_tdb: time_input_to_jd_tdb(ctx, end),
        },
    };
    let op = SankrantiOperation {
        target: request.target,
        config: resolve_sankranti_config(ctx, request.config)?,
        query,
    };
    Ok(dhruv_search::sankranti(eng, &op)?)
}

/// Ayanamsha request mode.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AyanamshaRequestMode {
    /// Mean ayanamsha.
    Mean,
    /// True ayanamsha from explicit delta-psi arcseconds.
    True { delta_psi_arcsec: f64 },
    /// Unified ayanamsha with `use_nutation`.
    Unified { use_nutation: bool },
}

/// Unified ayanamsha request.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AyanamshaRequest {
    pub system: AyanamshaSystem,
    pub at: TimeInput,
    pub mode: AyanamshaRequestMode,
}

/// Execute a unified ayanamsha operation.
pub fn ayanamsha_op(ctx: &DhruvContext, request: &AyanamshaRequest) -> Result<f64, DhruvError> {
    let (mode, use_nutation, delta_psi_arcsec) = match request.mode {
        AyanamshaRequestMode::Mean => (AyanamshaMode::Mean, false, 0.0),
        AyanamshaRequestMode::True { delta_psi_arcsec } => {
            (AyanamshaMode::True, false, delta_psi_arcsec)
        }
        AyanamshaRequestMode::Unified { use_nutation } => {
            (AyanamshaMode::Unified, use_nutation, 0.0)
        }
    };
    let op = AyanamshaOperation {
        system: request.system,
        mode,
        at_jd_tdb: time_input_to_jd_tdb(ctx, request.at),
        use_nutation,
        delta_psi_arcsec,
    };
    Ok(dhruv_vedic_ops::ayanamsha(&op)?)
}

/// Unified lunar-node request.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NodeRequest {
    pub node: LunarNode,
    pub mode: NodeMode,
    pub backend: NodeBackend,
    pub at: TimeInput,
}

/// Execute a unified lunar-node operation.
pub fn lunar_node_op(ctx: &DhruvContext, request: &NodeRequest) -> Result<f64, DhruvError> {
    let eng = ctx.engine();
    let op = NodeOperation {
        node: request.node,
        mode: request.mode,
        backend: request.backend,
        at_jd_tdb: time_input_to_jd_tdb(ctx, request.at),
    };
    Ok(dhruv_vedic_ops::lunar_node(eng, &op)?)
}

/// Unified panchang request.
#[derive(Debug, Clone, PartialEq)]
pub struct PanchangRequest {
    pub at: TimeInput,
    pub location: GeoLocation,
    pub riseset_config: Option<RiseSetConfig>,
    pub sankranti_config: Option<SankrantiConfig>,
    pub include_mask: u32,
}

fn resolve_riseset_config(
    ctx: &DhruvContext,
    explicit: Option<RiseSetConfig>,
) -> Result<RiseSetConfig, DhruvError> {
    if let Some(cfg) = explicit {
        return Ok(cfg);
    }
    if let Some(resolver) = ctx.resolver() {
        return resolver
            .resolve_riseset(None)
            .map(|v| v.value)
            .map_err(|e| DhruvError::Config(e.to_string()));
    }
    Ok(RiseSetConfig::default())
}

/// Execute a unified panchang operation.
pub fn panchang_op(
    ctx: &DhruvContext,
    request: &PanchangRequest,
    eop: &EopKernel,
) -> Result<PanchangResult, DhruvError> {
    let eng = ctx.engine();
    let op = PanchangOperation {
        at_utc: time_input_to_utc_for_context(ctx, request.at),
        location: request.location,
        riseset_config: resolve_riseset_config(ctx, request.riseset_config)?,
        sankranti_config: resolve_sankranti_config(ctx, request.sankranti_config)?,
        include_mask: request.include_mask,
    };
    Ok(dhruv_vedic_ops::panchang(eng, eop, &op)?)
}

/// Unified tara request.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TaraRequest {
    pub star: TaraId,
    pub output: TaraOutputKind,
    pub at: TimeInput,
    pub ayanamsha_deg: f64,
    pub config: Option<TaraConfig>,
    pub earth_state: Option<EarthState>,
}

fn resolve_tara_config(
    ctx: &DhruvContext,
    explicit: Option<TaraConfig>,
) -> Result<TaraConfig, DhruvError> {
    if let Some(cfg) = explicit {
        return Ok(cfg);
    }
    if let Some(resolver) = ctx.resolver() {
        return resolver
            .resolve_tara(None)
            .map(|v| v.value)
            .map_err(|e| DhruvError::Config(e.to_string()));
    }
    Ok(TaraConfig::default())
}

fn resolve_upagraha_config(
    ctx: &DhruvContext,
    explicit: Option<TimeUpagrahaConfig>,
) -> Result<TimeUpagrahaConfig, DhruvError> {
    if let Some(cfg) = explicit {
        return Ok(cfg);
    }
    if let Some(resolver) = ctx.resolver() {
        return resolver
            .resolve_full_kundali(None)
            .map(|v| v.value.upagraha_config)
            .map_err(|e| DhruvError::Config(e.to_string()));
    }
    Ok(TimeUpagrahaConfig::default())
}

fn resolve_node_dignity_policy(
    ctx: &DhruvContext,
    explicit: Option<NodeDignityPolicy>,
) -> Result<NodeDignityPolicy, DhruvError> {
    if let Some(policy) = explicit {
        return Ok(policy);
    }
    if let Some(resolver) = ctx.resolver() {
        return resolver
            .resolve_full_kundali(None)
            .map(|v| v.value.node_dignity_policy)
            .map_err(|e| DhruvError::Config(e.to_string()));
    }
    Ok(NodeDignityPolicy::default())
}

fn resolve_full_kundali_config(
    ctx: &DhruvContext,
    explicit: Option<FullKundaliConfig>,
) -> Result<FullKundaliConfig, DhruvError> {
    if let Some(cfg) = explicit {
        return Ok(cfg);
    }
    if let Some(resolver) = ctx.resolver() {
        return resolver
            .resolve_full_kundali(None)
            .map(|v| v.value)
            .map_err(|e| DhruvError::Config(e.to_string()));
    }
    Ok(FullKundaliConfig::default())
}

/// Execute a unified tara operation.
pub fn tara_op(
    ctx: &DhruvContext,
    catalog: &TaraCatalog,
    request: &TaraRequest,
) -> Result<TaraResult, DhruvError> {
    let op = TaraOperation {
        star: request.star,
        output: request.output,
        at_jd_tdb: time_input_to_jd_tdb(ctx, request.at),
        ayanamsha_deg: request.ayanamsha_deg,
        config: resolve_tara_config(ctx, request.config)?,
        earth_state: request.earth_state,
    };
    Ok(dhruv_vedic_ops::tara(catalog, &op)?)
}

/// Unified charakaraka request.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CharakarakaRequest {
    pub at: TimeInput,
    pub system: AyanamshaSystem,
    pub use_nutation: bool,
    pub scheme: CharakarakaScheme,
}

/// Compute Chara Karakas for the requested time.
pub fn charakaraka(
    ctx: &DhruvContext,
    eop: &EopKernel,
    request: &CharakarakaRequest,
) -> Result<CharakarakaResult, DhruvError> {
    let utc = time_input_to_utc_for_context(ctx, request.at);
    let aya_cfg = SankrantiConfig::new(request.system, request.use_nutation);
    Ok(dhruv_vedic_ops::charakaraka_for_date(
        ctx.engine(),
        eop,
        &utc,
        &aya_cfg,
        request.scheme,
    )?)
}

/// Unified upagraha request.
#[derive(Debug, Clone, Copy)]
pub struct UpagrahaRequest {
    pub at: TimeInput,
    pub location: GeoLocation,
    pub riseset_config: Option<RiseSetConfig>,
    pub sankranti_config: Option<SankrantiConfig>,
    pub upagraha_config: Option<TimeUpagrahaConfig>,
}

/// Compute all upagrahas through the context-first request surface.
pub fn upagraha_op(
    ctx: &DhruvContext,
    eop: &EopKernel,
    request: &UpagrahaRequest,
) -> Result<AllUpagrahas, DhruvError> {
    let utc = time_input_to_utc_for_context(ctx, request.at);
    let riseset_config = resolve_riseset_config(ctx, request.riseset_config)?;
    let sankranti_config = resolve_sankranti_config(ctx, request.sankranti_config)?;
    let upagraha_config = resolve_upagraha_config(ctx, request.upagraha_config)?;
    Ok(all_upagrahas_for_date_with_config(
        ctx.engine(),
        eop,
        &utc,
        &request.location,
        &riseset_config,
        &sankranti_config,
        &upagraha_config,
    )?)
}

/// Which avastha output the caller wants.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AvasthaTarget {
    All,
    Graha(Graha),
}

/// Unified avastha request.
#[derive(Debug, Clone, Copy)]
pub struct AvasthaRequest {
    pub at: TimeInput,
    pub location: GeoLocation,
    pub bhava_config: Option<BhavaConfig>,
    pub riseset_config: Option<RiseSetConfig>,
    pub sankranti_config: Option<SankrantiConfig>,
    pub node_policy: Option<NodeDignityPolicy>,
    pub amsha_selection: Option<AmshaSelectionConfig>,
    pub target: AvasthaTarget,
}

/// Avastha result shape selected by [`AvasthaRequest::target`].
#[derive(Debug, Clone, Copy)]
pub enum AvasthaResult {
    All(AllGrahaAvasthas),
    Graha(GrahaAvasthas),
}

/// Compute avastha through the context-first request surface.
pub fn avastha_op(
    ctx: &DhruvContext,
    eop: &EopKernel,
    request: &AvasthaRequest,
) -> Result<AvasthaResult, DhruvError> {
    let utc = time_input_to_utc_for_context(ctx, request.at);
    let bhava_config = resolve_bhava_config(ctx, request.bhava_config)?;
    let riseset_config = resolve_riseset_config(ctx, request.riseset_config)?;
    let sankranti_config = resolve_sankranti_config(ctx, request.sankranti_config)?;
    let node_policy = resolve_node_dignity_policy(ctx, request.node_policy)?;
    let amsha_selection = request.amsha_selection.unwrap_or_default();

    match request.target {
        AvasthaTarget::All => Ok(AvasthaResult::All(avastha_for_date(
            ctx.engine(),
            eop,
            &request.location,
            &utc,
            &bhava_config,
            &riseset_config,
            &sankranti_config,
            node_policy,
            &amsha_selection,
        )?)),
        AvasthaTarget::Graha(graha) => Ok(AvasthaResult::Graha(avastha_for_graha(
            ctx.engine(),
            eop,
            &request.location,
            &utc,
            &bhava_config,
            &riseset_config,
            &sankranti_config,
            node_policy,
            &amsha_selection,
            graha,
        )?)),
    }
}

/// Unified full-kundali request.
#[derive(Debug, Clone, Copy)]
pub struct FullKundaliRequest {
    pub at: TimeInput,
    pub location: GeoLocation,
    pub bhava_config: Option<BhavaConfig>,
    pub riseset_config: Option<RiseSetConfig>,
    pub sankranti_config: Option<SankrantiConfig>,
    pub config: Option<FullKundaliConfig>,
}

/// Compute a full kundali through the context-first request surface.
pub fn full_kundali(
    ctx: &DhruvContext,
    eop: &EopKernel,
    request: &FullKundaliRequest,
) -> Result<FullKundaliResult, DhruvError> {
    let utc = time_input_to_utc_for_context(ctx, request.at);
    let bhava_config = resolve_bhava_config(ctx, request.bhava_config)?;
    let riseset_config = resolve_riseset_config(ctx, request.riseset_config)?;
    let sankranti_config = resolve_sankranti_config(ctx, request.sankranti_config)?;
    let config = resolve_full_kundali_config(ctx, request.config)?;
    Ok(full_kundali_for_date(
        ctx.engine(),
        eop,
        &utc,
        &request.location,
        &bhava_config,
        &riseset_config,
        &sankranti_config,
        &config,
    )?)
}
