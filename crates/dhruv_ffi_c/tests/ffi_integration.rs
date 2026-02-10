//! Integration tests for the C FFI layer (require kernel files).

use std::path::PathBuf;
use std::ptr;

use dhruv_core::{Body, Frame, Observer};
use dhruv_ffi_c::*;
use dhruv_time::calendar_to_jd;

fn kernel_base() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../kernels/data")
}

fn kernels_available() -> bool {
    let base = kernel_base();
    base.join("de442s.bsp").exists() && base.join("naif0012.tls").exists()
}

fn eop_path_cstr() -> Option<std::ffi::CString> {
    let base = kernel_base();
    let path = base.join("finals2000A.all");
    if !path.exists() {
        return None;
    }
    Some(std::ffi::CString::new(path.to_str().unwrap()).unwrap())
}

fn all_kernels_available() -> bool {
    let base = kernel_base();
    kernels_available() && base.join("finals2000A.all").exists()
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

// ---------------------------------------------------------------------------
// Ayanamsha integration tests
// ---------------------------------------------------------------------------

#[test]
fn ffi_ayanamsha_mean_lahiri_j2000() {
    let mut out: f64 = 0.0;
    // SAFETY: Valid output pointer.
    let status = unsafe { dhruv_ayanamsha_mean_deg(0, 2_451_545.0, &mut out) };
    assert_eq!(status, DhruvStatus::Ok);
    assert!(
        (out - 23.85).abs() < 0.01,
        "Lahiri at J2000 = {out}, expected ~23.85"
    );
}

#[test]
fn ffi_ayanamsha_all_systems_valid() {
    let count = dhruv_ayanamsha_system_count();
    for code in 0..count as i32 {
        let mut out: f64 = 0.0;
        // SAFETY: Valid output pointer.
        let status = unsafe { dhruv_ayanamsha_mean_deg(code, 2_451_545.0, &mut out) };
        assert_eq!(status, DhruvStatus::Ok, "system code {code} failed");
        assert!(
            (19.0..=28.0).contains(&out),
            "system {code}: ayanamsha {out} out of range"
        );
    }
}

// ---------------------------------------------------------------------------
// EOP integration tests
// ---------------------------------------------------------------------------

#[test]
fn ffi_eop_lifecycle() {
    let path = match eop_path_cstr() {
        Some(p) => p,
        None => {
            eprintln!("Skipping: finals2000A.all not found");
            return;
        }
    };

    let mut eop_ptr: *mut DhruvEopHandle = ptr::null_mut();
    // SAFETY: Valid pointers.
    let status = unsafe { dhruv_eop_load(path.as_ptr() as *const u8, &mut eop_ptr) };
    assert_eq!(status, DhruvStatus::Ok);
    assert!(!eop_ptr.is_null());

    // SAFETY: Pointer was returned by dhruv_eop_load.
    let status = unsafe { dhruv_eop_free(eop_ptr) };
    assert_eq!(status, DhruvStatus::Ok);
}

// ---------------------------------------------------------------------------
// Sunrise/sunset integration tests
// ---------------------------------------------------------------------------

#[test]
fn ffi_sunrise_new_delhi() {
    if !all_kernels_available() {
        eprintln!("Skipping: not all kernel files available");
        return;
    }

    let engine_ptr = make_engine().unwrap();
    let lsk_path = lsk_path_cstr().unwrap();
    let eop_path = eop_path_cstr().unwrap();

    let mut lsk_ptr: *mut DhruvLskHandle = ptr::null_mut();
    // SAFETY: Valid pointers.
    let status = unsafe { dhruv_lsk_load(lsk_path.as_ptr() as *const u8, &mut lsk_ptr) };
    assert_eq!(status, DhruvStatus::Ok);

    let mut eop_ptr: *mut DhruvEopHandle = ptr::null_mut();
    // SAFETY: Valid pointers.
    let status = unsafe { dhruv_eop_load(eop_path.as_ptr() as *const u8, &mut eop_ptr) };
    assert_eq!(status, DhruvStatus::Ok);

    // New Delhi, 2024-03-20
    let loc = DhruvGeoLocation {
        latitude_deg: 28.6139,
        longitude_deg: 77.209,
        altitude_m: 0.0,
    };

    let jd_0h = calendar_to_jd(2024, 3, 20.0);
    let noon = dhruv_approximate_local_noon_jd(jd_0h, loc.longitude_deg);
    let cfg = dhruv_riseset_config_default();

    let mut result = DhruvRiseSetResult {
        result_type: -1,
        event_code: -1,
        jd_tdb: 0.0,
    };

    // SAFETY: All pointers are valid.
    let status = unsafe {
        dhruv_compute_rise_set(
            engine_ptr, lsk_ptr, eop_ptr, &loc,
            DHRUV_EVENT_SUNRISE, noon, &cfg, &mut result,
        )
    };
    assert_eq!(status, DhruvStatus::Ok);
    assert_eq!(result.result_type, DHRUV_RISESET_EVENT);
    assert_eq!(result.event_code, DHRUV_EVENT_SUNRISE);

    // Sunrise in New Delhi on 2024-03-20 is ~00:48 UTC (06:18 IST)
    // Convert to UTC for validation
    let mut utc = DhruvUtcTime {
        year: 0, month: 0, day: 0, hour: 0, minute: 0, second: 0.0,
    };
    // SAFETY: All pointers are valid.
    let status = unsafe { dhruv_riseset_result_to_utc(lsk_ptr, &result, &mut utc) };
    assert_eq!(status, DhruvStatus::Ok);
    assert_eq!(utc.year, 2024);
    assert_eq!(utc.month, 3);
    // Hour should be ~0 (00:xx UTC)
    let total_min = utc.hour * 60 + utc.minute;
    assert!(
        total_min < 6 * 60, // before 06:00 UTC
        "Sunrise UTC = {:02}:{:02}, expected ~00:48",
        utc.hour, utc.minute
    );

    // SAFETY: Pointers were returned by their respective _new/_load functions.
    unsafe { dhruv_eop_free(eop_ptr) };
    unsafe { dhruv_lsk_free(lsk_ptr) };
    unsafe { dhruv_engine_free(engine_ptr) };
}

#[test]
fn ffi_polar_never_sets_tromso() {
    if !all_kernels_available() {
        eprintln!("Skipping: not all kernel files available");
        return;
    }

    let engine_ptr = make_engine().unwrap();
    let lsk_path = lsk_path_cstr().unwrap();
    let eop_path = eop_path_cstr().unwrap();

    let mut lsk_ptr: *mut DhruvLskHandle = ptr::null_mut();
    // SAFETY: Valid pointers.
    let status = unsafe { dhruv_lsk_load(lsk_path.as_ptr() as *const u8, &mut lsk_ptr) };
    assert_eq!(status, DhruvStatus::Ok);

    let mut eop_ptr: *mut DhruvEopHandle = ptr::null_mut();
    // SAFETY: Valid pointers.
    let status = unsafe { dhruv_eop_load(eop_path.as_ptr() as *const u8, &mut eop_ptr) };
    assert_eq!(status, DhruvStatus::Ok);

    // Tromso, Norway (69.65°N), summer solstice — midnight sun
    let loc = DhruvGeoLocation {
        latitude_deg: 69.6496,
        longitude_deg: 18.9560,
        altitude_m: 0.0,
    };

    let jd_0h = calendar_to_jd(2024, 6, 21.0);
    let noon = dhruv_approximate_local_noon_jd(jd_0h, loc.longitude_deg);
    let cfg = dhruv_riseset_config_default();

    let mut result = DhruvRiseSetResult {
        result_type: -1,
        event_code: -1,
        jd_tdb: 0.0,
    };

    // SAFETY: All pointers are valid.
    let status = unsafe {
        dhruv_compute_rise_set(
            engine_ptr, lsk_ptr, eop_ptr, &loc,
            DHRUV_EVENT_SUNRISE, noon, &cfg, &mut result,
        )
    };
    assert_eq!(status, DhruvStatus::Ok);
    assert_eq!(
        result.result_type, DHRUV_RISESET_NEVER_SETS,
        "Tromso summer solstice should be NeverSets, got type={}",
        result.result_type
    );

    // Verify that converting NeverSets to UTC returns InvalidQuery
    let mut utc = DhruvUtcTime {
        year: 0, month: 0, day: 0, hour: 0, minute: 0, second: 0.0,
    };
    // SAFETY: All pointers are valid.
    let status = unsafe { dhruv_riseset_result_to_utc(lsk_ptr, &result, &mut utc) };
    assert_eq!(status, DhruvStatus::InvalidQuery);

    // SAFETY: Pointers were returned by their respective _new/_load functions.
    unsafe { dhruv_eop_free(eop_ptr) };
    unsafe { dhruv_lsk_free(lsk_ptr) };
    unsafe { dhruv_engine_free(engine_ptr) };
}

#[test]
fn ffi_all_events_new_delhi() {
    if !all_kernels_available() {
        eprintln!("Skipping: not all kernel files available");
        return;
    }

    let engine_ptr = make_engine().unwrap();
    let lsk_path = lsk_path_cstr().unwrap();
    let eop_path = eop_path_cstr().unwrap();

    let mut lsk_ptr: *mut DhruvLskHandle = ptr::null_mut();
    // SAFETY: Valid pointers.
    let status = unsafe { dhruv_lsk_load(lsk_path.as_ptr() as *const u8, &mut lsk_ptr) };
    assert_eq!(status, DhruvStatus::Ok);

    let mut eop_ptr: *mut DhruvEopHandle = ptr::null_mut();
    // SAFETY: Valid pointers.
    let status = unsafe { dhruv_eop_load(eop_path.as_ptr() as *const u8, &mut eop_ptr) };
    assert_eq!(status, DhruvStatus::Ok);

    let loc = DhruvGeoLocation {
        latitude_deg: 28.6139,
        longitude_deg: 77.209,
        altitude_m: 0.0,
    };

    let jd_0h = calendar_to_jd(2024, 3, 20.0);
    let noon = dhruv_approximate_local_noon_jd(jd_0h, loc.longitude_deg);
    let cfg = dhruv_riseset_config_default();

    let mut results = [DhruvRiseSetResult {
        result_type: -1,
        event_code: -1,
        jd_tdb: 0.0,
    }; 8];

    // SAFETY: All pointers are valid, results array has 8 elements.
    let status = unsafe {
        dhruv_compute_all_events(
            engine_ptr, lsk_ptr, eop_ptr, &loc,
            noon, &cfg, results.as_mut_ptr(),
        )
    };
    assert_eq!(status, DhruvStatus::Ok);

    // At equinox near the equator, all 8 events should occur
    for (i, r) in results.iter().enumerate() {
        assert_eq!(
            r.result_type, DHRUV_RISESET_EVENT,
            "event {i}: expected Event, got type={}",
            r.result_type
        );
    }

    // Dawn events should be in ascending JD order (AstroDawn < NautDawn < CivilDawn < Sunrise)
    for i in 0..3 {
        assert!(
            results[i].jd_tdb < results[i + 1].jd_tdb,
            "dawn order: event {} (jd={}) should be < event {} (jd={})",
            i, results[i].jd_tdb, i + 1, results[i + 1].jd_tdb
        );
    }

    // SAFETY: Pointers were returned by their respective _new/_load functions.
    unsafe { dhruv_eop_free(eop_ptr) };
    unsafe { dhruv_lsk_free(lsk_ptr) };
    unsafe { dhruv_engine_free(engine_ptr) };
}

#[test]
fn ffi_jd_tdb_to_utc_j2000() {
    let path = match lsk_path_cstr() {
        Some(p) => p,
        None => return,
    };

    let mut lsk_ptr: *mut DhruvLskHandle = ptr::null_mut();
    // SAFETY: Valid pointers.
    let status = unsafe { dhruv_lsk_load(path.as_ptr() as *const u8, &mut lsk_ptr) };
    assert_eq!(status, DhruvStatus::Ok);

    let mut utc = DhruvUtcTime {
        year: 0, month: 0, day: 0, hour: 0, minute: 0, second: 0.0,
    };

    // J2000.0 TDB ≈ 2000-01-01 12:00:00 UTC (within ~1 minute)
    // SAFETY: All pointers are valid.
    let status = unsafe { dhruv_jd_tdb_to_utc(lsk_ptr, 2_451_545.0, &mut utc) };
    assert_eq!(status, DhruvStatus::Ok);
    assert_eq!(utc.year, 2000);
    assert_eq!(utc.month, 1);
    assert_eq!(utc.day, 1);
    assert_eq!(utc.hour, 11); // TDB-UTC offset ~64s, so hour rounds to 11
    assert!(utc.minute >= 58, "expected ~11:58-11:59, got {:02}:{:02}", utc.hour, utc.minute);

    // SAFETY: Pointer was returned by dhruv_lsk_load.
    unsafe { dhruv_lsk_free(lsk_ptr) };
}
