//! C-facing adapter types for `ctara-eph-core`.

use std::path::PathBuf;
use std::ptr;

use eph_core::{Body, Engine, EngineConfig, EngineError, Frame, Observer, Query, StateVector};

/// ABI version for downstream bindings.
pub const EPH_API_VERSION: u32 = 3;

/// Fixed UTF-8 buffer size for path fields in C-compatible structs.
pub const EPH_PATH_CAPACITY: usize = 512;

/// Maximum number of SPK kernel paths in a C-compatible config.
pub const EPH_MAX_SPK_PATHS: usize = 8;

/// C-facing status codes.
#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EphStatus {
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

impl From<&EngineError> for EphStatus {
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
pub struct EphEngineConfig {
    pub spk_path_count: u32,
    pub spk_paths_utf8: [[u8; EPH_PATH_CAPACITY]; EPH_MAX_SPK_PATHS],
    pub lsk_path_utf8: [u8; EPH_PATH_CAPACITY],
    pub cache_capacity: u64,
    pub strict_validation: u8,
}

impl EphEngineConfig {
    /// Convenience constructor for a single SPK path (most common case).
    pub fn try_new(
        spk_path_utf8: &str,
        lsk_path_utf8: &str,
        cache_capacity: u64,
        strict_validation: bool,
    ) -> Result<Self, EphStatus> {
        Self::try_new_multi(&[spk_path_utf8], lsk_path_utf8, cache_capacity, strict_validation)
    }

    /// Constructor for multiple SPK paths.
    pub fn try_new_multi(
        spk_paths: &[&str],
        lsk_path_utf8: &str,
        cache_capacity: u64,
        strict_validation: bool,
    ) -> Result<Self, EphStatus> {
        if spk_paths.is_empty() || spk_paths.len() > EPH_MAX_SPK_PATHS {
            return Err(EphStatus::InvalidConfig);
        }

        let mut spk_paths_utf8 = [[0_u8; EPH_PATH_CAPACITY]; EPH_MAX_SPK_PATHS];
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

impl TryFrom<&EphEngineConfig> for EngineConfig {
    type Error = EngineError;

    fn try_from(value: &EphEngineConfig) -> Result<Self, Self::Error> {
        let count = value.spk_path_count as usize;
        if count == 0 || count > EPH_MAX_SPK_PATHS {
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
pub struct EphQuery {
    pub target: i32,
    pub observer: i32,
    pub frame: i32,
    pub epoch_tdb_jd: f64,
}

impl TryFrom<EphQuery> for Query {
    type Error = EngineError;

    fn try_from(value: EphQuery) -> Result<Self, Self::Error> {
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
pub struct EphStateVector {
    pub position_km: [f64; 3],
    pub velocity_km_s: [f64; 3],
}

impl From<StateVector> for EphStateVector {
    fn from(value: StateVector) -> Self {
        Self {
            position_km: value.position_km,
            velocity_km_s: value.velocity_km_s,
        }
    }
}

/// Opaque engine handle type for ABI consumers.
pub type EphEngineHandle = Engine;

/// Opaque LSK handle type for ABI consumers.
pub type EphLskHandle = eph_time::LeapSecondKernel;

/// Build a core engine from C-compatible config.
pub fn eph_engine_new_internal(config: &EphEngineConfig) -> Result<Engine, EphStatus> {
    let core_config = EngineConfig::try_from(config).map_err(|err| EphStatus::from(&err))?;
    Engine::new(core_config).map_err(|err| EphStatus::from(&err))
}

/// Query the engine using C-compatible types.
pub fn eph_engine_query_internal(
    engine: &Engine,
    query: EphQuery,
) -> Result<EphStateVector, EphStatus> {
    let core_query = Query::try_from(query).map_err(|err| EphStatus::from(&err))?;
    let state = engine
        .query(core_query)
        .map_err(|err| EphStatus::from(&err))?;
    Ok(EphStateVector::from(state))
}

/// Convenience helper for one-shot callers.
pub fn eph_query_once_internal(
    config: &EphEngineConfig,
    query: EphQuery,
) -> Result<EphStateVector, EphStatus> {
    let engine = eph_engine_new_internal(config)?;
    eph_engine_query_internal(&engine, query)
}

/// Return ABI version of the exported C API.
#[unsafe(no_mangle)]
pub extern "C" fn eph_api_version() -> u32 {
    EPH_API_VERSION
}

/// Create an engine handle.
///
/// # Safety
/// `config` and `out_engine` must be valid, non-null pointers.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eph_engine_new(
    config: *const EphEngineConfig,
    out_engine: *mut *mut EphEngineHandle,
) -> EphStatus {
    ffi_boundary(|| {
        if config.is_null() || out_engine.is_null() {
            return EphStatus::NullPointer;
        }

        // SAFETY: Pointers are checked for null and are only borrowed for this call.
        let config_ref = unsafe { &*config };
        // SAFETY: Pointer is checked for null and we only write a single pointer value.
        let out_engine_ref = unsafe { &mut *out_engine };

        match eph_engine_new_internal(config_ref) {
            Ok(engine) => {
                *out_engine_ref = Box::into_raw(Box::new(engine));
                EphStatus::Ok
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
pub unsafe extern "C" fn eph_engine_query(
    engine: *const EphEngineHandle,
    query: *const EphQuery,
    out_state: *mut EphStateVector,
) -> EphStatus {
    ffi_boundary(|| {
        if engine.is_null() || query.is_null() || out_state.is_null() {
            return EphStatus::NullPointer;
        }

        // SAFETY: Pointers are checked for null and only borrowed for this call.
        let engine_ref = unsafe { &*engine };
        // SAFETY: Pointer is checked for null and copied by value.
        let query_value = unsafe { *query };

        match eph_engine_query_internal(engine_ref, query_value) {
            Ok(state) => {
                // SAFETY: Pointer is checked for null and written once.
                unsafe { *out_state = state };
                EphStatus::Ok
            }
            Err(status) => status,
        }
    })
}

/// Destroy an engine handle allocated by [`eph_engine_new`].
///
/// # Safety
/// `engine` must be either null or a pointer returned by `eph_engine_new`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eph_engine_free(engine: *mut EphEngineHandle) -> EphStatus {
    ffi_boundary(|| {
        if engine.is_null() {
            return EphStatus::Ok;
        }

        // SAFETY: Ownership is transferred back from a pointer created by Box::into_raw.
        unsafe { drop(Box::from_raw(engine)) };
        EphStatus::Ok
    })
}

/// One-shot query helper (constructs and tears down engine internally).
///
/// # Safety
/// `config`, `query`, and `out_state` must be valid, non-null pointers.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eph_query_once(
    config: *const EphEngineConfig,
    query: *const EphQuery,
    out_state: *mut EphStateVector,
) -> EphStatus {
    ffi_boundary(|| {
        if config.is_null() || query.is_null() || out_state.is_null() {
            return EphStatus::NullPointer;
        }

        // SAFETY: Pointer checks performed above; references are ephemeral.
        let config_ref = unsafe { &*config };
        // SAFETY: Pointer checks performed above; copied by value.
        let query_value = unsafe { *query };

        match eph_query_once_internal(config_ref, query_value) {
            Ok(state) => {
                // SAFETY: Pointer checks performed above; write one value.
                unsafe { *out_state = state };
                EphStatus::Ok
            }
            Err(status) => status,
        }
    })
}

/// C-compatible spherical coordinates output.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EphSphericalCoords {
    /// Longitude in radians, range [0, 2*pi).
    pub lon_rad: f64,
    /// Latitude in radians, range [-pi/2, pi/2].
    pub lat_rad: f64,
    /// Distance from origin in km.
    pub distance_km: f64,
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
pub unsafe extern "C" fn eph_lsk_load(
    lsk_path_utf8: *const u8,
    out_lsk: *mut *mut EphLskHandle,
) -> EphStatus {
    ffi_boundary(|| {
        if lsk_path_utf8.is_null() || out_lsk.is_null() {
            return EphStatus::NullPointer;
        }

        // SAFETY: Pointer is checked for null; read until NUL byte.
        let c_str = unsafe { std::ffi::CStr::from_ptr(lsk_path_utf8 as *const i8) };
        let path_str = match c_str.to_str() {
            Ok(s) => s,
            Err(_) => return EphStatus::InvalidConfig,
        };

        match eph_time::LeapSecondKernel::load(std::path::Path::new(path_str)) {
            Ok(lsk) => {
                // SAFETY: Pointer is checked for null; write one pointer value.
                unsafe { *out_lsk = Box::into_raw(Box::new(lsk)) };
                EphStatus::Ok
            }
            Err(_) => {
                // SAFETY: Pointer is checked for null; write null on failure.
                unsafe { *out_lsk = ptr::null_mut() };
                EphStatus::KernelLoad
            }
        }
    })
}

/// Destroy an LSK handle allocated by [`eph_lsk_load`].
///
/// # Safety
/// `lsk` must be either null or a pointer returned by `eph_lsk_load`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eph_lsk_free(lsk: *mut EphLskHandle) -> EphStatus {
    ffi_boundary(|| {
        if lsk.is_null() {
            return EphStatus::Ok;
        }

        // SAFETY: Ownership is transferred back from a pointer created by Box::into_raw.
        unsafe { drop(Box::from_raw(lsk)) };
        EphStatus::Ok
    })
}

/// Convert UTC calendar date to TDB Julian Date using a standalone LSK handle.
///
/// Writes the resulting JD TDB into `out_jd_tdb`.
///
/// # Safety
/// `lsk` and `out_jd_tdb` must be valid, non-null pointers.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eph_utc_to_tdb_jd(
    lsk: *const EphLskHandle,
    year: i32,
    month: u32,
    day: u32,
    hour: u32,
    min: u32,
    sec: f64,
    out_jd_tdb: *mut f64,
) -> EphStatus {
    ffi_boundary(|| {
        if lsk.is_null() || out_jd_tdb.is_null() {
            return EphStatus::NullPointer;
        }

        // SAFETY: Pointer is checked for null above.
        let lsk_ref = unsafe { &*lsk };

        let epoch = eph_time::Epoch::from_utc(year, month, day, hour, min, sec, lsk_ref);

        // SAFETY: Pointer is checked for null above; write one value.
        unsafe { *out_jd_tdb = epoch.as_jd_tdb() };
        EphStatus::Ok
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
pub unsafe extern "C" fn eph_cartesian_to_spherical(
    position_km: *const [f64; 3],
    out_spherical: *mut EphSphericalCoords,
) -> EphStatus {
    ffi_boundary(|| {
        if position_km.is_null() || out_spherical.is_null() {
            return EphStatus::NullPointer;
        }

        // SAFETY: Pointer is checked for null; read 3 contiguous f64.
        let xyz = unsafe { &*position_km };
        let s = eph_frames::cartesian_to_spherical(xyz);

        // SAFETY: Pointer is checked for null; write one struct.
        unsafe {
            *out_spherical = EphSphericalCoords {
                lon_rad: s.lon_rad,
                lat_rad: s.lat_rad,
                distance_km: s.distance_km,
            };
        }
        EphStatus::Ok
    })
}

fn ffi_boundary(f: impl FnOnce() -> EphStatus) -> EphStatus {
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)) {
        Ok(status) => status,
        Err(_) => EphStatus::Internal,
    }
}

fn encode_c_utf8(input: &str) -> Result<[u8; EPH_PATH_CAPACITY], EphStatus> {
    if input.is_empty() {
        return Err(EphStatus::InvalidConfig);
    }

    let bytes = input.as_bytes();
    if bytes.len() >= EPH_PATH_CAPACITY {
        return Err(EphStatus::InvalidConfig);
    }
    if bytes.contains(&0) {
        return Err(EphStatus::InvalidConfig);
    }

    let mut out = [0_u8; EPH_PATH_CAPACITY];
    out[..bytes.len()].copy_from_slice(bytes);
    Ok(out)
}

fn decode_c_utf8(buffer: &[u8; EPH_PATH_CAPACITY]) -> Result<&str, std::str::Utf8Error> {
    let end = buffer
        .iter()
        .position(|b| *b == 0)
        .unwrap_or(EPH_PATH_CAPACITY);
    std::str::from_utf8(&buffer[..end])
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn kernel_base() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../kernels/data")
    }

    fn kernels_available() -> bool {
        let base = kernel_base();
        base.join("de442s.bsp").exists() && base.join("naif0012.tls").exists()
    }

    fn real_config() -> Option<EphEngineConfig> {
        if !kernels_available() {
            eprintln!("Skipping: kernel files not found");
            return None;
        }
        let base = kernel_base();
        Some(
            EphEngineConfig::try_new(
                base.join("de442s.bsp").to_str().unwrap(),
                base.join("naif0012.tls").to_str().unwrap(),
                256,
                true,
            )
            .expect("config should be valid"),
        )
    }

    #[test]
    fn status_maps_from_core_error() {
        let status = EphStatus::from(&EngineError::InvalidQuery("bad"));
        assert_eq!(status, EphStatus::InvalidQuery);
    }

    #[test]
    fn query_once_successfully_maps_through_core_contract() {
        let config = match real_config() {
            Some(c) => c,
            None => return,
        };
        let query = EphQuery {
            target: Body::Mars.code(),
            observer: Observer::Body(Body::Earth).code(),
            frame: Frame::IcrfJ2000.code(),
            epoch_tdb_jd: 2_460_000.25,
        };

        let result = eph_query_once_internal(&config, query).expect("query should succeed");
        assert!(result.position_km[0].is_finite());
    }

    #[test]
    fn query_rejects_invalid_body_code() {
        let config = match real_config() {
            Some(c) => c,
            None => return,
        };
        let query = EphQuery {
            target: -999,
            observer: Observer::SolarSystemBarycenter.code(),
            frame: Frame::IcrfJ2000.code(),
            epoch_tdb_jd: 2_460_000.25,
        };

        let result = eph_query_once_internal(&config, query);
        assert_eq!(result, Err(EphStatus::InvalidQuery));
    }

    #[test]
    fn ffi_lifecycle_create_query_free() {
        let config = match real_config() {
            Some(c) => c,
            None => return,
        };
        let query = EphQuery {
            target: Body::Mars.code(),
            observer: Observer::Body(Body::Earth).code(),
            frame: Frame::IcrfJ2000.code(),
            epoch_tdb_jd: 2_460_000.5,
        };

        let mut engine_ptr: *mut EphEngineHandle = ptr::null_mut();
        // SAFETY: Passing valid pointers created in this test scope.
        let create_status = unsafe { eph_engine_new(&config, &mut engine_ptr) };
        assert_eq!(create_status, EphStatus::Ok);
        assert!(!engine_ptr.is_null());

        let mut out_state = EphStateVector {
            position_km: [0.0; 3],
            velocity_km_s: [0.0; 3],
        };
        // SAFETY: Engine handle and output buffers are valid in this test.
        let query_status = unsafe { eph_engine_query(engine_ptr, &query, &mut out_state) };
        assert_eq!(query_status, EphStatus::Ok);
        assert!(out_state.position_km[0].is_finite());

        // SAFETY: Pointer was returned by eph_engine_new and not yet freed.
        let free_status = unsafe { eph_engine_free(engine_ptr) };
        assert_eq!(free_status, EphStatus::Ok);
    }

    #[test]
    fn ffi_new_rejects_null_output_pointer() {
        let config = match real_config() {
            Some(c) => c,
            None => return,
        };
        // SAFETY: Passing null out pointer intentionally to verify validation.
        let status = unsafe { eph_engine_new(&config, ptr::null_mut()) };
        assert_eq!(status, EphStatus::NullPointer);
    }

    #[test]
    fn ffi_query_rejects_null_input_pointer() {
        let mut out_state = EphStateVector {
            position_km: [0.0; 3],
            velocity_km_s: [0.0; 3],
        };
        // SAFETY: Null engine pointer is intentional for this validation test.
        let status = unsafe { eph_engine_query(ptr::null(), ptr::null(), &mut out_state) };
        assert_eq!(status, EphStatus::NullPointer);
    }

    fn lsk_path_cstr() -> Option<std::ffi::CString> {
        let base = kernel_base();
        let path = base.join("naif0012.tls");
        if !path.exists() {
            return None;
        }
        Some(std::ffi::CString::new(path.to_str().unwrap()).unwrap())
    }

    #[test]
    fn ffi_lsk_lifecycle() {
        let path = match lsk_path_cstr() {
            Some(p) => p,
            None => return,
        };

        let mut lsk_ptr: *mut EphLskHandle = ptr::null_mut();
        // SAFETY: Valid pointers created in this test scope.
        let status = unsafe { eph_lsk_load(path.as_ptr() as *const u8, &mut lsk_ptr) };
        assert_eq!(status, EphStatus::Ok);
        assert!(!lsk_ptr.is_null());

        // SAFETY: Pointer was returned by eph_lsk_load.
        let status = unsafe { eph_lsk_free(lsk_ptr) };
        assert_eq!(status, EphStatus::Ok);
    }

    #[test]
    fn ffi_lsk_load_rejects_null() {
        let mut lsk_ptr: *mut EphLskHandle = ptr::null_mut();
        // SAFETY: Null path pointer is intentional for validation.
        let status = unsafe { eph_lsk_load(ptr::null(), &mut lsk_ptr) };
        assert_eq!(status, EphStatus::NullPointer);
    }

    #[test]
    fn ffi_utc_to_tdb_jd_roundtrip() {
        let path = match lsk_path_cstr() {
            Some(p) => p,
            None => return,
        };

        let mut lsk_ptr: *mut EphLskHandle = ptr::null_mut();
        // SAFETY: Valid pointers created in this test scope.
        let status = unsafe { eph_lsk_load(path.as_ptr() as *const u8, &mut lsk_ptr) };
        assert_eq!(status, EphStatus::Ok);

        let mut jd_tdb: f64 = 0.0;
        // J2000.0 = 2000-01-01 12:00:00 UTC (approximately)
        // SAFETY: LSK handle and output are valid in this test.
        let status = unsafe {
            eph_utc_to_tdb_jd(lsk_ptr, 2000, 1, 1, 12, 0, 0.0, &mut jd_tdb)
        };
        assert_eq!(status, EphStatus::Ok);

        // Should be very close to J2000.0 (2451545.0), within ~1 minute of TDB-UTC offset.
        assert!(
            (jd_tdb - 2_451_545.0).abs() < 0.001,
            "expected ~2451545.0, got {jd_tdb}"
        );

        // SAFETY: Pointer was returned by eph_lsk_load.
        unsafe { eph_lsk_free(lsk_ptr) };
    }

    #[test]
    fn ffi_utc_to_tdb_jd_rejects_null() {
        let mut jd: f64 = 0.0;
        // SAFETY: Null LSK pointer is intentional for validation.
        let status = unsafe { eph_utc_to_tdb_jd(ptr::null(), 2000, 1, 1, 12, 0, 0.0, &mut jd) };
        assert_eq!(status, EphStatus::NullPointer);
    }

    #[test]
    fn ffi_cartesian_to_spherical_along_x() {
        let pos = [1.0e8_f64, 0.0, 0.0];
        let mut out = EphSphericalCoords {
            lon_rad: 0.0,
            lat_rad: 0.0,
            distance_km: 0.0,
        };
        // SAFETY: Both pointers are valid stack references.
        let status = unsafe { eph_cartesian_to_spherical(&pos, &mut out) };
        assert_eq!(status, EphStatus::Ok);
        assert!((out.lon_rad - 0.0).abs() < 1e-10);
        assert!((out.lat_rad - 0.0).abs() < 1e-10);
        assert!((out.distance_km - 1.0e8).abs() < 1e-3);
    }

    #[test]
    fn ffi_cartesian_to_spherical_rejects_null() {
        let mut out = EphSphericalCoords {
            lon_rad: 0.0,
            lat_rad: 0.0,
            distance_km: 0.0,
        };
        // SAFETY: Null position pointer is intentional for validation.
        let status = unsafe { eph_cartesian_to_spherical(ptr::null(), &mut out) };
        assert_eq!(status, EphStatus::NullPointer);
    }

    #[test]
    fn ffi_full_longitude_workflow() {
        // End-to-end: load LSK -> UTC to TDB JD -> query ecliptic -> spherical -> longitude
        let config = match real_config() {
            Some(c) => c,
            None => return,
        };
        let lsk_path = match lsk_path_cstr() {
            Some(p) => p,
            None => return,
        };

        // Load LSK independently
        let mut lsk_ptr: *mut EphLskHandle = ptr::null_mut();
        // SAFETY: Valid pointers.
        let status = unsafe { eph_lsk_load(lsk_path.as_ptr() as *const u8, &mut lsk_ptr) };
        assert_eq!(status, EphStatus::Ok);

        // Load engine for queries
        let mut engine_ptr: *mut EphEngineHandle = ptr::null_mut();
        // SAFETY: Valid pointers.
        let status = unsafe { eph_engine_new(&config, &mut engine_ptr) };
        assert_eq!(status, EphStatus::Ok);

        // Step 1: UTC to TDB JD (uses LSK, not engine)
        let mut jd_tdb: f64 = 0.0;
        // SAFETY: LSK handle and output are valid.
        let status = unsafe {
            eph_utc_to_tdb_jd(lsk_ptr, 2024, 3, 20, 12, 0, 0.0, &mut jd_tdb)
        };
        assert_eq!(status, EphStatus::Ok);

        // Step 2: Query Mars heliocentric ecliptic
        let query = EphQuery {
            target: Body::Mars.code(),
            observer: Body::Sun.code(),
            frame: Frame::EclipticJ2000.code(),
            epoch_tdb_jd: jd_tdb,
        };
        let mut state = EphStateVector {
            position_km: [0.0; 3],
            velocity_km_s: [0.0; 3],
        };
        // SAFETY: All pointers are valid.
        let status = unsafe { eph_engine_query(engine_ptr, &query, &mut state) };
        assert_eq!(status, EphStatus::Ok);

        // Step 3: Cartesian to spherical
        let mut spherical = EphSphericalCoords {
            lon_rad: 0.0,
            lat_rad: 0.0,
            distance_km: 0.0,
        };
        // SAFETY: Both pointers are valid.
        let status = unsafe {
            eph_cartesian_to_spherical(&state.position_km, &mut spherical)
        };
        assert_eq!(status, EphStatus::Ok);

        let lon_deg = spherical.lon_rad.to_degrees();
        assert!(lon_deg >= 0.0 && lon_deg < 360.0, "longitude {lon_deg} out of range");
        assert!(spherical.distance_km > 1.0e8, "Mars should be >1 AU from Sun");

        // SAFETY: Pointers were returned by their respective _new/_load functions.
        unsafe { eph_engine_free(engine_ptr) };
        unsafe { eph_lsk_free(lsk_ptr) };
    }
}
