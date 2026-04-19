//! Consistency tests for the embedded-catalog ayanamsha pipeline.
//!
//! Verifies:
//! - Default path == explicit embedded catalog path (no semantic gap)
//! - Static (hardcoded) and default (catalog) agree within tight bounds
//! - Jagganatha on the invariable plane uses the catalog path

use dhruv_frames::{PrecessionModel, ReferencePlane};
use dhruv_tara::TaraCatalog;
use dhruv_vedic_base::{
    AyanamshaSystem, ayanamsha_deg_on_plane, ayanamsha_deg_with_catalog_on_plane,
    ayanamsha_mean_deg_static_on_plane, ayanamsha_mean_deg_with_catalog_and_model,
    ayanamsha_mean_deg_with_model,
};

/// ayanamsha_mean_deg() == ayanamsha_mean_deg_with_catalog(Some(embedded))
/// for all systems at all epochs.
#[test]
fn default_matches_explicit_catalog_exactly() {
    let model = PrecessionModel::Iau2006;
    for t in [-2.0, -1.0, 0.0, 0.24, 0.5, 1.0, 2.0] {
        for &sys in AyanamshaSystem::all() {
            let default_aya = ayanamsha_mean_deg_with_model(sys, t, model);
            let explicit_aya = ayanamsha_mean_deg_with_catalog_and_model(
                sys,
                t,
                Some(TaraCatalog::embedded()),
                model,
            );
            assert!(
                (default_aya - explicit_aya).abs() < 1e-15,
                "{sys:?} t={t}: default={default_aya}, explicit={explicit_aya}"
            );
        }
    }
}

/// At J2000, static and catalog should agree within 1" since the static
/// coordinates were derived from the same catalog.
#[test]
fn static_and_catalog_bounded_at_j2000() {
    let model = PrecessionModel::Iau2006;
    let t = 0.0;
    for &sys in AyanamshaSystem::all() {
        let default_aya = ayanamsha_mean_deg_with_model(sys, t, model);
        let static_aya = dhruv_vedic_base::ayanamsha_mean_deg_static_with_model(sys, t, model);
        let diff_arcsec = (default_aya - static_aya).abs() * 3600.0;
        assert!(
            diff_arcsec < 1.0,
            "{sys:?}: default={default_aya:.6}, static={static_aya:.6} (diff={diff_arcsec:.2}\")"
        );
    }
}

/// Across ±2 centuries, static and catalog should agree within 10"
/// (proper motion drift accumulates over longer propagation intervals).
#[test]
fn static_and_catalog_bounded_across_epochs() {
    let model = PrecessionModel::Iau2006;
    for &sys in AyanamshaSystem::all() {
        for t in [-2.0, -1.0, 0.0, 0.5, 1.0, 2.0] {
            let default_aya = ayanamsha_mean_deg_with_model(sys, t, model);
            let static_aya = dhruv_vedic_base::ayanamsha_mean_deg_static_with_model(sys, t, model);
            let diff_arcsec = (default_aya - static_aya).abs() * 3600.0;
            assert!(
                diff_arcsec < 10.0,
                "{sys:?} t={t}: default={default_aya:.6}° vs static={static_aya:.6}° \
                 (diff={diff_arcsec:.1}\")"
            );
        }
    }
}

/// Verify Jagganatha goes through catalog path on invariable plane.
#[test]
fn jagganatha_default_uses_catalog_on_invariable_plane() {
    let t = 0.24;
    let model = PrecessionModel::Iau2006;
    let plane = ReferencePlane::Invariable;

    let default_aya = ayanamsha_deg_on_plane(AyanamshaSystem::Jagganatha, t, false, model, plane);
    let explicit_cat_aya = ayanamsha_deg_with_catalog_on_plane(
        AyanamshaSystem::Jagganatha,
        t,
        false,
        Some(TaraCatalog::embedded()),
        model,
        plane,
    );
    assert!(
        (default_aya - explicit_cat_aya).abs() < 1e-15,
        "Jagganatha invariable default={default_aya}, explicit={explicit_cat_aya}"
    );
}

/// Static (hardcoded) vs default (catalog) for Jagganatha ON THE INVARIABLE PLANE.
/// Should differ by < 5" (same catalog source, slight proper motion delta).
#[test]
fn jagganatha_static_vs_default_bounded_on_invariable() {
    let t = 0.0;
    let model = PrecessionModel::Iau2006;
    let plane = ReferencePlane::Invariable;

    let default_aya = ayanamsha_deg_on_plane(AyanamshaSystem::Jagganatha, t, false, model, plane);
    let static_aya =
        ayanamsha_mean_deg_static_on_plane(AyanamshaSystem::Jagganatha, t, model, plane);
    let diff = (default_aya - static_aya).abs() * 3600.0;
    assert!(
        diff < 5.0,
        "Jagganatha invariable default vs static: {diff}\""
    );
}
