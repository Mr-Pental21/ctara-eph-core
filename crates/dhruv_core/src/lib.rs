//! High-level query engine and computation graph.
//!
//! This crate provides the primary [`Engine`] that loads SPK and LSK
//! kernels and evaluates ephemeris queries by chaining SPK segments
//! through the NAIF body hierarchy.

use std::collections::HashMap;
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, RwLock, Weak};
use std::time::{SystemTime, UNIX_EPOCH};

use dhruv_time::{self, LeapSecondKernel};
use jpl_kernel::{KernelError, SpkEvaluation, SpkKernel};

/// Engine configuration used at startup time.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EngineConfig {
    pub spk_paths: Vec<PathBuf>,
    pub lsk_path: PathBuf,
    pub cache_capacity: usize,
    pub strict_validation: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct SpkIdentity {
    path: PathBuf,
    len: u64,
    modified_unix_nanos: Option<i128>,
}

#[derive(Debug, Clone)]
struct LoadedSpk {
    identity: SpkIdentity,
    path: PathBuf,
    kernel: Arc<SpkKernel>,
    segment_count: usize,
}

#[derive(Debug)]
struct SpkSet {
    generation: u64,
    entries: Vec<LoadedSpk>,
}

/// Read-only information about a loaded SPK kernel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadedSpkInfo {
    pub path: PathBuf,
    pub segment_count: usize,
    pub generation: u64,
}

/// Result of replacing an engine's active SPK set.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpkReplaceReport {
    pub generation: u64,
    pub active_count: usize,
    pub loaded_count: usize,
    pub reused_count: usize,
}

impl EngineConfig {
    /// Convenience constructor for single-kernel use.
    pub fn with_single_spk(
        spk_path: PathBuf,
        lsk_path: PathBuf,
        cache_capacity: usize,
        strict_validation: bool,
    ) -> Self {
        Self {
            spk_paths: vec![spk_path],
            lsk_path,
            cache_capacity,
            strict_validation,
        }
    }

    fn validate(&self) -> Result<(), EngineError> {
        if self.spk_paths.is_empty() {
            return Err(EngineError::InvalidConfig("spk_paths must not be empty"));
        }
        for path in &self.spk_paths {
            if path.as_os_str().is_empty() {
                return Err(EngineError::InvalidConfig(
                    "spk_paths must not contain empty paths",
                ));
            }
        }
        if self.lsk_path.as_os_str().is_empty() {
            return Err(EngineError::InvalidConfig("lsk_path must not be empty"));
        }
        if self.cache_capacity == 0 {
            return Err(EngineError::InvalidConfig(
                "cache_capacity must be greater than zero",
            ));
        }
        Ok(())
    }
}

/// Primary bodies supported by the core query contract.
///
/// These are physical bodies that exist as SPK segments in the kernel file.
/// Computed points (e.g. lunar nodes) are NOT included here — they belong
/// in downstream crates like `dhruv_vedic_base` via the `DerivedComputation` trait.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Body {
    Sun,
    Mercury,
    Venus,
    Earth,
    Moon,
    Mars,
    Jupiter,
    Saturn,
    Uranus,
    Neptune,
    Pluto,
}

impl Body {
    /// NAIF-style body code.
    pub const fn code(self) -> i32 {
        match self {
            Self::Sun => 10,
            Self::Mercury => 199,
            Self::Venus => 299,
            Self::Earth => 399,
            Self::Moon => 301,
            Self::Mars => 499,
            Self::Jupiter => 599,
            Self::Saturn => 699,
            Self::Uranus => 799,
            Self::Neptune => 899,
            Self::Pluto => 999,
        }
    }

    /// Convert a NAIF-style body code into a [`Body`].
    pub const fn from_code(code: i32) -> Option<Self> {
        match code {
            10 => Some(Self::Sun),
            199 => Some(Self::Mercury),
            299 => Some(Self::Venus),
            399 => Some(Self::Earth),
            301 => Some(Self::Moon),
            499 => Some(Self::Mars),
            599 => Some(Self::Jupiter),
            699 => Some(Self::Saturn),
            799 => Some(Self::Uranus),
            899 => Some(Self::Neptune),
            999 => Some(Self::Pluto),
            _ => None,
        }
    }
}

/// Observer used to evaluate relative state vectors.
///
/// Topocentric observers (geographic lat/lon/alt) are a higher-level concept
/// built on top of `Body(Earth)` queries — they do not belong in this enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Observer {
    SolarSystemBarycenter,
    Body(Body),
}

impl Observer {
    /// Compact code for FFI interoperability.
    pub const fn code(self) -> i32 {
        match self {
            Self::SolarSystemBarycenter => 0,
            Self::Body(body) => body.code(),
        }
    }

    /// Convert a compact observer code into an [`Observer`].
    pub const fn from_code(code: i32) -> Option<Self> {
        if code == 0 {
            return Some(Self::SolarSystemBarycenter);
        }
        match Body::from_code(code) {
            Some(body) => Some(Self::Body(body)),
            None => None,
        }
    }
}

/// Output reference frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Frame {
    IcrfJ2000,
    EclipticJ2000,
}

impl Frame {
    /// Compact frame code for FFI interoperability.
    pub const fn code(self) -> i32 {
        match self {
            Self::IcrfJ2000 => 0,
            Self::EclipticJ2000 => 1,
        }
    }

    /// Convert a compact frame code into a [`Frame`].
    pub const fn from_code(code: i32) -> Option<Self> {
        match code {
            0 => Some(Self::IcrfJ2000),
            1 => Some(Self::EclipticJ2000),
            _ => None,
        }
    }
}

/// Single ephemeris request.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Query {
    pub target: Body,
    pub observer: Observer,
    pub frame: Frame,
    pub epoch_tdb_jd: f64,
}

/// Cartesian state vector output.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StateVector {
    pub position_km: [f64; 3],
    pub velocity_km_s: [f64; 3],
}

/// Core engine errors.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum EngineError {
    InvalidConfig(&'static str),
    InvalidQuery(&'static str),
    KernelLoad(String),
    TimeConversion(String),
    UnsupportedQuery(&'static str),
    EpochOutOfRange { epoch_tdb_jd: f64 },
    Internal(String),
}

impl Display for EngineError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidConfig(msg) => write!(f, "invalid config: {msg}"),
            Self::InvalidQuery(msg) => write!(f, "invalid query: {msg}"),
            Self::KernelLoad(msg) => write!(f, "kernel load error: {msg}"),
            Self::TimeConversion(msg) => write!(f, "time conversion error: {msg}"),
            Self::UnsupportedQuery(msg) => write!(f, "unsupported query: {msg}"),
            Self::EpochOutOfRange { epoch_tdb_jd } => {
                write!(f, "epoch out of range: {epoch_tdb_jd}")
            }
            Self::Internal(msg) => write!(f, "internal error: {msg}"),
        }
    }
}

impl Error for EngineError {}

/// Output shape for extension-trait computations.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DerivedValue {
    Scalar(f64),
    Vector3([f64; 3]),
}

/// Extension seam that downstream crates can implement without tight coupling.
///
/// This is the primary mechanism for `dhruv_vedic_base` and downstream crates to add
/// derived quantities (ayanamsha, lunar nodes, etc.) without modifying core.
pub trait DerivedComputation: Send + Sync {
    fn name(&self) -> &'static str;
    fn compute(
        &self,
        engine: &Engine,
        query: &Query,
        state: &StateVector,
    ) -> Result<DerivedValue, EngineError>;
}

/// Telemetry from a query or batch of queries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct QueryStats {
    pub evaluations: u32,
    pub cache_hits: u32,
}

/// Per-request memoization context.
///
/// Created at the start of each query/batch, threaded through chain
/// resolution, dropped at the end. Keys use `epoch_tdb_s.to_bits()`
/// because epochs within one request are bit-identical from the same
/// `jd_to_tdb_seconds()` call.
struct ComputationContext {
    /// Cache key: (target, center, epoch_bits) -> evaluation result.
    cache: HashMap<(i32, i32, u64), SpkEvaluation>,
    evaluations: u32,
    cache_hits: u32,
}

impl ComputationContext {
    fn new() -> Self {
        Self {
            cache: HashMap::with_capacity(8),
            evaluations: 0,
            cache_hits: 0,
        }
    }

    fn stats(&self) -> QueryStats {
        QueryStats {
            evaluations: self.evaluations,
            cache_hits: self.cache_hits,
        }
    }
}

fn system_time_to_unix_nanos(time: SystemTime) -> Option<i128> {
    match time.duration_since(UNIX_EPOCH) {
        Ok(duration) => Some(duration.as_nanos() as i128),
        Err(err) => Some(-(err.duration().as_nanos() as i128)),
    }
}

fn spk_identity(path: &Path) -> Result<SpkIdentity, EngineError> {
    let canonical = fs::canonicalize(path).map_err(|e| {
        EngineError::KernelLoad(format!("failed to canonicalize {}: {e}", path.display()))
    })?;
    let metadata = fs::metadata(&canonical).map_err(|e| {
        EngineError::KernelLoad(format!(
            "failed to read metadata for {}: {e}",
            canonical.display()
        ))
    })?;
    let modified_unix_nanos = metadata.modified().ok().and_then(system_time_to_unix_nanos);
    Ok(SpkIdentity {
        path: canonical,
        len: metadata.len(),
        modified_unix_nanos,
    })
}

fn loaded_spk_from_kernel(identity: SpkIdentity, kernel: Arc<SpkKernel>) -> LoadedSpk {
    let segment_count = kernel.segments().len();
    LoadedSpk {
        path: identity.path.clone(),
        identity,
        kernel,
        segment_count,
    }
}

fn spk_infos_from_set(set: &SpkSet) -> Vec<LoadedSpkInfo> {
    set.entries
        .iter()
        .map(|entry| LoadedSpkInfo {
            path: entry.path.clone(),
            segment_count: entry.segment_count,
            generation: set.generation,
        })
        .collect()
}

fn load_spk_set(
    paths: &[PathBuf],
    generation: u64,
    current: Option<&Arc<SpkSet>>,
    cache: Option<&HashMap<SpkIdentity, Weak<SpkKernel>>>,
) -> Result<
    (
        Arc<SpkSet>,
        HashMap<SpkIdentity, Weak<SpkKernel>>,
        usize,
        usize,
    ),
    EngineError,
> {
    let mut entries = Vec::with_capacity(paths.len());
    let mut loaded_count = 0usize;
    let mut reused_count = 0usize;
    let mut fresh_cache = HashMap::with_capacity(paths.len());
    let mut local_kernels: HashMap<SpkIdentity, Arc<SpkKernel>> = HashMap::new();

    for path in paths {
        let identity = spk_identity(path)?;
        let mut kernel = current
            .and_then(|set| {
                set.entries
                    .iter()
                    .find(|entry| entry.identity == identity)
                    .map(|entry| Arc::clone(&entry.kernel))
            })
            .or_else(|| cache.and_then(|cache| cache.get(&identity).and_then(Weak::upgrade)))
            .or_else(|| local_kernels.get(&identity).map(Arc::clone));

        if kernel.is_some() {
            reused_count += 1;
        } else {
            let loaded = Arc::new(SpkKernel::load(&identity.path).map_err(|e| {
                EngineError::KernelLoad(format!("{}: {e}", identity.path.display()))
            })?);
            let after = spk_identity(&identity.path)?;
            if after != identity {
                return Err(EngineError::KernelLoad(format!(
                    "SPK metadata changed while loading {}",
                    identity.path.display()
                )));
            }
            loaded_count += 1;
            kernel = Some(Arc::clone(&loaded));
            local_kernels.insert(identity.clone(), loaded);
        }

        let kernel = kernel.expect("kernel is loaded or reused");
        fresh_cache.insert(identity.clone(), Arc::downgrade(&kernel));
        entries.push(loaded_spk_from_kernel(identity, kernel));
    }

    Ok((
        Arc::new(SpkSet {
            generation,
            entries,
        }),
        fresh_cache,
        loaded_count,
        reused_count,
    ))
}

/// Core query engine.
///
/// `Engine` is [`Send`] + [`Sync`], so it can be shared across threads
/// via `Arc<Engine>`. Each query creates its own short-lived
/// [`ComputationContext`] for memoization — no cross-request locking.
///
/// ```rust,ignore
/// let engine = Arc::new(Engine::new(config)?);
/// // Spawn threads that share the same engine:
/// let handle = std::thread::spawn({
///     let engine = Arc::clone(&engine);
///     move || engine.query(query)
/// });
/// ```
pub struct Engine {
    config: RwLock<EngineConfig>,
    spk_set: RwLock<Arc<SpkSet>>,
    replace_lock: Mutex<()>,
    kernel_cache: Mutex<HashMap<SpkIdentity, Weak<SpkKernel>>>,
    lsk: LeapSecondKernel,
}

// Manual Debug impl since SpkKernel's Debug shows raw data.
impl std::fmt::Debug for Engine {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let config = self.config.read().ok();
        let spk_set = self.spk_set.read().ok();
        let total_segments: usize = spk_set
            .as_ref()
            .map(|set| set.entries.iter().map(|entry| entry.segment_count).sum())
            .unwrap_or_default();
        let spk_kernel_count = spk_set
            .as_ref()
            .map(|set| set.entries.len())
            .unwrap_or_default();
        f.debug_struct("Engine")
            .field("config", &config.as_deref())
            .field("spk_kernel_count", &spk_kernel_count)
            .field("spk_total_segments", &total_segments)
            .finish()
    }
}

impl Engine {
    /// Create a new engine, loading SPK and LSK kernels from the config paths.
    pub fn new(config: EngineConfig) -> Result<Self, EngineError> {
        config.validate()?;
        let (spk_set, cache, _, _) = load_spk_set(&config.spk_paths, 0, None, None)?;
        let lsk = LeapSecondKernel::load(&config.lsk_path)
            .map_err(|e| EngineError::KernelLoad(e.to_string()))?;
        Ok(Self {
            config: RwLock::new(config),
            spk_set: RwLock::new(spk_set),
            replace_lock: Mutex::new(()),
            kernel_cache: Mutex::new(cache),
            lsk,
        })
    }

    pub fn config(&self) -> EngineConfig {
        self.config
            .read()
            .expect("engine config lock poisoned")
            .clone()
    }

    /// Current SPK set generation.
    pub fn spk_generation(&self) -> u64 {
        self.spk_set
            .read()
            .expect("engine SPK lock poisoned")
            .generation
    }

    /// Read information about the currently active SPK kernels.
    pub fn spk_infos(&self) -> Vec<LoadedSpkInfo> {
        let set = self.spk_snapshot();
        spk_infos_from_set(&set)
    }

    /// Replace the active SPK set. The old set remains active if loading fails.
    pub fn replace_spk_paths(
        &self,
        spk_paths: Vec<PathBuf>,
    ) -> Result<SpkReplaceReport, EngineError> {
        if spk_paths.is_empty() {
            return Err(EngineError::InvalidConfig("spk_paths must not be empty"));
        }
        for path in &spk_paths {
            if path.as_os_str().is_empty() {
                return Err(EngineError::InvalidConfig(
                    "spk_paths must not contain empty paths",
                ));
            }
        }

        let _replace_guard = self
            .replace_lock
            .lock()
            .expect("engine SPK replacement lock poisoned");
        let current = self.spk_snapshot();
        let next_generation = current.generation.saturating_add(1);
        let cache_guard = self
            .kernel_cache
            .lock()
            .expect("engine SPK cache lock poisoned");
        let (new_set, mut new_cache_entries, loaded_count, reused_count) = load_spk_set(
            &spk_paths,
            next_generation,
            Some(&current),
            Some(&cache_guard),
        )?;
        drop(cache_guard);

        {
            let mut set_guard = self.spk_set.write().expect("engine SPK lock poisoned");
            *set_guard = Arc::clone(&new_set);
        }
        {
            let mut config_guard = self.config.write().expect("engine config lock poisoned");
            config_guard.spk_paths = spk_paths;
        }
        {
            let mut cache_guard = self
                .kernel_cache
                .lock()
                .expect("engine SPK cache lock poisoned");
            cache_guard.retain(|_, weak| weak.strong_count() > 0);
            cache_guard.extend(new_cache_entries.drain());
        }

        Ok(SpkReplaceReport {
            generation: new_set.generation,
            active_count: new_set.entries.len(),
            loaded_count,
            reused_count,
        })
    }

    /// Access the loaded LSK kernel.
    pub fn lsk(&self) -> &LeapSecondKernel {
        &self.lsk
    }

    fn spk_snapshot(&self) -> Arc<SpkSet> {
        Arc::clone(&self.spk_set.read().expect("engine SPK lock poisoned"))
    }

    /// Evaluate (target, center) at epoch from the first kernel with a
    /// matching segment. Uses the computation context for memoization.
    fn evaluate_across(
        &self,
        spk_set: &SpkSet,
        target: i32,
        center: i32,
        epoch_tdb_s: f64,
        ctx: &mut ComputationContext,
    ) -> Result<SpkEvaluation, KernelError> {
        let key = (target, center, epoch_tdb_s.to_bits());
        if let Some(cached) = ctx.cache.get(&key) {
            ctx.cache_hits += 1;
            return Ok(*cached);
        }

        for entry in &spk_set.entries {
            match entry.kernel.evaluate(target, center, epoch_tdb_s) {
                Ok(eval) => {
                    ctx.evaluations += 1;
                    ctx.cache.insert(key, eval);
                    return Ok(eval);
                }
                Err(KernelError::EpochOutOfRange { .. }) => continue,
                Err(e) => return Err(e),
            }
        }
        Err(KernelError::EpochOutOfRange {
            target,
            center,
            epoch_tdb_s,
        })
    }

    /// Find the center body for a target across all kernels.
    fn center_for_across(&self, spk_set: &SpkSet, target: i32) -> Option<i32> {
        for entry in &spk_set.entries {
            if let Some(center) = entry.kernel.center_for(target) {
                return Some(center);
            }
        }
        None
    }

    /// Resolve a body to SSB (code 0) by walking the segment chain
    /// across all loaded kernels. Uses x99→barycenter fallback.
    fn resolve_to_ssb_across(
        &self,
        spk_set: &SpkSet,
        body_code: i32,
        epoch_tdb_s: f64,
        ctx: &mut ComputationContext,
    ) -> Result<[f64; 6], KernelError> {
        let mut code = body_code;
        let mut state = [0.0f64; 6];

        while code != 0 {
            let center = match self.center_for_across(spk_set, code) {
                Some(c) => c,
                None => {
                    let bary = jpl_kernel::planet_body_to_barycenter(code);
                    if bary != code {
                        code = bary;
                        continue;
                    }
                    return Err(KernelError::SegmentNotFound {
                        target: code,
                        center: -1,
                    });
                }
            };

            let eval = self.evaluate_across(spk_set, code, center, epoch_tdb_s, ctx)?;
            state[0] += eval.position_km[0];
            state[1] += eval.position_km[1];
            state[2] += eval.position_km[2];
            state[3] += eval.velocity_km_s[0];
            state[4] += eval.velocity_km_s[1];
            state[5] += eval.velocity_km_s[2];

            code = center;
        }

        Ok(state)
    }

    /// Evaluate an ephemeris query, returning a Cartesian state vector.
    pub fn query(&self, query: Query) -> Result<StateVector, EngineError> {
        let mut ctx = ComputationContext::new();
        let spk_set = self.spk_snapshot();
        self.query_with_ctx(&spk_set, query, &mut ctx)
    }

    /// Evaluate a query and return telemetry alongside the result.
    pub fn query_with_stats(&self, query: Query) -> Result<(StateVector, QueryStats), EngineError> {
        let mut ctx = ComputationContext::new();
        let spk_set = self.spk_snapshot();
        let state = self.query_with_ctx(&spk_set, query, &mut ctx)?;
        Ok((state, ctx.stats()))
    }

    /// Internal query implementation that threads a memoization context.
    fn query_with_ctx(
        &self,
        spk_set: &SpkSet,
        query: Query,
        ctx: &mut ComputationContext,
    ) -> Result<StateVector, EngineError> {
        if !query.epoch_tdb_jd.is_finite() {
            return Err(EngineError::InvalidQuery("epoch_tdb_jd must be finite"));
        }
        if let Observer::Body(body) = query.observer
            && body == query.target
        {
            return Err(EngineError::UnsupportedQuery(
                "target and observer body cannot be identical",
            ));
        }

        // Convert JD TDB to TDB seconds past J2000.
        let epoch_tdb_s = dhruv_time::jd_to_tdb_seconds(query.epoch_tdb_jd);

        // Resolve target to SSB across all loaded kernels.
        let target_ssb = self
            .resolve_to_ssb_across(spk_set, query.target.code(), epoch_tdb_s, ctx)
            .map_err(|e| EngineError::Internal(e.to_string()))?;

        // Resolve observer to SSB across all loaded kernels.
        let observer_ssb = match query.observer {
            Observer::SolarSystemBarycenter => [0.0f64; 6],
            Observer::Body(body) => self
                .resolve_to_ssb_across(spk_set, body.code(), epoch_tdb_s, ctx)
                .map_err(|e| EngineError::Internal(e.to_string()))?,
        };

        // Subtract observer from target.
        let mut state = StateVector {
            position_km: [
                target_ssb[0] - observer_ssb[0],
                target_ssb[1] - observer_ssb[1],
                target_ssb[2] - observer_ssb[2],
            ],
            velocity_km_s: [
                target_ssb[3] - observer_ssb[3],
                target_ssb[4] - observer_ssb[4],
                target_ssb[5] - observer_ssb[5],
            ],
        };

        // Frame rotation via dhruv_frames.
        if query.frame == Frame::EclipticJ2000 {
            state.position_km = dhruv_frames::icrf_to_ecliptic(&state.position_km);
            state.velocity_km_s = dhruv_frames::icrf_to_ecliptic(&state.velocity_km_s);
        }

        Ok(state)
    }

    /// Evaluate multiple queries, sharing memoization across queries at the
    /// same epoch. Returns results in input order.
    pub fn query_batch(&self, queries: &[Query]) -> Vec<Result<StateVector, EngineError>> {
        self.query_batch_with_stats(queries).0
    }

    /// Evaluate multiple queries with telemetry. Shares memoization across
    /// queries at the same epoch.
    pub fn query_batch_with_stats(
        &self,
        queries: &[Query],
    ) -> (Vec<Result<StateVector, EngineError>>, QueryStats) {
        let mut results: Vec<Result<StateVector, EngineError>> = Vec::with_capacity(queries.len());

        // Group by epoch bits to share context across same-epoch queries.
        // Build index groups, process each group with a shared context,
        // then scatter results back into the output vec.

        // Pre-fill results with placeholder errors.
        results.resize_with(queries.len(), || {
            Err(EngineError::Internal("unprocessed".into()))
        });

        // Collect (epoch_bits, original_index) and sort by epoch to group.
        let mut indexed: Vec<(u64, usize)> = queries
            .iter()
            .enumerate()
            .map(|(i, q)| (q.epoch_tdb_jd.to_bits(), i))
            .collect();
        indexed.sort_unstable_by_key(|(bits, _)| *bits);

        let mut total_stats = QueryStats::default();
        let spk_set = self.spk_snapshot();

        // Process groups of same-epoch queries.
        let mut group_start = 0;
        while group_start < indexed.len() {
            let epoch_bits = indexed[group_start].0;
            let mut group_end = group_start + 1;
            while group_end < indexed.len() && indexed[group_end].0 == epoch_bits {
                group_end += 1;
            }

            let mut ctx = ComputationContext::new();
            for &(_, idx) in &indexed[group_start..group_end] {
                results[idx] = self.query_with_ctx(&spk_set, queries[idx], &mut ctx);
            }

            let group_stats = ctx.stats();
            total_stats.evaluations += group_stats.evaluations;
            total_stats.cache_hits += group_stats.cache_hits;

            group_start = group_end;
        }

        (results, total_stats)
    }

    pub fn query_with_derived<D: DerivedComputation>(
        &self,
        query: Query,
        derived: &D,
    ) -> Result<(StateVector, DerivedValue), EngineError> {
        let state = self.query(query)?;
        let derived_value = derived.compute(self, &query, &state)?;
        Ok((state, derived_value))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kernel_paths() -> (PathBuf, PathBuf) {
        let base = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../kernels/data");
        (base.join("de442s.bsp"), base.join("naif0012.tls"))
    }

    #[test]
    fn engine_rejects_empty_spk_paths() {
        let (_, lsk) = kernel_paths();
        let config = EngineConfig {
            spk_paths: vec![],
            lsk_path: lsk,
            cache_capacity: 256,
            strict_validation: true,
        };
        assert!(matches!(
            Engine::new(config),
            Err(EngineError::InvalidConfig(_))
        ));
    }

    #[test]
    fn engine_rejects_empty_path_in_spk_paths() {
        let (_, lsk) = kernel_paths();
        let config = EngineConfig {
            spk_paths: vec![PathBuf::new()],
            lsk_path: lsk,
            cache_capacity: 256,
            strict_validation: true,
        };
        assert!(matches!(
            Engine::new(config),
            Err(EngineError::InvalidConfig(_))
        ));
    }

    #[test]
    fn engine_rejects_empty_lsk_path() {
        let (spk, _) = kernel_paths();
        let config = EngineConfig {
            spk_paths: vec![spk],
            lsk_path: PathBuf::new(),
            cache_capacity: 256,
            strict_validation: true,
        };
        assert!(matches!(
            Engine::new(config),
            Err(EngineError::InvalidConfig(_))
        ));
    }

    #[test]
    fn engine_rejects_zero_cache() {
        let (spk, lsk) = kernel_paths();
        let config = EngineConfig {
            spk_paths: vec![spk],
            lsk_path: lsk,
            cache_capacity: 0,
            strict_validation: true,
        };
        assert!(matches!(
            Engine::new(config),
            Err(EngineError::InvalidConfig(_))
        ));
    }

    #[test]
    fn with_single_spk_convenience() {
        let (spk, lsk) = kernel_paths();
        let config = EngineConfig::with_single_spk(spk, lsk, 256, true);
        assert_eq!(config.spk_paths.len(), 1);
        assert_eq!(config.cache_capacity, 256);
        assert!(config.strict_validation);
    }

    // Compile-time assertion: Engine must be Send + Sync.
    #[allow(dead_code)]
    const _: () = {
        fn assert_send_sync<T: Send + Sync>() {}
        fn check() {
            assert_send_sync::<Engine>();
        }
    };
}
