//! Dasha orchestration: bridges the ephemeris engine with the pure-math
//! dasha computation in dhruv_vedic_base.
//!
//! Provides two top-level entry points:
//! - `dasha_hierarchy_for_birth`: computes full hierarchy (levels 0..N)
//! - `dasha_snapshot_at`: finds active periods at a query time (efficient)

use dhruv_core::Engine;
use dhruv_time::{EopKernel, UtcTime};
use dhruv_vedic_base::BhavaConfig;
use dhruv_vedic_base::dasha::{
    BirthPeriod, DashaHierarchy, DashaSnapshot, DashaSystem, DashaVariationConfig,
    RashiDashaInputs, chakra_hierarchy, chakra_snapshot, chara_hierarchy, chara_snapshot,
    driga_hierarchy, driga_snapshot, kaal_chakra_hierarchy, kaal_chakra_snapshot, kala_hierarchy,
    kala_snapshot, karaka_kendradi_graha_hierarchy, karaka_kendradi_graha_snapshot,
    karaka_kendradi_hierarchy, karaka_kendradi_snapshot, kendradi_hierarchy, kendradi_snapshot,
    mandooka_hierarchy, mandooka_snapshot, nakshatra_config_for_system, nakshatra_hierarchy,
    nakshatra_snapshot, shoola_hierarchy, shoola_snapshot, sthira_hierarchy, sthira_snapshot,
    yogardha_hierarchy, yogardha_snapshot, yogini_config, yogini_hierarchy, yogini_snapshot,
};
use dhruv_vedic_base::riseset::compute_rise_set;
use dhruv_vedic_base::riseset_types::{GeoLocation, RiseSetConfig, RiseSetEvent, RiseSetResult};

use dhruv_frames::{ReferencePlane, ecliptic_lon_to_invariable_lon};

use crate::error::SearchError;
use crate::jyotish::graha_sidereal_longitudes_with_model;
use crate::panchang::moon_sidereal_longitude_at;
use crate::sankranti_types::SankrantiConfig;

/// Check if a dasha system is rashi-based.
pub(crate) fn is_rashi_system(system: DashaSystem) -> bool {
    matches!(
        system,
        DashaSystem::Chara
            | DashaSystem::Sthira
            | DashaSystem::Yogardha
            | DashaSystem::Driga
            | DashaSystem::Shoola
            | DashaSystem::Mandooka
            | DashaSystem::Chakra
            | DashaSystem::Kendradi
            | DashaSystem::KarakaKendradi
            | DashaSystem::KarakaKendradiGraha
    )
}

/// Check if a dasha system needs the Moon's sidereal longitude.
///
/// Returns true for nakshatra-based (10), Yogini, and KaalChakra systems.
/// Returns false for rashi-based (10) and Kala systems.
pub(crate) fn needs_moon_lon(system: DashaSystem) -> bool {
    !is_rashi_system(system) && system != DashaSystem::Kala
}

/// Check if a dasha system needs sunrise/sunset data.
///
/// Kala requires sunrise/sunset for its time-division algorithm.
/// Chakra optionally uses it for BirthPeriod determination.
pub(crate) fn needs_sunrise_sunset(system: DashaSystem) -> bool {
    system == DashaSystem::Kala || system == DashaSystem::Chakra
}

/// Classify birth period from sunrise/sunset JDs.
///
/// IMPORTANT: All three JD values MUST be in the same timescale.
/// Currently the codebase has a pre-existing mismatch (birth_jd is UTC,
/// sunrise/sunset are TDB — ~69s delta). This function is timescale-agnostic;
/// callers are responsible for consistency. See Risk R1.
///
/// Note: `BirthPeriod::Twilight` is not returned by this function. Computing
/// twilight requires solar depression angle utilities not yet available.
/// TODO: add Twilight classification when solar depression angle utilities are available
fn determine_birth_period(birth_jd: f64, sunrise_jd: f64, sunset_jd: f64) -> BirthPeriod {
    // KNOWN: UTC/TDB mismatch, see Risk R1
    if birth_jd >= sunrise_jd && birth_jd < sunset_jd {
        BirthPeriod::Day
    } else {
        BirthPeriod::Night
    }
}

/// Compute the Moon's sidereal longitude for dasha birth balance.
fn moon_sidereal_lon(
    engine: &Engine,
    _eop: &EopKernel,
    utc: &UtcTime,
    aya_config: &SankrantiConfig,
) -> Result<f64, SearchError> {
    let jd_tdb = utc.to_jd_tdb(engine.lsk());
    moon_sidereal_longitude_at(engine, jd_tdb, aya_config)
}

/// Assemble RashiDashaInputs from engine queries.
///
/// Computes sidereal longitudes for all 9 grahas and lagna, then builds
/// whole-sign house assignments.
fn assemble_rashi_inputs(
    engine: &Engine,
    eop: &EopKernel,
    utc: &UtcTime,
    location: &GeoLocation,
    aya_config: &SankrantiConfig,
) -> Result<RashiDashaInputs, SearchError> {
    let jd_tdb = utc.to_jd_tdb(engine.lsk());
    let graha_lons = graha_sidereal_longitudes_with_model(
        engine,
        jd_tdb,
        aya_config.ayanamsha_system,
        aya_config.use_nutation,
        aya_config.precession_model,
        aya_config.reference_plane,
    )?;

    let jd_utc = utc_to_jd_utc(utc);
    let lagna_rad = dhruv_vedic_base::lagna_longitude_rad(engine.lsk(), eop, location, jd_utc)?;
    let lagna_ecl_deg = lagna_rad.to_degrees();

    // Project lagna to reference plane before subtracting ayanamsha
    let lagna_on_plane = match aya_config.reference_plane {
        ReferencePlane::Ecliptic => lagna_ecl_deg,
        ReferencePlane::Invariable => ecliptic_lon_to_invariable_lon(lagna_ecl_deg),
    };

    let t = dhruv_vedic_base::ayanamsha::jd_tdb_to_centuries(jd_tdb);
    let aya = aya_config.ayanamsha_deg_at_centuries(t);
    let lagna_sid = dhruv_vedic_base::util::normalize_360(lagna_on_plane - aya);

    Ok(RashiDashaInputs::new(graha_lons.longitudes, lagna_sid))
}

/// Compute sunrise and sunset JD for Kala dasha.
///
/// Uses the birth date's local noon as search seed for both events.
fn compute_birth_sunrise_sunset(
    engine: &Engine,
    eop: &EopKernel,
    birth_utc: &UtcTime,
    location: &GeoLocation,
    riseset_config: &RiseSetConfig,
) -> Result<(f64, f64), SearchError> {
    let jd_utc = utc_to_jd_utc(birth_utc);
    let jd_midnight = jd_utc.floor() + 0.5;
    let jd_noon = dhruv_vedic_base::approximate_local_noon_jd(jd_midnight, location.longitude_deg);

    let sunrise_result = compute_rise_set(
        engine,
        engine.lsk(),
        eop,
        location,
        RiseSetEvent::Sunrise,
        jd_noon,
        riseset_config,
    )
    .map_err(|_| SearchError::NoConvergence("sunrise computation failed"))?;
    let sunrise_jd = match sunrise_result {
        RiseSetResult::Event { jd_tdb, .. } => jd_tdb,
        _ => {
            return Err(SearchError::NoConvergence(
                "sun never rises at this location",
            ));
        }
    };

    let sunset_result = compute_rise_set(
        engine,
        engine.lsk(),
        eop,
        location,
        RiseSetEvent::Sunset,
        jd_noon,
        riseset_config,
    )
    .map_err(|_| SearchError::NoConvergence("sunset computation failed"))?;
    let sunset_jd = match sunset_result {
        RiseSetResult::Event { jd_tdb, .. } => jd_tdb,
        _ => {
            return Err(SearchError::NoConvergence(
                "sun never sets at this location",
            ));
        }
    };

    Ok((sunrise_jd, sunset_jd))
}

/// Convert UtcTime to JD UTC (calendar only, no TDB).
fn utc_to_jd_utc(utc: &UtcTime) -> f64 {
    let y = utc.year as f64;
    let m = utc.month as f64;
    let d =
        utc.day as f64 + utc.hour as f64 / 24.0 + utc.minute as f64 / 1440.0 + utc.second / 86400.0;

    let (y2, m2) = if m <= 2.0 {
        (y - 1.0, m + 12.0)
    } else {
        (y, m)
    };
    let a = (y2 / 100.0).floor();
    let b = 2.0 - a + (a / 4.0).floor();

    (365.25 * (y2 + 4716.0)).floor() + (30.6001 * (m2 + 1.0)).floor() + d + b - 1524.5
}

/// Dispatch to the correct dasha engine for a given system.
fn dispatch_hierarchy(
    system: DashaSystem,
    birth_jd: f64,
    moon_sid_lon: f64,
    rashi_inputs: Option<&RashiDashaInputs>,
    sunrise_sunset: Option<(f64, f64)>,
    max_level: u8,
    variation: &DashaVariationConfig,
) -> Result<DashaHierarchy, SearchError> {
    // Try nakshatra-based systems first (10 systems)
    if let Some(cfg) = nakshatra_config_for_system(system) {
        return nakshatra_hierarchy(birth_jd, moon_sid_lon, &cfg, max_level, variation)
            .map_err(SearchError::from);
    }

    match system {
        DashaSystem::Yogini => {
            let cfg = yogini_config();
            yogini_hierarchy(birth_jd, moon_sid_lon, &cfg, max_level, variation)
                .map_err(SearchError::from)
        }
        // Rashi-based systems
        DashaSystem::Chara => {
            let ri = rashi_inputs.ok_or(SearchError::InvalidConfig("rashi inputs required"))?;
            chara_hierarchy(birth_jd, ri, max_level, variation).map_err(SearchError::from)
        }
        DashaSystem::Sthira => {
            let ri = rashi_inputs.ok_or(SearchError::InvalidConfig("rashi inputs required"))?;
            sthira_hierarchy(birth_jd, ri, max_level, variation).map_err(SearchError::from)
        }
        DashaSystem::Yogardha => {
            let ri = rashi_inputs.ok_or(SearchError::InvalidConfig("rashi inputs required"))?;
            yogardha_hierarchy(birth_jd, ri, max_level, variation).map_err(SearchError::from)
        }
        DashaSystem::Driga => {
            let ri = rashi_inputs.ok_or(SearchError::InvalidConfig("rashi inputs required"))?;
            driga_hierarchy(birth_jd, ri, max_level, variation).map_err(SearchError::from)
        }
        DashaSystem::Shoola => {
            let ri = rashi_inputs.ok_or(SearchError::InvalidConfig("rashi inputs required"))?;
            shoola_hierarchy(birth_jd, ri, max_level, variation).map_err(SearchError::from)
        }
        DashaSystem::Mandooka => {
            let ri = rashi_inputs.ok_or(SearchError::InvalidConfig("rashi inputs required"))?;
            mandooka_hierarchy(birth_jd, ri, max_level, variation).map_err(SearchError::from)
        }
        DashaSystem::Chakra => {
            let ri = rashi_inputs.ok_or(SearchError::InvalidConfig("rashi inputs required"))?;
            let period = if let Some((sunrise, sunset)) = sunrise_sunset {
                determine_birth_period(birth_jd, sunrise, sunset)
            } else {
                BirthPeriod::Day // fallback when sunrise/sunset not available
            };
            chakra_hierarchy(birth_jd, ri, period, max_level, variation).map_err(SearchError::from)
        }
        DashaSystem::Kendradi => {
            let ri = rashi_inputs.ok_or(SearchError::InvalidConfig("rashi inputs required"))?;
            kendradi_hierarchy(birth_jd, ri, max_level, variation).map_err(SearchError::from)
        }
        DashaSystem::KarakaKendradi => {
            let ri = rashi_inputs.ok_or(SearchError::InvalidConfig("rashi inputs required"))?;
            karaka_kendradi_hierarchy(birth_jd, ri, max_level, variation).map_err(SearchError::from)
        }
        DashaSystem::KarakaKendradiGraha => {
            let ri = rashi_inputs.ok_or(SearchError::InvalidConfig("rashi inputs required"))?;
            karaka_kendradi_graha_hierarchy(birth_jd, ri, max_level, variation)
                .map_err(SearchError::from)
        }
        DashaSystem::Kala => {
            let (sunrise, sunset) = sunrise_sunset.ok_or(SearchError::InvalidConfig(
                "sunrise/sunset required for Kala dasha",
            ))?;
            kala_hierarchy(birth_jd, sunrise, sunset, max_level, variation)
                .map_err(SearchError::from)
        }
        DashaSystem::KaalChakra => {
            kaal_chakra_hierarchy(birth_jd, moon_sid_lon, max_level, variation)
                .map_err(SearchError::from)
        }
        // Nakshatra-based systems already handled above
        _ => unreachable!("nakshatra systems handled before match"),
    }
}

/// Dispatch to the correct snapshot engine for a given system.
#[allow(clippy::too_many_arguments)]
fn dispatch_snapshot(
    system: DashaSystem,
    birth_jd: f64,
    moon_sid_lon: f64,
    rashi_inputs: Option<&RashiDashaInputs>,
    sunrise_sunset: Option<(f64, f64)>,
    query_jd: f64,
    max_level: u8,
    variation: &DashaVariationConfig,
) -> Result<DashaSnapshot, SearchError> {
    // Try nakshatra-based systems first (10 systems)
    if let Some(cfg) = nakshatra_config_for_system(system) {
        return Ok(nakshatra_snapshot(
            birth_jd,
            moon_sid_lon,
            &cfg,
            query_jd,
            max_level,
            variation,
        ));
    }

    match system {
        DashaSystem::Yogini => {
            let cfg = yogini_config();
            Ok(yogini_snapshot(
                birth_jd,
                moon_sid_lon,
                &cfg,
                query_jd,
                max_level,
                variation,
            ))
        }
        // Rashi-based systems
        DashaSystem::Chara => {
            let ri = rashi_inputs.ok_or(SearchError::InvalidConfig("rashi inputs required"))?;
            Ok(chara_snapshot(birth_jd, ri, query_jd, max_level, variation))
        }
        DashaSystem::Sthira => {
            let ri = rashi_inputs.ok_or(SearchError::InvalidConfig("rashi inputs required"))?;
            Ok(sthira_snapshot(
                birth_jd, ri, query_jd, max_level, variation,
            ))
        }
        DashaSystem::Yogardha => {
            let ri = rashi_inputs.ok_or(SearchError::InvalidConfig("rashi inputs required"))?;
            Ok(yogardha_snapshot(
                birth_jd, ri, query_jd, max_level, variation,
            ))
        }
        DashaSystem::Driga => {
            let ri = rashi_inputs.ok_or(SearchError::InvalidConfig("rashi inputs required"))?;
            Ok(driga_snapshot(birth_jd, ri, query_jd, max_level, variation))
        }
        DashaSystem::Shoola => {
            let ri = rashi_inputs.ok_or(SearchError::InvalidConfig("rashi inputs required"))?;
            Ok(shoola_snapshot(
                birth_jd, ri, query_jd, max_level, variation,
            ))
        }
        DashaSystem::Mandooka => {
            let ri = rashi_inputs.ok_or(SearchError::InvalidConfig("rashi inputs required"))?;
            Ok(mandooka_snapshot(
                birth_jd, ri, query_jd, max_level, variation,
            ))
        }
        DashaSystem::Chakra => {
            let ri = rashi_inputs.ok_or(SearchError::InvalidConfig("rashi inputs required"))?;
            let period = if let Some((sunrise, sunset)) = sunrise_sunset {
                determine_birth_period(birth_jd, sunrise, sunset)
            } else {
                BirthPeriod::Day // fallback when sunrise/sunset not available
            };
            Ok(chakra_snapshot(
                birth_jd, ri, period, query_jd, max_level, variation,
            ))
        }
        DashaSystem::Kendradi => {
            let ri = rashi_inputs.ok_or(SearchError::InvalidConfig("rashi inputs required"))?;
            Ok(kendradi_snapshot(
                birth_jd, ri, query_jd, max_level, variation,
            ))
        }
        DashaSystem::KarakaKendradi => {
            let ri = rashi_inputs.ok_or(SearchError::InvalidConfig("rashi inputs required"))?;
            Ok(karaka_kendradi_snapshot(
                birth_jd, ri, query_jd, max_level, variation,
            ))
        }
        DashaSystem::KarakaKendradiGraha => {
            let ri = rashi_inputs.ok_or(SearchError::InvalidConfig("rashi inputs required"))?;
            Ok(karaka_kendradi_graha_snapshot(
                birth_jd, ri, query_jd, max_level, variation,
            ))
        }
        DashaSystem::Kala => {
            let (sunrise, sunset) = sunrise_sunset.ok_or(SearchError::InvalidConfig(
                "sunrise/sunset required for Kala dasha",
            ))?;
            Ok(kala_snapshot(
                birth_jd, sunrise, sunset, query_jd, max_level, variation,
            ))
        }
        DashaSystem::KaalChakra => Ok(kaal_chakra_snapshot(
            birth_jd,
            moon_sid_lon,
            query_jd,
            max_level,
            variation,
        )),
        // Nakshatra-based systems already handled above
        _ => unreachable!("nakshatra systems handled before match"),
    }
}

/// Compute full hierarchy for a birth chart.
#[allow(clippy::too_many_arguments)]
pub fn dasha_hierarchy_for_birth(
    engine: &Engine,
    eop: &EopKernel,
    birth_utc: &UtcTime,
    location: &GeoLocation,
    system: DashaSystem,
    max_level: u8,
    _bhava_config: &BhavaConfig,
    riseset_config: &RiseSetConfig,
    aya_config: &SankrantiConfig,
    variation: &DashaVariationConfig,
) -> Result<DashaHierarchy, SearchError> {
    let birth_jd = utc_to_jd_utc(birth_utc);

    let moon_sid_lon = if needs_moon_lon(system) {
        moon_sidereal_lon(engine, eop, birth_utc, aya_config)?
    } else {
        0.0 // sentinel: never read by rashi/Kala dispatch paths
    };

    let rashi_inputs = if is_rashi_system(system) {
        Some(assemble_rashi_inputs(
            engine, eop, birth_utc, location, aya_config,
        )?)
    } else {
        None
    };

    let sunrise_sunset = if needs_sunrise_sunset(system) {
        Some(compute_birth_sunrise_sunset(
            engine,
            eop,
            birth_utc,
            location,
            riseset_config,
        )?)
    } else {
        None
    };

    dispatch_hierarchy(
        system,
        birth_jd,
        moon_sid_lon,
        rashi_inputs.as_ref(),
        sunrise_sunset,
        max_level,
        variation,
    )
}

/// Find active periods at a specific time.
///
/// Snapshot-only path: does NOT materialize full hierarchy. Efficient for deep levels.
#[allow(clippy::too_many_arguments)]
pub fn dasha_snapshot_at(
    engine: &Engine,
    eop: &EopKernel,
    birth_utc: &UtcTime,
    query_utc: &UtcTime,
    location: &GeoLocation,
    system: DashaSystem,
    max_level: u8,
    _bhava_config: &BhavaConfig,
    riseset_config: &RiseSetConfig,
    aya_config: &SankrantiConfig,
    variation: &DashaVariationConfig,
) -> Result<DashaSnapshot, SearchError> {
    let birth_jd = utc_to_jd_utc(birth_utc);
    let query_jd = utc_to_jd_utc(query_utc);

    let moon_sid_lon = if needs_moon_lon(system) {
        moon_sidereal_lon(engine, eop, birth_utc, aya_config)?
    } else {
        0.0 // sentinel: never read by rashi/Kala dispatch paths
    };

    let rashi_inputs = if is_rashi_system(system) {
        Some(assemble_rashi_inputs(
            engine, eop, birth_utc, location, aya_config,
        )?)
    } else {
        None
    };

    let sunrise_sunset = if needs_sunrise_sunset(system) {
        Some(compute_birth_sunrise_sunset(
            engine,
            eop,
            birth_utc,
            location,
            riseset_config,
        )?)
    } else {
        None
    };

    dispatch_snapshot(
        system,
        birth_jd,
        moon_sid_lon,
        rashi_inputs.as_ref(),
        sunrise_sunset,
        query_jd,
        max_level,
        variation,
    )
}

/// Pre-computed inputs for context-sharing dasha computation.
///
/// Callers populate only the fields needed by the target system:
/// - `moon_sid_lon`: required for nakshatra-based, Yogini, KaalChakra
/// - `rashi_inputs`: required for rashi-based systems (10)
/// - `sunrise_sunset`: required for Kala, optional for Chakra (BirthPeriod)
#[derive(Debug, Clone, Default)]
pub struct DashaInputs<'a> {
    pub moon_sid_lon: Option<f64>,
    pub rashi_inputs: Option<&'a RashiDashaInputs>,
    pub sunrise_sunset: Option<(f64, f64)>,
}

/// Context-sharing hierarchy computation using pre-computed inputs.
///
/// Callers are responsible for populating the `DashaInputs` fields
/// needed by the target system. Missing required fields result in errors.
pub fn dasha_hierarchy_with_inputs(
    birth_jd: f64,
    system: DashaSystem,
    max_level: u8,
    variation: &DashaVariationConfig,
    inputs: &DashaInputs<'_>,
) -> Result<DashaHierarchy, SearchError> {
    let moon_sid_lon = inputs.moon_sid_lon.unwrap_or(0.0);
    dispatch_hierarchy(
        system,
        birth_jd,
        moon_sid_lon,
        inputs.rashi_inputs,
        inputs.sunrise_sunset,
        max_level,
        variation,
    )
}

/// Context-sharing snapshot computation using pre-computed inputs.
pub fn dasha_snapshot_with_inputs(
    birth_jd: f64,
    query_jd: f64,
    system: DashaSystem,
    max_level: u8,
    variation: &DashaVariationConfig,
    inputs: &DashaInputs<'_>,
) -> Result<DashaSnapshot, SearchError> {
    let moon_sid_lon = inputs.moon_sid_lon.unwrap_or(0.0);
    dispatch_snapshot(
        system,
        birth_jd,
        moon_sid_lon,
        inputs.rashi_inputs,
        inputs.sunrise_sunset,
        query_jd,
        max_level,
        variation,
    )
}

/// Context-sharing variant for full_kundali_for_date integration.
///
/// Takes a pre-computed Moon sidereal longitude to avoid redundant queries.
/// For rashi-based systems, also accepts optional RashiDashaInputs.
/// For Kala dasha, requires sunrise/sunset JDs.
#[deprecated(since = "0.2.0", note = "Use dasha_hierarchy_with_inputs instead")]
pub fn dasha_hierarchy_with_moon(
    birth_jd: f64,
    moon_sid_lon: f64,
    rashi_inputs: Option<&RashiDashaInputs>,
    sunrise_sunset: Option<(f64, f64)>,
    system: DashaSystem,
    max_level: u8,
    variation: &DashaVariationConfig,
) -> Result<DashaHierarchy, SearchError> {
    let inputs = DashaInputs {
        moon_sid_lon: Some(moon_sid_lon),
        rashi_inputs,
        sunrise_sunset,
    };
    dasha_hierarchy_with_inputs(birth_jd, system, max_level, variation, &inputs)
}

/// Context-sharing snapshot variant.
///
/// For Kala dasha, requires sunrise/sunset JDs.
#[deprecated(since = "0.2.0", note = "Use dasha_snapshot_with_inputs instead")]
#[allow(clippy::too_many_arguments)]
pub fn dasha_snapshot_with_moon(
    birth_jd: f64,
    moon_sid_lon: f64,
    rashi_inputs: Option<&RashiDashaInputs>,
    sunrise_sunset: Option<(f64, f64)>,
    query_jd: f64,
    system: DashaSystem,
    max_level: u8,
    variation: &DashaVariationConfig,
) -> Result<DashaSnapshot, SearchError> {
    let inputs = DashaInputs {
        moon_sid_lon: Some(moon_sid_lon),
        rashi_inputs,
        sunrise_sunset,
    };
    dasha_snapshot_with_inputs(birth_jd, query_jd, system, max_level, variation, &inputs)
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- needs_moon_lon tests (Phase A1) ---

    #[test]
    fn test_needs_moon_lon_nakshatra() {
        let systems = [
            DashaSystem::Vimshottari,
            DashaSystem::Ashtottari,
            DashaSystem::Shodsottari,
            DashaSystem::Dwadashottari,
            DashaSystem::Panchottari,
            DashaSystem::Shatabdika,
            DashaSystem::Chaturashiti,
            DashaSystem::DwisaptatiSama,
            DashaSystem::Shashtihayani,
            DashaSystem::ShatTrimshaSama,
        ];
        for system in systems {
            assert!(needs_moon_lon(system), "{system:?} should need moon lon");
        }
    }

    #[test]
    fn test_needs_moon_lon_yogini() {
        assert!(needs_moon_lon(DashaSystem::Yogini));
    }

    #[test]
    fn test_needs_moon_lon_kaalchakra() {
        assert!(needs_moon_lon(DashaSystem::KaalChakra));
    }

    #[test]
    fn test_needs_moon_lon_rashi() {
        let systems = [
            DashaSystem::Chara,
            DashaSystem::Sthira,
            DashaSystem::Yogardha,
            DashaSystem::Driga,
            DashaSystem::Shoola,
            DashaSystem::Mandooka,
            DashaSystem::Chakra,
            DashaSystem::Kendradi,
            DashaSystem::KarakaKendradi,
            DashaSystem::KarakaKendradiGraha,
        ];
        for system in systems {
            assert!(
                !needs_moon_lon(system),
                "{system:?} should NOT need moon lon"
            );
        }
    }

    #[test]
    fn test_needs_moon_lon_kala() {
        assert!(!needs_moon_lon(DashaSystem::Kala));
    }

    // --- needs_sunrise_sunset tests (Phase A2) ---

    #[test]
    fn test_needs_sunrise_sunset_kala() {
        assert!(needs_sunrise_sunset(DashaSystem::Kala));
    }

    #[test]
    fn test_needs_sunrise_sunset_chakra() {
        assert!(needs_sunrise_sunset(DashaSystem::Chakra));
    }

    #[test]
    fn test_needs_sunrise_sunset_others() {
        let others = [
            DashaSystem::Vimshottari,
            DashaSystem::Ashtottari,
            DashaSystem::Shodsottari,
            DashaSystem::Dwadashottari,
            DashaSystem::Panchottari,
            DashaSystem::Shatabdika,
            DashaSystem::Chaturashiti,
            DashaSystem::DwisaptatiSama,
            DashaSystem::Shashtihayani,
            DashaSystem::ShatTrimshaSama,
            DashaSystem::Yogini,
            DashaSystem::Chara,
            DashaSystem::Sthira,
            DashaSystem::Yogardha,
            DashaSystem::Driga,
            DashaSystem::Shoola,
            DashaSystem::Mandooka,
            DashaSystem::Kendradi,
            DashaSystem::KarakaKendradi,
            DashaSystem::KarakaKendradiGraha,
            DashaSystem::KaalChakra,
        ];
        for system in others {
            assert!(
                !needs_sunrise_sunset(system),
                "{system:?} should NOT need sunrise/sunset"
            );
        }
    }

    // --- determine_birth_period tests (Phase A2) ---

    #[test]
    fn test_determine_birth_period_day() {
        // Birth between sunrise and sunset
        assert_eq!(
            determine_birth_period(100.5, 100.0, 101.0),
            BirthPeriod::Day
        );
    }

    #[test]
    fn test_determine_birth_period_night_before() {
        // Birth before sunrise
        assert_eq!(
            determine_birth_period(99.5, 100.0, 101.0),
            BirthPeriod::Night
        );
    }

    #[test]
    fn test_determine_birth_period_night_after() {
        // Birth after sunset
        assert_eq!(
            determine_birth_period(101.5, 100.0, 101.0),
            BirthPeriod::Night
        );
    }

    #[test]
    fn test_determine_birth_period_exact_sunrise() {
        // birth == sunrise → Day (>= boundary)
        assert_eq!(
            determine_birth_period(100.0, 100.0, 101.0),
            BirthPeriod::Day
        );
    }

    #[test]
    fn test_determine_birth_period_exact_sunset() {
        // birth == sunset → Night (< boundary)
        assert_eq!(
            determine_birth_period(101.0, 100.0, 101.0),
            BirthPeriod::Night
        );
    }

    #[test]
    fn test_determine_birth_period_epsilon_before_sunrise() {
        let sunrise = 100.0;
        let birth = sunrise - 1e-10;
        assert_eq!(
            determine_birth_period(birth, sunrise, 101.0),
            BirthPeriod::Night
        );
    }

    #[test]
    fn test_determine_birth_period_epsilon_after_sunset() {
        let sunset = 101.0;
        let birth = sunset + 1e-10;
        assert_eq!(
            determine_birth_period(birth, 100.0, sunset),
            BirthPeriod::Night
        );
    }

    #[test]
    fn test_determine_birth_period_timescale_note() {
        // Documents ~69s UTC/TDB boundary as known limitation (R1).
        // In JD, 69s = 69/86400 ≈ 0.000799 days.
        // A birth_utc 30s before sunrise_tdb may still classify as Day
        // because UTC is ~69s behind TDB. This is a pre-existing mismatch.
        let sunrise_tdb = 2460000.25; // example sunrise in TDB
        let delta_69s = 69.0 / 86400.0;
        let birth_utc_30s_before = sunrise_tdb - (30.0 / 86400.0);
        // birth_utc is ~30s before sunrise_tdb, but in the same timescale
        // this would be Day (because birth >= sunrise is true).
        // In reality, birth_utc is ~69s behind TDB, so the actual birth
        // time in TDB would be birth_utc + delta_69s, which is ~39s AFTER
        // sunrise_tdb. This test documents the ambiguity.
        let _ = delta_69s; // acknowledged
        assert_eq!(
            determine_birth_period(birth_utc_30s_before, sunrise_tdb, sunrise_tdb + 0.5),
            BirthPeriod::Night // 30s before → Night in direct comparison
        );
    }
}
