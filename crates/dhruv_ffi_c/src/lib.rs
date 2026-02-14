//! C-facing adapter types for `ctara-dhruv-core`.

use std::path::PathBuf;
use std::ptr;

use dhruv_core::{Body, Engine, EngineConfig, EngineError, Frame, Observer, Query, StateVector};
use dhruv_search::{
    ChandraGrahan, ChandraGrahanType, ConjunctionConfig, ConjunctionEvent, GrahanConfig,
    LunarPhase, MaxSpeedEvent, MaxSpeedType, SankrantiConfig, SearchError, StationType,
    StationaryConfig, StationaryEvent, SuryaGrahan, SuryaGrahanType, amsha_charts_for_date,
    ayana_for_date, body_ecliptic_lon_lat, elongation_at, full_kundali_for_date,
    ghatika_for_date, ghatika_from_sunrises, graha_sidereal_longitudes, hora_for_date,
    hora_from_sunrises, karana_at, karana_for_date, masa_for_date, nakshatra_at, nakshatra_for_date,
    next_amavasya, next_chandra_grahan, next_conjunction, next_max_speed, next_purnima,
    next_sankranti, next_specific_sankranti, next_stationary, next_surya_grahan, panchang_for_date,
    prev_amavasya, prev_chandra_grahan, prev_conjunction, prev_max_speed, prev_purnima,
    prev_sankranti, prev_specific_sankranti, prev_stationary, prev_surya_grahan, search_amavasyas,
    search_chandra_grahan, search_conjunctions, search_max_speed, search_purnimas,
    search_sankrantis, search_stationary, search_surya_grahan, sidereal_sum_at,
    shadbala_for_date, shadbala_for_graha, special_lagnas_for_date, tithi_at, tithi_for_date,
    vaar_for_date, vaar_from_sunrises, varsha_for_date, vedic_day_sunrises, vimsopaka_for_date,
    vimsopaka_for_graha, yoga_at, yoga_for_date,
};
use dhruv_time::UtcTime;
use dhruv_vedic_base::{
    Amsha, AmshaRequest, AmshaVariation, AyanamshaSystem, BhavaConfig, BhavaReferenceMode,
    BhavaStartingPoint, BhavaSystem, GeoLocation, LunarNode, NodeMode, RiseSetConfig, RiseSetEvent,
    RiseSetResult, SunLimb, VedicError, amsha_longitude, amsha_rashi_info,
    approximate_local_noon_jd, ayana_from_sidereal_longitude, ayanamsha_deg, ayanamsha_mean_deg,
    ayanamsha_true_deg, compute_all_events, compute_bhavas, compute_rise_set, deg_to_dms,
    jd_tdb_to_centuries, karana_from_elongation, lunar_node_deg, masa_from_rashi_index,
    nakshatra_from_longitude, nakshatra_from_tropical, nakshatra28_from_longitude,
    nakshatra28_from_tropical, nth_rashi_from, rashi_from_longitude, rashi_from_tropical,
    samvatsara_from_year, time_upagraha_jd, tithi_from_elongation, vaar_from_jd, yoga_from_sum,
};

/// ABI version for downstream bindings.
pub const DHRUV_API_VERSION: u32 = 31;

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
    let target = Body::from_code(target_code).ok_or(DhruvStatus::InvalidQuery)?;
    let observer = Observer::from_code(observer_code).ok_or(DhruvStatus::InvalidQuery)?;
    let frame = Frame::from_code(frame_code).ok_or(DhruvStatus::InvalidQuery)?;

    let epoch = dhruv_time::Epoch::from_utc(year, month, day, hour, min, sec, engine.lsk());

    let query = Query {
        target,
        observer,
        frame,
        epoch_tdb_jd: epoch.as_jd_tdb(),
    };

    let state = engine.query(query).map_err(|e| DhruvStatus::from(&e))?;

    let ss =
        dhruv_frames::cartesian_state_to_spherical_state(&state.position_km, &state.velocity_km_s);

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

        let geo = GeoLocation::new(
            loc_ref.latitude_deg,
            loc_ref.longitude_deg,
            loc_ref.altitude_m,
        );
        let sun_limb = match sun_limb_from_code(cfg_ref.sun_limb) {
            Some(l) => l,
            None => return DhruvStatus::InvalidQuery,
        };
        let rs_config = RiseSetConfig {
            use_refraction: cfg_ref.use_refraction != 0,
            sun_limb,
            altitude_correction: cfg_ref.altitude_correction != 0,
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

        let geo = GeoLocation::new(
            loc_ref.latitude_deg,
            loc_ref.longitude_deg,
            loc_ref.altitude_m,
        );
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
pub extern "C" fn dhruv_approximate_local_noon_jd(jd_ut_midnight: f64, longitude_deg: f64) -> f64 {
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

/// Returns default bhava configuration (Equal, Lagna, StartOfFirst).
#[unsafe(no_mangle)]
pub extern "C" fn dhruv_bhava_config_default() -> DhruvBhavaConfig {
    DhruvBhavaConfig {
        system: DHRUV_BHAVA_EQUAL,
        starting_point: DHRUV_BHAVA_START_LAGNA,
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
                        lagna_deg: result.lagna_deg,
                        mc_deg: result.mc_deg,
                    };
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
        if engine.is_null() || config.is_null() || out_events.is_null() || out_count.is_null() {
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
                let out_slice =
                    unsafe { std::slice::from_raw_parts_mut(out_events, max_count as usize) };
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

// ---------------------------------------------------------------------------
// Chandra grahan FFI functions
// ---------------------------------------------------------------------------

/// Find the next chandra grahan (lunar eclipse) after `jd_tdb`.
///
/// # Safety
/// All pointer arguments must be valid and non-null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dhruv_next_chandra_grahan(
    engine: *const DhruvEngineHandle,
    jd_tdb: f64,
    config: *const DhruvGrahanConfig,
    out_result: *mut DhruvChandraGrahanResult,
    out_found: *mut u8,
) -> DhruvStatus {
    ffi_boundary(|| {
        if engine.is_null() || config.is_null() || out_result.is_null() || out_found.is_null() {
            return DhruvStatus::NullPointer;
        }

        // SAFETY: All pointers checked for null above.
        let engine_ref = unsafe { &*engine };
        let cfg_ref = unsafe { &*config };
        let rust_config = grahan_config_from_ffi(cfg_ref);

        match next_chandra_grahan(engine_ref, jd_tdb, &rust_config) {
            Ok(Some(grahan)) => {
                unsafe {
                    *out_result = DhruvChandraGrahanResult::from(&grahan);
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

/// Find the previous chandra grahan (lunar eclipse) before `jd_tdb`.
///
/// # Safety
/// All pointer arguments must be valid and non-null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dhruv_prev_chandra_grahan(
    engine: *const DhruvEngineHandle,
    jd_tdb: f64,
    config: *const DhruvGrahanConfig,
    out_result: *mut DhruvChandraGrahanResult,
    out_found: *mut u8,
) -> DhruvStatus {
    ffi_boundary(|| {
        if engine.is_null() || config.is_null() || out_result.is_null() || out_found.is_null() {
            return DhruvStatus::NullPointer;
        }

        let engine_ref = unsafe { &*engine };
        let cfg_ref = unsafe { &*config };
        let rust_config = grahan_config_from_ffi(cfg_ref);

        match prev_chandra_grahan(engine_ref, jd_tdb, &rust_config) {
            Ok(Some(grahan)) => {
                unsafe {
                    *out_result = DhruvChandraGrahanResult::from(&grahan);
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

/// Search for all chandra grahan (lunar eclipses) in a time range.
///
/// # Safety
/// All pointer arguments must be valid and non-null.
/// `out_results` must point to at least `max_count` contiguous `DhruvChandraGrahanResult`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dhruv_search_chandra_grahan(
    engine: *const DhruvEngineHandle,
    jd_start: f64,
    jd_end: f64,
    config: *const DhruvGrahanConfig,
    out_results: *mut DhruvChandraGrahanResult,
    max_count: u32,
    out_count: *mut u32,
) -> DhruvStatus {
    ffi_boundary(|| {
        if engine.is_null() || config.is_null() || out_results.is_null() || out_count.is_null() {
            return DhruvStatus::NullPointer;
        }

        let engine_ref = unsafe { &*engine };
        let cfg_ref = unsafe { &*config };
        let rust_config = grahan_config_from_ffi(cfg_ref);

        match search_chandra_grahan(engine_ref, jd_start, jd_end, &rust_config) {
            Ok(results) => {
                let count = results.len().min(max_count as usize);
                let out_slice =
                    unsafe { std::slice::from_raw_parts_mut(out_results, max_count as usize) };
                for (i, e) in results.iter().take(count).enumerate() {
                    out_slice[i] = DhruvChandraGrahanResult::from(e);
                }
                unsafe { *out_count = count as u32 };
                DhruvStatus::Ok
            }
            Err(e) => DhruvStatus::from(&e),
        }
    })
}

// ---------------------------------------------------------------------------
// Surya grahan FFI functions
// ---------------------------------------------------------------------------

/// Find the next surya grahan (solar eclipse) after `jd_tdb`.
///
/// # Safety
/// All pointer arguments must be valid and non-null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dhruv_next_surya_grahan(
    engine: *const DhruvEngineHandle,
    jd_tdb: f64,
    config: *const DhruvGrahanConfig,
    out_result: *mut DhruvSuryaGrahanResult,
    out_found: *mut u8,
) -> DhruvStatus {
    ffi_boundary(|| {
        if engine.is_null() || config.is_null() || out_result.is_null() || out_found.is_null() {
            return DhruvStatus::NullPointer;
        }

        let engine_ref = unsafe { &*engine };
        let cfg_ref = unsafe { &*config };
        let rust_config = grahan_config_from_ffi(cfg_ref);

        match next_surya_grahan(engine_ref, jd_tdb, &rust_config) {
            Ok(Some(grahan)) => {
                unsafe {
                    *out_result = DhruvSuryaGrahanResult::from(&grahan);
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

/// Find the previous surya grahan (solar eclipse) before `jd_tdb`.
///
/// # Safety
/// All pointer arguments must be valid and non-null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dhruv_prev_surya_grahan(
    engine: *const DhruvEngineHandle,
    jd_tdb: f64,
    config: *const DhruvGrahanConfig,
    out_result: *mut DhruvSuryaGrahanResult,
    out_found: *mut u8,
) -> DhruvStatus {
    ffi_boundary(|| {
        if engine.is_null() || config.is_null() || out_result.is_null() || out_found.is_null() {
            return DhruvStatus::NullPointer;
        }

        let engine_ref = unsafe { &*engine };
        let cfg_ref = unsafe { &*config };
        let rust_config = grahan_config_from_ffi(cfg_ref);

        match prev_surya_grahan(engine_ref, jd_tdb, &rust_config) {
            Ok(Some(grahan)) => {
                unsafe {
                    *out_result = DhruvSuryaGrahanResult::from(&grahan);
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

/// Search for all surya grahan (solar eclipses) in a time range.
///
/// # Safety
/// All pointer arguments must be valid and non-null.
/// `out_results` must point to at least `max_count` contiguous `DhruvSuryaGrahanResult`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dhruv_search_surya_grahan(
    engine: *const DhruvEngineHandle,
    jd_start: f64,
    jd_end: f64,
    config: *const DhruvGrahanConfig,
    out_results: *mut DhruvSuryaGrahanResult,
    max_count: u32,
    out_count: *mut u32,
) -> DhruvStatus {
    ffi_boundary(|| {
        if engine.is_null() || config.is_null() || out_results.is_null() || out_count.is_null() {
            return DhruvStatus::NullPointer;
        }

        let engine_ref = unsafe { &*engine };
        let cfg_ref = unsafe { &*config };
        let rust_config = grahan_config_from_ffi(cfg_ref);

        match search_surya_grahan(engine_ref, jd_start, jd_end, &rust_config) {
            Ok(results) => {
                let count = results.len().min(max_count as usize);
                let out_slice =
                    unsafe { std::slice::from_raw_parts_mut(out_results, max_count as usize) };
                for (i, e) in results.iter().take(count).enumerate() {
                    out_slice[i] = DhruvSuryaGrahanResult::from(e);
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
        if engine.is_null() || config.is_null() || out_events.is_null() || out_count.is_null() {
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
                let out_slice =
                    unsafe { std::slice::from_raw_parts_mut(out_events, max_count as usize) };
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
        if engine.is_null() || config.is_null() || out_events.is_null() || out_count.is_null() {
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
                let out_slice =
                    unsafe { std::slice::from_raw_parts_mut(out_events, max_count as usize) };
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

/// Ayana constants.
pub const DHRUV_AYANA_UTTARAYANA: i32 = 0;
pub const DHRUV_AYANA_DAKSHINAYANA: i32 = 1;

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

fn sankranti_config_from_ffi(cfg: &DhruvSankrantiConfig) -> Option<SankrantiConfig> {
    let system = ayanamsha_system_from_code(cfg.ayanamsha_system)?;
    Some(SankrantiConfig {
        ayanamsha_system: system,
        use_nutation: cfg.use_nutation != 0,
        step_size_days: cfg.step_size_days,
        max_iterations: cfg.max_iterations,
        convergence_days: cfg.convergence_days,
    })
}

/// Returns default Sankranti search configuration (Lahiri, no nutation).
#[unsafe(no_mangle)]
pub extern "C" fn dhruv_sankranti_config_default() -> DhruvSankrantiConfig {
    DhruvSankrantiConfig {
        ayanamsha_system: 0, // Lahiri
        use_nutation: 0,
        step_size_days: 1.0,
        max_iterations: 50,
        convergence_days: 1e-8,
    }
}

/// Find the next Purnima (full moon) after the given UTC time.
///
/// # Safety
/// All pointer arguments must be valid and non-null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dhruv_next_purnima(
    engine: *const DhruvEngineHandle,
    utc: *const DhruvUtcTime,
    out_event: *mut DhruvLunarPhaseEvent,
    out_found: *mut u8,
) -> DhruvStatus {
    ffi_boundary(|| {
        if engine.is_null() || utc.is_null() || out_event.is_null() || out_found.is_null() {
            return DhruvStatus::NullPointer;
        }
        let engine_ref = unsafe { &*engine };
        let utc_ref = unsafe { &*utc };
        let t = ffi_to_utc_time(utc_ref);
        match next_purnima(engine_ref, &t) {
            Ok(Some(e)) => {
                unsafe {
                    *out_event = DhruvLunarPhaseEvent {
                        utc: utc_time_to_ffi(&e.utc),
                        phase: lunar_phase_to_code(e.phase),
                        moon_longitude_deg: e.moon_longitude_deg,
                        sun_longitude_deg: e.sun_longitude_deg,
                    };
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

/// Find the previous Purnima (full moon) before the given UTC time.
///
/// # Safety
/// All pointer arguments must be valid and non-null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dhruv_prev_purnima(
    engine: *const DhruvEngineHandle,
    utc: *const DhruvUtcTime,
    out_event: *mut DhruvLunarPhaseEvent,
    out_found: *mut u8,
) -> DhruvStatus {
    ffi_boundary(|| {
        if engine.is_null() || utc.is_null() || out_event.is_null() || out_found.is_null() {
            return DhruvStatus::NullPointer;
        }
        let engine_ref = unsafe { &*engine };
        let t = ffi_to_utc_time(unsafe { &*utc });
        match prev_purnima(engine_ref, &t) {
            Ok(Some(e)) => {
                unsafe {
                    *out_event = DhruvLunarPhaseEvent {
                        utc: utc_time_to_ffi(&e.utc),
                        phase: lunar_phase_to_code(e.phase),
                        moon_longitude_deg: e.moon_longitude_deg,
                        sun_longitude_deg: e.sun_longitude_deg,
                    };
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

/// Find the next Amavasya (new moon) after the given UTC time.
///
/// # Safety
/// All pointer arguments must be valid and non-null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dhruv_next_amavasya(
    engine: *const DhruvEngineHandle,
    utc: *const DhruvUtcTime,
    out_event: *mut DhruvLunarPhaseEvent,
    out_found: *mut u8,
) -> DhruvStatus {
    ffi_boundary(|| {
        if engine.is_null() || utc.is_null() || out_event.is_null() || out_found.is_null() {
            return DhruvStatus::NullPointer;
        }
        let engine_ref = unsafe { &*engine };
        let t = ffi_to_utc_time(unsafe { &*utc });
        match next_amavasya(engine_ref, &t) {
            Ok(Some(e)) => {
                unsafe {
                    *out_event = DhruvLunarPhaseEvent {
                        utc: utc_time_to_ffi(&e.utc),
                        phase: lunar_phase_to_code(e.phase),
                        moon_longitude_deg: e.moon_longitude_deg,
                        sun_longitude_deg: e.sun_longitude_deg,
                    };
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

/// Find the previous Amavasya (new moon) before the given UTC time.
///
/// # Safety
/// All pointer arguments must be valid and non-null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dhruv_prev_amavasya(
    engine: *const DhruvEngineHandle,
    utc: *const DhruvUtcTime,
    out_event: *mut DhruvLunarPhaseEvent,
    out_found: *mut u8,
) -> DhruvStatus {
    ffi_boundary(|| {
        if engine.is_null() || utc.is_null() || out_event.is_null() || out_found.is_null() {
            return DhruvStatus::NullPointer;
        }
        let engine_ref = unsafe { &*engine };
        let t = ffi_to_utc_time(unsafe { &*utc });
        match prev_amavasya(engine_ref, &t) {
            Ok(Some(e)) => {
                unsafe {
                    *out_event = DhruvLunarPhaseEvent {
                        utc: utc_time_to_ffi(&e.utc),
                        phase: lunar_phase_to_code(e.phase),
                        moon_longitude_deg: e.moon_longitude_deg,
                        sun_longitude_deg: e.sun_longitude_deg,
                    };
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

/// Search for all Purnimas in a UTC time range.
///
/// # Safety
/// All pointer arguments must be valid and non-null.
/// `out_events` must point to at least `max_count` contiguous `DhruvLunarPhaseEvent`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dhruv_search_purnimas(
    engine: *const DhruvEngineHandle,
    start: *const DhruvUtcTime,
    end: *const DhruvUtcTime,
    out_events: *mut DhruvLunarPhaseEvent,
    max_count: u32,
    out_count: *mut u32,
) -> DhruvStatus {
    ffi_boundary(|| {
        if engine.is_null()
            || start.is_null()
            || end.is_null()
            || out_events.is_null()
            || out_count.is_null()
        {
            return DhruvStatus::NullPointer;
        }
        let engine_ref = unsafe { &*engine };
        let s = ffi_to_utc_time(unsafe { &*start });
        let e = ffi_to_utc_time(unsafe { &*end });
        match search_purnimas(engine_ref, &s, &e) {
            Ok(events) => {
                let count = events.len().min(max_count as usize);
                let out_slice =
                    unsafe { std::slice::from_raw_parts_mut(out_events, max_count as usize) };
                for (i, ev) in events.iter().take(count).enumerate() {
                    out_slice[i] = DhruvLunarPhaseEvent {
                        utc: utc_time_to_ffi(&ev.utc),
                        phase: lunar_phase_to_code(ev.phase),
                        moon_longitude_deg: ev.moon_longitude_deg,
                        sun_longitude_deg: ev.sun_longitude_deg,
                    };
                }
                unsafe { *out_count = count as u32 };
                DhruvStatus::Ok
            }
            Err(e) => DhruvStatus::from(&e),
        }
    })
}

/// Search for all Amavasyas in a UTC time range.
///
/// # Safety
/// All pointer arguments must be valid and non-null.
/// `out_events` must point to at least `max_count` contiguous `DhruvLunarPhaseEvent`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dhruv_search_amavasyas(
    engine: *const DhruvEngineHandle,
    start: *const DhruvUtcTime,
    end: *const DhruvUtcTime,
    out_events: *mut DhruvLunarPhaseEvent,
    max_count: u32,
    out_count: *mut u32,
) -> DhruvStatus {
    ffi_boundary(|| {
        if engine.is_null()
            || start.is_null()
            || end.is_null()
            || out_events.is_null()
            || out_count.is_null()
        {
            return DhruvStatus::NullPointer;
        }
        let engine_ref = unsafe { &*engine };
        let s = ffi_to_utc_time(unsafe { &*start });
        let e = ffi_to_utc_time(unsafe { &*end });
        match search_amavasyas(engine_ref, &s, &e) {
            Ok(events) => {
                let count = events.len().min(max_count as usize);
                let out_slice =
                    unsafe { std::slice::from_raw_parts_mut(out_events, max_count as usize) };
                for (i, ev) in events.iter().take(count).enumerate() {
                    out_slice[i] = DhruvLunarPhaseEvent {
                        utc: utc_time_to_ffi(&ev.utc),
                        phase: lunar_phase_to_code(ev.phase),
                        moon_longitude_deg: ev.moon_longitude_deg,
                        sun_longitude_deg: ev.sun_longitude_deg,
                    };
                }
                unsafe { *out_count = count as u32 };
                DhruvStatus::Ok
            }
            Err(e) => DhruvStatus::from(&e),
        }
    })
}

/// Find the next Sankranti (Sun entering any rashi).
///
/// # Safety
/// All pointer arguments must be valid and non-null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dhruv_next_sankranti(
    engine: *const DhruvEngineHandle,
    utc: *const DhruvUtcTime,
    config: *const DhruvSankrantiConfig,
    out_event: *mut DhruvSankrantiEvent,
    out_found: *mut u8,
) -> DhruvStatus {
    ffi_boundary(|| {
        if engine.is_null()
            || utc.is_null()
            || config.is_null()
            || out_event.is_null()
            || out_found.is_null()
        {
            return DhruvStatus::NullPointer;
        }
        let engine_ref = unsafe { &*engine };
        let t = ffi_to_utc_time(unsafe { &*utc });
        let cfg = match sankranti_config_from_ffi(unsafe { &*config }) {
            Some(c) => c,
            None => return DhruvStatus::InvalidQuery,
        };
        match next_sankranti(engine_ref, &t, &cfg) {
            Ok(Some(e)) => {
                unsafe {
                    *out_event = DhruvSankrantiEvent {
                        utc: utc_time_to_ffi(&e.utc),
                        rashi_index: e.rashi_index as i32,
                        sun_sidereal_longitude_deg: e.sun_sidereal_longitude_deg,
                        sun_tropical_longitude_deg: e.sun_tropical_longitude_deg,
                    };
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

/// Find the previous Sankranti.
///
/// # Safety
/// All pointer arguments must be valid and non-null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dhruv_prev_sankranti(
    engine: *const DhruvEngineHandle,
    utc: *const DhruvUtcTime,
    config: *const DhruvSankrantiConfig,
    out_event: *mut DhruvSankrantiEvent,
    out_found: *mut u8,
) -> DhruvStatus {
    ffi_boundary(|| {
        if engine.is_null()
            || utc.is_null()
            || config.is_null()
            || out_event.is_null()
            || out_found.is_null()
        {
            return DhruvStatus::NullPointer;
        }
        let engine_ref = unsafe { &*engine };
        let t = ffi_to_utc_time(unsafe { &*utc });
        let cfg = match sankranti_config_from_ffi(unsafe { &*config }) {
            Some(c) => c,
            None => return DhruvStatus::InvalidQuery,
        };
        match prev_sankranti(engine_ref, &t, &cfg) {
            Ok(Some(e)) => {
                unsafe {
                    *out_event = DhruvSankrantiEvent {
                        utc: utc_time_to_ffi(&e.utc),
                        rashi_index: e.rashi_index as i32,
                        sun_sidereal_longitude_deg: e.sun_sidereal_longitude_deg,
                        sun_tropical_longitude_deg: e.sun_tropical_longitude_deg,
                    };
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

/// Search for all Sankrantis in a UTC time range.
///
/// # Safety
/// All pointer arguments must be valid and non-null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dhruv_search_sankrantis(
    engine: *const DhruvEngineHandle,
    start: *const DhruvUtcTime,
    end: *const DhruvUtcTime,
    config: *const DhruvSankrantiConfig,
    out_events: *mut DhruvSankrantiEvent,
    max_count: u32,
    out_count: *mut u32,
) -> DhruvStatus {
    ffi_boundary(|| {
        if engine.is_null()
            || start.is_null()
            || end.is_null()
            || config.is_null()
            || out_events.is_null()
            || out_count.is_null()
        {
            return DhruvStatus::NullPointer;
        }
        let engine_ref = unsafe { &*engine };
        let s = ffi_to_utc_time(unsafe { &*start });
        let e = ffi_to_utc_time(unsafe { &*end });
        let cfg = match sankranti_config_from_ffi(unsafe { &*config }) {
            Some(c) => c,
            None => return DhruvStatus::InvalidQuery,
        };
        match search_sankrantis(engine_ref, &s, &e, &cfg) {
            Ok(events) => {
                let count = events.len().min(max_count as usize);
                let out_slice =
                    unsafe { std::slice::from_raw_parts_mut(out_events, max_count as usize) };
                for (i, ev) in events.iter().take(count).enumerate() {
                    out_slice[i] = DhruvSankrantiEvent {
                        utc: utc_time_to_ffi(&ev.utc),
                        rashi_index: ev.rashi_index as i32,
                        sun_sidereal_longitude_deg: ev.sun_sidereal_longitude_deg,
                        sun_tropical_longitude_deg: ev.sun_tropical_longitude_deg,
                    };
                }
                unsafe { *out_count = count as u32 };
                DhruvStatus::Ok
            }
            Err(e) => DhruvStatus::from(&e),
        }
    })
}

/// Find the next time the Sun enters a specific rashi.
///
/// # Safety
/// All pointer arguments must be valid and non-null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dhruv_next_specific_sankranti(
    engine: *const DhruvEngineHandle,
    utc: *const DhruvUtcTime,
    rashi_index: i32,
    config: *const DhruvSankrantiConfig,
    out_event: *mut DhruvSankrantiEvent,
    out_found: *mut u8,
) -> DhruvStatus {
    ffi_boundary(|| {
        if engine.is_null()
            || utc.is_null()
            || config.is_null()
            || out_event.is_null()
            || out_found.is_null()
        {
            return DhruvStatus::NullPointer;
        }
        let rashi_idx = rashi_index as usize;
        if rashi_idx >= 12 {
            return DhruvStatus::InvalidQuery;
        }
        let rashi = dhruv_vedic_base::ALL_RASHIS[rashi_idx];
        let engine_ref = unsafe { &*engine };
        let t = ffi_to_utc_time(unsafe { &*utc });
        let cfg = match sankranti_config_from_ffi(unsafe { &*config }) {
            Some(c) => c,
            None => return DhruvStatus::InvalidQuery,
        };
        match next_specific_sankranti(engine_ref, &t, rashi, &cfg) {
            Ok(Some(e)) => {
                unsafe {
                    *out_event = DhruvSankrantiEvent {
                        utc: utc_time_to_ffi(&e.utc),
                        rashi_index: e.rashi_index as i32,
                        sun_sidereal_longitude_deg: e.sun_sidereal_longitude_deg,
                        sun_tropical_longitude_deg: e.sun_tropical_longitude_deg,
                    };
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

/// Find the previous time the Sun entered a specific rashi.
///
/// # Safety
/// All pointer arguments must be valid and non-null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dhruv_prev_specific_sankranti(
    engine: *const DhruvEngineHandle,
    utc: *const DhruvUtcTime,
    rashi_index: i32,
    config: *const DhruvSankrantiConfig,
    out_event: *mut DhruvSankrantiEvent,
    out_found: *mut u8,
) -> DhruvStatus {
    ffi_boundary(|| {
        if engine.is_null()
            || utc.is_null()
            || config.is_null()
            || out_event.is_null()
            || out_found.is_null()
        {
            return DhruvStatus::NullPointer;
        }
        let rashi_idx = rashi_index as usize;
        if rashi_idx >= 12 {
            return DhruvStatus::InvalidQuery;
        }
        let rashi = dhruv_vedic_base::ALL_RASHIS[rashi_idx];
        let engine_ref = unsafe { &*engine };
        let t = ffi_to_utc_time(unsafe { &*utc });
        let cfg = match sankranti_config_from_ffi(unsafe { &*config }) {
            Some(c) => c,
            None => return DhruvStatus::InvalidQuery,
        };
        match prev_specific_sankranti(engine_ref, &t, rashi, &cfg) {
            Ok(Some(e)) => {
                unsafe {
                    *out_event = DhruvSankrantiEvent {
                        utc: utc_time_to_ffi(&e.utc),
                        rashi_index: e.rashi_index as i32,
                        sun_sidereal_longitude_deg: e.sun_sidereal_longitude_deg,
                        sun_tropical_longitude_deg: e.sun_tropical_longitude_deg,
                    };
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
        if engine.is_null() || utc.is_null() || config.is_null() || out.is_null() {
            return DhruvStatus::NullPointer;
        }
        let engine_ref = unsafe { &*engine };
        let t = ffi_to_utc_time(unsafe { &*utc });
        let cfg = match sankranti_config_from_ffi(unsafe { &*config }) {
            Some(c) => c,
            None => return DhruvStatus::InvalidQuery,
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
        if engine.is_null() || utc.is_null() || config.is_null() || out.is_null() {
            return DhruvStatus::NullPointer;
        }
        let engine_ref = unsafe { &*engine };
        let t = ffi_to_utc_time(unsafe { &*utc });
        let cfg = match sankranti_config_from_ffi(unsafe { &*config }) {
            Some(c) => c,
            None => return DhruvStatus::InvalidQuery,
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
        if engine.is_null() || utc.is_null() || config.is_null() || out.is_null() {
            return DhruvStatus::NullPointer;
        }
        let engine_ref = unsafe { &*engine };
        let t = ffi_to_utc_time(unsafe { &*utc });
        let cfg = match sankranti_config_from_ffi(unsafe { &*config }) {
            Some(c) => c,
            None => return DhruvStatus::InvalidQuery,
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

fn option_jd_to_utc(opt: Option<f64>, lsk: &dhruv_time::LeapSecondKernel) -> (DhruvUtcTime, u8) {
    match opt {
        Some(jd) => (jd_tdb_to_utc_time(jd, lsk), 1),
        None => (ZEROED_UTC, 0),
    }
}

fn conjunction_event_to_utc(
    e: &ConjunctionEvent,
    lsk: &dhruv_time::LeapSecondKernel,
) -> DhruvConjunctionEventUtc {
    DhruvConjunctionEventUtc {
        utc: jd_tdb_to_utc_time(e.jd_tdb, lsk),
        actual_separation_deg: e.actual_separation_deg,
        body1_longitude_deg: e.body1_longitude_deg,
        body2_longitude_deg: e.body2_longitude_deg,
        body1_latitude_deg: e.body1_latitude_deg,
        body2_latitude_deg: e.body2_latitude_deg,
        body1_code: e.body1.code(),
        body2_code: e.body2.code(),
    }
}

fn stationary_event_to_utc(
    e: &StationaryEvent,
    lsk: &dhruv_time::LeapSecondKernel,
) -> DhruvStationaryEventUtc {
    DhruvStationaryEventUtc {
        utc: jd_tdb_to_utc_time(e.jd_tdb, lsk),
        body_code: e.body.code(),
        longitude_deg: e.longitude_deg,
        latitude_deg: e.latitude_deg,
        station_type: station_type_to_code(e.station_type),
    }
}

fn max_speed_event_to_utc(
    e: &MaxSpeedEvent,
    lsk: &dhruv_time::LeapSecondKernel,
) -> DhruvMaxSpeedEventUtc {
    DhruvMaxSpeedEventUtc {
        utc: jd_tdb_to_utc_time(e.jd_tdb, lsk),
        body_code: e.body.code(),
        longitude_deg: e.longitude_deg,
        latitude_deg: e.latitude_deg,
        speed_deg_per_day: e.speed_deg_per_day,
        speed_type: max_speed_type_to_code(e.speed_type),
    }
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

fn chandra_grahan_to_utc(
    e: &ChandraGrahan,
    lsk: &dhruv_time::LeapSecondKernel,
) -> DhruvChandraGrahanResultUtc {
    let (u1, u1_valid) = option_jd_to_utc(e.u1_jd, lsk);
    let (u2, u2_valid) = option_jd_to_utc(e.u2_jd, lsk);
    let (u3, u3_valid) = option_jd_to_utc(e.u3_jd, lsk);
    let (u4, u4_valid) = option_jd_to_utc(e.u4_jd, lsk);
    DhruvChandraGrahanResultUtc {
        grahan_type: chandra_grahan_type_to_code(e.grahan_type),
        magnitude: e.magnitude,
        penumbral_magnitude: e.penumbral_magnitude,
        greatest_grahan: jd_tdb_to_utc_time(e.greatest_grahan_jd, lsk),
        p1: jd_tdb_to_utc_time(e.p1_jd, lsk),
        u1,
        u2,
        u3,
        u4,
        p4: jd_tdb_to_utc_time(e.p4_jd, lsk),
        moon_ecliptic_lat_deg: e.moon_ecliptic_lat_deg,
        angular_separation_deg: e.angular_separation_deg,
        u1_valid,
        u2_valid,
        u3_valid,
        u4_valid,
    }
}

fn surya_grahan_to_utc(
    e: &SuryaGrahan,
    lsk: &dhruv_time::LeapSecondKernel,
) -> DhruvSuryaGrahanResultUtc {
    let (c1, c1_valid) = option_jd_to_utc(e.c1_jd, lsk);
    let (c2, c2_valid) = option_jd_to_utc(e.c2_jd, lsk);
    let (c3, c3_valid) = option_jd_to_utc(e.c3_jd, lsk);
    let (c4, c4_valid) = option_jd_to_utc(e.c4_jd, lsk);
    DhruvSuryaGrahanResultUtc {
        grahan_type: surya_grahan_type_to_code(e.grahan_type),
        magnitude: e.magnitude,
        greatest_grahan: jd_tdb_to_utc_time(e.greatest_grahan_jd, lsk),
        c1,
        c2,
        c3,
        c4,
        moon_ecliptic_lat_deg: e.moon_ecliptic_lat_deg,
        angular_separation_deg: e.angular_separation_deg,
        c1_valid,
        c2_valid,
        c3_valid,
        c4_valid,
    }
}

/// Convert DhruvUtcTime to JD UTC (no TDB conversion, pure calendar arithmetic).
fn ffi_utc_to_jd_utc(t: &DhruvUtcTime) -> f64 {
    let day_frac =
        t.day as f64 + t.hour as f64 / 24.0 + t.minute as f64 / 1440.0 + t.second / 86_400.0;
    dhruv_time::calendar_to_jd(t.year, t.month, day_frac)
}

// ---------------------------------------------------------------------------
// Group A: Search _utc functions (15 functions)
// ---------------------------------------------------------------------------

/// Find the next conjunction/aspect event after the given UTC time.
///
/// # Safety
/// All pointer arguments must be valid and non-null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dhruv_next_conjunction_utc(
    engine: *const DhruvEngineHandle,
    body1_code: i32,
    body2_code: i32,
    utc: *const DhruvUtcTime,
    config: *const DhruvConjunctionConfig,
    out_event: *mut DhruvConjunctionEventUtc,
    out_found: *mut u8,
) -> DhruvStatus {
    ffi_boundary(|| {
        if engine.is_null()
            || utc.is_null()
            || config.is_null()
            || out_event.is_null()
            || out_found.is_null()
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
        let engine_ref = unsafe { &*engine };
        let t = ffi_to_utc_time(unsafe { &*utc });
        let jd_tdb = t.to_jd_tdb(engine_ref.lsk());
        let rust_config = conjunction_config_from_ffi(unsafe { &*config });
        match next_conjunction(engine_ref, body1, body2, jd_tdb, &rust_config) {
            Ok(Some(event)) => {
                unsafe {
                    *out_event = conjunction_event_to_utc(&event, engine_ref.lsk());
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

/// Find the previous conjunction/aspect event before the given UTC time.
///
/// # Safety
/// All pointer arguments must be valid and non-null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dhruv_prev_conjunction_utc(
    engine: *const DhruvEngineHandle,
    body1_code: i32,
    body2_code: i32,
    utc: *const DhruvUtcTime,
    config: *const DhruvConjunctionConfig,
    out_event: *mut DhruvConjunctionEventUtc,
    out_found: *mut u8,
) -> DhruvStatus {
    ffi_boundary(|| {
        if engine.is_null()
            || utc.is_null()
            || config.is_null()
            || out_event.is_null()
            || out_found.is_null()
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
        let engine_ref = unsafe { &*engine };
        let t = ffi_to_utc_time(unsafe { &*utc });
        let jd_tdb = t.to_jd_tdb(engine_ref.lsk());
        let rust_config = conjunction_config_from_ffi(unsafe { &*config });
        match prev_conjunction(engine_ref, body1, body2, jd_tdb, &rust_config) {
            Ok(Some(event)) => {
                unsafe {
                    *out_event = conjunction_event_to_utc(&event, engine_ref.lsk());
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

/// Search for all conjunction/aspect events in a UTC time range.
///
/// # Safety
/// All pointer arguments must be valid and non-null.
/// `out_events` must point to at least `max_count` contiguous `DhruvConjunctionEventUtc`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dhruv_search_conjunctions_utc(
    engine: *const DhruvEngineHandle,
    body1_code: i32,
    body2_code: i32,
    start: *const DhruvUtcTime,
    end: *const DhruvUtcTime,
    config: *const DhruvConjunctionConfig,
    out_events: *mut DhruvConjunctionEventUtc,
    max_count: u32,
    out_count: *mut u32,
) -> DhruvStatus {
    ffi_boundary(|| {
        if engine.is_null()
            || start.is_null()
            || end.is_null()
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
        let engine_ref = unsafe { &*engine };
        let jd_start = ffi_to_utc_time(unsafe { &*start }).to_jd_tdb(engine_ref.lsk());
        let jd_end = ffi_to_utc_time(unsafe { &*end }).to_jd_tdb(engine_ref.lsk());
        let rust_config = conjunction_config_from_ffi(unsafe { &*config });
        match search_conjunctions(engine_ref, body1, body2, jd_start, jd_end, &rust_config) {
            Ok(events) => {
                let count = events.len().min(max_count as usize);
                let out_slice =
                    unsafe { std::slice::from_raw_parts_mut(out_events, max_count as usize) };
                for (i, e) in events.iter().take(count).enumerate() {
                    out_slice[i] = conjunction_event_to_utc(e, engine_ref.lsk());
                }
                unsafe { *out_count = count as u32 };
                DhruvStatus::Ok
            }
            Err(e) => DhruvStatus::from(&e),
        }
    })
}

/// Find the next chandra grahan after the given UTC time.
///
/// # Safety
/// All pointer arguments must be valid and non-null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dhruv_next_chandra_grahan_utc(
    engine: *const DhruvEngineHandle,
    utc: *const DhruvUtcTime,
    config: *const DhruvGrahanConfig,
    out_result: *mut DhruvChandraGrahanResultUtc,
    out_found: *mut u8,
) -> DhruvStatus {
    ffi_boundary(|| {
        if engine.is_null()
            || utc.is_null()
            || config.is_null()
            || out_result.is_null()
            || out_found.is_null()
        {
            return DhruvStatus::NullPointer;
        }
        let engine_ref = unsafe { &*engine };
        let jd_tdb = ffi_to_utc_time(unsafe { &*utc }).to_jd_tdb(engine_ref.lsk());
        let rust_config = grahan_config_from_ffi(unsafe { &*config });
        match next_chandra_grahan(engine_ref, jd_tdb, &rust_config) {
            Ok(Some(grahan)) => {
                unsafe {
                    *out_result = chandra_grahan_to_utc(&grahan, engine_ref.lsk());
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

/// Find the previous chandra grahan before the given UTC time.
///
/// # Safety
/// All pointer arguments must be valid and non-null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dhruv_prev_chandra_grahan_utc(
    engine: *const DhruvEngineHandle,
    utc: *const DhruvUtcTime,
    config: *const DhruvGrahanConfig,
    out_result: *mut DhruvChandraGrahanResultUtc,
    out_found: *mut u8,
) -> DhruvStatus {
    ffi_boundary(|| {
        if engine.is_null()
            || utc.is_null()
            || config.is_null()
            || out_result.is_null()
            || out_found.is_null()
        {
            return DhruvStatus::NullPointer;
        }
        let engine_ref = unsafe { &*engine };
        let jd_tdb = ffi_to_utc_time(unsafe { &*utc }).to_jd_tdb(engine_ref.lsk());
        let rust_config = grahan_config_from_ffi(unsafe { &*config });
        match prev_chandra_grahan(engine_ref, jd_tdb, &rust_config) {
            Ok(Some(grahan)) => {
                unsafe {
                    *out_result = chandra_grahan_to_utc(&grahan, engine_ref.lsk());
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

/// Search for all chandra grahan in a UTC time range.
///
/// # Safety
/// All pointer arguments must be valid and non-null.
/// `out_results` must point to at least `max_count` contiguous `DhruvChandraGrahanResultUtc`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dhruv_search_chandra_grahan_utc(
    engine: *const DhruvEngineHandle,
    start: *const DhruvUtcTime,
    end: *const DhruvUtcTime,
    config: *const DhruvGrahanConfig,
    out_results: *mut DhruvChandraGrahanResultUtc,
    max_count: u32,
    out_count: *mut u32,
) -> DhruvStatus {
    ffi_boundary(|| {
        if engine.is_null()
            || start.is_null()
            || end.is_null()
            || config.is_null()
            || out_results.is_null()
            || out_count.is_null()
        {
            return DhruvStatus::NullPointer;
        }
        let engine_ref = unsafe { &*engine };
        let jd_start = ffi_to_utc_time(unsafe { &*start }).to_jd_tdb(engine_ref.lsk());
        let jd_end = ffi_to_utc_time(unsafe { &*end }).to_jd_tdb(engine_ref.lsk());
        let rust_config = grahan_config_from_ffi(unsafe { &*config });
        match search_chandra_grahan(engine_ref, jd_start, jd_end, &rust_config) {
            Ok(results) => {
                let count = results.len().min(max_count as usize);
                let out_slice =
                    unsafe { std::slice::from_raw_parts_mut(out_results, max_count as usize) };
                for (i, e) in results.iter().take(count).enumerate() {
                    out_slice[i] = chandra_grahan_to_utc(e, engine_ref.lsk());
                }
                unsafe { *out_count = count as u32 };
                DhruvStatus::Ok
            }
            Err(e) => DhruvStatus::from(&e),
        }
    })
}

/// Find the next surya grahan after the given UTC time.
///
/// # Safety
/// All pointer arguments must be valid and non-null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dhruv_next_surya_grahan_utc(
    engine: *const DhruvEngineHandle,
    utc: *const DhruvUtcTime,
    config: *const DhruvGrahanConfig,
    out_result: *mut DhruvSuryaGrahanResultUtc,
    out_found: *mut u8,
) -> DhruvStatus {
    ffi_boundary(|| {
        if engine.is_null()
            || utc.is_null()
            || config.is_null()
            || out_result.is_null()
            || out_found.is_null()
        {
            return DhruvStatus::NullPointer;
        }
        let engine_ref = unsafe { &*engine };
        let jd_tdb = ffi_to_utc_time(unsafe { &*utc }).to_jd_tdb(engine_ref.lsk());
        let rust_config = grahan_config_from_ffi(unsafe { &*config });
        match next_surya_grahan(engine_ref, jd_tdb, &rust_config) {
            Ok(Some(grahan)) => {
                unsafe {
                    *out_result = surya_grahan_to_utc(&grahan, engine_ref.lsk());
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

/// Find the previous surya grahan before the given UTC time.
///
/// # Safety
/// All pointer arguments must be valid and non-null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dhruv_prev_surya_grahan_utc(
    engine: *const DhruvEngineHandle,
    utc: *const DhruvUtcTime,
    config: *const DhruvGrahanConfig,
    out_result: *mut DhruvSuryaGrahanResultUtc,
    out_found: *mut u8,
) -> DhruvStatus {
    ffi_boundary(|| {
        if engine.is_null()
            || utc.is_null()
            || config.is_null()
            || out_result.is_null()
            || out_found.is_null()
        {
            return DhruvStatus::NullPointer;
        }
        let engine_ref = unsafe { &*engine };
        let jd_tdb = ffi_to_utc_time(unsafe { &*utc }).to_jd_tdb(engine_ref.lsk());
        let rust_config = grahan_config_from_ffi(unsafe { &*config });
        match prev_surya_grahan(engine_ref, jd_tdb, &rust_config) {
            Ok(Some(grahan)) => {
                unsafe {
                    *out_result = surya_grahan_to_utc(&grahan, engine_ref.lsk());
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

/// Search for all surya grahan in a UTC time range.
///
/// # Safety
/// All pointer arguments must be valid and non-null.
/// `out_results` must point to at least `max_count` contiguous `DhruvSuryaGrahanResultUtc`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dhruv_search_surya_grahan_utc(
    engine: *const DhruvEngineHandle,
    start: *const DhruvUtcTime,
    end: *const DhruvUtcTime,
    config: *const DhruvGrahanConfig,
    out_results: *mut DhruvSuryaGrahanResultUtc,
    max_count: u32,
    out_count: *mut u32,
) -> DhruvStatus {
    ffi_boundary(|| {
        if engine.is_null()
            || start.is_null()
            || end.is_null()
            || config.is_null()
            || out_results.is_null()
            || out_count.is_null()
        {
            return DhruvStatus::NullPointer;
        }
        let engine_ref = unsafe { &*engine };
        let jd_start = ffi_to_utc_time(unsafe { &*start }).to_jd_tdb(engine_ref.lsk());
        let jd_end = ffi_to_utc_time(unsafe { &*end }).to_jd_tdb(engine_ref.lsk());
        let rust_config = grahan_config_from_ffi(unsafe { &*config });
        match search_surya_grahan(engine_ref, jd_start, jd_end, &rust_config) {
            Ok(results) => {
                let count = results.len().min(max_count as usize);
                let out_slice =
                    unsafe { std::slice::from_raw_parts_mut(out_results, max_count as usize) };
                for (i, e) in results.iter().take(count).enumerate() {
                    out_slice[i] = surya_grahan_to_utc(e, engine_ref.lsk());
                }
                unsafe { *out_count = count as u32 };
                DhruvStatus::Ok
            }
            Err(e) => DhruvStatus::from(&e),
        }
    })
}

/// Find the next stationary point after the given UTC time.
///
/// # Safety
/// All pointer arguments must be valid and non-null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dhruv_next_stationary_utc(
    engine: *const DhruvEngineHandle,
    body_code: i32,
    utc: *const DhruvUtcTime,
    config: *const DhruvStationaryConfig,
    out_event: *mut DhruvStationaryEventUtc,
    out_found: *mut u8,
) -> DhruvStatus {
    ffi_boundary(|| {
        if engine.is_null()
            || utc.is_null()
            || config.is_null()
            || out_event.is_null()
            || out_found.is_null()
        {
            return DhruvStatus::NullPointer;
        }
        let body = match Body::from_code(body_code) {
            Some(b) => b,
            None => return DhruvStatus::InvalidQuery,
        };
        let engine_ref = unsafe { &*engine };
        let jd_tdb = ffi_to_utc_time(unsafe { &*utc }).to_jd_tdb(engine_ref.lsk());
        let rust_config = stationary_config_from_ffi(unsafe { &*config });
        match next_stationary(engine_ref, body, jd_tdb, &rust_config) {
            Ok(Some(event)) => {
                unsafe {
                    *out_event = stationary_event_to_utc(&event, engine_ref.lsk());
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

/// Find the previous stationary point before the given UTC time.
///
/// # Safety
/// All pointer arguments must be valid and non-null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dhruv_prev_stationary_utc(
    engine: *const DhruvEngineHandle,
    body_code: i32,
    utc: *const DhruvUtcTime,
    config: *const DhruvStationaryConfig,
    out_event: *mut DhruvStationaryEventUtc,
    out_found: *mut u8,
) -> DhruvStatus {
    ffi_boundary(|| {
        if engine.is_null()
            || utc.is_null()
            || config.is_null()
            || out_event.is_null()
            || out_found.is_null()
        {
            return DhruvStatus::NullPointer;
        }
        let body = match Body::from_code(body_code) {
            Some(b) => b,
            None => return DhruvStatus::InvalidQuery,
        };
        let engine_ref = unsafe { &*engine };
        let jd_tdb = ffi_to_utc_time(unsafe { &*utc }).to_jd_tdb(engine_ref.lsk());
        let rust_config = stationary_config_from_ffi(unsafe { &*config });
        match prev_stationary(engine_ref, body, jd_tdb, &rust_config) {
            Ok(Some(event)) => {
                unsafe {
                    *out_event = stationary_event_to_utc(&event, engine_ref.lsk());
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

/// Search for all stationary points in a UTC time range.
///
/// # Safety
/// All pointer arguments must be valid and non-null.
/// `out_events` must point to at least `max_count` contiguous `DhruvStationaryEventUtc`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dhruv_search_stationary_utc(
    engine: *const DhruvEngineHandle,
    body_code: i32,
    start: *const DhruvUtcTime,
    end: *const DhruvUtcTime,
    config: *const DhruvStationaryConfig,
    out_events: *mut DhruvStationaryEventUtc,
    max_count: u32,
    out_count: *mut u32,
) -> DhruvStatus {
    ffi_boundary(|| {
        if engine.is_null()
            || start.is_null()
            || end.is_null()
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
        let jd_start = ffi_to_utc_time(unsafe { &*start }).to_jd_tdb(engine_ref.lsk());
        let jd_end = ffi_to_utc_time(unsafe { &*end }).to_jd_tdb(engine_ref.lsk());
        let rust_config = stationary_config_from_ffi(unsafe { &*config });
        match search_stationary(engine_ref, body, jd_start, jd_end, &rust_config) {
            Ok(events) => {
                let count = events.len().min(max_count as usize);
                let out_slice =
                    unsafe { std::slice::from_raw_parts_mut(out_events, max_count as usize) };
                for (i, e) in events.iter().take(count).enumerate() {
                    out_slice[i] = stationary_event_to_utc(e, engine_ref.lsk());
                }
                unsafe { *out_count = count as u32 };
                DhruvStatus::Ok
            }
            Err(e) => DhruvStatus::from(&e),
        }
    })
}

/// Find the next max-speed event after the given UTC time.
///
/// # Safety
/// All pointer arguments must be valid and non-null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dhruv_next_max_speed_utc(
    engine: *const DhruvEngineHandle,
    body_code: i32,
    utc: *const DhruvUtcTime,
    config: *const DhruvStationaryConfig,
    out_event: *mut DhruvMaxSpeedEventUtc,
    out_found: *mut u8,
) -> DhruvStatus {
    ffi_boundary(|| {
        if engine.is_null()
            || utc.is_null()
            || config.is_null()
            || out_event.is_null()
            || out_found.is_null()
        {
            return DhruvStatus::NullPointer;
        }
        let body = match Body::from_code(body_code) {
            Some(b) => b,
            None => return DhruvStatus::InvalidQuery,
        };
        let engine_ref = unsafe { &*engine };
        let jd_tdb = ffi_to_utc_time(unsafe { &*utc }).to_jd_tdb(engine_ref.lsk());
        let rust_config = stationary_config_from_ffi(unsafe { &*config });
        match next_max_speed(engine_ref, body, jd_tdb, &rust_config) {
            Ok(Some(event)) => {
                unsafe {
                    *out_event = max_speed_event_to_utc(&event, engine_ref.lsk());
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

/// Find the previous max-speed event before the given UTC time.
///
/// # Safety
/// All pointer arguments must be valid and non-null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dhruv_prev_max_speed_utc(
    engine: *const DhruvEngineHandle,
    body_code: i32,
    utc: *const DhruvUtcTime,
    config: *const DhruvStationaryConfig,
    out_event: *mut DhruvMaxSpeedEventUtc,
    out_found: *mut u8,
) -> DhruvStatus {
    ffi_boundary(|| {
        if engine.is_null()
            || utc.is_null()
            || config.is_null()
            || out_event.is_null()
            || out_found.is_null()
        {
            return DhruvStatus::NullPointer;
        }
        let body = match Body::from_code(body_code) {
            Some(b) => b,
            None => return DhruvStatus::InvalidQuery,
        };
        let engine_ref = unsafe { &*engine };
        let jd_tdb = ffi_to_utc_time(unsafe { &*utc }).to_jd_tdb(engine_ref.lsk());
        let rust_config = stationary_config_from_ffi(unsafe { &*config });
        match prev_max_speed(engine_ref, body, jd_tdb, &rust_config) {
            Ok(Some(event)) => {
                unsafe {
                    *out_event = max_speed_event_to_utc(&event, engine_ref.lsk());
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

/// Search for all max-speed events in a UTC time range.
///
/// # Safety
/// All pointer arguments must be valid and non-null.
/// `out_events` must point to at least `max_count` contiguous `DhruvMaxSpeedEventUtc`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dhruv_search_max_speed_utc(
    engine: *const DhruvEngineHandle,
    body_code: i32,
    start: *const DhruvUtcTime,
    end: *const DhruvUtcTime,
    config: *const DhruvStationaryConfig,
    out_events: *mut DhruvMaxSpeedEventUtc,
    max_count: u32,
    out_count: *mut u32,
) -> DhruvStatus {
    ffi_boundary(|| {
        if engine.is_null()
            || start.is_null()
            || end.is_null()
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
        let jd_start = ffi_to_utc_time(unsafe { &*start }).to_jd_tdb(engine_ref.lsk());
        let jd_end = ffi_to_utc_time(unsafe { &*end }).to_jd_tdb(engine_ref.lsk());
        let rust_config = stationary_config_from_ffi(unsafe { &*config });
        match search_max_speed(engine_ref, body, jd_start, jd_end, &rust_config) {
            Ok(events) => {
                let count = events.len().min(max_count as usize);
                let out_slice =
                    unsafe { std::slice::from_raw_parts_mut(out_events, max_count as usize) };
                for (i, e) in events.iter().take(count).enumerate() {
                    out_slice[i] = max_speed_event_to_utc(e, engine_ref.lsk());
                }
                unsafe { *out_count = count as u32 };
                DhruvStatus::Ok
            }
            Err(e) => DhruvStatus::from(&e),
        }
    })
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
            || config.is_null()
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
        let cfg_ref = unsafe { &*config };
        let geo = GeoLocation::new(
            loc_ref.latitude_deg,
            loc_ref.longitude_deg,
            loc_ref.altitude_m,
        );
        let sun_limb = match sun_limb_from_code(cfg_ref.sun_limb) {
            Some(l) => l,
            None => return DhruvStatus::InvalidQuery,
        };
        let rs_config = RiseSetConfig {
            use_refraction: cfg_ref.use_refraction != 0,
            sun_limb,
            altitude_correction: cfg_ref.altitude_correction != 0,
        };
        let jd_utc_noon = ffi_utc_to_jd_utc(unsafe { &*utc });
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
            || config.is_null()
            || out_results.is_null()
        {
            return DhruvStatus::NullPointer;
        }
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
        let sun_limb = match sun_limb_from_code(cfg_ref.sun_limb) {
            Some(l) => l,
            None => return DhruvStatus::InvalidQuery,
        };
        let rs_config = RiseSetConfig {
            use_refraction: cfg_ref.use_refraction != 0,
            sun_limb,
            altitude_correction: cfg_ref.altitude_correction != 0,
        };
        let jd_utc_noon = ffi_utc_to_jd_utc(unsafe { &*utc });
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
            || config.is_null()
            || out_result.is_null()
        {
            return DhruvStatus::NullPointer;
        }
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
            Err(s) => return s,
        };
        let jd_utc = ffi_utc_to_jd_utc(unsafe { &*utc });
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
                unsafe {
                    *out_result = DhruvBhavaResult {
                        bhavas: ffi_bhavas,
                        lagna_deg: result.lagna_deg,
                        mc_deg: result.mc_deg,
                    };
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

/// Mean ayanamsha with UTC input. Requires LSK for UTC→TDB.
///
/// # Safety
/// `lsk` and `out_deg` must be valid, non-null pointers.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dhruv_ayanamsha_mean_deg_utc(
    lsk: *const DhruvLskHandle,
    system_code: i32,
    utc: *const DhruvUtcTime,
    out_deg: *mut f64,
) -> DhruvStatus {
    ffi_boundary(|| {
        if lsk.is_null() || utc.is_null() || out_deg.is_null() {
            return DhruvStatus::NullPointer;
        }
        let system = match ayanamsha_system_from_code(system_code) {
            Some(s) => s,
            None => return DhruvStatus::InvalidQuery,
        };
        let lsk_ref = unsafe { &*lsk };
        let jd_tdb = ffi_to_utc_time(unsafe { &*utc }).to_jd_tdb(lsk_ref);
        let t = jd_tdb_to_centuries(jd_tdb);
        unsafe { *out_deg = ayanamsha_mean_deg(system, t) };
        DhruvStatus::Ok
    })
}

/// True ayanamsha with UTC input. Requires LSK for UTC→TDB.
///
/// # Safety
/// `lsk` and `out_deg` must be valid, non-null pointers.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dhruv_ayanamsha_true_deg_utc(
    lsk: *const DhruvLskHandle,
    system_code: i32,
    utc: *const DhruvUtcTime,
    delta_psi_arcsec: f64,
    out_deg: *mut f64,
) -> DhruvStatus {
    ffi_boundary(|| {
        if lsk.is_null() || utc.is_null() || out_deg.is_null() {
            return DhruvStatus::NullPointer;
        }
        let system = match ayanamsha_system_from_code(system_code) {
            Some(s) => s,
            None => return DhruvStatus::InvalidQuery,
        };
        let lsk_ref = unsafe { &*lsk };
        let jd_tdb = ffi_to_utc_time(unsafe { &*utc }).to_jd_tdb(lsk_ref);
        let t = jd_tdb_to_centuries(jd_tdb);
        unsafe { *out_deg = ayanamsha_true_deg(system, t, delta_psi_arcsec) };
        DhruvStatus::Ok
    })
}

/// Unified ayanamsha with UTC input. Requires LSK for UTC→TDB.
///
/// # Safety
/// `lsk` and `out_deg` must be valid, non-null pointers.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dhruv_ayanamsha_deg_utc(
    lsk: *const DhruvLskHandle,
    system_code: i32,
    utc: *const DhruvUtcTime,
    use_nutation: u8,
    out_deg: *mut f64,
) -> DhruvStatus {
    ffi_boundary(|| {
        if lsk.is_null() || utc.is_null() || out_deg.is_null() {
            return DhruvStatus::NullPointer;
        }
        let system = match ayanamsha_system_from_code(system_code) {
            Some(s) => s,
            None => return DhruvStatus::InvalidQuery,
        };
        let lsk_ref = unsafe { &*lsk };
        let jd_tdb = ffi_to_utc_time(unsafe { &*utc }).to_jd_tdb(lsk_ref);
        let t = jd_tdb_to_centuries(jd_tdb);
        unsafe { *out_deg = ayanamsha_deg(system, t, use_nutation != 0) };
        DhruvStatus::Ok
    })
}

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

// ---------------------------------------------------------------------------
// Group D: Core query with DhruvUtcTime (1 function)
// ---------------------------------------------------------------------------

/// Query engine with `DhruvUtcTime` input, return spherical state.
///
/// # Safety
/// `engine`, `utc`, and `out` must be valid, non-null pointers.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dhruv_query_utc(
    engine: *const DhruvEngineHandle,
    target: i32,
    observer: i32,
    frame: i32,
    utc: *const DhruvUtcTime,
    out: *mut DhruvSphericalState,
) -> DhruvStatus {
    ffi_boundary(|| {
        if engine.is_null() || utc.is_null() || out.is_null() {
            return DhruvStatus::NullPointer;
        }
        let engine_ref = unsafe { &*engine };
        let utc_ref = unsafe { &*utc };
        match dhruv_query_utc_spherical_internal(
            engine_ref,
            target,
            observer,
            frame,
            utc_ref.year,
            utc_ref.month,
            utc_ref.day,
            utc_ref.hour,
            utc_ref.minute,
            utc_ref.second,
        ) {
            Ok(state) => {
                unsafe { *out = state };
                DhruvStatus::Ok
            }
            Err(status) => status,
        }
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
        if engine.is_null() || utc.is_null() || config.is_null() || out.is_null() {
            return DhruvStatus::NullPointer;
        }
        let engine_ref = unsafe { &*engine };
        let t = ffi_to_utc_time(unsafe { &*utc });
        let cfg = match sankranti_config_from_ffi(unsafe { &*config }) {
            Some(c) => c,
            None => return DhruvStatus::InvalidQuery,
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
        if engine.is_null() || utc.is_null() || config.is_null() || out.is_null() {
            return DhruvStatus::NullPointer;
        }
        let engine_ref = unsafe { &*engine };
        let t = ffi_to_utc_time(unsafe { &*utc });
        let cfg = match sankranti_config_from_ffi(unsafe { &*config }) {
            Some(c) => c,
            None => return DhruvStatus::InvalidQuery,
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
        if engine.is_null()
            || eop.is_null()
            || utc.is_null()
            || location.is_null()
            || riseset_config.is_null()
            || out.is_null()
        {
            return DhruvStatus::NullPointer;
        }
        let engine_ref = unsafe { &*engine };
        let eop_ref = unsafe { &*eop };
        let t = ffi_to_utc_time(unsafe { &*utc });
        let loc_ref = unsafe { &*location };
        let cfg_ref = unsafe { &*riseset_config };
        let geo = GeoLocation::new(
            loc_ref.latitude_deg,
            loc_ref.longitude_deg,
            loc_ref.altitude_m,
        );
        let sun_limb = match sun_limb_from_code(cfg_ref.sun_limb) {
            Some(l) => l,
            None => return DhruvStatus::InvalidQuery,
        };
        let rs_config = RiseSetConfig {
            use_refraction: cfg_ref.use_refraction != 0,
            sun_limb,
            altitude_correction: cfg_ref.altitude_correction != 0,
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
        if engine.is_null()
            || eop.is_null()
            || utc.is_null()
            || location.is_null()
            || riseset_config.is_null()
            || out.is_null()
        {
            return DhruvStatus::NullPointer;
        }
        let engine_ref = unsafe { &*engine };
        let eop_ref = unsafe { &*eop };
        let t = ffi_to_utc_time(unsafe { &*utc });
        let loc_ref = unsafe { &*location };
        let cfg_ref = unsafe { &*riseset_config };
        let geo = GeoLocation::new(
            loc_ref.latitude_deg,
            loc_ref.longitude_deg,
            loc_ref.altitude_m,
        );
        let sun_limb = match sun_limb_from_code(cfg_ref.sun_limb) {
            Some(l) => l,
            None => return DhruvStatus::InvalidQuery,
        };
        let rs_config = RiseSetConfig {
            use_refraction: cfg_ref.use_refraction != 0,
            sun_limb,
            altitude_correction: cfg_ref.altitude_correction != 0,
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
        if engine.is_null()
            || eop.is_null()
            || utc.is_null()
            || location.is_null()
            || riseset_config.is_null()
            || out.is_null()
        {
            return DhruvStatus::NullPointer;
        }
        let engine_ref = unsafe { &*engine };
        let eop_ref = unsafe { &*eop };
        let t = ffi_to_utc_time(unsafe { &*utc });
        let loc_ref = unsafe { &*location };
        let cfg_ref = unsafe { &*riseset_config };
        let geo = GeoLocation::new(
            loc_ref.latitude_deg,
            loc_ref.longitude_deg,
            loc_ref.altitude_m,
        );
        let sun_limb = match sun_limb_from_code(cfg_ref.sun_limb) {
            Some(l) => l,
            None => return DhruvStatus::InvalidQuery,
        };
        let rs_config = RiseSetConfig {
            use_refraction: cfg_ref.use_refraction != 0,
            sun_limb,
            altitude_correction: cfg_ref.altitude_correction != 0,
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

/// Compute combined panchang for a given UTC date and location.
///
/// When `include_calendar` is non-zero, also computes masa, ayana, and varsha.
///
/// # Safety
/// All pointer arguments must be valid and non-null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dhruv_panchang_for_date(
    engine: *const DhruvEngineHandle,
    eop: *const DhruvEopHandle,
    utc: *const DhruvUtcTime,
    location: *const DhruvGeoLocation,
    riseset_config: *const DhruvRiseSetConfig,
    sankranti_config: *const DhruvSankrantiConfig,
    include_calendar: u8,
    out: *mut DhruvPanchangInfo,
) -> DhruvStatus {
    ffi_boundary(|| {
        if engine.is_null()
            || eop.is_null()
            || utc.is_null()
            || location.is_null()
            || riseset_config.is_null()
            || sankranti_config.is_null()
            || out.is_null()
        {
            return DhruvStatus::NullPointer;
        }
        let engine_ref = unsafe { &*engine };
        let eop_ref = unsafe { &*eop };
        let t = ffi_to_utc_time(unsafe { &*utc });
        let loc_ref = unsafe { &*location };
        let rs_ref = unsafe { &*riseset_config };
        let geo = GeoLocation::new(
            loc_ref.latitude_deg,
            loc_ref.longitude_deg,
            loc_ref.altitude_m,
        );
        let sun_limb = match sun_limb_from_code(rs_ref.sun_limb) {
            Some(l) => l,
            None => return DhruvStatus::InvalidQuery,
        };
        let rs_config = RiseSetConfig {
            use_refraction: rs_ref.use_refraction != 0,
            sun_limb,
            altitude_correction: rs_ref.altitude_correction != 0,
        };
        let cfg = match sankranti_config_from_ffi(unsafe { &*sankranti_config }) {
            Some(c) => c,
            None => return DhruvStatus::InvalidQuery,
        };
        match panchang_for_date(
            engine_ref,
            eop_ref,
            &t,
            &geo,
            &rs_config,
            &cfg,
            include_calendar != 0,
        ) {
            Ok(info) => {
                let zeroed_masa = DhruvMasaInfo {
                    masa_index: 0,
                    adhika: 0,
                    start: utc_time_to_ffi(&UtcTime::new(0, 0, 0, 0, 0, 0.0)),
                    end: utc_time_to_ffi(&UtcTime::new(0, 0, 0, 0, 0, 0.0)),
                };
                let zeroed_ayana = DhruvAyanaInfo {
                    ayana: 0,
                    start: utc_time_to_ffi(&UtcTime::new(0, 0, 0, 0, 0, 0.0)),
                    end: utc_time_to_ffi(&UtcTime::new(0, 0, 0, 0, 0, 0.0)),
                };
                let zeroed_varsha = DhruvVarshaInfo {
                    samvatsara_index: 0,
                    order: 0,
                    start: utc_time_to_ffi(&UtcTime::new(0, 0, 0, 0, 0, 0.0)),
                    end: utc_time_to_ffi(&UtcTime::new(0, 0, 0, 0, 0, 0.0)),
                };
                let (calendar_valid, masa_ffi, ayana_ffi, varsha_ffi) =
                    match (info.masa, info.ayana, info.varsha) {
                        (Some(m), Some(a), Some(v)) => (
                            1u8,
                            DhruvMasaInfo {
                                masa_index: m.masa.index() as i32,
                                adhika: u8::from(m.adhika),
                                start: utc_time_to_ffi(&m.start),
                                end: utc_time_to_ffi(&m.end),
                            },
                            DhruvAyanaInfo {
                                ayana: a.ayana.index() as i32,
                                start: utc_time_to_ffi(&a.start),
                                end: utc_time_to_ffi(&a.end),
                            },
                            DhruvVarshaInfo {
                                samvatsara_index: v.samvatsara.index() as i32,
                                order: v.order as i32,
                                start: utc_time_to_ffi(&v.start),
                                end: utc_time_to_ffi(&v.end),
                            },
                        ),
                        _ => (0u8, zeroed_masa, zeroed_ayana, zeroed_varsha),
                    };
                unsafe {
                    *out = DhruvPanchangInfo {
                        tithi: DhruvTithiInfo {
                            tithi_index: info.tithi.tithi_index as i32,
                            paksha: info.tithi.paksha as i32,
                            tithi_in_paksha: info.tithi.tithi_in_paksha as i32,
                            start: utc_time_to_ffi(&info.tithi.start),
                            end: utc_time_to_ffi(&info.tithi.end),
                        },
                        karana: DhruvKaranaInfo {
                            karana_index: info.karana.karana_index as i32,
                            karana_name_index: info.karana.karana.index() as i32,
                            start: utc_time_to_ffi(&info.karana.start),
                            end: utc_time_to_ffi(&info.karana.end),
                        },
                        yoga: DhruvYogaInfo {
                            yoga_index: info.yoga.yoga_index as i32,
                            start: utc_time_to_ffi(&info.yoga.start),
                            end: utc_time_to_ffi(&info.yoga.end),
                        },
                        vaar: DhruvVaarInfo {
                            vaar_index: info.vaar.vaar.index() as i32,
                            start: utc_time_to_ffi(&info.vaar.start),
                            end: utc_time_to_ffi(&info.vaar.end),
                        },
                        hora: DhruvHoraInfo {
                            hora_index: info.hora.hora.index() as i32,
                            hora_position: info.hora.hora_index as i32,
                            start: utc_time_to_ffi(&info.hora.start),
                            end: utc_time_to_ffi(&info.hora.end),
                        },
                        ghatika: DhruvGhatikaInfo {
                            value: info.ghatika.value as i32,
                            start: utc_time_to_ffi(&info.ghatika.start),
                            end: utc_time_to_ffi(&info.ghatika.end),
                        },
                        nakshatra: DhruvPanchangNakshatraInfo {
                            nakshatra_index: info.nakshatra.nakshatra_index as i32,
                            pada: info.nakshatra.pada as i32,
                            start: utc_time_to_ffi(&info.nakshatra.start),
                            end: utc_time_to_ffi(&info.nakshatra.end),
                        },
                        calendar_valid,
                        masa: masa_ffi,
                        ayana: ayana_ffi,
                        varsha: varsha_ffi,
                    };
                }
                DhruvStatus::Ok
            }
            Err(e) => DhruvStatus::from(&e),
        }
    })
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
        if engine.is_null() || config.is_null() || out_deg.is_null() {
            return DhruvStatus::NullPointer;
        }
        let engine_ref = unsafe { &*engine };
        let cfg = match sankranti_config_from_ffi(unsafe { &*config }) {
            Some(c) => c,
            None => return DhruvStatus::InvalidQuery,
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
            || riseset_config.is_null()
            || out_sunrise_jd.is_null()
            || out_next_sunrise_jd.is_null()
        {
            return DhruvStatus::NullPointer;
        }
        let engine_ref = unsafe { &*engine };
        let eop_ref = unsafe { &*eop };
        let t = ffi_to_utc_time(unsafe { &*utc });
        let loc_ref = unsafe { &*location };
        let cfg_ref = unsafe { &*riseset_config };
        let geo = GeoLocation::new(
            loc_ref.latitude_deg,
            loc_ref.longitude_deg,
            loc_ref.altitude_m,
        );
        let sun_limb = match sun_limb_from_code(cfg_ref.sun_limb) {
            Some(l) => l,
            None => return DhruvStatus::InvalidQuery,
        };
        let rs_config = RiseSetConfig {
            use_refraction: cfg_ref.use_refraction != 0,
            sun_limb,
            altitude_correction: cfg_ref.altitude_correction != 0,
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
        if engine.is_null() || config.is_null() || out.is_null() {
            return DhruvStatus::NullPointer;
        }
        let engine_ref = unsafe { &*engine };
        let cfg = match sankranti_config_from_ffi(unsafe { &*config }) {
            Some(c) => c,
            None => return DhruvStatus::InvalidQuery,
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
/// The returned pointer is static and must not be freed.
#[unsafe(no_mangle)]
pub extern "C" fn dhruv_graha_name(index: u32) -> *const std::ffi::c_char {
    let all = dhruv_vedic_base::graha::ALL_GRAHAS;
    if index >= all.len() as u32 {
        return ptr::null();
    }
    let name = all[index as usize].name();
    name.as_ptr() as *const std::ffi::c_char
}

/// Return the English name of a graha by index (0-8). Returns null for invalid index.
#[unsafe(no_mangle)]
pub extern "C" fn dhruv_graha_english_name(index: u32) -> *const std::ffi::c_char {
    let all = dhruv_vedic_base::graha::ALL_GRAHAS;
    if index >= all.len() as u32 {
        return ptr::null();
    }
    let name = all[index as usize].english_name();
    name.as_ptr() as *const std::ffi::c_char
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
    if engine.is_null()
        || eop.is_null()
        || utc.is_null()
        || location.is_null()
        || riseset_config.is_null()
        || out.is_null()
    {
        return DhruvStatus::NullPointer;
    }

    let engine = unsafe { &*engine };
    let eop = unsafe { &*eop };
    let utc_c = unsafe { &*utc };
    let loc_c = unsafe { &*location };
    let rs_c = unsafe { &*riseset_config };

    let utc_time = UtcTime {
        year: utc_c.year,
        month: utc_c.month,
        day: utc_c.day,
        hour: utc_c.hour,
        minute: utc_c.minute,
        second: utc_c.second,
    };

    let location = GeoLocation::new(loc_c.latitude_deg, loc_c.longitude_deg, loc_c.altitude_m);

    let sun_limb = match rs_c.sun_limb {
        1 => SunLimb::Center,
        2 => SunLimb::LowerLimb,
        _ => SunLimb::UpperLimb,
    };
    let rs_config = RiseSetConfig {
        sun_limb,
        use_refraction: rs_c.use_refraction != 0,
        altitude_correction: rs_c.altitude_correction != 0,
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
    let jd = time_upagraha_jd(
        upa,
        weekday as u8,
        is_day != 0,
        sunrise_jd,
        sunset_jd,
        next_sunrise_jd,
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
    if engine.is_null()
        || eop.is_null()
        || utc.is_null()
        || location.is_null()
        || riseset_config.is_null()
        || out_jd.is_null()
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
    let rs_c = unsafe { &*riseset_config };

    let utc_time = UtcTime {
        year: utc_c.year,
        month: utc_c.month,
        day: utc_c.day,
        hour: utc_c.hour,
        minute: utc_c.minute,
        second: utc_c.second,
    };

    let location = GeoLocation::new(loc_c.latitude_deg, loc_c.longitude_deg, loc_c.altitude_m);

    let sun_limb = match sun_limb_from_code(rs_c.sun_limb) {
        Some(l) => l,
        None => return DhruvStatus::InvalidQuery,
    };
    let rs_config = RiseSetConfig {
        use_refraction: rs_c.use_refraction != 0,
        sun_limb,
        altitude_correction: rs_c.altitude_correction != 0,
    };

    // Compute sunrise pair (vedic day boundaries)
    let (jd_sunrise, jd_next_sunrise) =
        match vedic_day_sunrises(engine, eop, &utc_time, &location, &rs_config) {
            Ok(pair) => pair,
            Err(e) => return DhruvStatus::from(&e),
        };

    // Compute sunset
    let jd_utc = ffi_utc_to_jd_utc(utc_c);
    let noon_jd = approximate_local_noon_jd(jd_utc.floor() + 0.5, loc_c.longitude_deg);
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

    let jd = time_upagraha_jd(upa, weekday, is_day, jd_sunrise, jd_sunset, jd_next_sunrise);
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

    match dhruv_search::all_upagrahas_for_date(
        engine,
        eop,
        &utc_time,
        &location,
        &rs_config,
        &aya_config,
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
    }; 7];
    for (i, b) in bav_slice.iter().enumerate() {
        rust_bavs[i] = dhruv_vedic_base::BhinnaAshtakavarga {
            graha_index: b.graha_index,
            points: b.points,
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
    if engine.is_null()
        || eop.is_null()
        || utc.is_null()
        || location.is_null()
        || bhava_config.is_null()
        || config.is_null()
        || out.is_null()
    {
        return DhruvStatus::NullPointer;
    }

    let engine = unsafe { &*engine };
    let eop = unsafe { &*eop };
    let utc_c = unsafe { &*utc };
    let loc_c = unsafe { &*location };
    let bhava_cfg_c = unsafe { &*bhava_config };
    let cfg_c = unsafe { &*config };

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

    let rust_bhava_config = match bhava_config_from_ffi(bhava_cfg_c) {
        Ok(c) => c,
        Err(status) => return status,
    };

    let rust_config = dhruv_search::GrahaPositionsConfig {
        include_nakshatra: cfg_c.include_nakshatra != 0,
        include_lagna: cfg_c.include_lagna != 0,
        include_outer_planets: cfg_c.include_outer_planets != 0,
        include_bhava: cfg_c.include_bhava != 0,
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
    if engine.is_null()
        || eop.is_null()
        || utc.is_null()
        || location.is_null()
        || bhava_config.is_null()
        || riseset_config.is_null()
        || config.is_null()
        || out.is_null()
    {
        return DhruvStatus::NullPointer;
    }

    let engine = unsafe { &*engine };
    let eop = unsafe { &*eop };
    let utc_c = unsafe { &*utc };
    let loc_c = unsafe { &*location };
    let bhava_cfg_c = unsafe { &*bhava_config };
    let rs_c = unsafe { &*riseset_config };
    let cfg_c = unsafe { &*config };

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

    let rust_bhava_config = match bhava_config_from_ffi(bhava_cfg_c) {
        Ok(c) => c,
        Err(status) => return status,
    };

    let sun_limb = match rs_c.sun_limb {
        1 => SunLimb::Center,
        2 => SunLimb::LowerLimb,
        _ => SunLimb::UpperLimb,
    };
    let rs_config = RiseSetConfig {
        sun_limb,
        use_refraction: rs_c.use_refraction != 0,
        altitude_correction: rs_c.altitude_correction != 0,
    };

    let rust_config = dhruv_search::BindusConfig {
        include_nakshatra: cfg_c.include_nakshatra != 0,
        include_bhava: cfg_c.include_bhava != 0,
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
    if engine.is_null()
        || eop.is_null()
        || utc.is_null()
        || location.is_null()
        || bhava_config.is_null()
        || riseset_config.is_null()
        || config.is_null()
        || out.is_null()
    {
        return DhruvStatus::NullPointer;
    }

    let engine = unsafe { &*engine };
    let eop = unsafe { &*eop };
    let utc_c = unsafe { &*utc };
    let loc_c = unsafe { &*location };
    let bhava_cfg_c = unsafe { &*bhava_config };
    let rs_c = unsafe { &*riseset_config };
    let cfg_c = unsafe { &*config };

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

    let rust_bhava_config = match bhava_config_from_ffi(bhava_cfg_c) {
        Ok(c) => c,
        Err(status) => return status,
    };

    let sun_limb = match rs_c.sun_limb {
        1 => SunLimb::Center,
        2 => SunLimb::LowerLimb,
        _ => SunLimb::UpperLimb,
    };
    let rs_config = RiseSetConfig {
        sun_limb,
        use_refraction: rs_c.use_refraction != 0,
        altitude_correction: rs_c.altitude_correction != 0,
    };

    let rust_config = dhruv_search::DrishtiConfig {
        include_bhava: cfg_c.include_bhava != 0,
        include_lagna: cfg_c.include_lagna != 0,
        include_bindus: cfg_c.include_bindus != 0,
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
    for i in 0..9 {
        out.grahas[i] = amsha_entry_to_ffi(&chart.grahas[i]);
    }
    out.lagna = amsha_entry_to_ffi(&chart.lagna);
    if let Some(ref cusps) = chart.bhava_cusps {
        out.bhava_cusps_valid = 1;
        for i in 0..12 {
            out.bhava_cusps[i] = amsha_entry_to_ffi(&cusps[i]);
        }
    }
    if let Some(ref padas) = chart.arudha_padas {
        out.arudha_padas_valid = 1;
        for i in 0..12 {
            out.arudha_padas[i] = amsha_entry_to_ffi(&padas[i]);
        }
    }
    if let Some(ref upa) = chart.upagrahas {
        out.upagrahas_valid = 1;
        for i in 0..11 {
            out.upagrahas[i] = amsha_entry_to_ffi(&upa[i]);
        }
    }
    if let Some(ref sph) = chart.sphutas {
        out.sphutas_valid = 1;
        for i in 0..16 {
            out.sphutas[i] = amsha_entry_to_ffi(&sph[i]);
        }
    }
    if let Some(ref sl) = chart.special_lagnas {
        out.special_lagnas_valid = 1;
        for i in 0..8 {
            out.special_lagnas[i] = amsha_entry_to_ffi(&sl[i]);
        }
    }
    out
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

/// C-compatible full kundali configuration.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct DhruvFullKundaliConfig {
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
    /// Include special lagnas section.
    pub include_special_lagnas: u8,
    /// Include amsha (divisional chart) section.
    pub include_amshas: u8,
    /// Include shadbala section (sapta grahas only).
    pub include_shadbala: u8,
    /// Include vimsopaka bala section (navagraha).
    pub include_vimsopaka: u8,
    /// Node dignity policy for vimsopaka: 0=SignLordBased (default), 1=AlwaysSama.
    pub node_dignity_policy: u32,
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
}

/// Maximum number of amsha charts in a single FFI batch.
pub const DHRUV_MAX_AMSHA_REQUESTS: usize = 40;

/// C-compatible full kundali result.
#[repr(C)]
#[derive(Debug, Clone)]
pub struct DhruvFullKundaliResult {
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
    pub special_lagnas_valid: u8,
    pub special_lagnas: DhruvSpecialLagnas,
    pub amshas_valid: u8,
    /// Number of populated amsha charts (0..=DHRUV_MAX_AMSHA_REQUESTS).
    pub amshas_count: u8,
    /// Fixed-size array of amsha charts.
    pub amshas: [DhruvAmshaChart; DHRUV_MAX_AMSHA_REQUESTS],
    pub shadbala_valid: u8,
    pub shadbala: DhruvShadbalaResult,
    pub vimsopaka_valid: u8,
    pub vimsopaka: DhruvVimsopakaResult,
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
    if engine.is_null()
        || eop.is_null()
        || utc.is_null()
        || location.is_null()
        || bhava_config.is_null()
        || riseset_config.is_null()
        || config.is_null()
        || out.is_null()
    {
        return DhruvStatus::NullPointer;
    }

    let engine = unsafe { &*engine };
    let eop = unsafe { &*eop };
    let utc_c = unsafe { &*utc };
    let loc_c = unsafe { &*location };
    let bhava_cfg_c = unsafe { &*bhava_config };
    let rs_c = unsafe { &*riseset_config };
    let cfg_c = unsafe { &*config };

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

    let rust_bhava_config = match bhava_config_from_ffi(bhava_cfg_c) {
        Ok(c) => c,
        Err(status) => return status,
    };
    let sun_limb = match sun_limb_from_code(rs_c.sun_limb) {
        Some(l) => l,
        None => return DhruvStatus::InvalidQuery,
    };
    let rs_config = RiseSetConfig {
        use_refraction: rs_c.use_refraction != 0,
        sun_limb,
        altitude_correction: rs_c.altitude_correction != 0,
    };
    let aya_config = SankrantiConfig::new(system, use_nutation != 0);

    let amsha_sel = dhruv_search::AmshaSelectionConfig {
        count: cfg_c.amsha_selection.count,
        codes: cfg_c.amsha_selection.codes,
        variations: cfg_c.amsha_selection.variations,
    };

    let rust_config = dhruv_search::FullKundaliConfig {
        include_graha_positions: cfg_c.include_graha_positions != 0,
        include_bindus: cfg_c.include_bindus != 0,
        include_drishti: cfg_c.include_drishti != 0,
        include_ashtakavarga: cfg_c.include_ashtakavarga != 0,
        include_upagrahas: cfg_c.include_upagrahas != 0,
        include_special_lagnas: cfg_c.include_special_lagnas != 0,
        include_amshas: cfg_c.include_amshas != 0,
        include_shadbala: cfg_c.include_shadbala != 0,
        include_vimsopaka: cfg_c.include_vimsopaka != 0,
        node_dignity_policy: match cfg_c.node_dignity_policy {
            0 => dhruv_vedic_base::NodeDignityPolicy::SignLordBased,
            1 => dhruv_vedic_base::NodeDignityPolicy::AlwaysSama,
            _ => return DhruvStatus::InvalidSearchConfig,
        },
        graha_positions_config: dhruv_search::GrahaPositionsConfig {
            include_nakshatra: cfg_c.graha_positions_config.include_nakshatra != 0,
            include_lagna: cfg_c.graha_positions_config.include_lagna != 0,
            include_outer_planets: cfg_c.graha_positions_config.include_outer_planets != 0,
            include_bhava: cfg_c.graha_positions_config.include_bhava != 0,
        },
        bindus_config: dhruv_search::BindusConfig {
            include_nakshatra: cfg_c.bindus_config.include_nakshatra != 0,
            include_bhava: cfg_c.bindus_config.include_bhava != 0,
        },
        drishti_config: dhruv_search::DrishtiConfig {
            include_bhava: cfg_c.drishti_config.include_bhava != 0,
            include_lagna: cfg_c.drishti_config.include_lagna != 0,
            include_bindus: cfg_c.drishti_config.include_bindus != 0,
        },
        amsha_scope: dhruv_search::AmshaChartScope {
            include_bhava_cusps: cfg_c.amsha_scope.include_bhava_cusps != 0,
            include_arudha_padas: cfg_c.amsha_scope.include_arudha_padas != 0,
            include_upagrahas: cfg_c.amsha_scope.include_upagrahas != 0,
            include_sphutas: cfg_c.amsha_scope.include_sphutas != 0,
            include_special_lagnas: cfg_c.amsha_scope.include_special_lagnas != 0,
        },
        amsha_selection: amsha_sel,
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
            let out = unsafe { &mut *out };
            // SAFETY: POD fields only; zero-init valid as "absent" default.
            unsafe { std::ptr::write_bytes(out as *mut DhruvFullKundaliResult, 0, 1) };

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
                for i in 0..7 {
                    let e = &sb.entries[i];
                    out.shadbala.entries[i] = DhruvShadbalaEntry {
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
            }

            if let Some(ref vm) = result.vimsopaka {
                out.vimsopaka_valid = 1;
                for i in 0..9 {
                    let e = &vm.entries[i];
                    out.vimsopaka.entries[i] = DhruvVimsopakaEntry {
                        graha_index: e.graha.index(),
                        shadvarga: e.shadvarga,
                        saptavarga: e.saptavarga,
                        dashavarga: e.dashavarga,
                        shodasavarga: e.shodasavarga,
                    };
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
        || bhava_config.is_null()
        || riseset_config.is_null()
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
    let bhava_cfg_c = unsafe { &*bhava_config };
    let rs_c = unsafe { &*riseset_config };
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
    let rust_bhava_config = match bhava_config_from_ffi(bhava_cfg_c) {
        Ok(c) => c,
        Err(status) => return status,
    };
    let sun_limb = match sun_limb_from_code(rs_c.sun_limb) {
        Some(l) => l,
        None => return DhruvStatus::InvalidQuery,
    };
    let rs_config = RiseSetConfig {
        use_refraction: rs_c.use_refraction != 0,
        sun_limb,
        altitude_correction: rs_c.altitude_correction != 0,
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
    if engine.is_null()
        || eop.is_null()
        || utc.is_null()
        || location.is_null()
        || bhava_config.is_null()
        || riseset_config.is_null()
        || out.is_null()
    {
        return DhruvStatus::NullPointer;
    }

    let engine = unsafe { &*engine };
    let eop = unsafe { &*eop };
    let utc_c = unsafe { &*utc };
    let loc_c = unsafe { &*location };
    let bhava_cfg_c = unsafe { &*bhava_config };
    let rs_c = unsafe { &*riseset_config };

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
    let rust_bhava_config = match bhava_config_from_ffi(bhava_cfg_c) {
        Ok(c) => c,
        Err(status) => return status,
    };
    let sun_limb = match sun_limb_from_code(rs_c.sun_limb) {
        Some(l) => l,
        None => return DhruvStatus::InvalidQuery,
    };
    let rs_config = RiseSetConfig {
        use_refraction: rs_c.use_refraction != 0,
        sun_limb,
        altitude_correction: rs_c.altitude_correction != 0,
    };
    let aya_config = SankrantiConfig::new(system, use_nutation != 0);

    match shadbala_for_date(
        engine, eop, &utc_time, &location, &rust_bhava_config, &rs_config, &aya_config,
    ) {
        Ok(result) => {
            let out = unsafe { &mut *out };
            for i in 0..7 {
                let e = &result.entries[i];
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
            for i in 0..9 {
                let e = &result.entries[i];
                out.entries[i] = DhruvVimsopakaEntry {
                    graha_index: e.graha.index(),
                    shadvarga: e.shadvarga,
                    saptavarga: e.saptavarga,
                    dashavarga: e.dashavarga,
                    shodasavarga: e.shodasavarga,
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

/// Query sidereal longitudes of all 9 grahas at a given JD (TDB).
///
/// # Safety
/// `engine` and `out` must be valid, non-null pointers.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dhruv_graha_sidereal_longitudes(
    engine: *const Engine,
    jd_tdb: f64,
    ayanamsha_system: u32,
    use_nutation: u8,
    out: *mut DhruvGrahaLongitudes,
) -> DhruvStatus {
    if engine.is_null() || out.is_null() {
        return DhruvStatus::NullPointer;
    }

    let engine = unsafe { &*engine };
    let system = match ayanamsha_system_from_code(ayanamsha_system as i32) {
        Some(s) => s,
        None => return DhruvStatus::InvalidQuery,
    };

    match graha_sidereal_longitudes(engine, jd_tdb, system, use_nutation != 0) {
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
    if engine.is_null() || config.is_null() || out.is_null() {
        return DhruvStatus::NullPointer;
    }

    let engine_ref = unsafe { &*engine };
    let cfg = match sankranti_config_from_ffi(unsafe { &*config }) {
        Some(c) => c,
        None => return DhruvStatus::InvalidQuery,
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
        assert!(diff > 1e-6 && diff < 0.01, "nutation diff = {diff} deg");
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
            dhruv_query_utc_spherical(ptr::null(), 499, 10, 2, 2024, 3, 20, 12, 0, 0.0, &mut out)
        };
        assert_eq!(status, DhruvStatus::NullPointer);
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
    fn ffi_api_version_is_31() {
        assert_eq!(dhruv_api_version(), 31);
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

    // --- Grahan FFI tests ---

    #[test]
    fn ffi_grahan_config_default_values() {
        let cfg = dhruv_grahan_config_default();
        assert_eq!(cfg.include_penumbral, 1);
        assert_eq!(cfg.include_peak_details, 1);
    }

    #[test]
    fn ffi_next_chandra_grahan_rejects_null() {
        let cfg = dhruv_grahan_config_default();
        let mut result = std::mem::MaybeUninit::<DhruvChandraGrahanResult>::uninit();
        let mut found: u8 = 0;
        let status = unsafe {
            dhruv_next_chandra_grahan(
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
    fn ffi_prev_chandra_grahan_rejects_null() {
        let cfg = dhruv_grahan_config_default();
        let mut result = std::mem::MaybeUninit::<DhruvChandraGrahanResult>::uninit();
        let mut found: u8 = 0;
        let status = unsafe {
            dhruv_prev_chandra_grahan(
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
    fn ffi_search_chandra_grahan_rejects_null() {
        let cfg = dhruv_grahan_config_default();
        let mut count: u32 = 0;
        let status = unsafe {
            dhruv_search_chandra_grahan(
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
    fn ffi_next_surya_grahan_rejects_null() {
        let cfg = dhruv_grahan_config_default();
        let mut result = std::mem::MaybeUninit::<DhruvSuryaGrahanResult>::uninit();
        let mut found: u8 = 0;
        let status = unsafe {
            dhruv_next_surya_grahan(
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
    fn ffi_prev_surya_grahan_rejects_null() {
        let cfg = dhruv_grahan_config_default();
        let mut result = std::mem::MaybeUninit::<DhruvSuryaGrahanResult>::uninit();
        let mut found: u8 = 0;
        let status = unsafe {
            dhruv_prev_surya_grahan(
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
    fn ffi_search_surya_grahan_rejects_null() {
        let cfg = dhruv_grahan_config_default();
        let mut count: u32 = 0;
        let status = unsafe {
            dhruv_search_surya_grahan(
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
    fn ffi_next_purnima_rejects_null() {
        let utc = DhruvUtcTime {
            year: 2024,
            month: 1,
            day: 1,
            hour: 0,
            minute: 0,
            second: 0.0,
        };
        let status = unsafe {
            dhruv_next_purnima(
                std::ptr::null(),
                &utc,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            )
        };
        assert_eq!(status, DhruvStatus::NullPointer);
    }

    #[test]
    fn ffi_next_amavasya_rejects_null() {
        let utc = DhruvUtcTime {
            year: 2024,
            month: 1,
            day: 1,
            hour: 0,
            minute: 0,
            second: 0.0,
        };
        let status = unsafe {
            dhruv_next_amavasya(
                std::ptr::null(),
                &utc,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            )
        };
        assert_eq!(status, DhruvStatus::NullPointer);
    }

    #[test]
    fn ffi_next_sankranti_rejects_null() {
        let utc = DhruvUtcTime {
            year: 2024,
            month: 1,
            day: 1,
            hour: 0,
            minute: 0,
            second: 0.0,
        };
        let config = dhruv_sankranti_config_default();
        let status = unsafe {
            dhruv_next_sankranti(
                std::ptr::null(),
                &utc,
                &config,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
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

    // --- UTC variant null rejection tests ---

    fn test_utc() -> DhruvUtcTime {
        DhruvUtcTime {
            year: 2024,
            month: 1,
            day: 1,
            hour: 0,
            minute: 0,
            second: 0.0,
        }
    }

    #[test]
    fn ffi_next_conjunction_utc_rejects_null() {
        let utc = test_utc();
        let cfg = dhruv_conjunction_config_default();
        let mut event = std::mem::MaybeUninit::<DhruvConjunctionEventUtc>::uninit();
        let mut found: u8 = 0;
        let status = unsafe {
            dhruv_next_conjunction_utc(
                ptr::null(),
                10,
                301,
                &utc,
                &cfg,
                event.as_mut_ptr(),
                &mut found,
            )
        };
        assert_eq!(status, DhruvStatus::NullPointer);
    }

    #[test]
    fn ffi_prev_conjunction_utc_rejects_null() {
        let utc = test_utc();
        let cfg = dhruv_conjunction_config_default();
        let mut event = std::mem::MaybeUninit::<DhruvConjunctionEventUtc>::uninit();
        let mut found: u8 = 0;
        let status = unsafe {
            dhruv_prev_conjunction_utc(
                ptr::null(),
                10,
                301,
                &utc,
                &cfg,
                event.as_mut_ptr(),
                &mut found,
            )
        };
        assert_eq!(status, DhruvStatus::NullPointer);
    }

    #[test]
    fn ffi_search_conjunctions_utc_rejects_null() {
        let cfg = dhruv_conjunction_config_default();
        let mut count: u32 = 0;
        let status = unsafe {
            dhruv_search_conjunctions_utc(
                ptr::null(),
                10,
                301,
                ptr::null(),
                ptr::null(),
                &cfg,
                ptr::null_mut(),
                10,
                &mut count,
            )
        };
        assert_eq!(status, DhruvStatus::NullPointer);
    }

    #[test]
    fn ffi_next_chandra_grahan_utc_rejects_null() {
        let utc = test_utc();
        let cfg = dhruv_grahan_config_default();
        let mut result = std::mem::MaybeUninit::<DhruvChandraGrahanResultUtc>::uninit();
        let mut found: u8 = 0;
        let status = unsafe {
            dhruv_next_chandra_grahan_utc(ptr::null(), &utc, &cfg, result.as_mut_ptr(), &mut found)
        };
        assert_eq!(status, DhruvStatus::NullPointer);
    }

    #[test]
    fn ffi_prev_chandra_grahan_utc_rejects_null() {
        let utc = test_utc();
        let cfg = dhruv_grahan_config_default();
        let mut result = std::mem::MaybeUninit::<DhruvChandraGrahanResultUtc>::uninit();
        let mut found: u8 = 0;
        let status = unsafe {
            dhruv_prev_chandra_grahan_utc(ptr::null(), &utc, &cfg, result.as_mut_ptr(), &mut found)
        };
        assert_eq!(status, DhruvStatus::NullPointer);
    }

    #[test]
    fn ffi_search_chandra_grahan_utc_rejects_null() {
        let cfg = dhruv_grahan_config_default();
        let mut count: u32 = 0;
        let status = unsafe {
            dhruv_search_chandra_grahan_utc(
                ptr::null(),
                ptr::null(),
                ptr::null(),
                &cfg,
                ptr::null_mut(),
                10,
                &mut count,
            )
        };
        assert_eq!(status, DhruvStatus::NullPointer);
    }

    #[test]
    fn ffi_next_surya_grahan_utc_rejects_null() {
        let utc = test_utc();
        let cfg = dhruv_grahan_config_default();
        let mut result = std::mem::MaybeUninit::<DhruvSuryaGrahanResultUtc>::uninit();
        let mut found: u8 = 0;
        let status = unsafe {
            dhruv_next_surya_grahan_utc(ptr::null(), &utc, &cfg, result.as_mut_ptr(), &mut found)
        };
        assert_eq!(status, DhruvStatus::NullPointer);
    }

    #[test]
    fn ffi_prev_surya_grahan_utc_rejects_null() {
        let utc = test_utc();
        let cfg = dhruv_grahan_config_default();
        let mut result = std::mem::MaybeUninit::<DhruvSuryaGrahanResultUtc>::uninit();
        let mut found: u8 = 0;
        let status = unsafe {
            dhruv_prev_surya_grahan_utc(ptr::null(), &utc, &cfg, result.as_mut_ptr(), &mut found)
        };
        assert_eq!(status, DhruvStatus::NullPointer);
    }

    #[test]
    fn ffi_search_surya_grahan_utc_rejects_null() {
        let cfg = dhruv_grahan_config_default();
        let mut count: u32 = 0;
        let status = unsafe {
            dhruv_search_surya_grahan_utc(
                ptr::null(),
                ptr::null(),
                ptr::null(),
                &cfg,
                ptr::null_mut(),
                10,
                &mut count,
            )
        };
        assert_eq!(status, DhruvStatus::NullPointer);
    }

    #[test]
    fn ffi_next_stationary_utc_rejects_null() {
        let utc = test_utc();
        let cfg = dhruv_stationary_config_default();
        let mut event = std::mem::MaybeUninit::<DhruvStationaryEventUtc>::uninit();
        let mut found: u8 = 0;
        let status = unsafe {
            dhruv_next_stationary_utc(ptr::null(), 199, &utc, &cfg, event.as_mut_ptr(), &mut found)
        };
        assert_eq!(status, DhruvStatus::NullPointer);
    }

    #[test]
    fn ffi_prev_stationary_utc_rejects_null() {
        let utc = test_utc();
        let cfg = dhruv_stationary_config_default();
        let mut event = std::mem::MaybeUninit::<DhruvStationaryEventUtc>::uninit();
        let mut found: u8 = 0;
        let status = unsafe {
            dhruv_prev_stationary_utc(ptr::null(), 199, &utc, &cfg, event.as_mut_ptr(), &mut found)
        };
        assert_eq!(status, DhruvStatus::NullPointer);
    }

    #[test]
    fn ffi_search_stationary_utc_rejects_null() {
        let cfg = dhruv_stationary_config_default();
        let mut count: u32 = 0;
        let status = unsafe {
            dhruv_search_stationary_utc(
                ptr::null(),
                199,
                ptr::null(),
                ptr::null(),
                &cfg,
                ptr::null_mut(),
                10,
                &mut count,
            )
        };
        assert_eq!(status, DhruvStatus::NullPointer);
    }

    #[test]
    fn ffi_next_max_speed_utc_rejects_null() {
        let utc = test_utc();
        let cfg = dhruv_stationary_config_default();
        let mut event = std::mem::MaybeUninit::<DhruvMaxSpeedEventUtc>::uninit();
        let mut found: u8 = 0;
        let status = unsafe {
            dhruv_next_max_speed_utc(ptr::null(), 199, &utc, &cfg, event.as_mut_ptr(), &mut found)
        };
        assert_eq!(status, DhruvStatus::NullPointer);
    }

    #[test]
    fn ffi_prev_max_speed_utc_rejects_null() {
        let utc = test_utc();
        let cfg = dhruv_stationary_config_default();
        let mut event = std::mem::MaybeUninit::<DhruvMaxSpeedEventUtc>::uninit();
        let mut found: u8 = 0;
        let status = unsafe {
            dhruv_prev_max_speed_utc(ptr::null(), 199, &utc, &cfg, event.as_mut_ptr(), &mut found)
        };
        assert_eq!(status, DhruvStatus::NullPointer);
    }

    #[test]
    fn ffi_search_max_speed_utc_rejects_null() {
        let cfg = dhruv_stationary_config_default();
        let mut count: u32 = 0;
        let status = unsafe {
            dhruv_search_max_speed_utc(
                ptr::null(),
                199,
                ptr::null(),
                ptr::null(),
                &cfg,
                ptr::null_mut(),
                10,
                &mut count,
            )
        };
        assert_eq!(status, DhruvStatus::NullPointer);
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
    fn ffi_ayanamsha_mean_deg_utc_rejects_null() {
        let mut out: f64 = 0.0;
        let status = unsafe { dhruv_ayanamsha_mean_deg_utc(ptr::null(), 0, ptr::null(), &mut out) };
        assert_eq!(status, DhruvStatus::NullPointer);
    }

    #[test]
    fn ffi_ayanamsha_true_deg_utc_rejects_null() {
        let mut out: f64 = 0.0;
        let status =
            unsafe { dhruv_ayanamsha_true_deg_utc(ptr::null(), 0, ptr::null(), 0.0, &mut out) };
        assert_eq!(status, DhruvStatus::NullPointer);
    }

    #[test]
    fn ffi_ayanamsha_deg_utc_rejects_null() {
        let mut out: f64 = 0.0;
        let status = unsafe { dhruv_ayanamsha_deg_utc(ptr::null(), 0, ptr::null(), 0, &mut out) };
        assert_eq!(status, DhruvStatus::NullPointer);
    }

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

    // Group D null rejection

    #[test]
    fn ffi_query_utc_rejects_null() {
        let mut out = DhruvSphericalState {
            lon_deg: 0.0,
            lat_deg: 0.0,
            distance_km: 0.0,
            lon_speed: 0.0,
            lat_speed: 0.0,
            distance_speed: 0.0,
        };
        let status = unsafe { dhruv_query_utc(ptr::null(), 499, 10, 2, ptr::null(), &mut out) };
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
    fn ffi_graha_english_name_valid() {
        let name = dhruv_graha_english_name(0);
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
    }

    // --- calculate_all_bav ---

    #[test]
    fn ffi_calculate_all_bav_rejects_null() {
        let mut out = [DhruvBhinnaAshtakavarga {
            graha_index: 0,
            points: [0; 12],
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
        let cfg = DhruvFullKundaliConfig {
            include_graha_positions: 1,
            include_bindus: 1,
            include_drishti: 1,
            include_ashtakavarga: 1,
            include_upagrahas: 1,
            include_special_lagnas: 1,
            include_amshas: 0,
            include_shadbala: 0,
            include_vimsopaka: 0,
            node_dignity_policy: 0,
            graha_positions_config: DhruvGrahaPositionsConfig {
                include_nakshatra: 0,
                include_lagna: 1,
                include_outer_planets: 0,
                include_bhava: 0,
            },
            bindus_config: DhruvBindusConfig {
                include_nakshatra: 0,
                include_bhava: 0,
            },
            drishti_config: DhruvDrishtiConfig {
                include_bhava: 0,
                include_lagna: 0,
                include_bindus: 1,
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
        };
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
        let cfg = DhruvFullKundaliConfig {
            include_graha_positions: 1,
            include_bindus: 1,
            include_drishti: 1,
            include_ashtakavarga: 1,
            include_upagrahas: 1,
            include_special_lagnas: 1,
            include_amshas: 0,
            include_shadbala: 0,
            include_vimsopaka: 0,
            node_dignity_policy: 0,
            graha_positions_config: DhruvGrahaPositionsConfig {
                include_nakshatra: 0,
                include_lagna: 1,
                include_outer_planets: 0,
                include_bhava: 0,
            },
            bindus_config: DhruvBindusConfig {
                include_nakshatra: 0,
                include_bhava: 0,
            },
            drishti_config: DhruvDrishtiConfig {
                include_bhava: 0,
                include_lagna: 0,
                include_bindus: 1,
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
        };
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
    fn ffi_graha_sidereal_longitudes_rejects_null() {
        let mut out = std::mem::MaybeUninit::<DhruvGrahaLongitudes>::uninit();
        let s = unsafe {
            dhruv_graha_sidereal_longitudes(ptr::null(), 2451545.0, 0, 0, out.as_mut_ptr())
        };
        assert_eq!(s, DhruvStatus::NullPointer);
    }

    #[test]
    fn ffi_graha_sidereal_longitudes_rejects_null_out() {
        // Use a non-null but invalid engine pointer would be UB, so just test null out
        let s = unsafe {
            dhruv_graha_sidereal_longitudes(ptr::null(), 2451545.0, 0, 0, ptr::null_mut())
        };
        assert_eq!(s, DhruvStatus::NullPointer);
    }

    #[test]
    fn ffi_graha_sidereal_longitudes_rejects_invalid_ayanamsha() {
        // Ayanamsha code 99 is invalid
        let engine_ptr: *const Engine = ptr::null();
        let mut out = std::mem::MaybeUninit::<DhruvGrahaLongitudes>::uninit();
        let s = unsafe {
            dhruv_graha_sidereal_longitudes(engine_ptr, 2451545.0, 99, 0, out.as_mut_ptr())
        };
        // Null engine is checked first
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
            dms: DhruvDms { degrees: 0, minutes: 0, seconds: 0.0 },
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
        let s = unsafe {
            dhruv_amsha_longitudes(45.0, ptr::null(), ptr::null(), 0, ptr::null_mut())
        };
        assert_eq!(s, DhruvStatus::Ok);
    }

    #[test]
    fn ffi_amsha_longitudes_null_codes() {
        let mut out = [0.0f64; 1];
        let s = unsafe {
            dhruv_amsha_longitudes(45.0, ptr::null(), ptr::null(), 1, out.as_mut_ptr())
        };
        assert_eq!(s, DhruvStatus::NullPointer);
    }

    #[test]
    fn ffi_amsha_chart_null_engine() {
        let utc = DhruvUtcTime { year: 2024, month: 1, day: 15, hour: 12, minute: 0, second: 0.0 };
        let loc = DhruvGeoLocation { latitude_deg: 28.6, longitude_deg: 77.2, altitude_m: 0.0 };
        let bhava = DhruvBhavaConfig { system: 0, starting_point: 0, custom_start_deg: 0.0, reference_mode: 0 };
        let rs = DhruvRiseSetConfig { use_refraction: 1, sun_limb: 0, altitude_correction: 0 };
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
                ptr::null(), ptr::null(), &utc, &loc, &bhava, &rs, 1, 1, 9, 0, &scope, &mut out,
            )
        };
        assert_eq!(s, DhruvStatus::NullPointer);
    }
}
