//! Vedic jyotish orchestration: queries engine for graha positions.
//!
//! Provides the bridge between the ephemeris engine and the pure-math
//! Vedic calculation modules. Queries all 9 graha positions at a given
//! epoch and converts to sidereal longitudes.

use dhruv_core::{Body, Engine};
use dhruv_time::{EopKernel, UtcTime};
use dhruv_vedic_base::{
    AllSpecialLagnas, AllUpagrahas, ArudhaResult, AshtakavargaResult, AyanamshaSystem, BhavaConfig,
    Graha, LunarNode, NodeMode, ALL_GRAHAS, ascendant_longitude_rad, ayanamsha_deg,
    calculate_ashtakavarga, compute_bhavas, ghatikas_since_sunrise, jd_tdb_to_centuries,
    lunar_node_deg, nth_rashi_from, rashi_lord_by_index, sun_based_upagrahas, time_upagraha_jd,
    normalize_360,
};
use dhruv_vedic_base::arudha::all_arudha_padas;
use dhruv_vedic_base::riseset::{compute_rise_set};
use dhruv_vedic_base::riseset_types::{GeoLocation, RiseSetConfig, RiseSetEvent, RiseSetResult};
use dhruv_vedic_base::special_lagna::all_special_lagnas;
use dhruv_vedic_base::upagraha::TIME_BASED_UPAGRAHAS;
use dhruv_vedic_base::vaar::vaar_from_jd;

use crate::conjunction::body_ecliptic_lon_lat;
use crate::error::SearchError;
use crate::jyotish_types::GrahaLongitudes;
use crate::panchang::vedic_day_sunrises;
use crate::sankranti_types::SankrantiConfig;

/// Map a Graha to its dhruv_core::Body for engine queries.
fn graha_to_body(graha: Graha) -> Option<Body> {
    match graha {
        Graha::Surya => Some(Body::Sun),
        Graha::Chandra => Some(Body::Moon),
        Graha::Mangal => Some(Body::Mars),
        Graha::Buddh => Some(Body::Mercury),
        Graha::Guru => Some(Body::Jupiter),
        Graha::Shukra => Some(Body::Venus),
        Graha::Shani => Some(Body::Saturn),
        Graha::Rahu | Graha::Ketu => None,
    }
}

/// Query all 9 graha sidereal longitudes at a given TDB epoch.
///
/// For the 7 physical planets, queries the engine for tropical ecliptic
/// longitude and subtracts ayanamsha. For Rahu/Ketu, uses the mean/true
/// node mathematical formulas.
pub fn graha_sidereal_longitudes(
    engine: &Engine,
    jd_tdb: f64,
    system: AyanamshaSystem,
    use_nutation: bool,
) -> Result<GrahaLongitudes, SearchError> {
    let t = jd_tdb_to_centuries(jd_tdb);
    let aya = ayanamsha_deg(system, t, use_nutation);

    let mut longitudes = [0.0f64; 9];

    for graha in ALL_GRAHAS {
        let idx = graha.index() as usize;
        match graha {
            Graha::Rahu => {
                let rahu_tropical = lunar_node_deg(LunarNode::Rahu, t, NodeMode::True);
                longitudes[idx] = normalize(rahu_tropical - aya);
            }
            Graha::Ketu => {
                let ketu_tropical = lunar_node_deg(LunarNode::Ketu, t, NodeMode::True);
                longitudes[idx] = normalize(ketu_tropical - aya);
            }
            _ => {
                let body = graha_to_body(graha).expect("sapta graha has body");
                let (lon_tropical, _lat) = body_ecliptic_lon_lat(engine, body, jd_tdb)?;
                longitudes[idx] = normalize(lon_tropical - aya);
            }
        }
    }

    Ok(GrahaLongitudes { longitudes })
}

/// Compute all 8 special lagnas for a given moment and location.
///
/// Orchestrates engine queries for Sun/Moon, Lagna computation, sunrise
/// determination, and delegates to the pure-math `all_special_lagnas()`.
pub fn special_lagnas_for_date(
    engine: &Engine,
    eop: &EopKernel,
    utc: &UtcTime,
    location: &GeoLocation,
    riseset_config: &RiseSetConfig,
    aya_config: &SankrantiConfig,
) -> Result<AllSpecialLagnas, SearchError> {
    let jd_tdb = utc.to_jd_tdb(engine.lsk());
    let t = jd_tdb_to_centuries(jd_tdb);
    let aya = ayanamsha_deg(aya_config.ayanamsha_system, t, aya_config.use_nutation);

    // Get Sun and Moon sidereal longitudes
    let (sun_tropical, _) = body_ecliptic_lon_lat(engine, Body::Sun, jd_tdb)?;
    let (moon_tropical, _) = body_ecliptic_lon_lat(engine, Body::Moon, jd_tdb)?;
    let sun_sid = normalize(sun_tropical - aya);
    let moon_sid = normalize(moon_tropical - aya);

    // Compute Lagna (Ascendant) sidereal longitude
    let jd_utc = utc_to_jd_utc(utc);
    let lagna_rad = ascendant_longitude_rad(engine.lsk(), eop, location, jd_utc)?;
    let lagna_tropical = lagna_rad.to_degrees();
    let lagna_sid = normalize(lagna_tropical - aya);

    // Get sunrise pair for ghatikas
    let (jd_sunrise, jd_next_sunrise) = vedic_day_sunrises(engine, eop, utc, location, riseset_config)?;
    let ghatikas = ghatikas_since_sunrise(jd_tdb, jd_sunrise, jd_next_sunrise);

    // Determine lords for Indu Lagna
    let lagna_rashi_idx = (lagna_sid / 30.0) as u8;
    let lagna_lord = rashi_lord_by_index(lagna_rashi_idx).unwrap_or(Graha::Surya);

    let moon_rashi_idx = (moon_sid / 30.0) as u8;
    let moon_9th_rashi_idx = nth_rashi_from(moon_rashi_idx, 9);
    let moon_9th_lord = rashi_lord_by_index(moon_9th_rashi_idx).unwrap_or(Graha::Surya);

    Ok(all_special_lagnas(
        sun_sid,
        moon_sid,
        lagna_sid,
        ghatikas,
        lagna_lord,
        moon_9th_lord,
    ))
}

/// Compute all 12 arudha padas for a given date and location.
///
/// Orchestrates bhava cusp computation, graha sidereal positions, resolves
/// lord longitudes for each house, and delegates to `all_arudha_padas()`.
pub fn arudha_padas_for_date(
    engine: &Engine,
    eop: &EopKernel,
    utc: &UtcTime,
    location: &GeoLocation,
    bhava_config: &BhavaConfig,
    aya_config: &SankrantiConfig,
) -> Result<[ArudhaResult; 12], SearchError> {
    let jd_tdb = utc.to_jd_tdb(engine.lsk());
    let jd_utc = utc_to_jd_utc(utc);

    // Get bhava cusps
    let bhava_result = compute_bhavas(engine, engine.lsk(), eop, location, jd_utc, bhava_config)?;

    // Get sidereal graha positions
    let graha_lons = graha_sidereal_longitudes(engine, jd_tdb, aya_config.ayanamsha_system, aya_config.use_nutation)?;

    // Convert cusp tropical longitudes to sidereal
    let t = jd_tdb_to_centuries(jd_tdb);
    let aya = ayanamsha_deg(aya_config.ayanamsha_system, t, aya_config.use_nutation);

    let mut cusp_sid = [0.0f64; 12];
    for i in 0..12 {
        cusp_sid[i] = normalize(bhava_result.bhavas[i].cusp_deg - aya);
    }

    // Resolve lord longitude for each house
    let mut lord_lons = [0.0f64; 12];
    for i in 0..12 {
        let cusp_rashi_idx = (cusp_sid[i] / 30.0) as u8;
        let lord = rashi_lord_by_index(cusp_rashi_idx).unwrap_or(Graha::Surya);
        lord_lons[i] = graha_lons.longitude(lord);
    }

    Ok(all_arudha_padas(&cusp_sid, &lord_lons))
}

/// Compute all 11 upagrahas for a given date and location.
///
/// Orchestrates sunrise/sunset computation, portion index determination,
/// lagna computation at portion times, and sun-based chain calculation.
pub fn all_upagrahas_for_date(
    engine: &Engine,
    eop: &EopKernel,
    utc: &UtcTime,
    location: &GeoLocation,
    riseset_config: &RiseSetConfig,
    aya_config: &SankrantiConfig,
) -> Result<AllUpagrahas, SearchError> {
    let jd_tdb = utc.to_jd_tdb(engine.lsk());
    let jd_utc = utc_to_jd_utc(utc);
    let t = jd_tdb_to_centuries(jd_tdb);
    let aya = ayanamsha_deg(aya_config.ayanamsha_system, t, aya_config.use_nutation);

    // Get sunrise and next sunrise (defines the vedic day)
    let (jd_sunrise, jd_next_sunrise) =
        vedic_day_sunrises(engine, eop, utc, location, riseset_config)?;

    // Get sunset for the same day
    let noon_jd = dhruv_vedic_base::approximate_local_noon_jd(
        jd_utc.floor() + 0.5,
        location.longitude_deg,
    );
    let sunset_result = compute_rise_set(
        engine,
        engine.lsk(),
        eop,
        location,
        RiseSetEvent::Sunset,
        noon_jd,
        riseset_config,
    )
    .map_err(|_| SearchError::NoConvergence("sunset computation failed"))?;
    let jd_sunset = match sunset_result {
        RiseSetResult::Event { jd_tdb: jd, .. } => jd,
        _ => return Err(SearchError::NoConvergence("sun never sets at this location")),
    };

    // Determine if birth time is during day (sunrise to sunset) or night
    let is_day = jd_tdb >= jd_sunrise && jd_tdb < jd_sunset;

    // Weekday of this vedic day (determined by sunrise)
    let weekday = vaar_from_jd(jd_sunrise).index();

    // Compute time-based upagrahas (lagna at portion start/end)
    let mut time_lons = [0.0f64; 6]; // Gulika, Maandi, Kaala, Mrityu, ArthaPrahara, YamaGhantaka
    for (i, &upa) in TIME_BASED_UPAGRAHAS.iter().enumerate() {
        let target_jd = time_upagraha_jd(
            upa,
            weekday,
            is_day,
            jd_sunrise,
            jd_sunset,
            jd_next_sunrise,
        );
        // Compute tropical lagna at this JD
        // target_jd is in TDB but ascendant_longitude_rad expects JD UTC
        // For this purpose the difference is negligible (~1s), but we use jd_utc convention
        let lagna_rad = ascendant_longitude_rad(engine.lsk(), eop, location, target_jd)?;
        let lagna_tropical = lagna_rad.to_degrees();
        time_lons[i] = normalize_360(lagna_tropical - aya);
    }

    // Compute sun-based upagrahas from sidereal Sun longitude
    let (sun_tropical, _) = body_ecliptic_lon_lat(engine, Body::Sun, jd_tdb)?;
    let sun_sid = normalize_360(sun_tropical - aya);
    let sun_up = sun_based_upagrahas(sun_sid);

    Ok(AllUpagrahas {
        gulika: time_lons[0],
        maandi: time_lons[1],
        kaala: time_lons[2],
        mrityu: time_lons[3],
        artha_prahara: time_lons[4],
        yama_ghantaka: time_lons[5],
        dhooma: sun_up.dhooma,
        vyatipata: sun_up.vyatipata,
        parivesha: sun_up.parivesha,
        indra_chapa: sun_up.indra_chapa,
        upaketu: sun_up.upaketu,
    })
}

/// Compute complete Ashtakavarga (BAV + SAV + Sodhana) for a given date and location.
///
/// Queries graha sidereal positions and ascendant, resolves rashi indices,
/// then delegates to the pure-math `calculate_ashtakavarga()`.
pub fn ashtakavarga_for_date(
    engine: &Engine,
    eop: &EopKernel,
    utc: &UtcTime,
    location: &GeoLocation,
    aya_config: &SankrantiConfig,
) -> Result<AshtakavargaResult, SearchError> {
    let jd_tdb = utc.to_jd_tdb(engine.lsk());
    let jd_utc = utc_to_jd_utc(utc);
    let t = jd_tdb_to_centuries(jd_tdb);
    let aya = ayanamsha_deg(aya_config.ayanamsha_system, t, aya_config.use_nutation);

    // Get sidereal longitudes for 7 sapta grahas (Sun..Saturn)
    let graha_lons = graha_sidereal_longitudes(engine, jd_tdb, aya_config.ayanamsha_system, aya_config.use_nutation)?;

    // Compute Lagna sidereal longitude
    let lagna_rad = ascendant_longitude_rad(engine.lsk(), eop, location, jd_utc)?;
    let lagna_tropical = lagna_rad.to_degrees();
    let lagna_sid = normalize(lagna_tropical - aya);

    // Extract rashi indices for 7 grahas (Sun..Saturn only, not Rahu/Ketu)
    let sapta = [
        Graha::Surya, Graha::Chandra, Graha::Mangal, Graha::Buddh,
        Graha::Guru, Graha::Shukra, Graha::Shani,
    ];
    let mut graha_rashis = [0u8; 7];
    for (i, &graha) in sapta.iter().enumerate() {
        graha_rashis[i] = (graha_lons.longitude(graha) / 30.0) as u8;
    }
    let lagna_rashi = (lagna_sid / 30.0) as u8;

    Ok(calculate_ashtakavarga(&graha_rashis, lagna_rashi))
}

/// Convert UtcTime to JD UTC (calendar only, no TDB conversion).
fn utc_to_jd_utc(utc: &UtcTime) -> f64 {
    // Julian Day from calendar date (Meeus algorithm)
    let y = utc.year as f64;
    let m = utc.month as f64;
    let d = utc.day as f64
        + utc.hour as f64 / 24.0
        + utc.minute as f64 / 1440.0
        + utc.second / 86400.0;

    let (y2, m2) = if m <= 2.0 { (y - 1.0, m + 12.0) } else { (y, m) };
    let a = (y2 / 100.0).floor();
    let b = 2.0 - a + (a / 4.0).floor();

    (365.25 * (y2 + 4716.0)).floor() + (30.6001 * (m2 + 1.0)).floor() + d + b - 1524.5
}

/// Normalize longitude to [0, 360).
fn normalize(deg: f64) -> f64 {
    let r = deg % 360.0;
    if r < 0.0 { r + 360.0 } else { r }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn graha_to_body_mapping() {
        assert_eq!(graha_to_body(Graha::Surya), Some(Body::Sun));
        assert_eq!(graha_to_body(Graha::Chandra), Some(Body::Moon));
        assert_eq!(graha_to_body(Graha::Mangal), Some(Body::Mars));
        assert_eq!(graha_to_body(Graha::Buddh), Some(Body::Mercury));
        assert_eq!(graha_to_body(Graha::Guru), Some(Body::Jupiter));
        assert_eq!(graha_to_body(Graha::Shukra), Some(Body::Venus));
        assert_eq!(graha_to_body(Graha::Shani), Some(Body::Saturn));
        assert_eq!(graha_to_body(Graha::Rahu), None);
        assert_eq!(graha_to_body(Graha::Ketu), None);
    }
}
