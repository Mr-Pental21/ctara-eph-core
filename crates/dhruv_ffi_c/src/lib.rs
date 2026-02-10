//! C-facing adapter types for `ctara-dhruv-core`.

use std::path::PathBuf;
use std::ptr;

use dhruv_core::{Body, Engine, EngineConfig, EngineError, Frame, Observer, Query, StateVector};

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
