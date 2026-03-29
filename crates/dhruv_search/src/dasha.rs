//! Dasha orchestration: bridges the ephemeris engine with the pure-math
//! dasha computation in dhruv_vedic_base.
//!
//! Provides two top-level entry points:
//! - `dasha_hierarchy_for_birth`: computes full hierarchy (levels 0..N)
//! - `dasha_snapshot_at`: finds active periods at a query time (efficient)

use dhruv_core::Engine;
use dhruv_time::{EopKernel, UtcTime, jd_to_tdb_seconds, tdb_seconds_to_jd};
use dhruv_vedic_base::BhavaConfig;
use dhruv_vedic_base::dasha::{
    BirthPeriod, DashaEntity, DashaHierarchy, DashaLevel, DashaPeriod, DashaSnapshot, DashaSystem,
    DashaVariationConfig, RashiDashaInputs, SubPeriodMethod, chakra_hierarchy, chakra_level0,
    chakra_snapshot, chara_hierarchy, chara_level0, chara_period_years, chara_snapshot,
    driga_hierarchy, driga_level0, driga_snapshot, kaal_chakra_children,
    kaal_chakra_complete_level, kaal_chakra_hierarchy, kaal_chakra_level0, kaal_chakra_snapshot,
    kala_children, kala_complete_level, kala_hierarchy, kala_level0, kala_snapshot,
    karaka_kendradi_graha_hierarchy, karaka_kendradi_graha_snapshot, karaka_kendradi_hierarchy,
    karaka_kendradi_snapshot, kendradi_hierarchy, kendradi_level0, kendradi_snapshot,
    mandooka_children, mandooka_complete_level, mandooka_hierarchy, mandooka_level0,
    mandooka_snapshot, nakshatra_children, nakshatra_complete_level, nakshatra_config_for_system,
    nakshatra_hierarchy, nakshatra_level0, nakshatra_snapshot, shoola_hierarchy, shoola_level0,
    shoola_snapshot, sthira_hierarchy, sthira_level0, sthira_snapshot, yogardha_hierarchy,
    yogardha_level0, yogardha_snapshot, yogini_children, yogini_complete_level, yogini_config,
    yogini_hierarchy, yogini_level0, yogini_snapshot,
};
use dhruv_vedic_base::riseset::compute_rise_set;
use dhruv_vedic_base::riseset_types::{GeoLocation, RiseSetConfig, RiseSetEvent, RiseSetResult};

use dhruv_frames::{ReferencePlane, ecliptic_lon_to_invariable_lon};

use crate::error::SearchError;
use crate::jyotish::graha_longitudes;
use crate::jyotish_types::GrahaLongitudesConfig;
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
/// Note: `BirthPeriod::Twilight` is not returned by this function. Computing
/// twilight requires solar depression angle utilities not yet available.
/// TODO: add Twilight classification when solar depression angle utilities are available
fn determine_birth_period(birth_jd: f64, sunrise_jd: f64, sunset_jd: f64) -> BirthPeriod {
    if birth_jd >= sunrise_jd && birth_jd < sunset_jd {
        BirthPeriod::Day
    } else {
        BirthPeriod::Night
    }
}

/// Compute the Moon's sidereal longitude for dasha birth balance.
fn moon_sidereal_lon(
    engine: &Engine,
    eop: &EopKernel,
    utc: &UtcTime,
    aya_config: &SankrantiConfig,
) -> Result<f64, SearchError> {
    let jd_tdb = crate::search_util::utc_to_jd_tdb_with_eop(engine, Some(eop), utc);
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
    let jd_tdb = crate::search_util::utc_to_jd_tdb_with_eop(engine, Some(eop), utc);
    let graha_lons = graha_longitudes(
        engine,
        jd_tdb,
        &GrahaLongitudesConfig::sidereal_with_model(
            aya_config.ayanamsha_system,
            aya_config.use_nutation,
            aya_config.precession_model,
            aya_config.reference_plane,
        ),
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

fn jd_tdb_to_jd_utc(engine: &Engine, jd_tdb: f64) -> f64 {
    let tdb_s = jd_to_tdb_seconds(jd_tdb);
    let utc_s = engine.lsk().tdb_to_utc(tdb_s);
    tdb_seconds_to_jd(utc_s)
}

/// Compute sunrise and sunset JD UTC for Kala dasha.
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
    let jd_midnight = dhruv_vedic_base::utc_day_start_jd(jd_utc);
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
    let sunrise_jd_tdb = match sunrise_result {
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
    let sunset_jd_tdb = match sunset_result {
        RiseSetResult::Event { jd_tdb, .. } => jd_tdb,
        _ => {
            return Err(SearchError::NoConvergence(
                "sun never sets at this location",
            ));
        }
    };

    Ok((
        jd_tdb_to_jd_utc(engine, sunrise_jd_tdb),
        jd_tdb_to_jd_utc(engine, sunset_jd_tdb),
    ))
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

#[derive(Debug, Clone, Default)]
struct ComputedDashaInputs {
    moon_sid_lon: Option<f64>,
    rashi_inputs: Option<RashiDashaInputs>,
    sunrise_sunset: Option<(f64, f64)>,
}

fn compute_dasha_inputs_for_birth(
    engine: &Engine,
    eop: &EopKernel,
    birth_utc: &UtcTime,
    location: &GeoLocation,
    system: DashaSystem,
    riseset_config: &RiseSetConfig,
    aya_config: &SankrantiConfig,
) -> Result<ComputedDashaInputs, SearchError> {
    let moon_sid_lon = if needs_moon_lon(system) {
        Some(moon_sidereal_lon(engine, eop, birth_utc, aya_config)?)
    } else {
        None
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

    Ok(ComputedDashaInputs {
        moon_sid_lon,
        rashi_inputs,
        sunrise_sunset,
    })
}

fn method_for_parent_level(
    parent_level: DashaLevel,
    system_default: SubPeriodMethod,
    variation: &DashaVariationConfig,
) -> SubPeriodMethod {
    variation.method_for_level(parent_level as u8, system_default)
}

fn complete_level_parent_method(
    parent_level: &[DashaPeriod],
    child_level: DashaLevel,
) -> Result<DashaLevel, SearchError> {
    if matches!(child_level, DashaLevel::Mahadasha) {
        return Err(SearchError::InvalidConfig(
            "child_level must be deeper than Mahadasha",
        ));
    }
    if parent_level.is_empty() {
        return Ok(match child_level {
            DashaLevel::Antardasha => DashaLevel::Mahadasha,
            DashaLevel::Pratyantardasha => DashaLevel::Antardasha,
            DashaLevel::Sookshmadasha => DashaLevel::Pratyantardasha,
            DashaLevel::Pranadasha => DashaLevel::Sookshmadasha,
            DashaLevel::Mahadasha => unreachable!(),
        });
    }

    let expected = parent_level[0]
        .level
        .child_level()
        .ok_or(SearchError::InvalidConfig(
            "parent level has no child level",
        ))?;
    if expected != child_level {
        return Err(SearchError::InvalidConfig(
            "child_level does not match the supplied parent periods",
        ));
    }
    if parent_level
        .iter()
        .any(|p| p.level != parent_level[0].level)
    {
        return Err(SearchError::InvalidConfig(
            "parent periods must all be at the same level",
        ));
    }
    Ok(parent_level[0].level)
}

fn dispatch_level0(
    system: DashaSystem,
    birth_jd: f64,
    moon_sid_lon: f64,
    rashi_inputs: Option<&RashiDashaInputs>,
    sunrise_sunset: Option<(f64, f64)>,
) -> Result<Vec<DashaPeriod>, SearchError> {
    if let Some(cfg) = nakshatra_config_for_system(system) {
        return Ok(nakshatra_level0(birth_jd, moon_sid_lon, &cfg));
    }

    match system {
        DashaSystem::Yogini => Ok(yogini_level0(birth_jd, moon_sid_lon, &yogini_config())),
        DashaSystem::Chara => Ok(chara_level0(
            birth_jd,
            rashi_inputs.ok_or(SearchError::InvalidConfig("rashi inputs required"))?,
        )),
        DashaSystem::Sthira => Ok(sthira_level0(
            birth_jd,
            rashi_inputs.ok_or(SearchError::InvalidConfig("rashi inputs required"))?,
        )),
        DashaSystem::Yogardha => Ok(yogardha_level0(
            birth_jd,
            rashi_inputs.ok_or(SearchError::InvalidConfig("rashi inputs required"))?,
        )),
        DashaSystem::Driga => Ok(driga_level0(
            birth_jd,
            rashi_inputs.ok_or(SearchError::InvalidConfig("rashi inputs required"))?,
        )),
        DashaSystem::Shoola => Ok(shoola_level0(
            birth_jd,
            rashi_inputs.ok_or(SearchError::InvalidConfig("rashi inputs required"))?,
        )),
        DashaSystem::Mandooka => Ok(mandooka_level0(
            birth_jd,
            rashi_inputs.ok_or(SearchError::InvalidConfig("rashi inputs required"))?,
        )),
        DashaSystem::Chakra => {
            let ri = rashi_inputs.ok_or(SearchError::InvalidConfig("rashi inputs required"))?;
            let birth_period = if let Some((sunrise, sunset)) = sunrise_sunset {
                determine_birth_period(birth_jd, sunrise, sunset)
            } else {
                BirthPeriod::Day
            };
            Ok(chakra_level0(birth_jd, ri, birth_period))
        }
        DashaSystem::Kendradi => Ok(kendradi_level0(
            birth_jd,
            rashi_inputs.ok_or(SearchError::InvalidConfig("rashi inputs required"))?,
        )),
        DashaSystem::KarakaKendradi => {
            Ok(dhruv_vedic_base::dasha::kendradi::karaka_kendradi_level0(
                birth_jd,
                rashi_inputs.ok_or(SearchError::InvalidConfig("rashi inputs required"))?,
            ))
        }
        DashaSystem::KarakaKendradiGraha => Ok(
            dhruv_vedic_base::dasha::kendradi::karaka_kendradi_graha_level0(
                birth_jd,
                rashi_inputs.ok_or(SearchError::InvalidConfig("rashi inputs required"))?,
            ),
        ),
        DashaSystem::Kala => {
            let (sunrise, sunset) = sunrise_sunset.ok_or(SearchError::InvalidConfig(
                "sunrise/sunset required for Kala dasha",
            ))?;
            Ok(kala_level0(birth_jd, sunrise, sunset))
        }
        DashaSystem::KaalChakra => Ok(kaal_chakra_level0(birth_jd, moon_sid_lon)),
        _ => unreachable!("nakshatra systems handled before match"),
    }
}

fn dispatch_children(
    system: DashaSystem,
    parent: &DashaPeriod,
    rashi_inputs: Option<&RashiDashaInputs>,
    variation: &DashaVariationConfig,
) -> Result<Vec<DashaPeriod>, SearchError> {
    if let Some(cfg) = nakshatra_config_for_system(system) {
        let method = method_for_parent_level(parent.level, cfg.default_method, variation);
        return Ok(nakshatra_children(parent, &cfg, method));
    }

    match system {
        DashaSystem::Yogini => {
            let cfg = yogini_config();
            let method = method_for_parent_level(parent.level, cfg.default_method, variation);
            Ok(yogini_children(parent, &cfg, method))
        }
        DashaSystem::Chara => {
            let ri = rashi_inputs.ok_or(SearchError::InvalidConfig("rashi inputs required"))?;
            let method = method_for_parent_level(
                parent.level,
                dhruv_vedic_base::dasha::chara::CHARA_DEFAULT_METHOD,
                variation,
            );
            Ok(dhruv_vedic_base::dasha::rashi_dasha::rashi_children(
                parent,
                &|r| chara_period_years(r, ri),
                (0..12u8).map(|r| chara_period_years(r, ri)).sum(),
                dhruv_vedic_base::dasha::chara::CHARA_DEFAULT_METHOD,
                method,
            ))
        }
        DashaSystem::Sthira => {
            let method = method_for_parent_level(
                parent.level,
                dhruv_vedic_base::dasha::sthira::STHIRA_DEFAULT_METHOD,
                variation,
            );
            Ok(dhruv_vedic_base::dasha::rashi_dasha::rashi_children(
                parent,
                &dhruv_vedic_base::dasha::sthira::sthira_period_years,
                dhruv_vedic_base::dasha::sthira::STHIRA_TOTAL_YEARS,
                dhruv_vedic_base::dasha::sthira::STHIRA_DEFAULT_METHOD,
                method,
            ))
        }
        DashaSystem::Yogardha => {
            let ri = rashi_inputs.ok_or(SearchError::InvalidConfig("rashi inputs required"))?;
            let method = method_for_parent_level(
                parent.level,
                dhruv_vedic_base::dasha::yogardha::YOGARDHA_DEFAULT_METHOD,
                variation,
            );
            Ok(dhruv_vedic_base::dasha::rashi_dasha::rashi_children(
                parent,
                &|r| dhruv_vedic_base::dasha::yogardha::yogardha_period_years(r, ri),
                (0..12u8)
                    .map(|r| dhruv_vedic_base::dasha::yogardha::yogardha_period_years(r, ri))
                    .sum(),
                dhruv_vedic_base::dasha::yogardha::YOGARDHA_DEFAULT_METHOD,
                method,
            ))
        }
        DashaSystem::Driga => {
            let method = method_for_parent_level(
                parent.level,
                dhruv_vedic_base::dasha::driga::DRIGA_DEFAULT_METHOD,
                variation,
            );
            Ok(dhruv_vedic_base::dasha::rashi_dasha::rashi_children(
                parent,
                &dhruv_vedic_base::dasha::driga::driga_period_years,
                dhruv_vedic_base::dasha::driga::DRIGA_TOTAL_YEARS,
                dhruv_vedic_base::dasha::driga::DRIGA_DEFAULT_METHOD,
                method,
            ))
        }
        DashaSystem::Shoola => {
            let method = method_for_parent_level(
                parent.level,
                dhruv_vedic_base::dasha::shoola::SHOOLA_DEFAULT_METHOD,
                variation,
            );
            Ok(dhruv_vedic_base::dasha::rashi_dasha::rashi_children(
                parent,
                &dhruv_vedic_base::dasha::shoola::shoola_period_years,
                dhruv_vedic_base::dasha::shoola::SHOOLA_TOTAL_YEARS,
                dhruv_vedic_base::dasha::shoola::SHOOLA_DEFAULT_METHOD,
                method,
            ))
        }
        DashaSystem::Mandooka => {
            let method = method_for_parent_level(
                parent.level,
                dhruv_vedic_base::dasha::mandooka::MANDOOKA_DEFAULT_METHOD,
                variation,
            );
            Ok(mandooka_children(parent, method))
        }
        DashaSystem::Chakra => {
            let method = method_for_parent_level(
                parent.level,
                dhruv_vedic_base::dasha::chakra::CHAKRA_DEFAULT_METHOD,
                variation,
            );
            Ok(dhruv_vedic_base::dasha::rashi_dasha::rashi_children(
                parent,
                &dhruv_vedic_base::dasha::chakra::chakra_period_years,
                dhruv_vedic_base::dasha::chakra::CHAKRA_TOTAL_YEARS,
                dhruv_vedic_base::dasha::chakra::CHAKRA_DEFAULT_METHOD,
                method,
            ))
        }
        DashaSystem::Kendradi => {
            let ri = rashi_inputs.ok_or(SearchError::InvalidConfig("rashi inputs required"))?;
            let method = method_for_parent_level(
                parent.level,
                dhruv_vedic_base::dasha::kendradi::KENDRADI_DEFAULT_METHOD,
                variation,
            );
            Ok(dhruv_vedic_base::dasha::rashi_dasha::rashi_children(
                parent,
                &|r| chara_period_years(r, ri),
                (0..12u8).map(|r| chara_period_years(r, ri)).sum(),
                dhruv_vedic_base::dasha::kendradi::KENDRADI_DEFAULT_METHOD,
                method,
            ))
        }
        DashaSystem::KarakaKendradi => {
            let ri = rashi_inputs.ok_or(SearchError::InvalidConfig("rashi inputs required"))?;
            let method = method_for_parent_level(
                parent.level,
                dhruv_vedic_base::dasha::kendradi::KENDRADI_DEFAULT_METHOD,
                variation,
            );
            Ok(dhruv_vedic_base::dasha::rashi_dasha::rashi_children(
                parent,
                &|r| chara_period_years(r, ri),
                (0..12u8).map(|r| chara_period_years(r, ri)).sum(),
                dhruv_vedic_base::dasha::kendradi::KENDRADI_DEFAULT_METHOD,
                method,
            ))
        }
        DashaSystem::KarakaKendradiGraha => {
            let ri = rashi_inputs.ok_or(SearchError::InvalidConfig("rashi inputs required"))?;
            let method = method_for_parent_level(
                parent.level,
                dhruv_vedic_base::dasha::kendradi::KENDRADI_DEFAULT_METHOD,
                variation,
            );
            Ok(dhruv_vedic_base::dasha::rashi_dasha::rashi_children(
                parent,
                &|r| chara_period_years(r, ri),
                (0..12u8).map(|r| chara_period_years(r, ri)).sum(),
                dhruv_vedic_base::dasha::kendradi::KENDRADI_DEFAULT_METHOD,
                method,
            ))
        }
        DashaSystem::Kala => {
            let method = method_for_parent_level(
                parent.level,
                dhruv_vedic_base::dasha::kala_data::KALA_DEFAULT_METHOD,
                variation,
            );
            Ok(kala_children(parent, method))
        }
        DashaSystem::KaalChakra => {
            let method = method_for_parent_level(
                parent.level,
                dhruv_vedic_base::dasha::kaal_chakra_data::KCD_DEFAULT_METHOD,
                variation,
            );
            Ok(kaal_chakra_children(parent, method))
        }
        _ => unreachable!("nakshatra systems handled before match"),
    }
}

fn dispatch_complete_level(
    system: DashaSystem,
    parent_level: &[DashaPeriod],
    rashi_inputs: Option<&RashiDashaInputs>,
    child_level: DashaLevel,
    variation: &DashaVariationConfig,
) -> Result<Vec<DashaPeriod>, SearchError> {
    let parent_depth = complete_level_parent_method(parent_level, child_level)?;

    if let Some(cfg) = nakshatra_config_for_system(system) {
        let method = method_for_parent_level(parent_depth, cfg.default_method, variation);
        return nakshatra_complete_level(parent_level, &cfg, child_level, method)
            .map_err(SearchError::from);
    }

    match system {
        DashaSystem::Yogini => {
            let cfg = yogini_config();
            let method = method_for_parent_level(parent_depth, cfg.default_method, variation);
            yogini_complete_level(parent_level, &cfg, child_level, method)
                .map_err(SearchError::from)
        }
        DashaSystem::Chara => {
            let ri = rashi_inputs.ok_or(SearchError::InvalidConfig("rashi inputs required"))?;
            let method = method_for_parent_level(
                parent_depth,
                dhruv_vedic_base::dasha::chara::CHARA_DEFAULT_METHOD,
                variation,
            );
            dhruv_vedic_base::dasha::rashi_dasha::rashi_complete_level(
                parent_level,
                &|r| chara_period_years(r, ri),
                (0..12u8).map(|r| chara_period_years(r, ri)).sum(),
                child_level,
                dhruv_vedic_base::dasha::chara::CHARA_DEFAULT_METHOD,
                method,
            )
            .map_err(SearchError::from)
        }
        DashaSystem::Sthira => {
            let method = method_for_parent_level(
                parent_depth,
                dhruv_vedic_base::dasha::sthira::STHIRA_DEFAULT_METHOD,
                variation,
            );
            dhruv_vedic_base::dasha::rashi_dasha::rashi_complete_level(
                parent_level,
                &dhruv_vedic_base::dasha::sthira::sthira_period_years,
                dhruv_vedic_base::dasha::sthira::STHIRA_TOTAL_YEARS,
                child_level,
                dhruv_vedic_base::dasha::sthira::STHIRA_DEFAULT_METHOD,
                method,
            )
            .map_err(SearchError::from)
        }
        DashaSystem::Yogardha => {
            let ri = rashi_inputs.ok_or(SearchError::InvalidConfig("rashi inputs required"))?;
            let method = method_for_parent_level(
                parent_depth,
                dhruv_vedic_base::dasha::yogardha::YOGARDHA_DEFAULT_METHOD,
                variation,
            );
            dhruv_vedic_base::dasha::rashi_dasha::rashi_complete_level(
                parent_level,
                &|r| dhruv_vedic_base::dasha::yogardha::yogardha_period_years(r, ri),
                (0..12u8)
                    .map(|r| dhruv_vedic_base::dasha::yogardha::yogardha_period_years(r, ri))
                    .sum(),
                child_level,
                dhruv_vedic_base::dasha::yogardha::YOGARDHA_DEFAULT_METHOD,
                method,
            )
            .map_err(SearchError::from)
        }
        DashaSystem::Driga => {
            let method = method_for_parent_level(
                parent_depth,
                dhruv_vedic_base::dasha::driga::DRIGA_DEFAULT_METHOD,
                variation,
            );
            dhruv_vedic_base::dasha::rashi_dasha::rashi_complete_level(
                parent_level,
                &dhruv_vedic_base::dasha::driga::driga_period_years,
                dhruv_vedic_base::dasha::driga::DRIGA_TOTAL_YEARS,
                child_level,
                dhruv_vedic_base::dasha::driga::DRIGA_DEFAULT_METHOD,
                method,
            )
            .map_err(SearchError::from)
        }
        DashaSystem::Shoola => {
            let method = method_for_parent_level(
                parent_depth,
                dhruv_vedic_base::dasha::shoola::SHOOLA_DEFAULT_METHOD,
                variation,
            );
            dhruv_vedic_base::dasha::rashi_dasha::rashi_complete_level(
                parent_level,
                &dhruv_vedic_base::dasha::shoola::shoola_period_years,
                dhruv_vedic_base::dasha::shoola::SHOOLA_TOTAL_YEARS,
                child_level,
                dhruv_vedic_base::dasha::shoola::SHOOLA_DEFAULT_METHOD,
                method,
            )
            .map_err(SearchError::from)
        }
        DashaSystem::Mandooka => {
            let method = method_for_parent_level(
                parent_depth,
                dhruv_vedic_base::dasha::mandooka::MANDOOKA_DEFAULT_METHOD,
                variation,
            );
            mandooka_complete_level(parent_level, child_level, method).map_err(SearchError::from)
        }
        DashaSystem::Chakra => {
            let method = method_for_parent_level(
                parent_depth,
                dhruv_vedic_base::dasha::chakra::CHAKRA_DEFAULT_METHOD,
                variation,
            );
            dhruv_vedic_base::dasha::rashi_dasha::rashi_complete_level(
                parent_level,
                &dhruv_vedic_base::dasha::chakra::chakra_period_years,
                dhruv_vedic_base::dasha::chakra::CHAKRA_TOTAL_YEARS,
                child_level,
                dhruv_vedic_base::dasha::chakra::CHAKRA_DEFAULT_METHOD,
                method,
            )
            .map_err(SearchError::from)
        }
        DashaSystem::Kendradi | DashaSystem::KarakaKendradi | DashaSystem::KarakaKendradiGraha => {
            let ri = rashi_inputs.ok_or(SearchError::InvalidConfig("rashi inputs required"))?;
            let method = method_for_parent_level(
                parent_depth,
                dhruv_vedic_base::dasha::kendradi::KENDRADI_DEFAULT_METHOD,
                variation,
            );
            dhruv_vedic_base::dasha::rashi_dasha::rashi_complete_level(
                parent_level,
                &|r| chara_period_years(r, ri),
                (0..12u8).map(|r| chara_period_years(r, ri)).sum(),
                child_level,
                dhruv_vedic_base::dasha::kendradi::KENDRADI_DEFAULT_METHOD,
                method,
            )
            .map_err(SearchError::from)
        }
        DashaSystem::Kala => {
            let method = method_for_parent_level(
                parent_depth,
                dhruv_vedic_base::dasha::kala_data::KALA_DEFAULT_METHOD,
                variation,
            );
            kala_complete_level(parent_level, child_level, method).map_err(SearchError::from)
        }
        DashaSystem::KaalChakra => {
            let method = method_for_parent_level(
                parent_depth,
                dhruv_vedic_base::dasha::kaal_chakra_data::KCD_DEFAULT_METHOD,
                variation,
            );
            kaal_chakra_complete_level(parent_level, child_level, method).map_err(SearchError::from)
        }
        _ => unreachable!("nakshatra systems handled before match"),
    }
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

/// Compute level-0 (mahadasha) periods for a birth chart.
#[allow(clippy::too_many_arguments)]
pub fn dasha_level0_for_birth(
    engine: &Engine,
    eop: &EopKernel,
    birth_utc: &UtcTime,
    location: &GeoLocation,
    system: DashaSystem,
    _bhava_config: &BhavaConfig,
    riseset_config: &RiseSetConfig,
    aya_config: &SankrantiConfig,
) -> Result<Vec<DashaPeriod>, SearchError> {
    let birth_jd = utc_to_jd_utc(birth_utc);
    let inputs = compute_dasha_inputs_for_birth(
        engine,
        eop,
        birth_utc,
        location,
        system,
        riseset_config,
        aya_config,
    )?;
    dispatch_level0(
        system,
        birth_jd,
        inputs.moon_sid_lon.unwrap_or(0.0),
        inputs.rashi_inputs.as_ref(),
        inputs.sunrise_sunset,
    )
}

/// Compute one specific level-0 (mahadasha) period for a birth chart.
#[allow(clippy::too_many_arguments)]
pub fn dasha_level0_entity_for_birth(
    engine: &Engine,
    eop: &EopKernel,
    birth_utc: &UtcTime,
    location: &GeoLocation,
    system: DashaSystem,
    entity: DashaEntity,
    bhava_config: &BhavaConfig,
    riseset_config: &RiseSetConfig,
    aya_config: &SankrantiConfig,
) -> Result<Option<DashaPeriod>, SearchError> {
    let periods = dasha_level0_for_birth(
        engine,
        eop,
        birth_utc,
        location,
        system,
        bhava_config,
        riseset_config,
        aya_config,
    )?;
    Ok(periods.into_iter().find(|p| p.entity == entity))
}

/// Compute all child periods for a parent period.
#[allow(clippy::too_many_arguments)]
pub fn dasha_children_for_birth(
    engine: &Engine,
    eop: &EopKernel,
    birth_utc: &UtcTime,
    location: &GeoLocation,
    system: DashaSystem,
    parent: &DashaPeriod,
    _bhava_config: &BhavaConfig,
    riseset_config: &RiseSetConfig,
    aya_config: &SankrantiConfig,
    variation: &DashaVariationConfig,
) -> Result<Vec<DashaPeriod>, SearchError> {
    let inputs = compute_dasha_inputs_for_birth(
        engine,
        eop,
        birth_utc,
        location,
        system,
        riseset_config,
        aya_config,
    )?;
    dispatch_children(system, parent, inputs.rashi_inputs.as_ref(), variation)
}

/// Compute one specific child period for a parent period.
#[allow(clippy::too_many_arguments)]
pub fn dasha_child_period_for_birth(
    engine: &Engine,
    eop: &EopKernel,
    birth_utc: &UtcTime,
    location: &GeoLocation,
    system: DashaSystem,
    parent: &DashaPeriod,
    child_entity: DashaEntity,
    bhava_config: &BhavaConfig,
    riseset_config: &RiseSetConfig,
    aya_config: &SankrantiConfig,
    variation: &DashaVariationConfig,
) -> Result<Option<DashaPeriod>, SearchError> {
    let children = dasha_children_for_birth(
        engine,
        eop,
        birth_utc,
        location,
        system,
        parent,
        bhava_config,
        riseset_config,
        aya_config,
        variation,
    )?;
    let child_level = match parent.level.child_level() {
        Some(level) => level,
        None => return Ok(None),
    };
    Ok(children
        .into_iter()
        .find(|period| period.entity == child_entity && period.level == child_level))
}

/// Compute a complete child level from a supplied parent level.
#[allow(clippy::too_many_arguments)]
pub fn dasha_complete_level_for_birth(
    engine: &Engine,
    eop: &EopKernel,
    birth_utc: &UtcTime,
    location: &GeoLocation,
    system: DashaSystem,
    parent_level: &[DashaPeriod],
    child_level: DashaLevel,
    _bhava_config: &BhavaConfig,
    riseset_config: &RiseSetConfig,
    aya_config: &SankrantiConfig,
    variation: &DashaVariationConfig,
) -> Result<Vec<DashaPeriod>, SearchError> {
    let inputs = compute_dasha_inputs_for_birth(
        engine,
        eop,
        birth_utc,
        location,
        system,
        riseset_config,
        aya_config,
    )?;
    dispatch_complete_level(
        system,
        parent_level,
        inputs.rashi_inputs.as_ref(),
        child_level,
        variation,
    )
}

/// Pre-computed inputs for context-sharing dasha computation.
///
/// Callers populate only the fields needed by the target system:
/// - `moon_sid_lon`: required for nakshatra-based, Yogini, KaalChakra
/// - `rashi_inputs`: required for rashi-based systems (10)
/// - `sunrise_sunset`: required for Kala as `(sunrise_jd_utc, sunset_jd_utc)`,
///   optional for Chakra (BirthPeriod)
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
/// For Kala dasha, requires sunrise/sunset JD UTC values.
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
/// For Kala dasha, requires sunrise/sunset JD UTC values.
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
    fn test_determine_birth_period_requires_consistent_scale_inputs() {
        let sunrise_utc = 2460000.25;
        let birth_utc_30s_before = sunrise_utc - (30.0 / 86400.0);
        assert_eq!(
            determine_birth_period(birth_utc_30s_before, sunrise_utc, sunrise_utc + 0.5),
            BirthPeriod::Night
        );
    }
}
