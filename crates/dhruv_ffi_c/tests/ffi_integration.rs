//! Integration tests for the C FFI layer (require kernel files).

use std::path::PathBuf;
use std::ptr;

use std::f64::consts::{PI, TAU};

use dhruv_core::{Body, Frame, Observer};
use dhruv_ffi_c::*;
use dhruv_time::{calendar_to_jd, gmst_rad, local_sidereal_time_rad};

const ZEROED_UTC: DhruvUtcTime = DhruvUtcTime {
    year: 0,
    month: 0,
    day: 0,
    hour: 0,
    minute: 0,
    second: 0.0,
};

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

fn default_utc_to_tdb_request(utc: DhruvUtcTime) -> DhruvUtcToTdbRequest {
    DhruvUtcToTdbRequest {
        utc,
        policy: DhruvTimePolicy {
            mode: DHRUV_TIME_POLICY_HYBRID_DELTA_T,
            options: DhruvTimeConversionOptions {
                warn_on_fallback: 1,
                delta_t_model: DHRUV_DELTA_T_MODEL_SMH2016_WITH_PRE720_QUADRATIC,
                freeze_future_dut1: 1,
                pre_range_dut1: 0.0,
                future_delta_t_transition: DHRUV_FUTURE_DELTA_T_TRANSITION_LEGACY_TT_UTC_BLEND,
                future_transition_years: 100.0,
                smh_future_family: DHRUV_SMH_FUTURE_FAMILY_ADDENDUM_2020_PIECEWISE,
            },
        },
    }
}

fn utc_to_tdb_jd_default(lsk_ptr: *mut DhruvLskHandle, utc: DhruvUtcTime) -> f64 {
    let request = default_utc_to_tdb_request(utc);
    let mut out = DhruvUtcToTdbResult {
        jd_tdb: 0.0,
        diagnostics: DhruvTimeDiagnostics {
            source: 0,
            tt_minus_utc_s: 0.0,
            warning_count: 0,
            warnings: [DhruvTimeWarning {
                kind: 0,
                utc_seconds: 0.0,
                first_entry_utc_seconds: 0.0,
                last_entry_utc_seconds: 0.0,
                used_delta_at_seconds: 0.0,
                mjd: 0.0,
                first_entry_mjd: 0.0,
                last_entry_mjd: 0.0,
                used_dut1_seconds: 0.0,
                delta_t_model: 0,
                delta_t_segment: 0,
            }; DHRUV_MAX_TIME_WARNINGS],
        },
    };
    let status = unsafe { dhruv_utc_to_tdb_jd(lsk_ptr, ptr::null(), &request, &mut out) };
    assert_eq!(status, DhruvStatus::Ok);
    out.jd_tdb
}

fn make_engine() -> Option<*mut DhruvEngineHandle> {
    let config = real_config()?;
    let mut engine_ptr: *mut DhruvEngineHandle = ptr::null_mut();
    // SAFETY: Valid pointers created in test scope.
    let status = unsafe { dhruv_engine_new(&config, &mut engine_ptr) };
    assert_eq!(status, DhruvStatus::Ok);
    Some(engine_ptr)
}

fn angular_separation_deg(a: f64, b: f64) -> f64 {
    ((a - b + 180.0).rem_euclid(360.0) - 180.0).abs()
}

#[test]
fn ffi_sankranti_search_ex_default_config_works() {
    let Some(engine_ptr) = make_engine() else {
        return;
    };

    let request = DhruvSankrantiSearchRequest {
        target_kind: DHRUV_SANKRANTI_TARGET_ANY,
        query_mode: DHRUV_SANKRANTI_QUERY_MODE_NEXT,
        rashi_index: 0,
        time_kind: DHRUV_SEARCH_TIME_JD_TDB,
        at_jd_tdb: calendar_to_jd(2024, 3, 20.0),
        start_jd_tdb: 0.0,
        end_jd_tdb: 0.0,
        at_utc: ZEROED_UTC,
        start_utc: ZEROED_UTC,
        end_utc: ZEROED_UTC,
        config: dhruv_sankranti_config_default(),
    };
    let mut event: DhruvSankrantiEvent = unsafe { std::mem::zeroed() };
    let mut found: u8 = 0;

    // SAFETY: Valid pointers and request for this test scope.
    let status = unsafe {
        dhruv_sankranti_search_ex(
            engine_ptr,
            &request,
            &mut event,
            &mut found,
            ptr::null_mut(),
            0,
            ptr::null_mut(),
        )
    };
    assert_eq!(status, DhruvStatus::Ok);
    assert_eq!(found, 1);

    // SAFETY: Pointer was returned by dhruv_engine_new.
    unsafe { dhruv_engine_free(engine_ptr) };
}

#[test]
fn ffi_conjunction_search_ex_default_config_works() {
    let Some(engine_ptr) = make_engine() else {
        return;
    };

    let request = DhruvConjunctionSearchRequest {
        body1_code: Body::Sun.code(),
        body2_code: Body::Mercury.code(),
        query_mode: DHRUV_CONJUNCTION_QUERY_MODE_NEXT,
        time_kind: DHRUV_SEARCH_TIME_JD_TDB,
        at_jd_tdb: 2_460_390.5,
        start_jd_tdb: 0.0,
        end_jd_tdb: 0.0,
        at_utc: ZEROED_UTC,
        start_utc: ZEROED_UTC,
        end_utc: ZEROED_UTC,
        config: dhruv_conjunction_config_default(),
    };
    let mut event: DhruvConjunctionEvent = unsafe { std::mem::zeroed() };
    let mut found: u8 = 0;

    // SAFETY: Valid pointers and request for this test scope.
    let status = unsafe {
        dhruv_conjunction_search_ex(
            engine_ptr,
            &request,
            &mut event,
            &mut found,
            ptr::null_mut(),
            0,
            ptr::null_mut(),
        )
    };
    assert_eq!(status, DhruvStatus::Ok);
    assert_eq!(found, 1);

    // SAFETY: Pointer was returned by dhruv_engine_new.
    unsafe { dhruv_engine_free(engine_ptr) };
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
    let status = unsafe { dhruv_lsk_load(path.as_ptr() as *const std::ffi::c_char, &mut lsk_ptr) };
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
    let status = unsafe { dhruv_lsk_load(path.as_ptr() as *const std::ffi::c_char, &mut lsk_ptr) };
    assert_eq!(status, DhruvStatus::Ok);

    let jd_tdb = utc_to_tdb_jd_default(
        lsk_ptr,
        DhruvUtcTime {
            year: 2000,
            month: 1,
            day: 1,
            hour: 12,
            minute: 0,
            second: 0.0,
        },
    );

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
    let status =
        unsafe { dhruv_lsk_load(lsk_path.as_ptr() as *const std::ffi::c_char, &mut lsk_ptr) };
    assert_eq!(status, DhruvStatus::Ok);

    // Load engine for queries
    let mut engine_ptr: *mut DhruvEngineHandle = ptr::null_mut();
    // SAFETY: Valid pointers.
    let status = unsafe { dhruv_engine_new(&config, &mut engine_ptr) };
    assert_eq!(status, DhruvStatus::Ok);

    // Step 1: UTC to TDB JD (uses LSK, not engine)
    let jd_tdb = utc_to_tdb_jd_default(
        lsk_ptr,
        DhruvUtcTime {
            year: 2024,
            month: 3,
            day: 20,
            hour: 12,
            minute: 0,
            second: 0.0,
        },
    );

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
        lon_deg: 0.0,
        lat_deg: 0.0,
        distance_km: 0.0,
    };
    // SAFETY: Both pointers are valid.
    let status = unsafe { dhruv_cartesian_to_spherical(&state.position_km, &mut spherical) };
    assert_eq!(status, DhruvStatus::Ok);

    assert!(
        spherical.lon_deg >= 0.0 && spherical.lon_deg < 360.0,
        "longitude {} out of range",
        spherical.lon_deg
    );
    assert!(
        spherical.distance_km > 1.0e8,
        "Mars should be >1 AU from Sun"
    );

    // SAFETY: Pointers were returned by their respective _new/_load functions.
    unsafe { dhruv_engine_free(engine_ptr) };
    unsafe { dhruv_lsk_free(lsk_ptr) };
}

#[test]
fn ffi_query_request_utc_spherical_mars_heliocentric() {
    let engine_ptr = match make_engine() {
        Some(e) => e,
        None => return,
    };

    let request = DhruvQueryRequest {
        target: Body::Mars.code(),
        observer: Body::Sun.code(),
        frame: Frame::EclipticJ2000.code(),
        time_kind: DHRUV_QUERY_TIME_UTC,
        epoch_tdb_jd: 0.0,
        utc: DhruvUtcTime {
            year: 2024,
            month: 3,
            day: 20,
            hour: 12,
            minute: 0,
            second: 0.0,
        },
        output_mode: DHRUV_QUERY_OUTPUT_SPHERICAL,
    };
    let mut out: DhruvQueryResult = unsafe { std::mem::zeroed() };

    // Mars heliocentric ecliptic at 2024-03-20 12:00:00 UTC
    // SAFETY: Engine handle and output are valid in this test.
    let status = unsafe { dhruv_engine_query_request(engine_ptr, &request, &mut out) };
    assert_eq!(status, DhruvStatus::Ok);

    assert!(
        out.spherical_state.lon_deg >= 0.0 && out.spherical_state.lon_deg < 360.0,
        "longitude {} out of range",
        out.spherical_state.lon_deg
    );
    assert!(
        out.spherical_state.distance_km > 1.0e8,
        "Mars should be >1 AU from Sun"
    );

    // SAFETY: Pointer was returned by dhruv_engine_new.
    unsafe { dhruv_engine_free(engine_ptr) };
}

#[test]
fn ffi_query_request_utc_spherical_speeds_finite() {
    let engine_ptr = match make_engine() {
        Some(e) => e,
        None => return,
    };

    let request = DhruvQueryRequest {
        target: Body::Earth.code(),
        observer: Body::Sun.code(),
        frame: Frame::EclipticJ2000.code(),
        time_kind: DHRUV_QUERY_TIME_UTC,
        epoch_tdb_jd: 0.0,
        utc: DhruvUtcTime {
            year: 2024,
            month: 6,
            day: 15,
            hour: 0,
            minute: 0,
            second: 0.0,
        },
        output_mode: DHRUV_QUERY_OUTPUT_SPHERICAL,
    };
    let mut out: DhruvQueryResult = unsafe { std::mem::zeroed() };

    // Earth ecliptic heliocentric — a moving body with known non-zero velocity
    // SAFETY: Engine handle and output are valid in this test.
    let status = unsafe { dhruv_engine_query_request(engine_ptr, &request, &mut out) };
    assert_eq!(status, DhruvStatus::Ok);

    assert!(
        out.spherical_state.lon_speed.is_finite(),
        "lon_speed not finite"
    );
    assert!(
        out.spherical_state.lat_speed.is_finite(),
        "lat_speed not finite"
    );
    assert!(
        out.spherical_state.distance_speed.is_finite(),
        "distance_speed not finite"
    );
    assert!(
        out.spherical_state.lon_speed != 0.0,
        "lon_speed should be non-zero for orbiting body"
    );

    // SAFETY: Pointer was returned by dhruv_engine_new.
    unsafe { dhruv_engine_free(engine_ptr) };
}

// ---------------------------------------------------------------------------
// Ayanamsha integration tests
// ---------------------------------------------------------------------------

#[test]
fn ffi_ayanamsha_mean_lahiri_j2000() {
    let mut out: f64 = 0.0;
    let req = DhruvAyanamshaComputeRequest {
        system_code: 0,
        mode: DHRUV_AYANAMSHA_MODE_MEAN,
        time_kind: DHRUV_AYANAMSHA_TIME_JD_TDB,
        jd_tdb: 2_451_545.0,
        utc: DhruvUtcTime {
            year: 2000,
            month: 1,
            day: 1,
            hour: 12,
            minute: 0,
            second: 0.0,
        },
        use_nutation: 0,
        delta_psi_arcsec: 0.0,
    };
    // SAFETY: Valid output pointer.
    let status = unsafe { dhruv_ayanamsha_compute_ex(ptr::null(), &req, ptr::null(), &mut out) };
    assert_eq!(status, DhruvStatus::Ok);
    assert!(
        (out - 23.857_052_898_247_307).abs() < 1e-12,
        "Lahiri at J2000 = {out}, expected mean anchor reference"
    );
}

#[test]
fn ffi_ayanamsha_all_systems_valid() {
    let count = dhruv_ayanamsha_system_count();
    for code in 0..count as i32 {
        let mut out: f64 = 0.0;
        let req = DhruvAyanamshaComputeRequest {
            system_code: code,
            mode: DHRUV_AYANAMSHA_MODE_MEAN,
            time_kind: DHRUV_AYANAMSHA_TIME_JD_TDB,
            jd_tdb: 2_451_545.0,
            utc: DhruvUtcTime {
                year: 2000,
                month: 1,
                day: 1,
                hour: 12,
                minute: 0,
                second: 0.0,
            },
            use_nutation: 0,
            delta_psi_arcsec: 0.0,
        };
        // SAFETY: Valid output pointer.
        let status =
            unsafe { dhruv_ayanamsha_compute_ex(ptr::null(), &req, ptr::null(), &mut out) };
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
    let status = unsafe { dhruv_eop_load(path.as_ptr() as *const std::ffi::c_char, &mut eop_ptr) };
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
    let status =
        unsafe { dhruv_lsk_load(lsk_path.as_ptr() as *const std::ffi::c_char, &mut lsk_ptr) };
    assert_eq!(status, DhruvStatus::Ok);

    let mut eop_ptr: *mut DhruvEopHandle = ptr::null_mut();
    // SAFETY: Valid pointers.
    let status =
        unsafe { dhruv_eop_load(eop_path.as_ptr() as *const std::ffi::c_char, &mut eop_ptr) };
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
            engine_ptr,
            lsk_ptr,
            eop_ptr,
            &loc,
            DHRUV_EVENT_SUNRISE,
            noon,
            &cfg,
            &mut result,
        )
    };
    assert_eq!(status, DhruvStatus::Ok);
    assert_eq!(result.result_type, DHRUV_RISESET_EVENT);
    assert_eq!(result.event_code, DHRUV_EVENT_SUNRISE);

    // Sunrise in New Delhi on 2024-03-20 is ~00:48 UTC (06:18 IST)
    // Convert to UTC for validation
    let mut utc = DhruvUtcTime {
        year: 0,
        month: 0,
        day: 0,
        hour: 0,
        minute: 0,
        second: 0.0,
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
        utc.hour,
        utc.minute
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
    let status =
        unsafe { dhruv_lsk_load(lsk_path.as_ptr() as *const std::ffi::c_char, &mut lsk_ptr) };
    assert_eq!(status, DhruvStatus::Ok);

    let mut eop_ptr: *mut DhruvEopHandle = ptr::null_mut();
    // SAFETY: Valid pointers.
    let status =
        unsafe { dhruv_eop_load(eop_path.as_ptr() as *const std::ffi::c_char, &mut eop_ptr) };
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
            engine_ptr,
            lsk_ptr,
            eop_ptr,
            &loc,
            DHRUV_EVENT_SUNRISE,
            noon,
            &cfg,
            &mut result,
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
        year: 0,
        month: 0,
        day: 0,
        hour: 0,
        minute: 0,
        second: 0.0,
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
    let status =
        unsafe { dhruv_lsk_load(lsk_path.as_ptr() as *const std::ffi::c_char, &mut lsk_ptr) };
    assert_eq!(status, DhruvStatus::Ok);

    let mut eop_ptr: *mut DhruvEopHandle = ptr::null_mut();
    // SAFETY: Valid pointers.
    let status =
        unsafe { dhruv_eop_load(eop_path.as_ptr() as *const std::ffi::c_char, &mut eop_ptr) };
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
            engine_ptr,
            lsk_ptr,
            eop_ptr,
            &loc,
            noon,
            &cfg,
            results.as_mut_ptr(),
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
            i,
            results[i].jd_tdb,
            i + 1,
            results[i + 1].jd_tdb
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
    let status = unsafe { dhruv_lsk_load(path.as_ptr() as *const std::ffi::c_char, &mut lsk_ptr) };
    assert_eq!(status, DhruvStatus::Ok);

    let mut utc = DhruvUtcTime {
        year: 0,
        month: 0,
        day: 0,
        hour: 0,
        minute: 0,
        second: 0.0,
    };

    // J2000.0 TDB ≈ 2000-01-01 12:00:00 UTC (within ~1 minute)
    // SAFETY: All pointers are valid.
    let status = unsafe { dhruv_jd_tdb_to_utc(lsk_ptr, 2_451_545.0, &mut utc) };
    assert_eq!(status, DhruvStatus::Ok);
    assert_eq!(utc.year, 2000);
    assert_eq!(utc.month, 1);
    assert_eq!(utc.day, 1);
    assert_eq!(utc.hour, 11); // TDB-UTC offset ~64s, so hour rounds to 11
    assert!(
        utc.minute >= 58,
        "expected ~11:58-11:59, got {:02}:{:02}",
        utc.hour,
        utc.minute
    );

    // SAFETY: Pointer was returned by dhruv_lsk_load.
    unsafe { dhruv_lsk_free(lsk_ptr) };
}

// ---------------------------------------------------------------------------
// Nutation integration tests
// ---------------------------------------------------------------------------

#[test]
fn ffi_nutation_at_2024() {
    let mut dpsi: f64 = 0.0;
    let mut deps: f64 = 0.0;
    let jd = 2_460_310.5; // ~2024-01-01
    // SAFETY: Valid pointers.
    let status = unsafe { dhruv_nutation_iau2000b(jd, &mut dpsi, &mut deps) };
    assert_eq!(status, DhruvStatus::Ok);
    assert!(dpsi.abs() < 18.0, "Δψ = {dpsi}");
    assert!(deps.abs() < 10.0, "Δε = {deps}");
}

#[test]
fn ffi_ayanamsha_deg_unified_matches_mean() {
    let mut unified: f64 = 0.0;
    let mut mean: f64 = 0.0;
    let jd = 2_460_310.5;
    let req_unified = DhruvAyanamshaComputeRequest {
        system_code: 0,
        mode: DHRUV_AYANAMSHA_MODE_UNIFIED,
        time_kind: DHRUV_AYANAMSHA_TIME_JD_TDB,
        jd_tdb: jd,
        utc: DhruvUtcTime {
            year: 2000,
            month: 1,
            day: 1,
            hour: 12,
            minute: 0,
            second: 0.0,
        },
        use_nutation: 0,
        delta_psi_arcsec: 0.0,
    };
    let req_mean = DhruvAyanamshaComputeRequest {
        mode: DHRUV_AYANAMSHA_MODE_MEAN,
        ..req_unified
    };
    // SAFETY: Valid pointers.
    let s1 =
        unsafe { dhruv_ayanamsha_compute_ex(ptr::null(), &req_unified, ptr::null(), &mut unified) };
    let s2 = unsafe { dhruv_ayanamsha_compute_ex(ptr::null(), &req_mean, ptr::null(), &mut mean) };
    assert_eq!(s1, DhruvStatus::Ok);
    assert_eq!(s2, DhruvStatus::Ok);
    assert!(
        (unified - mean).abs() < 1e-12,
        "unified={unified}, mean={mean}"
    );
}

// ---------------------------------------------------------------------------
// Sun limb comparison tests (require kernels)
// ---------------------------------------------------------------------------

#[test]
fn ffi_sunrise_lower_limb_later_than_upper() {
    if !all_kernels_available() {
        eprintln!("Skipping: not all kernel files available");
        return;
    }

    let engine_ptr = make_engine().unwrap();
    let lsk_path = lsk_path_cstr().unwrap();
    let eop_path = eop_path_cstr().unwrap();

    let mut lsk_ptr: *mut DhruvLskHandle = ptr::null_mut();
    let status =
        unsafe { dhruv_lsk_load(lsk_path.as_ptr() as *const std::ffi::c_char, &mut lsk_ptr) };
    assert_eq!(status, DhruvStatus::Ok);

    let mut eop_ptr: *mut DhruvEopHandle = ptr::null_mut();
    let status =
        unsafe { dhruv_eop_load(eop_path.as_ptr() as *const std::ffi::c_char, &mut eop_ptr) };
    assert_eq!(status, DhruvStatus::Ok);

    let loc = DhruvGeoLocation {
        latitude_deg: 28.6139,
        longitude_deg: 77.209,
        altitude_m: 0.0,
    };
    let jd_0h = calendar_to_jd(2024, 3, 20.0);
    let noon = dhruv_approximate_local_noon_jd(jd_0h, loc.longitude_deg);

    // UpperLimb sunrise
    let cfg_upper = DhruvRiseSetConfig {
        use_refraction: 1,
        sun_limb: DHRUV_SUN_LIMB_UPPER,
        altitude_correction: 1,
    };
    let mut result_upper = DhruvRiseSetResult {
        result_type: -1,
        event_code: -1,
        jd_tdb: 0.0,
    };
    // SAFETY: All pointers are valid.
    let status = unsafe {
        dhruv_compute_rise_set(
            engine_ptr,
            lsk_ptr,
            eop_ptr,
            &loc,
            DHRUV_EVENT_SUNRISE,
            noon,
            &cfg_upper,
            &mut result_upper,
        )
    };
    assert_eq!(status, DhruvStatus::Ok);
    assert_eq!(result_upper.result_type, DHRUV_RISESET_EVENT);

    // LowerLimb sunrise
    let cfg_lower = DhruvRiseSetConfig {
        use_refraction: 1,
        sun_limb: DHRUV_SUN_LIMB_LOWER,
        altitude_correction: 1,
    };
    let mut result_lower = DhruvRiseSetResult {
        result_type: -1,
        event_code: -1,
        jd_tdb: 0.0,
    };
    // SAFETY: All pointers are valid.
    let status = unsafe {
        dhruv_compute_rise_set(
            engine_ptr,
            lsk_ptr,
            eop_ptr,
            &loc,
            DHRUV_EVENT_SUNRISE,
            noon,
            &cfg_lower,
            &mut result_lower,
        )
    };
    assert_eq!(status, DhruvStatus::Ok);
    assert_eq!(result_lower.result_type, DHRUV_RISESET_EVENT);

    // Lower limb sunrise should be LATER than upper limb sunrise
    // (lower limb needs to rise higher → takes more time)
    assert!(
        result_lower.jd_tdb > result_upper.jd_tdb,
        "LowerLimb sunrise (jd={}) should be after UpperLimb (jd={})",
        result_lower.jd_tdb,
        result_upper.jd_tdb
    );

    // SAFETY: cleanup
    unsafe { dhruv_eop_free(eop_ptr) };
    unsafe { dhruv_lsk_free(lsk_ptr) };
    unsafe { dhruv_engine_free(engine_ptr) };
}

#[test]
fn ffi_center_mode_between_limbs() {
    if !all_kernels_available() {
        eprintln!("Skipping: not all kernel files available");
        return;
    }

    let engine_ptr = make_engine().unwrap();
    let lsk_path = lsk_path_cstr().unwrap();
    let eop_path = eop_path_cstr().unwrap();

    let mut lsk_ptr: *mut DhruvLskHandle = ptr::null_mut();
    let status =
        unsafe { dhruv_lsk_load(lsk_path.as_ptr() as *const std::ffi::c_char, &mut lsk_ptr) };
    assert_eq!(status, DhruvStatus::Ok);

    let mut eop_ptr: *mut DhruvEopHandle = ptr::null_mut();
    let status =
        unsafe { dhruv_eop_load(eop_path.as_ptr() as *const std::ffi::c_char, &mut eop_ptr) };
    assert_eq!(status, DhruvStatus::Ok);

    let loc = DhruvGeoLocation {
        latitude_deg: 28.6139,
        longitude_deg: 77.209,
        altitude_m: 0.0,
    };
    let jd_0h = calendar_to_jd(2024, 3, 20.0);
    let noon = dhruv_approximate_local_noon_jd(jd_0h, loc.longitude_deg);

    let mut results = [0.0_f64; 3]; // upper, center, lower
    for (i, limb_code) in [
        DHRUV_SUN_LIMB_UPPER,
        DHRUV_SUN_LIMB_CENTER,
        DHRUV_SUN_LIMB_LOWER,
    ]
    .iter()
    .enumerate()
    {
        let cfg = DhruvRiseSetConfig {
            use_refraction: 1,
            sun_limb: *limb_code,
            altitude_correction: 1,
        };
        let mut result = DhruvRiseSetResult {
            result_type: -1,
            event_code: -1,
            jd_tdb: 0.0,
        };
        // SAFETY: All pointers are valid.
        let status = unsafe {
            dhruv_compute_rise_set(
                engine_ptr,
                lsk_ptr,
                eop_ptr,
                &loc,
                DHRUV_EVENT_SUNRISE,
                noon,
                &cfg,
                &mut result,
            )
        };
        assert_eq!(status, DhruvStatus::Ok);
        assert_eq!(result.result_type, DHRUV_RISESET_EVENT);
        results[i] = result.jd_tdb;
    }

    // Order should be: UpperLimb < Center < LowerLimb
    assert!(
        results[0] < results[1],
        "UpperLimb ({}) should be before Center ({})",
        results[0],
        results[1]
    );
    assert!(
        results[1] < results[2],
        "Center ({}) should be before LowerLimb ({})",
        results[1],
        results[2]
    );

    // SAFETY: cleanup
    unsafe { dhruv_eop_free(eop_ptr) };
    unsafe { dhruv_lsk_free(lsk_ptr) };
    unsafe { dhruv_engine_free(engine_ptr) };
}

// ---------------------------------------------------------------------------
// Bhava (house) integration tests
// ---------------------------------------------------------------------------

#[test]
fn ffi_bhava_equal_new_delhi() {
    if !all_kernels_available() {
        eprintln!("Skipping: not all kernel files available");
        return;
    }

    let engine_ptr = make_engine().unwrap();
    let lsk_path = lsk_path_cstr().unwrap();
    let eop_path = eop_path_cstr().unwrap();

    let mut lsk_ptr: *mut DhruvLskHandle = ptr::null_mut();
    let status =
        unsafe { dhruv_lsk_load(lsk_path.as_ptr() as *const std::ffi::c_char, &mut lsk_ptr) };
    assert_eq!(status, DhruvStatus::Ok);

    let mut eop_ptr: *mut DhruvEopHandle = ptr::null_mut();
    let status =
        unsafe { dhruv_eop_load(eop_path.as_ptr() as *const std::ffi::c_char, &mut eop_ptr) };
    assert_eq!(status, DhruvStatus::Ok);

    let loc = DhruvGeoLocation {
        latitude_deg: 28.6139,
        longitude_deg: 77.209,
        altitude_m: 0.0,
    };

    let jd_utc = calendar_to_jd(2024, 3, 20.0) + 0.5; // noon UT
    let cfg = dhruv_bhava_config_default();

    let mut result = DhruvBhavaResult {
        bhavas: [DhruvBhava {
            number: 0,
            cusp_deg: 0.0,
            start_deg: 0.0,
            end_deg: 0.0,
        }; 12],
        lagna_deg: 0.0,
        mc_deg: 0.0,
        rashi_bhava_valid: 0,
        rashi_bhava_bhavas: [DhruvBhava {
            number: 0,
            cusp_deg: 0.0,
            start_deg: 0.0,
            end_deg: 0.0,
        }; 12],
        rashi_bhava_lagna_deg: 0.0,
        rashi_bhava_mc_deg: 0.0,
    };

    // SAFETY: All pointers are valid.
    let status = unsafe {
        dhruv_compute_bhavas(
            engine_ptr,
            lsk_ptr,
            eop_ptr,
            &loc,
            jd_utc,
            &cfg,
            &mut result,
        )
    };
    assert_eq!(status, DhruvStatus::Ok);

    // Ascendant should be in [0, 360)
    assert!(
        result.lagna_deg >= 0.0 && result.lagna_deg < 360.0,
        "Asc = {} deg",
        result.lagna_deg
    );

    // Equal cusps: each 30 deg apart
    for i in 0..12 {
        let next = (i + 1) % 12;
        let diff = (result.bhavas[next].cusp_deg - result.bhavas[i].cusp_deg).rem_euclid(360.0);
        assert!(
            (diff - 30.0).abs() < 0.01,
            "cusp diff [{i}->{next}] = {diff}, expected 30",
        );
    }

    // Cusp 1 ≈ Ascendant
    assert!(
        (result.bhavas[0].cusp_deg - result.lagna_deg).abs() < 0.01,
        "cusp 1 = {}, Asc = {}",
        result.bhavas[0].cusp_deg,
        result.lagna_deg
    );

    unsafe { dhruv_eop_free(eop_ptr) };
    unsafe { dhruv_lsk_free(lsk_ptr) };
    unsafe { dhruv_engine_free(engine_ptr) };
}

#[test]
fn ffi_lagna_deg_new_delhi() {
    if !all_kernels_available() {
        eprintln!("Skipping: not all kernel files available");
        return;
    }

    let lsk_path = lsk_path_cstr().unwrap();
    let eop_path = eop_path_cstr().unwrap();

    let mut lsk_ptr: *mut DhruvLskHandle = ptr::null_mut();
    let status =
        unsafe { dhruv_lsk_load(lsk_path.as_ptr() as *const std::ffi::c_char, &mut lsk_ptr) };
    assert_eq!(status, DhruvStatus::Ok);

    let mut eop_ptr: *mut DhruvEopHandle = ptr::null_mut();
    let status =
        unsafe { dhruv_eop_load(eop_path.as_ptr() as *const std::ffi::c_char, &mut eop_ptr) };
    assert_eq!(status, DhruvStatus::Ok);

    let loc = DhruvGeoLocation {
        latitude_deg: 28.6139,
        longitude_deg: 77.209,
        altitude_m: 0.0,
    };

    let jd_utc = calendar_to_jd(2024, 3, 20.0) + 0.5;
    let mut asc: f64 = 0.0;

    // SAFETY: All pointers are valid.
    let status = unsafe { dhruv_lagna_deg(lsk_ptr, eop_ptr, &loc, jd_utc, &mut asc) };
    assert_eq!(status, DhruvStatus::Ok);
    assert!(
        asc >= 0.0 && asc < 360.0,
        "Ascendant = {} deg, out of range",
        asc
    );

    // Rising-condition check: the FFI returns tropical lagna in degrees.
    // Reconstruct apparent LST (GAST) and true obliquity — matching production.
    {
        let lsk_rust =
            dhruv_time::LeapSecondKernel::load(&kernel_base().join("naif0012.tls")).unwrap();
        let eop_rust = dhruv_time::EopKernel::load(&kernel_base().join("finals2000A.all")).unwrap();
        let jd_ut1 = eop_rust.utc_to_ut1_jd(jd_utc).expect("EOP lookup");
        let gmst = gmst_rad(jd_ut1);
        let utc_s = dhruv_time::jd_to_tdb_seconds(jd_utc);
        let tdb_s = lsk_rust.utc_to_tdb(utc_s);
        let jd_tdb = dhruv_time::tdb_seconds_to_jd(tdb_s);
        let t = (jd_tdb - 2_451_545.0) / 36525.0;
        let (ee, eps_true) = dhruv_frames::equation_of_equinoxes_and_true_obliquity(t);
        let gast = gmst + ee;
        let lst = local_sidereal_time_rad(gast, loc.longitude_deg.to_radians());

        let lagna_rad = asc.to_radians();
        let ra = f64::atan2(lagna_rad.sin() * eps_true.cos(), lagna_rad.cos()).rem_euclid(TAU);
        let mut h = (lst - ra).rem_euclid(TAU);
        if h > PI {
            h -= TAU;
        }
        assert!(
            h < 0.0,
            "H = {:.4} rad ({:.2} deg) — FFI lagna should be rising (H < 0)",
            h,
            h.to_degrees()
        );
    }

    // MC should also work
    let mut mc: f64 = 0.0;
    let status = unsafe { dhruv_mc_deg(lsk_ptr, eop_ptr, &loc, jd_utc, &mut mc) };
    assert_eq!(status, DhruvStatus::Ok);
    assert!(mc >= 0.0 && mc < 360.0, "MC = {} deg, out of range", mc);

    unsafe { dhruv_eop_free(eop_ptr) };
    unsafe { dhruv_lsk_free(lsk_ptr) };
}

/// Verify dhruv_lagna_deg (JD UTC) and dhruv_lagna_deg_utc (UTC struct) agree.
#[test]
fn ffi_lagna_variants_agree() {
    if !all_kernels_available() {
        eprintln!("Skipping: not all kernel files available");
        return;
    }

    let lsk_path = lsk_path_cstr().unwrap();
    let eop_path = eop_path_cstr().unwrap();

    let mut lsk_ptr: *mut DhruvLskHandle = ptr::null_mut();
    let status =
        unsafe { dhruv_lsk_load(lsk_path.as_ptr() as *const std::ffi::c_char, &mut lsk_ptr) };
    assert_eq!(status, DhruvStatus::Ok);

    let mut eop_ptr: *mut DhruvEopHandle = ptr::null_mut();
    let status =
        unsafe { dhruv_eop_load(eop_path.as_ptr() as *const std::ffi::c_char, &mut eop_ptr) };
    assert_eq!(status, DhruvStatus::Ok);

    let loc = DhruvGeoLocation {
        latitude_deg: 28.6139,
        longitude_deg: 77.209,
        altitude_m: 0.0,
    };

    let jd_utc = calendar_to_jd(2024, 3, 20.5);
    let utc = DhruvUtcTime {
        year: 2024,
        month: 3,
        day: 20,
        hour: 12,
        minute: 0,
        second: 0.0,
    };

    let mut asc_jd: f64 = 0.0;
    let mut asc_utc: f64 = 0.0;

    let s1 = unsafe { dhruv_lagna_deg(lsk_ptr, eop_ptr, &loc, jd_utc, &mut asc_jd) };
    assert_eq!(s1, DhruvStatus::Ok);

    let s2 = unsafe { dhruv_lagna_deg_utc(lsk_ptr, eop_ptr, &loc, &utc, &mut asc_utc) };
    assert_eq!(s2, DhruvStatus::Ok);

    assert!(
        (asc_jd - asc_utc).abs() < 0.001,
        "JD variant={asc_jd}, UTC variant={asc_utc}"
    );

    unsafe { dhruv_eop_free(eop_ptr) };
    unsafe { dhruv_lsk_free(lsk_ptr) };
}

#[test]
fn ffi_bhava_body_starting_point() {
    if !all_kernels_available() {
        eprintln!("Skipping: not all kernel files available");
        return;
    }

    let engine_ptr = make_engine().unwrap();
    let lsk_path = lsk_path_cstr().unwrap();
    let eop_path = eop_path_cstr().unwrap();

    let mut lsk_ptr: *mut DhruvLskHandle = ptr::null_mut();
    let status =
        unsafe { dhruv_lsk_load(lsk_path.as_ptr() as *const std::ffi::c_char, &mut lsk_ptr) };
    assert_eq!(status, DhruvStatus::Ok);

    let mut eop_ptr: *mut DhruvEopHandle = ptr::null_mut();
    let status =
        unsafe { dhruv_eop_load(eop_path.as_ptr() as *const std::ffi::c_char, &mut eop_ptr) };
    assert_eq!(status, DhruvStatus::Ok);

    let loc = DhruvGeoLocation {
        latitude_deg: 28.6139,
        longitude_deg: 77.209,
        altitude_m: 0.0,
    };

    let jd_utc = calendar_to_jd(2024, 3, 20.0) + 0.5;

    // Use Sun as starting point (body code = 10)
    let cfg = DhruvBhavaConfig {
        system: DHRUV_BHAVA_EQUAL,
        starting_point: 10, // Sun's NAIF code
        custom_start_deg: 0.0,
        reference_mode: DHRUV_BHAVA_REF_START,
        ..dhruv_bhava_config_default()
    };

    let mut result = DhruvBhavaResult {
        bhavas: [DhruvBhava {
            number: 0,
            cusp_deg: 0.0,
            start_deg: 0.0,
            end_deg: 0.0,
        }; 12],
        lagna_deg: 0.0,
        mc_deg: 0.0,
        rashi_bhava_valid: 0,
        rashi_bhava_bhavas: [DhruvBhava {
            number: 0,
            cusp_deg: 0.0,
            start_deg: 0.0,
            end_deg: 0.0,
        }; 12],
        rashi_bhava_lagna_deg: 0.0,
        rashi_bhava_mc_deg: 0.0,
    };

    let status = unsafe {
        dhruv_compute_bhavas(
            engine_ptr,
            lsk_ptr,
            eop_ptr,
            &loc,
            jd_utc,
            &cfg,
            &mut result,
        )
    };
    assert_eq!(status, DhruvStatus::Ok);

    // Near equinox, Sun ≈ 0 deg ecliptic, so cusp 1 should be near 0/360
    assert!(
        result.bhavas[0].cusp_deg < 10.0 || result.bhavas[0].cusp_deg > 350.0,
        "cusp 1 = {} deg, expected near 0 (equinox Sun)",
        result.bhavas[0].cusp_deg
    );

    unsafe { dhruv_eop_free(eop_ptr) };
    unsafe { dhruv_lsk_free(lsk_ptr) };
    unsafe { dhruv_engine_free(engine_ptr) };
}

// ---------------------------------------------------------------------------
// UTC roundtrip integration tests
// ---------------------------------------------------------------------------

/// Helper: compare a DhruvUtcTime with an expected JD TDB by converting the UTC
/// time back to JD TDB via dhruv_utc_to_tdb_jd and checking they agree within
/// a tolerance (1 second ≈ 1.16e-5 days).
fn assert_utc_matches_jd(
    lsk_ptr: *mut DhruvLskHandle,
    utc: &DhruvUtcTime,
    expected_jd_tdb: f64,
    label: &str,
) {
    let request = default_utc_to_tdb_request(*utc);
    let mut out = DhruvUtcToTdbResult {
        jd_tdb: 0.0,
        diagnostics: DhruvTimeDiagnostics {
            source: 0,
            tt_minus_utc_s: 0.0,
            warning_count: 0,
            warnings: [DhruvTimeWarning {
                kind: 0,
                utc_seconds: 0.0,
                first_entry_utc_seconds: 0.0,
                last_entry_utc_seconds: 0.0,
                used_delta_at_seconds: 0.0,
                mjd: 0.0,
                first_entry_mjd: 0.0,
                last_entry_mjd: 0.0,
                used_dut1_seconds: 0.0,
                delta_t_model: 0,
                delta_t_segment: 0,
            }; DHRUV_MAX_TIME_WARNINGS],
        },
    };
    let status = unsafe { dhruv_utc_to_tdb_jd(lsk_ptr, ptr::null(), &request, &mut out) };
    assert_eq!(status, DhruvStatus::Ok, "{label}: utc_to_tdb_jd failed");
    let jd_roundtrip = out.jd_tdb;
    let diff_days = (jd_roundtrip - expected_jd_tdb).abs();
    assert!(
        diff_days < 2e-5, // ~1.7 seconds tolerance (roundtrip precision)
        "{label}: roundtrip diff = {diff_days} days (jd_rt={jd_roundtrip}, expected={expected_jd_tdb})"
    );
}

#[test]
fn ffi_utc_conjunction_roundtrip() {
    let engine_ptr = match make_engine() {
        Some(e) => e,
        None => return,
    };
    let lsk_path = lsk_path_cstr().unwrap();
    let mut lsk_ptr: *mut DhruvLskHandle = ptr::null_mut();
    let status =
        unsafe { dhruv_lsk_load(lsk_path.as_ptr() as *const std::ffi::c_char, &mut lsk_ptr) };
    assert_eq!(status, DhruvStatus::Ok);

    // Find next Sun-Moon conjunction after 2024-03-20 using direct JD input.
    let jd_start = calendar_to_jd(2024, 3, 20.5);
    let request_jd = DhruvConjunctionSearchRequest {
        body1_code: Body::Sun.code(),
        body2_code: Body::Moon.code(),
        query_mode: DHRUV_CONJUNCTION_QUERY_MODE_NEXT,
        time_kind: DHRUV_SEARCH_TIME_JD_TDB,
        at_jd_tdb: jd_start,
        start_jd_tdb: 0.0,
        end_jd_tdb: 0.0,
        at_utc: ZEROED_UTC,
        start_utc: ZEROED_UTC,
        end_utc: ZEROED_UTC,
        config: dhruv_conjunction_config_default(),
    };
    let mut jd_event: DhruvConjunctionEvent = unsafe { std::mem::zeroed() };
    let mut found: u8 = 0;
    let status = unsafe {
        dhruv_conjunction_search_ex(
            engine_ptr,
            &request_jd,
            &mut jd_event,
            &mut found,
            ptr::null_mut(),
            0,
            ptr::null_mut(),
        )
    };
    assert_eq!(status, DhruvStatus::Ok);
    assert_eq!(found, 1, "should find a conjunction");

    // Convert UTC start to JD and run the same unified query.
    let utc_start = DhruvUtcTime {
        year: 2024,
        month: 3,
        day: 20,
        hour: 12,
        minute: 0,
        second: 0.0,
    };
    let jd_from_utc_start = utc_to_tdb_jd_default(lsk_ptr, utc_start);
    let request_from_utc = DhruvConjunctionSearchRequest {
        time_kind: DHRUV_SEARCH_TIME_UTC,
        at_jd_tdb: jd_from_utc_start,
        at_utc: utc_start,
        ..request_jd
    };
    let mut utc_path_event: DhruvConjunctionEvent = unsafe { std::mem::zeroed() };
    let mut found_utc: u8 = 0;
    let status = unsafe {
        dhruv_conjunction_search_ex(
            engine_ptr,
            &request_from_utc,
            &mut utc_path_event,
            &mut found_utc,
            ptr::null_mut(),
            0,
            ptr::null_mut(),
        )
    };
    assert_eq!(status, DhruvStatus::Ok);
    assert_eq!(found_utc, 1, "UTC path should also find conjunction");

    assert!(
        (utc_path_event.jd_tdb - jd_event.jd_tdb).abs() < 2e-5,
        "time mismatch"
    );
    assert!(
        (utc_path_event.actual_separation_deg - jd_event.actual_separation_deg).abs() < 1e-6,
        "separation mismatch"
    );
    assert!(
        (utc_path_event.body1_longitude_deg - jd_event.body1_longitude_deg).abs() < 1e-6,
        "body1 lon mismatch"
    );

    unsafe { dhruv_lsk_free(lsk_ptr) };
    unsafe { dhruv_engine_free(engine_ptr) };
}

#[test]
fn ffi_utc_chandra_grahan_roundtrip() {
    let engine_ptr = match make_engine() {
        Some(e) => e,
        None => return,
    };
    let lsk_path = lsk_path_cstr().unwrap();
    let mut lsk_ptr: *mut DhruvLskHandle = ptr::null_mut();
    let status =
        unsafe { dhruv_lsk_load(lsk_path.as_ptr() as *const std::ffi::c_char, &mut lsk_ptr) };
    assert_eq!(status, DhruvStatus::Ok);

    let jd_start = calendar_to_jd(2024, 3, 1.0);
    let request_jd = DhruvGrahanSearchRequest {
        grahan_kind: DHRUV_GRAHAN_KIND_CHANDRA,
        query_mode: DHRUV_GRAHAN_QUERY_MODE_NEXT,
        time_kind: DHRUV_SEARCH_TIME_JD_TDB,
        at_jd_tdb: jd_start,
        start_jd_tdb: 0.0,
        end_jd_tdb: 0.0,
        at_utc: ZEROED_UTC,
        start_utc: ZEROED_UTC,
        end_utc: ZEROED_UTC,
        config: dhruv_grahan_config_default(),
    };
    let mut jd_result: DhruvChandraGrahanResult = unsafe { std::mem::zeroed() };
    let mut found: u8 = 0;
    let status = unsafe {
        dhruv_grahan_search_ex(
            engine_ptr,
            &request_jd,
            &mut jd_result,
            ptr::null_mut(),
            &mut found,
            ptr::null_mut(),
            ptr::null_mut(),
            0,
            ptr::null_mut(),
        )
    };
    assert_eq!(status, DhruvStatus::Ok);
    assert_eq!(found, 1, "should find a chandra grahan");

    // UTC path: convert anchor UTC -> JD, then call unified API.
    let utc_start = DhruvUtcTime {
        year: 2024,
        month: 3,
        day: 1,
        hour: 0,
        minute: 0,
        second: 0.0,
    };
    let jd_from_utc_start = utc_to_tdb_jd_default(lsk_ptr, utc_start);
    let request_from_utc = DhruvGrahanSearchRequest {
        time_kind: DHRUV_SEARCH_TIME_UTC,
        at_jd_tdb: jd_from_utc_start,
        at_utc: utc_start,
        ..request_jd
    };
    let mut utc_path_result: DhruvChandraGrahanResult = unsafe { std::mem::zeroed() };
    let mut found_utc: u8 = 0;
    let status = unsafe {
        dhruv_grahan_search_ex(
            engine_ptr,
            &request_from_utc,
            &mut utc_path_result,
            ptr::null_mut(),
            &mut found_utc,
            ptr::null_mut(),
            ptr::null_mut(),
            0,
            ptr::null_mut(),
        )
    };
    assert_eq!(status, DhruvStatus::Ok);
    assert_eq!(found_utc, 1, "UTC path should also find grahan");

    assert_eq!(
        utc_path_result.grahan_type, jd_result.grahan_type,
        "grahan type mismatch"
    );
    assert!(
        (utc_path_result.magnitude - jd_result.magnitude).abs() < 1e-6,
        "magnitude mismatch"
    );
    assert!(
        (utc_path_result.greatest_grahan_jd - jd_result.greatest_grahan_jd).abs() < 2e-5,
        "greatest grahan mismatch"
    );
    assert_eq!(
        utc_path_result.u1_jd < 0.0,
        jd_result.u1_jd < 0.0,
        "u1 presence mismatch"
    );
    assert_eq!(
        utc_path_result.u2_jd < 0.0,
        jd_result.u2_jd < 0.0,
        "u2 presence mismatch"
    );
    assert_eq!(
        utc_path_result.u3_jd < 0.0,
        jd_result.u3_jd < 0.0,
        "u3 presence mismatch"
    );
    assert_eq!(
        utc_path_result.u4_jd < 0.0,
        jd_result.u4_jd < 0.0,
        "u4 presence mismatch"
    );

    unsafe { dhruv_lsk_free(lsk_ptr) };
    unsafe { dhruv_engine_free(engine_ptr) };
}

#[test]
fn ffi_utc_surya_grahan_roundtrip() {
    let engine_ptr = match make_engine() {
        Some(e) => e,
        None => return,
    };
    let lsk_path = lsk_path_cstr().unwrap();
    let mut lsk_ptr: *mut DhruvLskHandle = ptr::null_mut();
    let status =
        unsafe { dhruv_lsk_load(lsk_path.as_ptr() as *const std::ffi::c_char, &mut lsk_ptr) };
    assert_eq!(status, DhruvStatus::Ok);

    let jd_start = calendar_to_jd(2024, 3, 1.0);
    let request_jd = DhruvGrahanSearchRequest {
        grahan_kind: DHRUV_GRAHAN_KIND_SURYA,
        query_mode: DHRUV_GRAHAN_QUERY_MODE_NEXT,
        time_kind: DHRUV_SEARCH_TIME_JD_TDB,
        at_jd_tdb: jd_start,
        start_jd_tdb: 0.0,
        end_jd_tdb: 0.0,
        at_utc: ZEROED_UTC,
        start_utc: ZEROED_UTC,
        end_utc: ZEROED_UTC,
        config: dhruv_grahan_config_default(),
    };
    let mut jd_result: DhruvSuryaGrahanResult = unsafe { std::mem::zeroed() };
    let mut found: u8 = 0;
    let status = unsafe {
        dhruv_grahan_search_ex(
            engine_ptr,
            &request_jd,
            ptr::null_mut(),
            &mut jd_result,
            &mut found,
            ptr::null_mut(),
            ptr::null_mut(),
            0,
            ptr::null_mut(),
        )
    };
    assert_eq!(status, DhruvStatus::Ok);
    assert_eq!(found, 1, "should find a surya grahan");

    // UTC path: convert anchor UTC -> JD, then call unified API.
    let utc_start = DhruvUtcTime {
        year: 2024,
        month: 3,
        day: 1,
        hour: 0,
        minute: 0,
        second: 0.0,
    };
    let jd_from_utc_start = utc_to_tdb_jd_default(lsk_ptr, utc_start);
    let request_from_utc = DhruvGrahanSearchRequest {
        time_kind: DHRUV_SEARCH_TIME_UTC,
        at_jd_tdb: jd_from_utc_start,
        at_utc: utc_start,
        ..request_jd
    };
    let mut utc_path_result: DhruvSuryaGrahanResult = unsafe { std::mem::zeroed() };
    let mut found_utc: u8 = 0;
    let status = unsafe {
        dhruv_grahan_search_ex(
            engine_ptr,
            &request_from_utc,
            ptr::null_mut(),
            &mut utc_path_result,
            &mut found_utc,
            ptr::null_mut(),
            ptr::null_mut(),
            0,
            ptr::null_mut(),
        )
    };
    assert_eq!(status, DhruvStatus::Ok);
    assert_eq!(found_utc, 1, "UTC path should also find grahan");

    assert_eq!(
        utc_path_result.grahan_type, jd_result.grahan_type,
        "grahan type mismatch"
    );
    assert!(
        (utc_path_result.magnitude - jd_result.magnitude).abs() < 1e-6,
        "magnitude mismatch"
    );
    assert!(
        (utc_path_result.greatest_grahan_jd - jd_result.greatest_grahan_jd).abs() < 2e-5,
        "greatest surya grahan mismatch"
    );
    assert_eq!(
        utc_path_result.c1_jd < 0.0,
        jd_result.c1_jd < 0.0,
        "c1 presence mismatch"
    );
    assert_eq!(
        utc_path_result.c2_jd < 0.0,
        jd_result.c2_jd < 0.0,
        "c2 presence mismatch"
    );
    assert_eq!(
        utc_path_result.c3_jd < 0.0,
        jd_result.c3_jd < 0.0,
        "c3 presence mismatch"
    );
    assert_eq!(
        utc_path_result.c4_jd < 0.0,
        jd_result.c4_jd < 0.0,
        "c4 presence mismatch"
    );

    unsafe { dhruv_lsk_free(lsk_ptr) };
    unsafe { dhruv_engine_free(engine_ptr) };
}

#[test]
fn ffi_utc_stationary_roundtrip() {
    let engine_ptr = match make_engine() {
        Some(e) => e,
        None => return,
    };
    let lsk_path = lsk_path_cstr().unwrap();
    let mut lsk_ptr: *mut DhruvLskHandle = ptr::null_mut();
    let status =
        unsafe { dhruv_lsk_load(lsk_path.as_ptr() as *const std::ffi::c_char, &mut lsk_ptr) };
    assert_eq!(status, DhruvStatus::Ok);

    let jd_start = calendar_to_jd(2024, 1, 1.0);
    let request_jd = DhruvMotionSearchRequest {
        body_code: Body::Mercury.code(),
        motion_kind: DHRUV_MOTION_KIND_STATIONARY,
        query_mode: DHRUV_MOTION_QUERY_MODE_NEXT,
        time_kind: DHRUV_SEARCH_TIME_JD_TDB,
        at_jd_tdb: jd_start,
        start_jd_tdb: 0.0,
        end_jd_tdb: 0.0,
        at_utc: ZEROED_UTC,
        start_utc: ZEROED_UTC,
        end_utc: ZEROED_UTC,
        config: dhruv_stationary_config_default(),
    };
    let mut jd_event: DhruvStationaryEvent = unsafe { std::mem::zeroed() };
    let mut found: u8 = 0;
    let status = unsafe {
        dhruv_motion_search_ex(
            engine_ptr,
            &request_jd,
            &mut jd_event,
            ptr::null_mut(),
            &mut found,
            ptr::null_mut(),
            ptr::null_mut(),
            0,
            ptr::null_mut(),
        )
    };
    assert_eq!(status, DhruvStatus::Ok);
    assert_eq!(found, 1);

    // UTC path: convert anchor UTC -> JD, then call unified API.
    let utc_start = DhruvUtcTime {
        year: 2024,
        month: 1,
        day: 1,
        hour: 0,
        minute: 0,
        second: 0.0,
    };
    let jd_from_utc_start = utc_to_tdb_jd_default(lsk_ptr, utc_start);
    let request_from_utc = DhruvMotionSearchRequest {
        time_kind: DHRUV_SEARCH_TIME_UTC,
        at_jd_tdb: jd_from_utc_start,
        at_utc: utc_start,
        ..request_jd
    };
    let mut utc_path_event: DhruvStationaryEvent = unsafe { std::mem::zeroed() };
    let mut found_utc: u8 = 0;
    let status = unsafe {
        dhruv_motion_search_ex(
            engine_ptr,
            &request_from_utc,
            &mut utc_path_event,
            ptr::null_mut(),
            &mut found_utc,
            ptr::null_mut(),
            ptr::null_mut(),
            0,
            ptr::null_mut(),
        )
    };
    assert_eq!(status, DhruvStatus::Ok);
    assert_eq!(found_utc, 1);

    assert!(
        (utc_path_event.jd_tdb - jd_event.jd_tdb).abs() < 2e-5,
        "stationary time mismatch"
    );
    assert_eq!(
        utc_path_event.station_type, jd_event.station_type,
        "station type mismatch"
    );
    assert!(
        (utc_path_event.longitude_deg - jd_event.longitude_deg).abs() < 1e-6,
        "longitude mismatch"
    );

    unsafe { dhruv_lsk_free(lsk_ptr) };
    unsafe { dhruv_engine_free(engine_ptr) };
}

#[test]
fn ffi_utc_query_roundtrip() {
    let engine_ptr = match make_engine() {
        Some(e) => e,
        None => return,
    };

    let request = DhruvQueryRequest {
        target: Body::Mars.code(),
        observer: Body::Sun.code(),
        frame: Frame::EclipticJ2000.code(),
        time_kind: DHRUV_QUERY_TIME_UTC,
        epoch_tdb_jd: 0.0,
        utc: DhruvUtcTime {
            year: 2024,
            month: 6,
            day: 15,
            hour: 0,
            minute: 0,
            second: 0.0,
        },
        output_mode: DHRUV_QUERY_OUTPUT_SPHERICAL,
    };
    let mut out_first: DhruvQueryResult = unsafe { std::mem::zeroed() };
    let status = unsafe { dhruv_engine_query_request(engine_ptr, &request, &mut out_first) };
    assert_eq!(status, DhruvStatus::Ok);

    let request_struct = DhruvQueryRequest {
        utc: request.utc,
        ..request
    };
    let mut out_second: DhruvQueryResult = unsafe { std::mem::zeroed() };
    let status =
        unsafe { dhruv_engine_query_request(engine_ptr, &request_struct, &mut out_second) };
    assert_eq!(status, DhruvStatus::Ok);

    assert!(
        (out_second.spherical_state.lon_deg - out_first.spherical_state.lon_deg).abs() < 1e-12,
        "lon_deg mismatch"
    );
    assert!(
        (out_second.spherical_state.lat_deg - out_first.spherical_state.lat_deg).abs() < 1e-12,
        "lat_deg mismatch"
    );
    assert!(
        (out_second.spherical_state.distance_km - out_first.spherical_state.distance_km).abs()
            < 1e-6,
        "distance_km mismatch"
    );
    assert!(
        (out_second.spherical_state.lon_speed - out_first.spherical_state.lon_speed).abs() < 1e-12,
        "lon_speed mismatch"
    );

    unsafe { dhruv_engine_free(engine_ptr) };
}

#[test]
fn ffi_utc_ayanamsha_roundtrip() {
    let lsk_path = match lsk_path_cstr() {
        Some(p) => p,
        None => return,
    };
    let mut lsk_ptr: *mut DhruvLskHandle = ptr::null_mut();
    let status =
        unsafe { dhruv_lsk_load(lsk_path.as_ptr() as *const std::ffi::c_char, &mut lsk_ptr) };
    assert_eq!(status, DhruvStatus::Ok);

    // JD version: Lahiri ayanamsha at 2024-01-01 TDB
    let jd = 2_460_310.5;
    let mut deg_jd: f64 = 0.0;
    let req_jd = DhruvAyanamshaComputeRequest {
        system_code: 0,
        mode: DHRUV_AYANAMSHA_MODE_UNIFIED,
        time_kind: DHRUV_AYANAMSHA_TIME_JD_TDB,
        jd_tdb: jd,
        utc: DhruvUtcTime {
            year: 2000,
            month: 1,
            day: 1,
            hour: 12,
            minute: 0,
            second: 0.0,
        },
        use_nutation: 0,
        delta_psi_arcsec: 0.0,
    };
    let status =
        unsafe { dhruv_ayanamsha_compute_ex(ptr::null(), &req_jd, ptr::null(), &mut deg_jd) };
    assert_eq!(status, DhruvStatus::Ok);

    // UTC version: approximate 2024-01-01 00:00 UTC
    let utc = DhruvUtcTime {
        year: 2024,
        month: 1,
        day: 1,
        hour: 0,
        minute: 0,
        second: 0.0,
    };
    let mut deg_utc: f64 = 0.0;
    let req_utc = DhruvAyanamshaComputeRequest {
        system_code: 0,
        mode: DHRUV_AYANAMSHA_MODE_UNIFIED,
        time_kind: DHRUV_AYANAMSHA_TIME_UTC,
        jd_tdb: 0.0,
        utc,
        use_nutation: 0,
        delta_psi_arcsec: 0.0,
    };
    let status =
        unsafe { dhruv_ayanamsha_compute_ex(lsk_ptr, &req_utc, ptr::null(), &mut deg_utc) };
    assert_eq!(status, DhruvStatus::Ok);

    // Should be very close (TDB-UTC offset is ~69 seconds, tiny ayanamsha difference)
    assert!(
        (deg_utc - deg_jd).abs() < 0.001,
        "ayanamsha mismatch: utc={deg_utc}, jd={deg_jd}"
    );

    unsafe { dhruv_lsk_free(lsk_ptr) };
}

#[test]
fn ffi_utc_sunrise_roundtrip() {
    if !all_kernels_available() {
        eprintln!("Skipping: not all kernel files available");
        return;
    }

    let engine_ptr = make_engine().unwrap();
    let lsk_path = lsk_path_cstr().unwrap();
    let eop_path = eop_path_cstr().unwrap();

    let mut lsk_ptr: *mut DhruvLskHandle = ptr::null_mut();
    let status =
        unsafe { dhruv_lsk_load(lsk_path.as_ptr() as *const std::ffi::c_char, &mut lsk_ptr) };
    assert_eq!(status, DhruvStatus::Ok);

    let mut eop_ptr: *mut DhruvEopHandle = ptr::null_mut();
    let status =
        unsafe { dhruv_eop_load(eop_path.as_ptr() as *const std::ffi::c_char, &mut eop_ptr) };
    assert_eq!(status, DhruvStatus::Ok);

    let loc = DhruvGeoLocation {
        latitude_deg: 28.6139,
        longitude_deg: 77.209,
        altitude_m: 0.0,
    };
    let jd_0h = calendar_to_jd(2024, 3, 20.0);
    let noon_jd = dhruv_approximate_local_noon_jd(jd_0h, loc.longitude_deg);
    let cfg = dhruv_riseset_config_default();

    // JD version
    let mut jd_result = DhruvRiseSetResult {
        result_type: -1,
        event_code: -1,
        jd_tdb: 0.0,
    };
    let status = unsafe {
        dhruv_compute_rise_set(
            engine_ptr,
            lsk_ptr,
            eop_ptr,
            &loc,
            DHRUV_EVENT_SUNRISE,
            noon_jd,
            &cfg,
            &mut jd_result,
        )
    };
    assert_eq!(status, DhruvStatus::Ok);
    assert_eq!(jd_result.result_type, DHRUV_RISESET_EVENT);

    // UTC version: noon on 2024-03-20 in New Delhi ≈ ~06:30 UTC
    let utc_noon = DhruvUtcTime {
        year: 2024,
        month: 3,
        day: 20,
        hour: 7,
        minute: 21,
        second: 0.0,
    };
    let mut utc_result = DhruvRiseSetResultUtc {
        result_type: -1,
        event_code: -1,
        utc: DhruvUtcTime {
            year: 0,
            month: 0,
            day: 0,
            hour: 0,
            minute: 0,
            second: 0.0,
        },
    };
    let status = unsafe {
        dhruv_compute_rise_set_utc(
            engine_ptr,
            lsk_ptr,
            eop_ptr,
            &loc,
            DHRUV_EVENT_SUNRISE,
            &utc_noon,
            &cfg,
            &mut utc_result,
        )
    };
    assert_eq!(status, DhruvStatus::Ok);
    assert_eq!(utc_result.result_type, DHRUV_RISESET_EVENT);

    // Compare: the result JD TDB from JD version should match UTC result converted back
    assert_utc_matches_jd(lsk_ptr, &utc_result.utc, jd_result.jd_tdb, "sunrise time");

    unsafe { dhruv_eop_free(eop_ptr) };
    unsafe { dhruv_lsk_free(lsk_ptr) };
    unsafe { dhruv_engine_free(engine_ptr) };
}

#[test]
fn ffi_time_upagraha_jd_utc_matches_manual_night_window_after_noon_utc() {
    if !all_kernels_available() {
        eprintln!("Skipping: not all kernel files available");
        return;
    }

    let engine_ptr = make_engine().unwrap();
    let lsk_path = lsk_path_cstr().unwrap();
    let eop_path = eop_path_cstr().unwrap();

    let mut lsk_ptr: *mut DhruvLskHandle = ptr::null_mut();
    let status =
        unsafe { dhruv_lsk_load(lsk_path.as_ptr() as *const std::ffi::c_char, &mut lsk_ptr) };
    assert_eq!(status, DhruvStatus::Ok);

    let mut eop_ptr: *mut DhruvEopHandle = ptr::null_mut();
    let status =
        unsafe { dhruv_eop_load(eop_path.as_ptr() as *const std::ffi::c_char, &mut eop_ptr) };
    assert_eq!(status, DhruvStatus::Ok);

    let loc = DhruvGeoLocation {
        latitude_deg: 12.9716,
        longitude_deg: 77.5946,
        altitude_m: 0.0,
    };
    let utc = DhruvUtcTime {
        year: 2026,
        month: 3,
        day: 17,
        hour: 15,
        minute: 6,
        second: 19.0,
    };
    let cfg = dhruv_riseset_config_default();

    let mut sunrise_jd = 0.0;
    let mut next_sunrise_jd = 0.0;
    let status = unsafe {
        dhruv_vedic_day_sunrises(
            engine_ptr,
            eop_ptr,
            &utc,
            &loc,
            &cfg,
            &mut sunrise_jd,
            &mut next_sunrise_jd,
        )
    };
    assert_eq!(status, DhruvStatus::Ok);

    let jd_0h = calendar_to_jd(utc.year, utc.month, utc.day as f64);
    let noon_jd = dhruv_approximate_local_noon_jd(jd_0h, loc.longitude_deg);
    let mut sunset = DhruvRiseSetResult {
        result_type: -1,
        event_code: -1,
        jd_tdb: 0.0,
    };
    let status = unsafe {
        dhruv_compute_rise_set(
            engine_ptr,
            lsk_ptr,
            eop_ptr,
            &loc,
            DHRUV_EVENT_SUNSET,
            noon_jd,
            &cfg,
            &mut sunset,
        )
    };
    assert_eq!(status, DhruvStatus::Ok);
    assert_eq!(sunset.result_type, DHRUV_RISESET_EVENT);

    let query_jd_tdb = utc_to_tdb_jd_default(lsk_ptr, utc);
    assert!(
        query_jd_tdb >= sunset.jd_tdb,
        "query should be in the night interval for this regression case"
    );

    for index in 0..6u32 {
        let mut manual_jd = 0.0;
        let status = unsafe {
            dhruv_time_upagraha_jd(
                index,
                2,
                0,
                sunrise_jd,
                sunset.jd_tdb,
                next_sunrise_jd,
                &mut manual_jd,
            )
        };
        assert_eq!(status, DhruvStatus::Ok);
        assert!(
            manual_jd >= sunset.jd_tdb && manual_jd <= next_sunrise_jd,
            "manual JD should lie in the night interval"
        );

        let mut utc_jd = 0.0;
        let status = unsafe {
            dhruv_time_upagraha_jd_utc(engine_ptr, eop_ptr, &utc, &loc, &cfg, index, &mut utc_jd)
        };
        assert_eq!(status, DhruvStatus::Ok);
        assert!(
            (utc_jd - manual_jd).abs() < 1e-8,
            "index {index}: utc_jd={utc_jd}, manual_jd={manual_jd}"
        );
    }

    unsafe {
        dhruv_eop_free(eop_ptr);
        dhruv_lsk_free(lsk_ptr);
        dhruv_engine_free(engine_ptr);
    }
}

#[test]
fn ffi_compute_rise_set_utc_matches_manual_same_day_sunset_after_noon_utc() {
    if !all_kernels_available() {
        eprintln!("Skipping: not all kernel files available");
        return;
    }

    let engine_ptr = make_engine().unwrap();
    let lsk_path = lsk_path_cstr().unwrap();
    let eop_path = eop_path_cstr().unwrap();

    let mut lsk_ptr: *mut DhruvLskHandle = ptr::null_mut();
    let status =
        unsafe { dhruv_lsk_load(lsk_path.as_ptr() as *const std::ffi::c_char, &mut lsk_ptr) };
    assert_eq!(status, DhruvStatus::Ok);

    let mut eop_ptr: *mut DhruvEopHandle = ptr::null_mut();
    let status =
        unsafe { dhruv_eop_load(eop_path.as_ptr() as *const std::ffi::c_char, &mut eop_ptr) };
    assert_eq!(status, DhruvStatus::Ok);

    let loc = DhruvGeoLocation {
        latitude_deg: 12.9716,
        longitude_deg: 77.5946,
        altitude_m: 0.0,
    };
    let utc = DhruvUtcTime {
        year: 2026,
        month: 3,
        day: 17,
        hour: 15,
        minute: 6,
        second: 19.0,
    };
    let cfg = dhruv_riseset_config_default();

    let jd_0h = calendar_to_jd(utc.year, utc.month, utc.day as f64);
    let noon_jd = dhruv_approximate_local_noon_jd(jd_0h, loc.longitude_deg);
    let mut manual = DhruvRiseSetResult {
        result_type: -1,
        event_code: -1,
        jd_tdb: 0.0,
    };
    let status = unsafe {
        dhruv_compute_rise_set(
            engine_ptr,
            lsk_ptr,
            eop_ptr,
            &loc,
            DHRUV_EVENT_SUNSET,
            noon_jd,
            &cfg,
            &mut manual,
        )
    };
    assert_eq!(status, DhruvStatus::Ok);
    assert_eq!(manual.result_type, DHRUV_RISESET_EVENT);

    let mut utc_result = DhruvRiseSetResultUtc {
        result_type: -1,
        event_code: -1,
        utc: DhruvUtcTime {
            year: 0,
            month: 0,
            day: 0,
            hour: 0,
            minute: 0,
            second: 0.0,
        },
    };
    let status = unsafe {
        dhruv_compute_rise_set_utc(
            engine_ptr,
            lsk_ptr,
            eop_ptr,
            &loc,
            DHRUV_EVENT_SUNSET,
            &utc,
            &cfg,
            &mut utc_result,
        )
    };
    assert_eq!(status, DhruvStatus::Ok);
    assert_eq!(utc_result.result_type, DHRUV_RISESET_EVENT);

    let mut manual_utc = DhruvUtcTime {
        year: 0,
        month: 0,
        day: 0,
        hour: 0,
        minute: 0,
        second: 0.0,
    };
    let status = unsafe { dhruv_riseset_result_to_utc(lsk_ptr, &manual, &mut manual_utc) };
    assert_eq!(status, DhruvStatus::Ok);

    let actual_seconds = utc_result.utc.hour as i64 * 3600
        + utc_result.utc.minute as i64 * 60
        + utc_result.utc.second.round() as i64;
    let expected_seconds = manual_utc.hour as i64 * 3600
        + manual_utc.minute as i64 * 60
        + manual_utc.second.round() as i64;

    assert_eq!(utc_result.utc.year, manual_utc.year);
    assert_eq!(utc_result.utc.month, manual_utc.month);
    assert_eq!(utc_result.utc.day, manual_utc.day);
    assert!(
        (actual_seconds - expected_seconds).abs() <= 1,
        "utc_result={:?}, manual_utc={:?}",
        utc_result.utc,
        manual_utc
    );

    unsafe {
        dhruv_eop_free(eop_ptr);
        dhruv_lsk_free(lsk_ptr);
        dhruv_engine_free(engine_ptr);
    }
}

#[test]
fn ffi_utc_nutation_roundtrip() {
    let lsk_path = match lsk_path_cstr() {
        Some(p) => p,
        None => return,
    };
    let mut lsk_ptr: *mut DhruvLskHandle = ptr::null_mut();
    let status =
        unsafe { dhruv_lsk_load(lsk_path.as_ptr() as *const std::ffi::c_char, &mut lsk_ptr) };
    assert_eq!(status, DhruvStatus::Ok);

    // JD version
    let jd = 2_460_310.5;
    let mut dpsi_jd: f64 = 0.0;
    let mut deps_jd: f64 = 0.0;
    let status = unsafe { dhruv_nutation_iau2000b(jd, &mut dpsi_jd, &mut deps_jd) };
    assert_eq!(status, DhruvStatus::Ok);

    // UTC version
    let utc = DhruvUtcTime {
        year: 2024,
        month: 1,
        day: 1,
        hour: 0,
        minute: 0,
        second: 0.0,
    };
    let mut dpsi_utc: f64 = 0.0;
    let mut deps_utc: f64 = 0.0;
    let status =
        unsafe { dhruv_nutation_iau2000b_utc(lsk_ptr, &utc, &mut dpsi_utc, &mut deps_utc) };
    assert_eq!(status, DhruvStatus::Ok);

    // Should be very close (nutation changes slowly)
    assert!(
        (dpsi_utc - dpsi_jd).abs() < 0.01,
        "dpsi: utc={dpsi_utc}, jd={dpsi_jd}"
    );
    assert!(
        (deps_utc - deps_jd).abs() < 0.01,
        "deps: utc={deps_utc}, jd={deps_jd}"
    );

    unsafe { dhruv_lsk_free(lsk_ptr) };
}

// ---------------------------------------------------------------------------
// Dasha / FullKundali integration tests
// ---------------------------------------------------------------------------

#[test]
fn ffi_dasha_selection_config_default_values() {
    let cfg = dhruv_dasha_selection_config_default();
    assert_eq!(cfg.count, 0);
    assert_eq!(cfg.systems, [0xFF; DHRUV_MAX_DASHA_SYSTEMS]);
    assert_eq!(cfg.max_levels, [0xFF; DHRUV_MAX_DASHA_SYSTEMS]);
    assert_eq!(cfg.max_level, 2);
    assert_eq!(cfg.level_methods, [0xFF; 5]);
    assert_eq!(cfg.yogini_scheme, 0);
    assert_eq!(cfg.use_abhijit, 1);
    assert_eq!(cfg.snapshot_time.time_kind, DHRUV_DASHA_TIME_NONE);
}

#[test]
fn ffi_full_kundali_result_free_null_is_noop() {
    // Must not crash.
    unsafe { dhruv_full_kundali_result_free(ptr::null_mut()) };
}

/// Helper: set up engine + EOP for FullKundali tests.
fn make_kundali_fixtures() -> Option<(*mut DhruvEngineHandle, *mut DhruvEopHandle)> {
    if !all_kernels_available() {
        return None;
    }
    let engine_ptr = make_engine()?;
    let eop_cstr = eop_path_cstr()?;
    let mut eop_ptr: *mut DhruvEopHandle = ptr::null_mut();
    let status =
        unsafe { dhruv_eop_load(eop_cstr.as_ptr() as *const std::ffi::c_char, &mut eop_ptr) };
    assert_eq!(status, DhruvStatus::Ok);
    Some((engine_ptr, eop_ptr))
}

fn kundali_test_params() -> (
    DhruvUtcTime,
    DhruvGeoLocation,
    DhruvBhavaConfig,
    DhruvRiseSetConfig,
) {
    let utc = DhruvUtcTime {
        year: 1990,
        month: 1,
        day: 15,
        hour: 6,
        minute: 30,
        second: 0.0,
    };
    let loc = DhruvGeoLocation {
        latitude_deg: 28.6139,
        longitude_deg: 77.2090,
        altitude_m: 0.0,
    };
    let bhava = dhruv_bhava_config_default();
    let rs = dhruv_riseset_config_default();
    (utc, loc, bhava, rs)
}

fn d2_cancer_leo_amsha_selection() -> DhruvAmshaSelectionConfig {
    let mut selection = DhruvAmshaSelectionConfig {
        count: 1,
        codes: [0; 40],
        variations: [0; 40],
    };
    selection.codes[0] = 2;
    selection.variations[0] = 1;
    selection
}

fn d9_default_amsha_selection() -> DhruvAmshaSelectionConfig {
    let mut selection = DhruvAmshaSelectionConfig {
        count: 1,
        codes: [0; 40],
        variations: [0; 40],
    };
    selection.codes[0] = 9;
    selection
}

#[test]
fn ffi_amsha_variation_catalogs_are_amsha_scoped() {
    let mut d2 = DhruvAmshaVariationList {
        amsha_code: 0,
        default_variation_code: 0,
        count: 0,
        variations: [DhruvAmshaVariationInfo {
            amsha_code: 0,
            variation_code: 0,
            name: [0; DHRUV_AMSHA_VARIATION_NAME_CAPACITY as usize],
            label: [0; DHRUV_AMSHA_VARIATION_LABEL_CAPACITY as usize],
            is_default: 0,
            description: [0; DHRUV_AMSHA_VARIATION_DESCRIPTION_CAPACITY as usize],
        }; DHRUV_MAX_AMSHA_VARIATIONS as usize],
    };
    let status = unsafe { dhruv_amsha_variations(2, &mut d2) };
    assert_eq!(status, DhruvStatus::Ok);
    assert_eq!(d2.amsha_code, 2);
    assert_eq!(d2.default_variation_code, 0);
    assert_eq!(d2.count, 2);
    assert_eq!(d2.variations[1].variation_code, 1);

    let mut many = DhruvAmshaVariationCatalogs {
        count: 0,
        lists: [DhruvAmshaVariationList {
            amsha_code: 0,
            default_variation_code: 0,
            count: 0,
            variations: [DhruvAmshaVariationInfo {
                amsha_code: 0,
                variation_code: 0,
                name: [0; DHRUV_AMSHA_VARIATION_NAME_CAPACITY as usize],
                label: [0; DHRUV_AMSHA_VARIATION_LABEL_CAPACITY as usize],
                is_default: 0,
                description: [0; DHRUV_AMSHA_VARIATION_DESCRIPTION_CAPACITY as usize],
            }; DHRUV_MAX_AMSHA_VARIATIONS as usize],
        }; DHRUV_MAX_AMSHA_REQUESTS as usize],
    };
    let amsha_codes = [2u16, 9u16];
    let status = unsafe { dhruv_amsha_variations_many(amsha_codes.as_ptr(), 2, &mut many) };
    assert_eq!(status, DhruvStatus::Ok);
    assert_eq!(many.count, 2);
    assert_eq!(many.lists[1].amsha_code, 9);
    assert_eq!(many.lists[1].count, 1);
    assert_eq!(many.lists[1].variations[0].variation_code, 0);
}

#[test]
fn ffi_bala_entrypoints_accept_amsha_selection() {
    let (engine_ptr, eop_ptr) = match make_kundali_fixtures() {
        Some(f) => f,
        None => return,
    };
    let (utc, loc, bhava, rs) = kundali_test_params();
    let d2_selection = d2_cancer_leo_amsha_selection();
    let d9_selection = d9_default_amsha_selection();

    let mut shadbala_default = std::mem::MaybeUninit::<DhruvShadbalaResult>::uninit();
    let status = unsafe {
        dhruv_shadbala_for_date(
            engine_ptr as *const _,
            eop_ptr as *const _,
            &utc,
            &loc,
            &bhava,
            &rs,
            0,
            1,
            ptr::null(),
            shadbala_default.as_mut_ptr(),
        )
    };
    assert_eq!(status, DhruvStatus::Ok);
    let shadbala_default = unsafe { shadbala_default.assume_init() };

    let mut shadbala_override = std::mem::MaybeUninit::<DhruvShadbalaResult>::uninit();
    let status = unsafe {
        dhruv_shadbala_for_date(
            engine_ptr as *const _,
            eop_ptr as *const _,
            &utc,
            &loc,
            &bhava,
            &rs,
            0,
            1,
            &d2_selection,
            shadbala_override.as_mut_ptr(),
        )
    };
    assert_eq!(status, DhruvStatus::Ok);
    let shadbala_override = unsafe { shadbala_override.assume_init() };
    assert!(
        shadbala_default
            .entries
            .iter()
            .zip(shadbala_override.entries.iter())
            .any(|(lhs, rhs)| (lhs.total_shashtiamsas - rhs.total_shashtiamsas).abs() > 1e-6),
        "D2 variation should change shadbala output"
    );

    let mut vimsopaka_default = std::mem::MaybeUninit::<DhruvVimsopakaResult>::uninit();
    let status = unsafe {
        dhruv_vimsopaka_for_date(
            engine_ptr as *const _,
            eop_ptr as *const _,
            &utc,
            &loc,
            0,
            1,
            0,
            ptr::null(),
            vimsopaka_default.as_mut_ptr(),
        )
    };
    assert_eq!(status, DhruvStatus::Ok);
    let vimsopaka_default = unsafe { vimsopaka_default.assume_init() };

    let mut vimsopaka_override = std::mem::MaybeUninit::<DhruvVimsopakaResult>::uninit();
    let status = unsafe {
        dhruv_vimsopaka_for_date(
            engine_ptr as *const _,
            eop_ptr as *const _,
            &utc,
            &loc,
            0,
            1,
            0,
            &d2_selection,
            vimsopaka_override.as_mut_ptr(),
        )
    };
    assert_eq!(status, DhruvStatus::Ok);
    let vimsopaka_override = unsafe { vimsopaka_override.assume_init() };
    assert!(
        vimsopaka_default
            .entries
            .iter()
            .zip(vimsopaka_override.entries.iter())
            .any(|(lhs, rhs)| {
                (lhs.shadvarga - rhs.shadvarga).abs() > 1e-6
                    || (lhs.saptavarga - rhs.saptavarga).abs() > 1e-6
                    || (lhs.dashavarga - rhs.dashavarga).abs() > 1e-6
                    || (lhs.shodasavarga - rhs.shodasavarga).abs() > 1e-6
            }),
        "D2 variation should change vimsopaka output"
    );

    let mut balas_default = std::mem::MaybeUninit::<DhruvBalaBundleResult>::uninit();
    let status = unsafe {
        dhruv_balas_for_date(
            engine_ptr as *const _,
            eop_ptr as *const _,
            &utc,
            &loc,
            &bhava,
            &rs,
            0,
            1,
            0,
            ptr::null(),
            balas_default.as_mut_ptr(),
        )
    };
    assert_eq!(status, DhruvStatus::Ok);
    let balas_default = unsafe { balas_default.assume_init() };

    let mut balas_override = std::mem::MaybeUninit::<DhruvBalaBundleResult>::uninit();
    let status = unsafe {
        dhruv_balas_for_date(
            engine_ptr as *const _,
            eop_ptr as *const _,
            &utc,
            &loc,
            &bhava,
            &rs,
            0,
            1,
            0,
            &d2_selection,
            balas_override.as_mut_ptr(),
        )
    };
    assert_eq!(status, DhruvStatus::Ok);
    let balas_override = unsafe { balas_override.assume_init() };
    assert!(
        balas_default
            .shadbala
            .entries
            .iter()
            .zip(balas_override.shadbala.entries.iter())
            .any(|(lhs, rhs)| (lhs.total_shashtiamsas - rhs.total_shashtiamsas).abs() > 1e-6),
        "D2 variation should change bundled shadbala output"
    );

    let mut avastha = std::mem::MaybeUninit::<DhruvAllGrahaAvasthas>::uninit();
    let status = unsafe {
        dhruv_avastha_for_date(
            engine_ptr as *const _,
            eop_ptr as *const _,
            &utc,
            &loc,
            &bhava,
            &rs,
            0,
            1,
            0,
            &d9_selection,
            avastha.as_mut_ptr(),
        )
    };
    assert_eq!(status, DhruvStatus::Ok);
    let avastha = unsafe { avastha.assume_init() };
    for entry in &avastha.entries {
        assert!(entry.baladi <= 4);
        assert!(entry.jagradadi <= 2);
        assert!(entry.deeptadi <= 8);
        assert!(entry.deeptadi_count >= 1);
        assert!(entry.deeptadi_count <= 9);
        assert_eq!(entry.deeptadi_states[0], entry.deeptadi);
        assert_ne!(entry.deeptadi_mask, 0);
        assert!(entry.lajjitadi == u8::MAX || entry.lajjitadi <= 5);
        assert!(entry.lajjitadi_count <= 6);
        assert_eq!(entry.lajjitadi_valid != 0, entry.lajjitadi_count > 0);
        if entry.lajjitadi_valid != 0 {
            assert_eq!(entry.lajjitadi_states[0], entry.lajjitadi);
            assert_ne!(entry.lajjitadi_mask, 0);
        }
        assert!(entry.sayanadi.avastha <= 11);
    }

    unsafe { dhruv_engine_free(engine_ptr) };
    unsafe { dhruv_eop_free(eop_ptr) };
}

#[test]
fn ffi_full_kundali_amsha_selection_returns_resolved_union() {
    let (engine_ptr, eop_ptr) = match make_kundali_fixtures() {
        Some(f) => f,
        None => return,
    };
    let (utc, loc, bhava, rs) = kundali_test_params();

    let mut fk_config = dhruv_full_kundali_config_default();
    fk_config.include_amshas = 1;
    fk_config.include_shadbala = 1;
    fk_config.include_vimsopaka = 1;
    fk_config.amsha_selection = d2_cancer_leo_amsha_selection();

    let mut result = std::mem::MaybeUninit::<DhruvFullKundaliResult>::uninit();
    let status = unsafe {
        dhruv_full_kundali_for_date(
            engine_ptr as *const _,
            eop_ptr as *const _,
            &utc,
            &loc,
            &bhava,
            &rs,
            0,
            1,
            &fk_config,
            result.as_mut_ptr(),
        )
    };
    assert_eq!(status, DhruvStatus::Ok);

    let result = unsafe { result.assume_init() };
    assert_eq!(result.amshas_valid, 1);
    assert_eq!(result.amshas_count, 16);
    assert_eq!(result.amshas[0].amsha_code, 2);
    assert_eq!(result.amshas[0].variation_code, 1);
    assert!(
        result.amshas[..result.amshas_count as usize]
            .iter()
            .any(|chart| chart.amsha_code == 60)
    );

    let mut result = result;
    unsafe { dhruv_full_kundali_result_free(&mut result) };
    unsafe { dhruv_engine_free(engine_ptr) };
    unsafe { dhruv_eop_free(eop_ptr) };
}

#[test]
fn ffi_full_kundali_result_free_double_free_same_pointer() {
    let (engine_ptr, eop_ptr) = match make_kundali_fixtures() {
        Some(f) => f,
        None => return,
    };
    let (utc, loc, bhava, rs) = kundali_test_params();

    // Config with Vimshottari dasha.
    let mut dasha_cfg = dhruv_dasha_selection_config_default();
    dasha_cfg.count = 1;
    dasha_cfg.systems[0] = 0; // Vimshottari

    let fk_config = DhruvFullKundaliConfig {
        include_bhava_cusps: 0,
        include_graha_positions: 0,
        include_bindus: 0,
        include_drishti: 0,
        include_ashtakavarga: 0,
        include_upagrahas: 0,
        include_sphutas: 0,
        include_special_lagnas: 0,
        include_amshas: 0,
        include_shadbala: 0,
        include_bhavabala: 0,
        include_vimsopaka: 0,
        include_avastha: 0,
        include_charakaraka: 0,
        charakaraka_scheme: 0,
        include_panchang: 0,
        include_calendar: 0,
        node_dignity_policy: 0,
        upagraha_config: dhruv_time_upagraha_config_default(),
        graha_positions_config: DhruvGrahaPositionsConfig {
            include_nakshatra: 0,
            include_lagna: 0,
            include_outer_planets: 0,
            include_bhava: 0,
        },
        bindus_config: DhruvBindusConfig {
            include_nakshatra: 0,
            include_bhava: 0,
            upagraha_config: dhruv_time_upagraha_config_default(),
        },
        drishti_config: DhruvDrishtiConfig {
            include_bhava: 0,
            include_lagna: 0,
            include_bindus: 0,
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
        include_dasha: 1,
        dasha_config: dasha_cfg,
    };

    let mut result = std::mem::MaybeUninit::<DhruvFullKundaliResult>::uninit();
    let status = unsafe {
        dhruv_full_kundali_for_date(
            engine_ptr as *const _,
            eop_ptr as *const _,
            &utc,
            &loc,
            &bhava,
            &rs,
            0,
            1,
            &fk_config,
            result.as_mut_ptr(),
        )
    };
    assert_eq!(status, DhruvStatus::Ok);
    let result_ptr = result.as_mut_ptr();

    // Verify dasha was produced.
    let r = unsafe { &*result_ptr };
    assert_eq!(r.dasha_count, 1);
    assert_eq!(r.dasha_systems[0], 0); // Vimshottari
    assert!(!r.dasha_handles[0].is_null());

    // First free.
    unsafe { dhruv_full_kundali_result_free(result_ptr) };
    let r = unsafe { &*result_ptr };
    assert_eq!(r.dasha_count, 0);
    assert!(r.dasha_handles[0].is_null());

    // Second free — must not crash (handles already nulled).
    unsafe { dhruv_full_kundali_result_free(result_ptr) };

    unsafe { dhruv_engine_free(engine_ptr) };
    unsafe { dhruv_eop_free(eop_ptr) };
}

#[test]
fn ffi_full_kundali_error_path_free_safety() {
    // Real objects for all pointers, but invalid ayanamsha code to trigger error.
    let (engine_ptr, eop_ptr) = match make_kundali_fixtures() {
        Some(f) => f,
        None => return,
    };
    let (utc, loc, bhava, rs) = kundali_test_params();

    let fk_config = DhruvFullKundaliConfig {
        include_bhava_cusps: 0,
        include_graha_positions: 1,
        include_bindus: 0,
        include_drishti: 0,
        include_ashtakavarga: 0,
        include_upagrahas: 0,
        include_sphutas: 0,
        include_special_lagnas: 0,
        include_amshas: 0,
        include_shadbala: 0,
        include_bhavabala: 0,
        include_vimsopaka: 0,
        include_avastha: 0,
        include_charakaraka: 0,
        charakaraka_scheme: 0,
        include_panchang: 0,
        include_calendar: 0,
        node_dignity_policy: 0,
        upagraha_config: dhruv_time_upagraha_config_default(),
        graha_positions_config: DhruvGrahaPositionsConfig {
            include_nakshatra: 0,
            include_lagna: 0,
            include_outer_planets: 0,
            include_bhava: 0,
        },
        bindus_config: DhruvBindusConfig {
            include_nakshatra: 0,
            include_bhava: 0,
            upagraha_config: dhruv_time_upagraha_config_default(),
        },
        drishti_config: DhruvDrishtiConfig {
            include_bhava: 0,
            include_lagna: 0,
            include_bindus: 0,
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
        include_dasha: 1,
        dasha_config: dhruv_dasha_selection_config_default(),
    };

    let mut result = std::mem::MaybeUninit::<DhruvFullKundaliResult>::uninit();
    // Invalid ayanamsha_system=999 triggers InvalidQuery error.
    let status = unsafe {
        dhruv_full_kundali_for_date(
            engine_ptr as *const _,
            eop_ptr as *const _,
            &utc,
            &loc,
            &bhava,
            &rs,
            999, // invalid
            1,
            &fk_config,
            result.as_mut_ptr(),
        )
    };
    assert_ne!(status, DhruvStatus::Ok);

    // result_free is safe because write_bytes zeroed at entry before the error return.
    unsafe { dhruv_full_kundali_result_free(result.as_mut_ptr()) };

    unsafe { dhruv_engine_free(engine_ptr) };
    unsafe { dhruv_eop_free(eop_ptr) };
}

#[test]
fn ffi_full_kundali_dasha_overflow_rejection() {
    let (engine_ptr, eop_ptr) = match make_kundali_fixtures() {
        Some(f) => f,
        None => return,
    };
    let (utc, loc, bhava, rs) = kundali_test_params();

    let mut dasha_cfg = dhruv_dasha_selection_config_default();
    dasha_cfg.count = 9; // exceeds MAX_DASHA_SYSTEMS

    let fk_config = DhruvFullKundaliConfig {
        include_bhava_cusps: 0,
        include_graha_positions: 0,
        include_bindus: 0,
        include_drishti: 0,
        include_ashtakavarga: 0,
        include_upagrahas: 0,
        include_sphutas: 0,
        include_special_lagnas: 0,
        include_amshas: 0,
        include_shadbala: 0,
        include_bhavabala: 0,
        include_vimsopaka: 0,
        include_avastha: 0,
        include_charakaraka: 0,
        charakaraka_scheme: 0,
        include_panchang: 0,
        include_calendar: 0,
        node_dignity_policy: 0,
        upagraha_config: dhruv_time_upagraha_config_default(),
        graha_positions_config: DhruvGrahaPositionsConfig {
            include_nakshatra: 0,
            include_lagna: 0,
            include_outer_planets: 0,
            include_bhava: 0,
        },
        bindus_config: DhruvBindusConfig {
            include_nakshatra: 0,
            include_bhava: 0,
            upagraha_config: dhruv_time_upagraha_config_default(),
        },
        drishti_config: DhruvDrishtiConfig {
            include_bhava: 0,
            include_lagna: 0,
            include_bindus: 0,
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
        include_dasha: 1,
        dasha_config: dasha_cfg,
    };

    let mut result = std::mem::MaybeUninit::<DhruvFullKundaliResult>::uninit();
    let status = unsafe {
        dhruv_full_kundali_for_date(
            engine_ptr as *const _,
            eop_ptr as *const _,
            &utc,
            &loc,
            &bhava,
            &rs,
            0,
            1,
            &fk_config,
            result.as_mut_ptr(),
        )
    };
    // Should fail validation (count > MAX_DASHA_SYSTEMS).
    assert_ne!(status, DhruvStatus::Ok);

    unsafe { dhruv_full_kundali_result_free(result.as_mut_ptr()) };
    unsafe { dhruv_engine_free(engine_ptr) };
    unsafe { dhruv_eop_free(eop_ptr) };
}

#[test]
fn ffi_full_kundali_dasha_partial_success_contract() {
    // Compute with 2 systems + snapshot. Verify snapshot system codes match hierarchy system codes.
    let (engine_ptr, eop_ptr) = match make_kundali_fixtures() {
        Some(f) => f,
        None => return,
    };
    let (utc, loc, bhava, rs) = kundali_test_params();

    let mut dasha_cfg = dhruv_dasha_selection_config_default();
    dasha_cfg.count = 2;
    dasha_cfg.systems[0] = 0; // Vimshottari
    dasha_cfg.systems[1] = 11; // Chara
    dasha_cfg.snapshot_time.time_kind = DHRUV_DASHA_TIME_UTC;
    dasha_cfg.snapshot_time.utc = DhruvUtcTime {
        year: 2020,
        month: 1,
        day: 1,
        hour: 12,
        minute: 0,
        second: 0.0,
    };

    let fk_config = DhruvFullKundaliConfig {
        include_bhava_cusps: 0,
        include_graha_positions: 0,
        include_bindus: 0,
        include_drishti: 0,
        include_ashtakavarga: 0,
        include_upagrahas: 0,
        include_sphutas: 0,
        include_special_lagnas: 0,
        include_amshas: 0,
        include_shadbala: 0,
        include_bhavabala: 0,
        include_vimsopaka: 0,
        include_avastha: 0,
        include_charakaraka: 0,
        charakaraka_scheme: 0,
        include_panchang: 0,
        include_calendar: 0,
        node_dignity_policy: 0,
        upagraha_config: dhruv_time_upagraha_config_default(),
        graha_positions_config: DhruvGrahaPositionsConfig {
            include_nakshatra: 0,
            include_lagna: 0,
            include_outer_planets: 0,
            include_bhava: 0,
        },
        bindus_config: DhruvBindusConfig {
            include_nakshatra: 0,
            include_bhava: 0,
            upagraha_config: dhruv_time_upagraha_config_default(),
        },
        drishti_config: DhruvDrishtiConfig {
            include_bhava: 0,
            include_lagna: 0,
            include_bindus: 0,
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
        include_dasha: 1,
        dasha_config: dasha_cfg,
    };

    let mut result = std::mem::MaybeUninit::<DhruvFullKundaliResult>::uninit();
    let status = unsafe {
        dhruv_full_kundali_for_date(
            engine_ptr as *const _,
            eop_ptr as *const _,
            &utc,
            &loc,
            &bhava,
            &rs,
            0,
            1,
            &fk_config,
            result.as_mut_ptr(),
        )
    };
    assert_eq!(status, DhruvStatus::Ok);

    let r = unsafe { &*result.as_ptr() };

    // Verify hierarchies were produced.
    assert!(r.dasha_count >= 1, "dasha_count={}", r.dasha_count);
    assert!(
        r.dasha_snapshot_count <= r.dasha_count,
        "snap={} > hier={}",
        r.dasha_snapshot_count,
        r.dasha_count
    );

    // Verify each snapshot's system code matches one of the hierarchy system codes.
    let hier_systems: Vec<u8> = r.dasha_systems[..r.dasha_count as usize].to_vec();
    for i in 0..r.dasha_snapshot_count as usize {
        let snap_sys = r.dasha_snapshots[i].system;
        assert!(
            hier_systems.contains(&snap_sys),
            "snapshot system {} not in hierarchy systems {:?}",
            snap_sys,
            hier_systems
        );
        let count = r.dasha_snapshots[i].count as usize;
        for j in 0..count {
            assert!(!r.dasha_snapshots[i].periods[j].entity_name.is_null());
        }
    }

    unsafe { dhruv_full_kundali_result_free(result.as_mut_ptr()) };
    unsafe { dhruv_engine_free(engine_ptr) };
    unsafe { dhruv_eop_free(eop_ptr) };
}

#[test]
fn ffi_full_kundali_panchang_only() {
    let (engine_ptr, eop_ptr) = match make_kundali_fixtures() {
        Some(f) => f,
        None => return,
    };
    let (utc, loc, bhava, rs) = kundali_test_params();

    let fk_config = DhruvFullKundaliConfig {
        include_bhava_cusps: 0,
        include_graha_positions: 0,
        include_bindus: 0,
        include_drishti: 0,
        include_ashtakavarga: 0,
        include_upagrahas: 0,
        include_sphutas: 0,
        include_special_lagnas: 0,
        include_amshas: 0,
        include_shadbala: 0,
        include_bhavabala: 0,
        include_vimsopaka: 0,
        include_avastha: 0,
        include_charakaraka: 0,
        charakaraka_scheme: 0,
        include_panchang: 1,
        include_calendar: 0,
        node_dignity_policy: 0,
        upagraha_config: dhruv_time_upagraha_config_default(),
        graha_positions_config: DhruvGrahaPositionsConfig {
            include_nakshatra: 0,
            include_lagna: 0,
            include_outer_planets: 0,
            include_bhava: 0,
        },
        bindus_config: DhruvBindusConfig {
            include_nakshatra: 0,
            include_bhava: 0,
            upagraha_config: dhruv_time_upagraha_config_default(),
        },
        drishti_config: DhruvDrishtiConfig {
            include_bhava: 0,
            include_lagna: 0,
            include_bindus: 0,
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
        include_dasha: 0,
        dasha_config: dhruv_dasha_selection_config_default(),
    };

    let mut result = std::mem::MaybeUninit::<DhruvFullKundaliResult>::uninit();
    let status = unsafe {
        dhruv_full_kundali_for_date(
            engine_ptr as *const _,
            eop_ptr as *const _,
            &utc,
            &loc,
            &bhava,
            &rs,
            0,
            1,
            &fk_config,
            result.as_mut_ptr(),
        )
    };
    assert_eq!(status, DhruvStatus::Ok);
    let r = unsafe { &*result.as_ptr() };

    assert_eq!(r.panchang_valid, 1);
    assert!((0..=29).contains(&r.panchang.tithi.tithi_index));
    assert!((0..=6).contains(&r.panchang.vaar.vaar_index));
    assert_eq!(r.panchang.calendar_valid, 0);
    assert_eq!(r.graha_positions_valid, 0);

    unsafe { dhruv_full_kundali_result_free(result.as_mut_ptr()) };
    unsafe { dhruv_engine_free(engine_ptr) };
    unsafe { dhruv_eop_free(eop_ptr) };
}

#[test]
fn ffi_full_kundali_panchang_disabled() {
    let (engine_ptr, eop_ptr) = match make_kundali_fixtures() {
        Some(f) => f,
        None => return,
    };
    let (utc, loc, bhava, rs) = kundali_test_params();

    let fk_config = DhruvFullKundaliConfig {
        include_bhava_cusps: 0,
        include_graha_positions: 0,
        include_bindus: 0,
        include_drishti: 0,
        include_ashtakavarga: 0,
        include_upagrahas: 0,
        include_sphutas: 0,
        include_special_lagnas: 0,
        include_amshas: 0,
        include_shadbala: 0,
        include_bhavabala: 0,
        include_vimsopaka: 0,
        include_avastha: 0,
        include_charakaraka: 0,
        charakaraka_scheme: 0,
        include_panchang: 0,
        include_calendar: 0,
        node_dignity_policy: 0,
        upagraha_config: dhruv_time_upagraha_config_default(),
        graha_positions_config: DhruvGrahaPositionsConfig {
            include_nakshatra: 0,
            include_lagna: 0,
            include_outer_planets: 0,
            include_bhava: 0,
        },
        bindus_config: DhruvBindusConfig {
            include_nakshatra: 0,
            include_bhava: 0,
            upagraha_config: dhruv_time_upagraha_config_default(),
        },
        drishti_config: DhruvDrishtiConfig {
            include_bhava: 0,
            include_lagna: 0,
            include_bindus: 0,
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
        include_dasha: 0,
        dasha_config: dhruv_dasha_selection_config_default(),
    };

    let mut result = std::mem::MaybeUninit::<DhruvFullKundaliResult>::uninit();
    let status = unsafe {
        dhruv_full_kundali_for_date(
            engine_ptr as *const _,
            eop_ptr as *const _,
            &utc,
            &loc,
            &bhava,
            &rs,
            0,
            1,
            &fk_config,
            result.as_mut_ptr(),
        )
    };
    assert_eq!(status, DhruvStatus::Ok);
    let r = unsafe { &*result.as_ptr() };

    assert_eq!(r.panchang_valid, 0);

    unsafe { dhruv_full_kundali_result_free(result.as_mut_ptr()) };
    unsafe { dhruv_engine_free(engine_ptr) };
    unsafe { dhruv_eop_free(eop_ptr) };
}

#[test]
fn ffi_full_kundali_calendar_implies_panchang() {
    let (engine_ptr, eop_ptr) = match make_kundali_fixtures() {
        Some(f) => f,
        None => return,
    };
    let (utc, loc, bhava, rs) = kundali_test_params();

    let fk_config = DhruvFullKundaliConfig {
        include_bhava_cusps: 0,
        include_graha_positions: 0,
        include_bindus: 0,
        include_drishti: 0,
        include_ashtakavarga: 0,
        include_upagrahas: 0,
        include_sphutas: 0,
        include_special_lagnas: 0,
        include_amshas: 0,
        include_shadbala: 0,
        include_bhavabala: 0,
        include_vimsopaka: 0,
        include_avastha: 0,
        include_charakaraka: 0,
        charakaraka_scheme: 0,
        include_panchang: 0,
        include_calendar: 1,
        node_dignity_policy: 0,
        upagraha_config: dhruv_time_upagraha_config_default(),
        graha_positions_config: DhruvGrahaPositionsConfig {
            include_nakshatra: 0,
            include_lagna: 0,
            include_outer_planets: 0,
            include_bhava: 0,
        },
        bindus_config: DhruvBindusConfig {
            include_nakshatra: 0,
            include_bhava: 0,
            upagraha_config: dhruv_time_upagraha_config_default(),
        },
        drishti_config: DhruvDrishtiConfig {
            include_bhava: 0,
            include_lagna: 0,
            include_bindus: 0,
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
        include_dasha: 0,
        dasha_config: dhruv_dasha_selection_config_default(),
    };

    let mut result = std::mem::MaybeUninit::<DhruvFullKundaliResult>::uninit();
    let status = unsafe {
        dhruv_full_kundali_for_date(
            engine_ptr as *const _,
            eop_ptr as *const _,
            &utc,
            &loc,
            &bhava,
            &rs,
            0,
            1,
            &fk_config,
            result.as_mut_ptr(),
        )
    };
    assert_eq!(status, DhruvStatus::Ok);
    let r = unsafe { &*result.as_ptr() };

    // include_calendar=1 implies panchang is present
    assert_eq!(r.panchang_valid, 1);
    assert!((0..=29).contains(&r.panchang.tithi.tithi_index));
    assert!((0..=6).contains(&r.panchang.vaar.vaar_index));
    // Calendar fields should be populated
    assert_eq!(r.panchang.calendar_valid, 1);
    assert!(r.panchang.masa.start.year > 0);
    assert!(r.panchang.masa.end.year > 0);

    unsafe { dhruv_full_kundali_result_free(result.as_mut_ptr()) };
    unsafe { dhruv_engine_free(engine_ptr) };
    unsafe { dhruv_eop_free(eop_ptr) };
}

#[test]
fn ffi_full_kundali_panchang_and_calendar() {
    let (engine_ptr, eop_ptr) = match make_kundali_fixtures() {
        Some(f) => f,
        None => return,
    };
    let (utc, loc, bhava, rs) = kundali_test_params();

    let fk_config = DhruvFullKundaliConfig {
        include_bhava_cusps: 0,
        include_graha_positions: 0,
        include_bindus: 0,
        include_drishti: 0,
        include_ashtakavarga: 0,
        include_upagrahas: 0,
        include_sphutas: 0,
        include_special_lagnas: 0,
        include_amshas: 0,
        include_shadbala: 0,
        include_bhavabala: 0,
        include_vimsopaka: 0,
        include_avastha: 0,
        include_charakaraka: 0,
        charakaraka_scheme: 0,
        include_panchang: 1,
        include_calendar: 1,
        node_dignity_policy: 0,
        upagraha_config: dhruv_time_upagraha_config_default(),
        graha_positions_config: DhruvGrahaPositionsConfig {
            include_nakshatra: 0,
            include_lagna: 0,
            include_outer_planets: 0,
            include_bhava: 0,
        },
        bindus_config: DhruvBindusConfig {
            include_nakshatra: 0,
            include_bhava: 0,
            upagraha_config: dhruv_time_upagraha_config_default(),
        },
        drishti_config: DhruvDrishtiConfig {
            include_bhava: 0,
            include_lagna: 0,
            include_bindus: 0,
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
        include_dasha: 0,
        dasha_config: dhruv_dasha_selection_config_default(),
    };

    let mut result = std::mem::MaybeUninit::<DhruvFullKundaliResult>::uninit();
    let status = unsafe {
        dhruv_full_kundali_for_date(
            engine_ptr as *const _,
            eop_ptr as *const _,
            &utc,
            &loc,
            &bhava,
            &rs,
            0,
            1,
            &fk_config,
            result.as_mut_ptr(),
        )
    };
    assert_eq!(status, DhruvStatus::Ok);
    let r = unsafe { &*result.as_ptr() };

    assert_eq!(r.panchang_valid, 1);
    assert!((0..=29).contains(&r.panchang.tithi.tithi_index));
    assert_eq!(r.panchang.calendar_valid, 1);
    assert!(r.panchang.masa.start.year > 0);

    // bhava_cusps not requested → not populated.
    assert_eq!(r.bhava_cusps_valid, 0);
    // ayanamsha is always populated.
    assert!(r.ayanamsha_deg > 22.0 && r.ayanamsha_deg < 25.0);

    unsafe { dhruv_full_kundali_result_free(result.as_mut_ptr()) };
    unsafe { dhruv_engine_free(engine_ptr) };
    unsafe { dhruv_eop_free(eop_ptr) };
}

#[test]
fn ffi_full_kundali_bhava_cusps_enabled() {
    let (engine_ptr, eop_ptr) = match make_kundali_fixtures() {
        Some(f) => f,
        None => return,
    };
    let (utc, loc, bhava, rs) = kundali_test_params();

    let fk_config = DhruvFullKundaliConfig {
        include_bhava_cusps: 1,
        include_graha_positions: 0,
        include_bindus: 0,
        include_drishti: 0,
        include_ashtakavarga: 0,
        include_upagrahas: 0,
        include_sphutas: 0,
        include_special_lagnas: 0,
        include_amshas: 0,
        include_shadbala: 0,
        include_bhavabala: 0,
        include_vimsopaka: 0,
        include_avastha: 0,
        include_charakaraka: 0,
        charakaraka_scheme: 0,
        include_panchang: 0,
        include_calendar: 0,
        node_dignity_policy: 0,
        upagraha_config: dhruv_time_upagraha_config_default(),
        graha_positions_config: DhruvGrahaPositionsConfig {
            include_nakshatra: 0,
            include_lagna: 0,
            include_outer_planets: 0,
            include_bhava: 0,
        },
        bindus_config: DhruvBindusConfig {
            include_nakshatra: 0,
            include_bhava: 0,
            upagraha_config: dhruv_time_upagraha_config_default(),
        },
        drishti_config: DhruvDrishtiConfig {
            include_bhava: 0,
            include_lagna: 0,
            include_bindus: 0,
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
        include_dasha: 0,
        dasha_config: dhruv_dasha_selection_config_default(),
    };

    let mut result = std::mem::MaybeUninit::<DhruvFullKundaliResult>::uninit();
    let status = unsafe {
        dhruv_full_kundali_for_date(
            engine_ptr as *const _,
            eop_ptr as *const _,
            &utc,
            &loc,
            &bhava,
            &rs,
            0,
            1,
            &fk_config,
            result.as_mut_ptr(),
        )
    };
    assert_eq!(status, DhruvStatus::Ok);
    let r = unsafe { &*result.as_ptr() };

    assert_eq!(r.bhava_cusps_valid, 1);
    for i in 0..12 {
        assert_eq!(r.bhava_cusps.bhavas[i].number, (i + 1) as u8);
        assert!(
            r.bhava_cusps.bhavas[i].cusp_deg >= 0.0 && r.bhava_cusps.bhavas[i].cusp_deg < 360.0
        );
    }
    assert!(r.bhava_cusps.lagna_deg >= 0.0 && r.bhava_cusps.lagna_deg < 360.0);
    assert!(r.bhava_cusps.mc_deg >= 0.0 && r.bhava_cusps.mc_deg < 360.0);
    assert!(r.ayanamsha_deg > 22.0 && r.ayanamsha_deg < 25.0);
    // graha_positions not requested
    assert_eq!(r.graha_positions_valid, 0);

    unsafe { dhruv_full_kundali_result_free(result.as_mut_ptr()) };
    unsafe { dhruv_engine_free(engine_ptr) };
    unsafe { dhruv_eop_free(eop_ptr) };
}

#[test]
fn ffi_full_kundali_bhava_cusps_disabled() {
    let (engine_ptr, eop_ptr) = match make_kundali_fixtures() {
        Some(f) => f,
        None => return,
    };
    let (utc, loc, bhava, rs) = kundali_test_params();

    let fk_config = DhruvFullKundaliConfig {
        include_bhava_cusps: 0,
        include_graha_positions: 0,
        include_bindus: 0,
        include_drishti: 0,
        include_ashtakavarga: 0,
        include_upagrahas: 0,
        include_sphutas: 0,
        include_special_lagnas: 0,
        include_amshas: 0,
        include_shadbala: 0,
        include_bhavabala: 0,
        include_vimsopaka: 0,
        include_avastha: 0,
        include_charakaraka: 0,
        charakaraka_scheme: 0,
        include_panchang: 1,
        include_calendar: 0,
        node_dignity_policy: 0,
        upagraha_config: dhruv_time_upagraha_config_default(),
        graha_positions_config: DhruvGrahaPositionsConfig {
            include_nakshatra: 0,
            include_lagna: 0,
            include_outer_planets: 0,
            include_bhava: 0,
        },
        bindus_config: DhruvBindusConfig {
            include_nakshatra: 0,
            include_bhava: 0,
            upagraha_config: dhruv_time_upagraha_config_default(),
        },
        drishti_config: DhruvDrishtiConfig {
            include_bhava: 0,
            include_lagna: 0,
            include_bindus: 0,
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
        include_dasha: 0,
        dasha_config: dhruv_dasha_selection_config_default(),
    };

    let mut result = std::mem::MaybeUninit::<DhruvFullKundaliResult>::uninit();
    let status = unsafe {
        dhruv_full_kundali_for_date(
            engine_ptr as *const _,
            eop_ptr as *const _,
            &utc,
            &loc,
            &bhava,
            &rs,
            0,
            1,
            &fk_config,
            result.as_mut_ptr(),
        )
    };
    assert_eq!(status, DhruvStatus::Ok);
    let r = unsafe { &*result.as_ptr() };

    // bhava_cusps disabled but panchang requested — should succeed
    assert_eq!(r.bhava_cusps_valid, 0);
    assert_eq!(r.panchang_valid, 1);
    assert!(r.ayanamsha_deg > 22.0 && r.ayanamsha_deg < 25.0);

    unsafe { dhruv_full_kundali_result_free(result.as_mut_ptr()) };
    unsafe { dhruv_engine_free(engine_ptr) };
    unsafe { dhruv_eop_free(eop_ptr) };
}

/// FFI: bhava cusps off + include_calendar=1 succeeds, calendar data populated.
#[test]
fn ffi_full_kundali_bhava_off_calendar_on() {
    let (engine_ptr, eop_ptr) = match make_kundali_fixtures() {
        Some(f) => f,
        None => return,
    };
    let (utc, loc, bhava, rs) = kundali_test_params();

    let mut fk_config = dhruv_full_kundali_config_default();
    fk_config.include_bhava_cusps = 0;
    fk_config.include_graha_positions = 0;
    fk_config.include_bindus = 0;
    fk_config.include_drishti = 0;
    fk_config.include_ashtakavarga = 0;
    fk_config.include_upagrahas = 0;
    fk_config.include_special_lagnas = 0;
    fk_config.include_panchang = 0;
    fk_config.include_calendar = 1;

    let mut result = std::mem::MaybeUninit::<DhruvFullKundaliResult>::uninit();
    let status = unsafe {
        dhruv_full_kundali_for_date(
            engine_ptr as *const _,
            eop_ptr as *const _,
            &utc,
            &loc,
            &bhava,
            &rs,
            0,
            1,
            &fk_config,
            result.as_mut_ptr(),
        )
    };
    assert_eq!(status, DhruvStatus::Ok);
    let r = unsafe { &*result.as_ptr() };

    assert_eq!(r.bhava_cusps_valid, 0, "bhava cusps should be off");
    assert_eq!(r.panchang_valid, 1, "calendar implies panchang");

    unsafe { dhruv_full_kundali_result_free(result.as_mut_ptr()) };
    unsafe { dhruv_engine_free(engine_ptr) };
    unsafe { dhruv_eop_free(eop_ptr) };
}

#[test]
fn ffi_full_kundali_exposes_root_sphutas() {
    let (engine_ptr, eop_ptr) = match make_kundali_fixtures() {
        Some(f) => f,
        None => return,
    };
    let (utc, loc, bhava, rs) = kundali_test_params();

    let mut fk_config = dhruv_full_kundali_config_default();
    fk_config.include_bhava_cusps = 0;
    fk_config.include_graha_positions = 0;
    fk_config.include_bindus = 0;
    fk_config.include_drishti = 0;
    fk_config.include_ashtakavarga = 0;
    fk_config.include_upagrahas = 0;
    fk_config.include_special_lagnas = 0;
    fk_config.include_panchang = 0;
    fk_config.include_calendar = 0;
    fk_config.include_dasha = 0;
    fk_config.include_sphutas = 1;

    let mut result = std::mem::MaybeUninit::<DhruvFullKundaliResult>::uninit();
    let status = unsafe {
        dhruv_full_kundali_for_date(
            engine_ptr as *const _,
            eop_ptr as *const _,
            &utc,
            &loc,
            &bhava,
            &rs,
            0,
            1,
            &fk_config,
            result.as_mut_ptr(),
        )
    };
    assert_eq!(status, DhruvStatus::Ok);
    let r = unsafe { &*result.as_ptr() };
    assert_eq!(r.sphutas_valid, 1);
    assert!(r.sphutas.longitudes.iter().all(|lon| lon.is_finite()));
    unsafe { dhruv_full_kundali_result_free(result.as_mut_ptr()) };
    unsafe { dhruv_engine_free(engine_ptr) };
    unsafe { dhruv_eop_free(eop_ptr) };
}

#[test]
fn ffi_full_kundali_dasha_per_system_max_levels() {
    let (engine_ptr, eop_ptr) = match make_kundali_fixtures() {
        Some(f) => f,
        None => return,
    };
    let (utc, loc, bhava, rs) = kundali_test_params();

    let mut fk_config = dhruv_full_kundali_config_default();
    fk_config.include_bhava_cusps = 0;
    fk_config.include_graha_positions = 0;
    fk_config.include_bindus = 0;
    fk_config.include_drishti = 0;
    fk_config.include_ashtakavarga = 0;
    fk_config.include_upagrahas = 0;
    fk_config.include_special_lagnas = 0;
    fk_config.include_panchang = 0;
    fk_config.include_calendar = 0;
    fk_config.include_sphutas = 0;
    fk_config.include_dasha = 1;
    fk_config.dasha_config.count = 2;
    fk_config.dasha_config.systems[0] = 0;
    fk_config.dasha_config.systems[1] = 1;
    fk_config.dasha_config.max_level = 4;
    fk_config.dasha_config.max_levels[0] = 0;
    fk_config.dasha_config.max_levels[1] = 1;

    let mut result = std::mem::MaybeUninit::<DhruvFullKundaliResult>::uninit();
    let status = unsafe {
        dhruv_full_kundali_for_date(
            engine_ptr as *const _,
            eop_ptr as *const _,
            &utc,
            &loc,
            &bhava,
            &rs,
            0,
            1,
            &fk_config,
            result.as_mut_ptr(),
        )
    };
    assert_eq!(status, DhruvStatus::Ok);
    let r = unsafe { &*result.as_ptr() };
    assert_eq!(r.dasha_count, 2);
    assert_eq!(r.dasha_systems[0], 0);
    assert_eq!(r.dasha_systems[1], 1);

    let mut level_count = 0_u8;
    let status = unsafe { dhruv_dasha_hierarchy_level_count(r.dasha_handles[0], &mut level_count) };
    assert_eq!(status, DhruvStatus::Ok);
    assert_eq!(level_count, 1);
    let status = unsafe { dhruv_dasha_hierarchy_level_count(r.dasha_handles[1], &mut level_count) };
    assert_eq!(status, DhruvStatus::Ok);
    assert_eq!(level_count, 2);

    unsafe { dhruv_full_kundali_result_free(result.as_mut_ptr()) };
    unsafe { dhruv_engine_free(engine_ptr) };
    unsafe { dhruv_eop_free(eop_ptr) };
}

#[test]
fn ffi_lunar_node_with_engine_rejects_null() {
    let mut out = 0.0_f64;
    // SAFETY: Null engine pointer is intentional for validation.
    let status = unsafe {
        dhruv_lunar_node_deg_with_engine(
            ptr::null(),
            DHRUV_NODE_RAHU,
            DHRUV_NODE_MODE_TRUE,
            2_451_545.0,
            &mut out,
        )
    };
    assert_eq!(status, DhruvStatus::NullPointer);
}

#[test]
fn ffi_lunar_node_with_engine_mean_matches_pure_utc() {
    let engine_ptr = match make_engine() {
        Some(p) => p,
        None => return,
    };
    let lsk_path = match lsk_path_cstr() {
        Some(p) => p,
        None => return,
    };
    let mut lsk_ptr: *mut DhruvLskHandle = ptr::null_mut();
    // SAFETY: Valid path and output pointer.
    let load_status =
        unsafe { dhruv_lsk_load(lsk_path.as_ptr() as *const std::ffi::c_char, &mut lsk_ptr) };
    assert_eq!(load_status, DhruvStatus::Ok);

    let utc = DhruvUtcTime {
        year: 1931,
        month: 12,
        day: 11,
        hour: 11,
        minute: 38,
        second: 18.0,
    };

    let mut pure_mean = 0.0_f64;
    let mut eng_mean = 0.0_f64;
    // SAFETY: Valid pointers.
    let s1 = unsafe {
        dhruv_lunar_node_deg_utc(
            lsk_ptr as *const _,
            DHRUV_NODE_RAHU,
            DHRUV_NODE_MODE_MEAN,
            &utc,
            &mut pure_mean,
        )
    };
    // SAFETY: Valid pointers.
    let s2 = unsafe {
        dhruv_lunar_node_deg_utc_with_engine(
            engine_ptr as *const _,
            lsk_ptr as *const _,
            DHRUV_NODE_RAHU,
            DHRUV_NODE_MODE_MEAN,
            &utc,
            &mut eng_mean,
        )
    };
    assert_eq!(s1, DhruvStatus::Ok);
    assert_eq!(s2, DhruvStatus::Ok);
    assert!(
        angular_separation_deg(pure_mean, eng_mean) < 1e-9,
        "mean-node mismatch: pure={pure_mean}, engine={eng_mean}"
    );

    // SAFETY: Handles created in this test and not yet freed.
    unsafe { dhruv_lsk_free(lsk_ptr) };
    // SAFETY: Handle created in this test and not yet freed.
    unsafe { dhruv_engine_free(engine_ptr) };
}

#[test]
fn ffi_lunar_node_fitted_tracks_osculating_utc() {
    let engine_ptr = match make_engine() {
        Some(p) => p,
        None => return,
    };
    let lsk_path = match lsk_path_cstr() {
        Some(p) => p,
        None => return,
    };
    let mut lsk_ptr: *mut DhruvLskHandle = ptr::null_mut();
    // SAFETY: Valid path and output pointer.
    let load_status =
        unsafe { dhruv_lsk_load(lsk_path.as_ptr() as *const std::ffi::c_char, &mut lsk_ptr) };
    assert_eq!(load_status, DhruvStatus::Ok);

    // Test at multiple epochs spanning the fit interval (1900–2100).
    let epochs = [
        DhruvUtcTime {
            year: 1931,
            month: 12,
            day: 11,
            hour: 11,
            minute: 38,
            second: 18.0,
        },
        DhruvUtcTime {
            year: 1970,
            month: 1,
            day: 1,
            hour: 0,
            minute: 0,
            second: 0.0,
        },
        DhruvUtcTime {
            year: 2000,
            month: 6,
            day: 21,
            hour: 12,
            minute: 0,
            second: 0.0,
        },
        DhruvUtcTime {
            year: 2024,
            month: 3,
            day: 15,
            hour: 6,
            minute: 30,
            second: 0.0,
        },
    ];

    for utc in &epochs {
        let mut pure_true = 0.0_f64;
        let mut eng_true = 0.0_f64;
        // SAFETY: Valid pointers.
        let s1 = unsafe {
            dhruv_lunar_node_deg_utc(
                lsk_ptr as *const _,
                DHRUV_NODE_RAHU,
                DHRUV_NODE_MODE_TRUE,
                utc,
                &mut pure_true,
            )
        };
        // SAFETY: Valid pointers.
        let s2 = unsafe {
            dhruv_lunar_node_deg_utc_with_engine(
                engine_ptr as *const _,
                lsk_ptr as *const _,
                DHRUV_NODE_RAHU,
                DHRUV_NODE_MODE_TRUE,
                utc,
                &mut eng_true,
            )
        };
        assert_eq!(s1, DhruvStatus::Ok);
        assert_eq!(s2, DhruvStatus::Ok);

        let diff = angular_separation_deg(pure_true, eng_true);
        // Pure-math 50-term fitted series should closely track the osculating
        // node (RMS ≈ 5″).  Allow up to 30″ = 0.0083° at any single epoch.
        assert!(
            diff < 0.0084,
            "year={}: fitted vs osculating diff={diff} deg ({:.1}\")",
            utc.year,
            diff * 3600.0,
        );
    }

    // SAFETY: Handles created in this test and not yet freed.
    unsafe { dhruv_lsk_free(lsk_ptr) };
    // SAFETY: Handle created in this test and not yet freed.
    unsafe { dhruv_engine_free(engine_ptr) };
}

// ---------------------------------------------------------------------------
// Graha Tropical Longitudes FFI
// ---------------------------------------------------------------------------

#[test]
fn ffi_graha_longitudes_tropical_basic() {
    let config = match real_config() {
        Some(c) => c,
        None => return,
    };
    let mut engine_ptr: *mut DhruvEngineHandle = ptr::null_mut();
    let s = unsafe { dhruv_engine_new(&config, &mut engine_ptr) };
    assert_eq!(s, DhruvStatus::Ok);

    let jd = 2_451_545.0; // J2000
    let mut out = DhruvGrahaLongitudes {
        longitudes: [0.0; 9],
    };
    let mut cfg = dhruv_graha_longitudes_config_default();
    cfg.kind = DHRUV_GRAHA_LONGITUDE_KIND_TROPICAL;
    let s = unsafe {
        dhruv_graha_longitudes(engine_ptr.cast::<dhruv_core::Engine>(), jd, &cfg, &mut out)
    };
    assert_eq!(s, DhruvStatus::Ok);

    for (i, &lon) in out.longitudes.iter().enumerate() {
        assert!(
            (0.0..360.0).contains(&lon),
            "graha {i}: tropical longitude {lon} not in [0, 360)"
        );
    }

    unsafe { dhruv_engine_free(engine_ptr) };
}

#[test]
fn ffi_tropical_equals_sidereal_plus_ayanamsha() {
    let config = match real_config() {
        Some(c) => c,
        None => return,
    };
    let mut engine_ptr: *mut DhruvEngineHandle = ptr::null_mut();
    let s = unsafe { dhruv_engine_new(&config, &mut engine_ptr) };
    assert_eq!(s, DhruvStatus::Ok);

    let jd = 2_451_545.0;
    let engine_raw = engine_ptr.cast::<dhruv_core::Engine>();

    let mut tropical_out = DhruvGrahaLongitudes {
        longitudes: [0.0; 9],
    };
    let mut tropical_cfg = dhruv_graha_longitudes_config_default();
    tropical_cfg.kind = DHRUV_GRAHA_LONGITUDE_KIND_TROPICAL;
    let s = unsafe { dhruv_graha_longitudes(engine_raw, jd, &tropical_cfg, &mut tropical_out) };
    assert_eq!(s, DhruvStatus::Ok);

    let mut sidereal_out = DhruvGrahaLongitudes {
        longitudes: [0.0; 9],
    };
    let sidereal_cfg = dhruv_graha_longitudes_config_default();
    let s = unsafe { dhruv_graha_longitudes(engine_raw, jd, &sidereal_cfg, &mut sidereal_out) };
    assert_eq!(s, DhruvStatus::Ok);

    let t = dhruv_vedic_base::jd_tdb_to_centuries(jd);
    let aya = dhruv_vedic_base::ayanamsha_deg(dhruv_vedic_base::AyanamshaSystem::Lahiri, t, false);

    for i in 0..9 {
        let reconstructed = (sidereal_out.longitudes[i] + aya).rem_euclid(360.0);
        let diff = (tropical_out.longitudes[i] - reconstructed).rem_euclid(360.0);
        let diff = if diff > 180.0 { diff - 360.0 } else { diff };
        assert!(
            diff.abs() < 1e-10,
            "graha {i}: tropical={}, sidereal+aya={}, diff={diff:.2e}",
            tropical_out.longitudes[i],
            reconstructed,
        );
    }

    unsafe { dhruv_engine_free(engine_ptr) };
}

#[test]
fn ffi_moving_osculating_apogees_for_date_basic_and_invalid_graha() {
    let config = match real_config() {
        Some(c) => c,
        None => return,
    };
    let mut engine_ptr: *mut DhruvEngineHandle = ptr::null_mut();
    let s = unsafe { dhruv_engine_new(&config, &mut engine_ptr) };
    assert_eq!(s, DhruvStatus::Ok);
    let engine_raw = engine_ptr.cast::<dhruv_core::Engine>();
    let utc = DhruvUtcTime {
        year: 2026,
        month: 4,
        day: 17,
        hour: 13,
        minute: 25,
        second: 39.0,
    };
    let cfg = dhruv_graha_longitudes_config_default();
    let grahas = [2u8, 3, 2, 6];
    let mut out: DhruvMovingOsculatingApogees = unsafe { std::mem::zeroed() };
    let s = unsafe {
        dhruv_moving_osculating_apogees_for_date(
            engine_raw,
            ptr::null(),
            &utc,
            grahas.as_ptr(),
            grahas.len() as u8,
            &cfg,
            &mut out,
        )
    };
    assert_eq!(s, DhruvStatus::Ok);
    assert_eq!(out.count, 4);
    assert_eq!(out.entries[0].graha_index, 2);
    assert_eq!(out.entries[2].graha_index, 2);
    assert_eq!(
        out.entries[0].sidereal_longitude,
        out.entries[2].sidereal_longitude
    );
    for entry in out.entries.iter().take(out.count as usize) {
        assert!((0.0..360.0).contains(&entry.sidereal_longitude));
        assert!((0.0..360.0).contains(&entry.reference_plane_longitude));
        assert!(entry.ayanamsha_deg.is_finite());
    }

    let invalid = [0u8];
    let s = unsafe {
        dhruv_moving_osculating_apogees_for_date(
            engine_raw,
            ptr::null(),
            &utc,
            invalid.as_ptr(),
            invalid.len() as u8,
            &cfg,
            &mut out,
        )
    };
    assert_eq!(s, DhruvStatus::InvalidSearchConfig);

    unsafe { dhruv_engine_free(engine_ptr) };
}
