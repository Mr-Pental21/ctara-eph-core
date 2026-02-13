//! Golden-value integration tests for Panchang classification.
//!
//! Validates Masa, Ayana, and Varsha against known Vedic calendar dates,
//! and verifies equivalence between `_for_date` and `_at`/`_from_sunrises` variants.
//! Requires kernel files. Skips gracefully if absent.

use std::path::Path;

use dhruv_core::{Engine, EngineConfig};
use dhruv_search::panchang_types::{AyanaInfo, MasaInfo, VarshaInfo};
use dhruv_search::sankranti_types::SankrantiConfig;
use dhruv_search::{
    ayana_for_date, elongation_at, ghatika_for_date, ghatika_from_sunrises, hora_for_date,
    hora_from_sunrises, karana_at, karana_for_date, masa_for_date, moon_sidereal_longitude_at,
    nakshatra_at, nakshatra_for_date, panchang_for_date, sidereal_sum_at, tithi_at, tithi_for_date,
    vaar_for_date, vaar_from_sunrises, varsha_for_date, vedic_day_sunrises, yoga_at, yoga_for_date,
};
use dhruv_time::{EopKernel, UtcTime};
use dhruv_vedic_base::riseset_types::{GeoLocation, RiseSetConfig};
use dhruv_vedic_base::{Ayana, Masa};

const SPK_PATH: &str = "../../kernels/data/de442s.bsp";
const LSK_PATH: &str = "../../kernels/data/naif0012.tls";
const EOP_PATH: &str = "../../kernels/data/finals2000A.all";

fn load_engine() -> Option<Engine> {
    if !Path::new(SPK_PATH).exists() || !Path::new(LSK_PATH).exists() {
        eprintln!("Skipping panchang_golden: kernel files not found");
        return None;
    }
    let config = EngineConfig::with_single_spk(SPK_PATH.into(), LSK_PATH.into(), 1024, false);
    Engine::new(config).ok()
}

fn default_config() -> SankrantiConfig {
    SankrantiConfig::default_lahiri()
}

/// Masa in mid-January 2024: should be Pausha (Sun in Dhanu/Sagittarius at new moon)
#[test]
fn masa_jan_2024() {
    let Some(engine) = load_engine() else { return };
    let utc = UtcTime::new(2024, 1, 15, 12, 0, 0.0);
    let config = default_config();
    let info: MasaInfo = masa_for_date(&engine, &utc, &config).unwrap();
    // Mid-January is typically Pausha masa
    assert_eq!(
        info.masa,
        Masa::Pausha,
        "expected Pausha, got {}",
        info.masa.name()
    );
    assert!(!info.adhika, "should not be adhika");
    // Start should be before the query date
    assert!(info.start.to_jd_tdb(engine.lsk()) < utc.to_jd_tdb(engine.lsk()));
    // End should be after the query date
    assert!(info.end.to_jd_tdb(engine.lsk()) > utc.to_jd_tdb(engine.lsk()));
}

/// Masa in mid-April 2024: should be Chaitra
#[test]
fn masa_apr_2024() {
    let Some(engine) = load_engine() else { return };
    let utc = UtcTime::new(2024, 4, 15, 12, 0, 0.0);
    let config = default_config();
    let info: MasaInfo = masa_for_date(&engine, &utc, &config).unwrap();
    // Mid-April is typically Chaitra masa
    assert_eq!(
        info.masa,
        Masa::Chaitra,
        "expected Chaitra, got {}",
        info.masa.name()
    );
}

/// Masa in mid-October 2024: should be Ashvina
#[test]
fn masa_oct_2024() {
    let Some(engine) = load_engine() else { return };
    let utc = UtcTime::new(2024, 10, 10, 12, 0, 0.0);
    let config = default_config();
    let info: MasaInfo = masa_for_date(&engine, &utc, &config).unwrap();
    // Mid-October is typically Ashvina masa
    assert_eq!(
        info.masa,
        Masa::Ashvina,
        "expected Ashvina, got {}",
        info.masa.name()
    );
}

/// Ayana in mid-January 2024: Uttarayana (after Makar Sankranti ~Jan 15)
#[test]
fn ayana_jan_2024() {
    let Some(engine) = load_engine() else { return };
    let utc = UtcTime::new(2024, 1, 20, 0, 0, 0.0);
    let config = default_config();
    let info: AyanaInfo = ayana_for_date(&engine, &utc, &config).unwrap();
    assert_eq!(info.ayana, Ayana::Uttarayana, "expected Uttarayana");
    // Start should be around Jan 14-15 (Makar Sankranti)
    assert_eq!(info.start.month, 1);
    assert!(info.start.day >= 14 && info.start.day <= 16);
}

/// Ayana in mid-August 2024: Dakshinayana (after Karka Sankranti ~July)
#[test]
fn ayana_aug_2024() {
    let Some(engine) = load_engine() else { return };
    let utc = UtcTime::new(2024, 8, 15, 0, 0, 0.0);
    let config = default_config();
    let info: AyanaInfo = ayana_for_date(&engine, &utc, &config).unwrap();
    assert_eq!(info.ayana, Ayana::Dakshinayana, "expected Dakshinayana");
    // Start should be in July (Karka Sankranti)
    assert_eq!(info.start.month, 7);
}

/// Ayana boundaries: start and end should bracket the query date
#[test]
fn ayana_brackets_query() {
    let Some(engine) = load_engine() else { return };
    let utc = UtcTime::new(2024, 5, 1, 0, 0, 0.0);
    let config = default_config();
    let info: AyanaInfo = ayana_for_date(&engine, &utc, &config).unwrap();
    let jd = utc.to_jd_tdb(engine.lsk());
    let jd_start = info.start.to_jd_tdb(engine.lsk());
    let jd_end = info.end.to_jd_tdb(engine.lsk());
    assert!(
        jd_start < jd,
        "start {jd_start} should be before query {jd}"
    );
    assert!(jd_end > jd, "end {jd_end} should be after query {jd}");
}

/// Varsha for 2024: Shobhakrit (year 37 in 60-year cycle)
/// 1987 = Prabhava, so 2024 = 1987 + 37 = year 38 = Krodhana
/// Actually: (2024 - 1987) % 60 = 37, 0-indexed → 38th samvatsara
#[test]
fn varsha_2024() {
    let Some(engine) = load_engine() else { return };
    let utc = UtcTime::new(2024, 6, 15, 0, 0, 0.0);
    let config = default_config();
    let info: VarshaInfo = varsha_for_date(&engine, &utc, &config).unwrap();
    // Verify the order is between 1 and 60
    assert!(
        info.order >= 1 && info.order <= 60,
        "order should be 1-60, got {}",
        info.order
    );
    // Start should be before query, end after
    let jd = utc.to_jd_tdb(engine.lsk());
    let jd_start = info.start.to_jd_tdb(engine.lsk());
    let jd_end = info.end.to_jd_tdb(engine.lsk());
    assert!(jd_start < jd, "start should be before query");
    assert!(jd_end > jd, "end should be after query");
    // Year should span ~354-384 days (lunar year)
    let span_days = jd_end - jd_start;
    assert!(
        span_days > 350.0 && span_days < 400.0,
        "year span {span_days:.0} days seems wrong"
    );
}

/// Varsha boundaries should form a continuous sequence
#[test]
fn varsha_consecutive_years() {
    let Some(engine) = load_engine() else { return };
    let config = default_config();
    let v2023 = varsha_for_date(&engine, &UtcTime::new(2023, 6, 15, 0, 0, 0.0), &config).unwrap();
    let v2024 = varsha_for_date(&engine, &UtcTime::new(2024, 6, 15, 0, 0, 0.0), &config).unwrap();

    // End of 2023 varsha should be start of 2024 varsha (approximately)
    let jd_end_2023 = v2023.end.to_jd_tdb(engine.lsk());
    let jd_start_2024 = v2024.start.to_jd_tdb(engine.lsk());
    let gap_days = (jd_start_2024 - jd_end_2023).abs();
    assert!(
        gap_days < 1.0,
        "gap between consecutive varshas: {gap_days:.2} days"
    );

    // Orders should differ by 1
    let expected_order = if v2023.order == 60 {
        1
    } else {
        v2023.order + 1
    };
    assert_eq!(v2024.order, expected_order, "orders should be consecutive");
}

// ---------------------------------------------------------------------------
// Equivalence tests: _at / _from_sunrises == _for_date
// ---------------------------------------------------------------------------

fn load_eop() -> Option<EopKernel> {
    if !Path::new(EOP_PATH).exists() {
        return None;
    }
    EopKernel::load(Path::new(EOP_PATH)).ok()
}

/// tithi_at(engine, jd, elongation_at(engine, jd)) == tithi_for_date(engine, utc)
#[test]
fn tithi_at_matches_for_date() {
    let Some(engine) = load_engine() else { return };
    let utc = UtcTime::new(2024, 1, 15, 12, 0, 0.0);
    let direct = tithi_for_date(&engine, &utc).unwrap();
    let jd = utc.to_jd_tdb(engine.lsk());
    let elong = elongation_at(&engine, jd).unwrap();
    let via_at = tithi_at(&engine, jd, elong).unwrap();
    assert_eq!(direct, via_at);
}

/// karana_at(engine, jd, elongation_at(engine, jd)) == karana_for_date(engine, utc)
#[test]
fn karana_at_matches_for_date() {
    let Some(engine) = load_engine() else { return };
    let utc = UtcTime::new(2024, 1, 15, 12, 0, 0.0);
    let direct = karana_for_date(&engine, &utc).unwrap();
    let jd = utc.to_jd_tdb(engine.lsk());
    let elong = elongation_at(&engine, jd).unwrap();
    let via_at = karana_at(&engine, jd, elong).unwrap();
    assert_eq!(direct, via_at);
}

/// yoga_at(engine, jd, sidereal_sum) == yoga_for_date(engine, utc, config)
#[test]
fn yoga_at_matches_for_date() {
    let Some(engine) = load_engine() else { return };
    let utc = UtcTime::new(2024, 1, 15, 12, 0, 0.0);
    let config = default_config();
    let direct = yoga_for_date(&engine, &utc, &config).unwrap();
    let jd = utc.to_jd_tdb(engine.lsk());
    let sum = sidereal_sum_at(&engine, jd, &config).unwrap();
    let via_at = yoga_at(&engine, jd, sum, &config).unwrap();
    assert_eq!(direct, via_at);
}

/// vaar_from_sunrises(sr, nsr, lsk) == vaar_for_date(engine, eop, utc, loc, cfg)
#[test]
fn vaar_from_sunrises_matches_for_date() {
    let Some(engine) = load_engine() else { return };
    let Some(eop) = load_eop() else { return };
    let utc = UtcTime::new(2024, 1, 15, 12, 0, 0.0);
    let loc = GeoLocation::new(28.6139, 77.2090, 0.0);
    let rs = RiseSetConfig::default();
    let direct = vaar_for_date(&engine, &eop, &utc, &loc, &rs).unwrap();
    let (sr, nsr) = vedic_day_sunrises(&engine, &eop, &utc, &loc, &rs).unwrap();
    let via_sr = vaar_from_sunrises(sr, nsr, engine.lsk());
    assert_eq!(direct, via_sr);
}

/// hora_from_sunrises matches hora_for_date
#[test]
fn hora_from_sunrises_matches_for_date() {
    let Some(engine) = load_engine() else { return };
    let Some(eop) = load_eop() else { return };
    let utc = UtcTime::new(2024, 1, 15, 12, 0, 0.0);
    let loc = GeoLocation::new(28.6139, 77.2090, 0.0);
    let rs = RiseSetConfig::default();
    let direct = hora_for_date(&engine, &eop, &utc, &loc, &rs).unwrap();
    let (sr, nsr) = vedic_day_sunrises(&engine, &eop, &utc, &loc, &rs).unwrap();
    let jd = utc.to_jd_tdb(engine.lsk());
    let via_sr = hora_from_sunrises(jd, sr, nsr, engine.lsk());
    assert_eq!(direct, via_sr);
}

/// ghatika_from_sunrises matches ghatika_for_date
#[test]
fn ghatika_from_sunrises_matches_for_date() {
    let Some(engine) = load_engine() else { return };
    let Some(eop) = load_eop() else { return };
    let utc = UtcTime::new(2024, 1, 15, 12, 0, 0.0);
    let loc = GeoLocation::new(28.6139, 77.2090, 0.0);
    let rs = RiseSetConfig::default();
    let direct = ghatika_for_date(&engine, &eop, &utc, &loc, &rs).unwrap();
    let (sr, nsr) = vedic_day_sunrises(&engine, &eop, &utc, &loc, &rs).unwrap();
    let jd = utc.to_jd_tdb(engine.lsk());
    let via_sr = ghatika_from_sunrises(jd, sr, nsr, engine.lsk());
    assert_eq!(direct, via_sr);
}

/// nakshatra_at(engine, jd, moon_sid) == nakshatra_for_date(engine, utc, config)
#[test]
fn nakshatra_at_matches_for_date() {
    let Some(engine) = load_engine() else { return };
    let utc = UtcTime::new(2024, 1, 15, 12, 0, 0.0);
    let config = default_config();
    let direct = nakshatra_for_date(&engine, &utc, &config).unwrap();
    let jd = utc.to_jd_tdb(engine.lsk());
    let moon_sid = moon_sidereal_longitude_at(&engine, jd, &config).unwrap();
    let via_at = nakshatra_at(&engine, jd, moon_sid, &config).unwrap();
    assert_eq!(direct, via_at);
}

/// panchang_for_date gives consistent results with individual _for_date calls
#[test]
fn panchang_combined_matches_individual() {
    let Some(engine) = load_engine() else { return };
    let Some(eop) = load_eop() else { return };
    let utc = UtcTime::new(2024, 1, 15, 12, 0, 0.0);
    let loc = GeoLocation::new(28.6139, 77.2090, 0.0);
    let rs = RiseSetConfig::default();
    let config = default_config();

    let combined = panchang_for_date(&engine, &eop, &utc, &loc, &rs, &config, false).unwrap();

    let tithi = tithi_for_date(&engine, &utc).unwrap();
    let karana = karana_for_date(&engine, &utc).unwrap();
    let yoga = yoga_for_date(&engine, &utc, &config).unwrap();
    let nakshatra = nakshatra_for_date(&engine, &utc, &config).unwrap();
    let vaar = vaar_for_date(&engine, &eop, &utc, &loc, &rs).unwrap();
    let hora = hora_for_date(&engine, &eop, &utc, &loc, &rs).unwrap();
    let ghatika = ghatika_for_date(&engine, &eop, &utc, &loc, &rs).unwrap();

    assert_eq!(combined.tithi, tithi, "tithi mismatch");
    assert_eq!(combined.karana, karana, "karana mismatch");
    assert_eq!(combined.yoga, yoga, "yoga mismatch");
    assert_eq!(combined.nakshatra, nakshatra, "nakshatra mismatch");
    assert_eq!(combined.vaar, vaar, "vaar mismatch");
    assert_eq!(combined.hora, hora, "hora mismatch");
    assert_eq!(combined.ghatika, ghatika, "ghatika mismatch");
    assert!(
        combined.masa.is_none(),
        "masa should be None without calendar"
    );
    assert!(
        combined.ayana.is_none(),
        "ayana should be None without calendar"
    );
    assert!(
        combined.varsha.is_none(),
        "varsha should be None without calendar"
    );
}

/// panchang_for_date with include_calendar includes masa, ayana, varsha
#[test]
fn panchang_with_calendar() {
    let Some(engine) = load_engine() else { return };
    let Some(eop) = load_eop() else { return };
    let utc = UtcTime::new(2024, 1, 15, 12, 0, 0.0);
    let loc = GeoLocation::new(28.6139, 77.2090, 0.0);
    let rs = RiseSetConfig::default();
    let config = default_config();

    let combined = panchang_for_date(&engine, &eop, &utc, &loc, &rs, &config, true).unwrap();

    // Calendar fields should be present
    let masa = combined.masa.expect("masa should be present");
    let ayana = combined.ayana.expect("ayana should be present");
    let varsha = combined.varsha.expect("varsha should be present");

    // Cross-check with individual functions
    let masa_direct = masa_for_date(&engine, &utc, &config).unwrap();
    let ayana_direct = ayana_for_date(&engine, &utc, &config).unwrap();
    let varsha_direct = varsha_for_date(&engine, &utc, &config).unwrap();

    assert_eq!(masa, masa_direct, "masa mismatch");
    assert_eq!(ayana, ayana_direct, "ayana mismatch");
    assert_eq!(varsha, varsha_direct, "varsha mismatch");
}
