//! C-facing adapter types for `ctara-dhruv-core`.

use std::path::PathBuf;
use std::ptr;

use dhruv_core::{Body, Engine, EngineConfig, EngineError, Frame, Observer, Query, StateVector};
use dhruv_search::{
    ConjunctionConfig, ConjunctionEvent, EclipseConfig, LunarEclipse, LunarEclipseType,
    MaxSpeedEvent, MaxSpeedType, SearchError, SolarEclipse, SolarEclipseType, StationaryConfig,
    StationaryEvent, StationType, next_conjunction, next_lunar_eclipse, next_max_speed,
    next_solar_eclipse, next_stationary, prev_conjunction, prev_lunar_eclipse, prev_max_speed,
    prev_solar_eclipse, prev_stationary, search_conjunctions, search_lunar_eclipses,
    search_max_speed, search_solar_eclipses, search_stationary,
};
use dhruv_vedic_base::{
    AyanamshaSystem, BhavaConfig, BhavaReferenceMode, BhavaStartingPoint, BhavaSystem,
    GeoLocation, LunarNode, NodeMode, RiseSetConfig, RiseSetEvent,
    RiseSetResult, SunLimb, VedicError, ayanamsha_deg, ayanamsha_mean_deg, ayanamsha_true_deg,
    approximate_local_noon_jd, compute_all_events, compute_bhavas, compute_rise_set,
    deg_to_dms, jd_tdb_to_centuries, lunar_node_deg, nakshatra28_from_longitude,
    nakshatra28_from_tropical, nakshatra_from_longitude, nakshatra_from_tropical,
    rashi_from_longitude, rashi_from_tropical,
};

/// ABI version for downstream bindings.
pub const DHRUV_API_VERSION: u32 = 9;

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
    Internal = 255,
}

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
        Self::try_new_multi(&[spk_path_utf8], lsk_path_utf8, cache_capacity, strict_validation)
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

/// Opaque engine handle type for ABI consumers.
pub type DhruvEngineHandle = Engine;

/// Opaque LSK handle type for ABI consumers.
pub type DhruvLskHandle = dhruv_time::LeapSecondKernel;

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
/// Writes the resulting JD TDB into `out_jd_tdb`.
///
/// # Safety
/// `lsk` and `out_jd_tdb` must be valid, non-null pointers.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dhruv_utc_to_tdb_jd(
    lsk: *const DhruvLskHandle,
    year: i32,
    month: u32,
    day: u32,
    hour: u32,
    min: u32,
    sec: f64,
    out_jd_tdb: *mut f64,
) -> DhruvStatus {
    ffi_boundary(|| {
        if lsk.is_null() || out_jd_tdb.is_null() {
            return DhruvStatus::NullPointer;
        }

        // SAFETY: Pointer is checked for null above.
        let lsk_ref = unsafe { &*lsk };

        let epoch = dhruv_time::Epoch::from_utc(year, month, day, hour, min, sec, lsk_ref);

        // SAFETY: Pointer is checked for null above; write one value.
        unsafe { *out_jd_tdb = epoch.as_jd_tdb() };
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

/// Query engine with UTC input, return spherical state with angular velocities.
#[allow(clippy::too_many_arguments)]
pub fn dhruv_query_utc_spherical_internal(
    engine: &Engine,
    target_code: i32,
    observer_code: i32,
    frame_code: i32,
    year: i32,
    month: u32,
    day: u32,
    hour: u32,
    min: u32,
    sec: f64,
) -> Result<DhruvSphericalState, DhruvStatus> {
    let target = Body::from_code(target_code)
        .ok_or(DhruvStatus::InvalidQuery)?;
    let observer = Observer::from_code(observer_code)
        .ok_or(DhruvStatus::InvalidQuery)?;
    let frame = Frame::from_code(frame_code)
        .ok_or(DhruvStatus::InvalidQuery)?;

    let epoch = dhruv_time::Epoch::from_utc(year, month, day, hour, min, sec, engine.lsk());

    let query = Query {
        target,
        observer,
        frame,
        epoch_tdb_jd: epoch.as_jd_tdb(),
    };

    let state = engine.query(query).map_err(|e| DhruvStatus::from(&e))?;

    let ss = dhruv_frames::cartesian_state_to_spherical_state(
        &state.position_km,
        &state.velocity_km_s,
    );

    Ok(DhruvSphericalState {
        lon_deg: ss.lon_deg,
        lat_deg: ss.lat_deg,
        distance_km: ss.distance_km,
        lon_speed: ss.lon_speed,
        lat_speed: ss.lat_speed,
        distance_speed: ss.distance_speed,
    })
}

/// Query engine from UTC calendar date, return spherical state with angular velocities.
///
/// Combines UTC→TDB conversion, Cartesian query, and spherical state conversion
/// in a single call.
///
/// # Safety
/// `engine` and `out` must be valid, non-null pointers.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dhruv_query_utc_spherical(
    engine: *const DhruvEngineHandle,
    target: i32,
    observer: i32,
    frame: i32,
    year: i32,
    month: u32,
    day: u32,
    hour: u32,
    min: u32,
    sec: f64,
    out: *mut DhruvSphericalState,
) -> DhruvStatus {
    ffi_boundary(|| {
        if engine.is_null() || out.is_null() {
            return DhruvStatus::NullPointer;
        }

        // SAFETY: Pointer is checked for null above.
        let engine_ref = unsafe { &*engine };

        match dhruv_query_utc_spherical_internal(
            engine_ref, target, observer, frame, year, month, day, hour, min, sec,
        ) {
            Ok(state) => {
                // SAFETY: Pointer is checked for null above; write one struct.
                unsafe { *out = state };
                DhruvStatus::Ok
            }
            Err(status) => status,
        }
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

/// Mean ayanamsha at a JD TDB. Pure math, no engine needed.
///
/// # Safety
/// `out_deg` must be a valid, non-null pointer.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dhruv_ayanamsha_mean_deg(
    system_code: i32,
    jd_tdb: f64,
    out_deg: *mut f64,
) -> DhruvStatus {
    ffi_boundary(|| {
        if out_deg.is_null() {
            return DhruvStatus::NullPointer;
        }

        let system = match ayanamsha_system_from_code(system_code) {
            Some(s) => s,
            None => return DhruvStatus::InvalidQuery,
        };

        let t = jd_tdb_to_centuries(jd_tdb);
        let deg = ayanamsha_mean_deg(system, t);

        // SAFETY: Pointer is checked for null; write one value.
        unsafe { *out_deg = deg };
        DhruvStatus::Ok
    })
}

/// True (nutation-corrected) ayanamsha at a JD TDB.
///
/// For TrueLahiri, adds delta_psi to mean value. For all others, returns mean.
///
/// # Safety
/// `out_deg` must be a valid, non-null pointer.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dhruv_ayanamsha_true_deg(
    system_code: i32,
    jd_tdb: f64,
    delta_psi_arcsec: f64,
    out_deg: *mut f64,
) -> DhruvStatus {
    ffi_boundary(|| {
        if out_deg.is_null() {
            return DhruvStatus::NullPointer;
        }

        let system = match ayanamsha_system_from_code(system_code) {
            Some(s) => s,
            None => return DhruvStatus::InvalidQuery,
        };

        let t = jd_tdb_to_centuries(jd_tdb);
        let deg = ayanamsha_true_deg(system, t, delta_psi_arcsec);

        // SAFETY: Pointer is checked for null; write one value.
        unsafe { *out_deg = deg };
        DhruvStatus::Ok
    })
}

/// Number of supported ayanamsha systems.
#[unsafe(no_mangle)]
pub extern "C" fn dhruv_ayanamsha_system_count() -> u32 {
    AyanamshaSystem::all().len() as u32
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
            || config.is_null()
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
        let cfg_ref = unsafe { &*config };

        let geo = GeoLocation::new(loc_ref.latitude_deg, loc_ref.longitude_deg, loc_ref.altitude_m);
        let sun_limb = match sun_limb_from_code(cfg_ref.sun_limb) {
            Some(l) => l,
            None => return DhruvStatus::InvalidQuery,
        };
        let rs_config = RiseSetConfig {
            use_refraction: cfg_ref.use_refraction != 0,
            sun_limb,
            altitude_correction: cfg_ref.altitude_correction != 0,
        };

        match compute_rise_set(engine_ref, lsk_ref, eop_ref, &geo, event, jd_utc_noon, &rs_config) {
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
            || config.is_null()
            || out_results.is_null()
        {
            return DhruvStatus::NullPointer;
        }

        // SAFETY: All pointers checked for null above.
        let engine_ref = unsafe { &*engine };
        let lsk_ref = unsafe { &*lsk };
        let eop_ref = unsafe { &*eop };
        let loc_ref = unsafe { &*location };
        let cfg_ref = unsafe { &*config };

        let geo = GeoLocation::new(loc_ref.latitude_deg, loc_ref.longitude_deg, loc_ref.altitude_m);
        let sun_limb = match sun_limb_from_code(cfg_ref.sun_limb) {
            Some(l) => l,
            None => return DhruvStatus::InvalidQuery,
        };
        let rs_config = RiseSetConfig {
            use_refraction: cfg_ref.use_refraction != 0,
            sun_limb,
            altitude_correction: cfg_ref.altitude_correction != 0,
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
pub extern "C" fn dhruv_approximate_local_noon_jd(
    jd_ut_midnight: f64,
    longitude_deg: f64,
) -> f64 {
    approximate_local_noon_jd(jd_ut_midnight, longitude_deg)
}

// ---------------------------------------------------------------------------
// Unified ayanamsha + standalone nutation
// ---------------------------------------------------------------------------

/// Unified ayanamsha computation. Computes nutation internally when needed.
///
/// When `use_nutation` is non-zero and the system uses the true equinox
/// (TrueLahiri), nutation in longitude is computed via IAU 2000B and applied.
///
/// # Safety
/// `out_deg` must be a valid, non-null pointer.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dhruv_ayanamsha_deg(
    system_code: i32,
    jd_tdb: f64,
    use_nutation: u8,
    out_deg: *mut f64,
) -> DhruvStatus {
    ffi_boundary(|| {
        if out_deg.is_null() {
            return DhruvStatus::NullPointer;
        }

        let system = match ayanamsha_system_from_code(system_code) {
            Some(s) => s,
            None => return DhruvStatus::InvalidQuery,
        };

        let t = jd_tdb_to_centuries(jd_tdb);
        let deg = ayanamsha_deg(system, t, use_nutation != 0);

        // SAFETY: Pointer is checked for null; write one value.
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

/// Starting point: use the Ascendant.
pub const DHRUV_BHAVA_START_ASCENDANT: i32 = -1;
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
    /// Starting point: -1=Ascendant, -2=custom deg, or positive NAIF body code.
    pub starting_point: i32,
    /// Custom ecliptic degree, used only when starting_point == -2.
    pub custom_start_deg: f64,
    /// Reference mode: 0=start of first, 1=middle of first.
    pub reference_mode: i32,
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
    pub ascendant_deg: f64,
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
        DHRUV_BHAVA_START_ASCENDANT => BhavaStartingPoint::Ascendant,
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

/// Returns default bhava configuration (Equal, Ascendant, StartOfFirst).
#[unsafe(no_mangle)]
pub extern "C" fn dhruv_bhava_config_default() -> DhruvBhavaConfig {
    DhruvBhavaConfig {
        system: DHRUV_BHAVA_EQUAL,
        starting_point: DHRUV_BHAVA_START_ASCENDANT,
        custom_start_deg: 0.0,
        reference_mode: DHRUV_BHAVA_REF_START,
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
            || config.is_null()
            || out_result.is_null()
        {
            return DhruvStatus::NullPointer;
        }

        // SAFETY: All pointers checked for null above.
        let engine_ref = unsafe { &*engine };
        let lsk_ref = unsafe { &*lsk };
        let eop_ref = unsafe { &*eop };
        let loc_ref = unsafe { &*location };
        let cfg_ref = unsafe { &*config };

        let geo = GeoLocation::new(
            loc_ref.latitude_deg,
            loc_ref.longitude_deg,
            loc_ref.altitude_m,
        );

        let rust_config = match bhava_config_from_ffi(cfg_ref) {
            Ok(c) => c,
            Err(status) => return status,
        };

        match compute_bhavas(engine_ref, lsk_ref, eop_ref, &geo, jd_utc, &rust_config) {
            Ok(result) => {
                let mut ffi_bhavas = [DhruvBhava {
                    number: 0,
                    cusp_deg: 0.0,
                    start_deg: 0.0,
                    end_deg: 0.0,
                }; 12];
                for (i, b) in result.bhavas.iter().enumerate() {
                    ffi_bhavas[i] = DhruvBhava {
                        number: b.number,
                        cusp_deg: b.cusp_deg,
                        start_deg: b.start_deg,
                        end_deg: b.end_deg,
                    };
                }
                // SAFETY: Pointer checked for null.
                unsafe {
                    *out_result = DhruvBhavaResult {
                        bhavas: ffi_bhavas,
                        ascendant_deg: result.ascendant_deg,
                        mc_deg: result.mc_deg,
                    };
                }
                DhruvStatus::Ok
            }
            Err(e) => DhruvStatus::from(&e),
        }
    })
}

/// Compute the Ascendant ecliptic longitude in degrees.
///
/// Requires LSK, EOP, and location (no engine needed).
///
/// # Safety
/// All pointer arguments must be valid and non-null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dhruv_ascendant_deg(
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

        match dhruv_vedic_base::ascendant_longitude_rad(lsk_ref, eop_ref, &geo, jd_utc) {
            Ok(rad) => {
                // SAFETY: Pointer checked for null.
                unsafe { *out_deg = rad.to_degrees() };
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

// ---------------------------------------------------------------------------
// Lunar node constants
// ---------------------------------------------------------------------------

/// Node code: Rahu (ascending node).
pub const DHRUV_NODE_RAHU: i32 = 0;
/// Node code: Ketu (descending node).
pub const DHRUV_NODE_KETU: i32 = 1;

/// Node mode: mean (polynomial only).
pub const DHRUV_NODE_MODE_MEAN: i32 = 0;
/// Node mode: true (mean + perturbation corrections).
pub const DHRUV_NODE_MODE_TRUE: i32 = 1;

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
/// Pure math, no engine needed.
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

/// Find the next conjunction/aspect event after `jd_tdb`.
///
/// Returns Ok with `found` set to 1 if an event was found, 0 otherwise.
///
/// # Safety
/// All pointer arguments must be valid and non-null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dhruv_next_conjunction(
    engine: *const DhruvEngineHandle,
    body1_code: i32,
    body2_code: i32,
    jd_tdb: f64,
    config: *const DhruvConjunctionConfig,
    out_event: *mut DhruvConjunctionEvent,
    out_found: *mut u8,
) -> DhruvStatus {
    ffi_boundary(|| {
        if engine.is_null() || config.is_null() || out_event.is_null() || out_found.is_null() {
            return DhruvStatus::NullPointer;
        }

        let body1 = match Body::from_code(body1_code) {
            Some(b) => b,
            None => return DhruvStatus::InvalidQuery,
        };
        let body2 = match Body::from_code(body2_code) {
            Some(b) => b,
            None => return DhruvStatus::InvalidQuery,
        };

        // SAFETY: All pointers checked for null above.
        let engine_ref = unsafe { &*engine };
        let cfg_ref = unsafe { &*config };
        let rust_config = conjunction_config_from_ffi(cfg_ref);

        match next_conjunction(engine_ref, body1, body2, jd_tdb, &rust_config) {
            Ok(Some(event)) => {
                // SAFETY: Pointers checked for null.
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
    })
}

/// Find the previous conjunction/aspect event before `jd_tdb`.
///
/// # Safety
/// All pointer arguments must be valid and non-null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dhruv_prev_conjunction(
    engine: *const DhruvEngineHandle,
    body1_code: i32,
    body2_code: i32,
    jd_tdb: f64,
    config: *const DhruvConjunctionConfig,
    out_event: *mut DhruvConjunctionEvent,
    out_found: *mut u8,
) -> DhruvStatus {
    ffi_boundary(|| {
        if engine.is_null() || config.is_null() || out_event.is_null() || out_found.is_null() {
            return DhruvStatus::NullPointer;
        }

        let body1 = match Body::from_code(body1_code) {
            Some(b) => b,
            None => return DhruvStatus::InvalidQuery,
        };
        let body2 = match Body::from_code(body2_code) {
            Some(b) => b,
            None => return DhruvStatus::InvalidQuery,
        };

        // SAFETY: All pointers checked for null above.
        let engine_ref = unsafe { &*engine };
        let cfg_ref = unsafe { &*config };
        let rust_config = conjunction_config_from_ffi(cfg_ref);

        match prev_conjunction(engine_ref, body1, body2, jd_tdb, &rust_config) {
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
    })
}

/// Search for all conjunction/aspect events in a time range.
///
/// Caller provides `out_events` pointing to an array of at least `max_count`
/// elements. The actual number found is written to `out_count`.
///
/// # Safety
/// All pointer arguments must be valid and non-null.
/// `out_events` must point to at least `max_count` contiguous `DhruvConjunctionEvent`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dhruv_search_conjunctions(
    engine: *const DhruvEngineHandle,
    body1_code: i32,
    body2_code: i32,
    jd_start: f64,
    jd_end: f64,
    config: *const DhruvConjunctionConfig,
    out_events: *mut DhruvConjunctionEvent,
    max_count: u32,
    out_count: *mut u32,
) -> DhruvStatus {
    ffi_boundary(|| {
        if engine.is_null()
            || config.is_null()
            || out_events.is_null()
            || out_count.is_null()
        {
            return DhruvStatus::NullPointer;
        }

        let body1 = match Body::from_code(body1_code) {
            Some(b) => b,
            None => return DhruvStatus::InvalidQuery,
        };
        let body2 = match Body::from_code(body2_code) {
            Some(b) => b,
            None => return DhruvStatus::InvalidQuery,
        };

        // SAFETY: All pointers checked for null above.
        let engine_ref = unsafe { &*engine };
        let cfg_ref = unsafe { &*config };
        let rust_config = conjunction_config_from_ffi(cfg_ref);

        match search_conjunctions(engine_ref, body1, body2, jd_start, jd_end, &rust_config) {
            Ok(events) => {
                let count = events.len().min(max_count as usize);
                // SAFETY: out_events points to at least max_count elements.
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
    })
}

// ---------------------------------------------------------------------------
// Eclipse search
// ---------------------------------------------------------------------------

/// Sentinel value for absent optional JD fields in eclipse results.
pub const DHRUV_JD_ABSENT: f64 = -1.0;

/// Lunar eclipse type: penumbral only.
pub const DHRUV_LUNAR_ECLIPSE_PENUMBRAL: i32 = 0;
/// Lunar eclipse type: partial (umbral).
pub const DHRUV_LUNAR_ECLIPSE_PARTIAL: i32 = 1;
/// Lunar eclipse type: total.
pub const DHRUV_LUNAR_ECLIPSE_TOTAL: i32 = 2;

/// Solar eclipse type: partial.
pub const DHRUV_SOLAR_ECLIPSE_PARTIAL: i32 = 0;
/// Solar eclipse type: annular.
pub const DHRUV_SOLAR_ECLIPSE_ANNULAR: i32 = 1;
/// Solar eclipse type: total.
pub const DHRUV_SOLAR_ECLIPSE_TOTAL: i32 = 2;
/// Solar eclipse type: hybrid.
pub const DHRUV_SOLAR_ECLIPSE_HYBRID: i32 = 3;

/// C-compatible eclipse search configuration.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DhruvEclipseConfig {
    /// Include penumbral-only lunar eclipses: 1 = yes, 0 = no.
    pub include_penumbral: u8,
    /// Include ecliptic latitude and angular separation at peak: 1 = yes, 0 = no.
    pub include_peak_details: u8,
}

/// Returns default eclipse configuration.
#[unsafe(no_mangle)]
pub extern "C" fn dhruv_eclipse_config_default() -> DhruvEclipseConfig {
    DhruvEclipseConfig {
        include_penumbral: 1,
        include_peak_details: 1,
    }
}

fn eclipse_config_from_ffi(cfg: &DhruvEclipseConfig) -> EclipseConfig {
    EclipseConfig {
        include_penumbral: cfg.include_penumbral != 0,
        include_peak_details: cfg.include_peak_details != 0,
    }
}

fn lunar_eclipse_type_to_code(t: LunarEclipseType) -> i32 {
    match t {
        LunarEclipseType::Penumbral => DHRUV_LUNAR_ECLIPSE_PENUMBRAL,
        LunarEclipseType::Partial => DHRUV_LUNAR_ECLIPSE_PARTIAL,
        LunarEclipseType::Total => DHRUV_LUNAR_ECLIPSE_TOTAL,
    }
}

fn solar_eclipse_type_to_code(t: SolarEclipseType) -> i32 {
    match t {
        SolarEclipseType::Partial => DHRUV_SOLAR_ECLIPSE_PARTIAL,
        SolarEclipseType::Annular => DHRUV_SOLAR_ECLIPSE_ANNULAR,
        SolarEclipseType::Total => DHRUV_SOLAR_ECLIPSE_TOTAL,
        SolarEclipseType::Hybrid => DHRUV_SOLAR_ECLIPSE_HYBRID,
    }
}

fn option_jd(opt: Option<f64>) -> f64 {
    opt.unwrap_or(DHRUV_JD_ABSENT)
}

/// C-compatible lunar eclipse result.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DhruvLunarEclipseResult {
    /// Eclipse type code (see DHRUV_LUNAR_ECLIPSE_* constants).
    pub eclipse_type: i32,
    /// Umbral magnitude.
    pub magnitude: f64,
    /// Penumbral magnitude.
    pub penumbral_magnitude: f64,
    /// Time of greatest eclipse (JD TDB).
    pub greatest_eclipse_jd: f64,
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
    /// Moon's ecliptic latitude at greatest eclipse, in degrees.
    pub moon_ecliptic_lat_deg: f64,
    /// Angular separation at greatest eclipse, in degrees.
    pub angular_separation_deg: f64,
}

impl From<&LunarEclipse> for DhruvLunarEclipseResult {
    fn from(e: &LunarEclipse) -> Self {
        Self {
            eclipse_type: lunar_eclipse_type_to_code(e.eclipse_type),
            magnitude: e.magnitude,
            penumbral_magnitude: e.penumbral_magnitude,
            greatest_eclipse_jd: e.greatest_eclipse_jd,
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

/// C-compatible solar eclipse result.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DhruvSolarEclipseResult {
    /// Eclipse type code (see DHRUV_SOLAR_ECLIPSE_* constants).
    pub eclipse_type: i32,
    /// Magnitude: ratio of apparent Moon diameter to Sun diameter.
    pub magnitude: f64,
    /// Time of greatest eclipse (JD TDB).
    pub greatest_eclipse_jd: f64,
    /// C1: First external contact (JD TDB). -1.0 if absent.
    pub c1_jd: f64,
    /// C2: First internal contact (JD TDB). -1.0 if absent.
    pub c2_jd: f64,
    /// C3: Last internal contact (JD TDB). -1.0 if absent.
    pub c3_jd: f64,
    /// C4: Last external contact (JD TDB). -1.0 if absent.
    pub c4_jd: f64,
    /// Moon's ecliptic latitude at greatest eclipse, in degrees.
    pub moon_ecliptic_lat_deg: f64,
    /// Angular separation at greatest eclipse, in degrees.
    pub angular_separation_deg: f64,
}

impl From<&SolarEclipse> for DhruvSolarEclipseResult {
    fn from(e: &SolarEclipse) -> Self {
        Self {
            eclipse_type: solar_eclipse_type_to_code(e.eclipse_type),
            magnitude: e.magnitude,
            greatest_eclipse_jd: e.greatest_eclipse_jd,
            c1_jd: option_jd(e.c1_jd),
            c2_jd: option_jd(e.c2_jd),
            c3_jd: option_jd(e.c3_jd),
            c4_jd: option_jd(e.c4_jd),
            moon_ecliptic_lat_deg: e.moon_ecliptic_lat_deg,
            angular_separation_deg: e.angular_separation_deg,
        }
    }
}

// ---------------------------------------------------------------------------
// Lunar eclipse FFI functions
// ---------------------------------------------------------------------------

/// Find the next lunar eclipse after `jd_tdb`.
///
/// # Safety
/// All pointer arguments must be valid and non-null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dhruv_next_lunar_eclipse(
    engine: *const DhruvEngineHandle,
    jd_tdb: f64,
    config: *const DhruvEclipseConfig,
    out_result: *mut DhruvLunarEclipseResult,
    out_found: *mut u8,
) -> DhruvStatus {
    ffi_boundary(|| {
        if engine.is_null() || config.is_null() || out_result.is_null() || out_found.is_null() {
            return DhruvStatus::NullPointer;
        }

        // SAFETY: All pointers checked for null above.
        let engine_ref = unsafe { &*engine };
        let cfg_ref = unsafe { &*config };
        let rust_config = eclipse_config_from_ffi(cfg_ref);

        match next_lunar_eclipse(engine_ref, jd_tdb, &rust_config) {
            Ok(Some(eclipse)) => {
                unsafe {
                    *out_result = DhruvLunarEclipseResult::from(&eclipse);
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
    })
}

/// Find the previous lunar eclipse before `jd_tdb`.
///
/// # Safety
/// All pointer arguments must be valid and non-null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dhruv_prev_lunar_eclipse(
    engine: *const DhruvEngineHandle,
    jd_tdb: f64,
    config: *const DhruvEclipseConfig,
    out_result: *mut DhruvLunarEclipseResult,
    out_found: *mut u8,
) -> DhruvStatus {
    ffi_boundary(|| {
        if engine.is_null() || config.is_null() || out_result.is_null() || out_found.is_null() {
            return DhruvStatus::NullPointer;
        }

        let engine_ref = unsafe { &*engine };
        let cfg_ref = unsafe { &*config };
        let rust_config = eclipse_config_from_ffi(cfg_ref);

        match prev_lunar_eclipse(engine_ref, jd_tdb, &rust_config) {
            Ok(Some(eclipse)) => {
                unsafe {
                    *out_result = DhruvLunarEclipseResult::from(&eclipse);
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
    })
}

/// Search for all lunar eclipses in a time range.
///
/// # Safety
/// All pointer arguments must be valid and non-null.
/// `out_results` must point to at least `max_count` contiguous `DhruvLunarEclipseResult`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dhruv_search_lunar_eclipses(
    engine: *const DhruvEngineHandle,
    jd_start: f64,
    jd_end: f64,
    config: *const DhruvEclipseConfig,
    out_results: *mut DhruvLunarEclipseResult,
    max_count: u32,
    out_count: *mut u32,
) -> DhruvStatus {
    ffi_boundary(|| {
        if engine.is_null()
            || config.is_null()
            || out_results.is_null()
            || out_count.is_null()
        {
            return DhruvStatus::NullPointer;
        }

        let engine_ref = unsafe { &*engine };
        let cfg_ref = unsafe { &*config };
        let rust_config = eclipse_config_from_ffi(cfg_ref);

        match search_lunar_eclipses(engine_ref, jd_start, jd_end, &rust_config) {
            Ok(eclipses) => {
                let count = eclipses.len().min(max_count as usize);
                let out_slice = unsafe {
                    std::slice::from_raw_parts_mut(out_results, max_count as usize)
                };
                for (i, e) in eclipses.iter().take(count).enumerate() {
                    out_slice[i] = DhruvLunarEclipseResult::from(e);
                }
                unsafe { *out_count = count as u32 };
                DhruvStatus::Ok
            }
            Err(e) => DhruvStatus::from(&e),
        }
    })
}

// ---------------------------------------------------------------------------
// Solar eclipse FFI functions
// ---------------------------------------------------------------------------

/// Find the next solar eclipse after `jd_tdb`.
///
/// # Safety
/// All pointer arguments must be valid and non-null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dhruv_next_solar_eclipse(
    engine: *const DhruvEngineHandle,
    jd_tdb: f64,
    config: *const DhruvEclipseConfig,
    out_result: *mut DhruvSolarEclipseResult,
    out_found: *mut u8,
) -> DhruvStatus {
    ffi_boundary(|| {
        if engine.is_null() || config.is_null() || out_result.is_null() || out_found.is_null() {
            return DhruvStatus::NullPointer;
        }

        let engine_ref = unsafe { &*engine };
        let cfg_ref = unsafe { &*config };
        let rust_config = eclipse_config_from_ffi(cfg_ref);

        match next_solar_eclipse(engine_ref, jd_tdb, &rust_config) {
            Ok(Some(eclipse)) => {
                unsafe {
                    *out_result = DhruvSolarEclipseResult::from(&eclipse);
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
    })
}

/// Find the previous solar eclipse before `jd_tdb`.
///
/// # Safety
/// All pointer arguments must be valid and non-null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dhruv_prev_solar_eclipse(
    engine: *const DhruvEngineHandle,
    jd_tdb: f64,
    config: *const DhruvEclipseConfig,
    out_result: *mut DhruvSolarEclipseResult,
    out_found: *mut u8,
) -> DhruvStatus {
    ffi_boundary(|| {
        if engine.is_null() || config.is_null() || out_result.is_null() || out_found.is_null() {
            return DhruvStatus::NullPointer;
        }

        let engine_ref = unsafe { &*engine };
        let cfg_ref = unsafe { &*config };
        let rust_config = eclipse_config_from_ffi(cfg_ref);

        match prev_solar_eclipse(engine_ref, jd_tdb, &rust_config) {
            Ok(Some(eclipse)) => {
                unsafe {
                    *out_result = DhruvSolarEclipseResult::from(&eclipse);
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
    })
}

/// Search for all solar eclipses in a time range.
///
/// # Safety
/// All pointer arguments must be valid and non-null.
/// `out_results` must point to at least `max_count` contiguous `DhruvSolarEclipseResult`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dhruv_search_solar_eclipses(
    engine: *const DhruvEngineHandle,
    jd_start: f64,
    jd_end: f64,
    config: *const DhruvEclipseConfig,
    out_results: *mut DhruvSolarEclipseResult,
    max_count: u32,
    out_count: *mut u32,
) -> DhruvStatus {
    ffi_boundary(|| {
        if engine.is_null()
            || config.is_null()
            || out_results.is_null()
            || out_count.is_null()
        {
            return DhruvStatus::NullPointer;
        }

        let engine_ref = unsafe { &*engine };
        let cfg_ref = unsafe { &*config };
        let rust_config = eclipse_config_from_ffi(cfg_ref);

        match search_solar_eclipses(engine_ref, jd_start, jd_end, &rust_config) {
            Ok(eclipses) => {
                let count = eclipses.len().min(max_count as usize);
                let out_slice = unsafe {
                    std::slice::from_raw_parts_mut(out_results, max_count as usize)
                };
                for (i, e) in eclipses.iter().take(count).enumerate() {
                    out_slice[i] = DhruvSolarEclipseResult::from(e);
                }
                unsafe { *out_count = count as u32 };
                DhruvStatus::Ok
            }
            Err(e) => DhruvStatus::from(&e),
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

/// Find the next stationary point after `jd_tdb`.
///
/// # Safety
/// All pointer arguments must be valid and non-null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dhruv_next_stationary(
    engine: *const DhruvEngineHandle,
    body_code: i32,
    jd_tdb: f64,
    config: *const DhruvStationaryConfig,
    out_event: *mut DhruvStationaryEvent,
    out_found: *mut u8,
) -> DhruvStatus {
    ffi_boundary(|| {
        if engine.is_null() || config.is_null() || out_event.is_null() || out_found.is_null() {
            return DhruvStatus::NullPointer;
        }

        let body = match Body::from_code(body_code) {
            Some(b) => b,
            None => return DhruvStatus::InvalidQuery,
        };

        let engine_ref = unsafe { &*engine };
        let cfg_ref = unsafe { &*config };
        let rust_config = stationary_config_from_ffi(cfg_ref);

        match next_stationary(engine_ref, body, jd_tdb, &rust_config) {
            Ok(Some(event)) => {
                unsafe {
                    *out_event = DhruvStationaryEvent::from(&event);
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
    })
}

/// Find the previous stationary point before `jd_tdb`.
///
/// # Safety
/// All pointer arguments must be valid and non-null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dhruv_prev_stationary(
    engine: *const DhruvEngineHandle,
    body_code: i32,
    jd_tdb: f64,
    config: *const DhruvStationaryConfig,
    out_event: *mut DhruvStationaryEvent,
    out_found: *mut u8,
) -> DhruvStatus {
    ffi_boundary(|| {
        if engine.is_null() || config.is_null() || out_event.is_null() || out_found.is_null() {
            return DhruvStatus::NullPointer;
        }

        let body = match Body::from_code(body_code) {
            Some(b) => b,
            None => return DhruvStatus::InvalidQuery,
        };

        let engine_ref = unsafe { &*engine };
        let cfg_ref = unsafe { &*config };
        let rust_config = stationary_config_from_ffi(cfg_ref);

        match prev_stationary(engine_ref, body, jd_tdb, &rust_config) {
            Ok(Some(event)) => {
                unsafe {
                    *out_event = DhruvStationaryEvent::from(&event);
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
    })
}

/// Search for all stationary points in a time range.
///
/// # Safety
/// All pointer arguments must be valid and non-null.
/// `out_events` must point to at least `max_count` contiguous `DhruvStationaryEvent`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dhruv_search_stationary(
    engine: *const DhruvEngineHandle,
    body_code: i32,
    jd_start: f64,
    jd_end: f64,
    config: *const DhruvStationaryConfig,
    out_events: *mut DhruvStationaryEvent,
    max_count: u32,
    out_count: *mut u32,
) -> DhruvStatus {
    ffi_boundary(|| {
        if engine.is_null()
            || config.is_null()
            || out_events.is_null()
            || out_count.is_null()
        {
            return DhruvStatus::NullPointer;
        }

        let body = match Body::from_code(body_code) {
            Some(b) => b,
            None => return DhruvStatus::InvalidQuery,
        };

        let engine_ref = unsafe { &*engine };
        let cfg_ref = unsafe { &*config };
        let rust_config = stationary_config_from_ffi(cfg_ref);

        match search_stationary(engine_ref, body, jd_start, jd_end, &rust_config) {
            Ok(events) => {
                let count = events.len().min(max_count as usize);
                let out_slice = unsafe {
                    std::slice::from_raw_parts_mut(out_events, max_count as usize)
                };
                for (i, e) in events.iter().take(count).enumerate() {
                    out_slice[i] = DhruvStationaryEvent::from(e);
                }
                unsafe { *out_count = count as u32 };
                DhruvStatus::Ok
            }
            Err(e) => DhruvStatus::from(&e),
        }
    })
}

/// Find the next max-speed event after `jd_tdb`.
///
/// # Safety
/// All pointer arguments must be valid and non-null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dhruv_next_max_speed(
    engine: *const DhruvEngineHandle,
    body_code: i32,
    jd_tdb: f64,
    config: *const DhruvStationaryConfig,
    out_event: *mut DhruvMaxSpeedEvent,
    out_found: *mut u8,
) -> DhruvStatus {
    ffi_boundary(|| {
        if engine.is_null() || config.is_null() || out_event.is_null() || out_found.is_null() {
            return DhruvStatus::NullPointer;
        }

        let body = match Body::from_code(body_code) {
            Some(b) => b,
            None => return DhruvStatus::InvalidQuery,
        };

        let engine_ref = unsafe { &*engine };
        let cfg_ref = unsafe { &*config };
        let rust_config = stationary_config_from_ffi(cfg_ref);

        match next_max_speed(engine_ref, body, jd_tdb, &rust_config) {
            Ok(Some(event)) => {
                unsafe {
                    *out_event = DhruvMaxSpeedEvent::from(&event);
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
    })
}

/// Find the previous max-speed event before `jd_tdb`.
///
/// # Safety
/// All pointer arguments must be valid and non-null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dhruv_prev_max_speed(
    engine: *const DhruvEngineHandle,
    body_code: i32,
    jd_tdb: f64,
    config: *const DhruvStationaryConfig,
    out_event: *mut DhruvMaxSpeedEvent,
    out_found: *mut u8,
) -> DhruvStatus {
    ffi_boundary(|| {
        if engine.is_null() || config.is_null() || out_event.is_null() || out_found.is_null() {
            return DhruvStatus::NullPointer;
        }

        let body = match Body::from_code(body_code) {
            Some(b) => b,
            None => return DhruvStatus::InvalidQuery,
        };

        let engine_ref = unsafe { &*engine };
        let cfg_ref = unsafe { &*config };
        let rust_config = stationary_config_from_ffi(cfg_ref);

        match prev_max_speed(engine_ref, body, jd_tdb, &rust_config) {
            Ok(Some(event)) => {
                unsafe {
                    *out_event = DhruvMaxSpeedEvent::from(&event);
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
    })
}

/// Search for all max-speed events in a time range.
///
/// # Safety
/// All pointer arguments must be valid and non-null.
/// `out_events` must point to at least `max_count` contiguous `DhruvMaxSpeedEvent`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dhruv_search_max_speed(
    engine: *const DhruvEngineHandle,
    body_code: i32,
    jd_start: f64,
    jd_end: f64,
    config: *const DhruvStationaryConfig,
    out_events: *mut DhruvMaxSpeedEvent,
    max_count: u32,
    out_count: *mut u32,
) -> DhruvStatus {
    ffi_boundary(|| {
        if engine.is_null()
            || config.is_null()
            || out_events.is_null()
            || out_count.is_null()
        {
            return DhruvStatus::NullPointer;
        }

        let body = match Body::from_code(body_code) {
            Some(b) => b,
            None => return DhruvStatus::InvalidQuery,
        };

        let engine_ref = unsafe { &*engine };
        let cfg_ref = unsafe { &*config };
        let rust_config = stationary_config_from_ffi(cfg_ref);

        match search_max_speed(engine_ref, body, jd_start, jd_end, &rust_config) {
            Ok(events) => {
                let count = events.len().min(max_count as usize);
                let out_slice = unsafe {
                    std::slice::from_raw_parts_mut(out_events, max_count as usize)
                };
                for (i, e) in events.iter().take(count).enumerate() {
                    out_slice[i] = DhruvMaxSpeedEvent::from(e);
                }
                unsafe { *out_count = count as u32 };
                DhruvStatus::Ok
            }
            Err(e) => DhruvStatus::from(&e),
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
pub unsafe extern "C" fn dhruv_deg_to_dms(
    degrees: f64,
    out: *mut DhruvDms,
) -> DhruvStatus {
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
    fn ffi_lsk_load_rejects_null() {
        let mut lsk_ptr: *mut DhruvLskHandle = ptr::null_mut();
        // SAFETY: Null path pointer is intentional for validation.
        let status = unsafe { dhruv_lsk_load(ptr::null(), &mut lsk_ptr) };
        assert_eq!(status, DhruvStatus::NullPointer);
    }

    #[test]
    fn ffi_utc_to_tdb_jd_rejects_null() {
        let mut jd: f64 = 0.0;
        // SAFETY: Null LSK pointer is intentional for validation.
        let status = unsafe { dhruv_utc_to_tdb_jd(ptr::null(), 2000, 1, 1, 12, 0, 0.0, &mut jd) };
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
    fn ffi_ayanamsha_rejects_invalid_code() {
        let mut out: f64 = 0.0;
        // SAFETY: Valid output pointer, invalid system code.
        let status = unsafe { dhruv_ayanamsha_mean_deg(99, 2_451_545.0, &mut out) };
        assert_eq!(status, DhruvStatus::InvalidQuery);
    }

    #[test]
    fn ffi_ayanamsha_rejects_null_output() {
        // SAFETY: Null output pointer is intentional for validation.
        let status = unsafe { dhruv_ayanamsha_mean_deg(0, 2_451_545.0, ptr::null_mut()) };
        assert_eq!(status, DhruvStatus::NullPointer);
    }

    #[test]
    fn ffi_ayanamsha_lahiri_at_j2000() {
        let mut out: f64 = 0.0;
        // SAFETY: Valid pointers.
        let status = unsafe { dhruv_ayanamsha_mean_deg(0, 2_451_545.0, &mut out) };
        assert_eq!(status, DhruvStatus::Ok);
        assert!(
            (out - 23.853).abs() < 0.01,
            "Lahiri at J2000 = {out}, expected ~23.853"
        );
    }

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
    fn ffi_ayanamsha_deg_mean_matches_old() {
        let mut unified: f64 = 0.0;
        let mut old: f64 = 0.0;
        // Lahiri at J2000, use_nutation=0 → should match mean
        // SAFETY: Valid pointers.
        let s1 = unsafe { dhruv_ayanamsha_deg(0, 2_451_545.0, 0, &mut unified) };
        let s2 = unsafe { dhruv_ayanamsha_mean_deg(0, 2_451_545.0, &mut old) };
        assert_eq!(s1, DhruvStatus::Ok);
        assert_eq!(s2, DhruvStatus::Ok);
        assert!((unified - old).abs() < 1e-15);
    }

    #[test]
    fn ffi_ayanamsha_deg_true_lahiri_with_nutation() {
        let mut with_nut: f64 = 0.0;
        let mut without: f64 = 0.0;
        let jd = 2_460_310.5; // ~2024-01-01
        // TrueLahiri = system code 1
        // SAFETY: Valid pointers.
        let s1 = unsafe { dhruv_ayanamsha_deg(1, jd, 1, &mut with_nut) };
        let s2 = unsafe { dhruv_ayanamsha_deg(1, jd, 0, &mut without) };
        assert_eq!(s1, DhruvStatus::Ok);
        assert_eq!(s2, DhruvStatus::Ok);
        let diff = (with_nut - without).abs();
        assert!(
            diff > 1e-6 && diff < 0.01,
            "nutation diff = {diff} deg"
        );
    }

    #[test]
    fn ffi_nutation_iau2000b_at_j2000() {
        let mut dpsi: f64 = 0.0;
        let mut deps: f64 = 0.0;
        // SAFETY: Valid pointers.
        let status = unsafe {
            dhruv_nutation_iau2000b(2_451_545.0, &mut dpsi, &mut deps)
        };
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
        let status = unsafe {
            dhruv_nutation_iau2000b(2_451_545.0, &mut dpsi, ptr::null_mut())
        };
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
        let status = unsafe {
            dhruv_riseset_result_to_utc(fake_lsk as *const _, &result, &mut out)
        };
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

    #[test]
    fn ffi_query_utc_spherical_rejects_null() {
        let mut out = DhruvSphericalState {
            lon_deg: 0.0,
            lat_deg: 0.0,
            distance_km: 0.0,
            lon_speed: 0.0,
            lat_speed: 0.0,
            distance_speed: 0.0,
        };

        // SAFETY: Null engine pointer is intentional for validation.
        let status = unsafe {
            dhruv_query_utc_spherical(
                ptr::null(),
                499, 10, 2,
                2024, 3, 20, 12, 0, 0.0,
                &mut out,
            )
        };
        assert_eq!(status, DhruvStatus::NullPointer);
    }

    // --- Bhava tests ---

    #[test]
    fn ffi_bhava_config_default_values() {
        let cfg = dhruv_bhava_config_default();
        assert_eq!(cfg.system, DHRUV_BHAVA_EQUAL);
        assert_eq!(cfg.starting_point, DHRUV_BHAVA_START_ASCENDANT);
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
            ascendant_deg: 0.0,
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
            starting_point: DHRUV_BHAVA_START_ASCENDANT,
            custom_start_deg: 0.0,
            reference_mode: DHRUV_BHAVA_REF_START,
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
        };
        let result = bhava_config_from_ffi(&cfg);
        assert_eq!(result, Err(DhruvStatus::InvalidQuery));
    }

    #[test]
    fn ffi_ascendant_deg_rejects_null() {
        let mut out: f64 = 0.0;
        // SAFETY: Null pointers intentional for validation.
        let status = unsafe {
            dhruv_ascendant_deg(ptr::null(), ptr::null(), ptr::null(), 0.0, &mut out)
        };
        assert_eq!(status, DhruvStatus::NullPointer);
    }

    #[test]
    fn ffi_mc_deg_rejects_null() {
        let mut out: f64 = 0.0;
        // SAFETY: Null pointers intentional for validation.
        let status = unsafe {
            dhruv_mc_deg(ptr::null(), ptr::null(), ptr::null(), 0.0, &mut out)
        };
        assert_eq!(status, DhruvStatus::NullPointer);
    }

    // --- Lunar node tests ---

    #[test]
    fn ffi_lunar_node_rejects_invalid_node_code() {
        let mut out: f64 = 0.0;
        // SAFETY: Valid output pointer, invalid node code.
        let status = unsafe { dhruv_lunar_node_deg(99, DHRUV_NODE_MODE_MEAN, 2_451_545.0, &mut out) };
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
        let status = unsafe { dhruv_lunar_node_deg(DHRUV_NODE_RAHU, DHRUV_NODE_MODE_MEAN, 2_451_545.0, ptr::null_mut()) };
        assert_eq!(status, DhruvStatus::NullPointer);
    }

    #[test]
    fn ffi_lunar_node_rahu_at_j2000() {
        let mut out: f64 = 0.0;
        // SAFETY: Valid pointers.
        let status = unsafe { dhruv_lunar_node_deg(DHRUV_NODE_RAHU, DHRUV_NODE_MODE_MEAN, 2_451_545.0, &mut out) };
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
        let s1 = unsafe { dhruv_lunar_node_deg(DHRUV_NODE_RAHU, DHRUV_NODE_MODE_MEAN, 2_451_545.0, &mut rahu) };
        let s2 = unsafe { dhruv_lunar_node_deg(DHRUV_NODE_KETU, DHRUV_NODE_MODE_MEAN, 2_451_545.0, &mut ketu) };
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
    fn ffi_api_version_is_9() {
        assert_eq!(dhruv_api_version(), 9);
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
        let err = SearchError::Engine(EngineError::EpochOutOfRange {
            epoch_tdb_jd: 0.0,
        });
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
    fn ffi_next_conjunction_rejects_null() {
        let cfg = dhruv_conjunction_config_default();
        let mut event = std::mem::MaybeUninit::<DhruvConjunctionEvent>::uninit();
        let mut found: u8 = 0;
        // SAFETY: Null engine pointer is intentional for validation.
        let status = unsafe {
            dhruv_next_conjunction(
                ptr::null(),
                10,
                301,
                2_460_000.5,
                &cfg,
                event.as_mut_ptr(),
                &mut found,
            )
        };
        assert_eq!(status, DhruvStatus::NullPointer);
    }

    #[test]
    fn ffi_prev_conjunction_rejects_null() {
        let cfg = dhruv_conjunction_config_default();
        let mut event = std::mem::MaybeUninit::<DhruvConjunctionEvent>::uninit();
        let mut found: u8 = 0;
        let status = unsafe {
            dhruv_prev_conjunction(
                ptr::null(),
                10,
                301,
                2_460_000.5,
                &cfg,
                event.as_mut_ptr(),
                &mut found,
            )
        };
        assert_eq!(status, DhruvStatus::NullPointer);
    }

    #[test]
    fn ffi_search_conjunctions_rejects_null() {
        let cfg = dhruv_conjunction_config_default();
        let mut count: u32 = 0;
        let status = unsafe {
            dhruv_search_conjunctions(
                ptr::null(),
                10,
                301,
                2_460_000.5,
                2_460_100.5,
                &cfg,
                ptr::null_mut(),
                10,
                &mut count,
            )
        };
        assert_eq!(status, DhruvStatus::NullPointer);
    }

    #[test]
    fn ffi_next_conjunction_rejects_invalid_body() {
        // Use a dangling engine pointer — function should reject body code first.
        let fake_engine = std::ptr::NonNull::<DhruvEngineHandle>::dangling().as_ptr();
        let cfg = dhruv_conjunction_config_default();
        let mut event = std::mem::MaybeUninit::<DhruvConjunctionEvent>::uninit();
        let mut found: u8 = 0;
        let status = unsafe {
            dhruv_next_conjunction(
                fake_engine as *const _,
                999999,
                301,
                2_460_000.5,
                &cfg,
                event.as_mut_ptr(),
                &mut found,
            )
        };
        assert_eq!(status, DhruvStatus::InvalidQuery);
    }

    // --- Eclipse FFI tests ---

    #[test]
    fn ffi_eclipse_config_default_values() {
        let cfg = dhruv_eclipse_config_default();
        assert_eq!(cfg.include_penumbral, 1);
        assert_eq!(cfg.include_peak_details, 1);
    }

    #[test]
    fn ffi_next_lunar_eclipse_rejects_null() {
        let cfg = dhruv_eclipse_config_default();
        let mut result = std::mem::MaybeUninit::<DhruvLunarEclipseResult>::uninit();
        let mut found: u8 = 0;
        let status = unsafe {
            dhruv_next_lunar_eclipse(
                ptr::null(),
                2_460_000.5,
                &cfg,
                result.as_mut_ptr(),
                &mut found,
            )
        };
        assert_eq!(status, DhruvStatus::NullPointer);
    }

    #[test]
    fn ffi_prev_lunar_eclipse_rejects_null() {
        let cfg = dhruv_eclipse_config_default();
        let mut result = std::mem::MaybeUninit::<DhruvLunarEclipseResult>::uninit();
        let mut found: u8 = 0;
        let status = unsafe {
            dhruv_prev_lunar_eclipse(
                ptr::null(),
                2_460_000.5,
                &cfg,
                result.as_mut_ptr(),
                &mut found,
            )
        };
        assert_eq!(status, DhruvStatus::NullPointer);
    }

    #[test]
    fn ffi_search_lunar_eclipses_rejects_null() {
        let cfg = dhruv_eclipse_config_default();
        let mut count: u32 = 0;
        let status = unsafe {
            dhruv_search_lunar_eclipses(
                ptr::null(),
                2_460_000.5,
                2_460_400.5,
                &cfg,
                ptr::null_mut(),
                10,
                &mut count,
            )
        };
        assert_eq!(status, DhruvStatus::NullPointer);
    }

    #[test]
    fn ffi_next_solar_eclipse_rejects_null() {
        let cfg = dhruv_eclipse_config_default();
        let mut result = std::mem::MaybeUninit::<DhruvSolarEclipseResult>::uninit();
        let mut found: u8 = 0;
        let status = unsafe {
            dhruv_next_solar_eclipse(
                ptr::null(),
                2_460_000.5,
                &cfg,
                result.as_mut_ptr(),
                &mut found,
            )
        };
        assert_eq!(status, DhruvStatus::NullPointer);
    }

    #[test]
    fn ffi_prev_solar_eclipse_rejects_null() {
        let cfg = dhruv_eclipse_config_default();
        let mut result = std::mem::MaybeUninit::<DhruvSolarEclipseResult>::uninit();
        let mut found: u8 = 0;
        let status = unsafe {
            dhruv_prev_solar_eclipse(
                ptr::null(),
                2_460_000.5,
                &cfg,
                result.as_mut_ptr(),
                &mut found,
            )
        };
        assert_eq!(status, DhruvStatus::NullPointer);
    }

    #[test]
    fn ffi_search_solar_eclipses_rejects_null() {
        let cfg = dhruv_eclipse_config_default();
        let mut count: u32 = 0;
        let status = unsafe {
            dhruv_search_solar_eclipses(
                ptr::null(),
                2_460_000.5,
                2_460_400.5,
                &cfg,
                ptr::null_mut(),
                10,
                &mut count,
            )
        };
        assert_eq!(status, DhruvStatus::NullPointer);
    }

    #[test]
    fn ffi_lunar_eclipse_type_constants() {
        assert_eq!(DHRUV_LUNAR_ECLIPSE_PENUMBRAL, 0);
        assert_eq!(DHRUV_LUNAR_ECLIPSE_PARTIAL, 1);
        assert_eq!(DHRUV_LUNAR_ECLIPSE_TOTAL, 2);
    }

    #[test]
    fn ffi_solar_eclipse_type_constants() {
        assert_eq!(DHRUV_SOLAR_ECLIPSE_PARTIAL, 0);
        assert_eq!(DHRUV_SOLAR_ECLIPSE_ANNULAR, 1);
        assert_eq!(DHRUV_SOLAR_ECLIPSE_TOTAL, 2);
        assert_eq!(DHRUV_SOLAR_ECLIPSE_HYBRID, 3);
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
    fn ffi_next_stationary_rejects_null() {
        let cfg = dhruv_stationary_config_default();
        let mut event = std::mem::MaybeUninit::<DhruvStationaryEvent>::uninit();
        let mut found: u8 = 0;
        let status = unsafe {
            dhruv_next_stationary(
                ptr::null(),
                199,
                2_460_000.5,
                &cfg,
                event.as_mut_ptr(),
                &mut found,
            )
        };
        assert_eq!(status, DhruvStatus::NullPointer);
    }

    #[test]
    fn ffi_prev_stationary_rejects_null() {
        let cfg = dhruv_stationary_config_default();
        let mut event = std::mem::MaybeUninit::<DhruvStationaryEvent>::uninit();
        let mut found: u8 = 0;
        let status = unsafe {
            dhruv_prev_stationary(
                ptr::null(),
                199,
                2_460_000.5,
                &cfg,
                event.as_mut_ptr(),
                &mut found,
            )
        };
        assert_eq!(status, DhruvStatus::NullPointer);
    }

    #[test]
    fn ffi_search_stationary_rejects_null() {
        let cfg = dhruv_stationary_config_default();
        let mut count: u32 = 0;
        let status = unsafe {
            dhruv_search_stationary(
                ptr::null(),
                199,
                2_460_000.5,
                2_460_100.5,
                &cfg,
                ptr::null_mut(),
                10,
                &mut count,
            )
        };
        assert_eq!(status, DhruvStatus::NullPointer);
    }

    #[test]
    fn ffi_next_stationary_rejects_invalid_body() {
        let fake_engine = std::ptr::NonNull::<DhruvEngineHandle>::dangling().as_ptr();
        let cfg = dhruv_stationary_config_default();
        let mut event = std::mem::MaybeUninit::<DhruvStationaryEvent>::uninit();
        let mut found: u8 = 0;
        let status = unsafe {
            dhruv_next_stationary(
                fake_engine as *const _,
                999999,
                2_460_000.5,
                &cfg,
                event.as_mut_ptr(),
                &mut found,
            )
        };
        assert_eq!(status, DhruvStatus::InvalidQuery);
    }

    #[test]
    fn ffi_next_stationary_rejects_sun() {
        let fake_engine = std::ptr::NonNull::<DhruvEngineHandle>::dangling().as_ptr();
        let cfg = dhruv_stationary_config_default();
        let mut event = std::mem::MaybeUninit::<DhruvStationaryEvent>::uninit();
        let mut found: u8 = 0;
        let status = unsafe {
            dhruv_next_stationary(
                fake_engine as *const _,
                10, // Sun
                2_460_000.5,
                &cfg,
                event.as_mut_ptr(),
                &mut found,
            )
        };
        assert_eq!(status, DhruvStatus::InvalidSearchConfig);
    }

    #[test]
    fn ffi_next_max_speed_rejects_null() {
        let cfg = dhruv_stationary_config_default();
        let mut event = std::mem::MaybeUninit::<DhruvMaxSpeedEvent>::uninit();
        let mut found: u8 = 0;
        let status = unsafe {
            dhruv_next_max_speed(
                ptr::null(),
                199,
                2_460_000.5,
                &cfg,
                event.as_mut_ptr(),
                &mut found,
            )
        };
        assert_eq!(status, DhruvStatus::NullPointer);
    }

    #[test]
    fn ffi_prev_max_speed_rejects_null() {
        let cfg = dhruv_stationary_config_default();
        let mut event = std::mem::MaybeUninit::<DhruvMaxSpeedEvent>::uninit();
        let mut found: u8 = 0;
        let status = unsafe {
            dhruv_prev_max_speed(
                ptr::null(),
                199,
                2_460_000.5,
                &cfg,
                event.as_mut_ptr(),
                &mut found,
            )
        };
        assert_eq!(status, DhruvStatus::NullPointer);
    }

    #[test]
    fn ffi_search_max_speed_rejects_null() {
        let cfg = dhruv_stationary_config_default();
        let mut count: u32 = 0;
        let status = unsafe {
            dhruv_search_max_speed(
                ptr::null(),
                199,
                2_460_000.5,
                2_460_100.5,
                &cfg,
                ptr::null_mut(),
                10,
                &mut count,
            )
        };
        assert_eq!(status, DhruvStatus::NullPointer);
    }

    #[test]
    fn ffi_next_max_speed_rejects_earth() {
        let fake_engine = std::ptr::NonNull::<DhruvEngineHandle>::dangling().as_ptr();
        let cfg = dhruv_stationary_config_default();
        let mut event = std::mem::MaybeUninit::<DhruvMaxSpeedEvent>::uninit();
        let mut found: u8 = 0;
        let status = unsafe {
            dhruv_next_max_speed(
                fake_engine as *const _,
                399, // Earth
                2_460_000.5,
                &cfg,
                event.as_mut_ptr(),
                &mut found,
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
        let status = unsafe {
            dhruv_rashi_from_tropical(280.5, 0, 2_451_545.0, 0, out.as_mut_ptr())
        };
        assert_eq!(status, DhruvStatus::Ok);
        let info = unsafe { out.assume_init() };
        assert_eq!(info.rashi_index, 8); // Dhanu
    }

    #[test]
    fn ffi_rashi_from_tropical_invalid_system() {
        let mut out = std::mem::MaybeUninit::<DhruvRashiInfo>::uninit();
        let status = unsafe {
            dhruv_rashi_from_tropical(280.5, 99, 2_451_545.0, 0, out.as_mut_ptr())
        };
        assert_eq!(status, DhruvStatus::InvalidQuery);
    }

    #[test]
    fn ffi_nakshatra_from_tropical_rejects_null() {
        let status = unsafe {
            dhruv_nakshatra_from_tropical(280.5, 0, 2_451_545.0, 0, ptr::null_mut())
        };
        assert_eq!(status, DhruvStatus::NullPointer);
    }

    #[test]
    fn ffi_nakshatra28_from_tropical_rejects_null() {
        let status = unsafe {
            dhruv_nakshatra28_from_tropical(280.5, 0, 2_451_545.0, 0, ptr::null_mut())
        };
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
}
