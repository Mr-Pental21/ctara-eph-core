use std::path::PathBuf;

use criterion::{Criterion, black_box, criterion_group, criterion_main};
use dhruv_rs::{
    Body, EngineConfig, LunarPhaseKind, LunarPhaseRequest, LunarPhaseRequestQuery,
    LunarPhaseResult, Observer, TimeInput, UtcDate, init, is_initialized, longitude, lunar_phase,
};

fn ensure_init() -> bool {
    if is_initialized() {
        return true;
    }

    let base = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../kernels/data");
    let spk = base.join("de442s.bsp");
    let lsk = base.join("naif0012.tls");
    if !spk.exists() || !lsk.exists() {
        eprintln!("Skipping benchmarks: kernel files not found");
        return false;
    }

    let config = EngineConfig::with_single_spk(spk, lsk, 256, true);
    init(config).is_ok()
}

fn rs_query_bench(c: &mut Criterion) {
    if !ensure_init() {
        return;
    }
    let date = UtcDate::new(2024, 3, 20, 12, 0, 0.0);

    let mut group = c.benchmark_group("dhruv_rs_query");
    group.bench_function("longitude_mars_earth", |b| {
        b.iter(|| {
            longitude(Body::Mars, Observer::Body(Body::Earth), black_box(date))
                .expect("query should succeed")
        })
    });
    group.finish();
}

fn rs_search_bench(c: &mut Criterion) {
    if !ensure_init() {
        return;
    }
    let date = UtcDate::new(2024, 3, 20, 12, 0, 0.0);

    let mut group = c.benchmark_group("dhruv_rs_search");
    group.sample_size(20);
    group.bench_function("lunar_phase_next_purnima", |b| {
        b.iter(|| {
            let req = LunarPhaseRequest {
                kind: LunarPhaseKind::Purnima,
                query: LunarPhaseRequestQuery::Next {
                    at: TimeInput::Utc(black_box(date)),
                },
            };
            match lunar_phase(&req).expect("search should succeed") {
                LunarPhaseResult::Single(Some(_)) => {}
                other => panic!("unexpected result shape: {other:?}"),
            }
        })
    });
    group.finish();
}

criterion_group!(benches, rs_query_bench, rs_search_bench);
criterion_main!(benches);
