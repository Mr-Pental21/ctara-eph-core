use std::path::PathBuf;

use criterion::{
    BenchmarkGroup, Criterion, black_box, criterion_group, criterion_main, measurement::WallTime,
};
use dhruv_core::{Body, Frame, Observer};
use dhruv_ffi_c::{
    DhruvBhinnaAshtakavarga, DhruvDrishtiEntry, DhruvEngineConfig, DhruvGrahaDrishtiMatrix,
    DhruvGrahaLongitudes, DhruvKaranaPosition, DhruvNakshatra28Info, DhruvNakshatraInfo,
    DhruvPanchangNakshatraInfo, DhruvQuery, DhruvRashiInfo, DhruvSamvatsaraResult,
    DhruvSarvaAshtakavarga, DhruvSphericalCoords, DhruvSphericalState, DhruvStateVector,
    DhruvStatus, DhruvTithiPosition, DhruvUtcTime, DhruvYogaPosition,
    dhruv_ayana_from_sidereal_longitude, dhruv_ayanamsha_deg, dhruv_ayanamsha_mean_deg,
    dhruv_ayanamsha_true_deg, dhruv_calculate_all_bav, dhruv_calculate_bav, dhruv_calculate_sav,
    dhruv_cartesian_to_spherical, dhruv_ekadhipatya_sodhana, dhruv_engine_new_internal,
    dhruv_engine_query, dhruv_engine_query_internal, dhruv_ghatika_from_elapsed,
    dhruv_ghatikas_since_sunrise, dhruv_graha_drishti, dhruv_graha_drishti_matrix,
    dhruv_graha_sidereal_longitudes, dhruv_hora_at, dhruv_jd_tdb_to_utc,
    dhruv_karana_from_elongation, dhruv_lunar_node_deg, dhruv_masa_from_rashi_index,
    dhruv_nakshatra_at, dhruv_nakshatra_from_longitude, dhruv_nakshatra_from_tropical,
    dhruv_nakshatra28_from_longitude, dhruv_nth_rashi_from, dhruv_nutation_iau2000b,
    dhruv_query_once, dhruv_query_once_internal, dhruv_query_utc, dhruv_query_utc_spherical,
    dhruv_query_utc_spherical_internal, dhruv_rashi_from_longitude, dhruv_rashi_from_tropical,
    dhruv_rashi_lord, dhruv_samvatsara_from_year, dhruv_sankranti_config_default,
    dhruv_time_upagraha_jd, dhruv_tithi_from_elongation, dhruv_trikona_sodhana,
    dhruv_utc_to_tdb_jd, dhruv_vaar_from_jd, dhruv_yoga_from_sum,
};
use dhruv_frames::{
    cartesian_to_spherical as rust_cartesian_to_spherical,
    nutation_iau2000b as rust_nutation_iau2000b,
};
use dhruv_search::{
    SankrantiConfig, graha_sidereal_longitudes as rust_graha_sidereal_longitudes,
    nakshatra_at as rust_nakshatra_at,
};
use dhruv_time::{Epoch, UtcTime};
use dhruv_vedic_base::{
    AyanamshaSystem, Graha, LunarNode, NodeMode, Upagraha, Vaar,
    ayana_from_sidereal_longitude as rust_ayana_from_sidereal_longitude,
    ayanamsha_deg as rust_ayanamsha_deg, ayanamsha_mean_deg as rust_ayanamsha_mean_deg,
    ayanamsha_true_deg as rust_ayanamsha_true_deg, calculate_all_bav as rust_calculate_all_bav,
    calculate_bav as rust_calculate_bav, calculate_sav as rust_calculate_sav,
    ekadhipatya_sodhana as rust_ekadhipatya_sodhana,
    ghatika_from_elapsed as rust_ghatika_from_elapsed,
    ghatikas_since_sunrise as rust_ghatikas_since_sunrise, graha_drishti as rust_graha_drishti,
    graha_drishti_matrix as rust_graha_drishti_matrix, hora_at as rust_hora_at,
    jd_tdb_to_centuries as rust_jd_tdb_to_centuries,
    karana_from_elongation as rust_karana_from_elongation, lunar_node_deg as rust_lunar_node_deg,
    masa_from_rashi_index as rust_masa_from_rashi_index,
    nakshatra_from_longitude as rust_nakshatra_from_longitude,
    nakshatra_from_tropical as rust_nakshatra_from_tropical,
    nakshatra28_from_longitude as rust_nakshatra28_from_longitude,
    nth_rashi_from as rust_nth_rashi_from, rashi_from_longitude as rust_rashi_from_longitude,
    rashi_from_tropical as rust_rashi_from_tropical,
    rashi_lord_by_index as rust_rashi_lord_by_index,
    samvatsara_from_year as rust_samvatsara_from_year, time_upagraha_jd as rust_time_upagraha_jd,
    tithi_from_elongation as rust_tithi_from_elongation, trikona_sodhana as rust_trikona_sodhana,
    vaar_from_jd as rust_vaar_from_jd, yoga_from_sum as rust_yoga_from_sum,
};

struct FfiContext {
    config: DhruvEngineConfig,
    query: DhruvQuery,
    engine: dhruv_ffi_c::DhruvEngineHandle,
    lsk: dhruv_time::LeapSecondKernel,
    eop: dhruv_time::EopKernel,
}

fn zeroed<T>() -> T {
    // Bench output structs are C POD reprs; zero-init is valid.
    unsafe { std::mem::zeroed() }
}

fn expect_ok(status: DhruvStatus) {
    assert_eq!(
        status as i32,
        DhruvStatus::Ok as i32,
        "ffi status: {status:?}"
    );
}

fn ffi_utc_to_rust(utc: &DhruvUtcTime) -> UtcTime {
    UtcTime::new(
        utc.year, utc.month, utc.day, utc.hour, utc.minute, utc.second,
    )
}

fn ffi_utc_to_jd_tdb(utc: &DhruvUtcTime, lsk: &dhruv_time::LeapSecondKernel) -> f64 {
    ffi_utc_to_rust(utc).to_jd_tdb(lsk)
}

fn ffi_utc_to_jd_utc(utc: &DhruvUtcTime) -> f64 {
    let day_frac = utc.day as f64
        + utc.hour as f64 / 24.0
        + utc.minute as f64 / 1_440.0
        + utc.second / 86_400.0;
    dhruv_time::calendar_to_jd(utc.year, utc.month, day_frac)
}

fn bench_pair<F1, F2, R1, R2>(
    group: &mut BenchmarkGroup<'_, WallTime>,
    name: &str,
    mut rust_fn: F1,
    mut ffi_fn: F2,
) where
    F1: FnMut() -> R1,
    F2: FnMut() -> R2,
{
    group.bench_function(format!("{name}/rust"), |b| b.iter(|| black_box(rust_fn())));
    group.bench_function(format!("{name}/ffi"), |b| b.iter(|| black_box(ffi_fn())));
}

fn load_context() -> Option<FfiContext> {
    let base = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../kernels/data");
    let spk = base.join("de442s.bsp");
    let lsk = base.join("naif0012.tls");
    let eop = base.join("finals2000A.all");
    if !spk.exists() || !lsk.exists() || !eop.exists() {
        eprintln!("Skipping benchmarks: kernel files not found");
        return None;
    }

    let config = DhruvEngineConfig::try_new(
        spk.to_str().expect("utf-8 path"),
        lsk.to_str().expect("utf-8 path"),
        256,
        true,
    )
    .expect("valid config");
    let engine = dhruv_engine_new_internal(&config).expect("engine should load");
    let lsk = dhruv_time::LeapSecondKernel::load(&lsk).expect("lsk should load");
    let eop = dhruv_time::EopKernel::load(&eop).expect("eop should load");
    let query = DhruvQuery {
        target: Body::Mars.code(),
        observer: Observer::Body(Body::Earth).code(),
        frame: Frame::IcrfJ2000.code(),
        epoch_tdb_jd: 2_460_000.5,
    };

    Some(FfiContext {
        config,
        query,
        engine,
        lsk,
        eop,
    })
}

fn ffi_query_bench(c: &mut Criterion) {
    let ctx = match load_context() {
        Some(v) => v,
        None => return,
    };

    let mut group = c.benchmark_group("ffi_query");
    group.bench_function("engine_query_internal", |b| {
        b.iter(|| {
            dhruv_engine_query_internal(black_box(&ctx.engine), black_box(ctx.query))
                .expect("query should succeed")
        })
    });

    // Includes engine construction/destruction path for one-shot callers.
    group.sample_size(20);
    group.bench_function("query_once_internal", |b| {
        b.iter(|| {
            dhruv_query_once_internal(black_box(&ctx.config), black_box(ctx.query))
                .expect("query should succeed")
        })
    });
    group.finish();
}

fn ffi_core_cabi_bench(c: &mut Criterion) {
    let ctx = match load_context() {
        Some(v) => v,
        None => return,
    };

    let mut group = c.benchmark_group("ffi_core_cabi");
    group.sample_size(20);

    let mut out_state: DhruvStateVector = zeroed();
    bench_pair(
        &mut group,
        "engine_query",
        || {
            dhruv_engine_query_internal(black_box(&ctx.engine), black_box(ctx.query))
                .expect("query should succeed")
                .position_km[0]
        },
        || unsafe {
            expect_ok(dhruv_engine_query(
                black_box(&ctx.engine as *const _),
                black_box(&ctx.query as *const _),
                &mut out_state,
            ));
            out_state.position_km[0]
        },
    );

    let mut out_once: DhruvStateVector = zeroed();
    bench_pair(
        &mut group,
        "query_once",
        || {
            dhruv_query_once_internal(black_box(&ctx.config), black_box(ctx.query))
                .expect("query should succeed")
                .position_km[0]
        },
        || unsafe {
            expect_ok(dhruv_query_once(
                black_box(&ctx.config as *const _),
                black_box(&ctx.query as *const _),
                &mut out_once,
            ));
            out_once.position_km[0]
        },
    );

    let mut out_spherical: DhruvSphericalState = zeroed();
    bench_pair(
        &mut group,
        "query_utc_spherical",
        || {
            dhruv_query_utc_spherical_internal(
                black_box(&ctx.engine),
                Body::Mars.code(),
                Observer::Body(Body::Earth).code(),
                Frame::IcrfJ2000.code(),
                2024,
                3,
                20,
                12,
                0,
                0.0,
            )
            .expect("query should succeed")
            .lon_deg
        },
        || unsafe {
            expect_ok(dhruv_query_utc_spherical(
                black_box(&ctx.engine as *const _),
                Body::Mars.code(),
                Observer::Body(Body::Earth).code(),
                Frame::IcrfJ2000.code(),
                2024,
                3,
                20,
                12,
                0,
                0.0,
                &mut out_spherical,
            ));
            out_spherical.lon_deg
        },
    );

    let utc = DhruvUtcTime {
        year: 2024,
        month: 3,
        day: 20,
        hour: 12,
        minute: 0,
        second: 0.0,
    };
    let mut out_utc_query: DhruvSphericalState = zeroed();
    bench_pair(
        &mut group,
        "query_utc_struct",
        || {
            dhruv_query_utc_spherical_internal(
                black_box(&ctx.engine),
                Body::Mars.code(),
                Observer::Body(Body::Earth).code(),
                Frame::IcrfJ2000.code(),
                utc.year,
                utc.month,
                utc.day,
                utc.hour,
                utc.minute,
                utc.second,
            )
            .expect("query should succeed")
            .lat_deg
        },
        || unsafe {
            expect_ok(dhruv_query_utc(
                black_box(&ctx.engine as *const _),
                Body::Mars.code(),
                Observer::Body(Body::Earth).code(),
                Frame::IcrfJ2000.code(),
                black_box(&utc as *const _),
                &mut out_utc_query,
            ));
            out_utc_query.lat_deg
        },
    );

    group.finish();
}

fn ffi_time_frame_bench(c: &mut Criterion) {
    let ctx = match load_context() {
        Some(v) => v,
        None => return,
    };

    let mut group = c.benchmark_group("ffi_time_frame");

    let (year, month, day, hour, min, sec) = (2024_i32, 3_u32, 20_u32, 12_u32, 0_u32, 0.0_f64);
    let mut out_jd = 0.0_f64;
    bench_pair(
        &mut group,
        "utc_to_tdb_jd",
        || {
            Epoch::from_utc(
                black_box(year),
                black_box(month),
                black_box(day),
                black_box(hour),
                black_box(min),
                black_box(sec),
                black_box(&ctx.lsk),
            )
            .as_jd_tdb()
        },
        || unsafe {
            expect_ok(dhruv_utc_to_tdb_jd(
                black_box(&ctx.lsk as *const _),
                year,
                month,
                day,
                hour,
                min,
                sec,
                &mut out_jd,
            ));
            out_jd
        },
    );

    let jd_tdb = 2_460_000.5_f64;
    let mut out_utc: DhruvUtcTime = zeroed();
    bench_pair(
        &mut group,
        "jd_tdb_to_utc",
        || UtcTime::from_jd_tdb(black_box(jd_tdb), black_box(&ctx.lsk)).second,
        || unsafe {
            expect_ok(dhruv_jd_tdb_to_utc(
                black_box(&ctx.lsk as *const _),
                jd_tdb,
                &mut out_utc,
            ));
            out_utc.second
        },
    );

    let xyz = [123_456.0_f64, -65_432.0_f64, 11_111.0_f64];
    let mut out_spherical: DhruvSphericalCoords = zeroed();
    bench_pair(
        &mut group,
        "cartesian_to_spherical",
        || rust_cartesian_to_spherical(black_box(&xyz)).lon_deg,
        || unsafe {
            expect_ok(dhruv_cartesian_to_spherical(
                black_box(&xyz as *const _),
                &mut out_spherical,
            ));
            out_spherical.lon_deg
        },
    );

    group.finish();
}

fn ffi_vedic_primitives_bench(c: &mut Criterion) {
    let mut group = c.benchmark_group("ffi_vedic_primitives");
    let ctx = load_context();

    let jd_tdb = 2_460_000.5_f64;
    let t = rust_jd_tdb_to_centuries(jd_tdb);
    let system = AyanamshaSystem::all()[0];
    let system_code = 0_i32;

    let mut out_deg = 0.0_f64;
    bench_pair(
        &mut group,
        "ayanamsha_mean_deg",
        || rust_ayanamsha_mean_deg(black_box(system), black_box(t)),
        || unsafe {
            expect_ok(dhruv_ayanamsha_mean_deg(system_code, jd_tdb, &mut out_deg));
            out_deg
        },
    );

    let (dpsi, _) = rust_nutation_iau2000b(t);
    bench_pair(
        &mut group,
        "ayanamsha_true_deg",
        || rust_ayanamsha_true_deg(black_box(system), black_box(t), black_box(dpsi)),
        || unsafe {
            expect_ok(dhruv_ayanamsha_true_deg(
                system_code,
                jd_tdb,
                dpsi,
                &mut out_deg,
            ));
            out_deg
        },
    );

    bench_pair(
        &mut group,
        "ayanamsha_deg",
        || rust_ayanamsha_deg(black_box(system), black_box(t), false),
        || unsafe {
            expect_ok(dhruv_ayanamsha_deg(system_code, jd_tdb, 0, &mut out_deg));
            out_deg
        },
    );

    let mut out_dpsi = 0.0_f64;
    let mut out_deps = 0.0_f64;
    bench_pair(
        &mut group,
        "nutation_iau2000b",
        || rust_nutation_iau2000b(black_box(t)).0,
        || unsafe {
            expect_ok(dhruv_nutation_iau2000b(
                jd_tdb,
                &mut out_dpsi,
                &mut out_deps,
            ));
            out_dpsi
        },
    );

    bench_pair(
        &mut group,
        "lunar_node_deg",
        || {
            rust_lunar_node_deg(
                black_box(LunarNode::Rahu),
                black_box(t),
                black_box(NodeMode::True),
            )
        },
        || unsafe {
            expect_ok(dhruv_lunar_node_deg(0, 1, jd_tdb, &mut out_deg));
            out_deg
        },
    );

    let sidereal_lon = 123.456_f64;
    let tropical_lon = 210.123_f64;
    let mut out_rashi: DhruvRashiInfo = zeroed();
    let mut out_nak: DhruvNakshatraInfo = zeroed();
    let mut out_nak28: DhruvNakshatra28Info = zeroed();

    bench_pair(
        &mut group,
        "rashi_from_longitude",
        || rust_rashi_from_longitude(black_box(sidereal_lon)).rashi_index,
        || unsafe {
            expect_ok(dhruv_rashi_from_longitude(sidereal_lon, &mut out_rashi));
            out_rashi.rashi_index
        },
    );

    bench_pair(
        &mut group,
        "nakshatra_from_longitude",
        || rust_nakshatra_from_longitude(black_box(sidereal_lon)).nakshatra_index,
        || unsafe {
            expect_ok(dhruv_nakshatra_from_longitude(sidereal_lon, &mut out_nak));
            out_nak.nakshatra_index
        },
    );

    bench_pair(
        &mut group,
        "nakshatra28_from_longitude",
        || rust_nakshatra28_from_longitude(black_box(sidereal_lon)).nakshatra_index,
        || unsafe {
            expect_ok(dhruv_nakshatra28_from_longitude(
                sidereal_lon,
                &mut out_nak28,
            ));
            out_nak28.nakshatra_index
        },
    );

    let mut out_dms: dhruv_ffi_c::DhruvDms = zeroed();
    bench_pair(
        &mut group,
        "deg_to_dms",
        || {
            rust_rashi_from_longitude(black_box(sidereal_lon))
                .dms
                .degrees as i32
        },
        || unsafe {
            expect_ok(dhruv_ffi_c::dhruv_deg_to_dms(sidereal_lon, &mut out_dms));
            out_dms.degrees as i32
        },
    );

    bench_pair(
        &mut group,
        "rashi_from_tropical",
        || {
            rust_rashi_from_tropical(black_box(tropical_lon), system, black_box(jd_tdb), false)
                .rashi_index
        },
        || unsafe {
            expect_ok(dhruv_rashi_from_tropical(
                tropical_lon,
                system_code,
                jd_tdb,
                0,
                &mut out_rashi,
            ));
            out_rashi.rashi_index
        },
    );

    bench_pair(
        &mut group,
        "nakshatra_from_tropical",
        || {
            rust_nakshatra_from_tropical(black_box(tropical_lon), system, black_box(jd_tdb), false)
                .nakshatra_index
        },
        || unsafe {
            expect_ok(dhruv_nakshatra_from_tropical(
                tropical_lon,
                system_code,
                jd_tdb,
                0,
                &mut out_nak,
            ));
            out_nak.nakshatra_index
        },
    );

    bench_pair(
        &mut group,
        "nakshatra28_from_tropical",
        || {
            dhruv_vedic_base::nakshatra28_from_tropical(
                black_box(tropical_lon),
                system,
                black_box(jd_tdb),
                false,
            )
            .nakshatra_index
        },
        || unsafe {
            expect_ok(dhruv_ffi_c::dhruv_nakshatra28_from_tropical(
                tropical_lon,
                system_code,
                jd_tdb,
                0,
                &mut out_nak28,
            ));
            out_nak28.nakshatra_index
        },
    );

    if let Some(ctx) = ctx {
        let utc = DhruvUtcTime {
            year: 2024,
            month: 3,
            day: 20,
            hour: 12,
            minute: 0,
            second: 0.0,
        };
        let jd_tdb_utc = ffi_utc_to_jd_tdb(&utc, &ctx.lsk);
        let t_utc = rust_jd_tdb_to_centuries(jd_tdb_utc);
        let mut out_utc_deg = 0.0_f64;
        let mut out_utc_dpsi = 0.0_f64;
        let mut out_utc_deps = 0.0_f64;

        bench_pair(
            &mut group,
            "ayanamsha_mean_deg_utc",
            || rust_ayanamsha_mean_deg(black_box(system), black_box(t_utc)),
            || unsafe {
                expect_ok(dhruv_ffi_c::dhruv_ayanamsha_mean_deg_utc(
                    black_box(&ctx.lsk as *const _),
                    system_code,
                    black_box(&utc as *const _),
                    &mut out_utc_deg,
                ));
                out_utc_deg
            },
        );

        bench_pair(
            &mut group,
            "ayanamsha_true_deg_utc",
            || rust_ayanamsha_true_deg(black_box(system), black_box(t_utc), black_box(dpsi)),
            || unsafe {
                expect_ok(dhruv_ffi_c::dhruv_ayanamsha_true_deg_utc(
                    black_box(&ctx.lsk as *const _),
                    system_code,
                    black_box(&utc as *const _),
                    dpsi,
                    &mut out_utc_deg,
                ));
                out_utc_deg
            },
        );

        bench_pair(
            &mut group,
            "ayanamsha_deg_utc",
            || rust_ayanamsha_deg(black_box(system), black_box(t_utc), false),
            || unsafe {
                expect_ok(dhruv_ffi_c::dhruv_ayanamsha_deg_utc(
                    black_box(&ctx.lsk as *const _),
                    system_code,
                    black_box(&utc as *const _),
                    0,
                    &mut out_utc_deg,
                ));
                out_utc_deg
            },
        );

        bench_pair(
            &mut group,
            "nutation_iau2000b_utc",
            || rust_nutation_iau2000b(black_box(t_utc)).0,
            || unsafe {
                expect_ok(dhruv_ffi_c::dhruv_nutation_iau2000b_utc(
                    black_box(&ctx.lsk as *const _),
                    black_box(&utc as *const _),
                    &mut out_utc_dpsi,
                    &mut out_utc_deps,
                ));
                out_utc_dpsi
            },
        );

        bench_pair(
            &mut group,
            "lunar_node_deg_utc",
            || {
                rust_lunar_node_deg(
                    black_box(LunarNode::Rahu),
                    black_box(t_utc),
                    black_box(NodeMode::True),
                )
            },
            || unsafe {
                expect_ok(dhruv_ffi_c::dhruv_lunar_node_deg_utc(
                    black_box(&ctx.lsk as *const _),
                    0,
                    1,
                    black_box(&utc as *const _),
                    &mut out_utc_deg,
                ));
                out_utc_deg
            },
        );

        bench_pair(
            &mut group,
            "rashi_from_tropical_utc",
            || {
                rust_rashi_from_tropical(
                    black_box(tropical_lon),
                    system,
                    black_box(jd_tdb_utc),
                    false,
                )
                .rashi_index
            },
            || unsafe {
                expect_ok(dhruv_ffi_c::dhruv_rashi_from_tropical_utc(
                    black_box(&ctx.lsk as *const _),
                    tropical_lon,
                    system_code,
                    black_box(&utc as *const _),
                    0,
                    &mut out_rashi,
                ));
                out_rashi.rashi_index
            },
        );

        bench_pair(
            &mut group,
            "nakshatra_from_tropical_utc",
            || {
                rust_nakshatra_from_tropical(
                    black_box(tropical_lon),
                    system,
                    black_box(jd_tdb_utc),
                    false,
                )
                .nakshatra_index
            },
            || unsafe {
                expect_ok(dhruv_ffi_c::dhruv_nakshatra_from_tropical_utc(
                    black_box(&ctx.lsk as *const _),
                    tropical_lon,
                    system_code,
                    black_box(&utc as *const _),
                    0,
                    &mut out_nak,
                ));
                out_nak.nakshatra_index
            },
        );

        bench_pair(
            &mut group,
            "nakshatra28_from_tropical_utc",
            || {
                dhruv_vedic_base::nakshatra28_from_tropical(
                    black_box(tropical_lon),
                    system,
                    black_box(jd_tdb_utc),
                    false,
                )
                .nakshatra_index
            },
            || unsafe {
                expect_ok(dhruv_ffi_c::dhruv_nakshatra28_from_tropical_utc(
                    black_box(&ctx.lsk as *const _),
                    tropical_lon,
                    system_code,
                    black_box(&utc as *const _),
                    0,
                    &mut out_nak28,
                ));
                out_nak28.nakshatra_index
            },
        );
    }

    group.finish();
}

fn ffi_classifier_bench(c: &mut Criterion) {
    let mut group = c.benchmark_group("ffi_classifier");

    let elong = 211.75_f64;
    let mut tithi_out: DhruvTithiPosition = zeroed();
    bench_pair(
        &mut group,
        "tithi_from_elongation",
        || rust_tithi_from_elongation(black_box(elong)).tithi_index,
        || unsafe {
            let _ = dhruv_tithi_from_elongation(black_box(elong), &mut tithi_out);
            tithi_out.tithi_index
        },
    );

    let mut karana_out: DhruvKaranaPosition = zeroed();
    bench_pair(
        &mut group,
        "karana_from_elongation",
        || rust_karana_from_elongation(black_box(elong)).karana_index,
        || unsafe {
            let _ = dhruv_karana_from_elongation(black_box(elong), &mut karana_out);
            karana_out.karana_index
        },
    );

    let sum = 278.31_f64;
    let mut yoga_out: DhruvYogaPosition = zeroed();
    bench_pair(
        &mut group,
        "yoga_from_sum",
        || rust_yoga_from_sum(black_box(sum)).yoga_index,
        || unsafe {
            let _ = dhruv_yoga_from_sum(black_box(sum), &mut yoga_out);
            yoga_out.yoga_index
        },
    );

    let jd = 2_460_000.5_f64;
    bench_pair(
        &mut group,
        "vaar_from_jd",
        || rust_vaar_from_jd(black_box(jd)).index() as i32,
        || dhruv_vaar_from_jd(black_box(jd)),
    );

    let rashi_idx = 4_u32;
    bench_pair(
        &mut group,
        "masa_from_rashi_index",
        || rust_masa_from_rashi_index(black_box(rashi_idx as u8)).index() as i32,
        || dhruv_masa_from_rashi_index(black_box(rashi_idx)),
    );

    let ayana_lon = 140.0_f64;
    bench_pair(
        &mut group,
        "ayana_from_sidereal_longitude",
        || rust_ayana_from_sidereal_longitude(black_box(ayana_lon)).index() as i32,
        || dhruv_ayana_from_sidereal_longitude(black_box(ayana_lon)),
    );

    let mut samvatsara_out: DhruvSamvatsaraResult = zeroed();
    bench_pair(
        &mut group,
        "samvatsara_from_year",
        || {
            let (s, _) = rust_samvatsara_from_year(black_box(2024));
            s.index() as i32
        },
        || unsafe {
            let _ = dhruv_samvatsara_from_year(black_box(2024), &mut samvatsara_out);
            samvatsara_out.samvatsara_index
        },
    );

    bench_pair(
        &mut group,
        "nth_rashi_from",
        || rust_nth_rashi_from(black_box(2), black_box(5)) as i32,
        || dhruv_nth_rashi_from(black_box(2), black_box(5)),
    );

    bench_pair(
        &mut group,
        "rashi_lord_by_index",
        || {
            rust_rashi_lord_by_index(black_box(2))
                .map(|g| g.index() as i32)
                .unwrap_or(-1)
        },
        || dhruv_rashi_lord(black_box(2)),
    );

    group.finish();
}

fn ffi_ashtakavarga_bench(c: &mut Criterion) {
    let mut group = c.benchmark_group("ffi_ashtakavarga");

    let rashis: [u8; 7] = [0, 1, 2, 3, 4, 5, 6];
    let lagna = 0_u8;

    let mut bav_out: DhruvBhinnaAshtakavarga = zeroed();
    bench_pair(
        &mut group,
        "calculate_bav",
        || rust_calculate_bav(black_box(0), black_box(&rashis), black_box(lagna)).points[0],
        || unsafe {
            let _ = dhruv_calculate_bav(black_box(0), rashis.as_ptr(), lagna, &mut bav_out);
            bav_out.points[0]
        },
    );

    let mut bavs_out: [DhruvBhinnaAshtakavarga; 7] = zeroed();
    bench_pair(
        &mut group,
        "calculate_all_bav",
        || rust_calculate_all_bav(black_box(&rashis), black_box(lagna))[0].points[0],
        || unsafe {
            let _ = dhruv_calculate_all_bav(rashis.as_ptr(), lagna, bavs_out.as_mut_ptr());
            bavs_out[0].points[0]
        },
    );

    let rust_bavs = rust_calculate_all_bav(&rashis, lagna);
    let mut ffi_bavs: [DhruvBhinnaAshtakavarga; 7] = zeroed();
    unsafe {
        let _ = dhruv_calculate_all_bav(rashis.as_ptr(), lagna, ffi_bavs.as_mut_ptr());
    }
    let mut sav_out: DhruvSarvaAshtakavarga = zeroed();
    bench_pair(
        &mut group,
        "calculate_sav",
        || rust_calculate_sav(black_box(&rust_bavs)).total_points[0],
        || unsafe {
            let _ = dhruv_calculate_sav(ffi_bavs.as_ptr(), &mut sav_out);
            sav_out.total_points[0]
        },
    );

    let totals: [u8; 12] = [5, 3, 2, 7, 1, 6, 4, 2, 8, 3, 5, 1];
    let mut trikona_out = [0_u8; 12];
    bench_pair(
        &mut group,
        "trikona_sodhana",
        || rust_trikona_sodhana(black_box(&totals))[0],
        || unsafe {
            let _ = dhruv_trikona_sodhana(totals.as_ptr(), trikona_out.as_mut_ptr());
            trikona_out[0]
        },
    );

    let rust_trikona = rust_trikona_sodhana(&totals);
    let mut ekadhipatya_out = [0_u8; 12];
    bench_pair(
        &mut group,
        "ekadhipatya_sodhana",
        || rust_ekadhipatya_sodhana(black_box(&rust_trikona))[0],
        || unsafe {
            let _ = dhruv_ekadhipatya_sodhana(rust_trikona.as_ptr(), ekadhipatya_out.as_mut_ptr());
            ekadhipatya_out[0]
        },
    );

    group.finish();
}

fn ffi_drishti_bench(c: &mut Criterion) {
    let mut group = c.benchmark_group("ffi_drishti");

    let source = 123.45_f64;
    let target = 278.90_f64;
    let mut drishti_out: DhruvDrishtiEntry = zeroed();
    bench_pair(
        &mut group,
        "graha_drishti",
        || {
            rust_graha_drishti(black_box(Graha::Guru), black_box(source), black_box(target))
                .total_virupa
        },
        || unsafe {
            let _ = dhruv_graha_drishti(black_box(4), source, target, &mut drishti_out);
            drishti_out.total_virupa
        },
    );

    let longitudes: [f64; 9] = [10.0, 33.0, 77.0, 121.0, 166.0, 201.0, 243.0, 299.0, 119.0];
    let mut matrix_out: DhruvGrahaDrishtiMatrix = zeroed();
    bench_pair(
        &mut group,
        "graha_drishti_matrix",
        || rust_graha_drishti_matrix(black_box(&longitudes)).entries[0][1].total_virupa,
        || unsafe {
            let _ = dhruv_graha_drishti_matrix(longitudes.as_ptr(), &mut matrix_out);
            matrix_out.entries[0][1].total_virupa
        },
    );

    group.finish();
}

fn ffi_ghatika_hora_bench(c: &mut Criterion) {
    let mut group = c.benchmark_group("ffi_ghatika_hora");

    let seconds_since_sunrise = 21_600.0_f64;
    let vedic_day_duration_seconds = 86_400.0_f64;
    let mut ghatika_value: u8 = 0;
    let mut ghatika_index: u8 = 0;
    bench_pair(
        &mut group,
        "ghatika_from_elapsed",
        || {
            rust_ghatika_from_elapsed(
                black_box(seconds_since_sunrise),
                black_box(vedic_day_duration_seconds),
            )
            .index
        },
        || unsafe {
            let _ = dhruv_ghatika_from_elapsed(
                seconds_since_sunrise,
                vedic_day_duration_seconds,
                &mut ghatika_value,
                &mut ghatika_index,
            );
            ghatika_index
        },
    );

    let jd_moment = 2_460_000.75_f64;
    let jd_sunrise = 2_460_000.25_f64;
    let jd_next_sunrise = 2_460_001.25_f64;
    let mut ghatikas_out = 0.0_f64;
    bench_pair(
        &mut group,
        "ghatikas_since_sunrise",
        || {
            rust_ghatikas_since_sunrise(
                black_box(jd_moment),
                black_box(jd_sunrise),
                black_box(jd_next_sunrise),
            )
        },
        || unsafe {
            let _ = dhruv_ghatikas_since_sunrise(
                jd_moment,
                jd_sunrise,
                jd_next_sunrise,
                &mut ghatikas_out,
            );
            ghatikas_out
        },
    );

    bench_pair(
        &mut group,
        "hora_at",
        || rust_hora_at(black_box(Vaar::Ravivaar), black_box(11)).index() as i32,
        || dhruv_hora_at(black_box(0), black_box(11)),
    );

    group.finish();
}

fn ffi_upagraha_bench(c: &mut Criterion) {
    let mut group = c.benchmark_group("ffi_upagraha");

    let sunrise_jd = 2_460_000.25_f64;
    let sunset_jd = 2_460_000.75_f64;
    let next_sunrise_jd = 2_460_001.25_f64;
    let mut out_jd = 0.0_f64;

    bench_pair(
        &mut group,
        "time_upagraha_jd",
        || {
            rust_time_upagraha_jd(
                black_box(Upagraha::Gulika),
                black_box(0),
                black_box(true),
                black_box(sunrise_jd),
                black_box(sunset_jd),
                black_box(next_sunrise_jd),
            )
        },
        || unsafe {
            let _ = dhruv_time_upagraha_jd(
                black_box(0),
                black_box(0),
                black_box(1),
                black_box(sunrise_jd),
                black_box(sunset_jd),
                black_box(next_sunrise_jd),
                &mut out_jd,
            );
            out_jd
        },
    );

    group.finish();
}

fn ffi_search_sidereal_bench(c: &mut Criterion) {
    let ctx = match load_context() {
        Some(v) => v,
        None => return,
    };

    let jd = 2_460_000.5_f64;
    let cfg_rust = SankrantiConfig::new(AyanamshaSystem::Lahiri, false);
    let cfg_ffi = dhruv_sankranti_config_default();
    let ayanamsha_code = cfg_ffi.ayanamsha_system as u32;

    let moon_lon = rust_graha_sidereal_longitudes(&ctx.engine, jd, AyanamshaSystem::Lahiri, false)
        .expect("graha sidereal longitudes should succeed")
        .longitudes[1];

    let mut group = c.benchmark_group("ffi_search_sidereal");
    group.sample_size(20);

    let mut longitudes_out: DhruvGrahaLongitudes = zeroed();
    bench_pair(
        &mut group,
        "graha_sidereal_longitudes",
        || {
            rust_graha_sidereal_longitudes(
                black_box(&ctx.engine),
                black_box(jd),
                AyanamshaSystem::Lahiri,
                false,
            )
            .expect("graha sidereal longitudes should succeed")
            .longitudes[1]
        },
        || unsafe {
            let _ = dhruv_graha_sidereal_longitudes(
                black_box(&ctx.engine as *const _),
                black_box(jd),
                black_box(ayanamsha_code),
                black_box(0),
                &mut longitudes_out,
            );
            longitudes_out.longitudes[1]
        },
    );

    let mut nakshatra_out: DhruvPanchangNakshatraInfo = zeroed();
    bench_pair(
        &mut group,
        "nakshatra_at",
        || {
            rust_nakshatra_at(
                black_box(&ctx.engine),
                black_box(jd),
                black_box(moon_lon),
                black_box(&cfg_rust),
            )
            .expect("nakshatra_at should succeed")
            .nakshatra_index
        },
        || unsafe {
            let _ = dhruv_nakshatra_at(
                black_box(&ctx.engine as *const _),
                black_box(jd),
                black_box(moon_lon),
                black_box(&cfg_ffi as *const _),
                &mut nakshatra_out,
            );
            nakshatra_out.nakshatra_index
        },
    );

    group.finish();
}

fn ffi_search_conjunction_grahan_bench(c: &mut Criterion) {
    let ctx = match load_context() {
        Some(v) => v,
        None => return,
    };

    let mut group = c.benchmark_group("ffi_search_conjunction_grahan");
    group.sample_size(10);

    let jd = 2_460_000.5_f64;
    let jd_end = 2_460_400.5_f64;

    let conj_cfg_rust = dhruv_search::ConjunctionConfig {
        target_separation_deg: 0.0,
        step_size_days: 0.5,
        max_iterations: 50,
        convergence_days: 1e-8,
    };
    let conj_cfg_ffi = dhruv_ffi_c::dhruv_conjunction_config_default();

    let mut conj_out: dhruv_ffi_c::DhruvConjunctionEvent = zeroed();
    let mut found_u8 = 0_u8;
    bench_pair(
        &mut group,
        "next_conjunction",
        || {
            dhruv_search::next_conjunction(
                black_box(&ctx.engine),
                Body::Sun,
                Body::Moon,
                black_box(jd),
                black_box(&conj_cfg_rust),
            )
            .expect("next conjunction")
            .map(|e| e.jd_tdb)
            .unwrap_or(-1.0)
        },
        || unsafe {
            expect_ok(dhruv_ffi_c::dhruv_next_conjunction(
                black_box(&ctx.engine as *const _),
                Body::Sun.code(),
                Body::Moon.code(),
                jd,
                black_box(&conj_cfg_ffi as *const _),
                &mut conj_out,
                &mut found_u8,
            ));
            if found_u8 == 0 { -1.0 } else { conj_out.jd_tdb }
        },
    );

    bench_pair(
        &mut group,
        "prev_conjunction",
        || {
            dhruv_search::prev_conjunction(
                black_box(&ctx.engine),
                Body::Sun,
                Body::Moon,
                black_box(jd),
                black_box(&conj_cfg_rust),
            )
            .expect("prev conjunction")
            .map(|e| e.jd_tdb)
            .unwrap_or(-1.0)
        },
        || unsafe {
            expect_ok(dhruv_ffi_c::dhruv_prev_conjunction(
                black_box(&ctx.engine as *const _),
                Body::Sun.code(),
                Body::Moon.code(),
                jd,
                black_box(&conj_cfg_ffi as *const _),
                &mut conj_out,
                &mut found_u8,
            ));
            if found_u8 == 0 { -1.0 } else { conj_out.jd_tdb }
        },
    );

    let mut conj_outs: [dhruv_ffi_c::DhruvConjunctionEvent; 32] = zeroed();
    let mut out_count = 0_u32;
    bench_pair(
        &mut group,
        "search_conjunctions",
        || {
            dhruv_search::search_conjunctions(
                black_box(&ctx.engine),
                Body::Sun,
                Body::Moon,
                black_box(jd),
                black_box(jd_end),
                black_box(&conj_cfg_rust),
            )
            .expect("search conjunctions")
            .len() as f64
        },
        || unsafe {
            expect_ok(dhruv_ffi_c::dhruv_search_conjunctions(
                black_box(&ctx.engine as *const _),
                Body::Sun.code(),
                Body::Moon.code(),
                jd,
                jd_end,
                black_box(&conj_cfg_ffi as *const _),
                conj_outs.as_mut_ptr(),
                32,
                &mut out_count,
            ));
            out_count as f64
        },
    );

    let grahan_cfg_rust = dhruv_search::GrahanConfig::default();
    let grahan_cfg_ffi = dhruv_ffi_c::dhruv_grahan_config_default();

    let mut chandra_out: dhruv_ffi_c::DhruvChandraGrahanResult = zeroed();
    bench_pair(
        &mut group,
        "next_chandra_grahan",
        || {
            dhruv_search::next_chandra_grahan(
                black_box(&ctx.engine),
                black_box(jd),
                black_box(&grahan_cfg_rust),
            )
            .expect("next chandra grahan")
            .map(|e| e.greatest_grahan_jd)
            .unwrap_or(-1.0)
        },
        || unsafe {
            expect_ok(dhruv_ffi_c::dhruv_next_chandra_grahan(
                black_box(&ctx.engine as *const _),
                jd,
                black_box(&grahan_cfg_ffi as *const _),
                &mut chandra_out,
                &mut found_u8,
            ));
            if found_u8 == 0 {
                -1.0
            } else {
                chandra_out.greatest_grahan_jd
            }
        },
    );

    bench_pair(
        &mut group,
        "prev_chandra_grahan",
        || {
            dhruv_search::prev_chandra_grahan(
                black_box(&ctx.engine),
                black_box(jd),
                black_box(&grahan_cfg_rust),
            )
            .expect("prev chandra grahan")
            .map(|e| e.greatest_grahan_jd)
            .unwrap_or(-1.0)
        },
        || unsafe {
            expect_ok(dhruv_ffi_c::dhruv_prev_chandra_grahan(
                black_box(&ctx.engine as *const _),
                jd,
                black_box(&grahan_cfg_ffi as *const _),
                &mut chandra_out,
                &mut found_u8,
            ));
            if found_u8 == 0 {
                -1.0
            } else {
                chandra_out.greatest_grahan_jd
            }
        },
    );

    let mut chandra_outs: [dhruv_ffi_c::DhruvChandraGrahanResult; 16] = zeroed();
    bench_pair(
        &mut group,
        "search_chandra_grahan",
        || {
            dhruv_search::search_chandra_grahan(
                black_box(&ctx.engine),
                black_box(jd),
                black_box(jd_end),
                black_box(&grahan_cfg_rust),
            )
            .expect("search chandra grahan")
            .len() as f64
        },
        || unsafe {
            expect_ok(dhruv_ffi_c::dhruv_search_chandra_grahan(
                black_box(&ctx.engine as *const _),
                jd,
                jd_end,
                black_box(&grahan_cfg_ffi as *const _),
                chandra_outs.as_mut_ptr(),
                16,
                &mut out_count,
            ));
            out_count as f64
        },
    );

    let mut surya_out: dhruv_ffi_c::DhruvSuryaGrahanResult = zeroed();
    bench_pair(
        &mut group,
        "next_surya_grahan",
        || {
            dhruv_search::next_surya_grahan(
                black_box(&ctx.engine),
                black_box(jd),
                black_box(&grahan_cfg_rust),
            )
            .expect("next surya grahan")
            .map(|e| e.greatest_grahan_jd)
            .unwrap_or(-1.0)
        },
        || unsafe {
            expect_ok(dhruv_ffi_c::dhruv_next_surya_grahan(
                black_box(&ctx.engine as *const _),
                jd,
                black_box(&grahan_cfg_ffi as *const _),
                &mut surya_out,
                &mut found_u8,
            ));
            if found_u8 == 0 {
                -1.0
            } else {
                surya_out.greatest_grahan_jd
            }
        },
    );

    bench_pair(
        &mut group,
        "prev_surya_grahan",
        || {
            dhruv_search::prev_surya_grahan(
                black_box(&ctx.engine),
                black_box(jd),
                black_box(&grahan_cfg_rust),
            )
            .expect("prev surya grahan")
            .map(|e| e.greatest_grahan_jd)
            .unwrap_or(-1.0)
        },
        || unsafe {
            expect_ok(dhruv_ffi_c::dhruv_prev_surya_grahan(
                black_box(&ctx.engine as *const _),
                jd,
                black_box(&grahan_cfg_ffi as *const _),
                &mut surya_out,
                &mut found_u8,
            ));
            if found_u8 == 0 {
                -1.0
            } else {
                surya_out.greatest_grahan_jd
            }
        },
    );

    let mut surya_outs: [dhruv_ffi_c::DhruvSuryaGrahanResult; 16] = zeroed();
    bench_pair(
        &mut group,
        "search_surya_grahan",
        || {
            dhruv_search::search_surya_grahan(
                black_box(&ctx.engine),
                black_box(jd),
                black_box(jd_end),
                black_box(&grahan_cfg_rust),
            )
            .expect("search surya grahan")
            .len() as f64
        },
        || unsafe {
            expect_ok(dhruv_ffi_c::dhruv_search_surya_grahan(
                black_box(&ctx.engine as *const _),
                jd,
                jd_end,
                black_box(&grahan_cfg_ffi as *const _),
                surya_outs.as_mut_ptr(),
                16,
                &mut out_count,
            ));
            out_count as f64
        },
    );

    let utc = DhruvUtcTime {
        year: 2024,
        month: 1,
        day: 1,
        hour: 0,
        minute: 0,
        second: 0.0,
    };
    let utc_end = DhruvUtcTime {
        year: 2025,
        month: 1,
        day: 1,
        hour: 0,
        minute: 0,
        second: 0.0,
    };
    let jd_utc_start_tdb = ffi_utc_to_jd_tdb(&utc, &ctx.lsk);
    let jd_utc_end_tdb = ffi_utc_to_jd_tdb(&utc_end, &ctx.lsk);

    let mut conj_out_utc: dhruv_ffi_c::DhruvConjunctionEventUtc = zeroed();
    bench_pair(
        &mut group,
        "next_conjunction_utc",
        || {
            dhruv_search::next_conjunction(
                black_box(&ctx.engine),
                Body::Sun,
                Body::Moon,
                black_box(jd_utc_start_tdb),
                black_box(&conj_cfg_rust),
            )
            .expect("next conjunction utc-rust")
            .map(|e| e.jd_tdb)
            .unwrap_or(-1.0)
        },
        || unsafe {
            expect_ok(dhruv_ffi_c::dhruv_next_conjunction_utc(
                black_box(&ctx.engine as *const _),
                Body::Sun.code(),
                Body::Moon.code(),
                black_box(&utc as *const _),
                black_box(&conj_cfg_ffi as *const _),
                &mut conj_out_utc,
                &mut found_u8,
            ));
            if found_u8 == 0 {
                -1.0
            } else {
                ffi_utc_to_jd_tdb(&conj_out_utc.utc, &ctx.lsk)
            }
        },
    );

    bench_pair(
        &mut group,
        "prev_conjunction_utc",
        || {
            dhruv_search::prev_conjunction(
                black_box(&ctx.engine),
                Body::Sun,
                Body::Moon,
                black_box(jd_utc_start_tdb),
                black_box(&conj_cfg_rust),
            )
            .expect("prev conjunction utc-rust")
            .map(|e| e.jd_tdb)
            .unwrap_or(-1.0)
        },
        || unsafe {
            expect_ok(dhruv_ffi_c::dhruv_prev_conjunction_utc(
                black_box(&ctx.engine as *const _),
                Body::Sun.code(),
                Body::Moon.code(),
                black_box(&utc as *const _),
                black_box(&conj_cfg_ffi as *const _),
                &mut conj_out_utc,
                &mut found_u8,
            ));
            if found_u8 == 0 {
                -1.0
            } else {
                ffi_utc_to_jd_tdb(&conj_out_utc.utc, &ctx.lsk)
            }
        },
    );

    let mut conj_outs_utc: [dhruv_ffi_c::DhruvConjunctionEventUtc; 32] = zeroed();
    bench_pair(
        &mut group,
        "search_conjunctions_utc",
        || {
            dhruv_search::search_conjunctions(
                black_box(&ctx.engine),
                Body::Sun,
                Body::Moon,
                black_box(jd_utc_start_tdb),
                black_box(jd_utc_end_tdb),
                black_box(&conj_cfg_rust),
            )
            .expect("search conjunctions utc-rust")
            .len() as f64
        },
        || unsafe {
            expect_ok(dhruv_ffi_c::dhruv_search_conjunctions_utc(
                black_box(&ctx.engine as *const _),
                Body::Sun.code(),
                Body::Moon.code(),
                black_box(&utc as *const _),
                black_box(&utc_end as *const _),
                black_box(&conj_cfg_ffi as *const _),
                conj_outs_utc.as_mut_ptr(),
                32,
                &mut out_count,
            ));
            out_count as f64
        },
    );

    let mut chandra_out_utc: dhruv_ffi_c::DhruvChandraGrahanResultUtc = zeroed();
    bench_pair(
        &mut group,
        "next_chandra_grahan_utc",
        || {
            dhruv_search::next_chandra_grahan(
                black_box(&ctx.engine),
                black_box(jd_utc_start_tdb),
                black_box(&grahan_cfg_rust),
            )
            .expect("next chandra grahan utc-rust")
            .map(|e| e.greatest_grahan_jd)
            .unwrap_or(-1.0)
        },
        || unsafe {
            expect_ok(dhruv_ffi_c::dhruv_next_chandra_grahan_utc(
                black_box(&ctx.engine as *const _),
                black_box(&utc as *const _),
                black_box(&grahan_cfg_ffi as *const _),
                &mut chandra_out_utc,
                &mut found_u8,
            ));
            if found_u8 == 0 {
                -1.0
            } else {
                ffi_utc_to_jd_tdb(&chandra_out_utc.greatest_grahan, &ctx.lsk)
            }
        },
    );

    bench_pair(
        &mut group,
        "prev_chandra_grahan_utc",
        || {
            dhruv_search::prev_chandra_grahan(
                black_box(&ctx.engine),
                black_box(jd_utc_start_tdb),
                black_box(&grahan_cfg_rust),
            )
            .expect("prev chandra grahan utc-rust")
            .map(|e| e.greatest_grahan_jd)
            .unwrap_or(-1.0)
        },
        || unsafe {
            expect_ok(dhruv_ffi_c::dhruv_prev_chandra_grahan_utc(
                black_box(&ctx.engine as *const _),
                black_box(&utc as *const _),
                black_box(&grahan_cfg_ffi as *const _),
                &mut chandra_out_utc,
                &mut found_u8,
            ));
            if found_u8 == 0 {
                -1.0
            } else {
                ffi_utc_to_jd_tdb(&chandra_out_utc.greatest_grahan, &ctx.lsk)
            }
        },
    );

    let mut chandra_outs_utc: [dhruv_ffi_c::DhruvChandraGrahanResultUtc; 16] = zeroed();
    bench_pair(
        &mut group,
        "search_chandra_grahan_utc",
        || {
            dhruv_search::search_chandra_grahan(
                black_box(&ctx.engine),
                black_box(jd_utc_start_tdb),
                black_box(jd_utc_end_tdb),
                black_box(&grahan_cfg_rust),
            )
            .expect("search chandra grahan utc-rust")
            .len() as f64
        },
        || unsafe {
            expect_ok(dhruv_ffi_c::dhruv_search_chandra_grahan_utc(
                black_box(&ctx.engine as *const _),
                black_box(&utc as *const _),
                black_box(&utc_end as *const _),
                black_box(&grahan_cfg_ffi as *const _),
                chandra_outs_utc.as_mut_ptr(),
                16,
                &mut out_count,
            ));
            out_count as f64
        },
    );

    let mut surya_out_utc: dhruv_ffi_c::DhruvSuryaGrahanResultUtc = zeroed();
    bench_pair(
        &mut group,
        "next_surya_grahan_utc",
        || {
            dhruv_search::next_surya_grahan(
                black_box(&ctx.engine),
                black_box(jd_utc_start_tdb),
                black_box(&grahan_cfg_rust),
            )
            .expect("next surya grahan utc-rust")
            .map(|e| e.greatest_grahan_jd)
            .unwrap_or(-1.0)
        },
        || unsafe {
            expect_ok(dhruv_ffi_c::dhruv_next_surya_grahan_utc(
                black_box(&ctx.engine as *const _),
                black_box(&utc as *const _),
                black_box(&grahan_cfg_ffi as *const _),
                &mut surya_out_utc,
                &mut found_u8,
            ));
            if found_u8 == 0 {
                -1.0
            } else {
                ffi_utc_to_jd_tdb(&surya_out_utc.greatest_grahan, &ctx.lsk)
            }
        },
    );

    bench_pair(
        &mut group,
        "prev_surya_grahan_utc",
        || {
            dhruv_search::prev_surya_grahan(
                black_box(&ctx.engine),
                black_box(jd_utc_start_tdb),
                black_box(&grahan_cfg_rust),
            )
            .expect("prev surya grahan utc-rust")
            .map(|e| e.greatest_grahan_jd)
            .unwrap_or(-1.0)
        },
        || unsafe {
            expect_ok(dhruv_ffi_c::dhruv_prev_surya_grahan_utc(
                black_box(&ctx.engine as *const _),
                black_box(&utc as *const _),
                black_box(&grahan_cfg_ffi as *const _),
                &mut surya_out_utc,
                &mut found_u8,
            ));
            if found_u8 == 0 {
                -1.0
            } else {
                ffi_utc_to_jd_tdb(&surya_out_utc.greatest_grahan, &ctx.lsk)
            }
        },
    );

    let mut surya_outs_utc: [dhruv_ffi_c::DhruvSuryaGrahanResultUtc; 16] = zeroed();
    bench_pair(
        &mut group,
        "search_surya_grahan_utc",
        || {
            dhruv_search::search_surya_grahan(
                black_box(&ctx.engine),
                black_box(jd_utc_start_tdb),
                black_box(jd_utc_end_tdb),
                black_box(&grahan_cfg_rust),
            )
            .expect("search surya grahan utc-rust")
            .len() as f64
        },
        || unsafe {
            expect_ok(dhruv_ffi_c::dhruv_search_surya_grahan_utc(
                black_box(&ctx.engine as *const _),
                black_box(&utc as *const _),
                black_box(&utc_end as *const _),
                black_box(&grahan_cfg_ffi as *const _),
                surya_outs_utc.as_mut_ptr(),
                16,
                &mut out_count,
            ));
            out_count as f64
        },
    );

    group.finish();
}

fn ffi_search_stationary_bench(c: &mut Criterion) {
    let ctx = match load_context() {
        Some(v) => v,
        None => return,
    };

    let mut group = c.benchmark_group("ffi_search_stationary");
    group.sample_size(10);

    let body = Body::Mercury;
    let jd = 2_460_000.5_f64;
    let jd_end = 2_460_400.5_f64;
    let cfg_rust = dhruv_search::StationaryConfig::inner_planet();
    let cfg_ffi = dhruv_ffi_c::dhruv_stationary_config_default();

    let mut found_u8 = 0_u8;
    let mut out_stationary: dhruv_ffi_c::DhruvStationaryEvent = zeroed();
    bench_pair(
        &mut group,
        "next_stationary",
        || {
            dhruv_search::next_stationary(
                black_box(&ctx.engine),
                body,
                black_box(jd),
                black_box(&cfg_rust),
            )
            .expect("next stationary")
            .map(|e| e.jd_tdb)
            .unwrap_or(-1.0)
        },
        || unsafe {
            expect_ok(dhruv_ffi_c::dhruv_next_stationary(
                black_box(&ctx.engine as *const _),
                body.code(),
                jd,
                black_box(&cfg_ffi as *const _),
                &mut out_stationary,
                &mut found_u8,
            ));
            if found_u8 == 0 {
                -1.0
            } else {
                out_stationary.jd_tdb
            }
        },
    );

    bench_pair(
        &mut group,
        "prev_stationary",
        || {
            dhruv_search::prev_stationary(
                black_box(&ctx.engine),
                body,
                black_box(jd),
                black_box(&cfg_rust),
            )
            .expect("prev stationary")
            .map(|e| e.jd_tdb)
            .unwrap_or(-1.0)
        },
        || unsafe {
            expect_ok(dhruv_ffi_c::dhruv_prev_stationary(
                black_box(&ctx.engine as *const _),
                body.code(),
                jd,
                black_box(&cfg_ffi as *const _),
                &mut out_stationary,
                &mut found_u8,
            ));
            if found_u8 == 0 {
                -1.0
            } else {
                out_stationary.jd_tdb
            }
        },
    );

    let mut out_stationary_events: [dhruv_ffi_c::DhruvStationaryEvent; 32] = zeroed();
    let mut out_count = 0_u32;
    bench_pair(
        &mut group,
        "search_stationary",
        || {
            dhruv_search::search_stationary(
                black_box(&ctx.engine),
                body,
                black_box(jd),
                black_box(jd_end),
                black_box(&cfg_rust),
            )
            .expect("search stationary")
            .len() as f64
        },
        || unsafe {
            expect_ok(dhruv_ffi_c::dhruv_search_stationary(
                black_box(&ctx.engine as *const _),
                body.code(),
                jd,
                jd_end,
                black_box(&cfg_ffi as *const _),
                out_stationary_events.as_mut_ptr(),
                32,
                &mut out_count,
            ));
            out_count as f64
        },
    );

    let mut out_max_speed: dhruv_ffi_c::DhruvMaxSpeedEvent = zeroed();
    bench_pair(
        &mut group,
        "next_max_speed",
        || {
            dhruv_search::next_max_speed(
                black_box(&ctx.engine),
                body,
                black_box(jd),
                black_box(&cfg_rust),
            )
            .expect("next max speed")
            .map(|e| e.jd_tdb)
            .unwrap_or(-1.0)
        },
        || unsafe {
            expect_ok(dhruv_ffi_c::dhruv_next_max_speed(
                black_box(&ctx.engine as *const _),
                body.code(),
                jd,
                black_box(&cfg_ffi as *const _),
                &mut out_max_speed,
                &mut found_u8,
            ));
            if found_u8 == 0 {
                -1.0
            } else {
                out_max_speed.jd_tdb
            }
        },
    );

    bench_pair(
        &mut group,
        "prev_max_speed",
        || {
            dhruv_search::prev_max_speed(
                black_box(&ctx.engine),
                body,
                black_box(jd),
                black_box(&cfg_rust),
            )
            .expect("prev max speed")
            .map(|e| e.jd_tdb)
            .unwrap_or(-1.0)
        },
        || unsafe {
            expect_ok(dhruv_ffi_c::dhruv_prev_max_speed(
                black_box(&ctx.engine as *const _),
                body.code(),
                jd,
                black_box(&cfg_ffi as *const _),
                &mut out_max_speed,
                &mut found_u8,
            ));
            if found_u8 == 0 {
                -1.0
            } else {
                out_max_speed.jd_tdb
            }
        },
    );

    let mut out_max_speed_events: [dhruv_ffi_c::DhruvMaxSpeedEvent; 32] = zeroed();
    bench_pair(
        &mut group,
        "search_max_speed",
        || {
            dhruv_search::search_max_speed(
                black_box(&ctx.engine),
                body,
                black_box(jd),
                black_box(jd_end),
                black_box(&cfg_rust),
            )
            .expect("search max speed")
            .len() as f64
        },
        || unsafe {
            expect_ok(dhruv_ffi_c::dhruv_search_max_speed(
                black_box(&ctx.engine as *const _),
                body.code(),
                jd,
                jd_end,
                black_box(&cfg_ffi as *const _),
                out_max_speed_events.as_mut_ptr(),
                32,
                &mut out_count,
            ));
            out_count as f64
        },
    );

    let utc = DhruvUtcTime {
        year: 2024,
        month: 1,
        day: 1,
        hour: 0,
        minute: 0,
        second: 0.0,
    };
    let utc_end = DhruvUtcTime {
        year: 2025,
        month: 1,
        day: 1,
        hour: 0,
        minute: 0,
        second: 0.0,
    };
    let jd_utc_start_tdb = ffi_utc_to_jd_tdb(&utc, &ctx.lsk);
    let jd_utc_end_tdb = ffi_utc_to_jd_tdb(&utc_end, &ctx.lsk);

    let mut out_stationary_utc: dhruv_ffi_c::DhruvStationaryEventUtc = zeroed();
    bench_pair(
        &mut group,
        "next_stationary_utc",
        || {
            dhruv_search::next_stationary(
                black_box(&ctx.engine),
                body,
                black_box(jd_utc_start_tdb),
                black_box(&cfg_rust),
            )
            .expect("next stationary utc-rust")
            .map(|e| e.jd_tdb)
            .unwrap_or(-1.0)
        },
        || unsafe {
            expect_ok(dhruv_ffi_c::dhruv_next_stationary_utc(
                black_box(&ctx.engine as *const _),
                body.code(),
                black_box(&utc as *const _),
                black_box(&cfg_ffi as *const _),
                &mut out_stationary_utc,
                &mut found_u8,
            ));
            if found_u8 == 0 {
                -1.0
            } else {
                ffi_utc_to_jd_tdb(&out_stationary_utc.utc, &ctx.lsk)
            }
        },
    );

    bench_pair(
        &mut group,
        "prev_stationary_utc",
        || {
            dhruv_search::prev_stationary(
                black_box(&ctx.engine),
                body,
                black_box(jd_utc_start_tdb),
                black_box(&cfg_rust),
            )
            .expect("prev stationary utc-rust")
            .map(|e| e.jd_tdb)
            .unwrap_or(-1.0)
        },
        || unsafe {
            expect_ok(dhruv_ffi_c::dhruv_prev_stationary_utc(
                black_box(&ctx.engine as *const _),
                body.code(),
                black_box(&utc as *const _),
                black_box(&cfg_ffi as *const _),
                &mut out_stationary_utc,
                &mut found_u8,
            ));
            if found_u8 == 0 {
                -1.0
            } else {
                ffi_utc_to_jd_tdb(&out_stationary_utc.utc, &ctx.lsk)
            }
        },
    );

    let mut out_stationary_events_utc: [dhruv_ffi_c::DhruvStationaryEventUtc; 32] = zeroed();
    bench_pair(
        &mut group,
        "search_stationary_utc",
        || {
            dhruv_search::search_stationary(
                black_box(&ctx.engine),
                body,
                black_box(jd_utc_start_tdb),
                black_box(jd_utc_end_tdb),
                black_box(&cfg_rust),
            )
            .expect("search stationary utc-rust")
            .len() as f64
        },
        || unsafe {
            expect_ok(dhruv_ffi_c::dhruv_search_stationary_utc(
                black_box(&ctx.engine as *const _),
                body.code(),
                black_box(&utc as *const _),
                black_box(&utc_end as *const _),
                black_box(&cfg_ffi as *const _),
                out_stationary_events_utc.as_mut_ptr(),
                32,
                &mut out_count,
            ));
            out_count as f64
        },
    );

    let mut out_max_speed_utc: dhruv_ffi_c::DhruvMaxSpeedEventUtc = zeroed();
    bench_pair(
        &mut group,
        "next_max_speed_utc",
        || {
            dhruv_search::next_max_speed(
                black_box(&ctx.engine),
                body,
                black_box(jd_utc_start_tdb),
                black_box(&cfg_rust),
            )
            .expect("next max speed utc-rust")
            .map(|e| e.jd_tdb)
            .unwrap_or(-1.0)
        },
        || unsafe {
            expect_ok(dhruv_ffi_c::dhruv_next_max_speed_utc(
                black_box(&ctx.engine as *const _),
                body.code(),
                black_box(&utc as *const _),
                black_box(&cfg_ffi as *const _),
                &mut out_max_speed_utc,
                &mut found_u8,
            ));
            if found_u8 == 0 {
                -1.0
            } else {
                ffi_utc_to_jd_tdb(&out_max_speed_utc.utc, &ctx.lsk)
            }
        },
    );

    bench_pair(
        &mut group,
        "prev_max_speed_utc",
        || {
            dhruv_search::prev_max_speed(
                black_box(&ctx.engine),
                body,
                black_box(jd_utc_start_tdb),
                black_box(&cfg_rust),
            )
            .expect("prev max speed utc-rust")
            .map(|e| e.jd_tdb)
            .unwrap_or(-1.0)
        },
        || unsafe {
            expect_ok(dhruv_ffi_c::dhruv_prev_max_speed_utc(
                black_box(&ctx.engine as *const _),
                body.code(),
                black_box(&utc as *const _),
                black_box(&cfg_ffi as *const _),
                &mut out_max_speed_utc,
                &mut found_u8,
            ));
            if found_u8 == 0 {
                -1.0
            } else {
                ffi_utc_to_jd_tdb(&out_max_speed_utc.utc, &ctx.lsk)
            }
        },
    );

    let mut out_max_speed_events_utc: [dhruv_ffi_c::DhruvMaxSpeedEventUtc; 32] = zeroed();
    bench_pair(
        &mut group,
        "search_max_speed_utc",
        || {
            dhruv_search::search_max_speed(
                black_box(&ctx.engine),
                body,
                black_box(jd_utc_start_tdb),
                black_box(jd_utc_end_tdb),
                black_box(&cfg_rust),
            )
            .expect("search max speed utc-rust")
            .len() as f64
        },
        || unsafe {
            expect_ok(dhruv_ffi_c::dhruv_search_max_speed_utc(
                black_box(&ctx.engine as *const _),
                body.code(),
                black_box(&utc as *const _),
                black_box(&utc_end as *const _),
                black_box(&cfg_ffi as *const _),
                out_max_speed_events_utc.as_mut_ptr(),
                32,
                &mut out_count,
            ));
            out_count as f64
        },
    );

    group.finish();
}

fn ffi_lunar_sankranti_bench(c: &mut Criterion) {
    let ctx = match load_context() {
        Some(v) => v,
        None => return,
    };

    let mut group = c.benchmark_group("ffi_lunar_sankranti");
    group.sample_size(10);

    let utc = DhruvUtcTime {
        year: 2024,
        month: 1,
        day: 1,
        hour: 0,
        minute: 0,
        second: 0.0,
    };
    let utc_end = DhruvUtcTime {
        year: 2025,
        month: 1,
        day: 1,
        hour: 0,
        minute: 0,
        second: 0.0,
    };
    let utc_rust = ffi_utc_to_rust(&utc);
    let utc_end_rust = ffi_utc_to_rust(&utc_end);

    let mut found_u8 = 0_u8;
    let mut lunar_out: dhruv_ffi_c::DhruvLunarPhaseEvent = zeroed();
    let mut out_count = 0_u32;

    bench_pair(
        &mut group,
        "next_purnima",
        || {
            dhruv_search::next_purnima(black_box(&ctx.engine), black_box(&utc_rust))
                .expect("next purnima")
                .map(|e| e.utc.to_jd_tdb(ctx.engine.lsk()))
                .unwrap_or(-1.0)
        },
        || unsafe {
            expect_ok(dhruv_ffi_c::dhruv_next_purnima(
                black_box(&ctx.engine as *const _),
                black_box(&utc as *const _),
                &mut lunar_out,
                &mut found_u8,
            ));
            if found_u8 == 0 {
                -1.0
            } else {
                ffi_utc_to_jd_tdb(&lunar_out.utc, &ctx.lsk)
            }
        },
    );

    bench_pair(
        &mut group,
        "prev_purnima",
        || {
            dhruv_search::prev_purnima(black_box(&ctx.engine), black_box(&utc_rust))
                .expect("prev purnima")
                .map(|e| e.utc.to_jd_tdb(ctx.engine.lsk()))
                .unwrap_or(-1.0)
        },
        || unsafe {
            expect_ok(dhruv_ffi_c::dhruv_prev_purnima(
                black_box(&ctx.engine as *const _),
                black_box(&utc as *const _),
                &mut lunar_out,
                &mut found_u8,
            ));
            if found_u8 == 0 {
                -1.0
            } else {
                ffi_utc_to_jd_tdb(&lunar_out.utc, &ctx.lsk)
            }
        },
    );

    let mut lunar_outs: [dhruv_ffi_c::DhruvLunarPhaseEvent; 32] = zeroed();
    bench_pair(
        &mut group,
        "search_purnimas",
        || {
            dhruv_search::search_purnimas(
                black_box(&ctx.engine),
                black_box(&utc_rust),
                black_box(&utc_end_rust),
            )
            .expect("search purnimas")
            .len() as f64
        },
        || unsafe {
            expect_ok(dhruv_ffi_c::dhruv_search_purnimas(
                black_box(&ctx.engine as *const _),
                black_box(&utc as *const _),
                black_box(&utc_end as *const _),
                lunar_outs.as_mut_ptr(),
                32,
                &mut out_count,
            ));
            out_count as f64
        },
    );

    bench_pair(
        &mut group,
        "next_amavasya",
        || {
            dhruv_search::next_amavasya(black_box(&ctx.engine), black_box(&utc_rust))
                .expect("next amavasya")
                .map(|e| e.utc.to_jd_tdb(ctx.engine.lsk()))
                .unwrap_or(-1.0)
        },
        || unsafe {
            expect_ok(dhruv_ffi_c::dhruv_next_amavasya(
                black_box(&ctx.engine as *const _),
                black_box(&utc as *const _),
                &mut lunar_out,
                &mut found_u8,
            ));
            if found_u8 == 0 {
                -1.0
            } else {
                ffi_utc_to_jd_tdb(&lunar_out.utc, &ctx.lsk)
            }
        },
    );

    bench_pair(
        &mut group,
        "prev_amavasya",
        || {
            dhruv_search::prev_amavasya(black_box(&ctx.engine), black_box(&utc_rust))
                .expect("prev amavasya")
                .map(|e| e.utc.to_jd_tdb(ctx.engine.lsk()))
                .unwrap_or(-1.0)
        },
        || unsafe {
            expect_ok(dhruv_ffi_c::dhruv_prev_amavasya(
                black_box(&ctx.engine as *const _),
                black_box(&utc as *const _),
                &mut lunar_out,
                &mut found_u8,
            ));
            if found_u8 == 0 {
                -1.0
            } else {
                ffi_utc_to_jd_tdb(&lunar_out.utc, &ctx.lsk)
            }
        },
    );

    bench_pair(
        &mut group,
        "search_amavasyas",
        || {
            dhruv_search::search_amavasyas(
                black_box(&ctx.engine),
                black_box(&utc_rust),
                black_box(&utc_end_rust),
            )
            .expect("search amavasyas")
            .len() as f64
        },
        || unsafe {
            expect_ok(dhruv_ffi_c::dhruv_search_amavasyas(
                black_box(&ctx.engine as *const _),
                black_box(&utc as *const _),
                black_box(&utc_end as *const _),
                lunar_outs.as_mut_ptr(),
                32,
                &mut out_count,
            ));
            out_count as f64
        },
    );

    let sankranti_cfg_rust = SankrantiConfig::new(AyanamshaSystem::Lahiri, false);
    let sankranti_cfg_ffi = dhruv_sankranti_config_default();
    let mut sankranti_out: dhruv_ffi_c::DhruvSankrantiEvent = zeroed();

    bench_pair(
        &mut group,
        "next_sankranti",
        || {
            dhruv_search::next_sankranti(
                black_box(&ctx.engine),
                black_box(&utc_rust),
                black_box(&sankranti_cfg_rust),
            )
            .expect("next sankranti")
            .map(|e| e.utc.to_jd_tdb(ctx.engine.lsk()))
            .unwrap_or(-1.0)
        },
        || unsafe {
            expect_ok(dhruv_ffi_c::dhruv_next_sankranti(
                black_box(&ctx.engine as *const _),
                black_box(&utc as *const _),
                black_box(&sankranti_cfg_ffi as *const _),
                &mut sankranti_out,
                &mut found_u8,
            ));
            if found_u8 == 0 {
                -1.0
            } else {
                ffi_utc_to_jd_tdb(&sankranti_out.utc, &ctx.lsk)
            }
        },
    );

    bench_pair(
        &mut group,
        "prev_sankranti",
        || {
            dhruv_search::prev_sankranti(
                black_box(&ctx.engine),
                black_box(&utc_rust),
                black_box(&sankranti_cfg_rust),
            )
            .expect("prev sankranti")
            .map(|e| e.utc.to_jd_tdb(ctx.engine.lsk()))
            .unwrap_or(-1.0)
        },
        || unsafe {
            expect_ok(dhruv_ffi_c::dhruv_prev_sankranti(
                black_box(&ctx.engine as *const _),
                black_box(&utc as *const _),
                black_box(&sankranti_cfg_ffi as *const _),
                &mut sankranti_out,
                &mut found_u8,
            ));
            if found_u8 == 0 {
                -1.0
            } else {
                ffi_utc_to_jd_tdb(&sankranti_out.utc, &ctx.lsk)
            }
        },
    );

    let mut sankranti_outs: [dhruv_ffi_c::DhruvSankrantiEvent; 32] = zeroed();
    bench_pair(
        &mut group,
        "search_sankrantis",
        || {
            dhruv_search::search_sankrantis(
                black_box(&ctx.engine),
                black_box(&utc_rust),
                black_box(&utc_end_rust),
                black_box(&sankranti_cfg_rust),
            )
            .expect("search sankrantis")
            .len() as f64
        },
        || unsafe {
            expect_ok(dhruv_ffi_c::dhruv_search_sankrantis(
                black_box(&ctx.engine as *const _),
                black_box(&utc as *const _),
                black_box(&utc_end as *const _),
                black_box(&sankranti_cfg_ffi as *const _),
                sankranti_outs.as_mut_ptr(),
                32,
                &mut out_count,
            ));
            out_count as f64
        },
    );

    bench_pair(
        &mut group,
        "next_specific_sankranti",
        || {
            dhruv_search::next_specific_sankranti(
                black_box(&ctx.engine),
                black_box(&utc_rust),
                dhruv_vedic_base::ALL_RASHIS[0],
                black_box(&sankranti_cfg_rust),
            )
            .expect("next specific sankranti")
            .map(|e| e.utc.to_jd_tdb(ctx.engine.lsk()))
            .unwrap_or(-1.0)
        },
        || unsafe {
            expect_ok(dhruv_ffi_c::dhruv_next_specific_sankranti(
                black_box(&ctx.engine as *const _),
                black_box(&utc as *const _),
                0,
                black_box(&sankranti_cfg_ffi as *const _),
                &mut sankranti_out,
                &mut found_u8,
            ));
            if found_u8 == 0 {
                -1.0
            } else {
                ffi_utc_to_jd_tdb(&sankranti_out.utc, &ctx.lsk)
            }
        },
    );

    bench_pair(
        &mut group,
        "prev_specific_sankranti",
        || {
            dhruv_search::prev_specific_sankranti(
                black_box(&ctx.engine),
                black_box(&utc_rust),
                dhruv_vedic_base::ALL_RASHIS[0],
                black_box(&sankranti_cfg_rust),
            )
            .expect("prev specific sankranti")
            .map(|e| e.utc.to_jd_tdb(ctx.engine.lsk()))
            .unwrap_or(-1.0)
        },
        || unsafe {
            expect_ok(dhruv_ffi_c::dhruv_prev_specific_sankranti(
                black_box(&ctx.engine as *const _),
                black_box(&utc as *const _),
                0,
                black_box(&sankranti_cfg_ffi as *const _),
                &mut sankranti_out,
                &mut found_u8,
            ));
            if found_u8 == 0 {
                -1.0
            } else {
                ffi_utc_to_jd_tdb(&sankranti_out.utc, &ctx.lsk)
            }
        },
    );

    let mut masa_out: dhruv_ffi_c::DhruvMasaInfo = zeroed();
    bench_pair(
        &mut group,
        "masa_for_date",
        || {
            dhruv_search::masa_for_date(
                black_box(&ctx.engine),
                black_box(&utc_rust),
                black_box(&sankranti_cfg_rust),
            )
            .expect("masa_for_date")
            .masa
            .index() as i32
        },
        || unsafe {
            expect_ok(dhruv_ffi_c::dhruv_masa_for_date(
                black_box(&ctx.engine as *const _),
                black_box(&utc as *const _),
                black_box(&sankranti_cfg_ffi as *const _),
                &mut masa_out,
            ));
            masa_out.masa_index
        },
    );

    let mut ayana_out: dhruv_ffi_c::DhruvAyanaInfo = zeroed();
    bench_pair(
        &mut group,
        "ayana_for_date",
        || {
            dhruv_search::ayana_for_date(
                black_box(&ctx.engine),
                black_box(&utc_rust),
                black_box(&sankranti_cfg_rust),
            )
            .expect("ayana_for_date")
            .ayana
            .index() as i32
        },
        || unsafe {
            expect_ok(dhruv_ffi_c::dhruv_ayana_for_date(
                black_box(&ctx.engine as *const _),
                black_box(&utc as *const _),
                black_box(&sankranti_cfg_ffi as *const _),
                &mut ayana_out,
            ));
            ayana_out.ayana
        },
    );

    let mut varsha_out: dhruv_ffi_c::DhruvVarshaInfo = zeroed();
    bench_pair(
        &mut group,
        "varsha_for_date",
        || {
            dhruv_search::varsha_for_date(
                black_box(&ctx.engine),
                black_box(&utc_rust),
                black_box(&sankranti_cfg_rust),
            )
            .expect("varsha_for_date")
            .samvatsara
            .index() as i32
        },
        || unsafe {
            expect_ok(dhruv_ffi_c::dhruv_varsha_for_date(
                black_box(&ctx.engine as *const _),
                black_box(&utc as *const _),
                black_box(&sankranti_cfg_ffi as *const _),
                &mut varsha_out,
            ));
            varsha_out.samvatsara_index
        },
    );

    group.finish();
}

fn ffi_riseset_bhava_bench(c: &mut Criterion) {
    let ctx = match load_context() {
        Some(v) => v,
        None => return,
    };

    let mut group = c.benchmark_group("ffi_riseset_bhava");
    group.sample_size(10);

    let loc_rust = dhruv_vedic_base::GeoLocation::new(28.6139, 77.2090, 216.0);
    let loc_ffi = dhruv_ffi_c::DhruvGeoLocation {
        latitude_deg: 28.6139,
        longitude_deg: 77.2090,
        altitude_m: 216.0,
    };
    let rs_cfg_rust = dhruv_vedic_base::RiseSetConfig::default();
    let rs_cfg_ffi = dhruv_ffi_c::dhruv_riseset_config_default();
    let bhava_cfg_rust = dhruv_vedic_base::BhavaConfig::default();
    let bhava_cfg_ffi = dhruv_ffi_c::dhruv_bhava_config_default();

    let jd_utc_midnight = 2_460_000.5_f64;
    let jd_utc_noon = dhruv_vedic_base::approximate_local_noon_jd(jd_utc_midnight, 77.2090);

    bench_pair(
        &mut group,
        "approximate_local_noon_jd",
        || dhruv_vedic_base::approximate_local_noon_jd(black_box(jd_utc_midnight), 77.2090),
        || dhruv_ffi_c::dhruv_approximate_local_noon_jd(black_box(jd_utc_midnight), 77.2090),
    );

    let mut riseset_out: dhruv_ffi_c::DhruvRiseSetResult = zeroed();
    bench_pair(
        &mut group,
        "compute_rise_set",
        || match dhruv_vedic_base::compute_rise_set(
            black_box(&ctx.engine),
            black_box(&ctx.lsk),
            black_box(&ctx.eop),
            black_box(&loc_rust),
            dhruv_vedic_base::RiseSetEvent::Sunrise,
            black_box(jd_utc_noon),
            black_box(&rs_cfg_rust),
        )
        .expect("compute_rise_set")
        {
            dhruv_vedic_base::RiseSetResult::Event { jd_tdb, .. } => jd_tdb,
            _ => -1.0,
        },
        || unsafe {
            expect_ok(dhruv_ffi_c::dhruv_compute_rise_set(
                black_box(&ctx.engine as *const _),
                black_box(&ctx.lsk as *const _),
                black_box(&ctx.eop as *const _),
                black_box(&loc_ffi as *const _),
                dhruv_ffi_c::DHRUV_EVENT_SUNRISE,
                jd_utc_noon,
                black_box(&rs_cfg_ffi as *const _),
                &mut riseset_out,
            ));
            if riseset_out.result_type == dhruv_ffi_c::DHRUV_RISESET_EVENT {
                riseset_out.jd_tdb
            } else {
                -1.0
            }
        },
    );

    let sunrise_jd_tdb = match dhruv_vedic_base::compute_rise_set(
        &ctx.engine,
        &ctx.lsk,
        &ctx.eop,
        &loc_rust,
        dhruv_vedic_base::RiseSetEvent::Sunrise,
        jd_utc_noon,
        &rs_cfg_rust,
    )
    .expect("seed rise_set result")
    {
        dhruv_vedic_base::RiseSetResult::Event { jd_tdb, .. } => jd_tdb,
        _ => panic!("expected sunrise event"),
    };
    let riseset_event = dhruv_ffi_c::DhruvRiseSetResult {
        result_type: dhruv_ffi_c::DHRUV_RISESET_EVENT,
        event_code: dhruv_ffi_c::DHRUV_EVENT_SUNRISE,
        jd_tdb: sunrise_jd_tdb,
    };
    let mut riseset_utc_conv: DhruvUtcTime = zeroed();
    bench_pair(
        &mut group,
        "riseset_result_to_utc",
        || UtcTime::from_jd_tdb(black_box(sunrise_jd_tdb), black_box(&ctx.lsk)).second,
        || unsafe {
            expect_ok(dhruv_ffi_c::dhruv_riseset_result_to_utc(
                black_box(&ctx.lsk as *const _),
                black_box(&riseset_event as *const _),
                &mut riseset_utc_conv,
            ));
            riseset_utc_conv.second
        },
    );

    let mut riseset_all_out: [dhruv_ffi_c::DhruvRiseSetResult; 8] = zeroed();
    bench_pair(
        &mut group,
        "compute_all_events",
        || {
            dhruv_vedic_base::compute_all_events(
                black_box(&ctx.engine),
                black_box(&ctx.lsk),
                black_box(&ctx.eop),
                black_box(&loc_rust),
                black_box(jd_utc_noon),
                black_box(&rs_cfg_rust),
            )
            .expect("compute_all_events")
            .len() as f64
        },
        || unsafe {
            expect_ok(dhruv_ffi_c::dhruv_compute_all_events(
                black_box(&ctx.engine as *const _),
                black_box(&ctx.lsk as *const _),
                black_box(&ctx.eop as *const _),
                black_box(&loc_ffi as *const _),
                jd_utc_noon,
                black_box(&rs_cfg_ffi as *const _),
                riseset_all_out.as_mut_ptr(),
            ));
            8.0
        },
    );

    let utc = DhruvUtcTime {
        year: 2024,
        month: 1,
        day: 15,
        hour: 12,
        minute: 0,
        second: 0.0,
    };
    let jd_utc_from_struct = ffi_utc_to_jd_utc(&utc);

    let mut riseset_out_utc: dhruv_ffi_c::DhruvRiseSetResultUtc = zeroed();
    bench_pair(
        &mut group,
        "compute_rise_set_utc",
        || match dhruv_vedic_base::compute_rise_set(
            black_box(&ctx.engine),
            black_box(&ctx.lsk),
            black_box(&ctx.eop),
            black_box(&loc_rust),
            dhruv_vedic_base::RiseSetEvent::Sunrise,
            black_box(jd_utc_from_struct),
            black_box(&rs_cfg_rust),
        )
        .expect("compute_rise_set utc-rust")
        {
            dhruv_vedic_base::RiseSetResult::Event { jd_tdb, .. } => jd_tdb,
            _ => -1.0,
        },
        || unsafe {
            expect_ok(dhruv_ffi_c::dhruv_compute_rise_set_utc(
                black_box(&ctx.engine as *const _),
                black_box(&ctx.lsk as *const _),
                black_box(&ctx.eop as *const _),
                black_box(&loc_ffi as *const _),
                dhruv_ffi_c::DHRUV_EVENT_SUNRISE,
                black_box(&utc as *const _),
                black_box(&rs_cfg_ffi as *const _),
                &mut riseset_out_utc,
            ));
            if riseset_out_utc.result_type == dhruv_ffi_c::DHRUV_RISESET_EVENT {
                ffi_utc_to_jd_tdb(&riseset_out_utc.utc, &ctx.lsk)
            } else {
                -1.0
            }
        },
    );

    let mut riseset_all_out_utc: [dhruv_ffi_c::DhruvRiseSetResultUtc; 8] = zeroed();
    bench_pair(
        &mut group,
        "compute_all_events_utc",
        || {
            dhruv_vedic_base::compute_all_events(
                black_box(&ctx.engine),
                black_box(&ctx.lsk),
                black_box(&ctx.eop),
                black_box(&loc_rust),
                black_box(jd_utc_from_struct),
                black_box(&rs_cfg_rust),
            )
            .expect("compute_all_events utc-rust")
            .len() as f64
        },
        || unsafe {
            expect_ok(dhruv_ffi_c::dhruv_compute_all_events_utc(
                black_box(&ctx.engine as *const _),
                black_box(&ctx.lsk as *const _),
                black_box(&ctx.eop as *const _),
                black_box(&loc_ffi as *const _),
                black_box(&utc as *const _),
                black_box(&rs_cfg_ffi as *const _),
                riseset_all_out_utc.as_mut_ptr(),
            ));
            8.0
        },
    );

    let mut bhava_out: dhruv_ffi_c::DhruvBhavaResult = zeroed();
    bench_pair(
        &mut group,
        "compute_bhavas",
        || {
            dhruv_vedic_base::compute_bhavas(
                black_box(&ctx.engine),
                black_box(&ctx.lsk),
                black_box(&ctx.eop),
                black_box(&loc_rust),
                black_box(jd_utc_noon),
                black_box(&bhava_cfg_rust),
            )
            .expect("compute_bhavas")
            .lagna_deg
        },
        || unsafe {
            expect_ok(dhruv_ffi_c::dhruv_compute_bhavas(
                black_box(&ctx.engine as *const _),
                black_box(&ctx.lsk as *const _),
                black_box(&ctx.eop as *const _),
                black_box(&loc_ffi as *const _),
                jd_utc_noon,
                black_box(&bhava_cfg_ffi as *const _),
                &mut bhava_out,
            ));
            bhava_out.lagna_deg
        },
    );

    bench_pair(
        &mut group,
        "compute_bhavas_utc",
        || {
            dhruv_vedic_base::compute_bhavas(
                black_box(&ctx.engine),
                black_box(&ctx.lsk),
                black_box(&ctx.eop),
                black_box(&loc_rust),
                black_box(jd_utc_from_struct),
                black_box(&bhava_cfg_rust),
            )
            .expect("compute_bhavas utc-rust")
            .lagna_deg
        },
        || unsafe {
            expect_ok(dhruv_ffi_c::dhruv_compute_bhavas_utc(
                black_box(&ctx.engine as *const _),
                black_box(&ctx.lsk as *const _),
                black_box(&ctx.eop as *const _),
                black_box(&loc_ffi as *const _),
                black_box(&utc as *const _),
                black_box(&bhava_cfg_ffi as *const _),
                &mut bhava_out,
            ));
            bhava_out.lagna_deg
        },
    );

    let mut out_deg = 0.0_f64;
    bench_pair(
        &mut group,
        "lagna_deg",
        || {
            dhruv_vedic_base::lagna_longitude_rad(
                black_box(&ctx.lsk),
                black_box(&ctx.eop),
                black_box(&loc_rust),
                black_box(jd_utc_noon),
            )
            .expect("lagna")
            .to_degrees()
        },
        || unsafe {
            expect_ok(dhruv_ffi_c::dhruv_lagna_deg(
                black_box(&ctx.lsk as *const _),
                black_box(&ctx.eop as *const _),
                black_box(&loc_ffi as *const _),
                jd_utc_noon,
                &mut out_deg,
            ));
            out_deg
        },
    );

    bench_pair(
        &mut group,
        "mc_deg",
        || {
            dhruv_vedic_base::mc_longitude_rad(
                black_box(&ctx.lsk),
                black_box(&ctx.eop),
                black_box(&loc_rust),
                black_box(jd_utc_noon),
            )
            .expect("mc")
            .to_degrees()
        },
        || unsafe {
            expect_ok(dhruv_ffi_c::dhruv_mc_deg(
                black_box(&ctx.lsk as *const _),
                black_box(&ctx.eop as *const _),
                black_box(&loc_ffi as *const _),
                jd_utc_noon,
                &mut out_deg,
            ));
            out_deg
        },
    );

    bench_pair(
        &mut group,
        "ramc_deg",
        || {
            dhruv_vedic_base::ramc_rad(
                black_box(&ctx.lsk),
                black_box(&ctx.eop),
                black_box(&loc_rust),
                black_box(jd_utc_noon),
            )
            .expect("ramc")
            .to_degrees()
        },
        || unsafe {
            expect_ok(dhruv_ffi_c::dhruv_ramc_deg(
                black_box(&ctx.lsk as *const _),
                black_box(&ctx.eop as *const _),
                black_box(&loc_ffi as *const _),
                jd_utc_noon,
                &mut out_deg,
            ));
            out_deg
        },
    );

    bench_pair(
        &mut group,
        "lagna_deg_utc",
        || {
            dhruv_vedic_base::lagna_longitude_rad(
                black_box(&ctx.lsk),
                black_box(&ctx.eop),
                black_box(&loc_rust),
                black_box(jd_utc_from_struct),
            )
            .expect("lagna utc-rust")
            .to_degrees()
        },
        || unsafe {
            expect_ok(dhruv_ffi_c::dhruv_lagna_deg_utc(
                black_box(&ctx.lsk as *const _),
                black_box(&ctx.eop as *const _),
                black_box(&loc_ffi as *const _),
                black_box(&utc as *const _),
                &mut out_deg,
            ));
            out_deg
        },
    );

    bench_pair(
        &mut group,
        "mc_deg_utc",
        || {
            dhruv_vedic_base::mc_longitude_rad(
                black_box(&ctx.lsk),
                black_box(&ctx.eop),
                black_box(&loc_rust),
                black_box(jd_utc_from_struct),
            )
            .expect("mc utc-rust")
            .to_degrees()
        },
        || unsafe {
            expect_ok(dhruv_ffi_c::dhruv_mc_deg_utc(
                black_box(&ctx.lsk as *const _),
                black_box(&ctx.eop as *const _),
                black_box(&loc_ffi as *const _),
                black_box(&utc as *const _),
                &mut out_deg,
            ));
            out_deg
        },
    );

    bench_pair(
        &mut group,
        "ramc_deg_utc",
        || {
            dhruv_vedic_base::ramc_rad(
                black_box(&ctx.lsk),
                black_box(&ctx.eop),
                black_box(&loc_rust),
                black_box(jd_utc_from_struct),
            )
            .expect("ramc utc-rust")
            .to_degrees()
        },
        || unsafe {
            expect_ok(dhruv_ffi_c::dhruv_ramc_deg_utc(
                black_box(&ctx.lsk as *const _),
                black_box(&ctx.eop as *const _),
                black_box(&loc_ffi as *const _),
                black_box(&utc as *const _),
                &mut out_deg,
            ));
            out_deg
        },
    );

    group.finish();
}

fn ffi_panchang_runtime_bench(c: &mut Criterion) {
    let ctx = match load_context() {
        Some(v) => v,
        None => return,
    };

    let mut group = c.benchmark_group("ffi_panchang_runtime");
    group.sample_size(10);

    let loc_rust = dhruv_vedic_base::GeoLocation::new(28.6139, 77.2090, 216.0);
    let loc_ffi = dhruv_ffi_c::DhruvGeoLocation {
        latitude_deg: 28.6139,
        longitude_deg: 77.2090,
        altitude_m: 216.0,
    };
    let rs_cfg_rust = dhruv_vedic_base::RiseSetConfig::default();
    let rs_cfg_ffi = dhruv_ffi_c::dhruv_riseset_config_default();
    let sank_cfg_rust = SankrantiConfig::new(AyanamshaSystem::Lahiri, false);
    let sank_cfg_ffi = dhruv_sankranti_config_default();

    let utc = DhruvUtcTime {
        year: 2024,
        month: 1,
        day: 15,
        hour: 12,
        minute: 0,
        second: 0.0,
    };
    let utc_rust = ffi_utc_to_rust(&utc);
    let jd_tdb = ffi_utc_to_jd_tdb(&utc, &ctx.lsk);

    let mut out_f64 = 0.0_f64;
    bench_pair(
        &mut group,
        "elongation_at",
        || {
            dhruv_search::elongation_at(black_box(&ctx.engine), black_box(jd_tdb))
                .expect("elongation_at")
        },
        || unsafe {
            expect_ok(dhruv_ffi_c::dhruv_elongation_at(
                black_box(&ctx.engine as *const _),
                jd_tdb,
                &mut out_f64,
            ));
            out_f64
        },
    );

    bench_pair(
        &mut group,
        "sidereal_sum_at",
        || {
            dhruv_search::sidereal_sum_at(
                black_box(&ctx.engine),
                black_box(jd_tdb),
                black_box(&sank_cfg_rust),
            )
            .expect("sidereal_sum_at")
        },
        || unsafe {
            expect_ok(dhruv_ffi_c::dhruv_sidereal_sum_at(
                black_box(&ctx.engine as *const _),
                jd_tdb,
                black_box(&sank_cfg_ffi as *const _),
                &mut out_f64,
            ));
            out_f64
        },
    );

    let mut out_lon = 0.0_f64;
    let mut out_lat = 0.0_f64;
    bench_pair(
        &mut group,
        "body_ecliptic_lon_lat",
        || {
            dhruv_search::body_ecliptic_lon_lat(
                black_box(&ctx.engine),
                Body::Moon,
                black_box(jd_tdb),
            )
            .expect("body lon/lat")
            .0
        },
        || unsafe {
            expect_ok(dhruv_ffi_c::dhruv_body_ecliptic_lon_lat(
                black_box(&ctx.engine as *const _),
                Body::Moon.code(),
                jd_tdb,
                &mut out_lon,
                &mut out_lat,
            ));
            out_lon
        },
    );

    let mut out_tithi: dhruv_ffi_c::DhruvTithiInfo = zeroed();
    bench_pair(
        &mut group,
        "tithi_for_date",
        || {
            dhruv_search::tithi_for_date(black_box(&ctx.engine), black_box(&utc_rust))
                .expect("tithi_for_date")
                .tithi_index as i32
        },
        || unsafe {
            expect_ok(dhruv_ffi_c::dhruv_tithi_for_date(
                black_box(&ctx.engine as *const _),
                black_box(&utc as *const _),
                &mut out_tithi,
            ));
            out_tithi.tithi_index
        },
    );

    let mut out_karana: dhruv_ffi_c::DhruvKaranaInfo = zeroed();
    bench_pair(
        &mut group,
        "karana_for_date",
        || {
            dhruv_search::karana_for_date(black_box(&ctx.engine), black_box(&utc_rust))
                .expect("karana_for_date")
                .karana_index as i32
        },
        || unsafe {
            expect_ok(dhruv_ffi_c::dhruv_karana_for_date(
                black_box(&ctx.engine as *const _),
                black_box(&utc as *const _),
                &mut out_karana,
            ));
            out_karana.karana_index
        },
    );

    let mut out_yoga: dhruv_ffi_c::DhruvYogaInfo = zeroed();
    bench_pair(
        &mut group,
        "yoga_for_date",
        || {
            dhruv_search::yoga_for_date(
                black_box(&ctx.engine),
                black_box(&utc_rust),
                black_box(&sank_cfg_rust),
            )
            .expect("yoga_for_date")
            .yoga_index as i32
        },
        || unsafe {
            expect_ok(dhruv_ffi_c::dhruv_yoga_for_date(
                black_box(&ctx.engine as *const _),
                black_box(&utc as *const _),
                black_box(&sank_cfg_ffi as *const _),
                &mut out_yoga,
            ));
            out_yoga.yoga_index
        },
    );

    let mut out_nakshatra: dhruv_ffi_c::DhruvPanchangNakshatraInfo = zeroed();
    bench_pair(
        &mut group,
        "nakshatra_for_date",
        || {
            dhruv_search::nakshatra_for_date(
                black_box(&ctx.engine),
                black_box(&utc_rust),
                black_box(&sank_cfg_rust),
            )
            .expect("nakshatra_for_date")
            .nakshatra_index as i32
        },
        || unsafe {
            expect_ok(dhruv_ffi_c::dhruv_nakshatra_for_date(
                black_box(&ctx.engine as *const _),
                black_box(&utc as *const _),
                black_box(&sank_cfg_ffi as *const _),
                &mut out_nakshatra,
            ));
            out_nakshatra.nakshatra_index
        },
    );

    let mut sunrise_jd = 0.0_f64;
    let mut next_sunrise_jd = 0.0_f64;
    bench_pair(
        &mut group,
        "vedic_day_sunrises",
        || {
            dhruv_search::vedic_day_sunrises(
                black_box(&ctx.engine),
                black_box(&ctx.eop),
                black_box(&utc_rust),
                black_box(&loc_rust),
                black_box(&rs_cfg_rust),
            )
            .expect("vedic_day_sunrises")
            .0
        },
        || unsafe {
            expect_ok(dhruv_ffi_c::dhruv_vedic_day_sunrises(
                black_box(&ctx.engine as *const _),
                black_box(&ctx.eop as *const _),
                black_box(&utc as *const _),
                black_box(&loc_ffi as *const _),
                black_box(&rs_cfg_ffi as *const _),
                &mut sunrise_jd,
                &mut next_sunrise_jd,
            ));
            sunrise_jd
        },
    );

    let mut out_vaar: dhruv_ffi_c::DhruvVaarInfo = zeroed();
    bench_pair(
        &mut group,
        "vaar_for_date",
        || {
            dhruv_search::vaar_for_date(
                black_box(&ctx.engine),
                black_box(&ctx.eop),
                black_box(&utc_rust),
                black_box(&loc_rust),
                black_box(&rs_cfg_rust),
            )
            .expect("vaar_for_date")
            .vaar
            .index() as i32
        },
        || unsafe {
            expect_ok(dhruv_ffi_c::dhruv_vaar_for_date(
                black_box(&ctx.engine as *const _),
                black_box(&ctx.eop as *const _),
                black_box(&utc as *const _),
                black_box(&loc_ffi as *const _),
                black_box(&rs_cfg_ffi as *const _),
                &mut out_vaar,
            ));
            out_vaar.vaar_index
        },
    );

    let mut out_hora: dhruv_ffi_c::DhruvHoraInfo = zeroed();
    bench_pair(
        &mut group,
        "hora_for_date",
        || {
            dhruv_search::hora_for_date(
                black_box(&ctx.engine),
                black_box(&ctx.eop),
                black_box(&utc_rust),
                black_box(&loc_rust),
                black_box(&rs_cfg_rust),
            )
            .expect("hora_for_date")
            .hora_index as i32
        },
        || unsafe {
            expect_ok(dhruv_ffi_c::dhruv_hora_for_date(
                black_box(&ctx.engine as *const _),
                black_box(&ctx.eop as *const _),
                black_box(&utc as *const _),
                black_box(&loc_ffi as *const _),
                black_box(&rs_cfg_ffi as *const _),
                &mut out_hora,
            ));
            out_hora.hora_position
        },
    );

    let mut out_ghatika: dhruv_ffi_c::DhruvGhatikaInfo = zeroed();
    bench_pair(
        &mut group,
        "ghatika_for_date",
        || {
            dhruv_search::ghatika_for_date(
                black_box(&ctx.engine),
                black_box(&ctx.eop),
                black_box(&utc_rust),
                black_box(&loc_rust),
                black_box(&rs_cfg_rust),
            )
            .expect("ghatika_for_date")
            .value as i32
        },
        || unsafe {
            expect_ok(dhruv_ffi_c::dhruv_ghatika_for_date(
                black_box(&ctx.engine as *const _),
                black_box(&ctx.eop as *const _),
                black_box(&utc as *const _),
                black_box(&loc_ffi as *const _),
                black_box(&rs_cfg_ffi as *const _),
                &mut out_ghatika,
            ));
            out_ghatika.value
        },
    );

    let elong_for_at =
        dhruv_search::elongation_at(&ctx.engine, jd_tdb).expect("elongation input for *_at");
    let sum_for_at = dhruv_search::sidereal_sum_at(&ctx.engine, jd_tdb, &sank_cfg_rust)
        .expect("sum input for *_at");
    let (sr_jd, nsr_jd) =
        dhruv_search::vedic_day_sunrises(&ctx.engine, &ctx.eop, &utc_rust, &loc_rust, &rs_cfg_rust)
            .expect("sunrises");

    bench_pair(
        &mut group,
        "tithi_at",
        || {
            dhruv_search::tithi_at(
                black_box(&ctx.engine),
                black_box(jd_tdb),
                black_box(elong_for_at),
            )
            .expect("tithi_at")
            .tithi_index as i32
        },
        || unsafe {
            expect_ok(dhruv_ffi_c::dhruv_tithi_at(
                black_box(&ctx.engine as *const _),
                jd_tdb,
                elong_for_at,
                &mut out_tithi,
            ));
            out_tithi.tithi_index
        },
    );

    bench_pair(
        &mut group,
        "karana_at",
        || {
            dhruv_search::karana_at(
                black_box(&ctx.engine),
                black_box(jd_tdb),
                black_box(elong_for_at),
            )
            .expect("karana_at")
            .karana_index as i32
        },
        || unsafe {
            expect_ok(dhruv_ffi_c::dhruv_karana_at(
                black_box(&ctx.engine as *const _),
                jd_tdb,
                elong_for_at,
                &mut out_karana,
            ));
            out_karana.karana_index
        },
    );

    bench_pair(
        &mut group,
        "yoga_at",
        || {
            dhruv_search::yoga_at(
                black_box(&ctx.engine),
                black_box(jd_tdb),
                black_box(sum_for_at),
                black_box(&sank_cfg_rust),
            )
            .expect("yoga_at")
            .yoga_index as i32
        },
        || unsafe {
            expect_ok(dhruv_ffi_c::dhruv_yoga_at(
                black_box(&ctx.engine as *const _),
                jd_tdb,
                sum_for_at,
                black_box(&sank_cfg_ffi as *const _),
                &mut out_yoga,
            ));
            out_yoga.yoga_index
        },
    );

    bench_pair(
        &mut group,
        "vaar_from_sunrises",
        || {
            dhruv_search::vaar_from_sunrises(
                black_box(sr_jd),
                black_box(nsr_jd),
                black_box(&ctx.lsk),
            )
            .vaar
            .index() as i32
        },
        || unsafe {
            expect_ok(dhruv_ffi_c::dhruv_vaar_from_sunrises(
                black_box(&ctx.lsk as *const _),
                sr_jd,
                nsr_jd,
                &mut out_vaar,
            ));
            out_vaar.vaar_index
        },
    );

    bench_pair(
        &mut group,
        "hora_from_sunrises",
        || {
            dhruv_search::hora_from_sunrises(
                black_box(jd_tdb),
                black_box(sr_jd),
                black_box(nsr_jd),
                black_box(&ctx.lsk),
            )
            .hora_index as i32
        },
        || unsafe {
            expect_ok(dhruv_ffi_c::dhruv_hora_from_sunrises(
                black_box(&ctx.lsk as *const _),
                jd_tdb,
                sr_jd,
                nsr_jd,
                &mut out_hora,
            ));
            out_hora.hora_position
        },
    );

    bench_pair(
        &mut group,
        "ghatika_from_sunrises",
        || {
            dhruv_search::ghatika_from_sunrises(
                black_box(jd_tdb),
                black_box(sr_jd),
                black_box(nsr_jd),
                black_box(&ctx.lsk),
            )
            .value as i32
        },
        || unsafe {
            expect_ok(dhruv_ffi_c::dhruv_ghatika_from_sunrises(
                black_box(&ctx.lsk as *const _),
                jd_tdb,
                sr_jd,
                nsr_jd,
                &mut out_ghatika,
            ));
            out_ghatika.value
        },
    );

    let mut out_panchang: dhruv_ffi_c::DhruvPanchangInfo = zeroed();
    bench_pair(
        &mut group,
        "panchang_for_date",
        || {
            dhruv_search::panchang_for_date(
                black_box(&ctx.engine),
                black_box(&ctx.eop),
                black_box(&utc_rust),
                black_box(&loc_rust),
                black_box(&rs_cfg_rust),
                black_box(&sank_cfg_rust),
                true,
            )
            .expect("panchang_for_date")
            .tithi
            .tithi_index as i32
        },
        || unsafe {
            expect_ok(dhruv_ffi_c::dhruv_panchang_for_date(
                black_box(&ctx.engine as *const _),
                black_box(&ctx.eop as *const _),
                black_box(&utc as *const _),
                black_box(&loc_ffi as *const _),
                black_box(&rs_cfg_ffi as *const _),
                black_box(&sank_cfg_ffi as *const _),
                1,
                &mut out_panchang,
            ));
            out_panchang.tithi.tithi_index
        },
    );

    group.finish();
}

fn ffi_orchestrators_bench(c: &mut Criterion) {
    let ctx = match load_context() {
        Some(v) => v,
        None => return,
    };

    let mut group = c.benchmark_group("ffi_orchestrators");
    group.sample_size(10);

    let loc_rust = dhruv_vedic_base::GeoLocation::new(28.6139, 77.2090, 216.0);
    let loc_ffi = dhruv_ffi_c::DhruvGeoLocation {
        latitude_deg: 28.6139,
        longitude_deg: 77.2090,
        altitude_m: 216.0,
    };
    let rs_cfg_rust = dhruv_vedic_base::RiseSetConfig::default();
    let rs_cfg_ffi = dhruv_ffi_c::dhruv_riseset_config_default();
    let bhava_cfg_rust = dhruv_vedic_base::BhavaConfig::default();
    let bhava_cfg_ffi = dhruv_ffi_c::dhruv_bhava_config_default();
    let aya_cfg_rust = SankrantiConfig::new(AyanamshaSystem::Lahiri, false);
    let aya_system_code = 0_u32;
    let use_nutation = 0_u8;

    let utc = DhruvUtcTime {
        year: 2024,
        month: 1,
        day: 15,
        hour: 12,
        minute: 0,
        second: 0.0,
    };
    let utc_rust = ffi_utc_to_rust(&utc);

    let rashis: [u8; 7] = [0, 1, 2, 3, 4, 5, 6];
    let lagna_rashi = 0_u8;
    let mut ashta_out: dhruv_ffi_c::DhruvAshtakavargaResult = zeroed();
    bench_pair(
        &mut group,
        "calculate_ashtakavarga",
        || {
            dhruv_vedic_base::calculate_ashtakavarga(black_box(&rashis), black_box(lagna_rashi))
                .sav
                .total_points[0] as i32
        },
        || unsafe {
            expect_ok(dhruv_ffi_c::dhruv_calculate_ashtakavarga(
                black_box(rashis.as_ptr()),
                lagna_rashi,
                &mut ashta_out,
            ));
            ashta_out.sav.total_points[0] as i32
        },
    );

    bench_pair(
        &mut group,
        "ashtakavarga_for_date",
        || {
            dhruv_search::ashtakavarga_for_date(
                black_box(&ctx.engine),
                black_box(&ctx.eop),
                black_box(&utc_rust),
                black_box(&loc_rust),
                black_box(&aya_cfg_rust),
            )
            .expect("ashtakavarga_for_date")
            .sav
            .total_points[0] as i32
        },
        || unsafe {
            expect_ok(dhruv_ffi_c::dhruv_ashtakavarga_for_date(
                black_box(&ctx.engine as *const _),
                black_box(&ctx.eop as *const _),
                black_box(&utc as *const _),
                black_box(&loc_ffi as *const _),
                aya_system_code,
                use_nutation,
                &mut ashta_out,
            ));
            ashta_out.sav.total_points[0] as i32
        },
    );

    let gp_cfg_rust = dhruv_search::GrahaPositionsConfig {
        include_nakshatra: true,
        include_lagna: true,
        include_outer_planets: false,
        include_bhava: true,
    };
    let gp_cfg_ffi = dhruv_ffi_c::DhruvGrahaPositionsConfig {
        include_nakshatra: 1,
        include_lagna: 1,
        include_outer_planets: 0,
        include_bhava: 1,
    };
    let mut gp_out: dhruv_ffi_c::DhruvGrahaPositions = zeroed();
    bench_pair(
        &mut group,
        "graha_positions",
        || {
            dhruv_search::graha_positions(
                black_box(&ctx.engine),
                black_box(&ctx.eop),
                black_box(&utc_rust),
                black_box(&loc_rust),
                black_box(&bhava_cfg_rust),
                black_box(&aya_cfg_rust),
                black_box(&gp_cfg_rust),
            )
            .expect("graha_positions")
            .grahas[0]
                .sidereal_longitude
        },
        || unsafe {
            expect_ok(dhruv_ffi_c::dhruv_graha_positions(
                black_box(&ctx.engine as *const _),
                black_box(&ctx.eop as *const _),
                black_box(&utc as *const _),
                black_box(&loc_ffi as *const _),
                black_box(&bhava_cfg_ffi as *const _),
                aya_system_code,
                use_nutation,
                black_box(&gp_cfg_ffi as *const _),
                &mut gp_out,
            ));
            gp_out.grahas[0].sidereal_longitude
        },
    );

    let bind_cfg_rust = dhruv_search::BindusConfig {
        include_nakshatra: false,
        include_bhava: false,
    };
    let bind_cfg_ffi = dhruv_ffi_c::DhruvBindusConfig {
        include_nakshatra: 0,
        include_bhava: 0,
    };
    let mut bind_out: dhruv_ffi_c::DhruvBindusResult = zeroed();
    bench_pair(
        &mut group,
        "core_bindus",
        || {
            dhruv_search::core_bindus(
                black_box(&ctx.engine),
                black_box(&ctx.eop),
                black_box(&utc_rust),
                black_box(&loc_rust),
                black_box(&bhava_cfg_rust),
                black_box(&rs_cfg_rust),
                black_box(&aya_cfg_rust),
                black_box(&bind_cfg_rust),
            )
            .expect("core_bindus")
            .bhrigu_bindu
            .sidereal_longitude
        },
        || unsafe {
            expect_ok(dhruv_ffi_c::dhruv_core_bindus(
                black_box(&ctx.engine as *const _),
                black_box(&ctx.eop as *const _),
                black_box(&utc as *const _),
                black_box(&loc_ffi as *const _),
                black_box(&bhava_cfg_ffi as *const _),
                black_box(&rs_cfg_ffi as *const _),
                aya_system_code,
                use_nutation,
                black_box(&bind_cfg_ffi as *const _),
                &mut bind_out,
            ));
            bind_out.bhrigu_bindu.sidereal_longitude
        },
    );

    let drishti_cfg_rust = dhruv_search::DrishtiConfig {
        include_bhava: false,
        include_lagna: false,
        include_bindus: false,
    };
    let drishti_cfg_ffi = dhruv_ffi_c::DhruvDrishtiConfig {
        include_bhava: 0,
        include_lagna: 0,
        include_bindus: 0,
    };
    let mut drishti_out: dhruv_ffi_c::DhruvDrishtiResult = zeroed();
    bench_pair(
        &mut group,
        "drishti",
        || {
            dhruv_search::drishti_for_date(
                black_box(&ctx.engine),
                black_box(&ctx.eop),
                black_box(&utc_rust),
                black_box(&loc_rust),
                black_box(&bhava_cfg_rust),
                black_box(&rs_cfg_rust),
                black_box(&aya_cfg_rust),
                black_box(&drishti_cfg_rust),
            )
            .expect("drishti_for_date")
            .graha_to_graha
            .entries[0][1]
                .total_virupa
        },
        || unsafe {
            expect_ok(dhruv_ffi_c::dhruv_drishti(
                black_box(&ctx.engine as *const _),
                black_box(&ctx.eop as *const _),
                black_box(&utc as *const _),
                black_box(&loc_ffi as *const _),
                black_box(&bhava_cfg_ffi as *const _),
                black_box(&rs_cfg_ffi as *const _),
                aya_system_code,
                use_nutation,
                black_box(&drishti_cfg_ffi as *const _),
                &mut drishti_out,
            ));
            drishti_out.graha_to_graha[0][1].total_virupa
        },
    );

    let mut sl_out: dhruv_ffi_c::DhruvSpecialLagnas = zeroed();
    bench_pair(
        &mut group,
        "special_lagnas_for_date",
        || {
            dhruv_search::special_lagnas_for_date(
                black_box(&ctx.engine),
                black_box(&ctx.eop),
                black_box(&utc_rust),
                black_box(&loc_rust),
                black_box(&rs_cfg_rust),
                black_box(&aya_cfg_rust),
            )
            .expect("special_lagnas_for_date")
            .bhava_lagna
        },
        || unsafe {
            expect_ok(dhruv_ffi_c::dhruv_special_lagnas_for_date(
                black_box(&ctx.engine as *const _),
                black_box(&ctx.eop as *const _),
                black_box(&utc as *const _),
                black_box(&loc_ffi as *const _),
                black_box(&rs_cfg_ffi as *const _),
                aya_system_code,
                use_nutation,
                &mut sl_out,
            ));
            sl_out.bhava_lagna
        },
    );

    let mut arudha_out: [dhruv_ffi_c::DhruvArudhaResult; 12] = zeroed();
    bench_pair(
        &mut group,
        "arudha_padas_for_date",
        || {
            dhruv_search::arudha_padas_for_date(
                black_box(&ctx.engine),
                black_box(&ctx.eop),
                black_box(&utc_rust),
                black_box(&loc_rust),
                black_box(&bhava_cfg_rust),
                black_box(&aya_cfg_rust),
            )
            .expect("arudha_padas_for_date")[0]
                .longitude_deg
        },
        || unsafe {
            expect_ok(dhruv_ffi_c::dhruv_arudha_padas_for_date(
                black_box(&ctx.engine as *const _),
                black_box(&ctx.eop as *const _),
                black_box(&utc as *const _),
                black_box(&loc_ffi as *const _),
                aya_system_code,
                use_nutation,
                arudha_out.as_mut_ptr(),
            ));
            arudha_out[0].longitude_deg
        },
    );

    let mut upagraha_out: dhruv_ffi_c::DhruvAllUpagrahas = zeroed();
    bench_pair(
        &mut group,
        "all_upagrahas_for_date",
        || {
            dhruv_search::all_upagrahas_for_date(
                black_box(&ctx.engine),
                black_box(&ctx.eop),
                black_box(&utc_rust),
                black_box(&loc_rust),
                black_box(&rs_cfg_rust),
                black_box(&aya_cfg_rust),
            )
            .expect("all_upagrahas_for_date")
            .gulika
        },
        || unsafe {
            expect_ok(dhruv_ffi_c::dhruv_all_upagrahas_for_date(
                black_box(&ctx.engine as *const _),
                black_box(&ctx.eop as *const _),
                black_box(&utc as *const _),
                black_box(&loc_ffi as *const _),
                aya_system_code,
                use_nutation,
                &mut upagraha_out,
            ));
            upagraha_out.gulika
        },
    );

    let sun_sid = 123.456_f64;
    bench_pair(
        &mut group,
        "sun_based_upagrahas",
        || dhruv_vedic_base::sun_based_upagrahas(black_box(sun_sid)).dhooma,
        || unsafe {
            expect_ok(dhruv_ffi_c::dhruv_sun_based_upagrahas(
                sun_sid,
                &mut upagraha_out,
            ));
            upagraha_out.dhooma
        },
    );

    let mut out_jd = 0.0_f64;
    bench_pair(
        &mut group,
        "time_upagraha_jd_utc",
        || {
            let (sr, nsr) = dhruv_search::vedic_day_sunrises(
                black_box(&ctx.engine),
                black_box(&ctx.eop),
                black_box(&utc_rust),
                black_box(&loc_rust),
                black_box(&rs_cfg_rust),
            )
            .expect("sunrises for time_upagraha_jd_utc");
            let jd_utc = ffi_utc_to_jd_utc(&utc);
            let noon_jd = dhruv_vedic_base::approximate_local_noon_jd(
                jd_utc.floor() + 0.5,
                loc_rust.longitude_deg,
            );
            let sunset_jd = match dhruv_vedic_base::compute_rise_set(
                black_box(&ctx.engine),
                black_box(&ctx.lsk),
                black_box(&ctx.eop),
                black_box(&loc_rust),
                dhruv_vedic_base::RiseSetEvent::Sunset,
                noon_jd,
                black_box(&rs_cfg_rust),
            )
            .expect("sunset for time_upagraha_jd_utc")
            {
                dhruv_vedic_base::RiseSetResult::Event { jd_tdb, .. } => jd_tdb,
                _ => sr,
            };
            let moment = utc_rust.to_jd_tdb(ctx.engine.lsk());
            let is_day = moment >= sr && moment < sunset_jd;
            let weekday = dhruv_vedic_base::vaar_from_jd(sr).index();
            dhruv_vedic_base::time_upagraha_jd(
                black_box(Upagraha::Gulika),
                black_box(weekday),
                black_box(is_day),
                black_box(sr),
                black_box(sunset_jd),
                black_box(nsr),
            )
        },
        || unsafe {
            expect_ok(dhruv_ffi_c::dhruv_time_upagraha_jd_utc(
                black_box(&ctx.engine as *const _),
                black_box(&ctx.eop as *const _),
                black_box(&utc as *const _),
                black_box(&loc_ffi as *const _),
                black_box(&rs_cfg_ffi as *const _),
                0,
                &mut out_jd,
            ));
            out_jd
        },
    );

    let sphuta_inputs = dhruv_vedic_base::SphutalInputs {
        sun: 120.0,
        moon: 45.0,
        mars: 300.0,
        jupiter: 210.0,
        venus: 180.0,
        rahu: 25.0,
        lagna: 10.0,
        eighth_lord: 95.0,
        gulika: 130.0,
    };
    let ffi_sphuta_inputs = dhruv_ffi_c::DhruvSphutalInputs {
        sun: sphuta_inputs.sun,
        moon: sphuta_inputs.moon,
        mars: sphuta_inputs.mars,
        jupiter: sphuta_inputs.jupiter,
        venus: sphuta_inputs.venus,
        rahu: sphuta_inputs.rahu,
        lagna: sphuta_inputs.lagna,
        eighth_lord: sphuta_inputs.eighth_lord,
        gulika: sphuta_inputs.gulika,
    };
    let mut ffi_sphuta_out: dhruv_ffi_c::DhruvSphutalResult = zeroed();
    bench_pair(
        &mut group,
        "all_sphutas",
        || dhruv_vedic_base::all_sphutas(black_box(&sphuta_inputs))[0].1,
        || unsafe {
            expect_ok(dhruv_ffi_c::dhruv_all_sphutas(
                black_box(&ffi_sphuta_inputs as *const _),
                &mut ffi_sphuta_out,
            ));
            ffi_sphuta_out.longitudes[0]
        },
    );

    group.finish();
}

fn ffi_scalar_wrappers_bench(c: &mut Criterion) {
    let mut group = c.benchmark_group("ffi_scalar_wrappers");
    group.sample_size(10);

    let sun = 120.0_f64;
    let moon = 45.0_f64;
    let mars = 300.0_f64;
    let jupiter = 210.0_f64;
    let venus = 180.0_f64;
    let rahu = 25.0_f64;
    let lagna = 10.0_f64;
    let eighth_lord = 95.0_f64;
    let gulika = 130.0_f64;

    let mut arudha_rashi = 0_u8;
    bench_pair(
        &mut group,
        "arudha_pada",
        || dhruv_vedic_base::arudha_pada(black_box(80.0), black_box(210.0)).0,
        || unsafe { dhruv_ffi_c::dhruv_arudha_pada(80.0, 210.0, &mut arudha_rashi) },
    );

    bench_pair(
        &mut group,
        "bhrigu_bindu",
        || dhruv_vedic_base::bhrigu_bindu(black_box(rahu), black_box(moon)),
        || dhruv_ffi_c::dhruv_bhrigu_bindu(black_box(rahu), black_box(moon)),
    );
    bench_pair(
        &mut group,
        "prana_sphuta",
        || dhruv_vedic_base::prana_sphuta(black_box(lagna), black_box(moon)),
        || dhruv_ffi_c::dhruv_prana_sphuta(black_box(lagna), black_box(moon)),
    );
    bench_pair(
        &mut group,
        "deha_sphuta",
        || dhruv_vedic_base::deha_sphuta(black_box(moon), black_box(lagna)),
        || dhruv_ffi_c::dhruv_deha_sphuta(black_box(moon), black_box(lagna)),
    );
    bench_pair(
        &mut group,
        "mrityu_sphuta",
        || dhruv_vedic_base::mrityu_sphuta(black_box(eighth_lord), black_box(lagna)),
        || dhruv_ffi_c::dhruv_mrityu_sphuta(black_box(eighth_lord), black_box(lagna)),
    );
    bench_pair(
        &mut group,
        "tithi_sphuta",
        || dhruv_vedic_base::tithi_sphuta(black_box(moon), black_box(sun), black_box(lagna)),
        || dhruv_ffi_c::dhruv_tithi_sphuta(black_box(moon), black_box(sun), black_box(lagna)),
    );
    bench_pair(
        &mut group,
        "yoga_sphuta",
        || dhruv_vedic_base::yoga_sphuta(black_box(sun), black_box(moon)),
        || dhruv_ffi_c::dhruv_yoga_sphuta(black_box(sun), black_box(moon)),
    );
    bench_pair(
        &mut group,
        "yoga_sphuta_normalized",
        || dhruv_vedic_base::yoga_sphuta_normalized(black_box(sun), black_box(moon)),
        || dhruv_ffi_c::dhruv_yoga_sphuta_normalized(black_box(sun), black_box(moon)),
    );
    bench_pair(
        &mut group,
        "rahu_tithi_sphuta",
        || dhruv_vedic_base::rahu_tithi_sphuta(black_box(rahu), black_box(sun), black_box(lagna)),
        || dhruv_ffi_c::dhruv_rahu_tithi_sphuta(black_box(rahu), black_box(sun), black_box(lagna)),
    );
    bench_pair(
        &mut group,
        "kshetra_sphuta",
        || {
            dhruv_vedic_base::kshetra_sphuta(
                black_box(venus),
                black_box(moon),
                black_box(mars),
                black_box(jupiter),
                black_box(lagna),
            )
        },
        || {
            dhruv_ffi_c::dhruv_kshetra_sphuta(
                black_box(venus),
                black_box(moon),
                black_box(mars),
                black_box(jupiter),
                black_box(lagna),
            )
        },
    );
    bench_pair(
        &mut group,
        "beeja_sphuta",
        || dhruv_vedic_base::beeja_sphuta(black_box(sun), black_box(venus), black_box(jupiter)),
        || dhruv_ffi_c::dhruv_beeja_sphuta(black_box(sun), black_box(venus), black_box(jupiter)),
    );

    let trisphuta_val = dhruv_vedic_base::trisphuta(lagna, moon, gulika);
    let chatussphuta_val = dhruv_vedic_base::chatussphuta(trisphuta_val, sun);
    bench_pair(
        &mut group,
        "trisphuta",
        || dhruv_vedic_base::trisphuta(black_box(lagna), black_box(moon), black_box(gulika)),
        || dhruv_ffi_c::dhruv_trisphuta(black_box(lagna), black_box(moon), black_box(gulika)),
    );
    bench_pair(
        &mut group,
        "chatussphuta",
        || dhruv_vedic_base::chatussphuta(black_box(trisphuta_val), black_box(sun)),
        || dhruv_ffi_c::dhruv_chatussphuta(black_box(trisphuta_val), black_box(sun)),
    );
    bench_pair(
        &mut group,
        "panchasphuta",
        || dhruv_vedic_base::panchasphuta(black_box(chatussphuta_val), black_box(rahu)),
        || dhruv_ffi_c::dhruv_panchasphuta(black_box(chatussphuta_val), black_box(rahu)),
    );
    bench_pair(
        &mut group,
        "sookshma_trisphuta",
        || {
            dhruv_vedic_base::sookshma_trisphuta(
                black_box(lagna),
                black_box(moon),
                black_box(gulika),
                black_box(sun),
            )
        },
        || {
            dhruv_ffi_c::dhruv_sookshma_trisphuta(
                black_box(lagna),
                black_box(moon),
                black_box(gulika),
                black_box(sun),
            )
        },
    );
    bench_pair(
        &mut group,
        "avayoga_sphuta",
        || dhruv_vedic_base::avayoga_sphuta(black_box(sun), black_box(moon)),
        || dhruv_ffi_c::dhruv_avayoga_sphuta(black_box(sun), black_box(moon)),
    );
    bench_pair(
        &mut group,
        "kunda",
        || dhruv_vedic_base::kunda(black_box(lagna), black_box(moon), black_box(mars)),
        || dhruv_ffi_c::dhruv_kunda(black_box(lagna), black_box(moon), black_box(mars)),
    );

    let ghatikas = 23.5_f64;
    let vighatikas = 120.0_f64;
    let hora_lagna_lon = dhruv_vedic_base::hora_lagna(sun, ghatikas);
    bench_pair(
        &mut group,
        "bhava_lagna",
        || dhruv_vedic_base::bhava_lagna(black_box(sun), black_box(ghatikas)),
        || dhruv_ffi_c::dhruv_bhava_lagna(black_box(sun), black_box(ghatikas)),
    );
    bench_pair(
        &mut group,
        "hora_lagna",
        || dhruv_vedic_base::hora_lagna(black_box(sun), black_box(ghatikas)),
        || dhruv_ffi_c::dhruv_hora_lagna(black_box(sun), black_box(ghatikas)),
    );
    bench_pair(
        &mut group,
        "ghati_lagna",
        || dhruv_vedic_base::ghati_lagna(black_box(sun), black_box(ghatikas)),
        || dhruv_ffi_c::dhruv_ghati_lagna(black_box(sun), black_box(ghatikas)),
    );
    bench_pair(
        &mut group,
        "vighati_lagna",
        || dhruv_vedic_base::vighati_lagna(black_box(lagna), black_box(vighatikas)),
        || dhruv_ffi_c::dhruv_vighati_lagna(black_box(lagna), black_box(vighatikas)),
    );
    bench_pair(
        &mut group,
        "varnada_lagna",
        || dhruv_vedic_base::varnada_lagna(black_box(lagna), black_box(hora_lagna_lon)),
        || dhruv_ffi_c::dhruv_varnada_lagna(black_box(lagna), black_box(hora_lagna_lon)),
    );
    bench_pair(
        &mut group,
        "sree_lagna",
        || dhruv_vedic_base::sree_lagna(black_box(moon), black_box(lagna)),
        || dhruv_ffi_c::dhruv_sree_lagna(black_box(moon), black_box(lagna)),
    );
    bench_pair(
        &mut group,
        "pranapada_lagna",
        || dhruv_vedic_base::pranapada_lagna(black_box(sun), black_box(ghatikas)),
        || dhruv_ffi_c::dhruv_pranapada_lagna(black_box(sun), black_box(ghatikas)),
    );
    bench_pair(
        &mut group,
        "indu_lagna",
        || {
            dhruv_vedic_base::indu_lagna(
                black_box(moon),
                black_box(dhruv_vedic_base::Graha::Surya),
                black_box(dhruv_vedic_base::Graha::Chandra),
            )
        },
        || dhruv_ffi_c::dhruv_indu_lagna(black_box(moon), 0, 1),
    );

    group.finish();
}

criterion_group!(
    benches,
    ffi_query_bench,
    ffi_core_cabi_bench,
    ffi_time_frame_bench,
    ffi_vedic_primitives_bench,
    ffi_classifier_bench,
    ffi_ashtakavarga_bench,
    ffi_drishti_bench,
    ffi_ghatika_hora_bench,
    ffi_upagraha_bench,
    ffi_search_sidereal_bench,
    ffi_search_conjunction_grahan_bench,
    ffi_search_stationary_bench,
    ffi_lunar_sankranti_bench,
    ffi_riseset_bhava_bench,
    ffi_panchang_runtime_bench,
    ffi_orchestrators_bench,
    ffi_scalar_wrappers_bench
);
criterion_main!(benches);
