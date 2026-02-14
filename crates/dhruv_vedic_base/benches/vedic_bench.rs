use criterion::{Criterion, black_box, criterion_group, criterion_main};
use dhruv_vedic_base::{
    Amsha, AmshaRequest, AyanamshaSystem, Graha, LunarNode, NodeDignityPolicy, NodeMode,
    SHODASHAVARGA, amsha_longitude, amsha_longitudes, ayanamsha_deg, lunar_node_deg,
    nakshatra_from_tropical, rashi_from_tropical, tithi_from_elongation, yoga_from_sum,
};
use dhruv_vedic_base::shadbala::{
    KalaBalaInputs, ShadbalaInputs, all_shadbalas_from_inputs, shadbala_from_inputs,
};
use dhruv_vedic_base::vimsopaka::{SHODASAVARGA as SHODASAVARGA_WEIGHTS, all_vimsopaka_balas};

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

fn shadbala_bench(c: &mut Criterion) {
    let sidereal_lons = [280.0, 120.0, 15.0, 170.0, 220.0, 350.0, 195.0, 90.0, 270.0];
    let bhava_numbers = [10u8, 4, 1, 7, 1, 4, 7];
    let speeds = [1.0, 13.0, 0.5, 1.5, 0.12, 1.2, 0.08];
    let kala = KalaBalaInputs {
        is_daytime: true,
        day_night_fraction: 0.5,
        moon_sun_elongation: 120.0,
        year_lord: Graha::Surya,
        month_lord: Graha::Mangal,
        weekday_lord: Graha::Shukra,
        hora_lord: Graha::Buddh,
        graha_declinations: [23.0, 5.0, -15.0, 10.0, -20.0, 8.0, -3.0],
        sidereal_lons: [280.0, 120.0, 15.0, 170.0, 220.0, 350.0, 195.0],
    };
    let varga_rashi = [[9, 4, 0, 5, 7, 11, 6]; 7];
    let inputs = ShadbalaInputs {
        sidereal_lons,
        bhava_numbers,
        speeds,
        kala,
        varga_rashi_indices: varga_rashi,
    };

    let mut group = c.benchmark_group("shadbala");
    group.bench_function("shadbala_single", |b| {
        b.iter(|| shadbala_from_inputs(black_box(Graha::Surya), black_box(&inputs)))
    });
    group.bench_function("shadbala_all", |b| {
        b.iter(|| all_shadbalas_from_inputs(black_box(&inputs)))
    });
    group.finish();
}

fn vimsopaka_bench(c: &mut Criterion) {
    let sidereal_lons = [280.0, 120.0, 15.0, 170.0, 220.0, 350.0, 195.0, 90.0, 270.0];
    let policy = NodeDignityPolicy::SignLordBased;

    let mut group = c.benchmark_group("vimsopaka");
    let weights: &[_] = &SHODASAVARGA_WEIGHTS;
    group.bench_function("vimsopaka_shodasavarga_9", |b| {
        b.iter(|| {
            all_vimsopaka_balas(
                black_box(&sidereal_lons),
                black_box(weights),
                black_box(policy),
            )
        })
    });
    group.finish();
}

fn avastha_bench(c: &mut Criterion) {
    use dhruv_vedic_base::avastha::{
        AvasthaInputs, LajjitadiInputs, SayanadiInputs, all_avasthas,
    };
    use dhruv_vedic_base::{Dignity, GrahaDrishtiMatrix, graha_drishti_matrix};

    let sidereal_lons = [15.0, 120.0, 200.0, 300.0, 50.0, 160.0, 270.0, 335.0, 155.0];
    let rashi_indices = [0u8, 3, 6, 9, 1, 5, 8, 11, 5];
    let bhava_numbers = [1u8, 4, 7, 10, 2, 6, 9, 12, 5];
    let dignities = [
        Dignity::OwnSign, Dignity::Mitra, Dignity::Exalted, Dignity::Sama,
        Dignity::Debilitated, Dignity::Shatru, Dignity::Moolatrikone,
        Dignity::Sama, Dignity::Sama,
    ];
    let is_combust = [false; 9];
    let is_retrograde = [false, false, true, false, false, false, false, false, false];
    let lost_war = [false; 9];
    let drishti_matrix = graha_drishti_matrix(&sidereal_lons);

    let inputs = AvasthaInputs {
        sidereal_lons,
        rashi_indices,
        bhava_numbers,
        dignities,
        is_combust,
        is_retrograde,
        lost_war,
        lajjitadi: LajjitadiInputs {
            rashi_indices,
            bhava_numbers,
            dignities,
            drishti_matrix,
        },
        sayanadi: SayanadiInputs {
            nakshatra_indices: [1, 8, 14, 22, 3, 11, 20, 24, 11],
            navamsa_numbers: [2, 5, 7, 1, 4, 9, 3, 6, 8],
            janma_nakshatra: 8,
            birth_ghatikas: 25,
            lagna_rashi_number: 1,
        },
    };

    let mut group = c.benchmark_group("avastha");
    group.bench_function("all_avasthas_9", |b| {
        b.iter(|| all_avasthas(black_box(&inputs)))
    });
    group.finish();
}

criterion_group!(
    benches,
    ayanamsha_bench,
    zodiac_bench,
    panchang_primitives_bench,
    amsha_bench,
    shadbala_bench,
    vimsopaka_bench,
    avastha_bench
);
criterion_main!(benches);
