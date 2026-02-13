//! Integration tests for SPK kernel loading and evaluation (require de442s.bsp).

use std::path::Path;

use jpl_kernel::SpkKernel;

fn kernel_path() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../kernels/data/de442s.bsp")
}

fn load_kernel() -> Option<SpkKernel> {
    let path = kernel_path();
    if !path.exists() {
        eprintln!("Skipping: kernel not found at {}", path.display());
        return None;
    }
    Some(SpkKernel::load(&path).expect("should load de442s.bsp"))
}

#[test]
fn load_de442s_segments() {
    let kernel = match load_kernel() {
        Some(k) => k,
        None => return,
    };
    let segments = kernel.segments();

    // DE442s should have segments for the major bodies.
    assert!(
        segments.len() >= 13,
        "expected at least 13 segments, got {}",
        segments.len()
    );

    // All segments should be Type 2.
    for seg in segments {
        assert_eq!(seg.data_type, 2, "expected Type 2, got {}", seg.data_type);
        assert_eq!(seg.frame, 1, "expected J2000 frame (1), got {}", seg.frame);
    }

    // Check that we have a segment for Earth (399) with center EMB (3).
    let earth_seg = segments
        .iter()
        .find(|s| s.target == 399)
        .expect("should have Earth segment");
    assert_eq!(earth_seg.center, 3);
}

#[test]
fn evaluate_mars_barycenter_at_j2000() {
    let kernel = match load_kernel() {
        Some(k) => k,
        None => return,
    };
    // Mars Barycenter (4) relative to SSB (0) at J2000.0 (epoch = 0.0)
    let eval = kernel
        .evaluate(4, 0, 0.0)
        .expect("should evaluate Mars Bary at J2000");

    // Sanity: Mars should be roughly 1-3 AU from SSB.
    let r =
        (eval.position_km[0].powi(2) + eval.position_km[1].powi(2) + eval.position_km[2].powi(2))
            .sqrt();
    let au_km = 1.496e8;
    assert!(
        r > 0.5 * au_km && r < 4.0 * au_km,
        "Mars distance {r:.0} km not in expected range"
    );

    // Velocity should be nonzero and reasonable (< 100 km/s).
    let v = (eval.velocity_km_s[0].powi(2)
        + eval.velocity_km_s[1].powi(2)
        + eval.velocity_km_s[2].powi(2))
    .sqrt();
    assert!(
        v > 0.1 && v < 100.0,
        "Mars velocity {v:.3} km/s out of range"
    );
}

#[test]
fn resolve_mars_to_ssb() {
    let kernel = match load_kernel() {
        Some(k) => k,
        None => return,
    };

    // Mars (499) → Mars Bary (4) → SSB (0)
    let state = kernel
        .resolve_to_ssb(499, 0.0)
        .expect("should resolve Mars to SSB");

    let r = (state[0].powi(2) + state[1].powi(2) + state[2].powi(2)).sqrt();
    let au_km = 1.496e8;
    assert!(
        r > 0.5 * au_km && r < 4.0 * au_km,
        "Mars(499) SSB distance {r:.0} km out of range"
    );
}
