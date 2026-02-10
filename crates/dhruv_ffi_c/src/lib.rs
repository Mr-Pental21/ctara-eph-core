//! C-facing adapter types for `ctara-dhruv-core`.

use std::path::PathBuf;
use std::ptr;

use dhruv_core::{Body, Engine, EngineConfig, EngineError, Frame, Observer, Query, StateVector};
use dhruv_vedic_base::{
    AyanamshaSystem, GeoLocation, RiseSetConfig, RiseSetEvent, RiseSetResult, SunLimb,
    VedicError, ayanamsha_deg, ayanamsha_mean_deg, ayanamsha_true_deg,
    approximate_local_noon_jd, compute_all_events, compute_rise_set, jd_tdb_to_centuries,
};

/// ABI version for downstream bindings.
pub const DHRUV_API_VERSION: u32 = 4;

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
    /// Longitude in radians, range [0, 2*pi).
    pub lon_rad: f64,
    /// Latitude in radians, range [-pi/2, pi/2].
    pub lat_rad: f64,
    /// Distance from origin in km.
    pub distance_km: f64,
}

/// C-compatible spherical state with angular velocities.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DhruvSphericalState {
    /// Longitude in radians, range [0, 2*pi).
    pub lon_rad: f64,
    /// Latitude in radians, range [-pi/2, pi/2].
    pub lat_rad: f64,
    /// Distance from origin in km.
    pub distance_km: f64,
    /// Longitude rate of change in rad/s.
    pub lon_speed: f64,
    /// Latitude rate of change in rad/s.
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
/// Pure math, no engine needed. Writes longitude (radians, 0..2pi),
/// latitude (radians, -pi/2..pi/2), and distance (km) into `out_spherical`.
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
                lon_rad: s.lon_rad,
                lat_rad: s.lat_rad,
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
        lon_rad: ss.lon_rad,
        lat_rad: ss.lat_rad,
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
            lon_rad: 0.0,
            lat_rad: 0.0,
            distance_km: 0.0,
        };
        // SAFETY: Both pointers are valid stack references.
        let status = unsafe { dhruv_cartesian_to_spherical(&pos, &mut out) };
        assert_eq!(status, DhruvStatus::Ok);
        assert!((out.lon_rad - 0.0).abs() < 1e-10);
        assert!((out.lat_rad - 0.0).abs() < 1e-10);
        assert!((out.distance_km - 1.0e8).abs() < 1e-3);
    }

    #[test]
    fn ffi_cartesian_to_spherical_rejects_null() {
        let mut out = DhruvSphericalCoords {
            lon_rad: 0.0,
            lat_rad: 0.0,
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
            lon_rad: 0.0,
            lat_rad: 0.0,
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
}
