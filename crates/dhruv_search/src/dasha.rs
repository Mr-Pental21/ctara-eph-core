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
    driga_hierarchy, driga_snapshot, karaka_kendradi_graha_hierarchy,
    karaka_kendradi_graha_snapshot, karaka_kendradi_hierarchy, karaka_kendradi_snapshot,
    kendradi_hierarchy, kendradi_snapshot, mandooka_hierarchy, mandooka_snapshot,
    nakshatra_config_for_system, nakshatra_hierarchy, nakshatra_snapshot, shoola_hierarchy,
    shoola_snapshot, sthira_hierarchy, sthira_snapshot, yogardha_hierarchy, yogardha_snapshot,
    yogini_config, yogini_hierarchy, yogini_snapshot,
};
use dhruv_vedic_base::riseset_types::{GeoLocation, RiseSetConfig};

use crate::error::SearchError;
use crate::jyotish::graha_sidereal_longitudes;
use crate::panchang::moon_sidereal_longitude_at;
use crate::sankranti_types::SankrantiConfig;

/// Check if a dasha system is rashi-based.
fn is_rashi_system(system: DashaSystem) -> bool {
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
    let graha_lons = graha_sidereal_longitudes(
        engine,
        jd_tdb,
        aya_config.ayanamsha_system,
        aya_config.use_nutation,
    )?;

    let jd_utc = utc_to_jd_utc(utc);
    let lagna_rad = dhruv_vedic_base::lagna_longitude_rad(engine.lsk(), eop, location, jd_utc)?;
    let t = dhruv_vedic_base::ayanamsha::jd_tdb_to_centuries(jd_tdb);
    let aya = dhruv_vedic_base::ayanamsha::ayanamsha_deg(
        aya_config.ayanamsha_system,
        t,
        aya_config.use_nutation,
    );
    let lagna_sid = dhruv_vedic_base::util::normalize_360(lagna_rad.to_degrees() - aya);

    Ok(RashiDashaInputs::new(graha_lons.longitudes, lagna_sid))
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
            // Default to Day birth for Chakra (proper determination requires sunrise data)
            chakra_hierarchy(birth_jd, ri, BirthPeriod::Day, max_level, variation)
                .map_err(SearchError::from)
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
        _ => Err(SearchError::InvalidConfig(
            "dasha system not yet implemented",
        )),
    }
}

/// Dispatch to the correct snapshot engine for a given system.
fn dispatch_snapshot(
    system: DashaSystem,
    birth_jd: f64,
    moon_sid_lon: f64,
    rashi_inputs: Option<&RashiDashaInputs>,
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
            Ok(chakra_snapshot(
                birth_jd,
                ri,
                BirthPeriod::Day,
                query_jd,
                max_level,
                variation,
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
        _ => Err(SearchError::InvalidConfig(
            "dasha system not yet implemented",
        )),
    }
}

/// Compute full hierarchy for a birth chart.
pub fn dasha_hierarchy_for_birth(
    engine: &Engine,
    eop: &EopKernel,
    birth_utc: &UtcTime,
    location: &GeoLocation,
    system: DashaSystem,
    max_level: u8,
    _bhava_config: &BhavaConfig,
    _riseset_config: &RiseSetConfig,
    aya_config: &SankrantiConfig,
    variation: &DashaVariationConfig,
) -> Result<DashaHierarchy, SearchError> {
    let birth_jd = utc_to_jd_utc(birth_utc);
    let moon_sid_lon = moon_sidereal_lon(engine, eop, birth_utc, aya_config)?;

    let rashi_inputs = if is_rashi_system(system) {
        Some(assemble_rashi_inputs(
            engine, eop, birth_utc, location, aya_config,
        )?)
    } else {
        None
    };

    dispatch_hierarchy(
        system,
        birth_jd,
        moon_sid_lon,
        rashi_inputs.as_ref(),
        max_level,
        variation,
    )
}

/// Find active periods at a specific time.
///
/// Snapshot-only path: does NOT materialize full hierarchy. Efficient for deep levels.
pub fn dasha_snapshot_at(
    engine: &Engine,
    eop: &EopKernel,
    birth_utc: &UtcTime,
    query_utc: &UtcTime,
    location: &GeoLocation,
    system: DashaSystem,
    max_level: u8,
    _bhava_config: &BhavaConfig,
    _riseset_config: &RiseSetConfig,
    aya_config: &SankrantiConfig,
    variation: &DashaVariationConfig,
) -> Result<DashaSnapshot, SearchError> {
    let birth_jd = utc_to_jd_utc(birth_utc);
    let query_jd = utc_to_jd_utc(query_utc);
    let moon_sid_lon = moon_sidereal_lon(engine, eop, birth_utc, aya_config)?;

    let rashi_inputs = if is_rashi_system(system) {
        Some(assemble_rashi_inputs(
            engine, eop, birth_utc, location, aya_config,
        )?)
    } else {
        None
    };

    dispatch_snapshot(
        system,
        birth_jd,
        moon_sid_lon,
        rashi_inputs.as_ref(),
        query_jd,
        max_level,
        variation,
    )
}

/// Context-sharing variant for full_kundali_for_date integration.
///
/// Takes a pre-computed Moon sidereal longitude to avoid redundant queries.
/// For rashi-based systems, also accepts optional RashiDashaInputs.
pub fn dasha_hierarchy_with_moon(
    birth_jd: f64,
    moon_sid_lon: f64,
    rashi_inputs: Option<&RashiDashaInputs>,
    system: DashaSystem,
    max_level: u8,
    variation: &DashaVariationConfig,
) -> Result<DashaHierarchy, SearchError> {
    dispatch_hierarchy(
        system,
        birth_jd,
        moon_sid_lon,
        rashi_inputs,
        max_level,
        variation,
    )
}

/// Context-sharing snapshot variant.
pub fn dasha_snapshot_with_moon(
    birth_jd: f64,
    moon_sid_lon: f64,
    rashi_inputs: Option<&RashiDashaInputs>,
    query_jd: f64,
    system: DashaSystem,
    max_level: u8,
    variation: &DashaVariationConfig,
) -> Result<DashaSnapshot, SearchError> {
    dispatch_snapshot(
        system,
        birth_jd,
        moon_sid_lon,
        rashi_inputs,
        query_jd,
        max_level,
        variation,
    )
}
