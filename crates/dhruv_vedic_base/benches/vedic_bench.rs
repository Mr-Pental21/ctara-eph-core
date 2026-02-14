use criterion::{Criterion, black_box, criterion_group, criterion_main};
use dhruv_vedic_base::{
    Amsha, AmshaRequest, AyanamshaSystem, LunarNode, NodeMode, SHODASHAVARGA, amsha_longitude,
    amsha_longitudes, ayanamsha_deg, lunar_node_deg, nakshatra_from_tropical, rashi_from_tropical,
    tithi_from_elongation, yoga_from_sum,
};

fn ayanamsha_bench(c: &mut Criterion) {
    let t = 0.24;

    let mut group = c.benchmark_group("ayanamsha");
    group.bench_function("lahiri_mean", |b| {
        b.iter(|| ayanamsha_deg(AyanamshaSystem::Lahiri, black_box(t), false))
    });
    group.bench_function("lahiri_true", |b| {
        b.iter(|| ayanamsha_deg(AyanamshaSystem::Lahiri, black_box(t), true))
    });
    group.finish();
}

fn zodiac_bench(c: &mut Criterion) {
    let tropical_lon = 123.456;
    let jd_tdb = 2_460_000.5;

    let mut group = c.benchmark_group("zodiac");
    group.bench_function("rashi_from_tropical", |b| {
        b.iter(|| {
            rashi_from_tropical(
                black_box(tropical_lon),
                AyanamshaSystem::Lahiri,
                black_box(jd_tdb),
                false,
            )
        })
    });
    group.bench_function("nakshatra_from_tropical", |b| {
        b.iter(|| {
            nakshatra_from_tropical(
                black_box(tropical_lon),
                AyanamshaSystem::Lahiri,
                black_box(jd_tdb),
                false,
            )
        })
    });
    group.finish();
}

fn panchang_primitives_bench(c: &mut Criterion) {
    let elong = 211.75;
    let sum = 278.31;
    let t = 0.24;

    let mut group = c.benchmark_group("panchang_primitives");
    group.bench_function("tithi_from_elongation", |b| {
        b.iter(|| tithi_from_elongation(black_box(elong)))
    });
    group.bench_function("yoga_from_sum", |b| {
        b.iter(|| yoga_from_sum(black_box(sum)))
    });
    group.bench_function("lunar_node_true_rahu", |b| {
        b.iter(|| lunar_node_deg(LunarNode::Rahu, black_box(t), NodeMode::True))
    });
    group.finish();
}

fn amsha_bench(c: &mut Criterion) {
    let lon = 123.456;

    let mut group = c.benchmark_group("amsha");
    group.bench_function("amsha_longitude_d9", |b| {
        b.iter(|| amsha_longitude(black_box(lon), Amsha::D9, None))
    });
    let requests: Vec<AmshaRequest> = SHODASHAVARGA.iter().map(|&a| AmshaRequest::new(a)).collect();
    group.bench_function("amsha_longitudes_shodashavarga", |b| {
        b.iter(|| amsha_longitudes(black_box(lon), black_box(&requests)))
    });
    group.finish();
}

criterion_group!(
    benches,
    ayanamsha_bench,
    zodiac_bench,
    panchang_primitives_bench,
    amsha_bench
);
criterion_main!(benches);
