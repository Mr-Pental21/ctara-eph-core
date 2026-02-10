//! Integration tests for the C FFI layer (require kernel files).

use std::path::PathBuf;
use std::ptr;

use dhruv_core::{Body, Frame, Observer};
use dhruv_ffi_c::*;

fn kernel_base() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../kernels/data")
}

fn kernels_available() -> bool {
    let base = kernel_base();
    base.join("de442s.bsp").exists() && base.join("naif0012.tls").exists()
}

fn real_config() -> Option<DhruvEngineConfig> {
    if !kernels_available() {
        eprintln!("Skipping: kernel files not found");
        return None;
    }
    let base = kernel_base();
    Some(
        DhruvEngineConfig::try_new(
            base.join("de442s.bsp").to_str().unwrap(),
            base.join("naif0012.tls").to_str().unwrap(),
            256,
            true,
        )
        .expect("config should be valid"),
    )
}

fn lsk_path_cstr() -> Option<std::ffi::CString> {
    let base = kernel_base();
    let path = base.join("naif0012.tls");
    if !path.exists() {
        return None;
    }
    Some(std::ffi::CString::new(path.to_str().unwrap()).unwrap())
}

fn make_engine() -> Option<*mut DhruvEngineHandle> {
    let config = real_config()?;
    let mut engine_ptr: *mut DhruvEngineHandle = ptr::null_mut();
    // SAFETY: Valid pointers created in test scope.
    let status = unsafe { dhruv_engine_new(&config, &mut engine_ptr) };
    assert_eq!(status, DhruvStatus::Ok);
    Some(engine_ptr)
}

#[test]
fn query_once_successfully_maps_through_core_contract() {
    let config = match real_config() {
        Some(c) => c,
        None => return,
    };
    let query = DhruvQuery {
        target: Body::Mars.code(),
        observer: Observer::Body(Body::Earth).code(),
        frame: Frame::IcrfJ2000.code(),
        epoch_tdb_jd: 2_460_000.25,
    };

    let result = dhruv_query_once_internal(&config, query).expect("query should succeed");
    assert!(result.position_km[0].is_finite());
}

#[test]
fn query_rejects_invalid_body_code() {
    let config = match real_config() {
        Some(c) => c,
        None => return,
    };
    let query = DhruvQuery {
        target: -999,
        observer: Observer::SolarSystemBarycenter.code(),
        frame: Frame::IcrfJ2000.code(),
        epoch_tdb_jd: 2_460_000.25,
    };

    let result = dhruv_query_once_internal(&config, query);
    assert_eq!(result, Err(DhruvStatus::InvalidQuery));
}

#[test]
fn ffi_lifecycle_create_query_free() {
    let config = match real_config() {
        Some(c) => c,
        None => return,
    };
    let query = DhruvQuery {
        target: Body::Mars.code(),
        observer: Observer::Body(Body::Earth).code(),
        frame: Frame::IcrfJ2000.code(),
        epoch_tdb_jd: 2_460_000.5,
    };

    let mut engine_ptr: *mut DhruvEngineHandle = ptr::null_mut();
    // SAFETY: Passing valid pointers created in this test scope.
    let create_status = unsafe { dhruv_engine_new(&config, &mut engine_ptr) };
    assert_eq!(create_status, DhruvStatus::Ok);
    assert!(!engine_ptr.is_null());

    let mut out_state = DhruvStateVector {
        position_km: [0.0; 3],
        velocity_km_s: [0.0; 3],
    };
    // SAFETY: Engine handle and output buffers are valid in this test.
    let query_status = unsafe { dhruv_engine_query(engine_ptr, &query, &mut out_state) };
    assert_eq!(query_status, DhruvStatus::Ok);
    assert!(out_state.position_km[0].is_finite());

    // SAFETY: Pointer was returned by dhruv_engine_new and not yet freed.
    let free_status = unsafe { dhruv_engine_free(engine_ptr) };
    assert_eq!(free_status, DhruvStatus::Ok);
}

#[test]
fn ffi_new_rejects_null_output_pointer() {
    let config = match real_config() {
        Some(c) => c,
        None => return,
    };
    // SAFETY: Passing null out pointer intentionally to verify validation.
    let status = unsafe { dhruv_engine_new(&config, ptr::null_mut()) };
    assert_eq!(status, DhruvStatus::NullPointer);
}

#[test]
fn ffi_lsk_lifecycle() {
    let path = match lsk_path_cstr() {
        Some(p) => p,
        None => return,
    };

    let mut lsk_ptr: *mut DhruvLskHandle = ptr::null_mut();
    // SAFETY: Valid pointers created in this test scope.
    let status = unsafe { dhruv_lsk_load(path.as_ptr() as *const u8, &mut lsk_ptr) };
    assert_eq!(status, DhruvStatus::Ok);
    assert!(!lsk_ptr.is_null());

    // SAFETY: Pointer was returned by dhruv_lsk_load.
    let status = unsafe { dhruv_lsk_free(lsk_ptr) };
    assert_eq!(status, DhruvStatus::Ok);
}

#[test]
fn ffi_utc_to_tdb_jd_roundtrip() {
    let path = match lsk_path_cstr() {
        Some(p) => p,
        None => return,
    };

    let mut lsk_ptr: *mut DhruvLskHandle = ptr::null_mut();
    // SAFETY: Valid pointers created in this test scope.
    let status = unsafe { dhruv_lsk_load(path.as_ptr() as *const u8, &mut lsk_ptr) };
    assert_eq!(status, DhruvStatus::Ok);

    let mut jd_tdb: f64 = 0.0;
    // J2000.0 = 2000-01-01 12:00:00 UTC (approximately)
    // SAFETY: LSK handle and output are valid in this test.
    let status = unsafe {
        dhruv_utc_to_tdb_jd(lsk_ptr, 2000, 1, 1, 12, 0, 0.0, &mut jd_tdb)
    };
    assert_eq!(status, DhruvStatus::Ok);

    // Should be very close to J2000.0 (2451545.0), within ~1 minute of TDB-UTC offset.
    assert!(
        (jd_tdb - 2_451_545.0).abs() < 0.001,
        "expected ~2451545.0, got {jd_tdb}"
    );

    // SAFETY: Pointer was returned by dhruv_lsk_load.
    unsafe { dhruv_lsk_free(lsk_ptr) };
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
    let mut lsk_ptr: *mut DhruvLskHandle = ptr::null_mut();
    // SAFETY: Valid pointers.
    let status = unsafe { dhruv_lsk_load(lsk_path.as_ptr() as *const u8, &mut lsk_ptr) };
    assert_eq!(status, DhruvStatus::Ok);

    // Load engine for queries
    let mut engine_ptr: *mut DhruvEngineHandle = ptr::null_mut();
    // SAFETY: Valid pointers.
    let status = unsafe { dhruv_engine_new(&config, &mut engine_ptr) };
    assert_eq!(status, DhruvStatus::Ok);

    // Step 1: UTC to TDB JD (uses LSK, not engine)
    let mut jd_tdb: f64 = 0.0;
    // SAFETY: LSK handle and output are valid.
    let status = unsafe {
        dhruv_utc_to_tdb_jd(lsk_ptr, 2024, 3, 20, 12, 0, 0.0, &mut jd_tdb)
    };
    assert_eq!(status, DhruvStatus::Ok);

    // Step 2: Query Mars heliocentric ecliptic
    let query = DhruvQuery {
        target: Body::Mars.code(),
        observer: Body::Sun.code(),
        frame: Frame::EclipticJ2000.code(),
        epoch_tdb_jd: jd_tdb,
    };
    let mut state = DhruvStateVector {
        position_km: [0.0; 3],
        velocity_km_s: [0.0; 3],
    };
    // SAFETY: All pointers are valid.
    let status = unsafe { dhruv_engine_query(engine_ptr, &query, &mut state) };
    assert_eq!(status, DhruvStatus::Ok);

    // Step 3: Cartesian to spherical
    let mut spherical = DhruvSphericalCoords {
        lon_rad: 0.0,
        lat_rad: 0.0,
        distance_km: 0.0,
    };
    // SAFETY: Both pointers are valid.
    let status = unsafe {
        dhruv_cartesian_to_spherical(&state.position_km, &mut spherical)
    };
    assert_eq!(status, DhruvStatus::Ok);

    let lon_deg = spherical.lon_rad.to_degrees();
    assert!(lon_deg >= 0.0 && lon_deg < 360.0, "longitude {lon_deg} out of range");
    assert!(spherical.distance_km > 1.0e8, "Mars should be >1 AU from Sun");

    // SAFETY: Pointers were returned by their respective _new/_load functions.
    unsafe { dhruv_engine_free(engine_ptr) };
    unsafe { dhruv_lsk_free(lsk_ptr) };
}

#[test]
fn ffi_query_utc_spherical_mars_heliocentric() {
    let engine_ptr = match make_engine() {
        Some(e) => e,
        None => return,
    };

    let mut out = DhruvSphericalState {
        lon_rad: 0.0,
        lat_rad: 0.0,
        distance_km: 0.0,
        lon_speed: 0.0,
        lat_speed: 0.0,
        distance_speed: 0.0,
    };

    // Mars heliocentric ecliptic at 2024-03-20 12:00:00 UTC
    // SAFETY: Engine handle and output are valid in this test.
    let status = unsafe {
        dhruv_query_utc_spherical(
            engine_ptr,
            Body::Mars.code(),
            Body::Sun.code(),
            Frame::EclipticJ2000.code(),
            2024, 3, 20, 12, 0, 0.0,
            &mut out,
        )
    };
    assert_eq!(status, DhruvStatus::Ok);

    let lon_deg = out.lon_rad.to_degrees();
    assert!(
        lon_deg >= 0.0 && lon_deg < 360.0,
        "longitude {lon_deg} out of range"
    );
    assert!(out.distance_km > 1.0e8, "Mars should be >1 AU from Sun");

    // SAFETY: Pointer was returned by dhruv_engine_new.
    unsafe { dhruv_engine_free(engine_ptr) };
}

#[test]
fn ffi_query_utc_spherical_speeds_finite() {
    let engine_ptr = match make_engine() {
        Some(e) => e,
        None => return,
    };

    let mut out = DhruvSphericalState {
        lon_rad: 0.0,
        lat_rad: 0.0,
        distance_km: 0.0,
        lon_speed: 0.0,
        lat_speed: 0.0,
        distance_speed: 0.0,
    };

    // Earth ecliptic heliocentric — a moving body with known non-zero velocity
    // SAFETY: Engine handle and output are valid in this test.
    let status = unsafe {
        dhruv_query_utc_spherical(
            engine_ptr,
            Body::Earth.code(),
            Body::Sun.code(),
            Frame::EclipticJ2000.code(),
            2024, 6, 15, 0, 0, 0.0,
            &mut out,
        )
    };
    assert_eq!(status, DhruvStatus::Ok);

    assert!(out.lon_speed.is_finite(), "lon_speed not finite");
    assert!(out.lat_speed.is_finite(), "lat_speed not finite");
    assert!(out.distance_speed.is_finite(), "distance_speed not finite");
    assert!(out.lon_speed != 0.0, "lon_speed should be non-zero for orbiting body");

    // SAFETY: Pointer was returned by dhruv_engine_new.
    unsafe { dhruv_engine_free(engine_ptr) };
}
