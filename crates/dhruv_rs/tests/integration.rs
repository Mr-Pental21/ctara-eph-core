//! Integration tests for dhruv_rs context-first APIs (require kernels).

use std::path::PathBuf;

use dhruv_rs::*;

fn kernel_paths() -> (PathBuf, PathBuf) {
    let base = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../kernels/data");
    (base.join("de442s.bsp"), base.join("naif0012.tls"))
}

fn eop_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../kernels/data")
        .join("finals2000A.all")
}

fn kernels_available() -> bool {
    let (spk, lsk) = kernel_paths();
    spk.exists() && lsk.exists()
}

fn eop_available() -> bool {
    eop_path().exists()
}

fn make_context() -> Option<DhruvContext> {
    if !kernels_available() {
        eprintln!("Skipping: kernel files not found");
        return None;
    }
    let (spk, lsk) = kernel_paths();
    let config = EngineConfig::with_single_spk(spk, lsk, 256, true);
    Some(DhruvContext::new(config).expect("context init"))
}

fn load_eop() -> Option<EopKernel> {
    if !eop_available() {
        eprintln!("Skipping: EOP file not found");
        return None;
    }
    EopKernel::load(&eop_path()).ok()
}

#[test]
fn context_builds_with_engine() {
    if let Some(ctx) = make_context() {
        let _ = ctx.engine();
    }
}

#[test]
fn conjunction_next_runs() {
    let Some(ctx) = make_context() else {
        return;
    };

    let req = ConjunctionRequest {
        body1: Body::Sun,
        body2: Body::Mercury,
        config: Some(ConjunctionConfig::conjunction(1.0)),
        query: ConjunctionRequestQuery::Next {
            at: TimeInput::Utc(UtcDate::new(2024, 3, 20, 0, 0, 0.0)),
        },
    };

    let out = conjunction(&ctx, &req).expect("conjunction op should run");
    match out {
        ConjunctionResult::Single(_) => {}
        _ => panic!("expected single conjunction result"),
    }
}

#[test]
fn sankranti_range_runs() {
    let Some(ctx) = make_context() else {
        return;
    };

    let req = SankrantiRequest {
        target: SankrantiTarget::Any,
        config: Some(SankrantiConfig::default_lahiri()),
        query: SankrantiRequestQuery::Range {
            start: TimeInput::Utc(UtcDate::new(2024, 1, 1, 0, 0, 0.0)),
            end: TimeInput::Utc(UtcDate::new(2024, 12, 31, 0, 0, 0.0)),
        },
    };

    let out = sankranti(&ctx, &req).expect("sankranti op should run");
    match out {
        SankrantiResult::Many(v) => assert!(!v.is_empty()),
        _ => panic!("expected many sankranti results"),
    }
}

#[test]
fn context_time_policy_roundtrip() {
    let Some(mut ctx) = make_context() else {
        return;
    };

    let p = TimeConversionPolicy::StrictLsk;
    ctx.set_time_conversion_policy(p);
    assert_eq!(ctx.time_conversion_policy(), p);
}

#[test]
fn amsha_low_level_helpers_run() {
    let lon = amsha_longitude(45.0, Amsha::D9, None);
    assert!((0.0..360.0).contains(&lon));

    let requests = [AmshaRequest::new(Amsha::D1), AmshaRequest::new(Amsha::D9)];
    let lons = amsha_longitudes(45.0, &requests);
    assert_eq!(lons.len(), 2);
    assert!((lons[0] - 45.0).abs() < 1e-9);

    let info = amsha_rashi_info(45.0, Amsha::D9, None);
    assert_eq!(info.rashi_index, (lon / 30.0).floor() as u8);
}

#[test]
fn amsha_chart_for_date_runs() {
    let Some(ctx) = make_context() else {
        return;
    };
    let Some(eop) = load_eop() else {
        return;
    };

    let location = GeoLocation::new(28.6139, 77.2090, 0.0);
    let chart = amsha_chart_for_date(
        &ctx,
        &eop,
        UtcDate::new(2024, 1, 15, 12, 0, 0.0),
        &location,
        &BhavaConfig::default(),
        &RiseSetConfig::default(),
        AyanamshaSystem::Lahiri,
        false,
        Amsha::D9,
        AmshaVariation::TraditionalParashari,
        &AmshaChartScope::default(),
    )
    .expect("amsha chart should run");

    assert_eq!(chart.amsha, Amsha::D9);
    assert_eq!(chart.variation, AmshaVariation::TraditionalParashari);
    assert!(
        chart
            .grahas
            .iter()
            .all(|entry| { entry.sidereal_longitude >= 0.0 && entry.sidereal_longitude < 360.0 })
    );
    assert!(chart.lagna.sidereal_longitude >= 0.0 && chart.lagna.sidereal_longitude < 360.0);
}
