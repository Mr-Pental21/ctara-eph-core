//! Panchang classification: Tithi, Karana, Yoga, Nakshatra, Vaar, Hora,
//! Ghatika, Masa, Ayana, and Varsha determination.
//!
//! Given a UTC date, these functions determine the panchang elements.
//! All functions accept and return UTC times; JD TDB is internal only.
//!
//! Clean-room implementation from standard Vedic panchang conventions.

use dhruv_core::{Body, Engine};
use dhruv_time::{EopKernel, LeapSecondKernel, UtcTime, calendar_to_jd};
use dhruv_vedic_base::{
    Ayana, GeoLocation, HORA_COUNT, KARANA_SEGMENT_DEG, NAKSHATRA_SPAN_27, Rashi, RiseSetConfig,
    RiseSetEvent, RiseSetResult, TITHI_SEGMENT_DEG, YOGA_SEGMENT_DEG, approximate_local_noon_jd,
    ayana_from_sidereal_longitude, ayanamsha_deg, compute_rise_set, ghatika_from_elapsed, hora_at,
    jd_tdb_to_centuries, karana_from_elongation, masa_from_rashi_index, nakshatra_from_longitude,
    rashi_from_longitude, samvatsara_from_year, tithi_from_elongation, vaar_from_jd, yoga_from_sum,
};

use crate::conjunction::body_ecliptic_lon_lat;
use crate::error::SearchError;
use crate::lunar_phase::{next_amavasya, prev_amavasya};
use crate::panchang_types::{
    AyanaInfo, GhatikaInfo, HoraInfo, KaranaInfo, MasaInfo, PanchangInfo, PanchangNakshatraInfo,
    TithiInfo, VaarInfo, VarshaInfo, YogaInfo,
};
use crate::sankranti::{next_specific_sankranti, prev_specific_sankranti};
use crate::sankranti_types::SankrantiConfig;
use crate::search_util::{find_zero_crossing, normalize_to_pm180};

/// Get Sun's sidereal rashi index at a given JD TDB.
fn sun_sidereal_rashi_index(
    engine: &Engine,
    jd_tdb: f64,
    config: &SankrantiConfig,
) -> Result<u8, SearchError> {
    let (tropical_lon, _lat) = body_ecliptic_lon_lat(engine, dhruv_core::Body::Sun, jd_tdb)?;
    let t = jd_tdb_to_centuries(jd_tdb);
    let aya = ayanamsha_deg(config.ayanamsha_system, t, config.use_nutation);
    let sid = (tropical_lon - aya).rem_euclid(360.0);
    Ok(rashi_from_longitude(sid).rashi_index)
}

/// Get Sun's sidereal longitude at a given JD TDB.
fn sun_sidereal_longitude(
    engine: &Engine,
    jd_tdb: f64,
    config: &SankrantiConfig,
) -> Result<f64, SearchError> {
    let (tropical_lon, _lat) = body_ecliptic_lon_lat(engine, dhruv_core::Body::Sun, jd_tdb)?;
    let t = jd_tdb_to_centuries(jd_tdb);
    let aya = ayanamsha_deg(config.ayanamsha_system, t, config.use_nutation);
    Ok((tropical_lon - aya).rem_euclid(360.0))
}

/// Determine the Masa (lunar month, Amanta system) for a given date.
///
/// Amanta: month runs from new moon to new moon.
/// Month is named after the rashi the Sun is in at the *next* new moon.
/// If the Sun's rashi doesn't change between prev and next new moon, it's an adhika month.
pub fn masa_for_date(
    engine: &Engine,
    utc: &UtcTime,
    config: &SankrantiConfig,
) -> Result<MasaInfo, SearchError> {
    // Find bracketing new moons
    let prev_nm = prev_amavasya(engine, utc)?.ok_or(SearchError::NoConvergence(
        "could not find previous new moon",
    ))?;
    let next_nm = next_amavasya(engine, utc)?
        .ok_or(SearchError::NoConvergence("could not find next new moon"))?;

    let prev_nm_jd = prev_nm.utc.to_jd_tdb(engine.lsk());
    let next_nm_jd = next_nm.utc.to_jd_tdb(engine.lsk());

    // Sun's sidereal rashi at each new moon
    let rashi_at_prev = sun_sidereal_rashi_index(engine, prev_nm_jd, config)?;
    let rashi_at_next = sun_sidereal_rashi_index(engine, next_nm_jd, config)?;

    let (masa, adhika) = if rashi_at_prev != rashi_at_next {
        // Normal month: named after rashi at next new moon
        (masa_from_rashi_index(rashi_at_next), false)
    } else {
        // Adhika month: Sun stayed in the same rashi
        // Named after the next rashi (unchanged_rashi + 1)
        (masa_from_rashi_index((rashi_at_prev + 1) % 12), true)
    };

    Ok(MasaInfo {
        masa,
        adhika,
        start: prev_nm.utc,
        end: next_nm.utc,
    })
}

/// Determine the Ayana (solstice period) for a given date.
///
/// Uttarayana starts at Makar Sankranti (Sun enters Makara, sidereal 270 deg).
/// Dakshinayana starts at Karka Sankranti (Sun enters Karka, sidereal 90 deg).
pub fn ayana_for_date(
    engine: &Engine,
    utc: &UtcTime,
    config: &SankrantiConfig,
) -> Result<AyanaInfo, SearchError> {
    let jd = utc.to_jd_tdb(engine.lsk());
    let sid_lon = sun_sidereal_longitude(engine, jd, config)?;
    let current_ayana = ayana_from_sidereal_longitude(sid_lon);

    // Determine start/end based on which ayana we're in
    let (start_rashi, end_rashi) = match current_ayana {
        Ayana::Uttarayana => (Rashi::Makara, Rashi::Karka),
        Ayana::Dakshinayana => (Rashi::Karka, Rashi::Makara),
    };

    // Find the start of this ayana (previous transition)
    let start_event = prev_specific_sankranti(engine, utc, start_rashi, config)?.ok_or(
        SearchError::NoConvergence("could not find ayana start sankranti"),
    )?;

    // Find the end of this ayana (next transition)
    let end_event = next_specific_sankranti(engine, utc, end_rashi, config)?.ok_or(
        SearchError::NoConvergence("could not find ayana end sankranti"),
    )?;

    Ok(AyanaInfo {
        ayana: current_ayana,
        start: start_event.utc,
        end: end_event.utc,
    })
}

/// Determine the Varsha (60-year samvatsara cycle position) for a given date.
///
/// The Vedic year starts at Chaitra Pratipada: the first new moon after Mesha Sankranti
/// (Sun entering sidereal 0 deg / Mesha rashi).
pub fn varsha_for_date(
    engine: &Engine,
    utc: &UtcTime,
    config: &SankrantiConfig,
) -> Result<VarshaInfo, SearchError> {
    // Strategy: find Mesha Sankranti for this year, then next new moon after it = year start.
    // If that year start is after our date, go back one year.

    let year_start = find_chaitra_pratipada_for(engine, utc, config)?;
    let year_start_jd = year_start.to_jd_tdb(engine.lsk());
    let jd = utc.to_jd_tdb(engine.lsk());

    let (actual_start, actual_end) = if year_start_jd > jd {
        // Our date is before this year's Chaitra Pratipada — go back one year
        let prev_year_utc = UtcTime::new(utc.year - 1, 1, 15, 0, 0, 0.0);
        let prev_start = find_chaitra_pratipada_for(engine, &prev_year_utc, config)?;
        (prev_start, year_start)
    } else {
        // Find next year's Chaitra Pratipada
        let next_year_utc = UtcTime::new(utc.year + 1, 1, 15, 0, 0, 0.0);
        let next_start = find_chaitra_pratipada_for(engine, &next_year_utc, config)?;
        let next_start_jd = next_start.to_jd_tdb(engine.lsk());
        if next_start_jd <= jd {
            // Edge case: we're after next year's start too
            let following_year_utc = UtcTime::new(utc.year + 2, 1, 15, 0, 0, 0.0);
            let following_start = find_chaitra_pratipada_for(engine, &following_year_utc, config)?;
            (next_start, following_start)
        } else {
            (year_start, next_start)
        }
    };

    // Use the calendar year of the start to determine the samvatsara
    let (samvatsara, order) = samvatsara_from_year(actual_start.year);

    Ok(VarshaInfo {
        samvatsara,
        order,
        start: actual_start,
        end: actual_end,
    })
}

/// Find Chaitra Pratipada (Vedic new year) near a given date.
///
/// Chaitra Pratipada = first new moon after Mesha Sankranti in the same calendar year.
fn find_chaitra_pratipada_for(
    engine: &Engine,
    utc: &UtcTime,
    config: &SankrantiConfig,
) -> Result<UtcTime, SearchError> {
    // Find Mesha Sankranti (Sun entering sidereal 0 deg) near this date
    let search_start = UtcTime::new(utc.year, 1, 15, 0, 0, 0.0);
    let mesha_sankranti = next_specific_sankranti(engine, &search_start, Rashi::Mesha, config)?
        .ok_or(SearchError::NoConvergence("could not find Mesha Sankranti"))?;

    // Find the next new moon after Mesha Sankranti
    let nm = next_amavasya(engine, &mesha_sankranti.utc)?.ok_or(SearchError::NoConvergence(
        "could not find new moon after Mesha Sankranti",
    ))?;

    Ok(nm.utc)
}

// ---------------------------------------------------------------------------
// Category A: Tithi, Karana, Yoga (angular search)
// ---------------------------------------------------------------------------

/// Moon-Sun elongation in tropical coordinates at a given JD TDB.
///
/// Returns (Moon_lon - Sun_lon) mod 360 in degrees [0, 360).
/// Ayanamsha cancels in the difference, so tropical coords suffice.
pub fn elongation_at(engine: &Engine, jd_tdb: f64) -> Result<f64, SearchError> {
    let (moon_lon, _) = body_ecliptic_lon_lat(engine, Body::Moon, jd_tdb)?;
    let (sun_lon, _) = body_ecliptic_lon_lat(engine, Body::Sun, jd_tdb)?;
    Ok((moon_lon - sun_lon).rem_euclid(360.0))
}

/// Sum of Moon and Sun sidereal longitudes at a given JD TDB.
///
/// Returns (Moon_sid + Sun_sid) mod 360 in degrees [0, 360).
/// Ayanamsha does NOT cancel in the sum, so sidereal coords are needed.
pub fn sidereal_sum_at(
    engine: &Engine,
    jd_tdb: f64,
    config: &SankrantiConfig,
) -> Result<f64, SearchError> {
    let (moon_trop, _) = body_ecliptic_lon_lat(engine, Body::Moon, jd_tdb)?;
    let (sun_trop, _) = body_ecliptic_lon_lat(engine, Body::Sun, jd_tdb)?;
    let t = jd_tdb_to_centuries(jd_tdb);
    let aya = ayanamsha_deg(config.ayanamsha_system, t, config.use_nutation);
    let moon_sid = (moon_trop - aya).rem_euclid(360.0);
    let sun_sid = (sun_trop - aya).rem_euclid(360.0);
    Ok((moon_sid + sun_sid).rem_euclid(360.0))
}

/// Moon's sidereal longitude at a given JD TDB.
///
/// Returns Moon_sid mod 360 in degrees [0, 360).
pub fn moon_sidereal_longitude_at(
    engine: &Engine,
    jd_tdb: f64,
    config: &SankrantiConfig,
) -> Result<f64, SearchError> {
    let (moon_trop, _) = body_ecliptic_lon_lat(engine, Body::Moon, jd_tdb)?;
    let t = jd_tdb_to_centuries(jd_tdb);
    let aya = ayanamsha_deg(config.ayanamsha_system, t, config.use_nutation);
    Ok((moon_trop - aya).rem_euclid(360.0))
}

/// Determine the Moon's Nakshatra (27-scheme) for a given date.
///
/// Returns nakshatra name, index, pada, and start/end times.
pub fn nakshatra_for_date(
    engine: &Engine,
    utc: &UtcTime,
    config: &SankrantiConfig,
) -> Result<PanchangNakshatraInfo, SearchError> {
    let jd = utc.to_jd_tdb(engine.lsk());
    let moon_sid = moon_sidereal_longitude_at(engine, jd, config)?;
    nakshatra_at(engine, jd, moon_sid, config)
}

/// Determine the Moon's Nakshatra from a pre-computed sidereal longitude.
///
/// Accepts Moon's sidereal longitude in degrees [0, 360) at `jd_tdb`.
/// The engine is still needed for boundary bisection (finding start/end times).
pub fn nakshatra_at(
    engine: &Engine,
    jd_tdb: f64,
    moon_sidereal_deg: f64,
    config: &SankrantiConfig,
) -> Result<PanchangNakshatraInfo, SearchError> {
    let pos = nakshatra_from_longitude(moon_sidereal_deg);

    let start_target = (pos.nakshatra_index as f64) * NAKSHATRA_SPAN_27;
    let end_target = ((pos.nakshatra_index as f64) + 1.0) * NAKSHATRA_SPAN_27;

    // Moon moves ~13.2 deg/day, so one nakshatra (~13.33 deg) ≈ 1 day.
    // Step 0.5 days for boundary search.
    let moon_fn =
        |t: f64| -> Result<f64, SearchError> { moon_sidereal_longitude_at(engine, t, config) };

    let start_jd = find_angle_boundary(&moon_fn, jd_tdb, start_target, -0.5, 20)?
        .ok_or(SearchError::NoConvergence("could not find nakshatra start"))?;
    let end_jd = find_angle_boundary(&moon_fn, jd_tdb, end_target, 0.5, 20)?
        .ok_or(SearchError::NoConvergence("could not find nakshatra end"))?;

    Ok(PanchangNakshatraInfo {
        nakshatra: pos.nakshatra,
        nakshatra_index: pos.nakshatra_index,
        pada: pos.pada,
        start: UtcTime::from_jd_tdb(start_jd, engine.lsk()),
        end: UtcTime::from_jd_tdb(end_jd, engine.lsk()),
    })
}

/// Generic boundary search for angular segments.
///
/// Finds the JD TDB where `f(t) = target_deg` by searching for a zero of
/// `normalize(f(t) - target_deg)`. Searches from `jd_start` in the given direction.
fn find_angle_boundary(
    f: &dyn Fn(f64) -> Result<f64, SearchError>,
    jd_start: f64,
    target_deg: f64,
    step: f64,
    max_steps: usize,
) -> Result<Option<f64>, SearchError> {
    let wrapped = |t: f64| -> Result<f64, SearchError> {
        let val = f(t)?;
        Ok(normalize_to_pm180(val - target_deg))
    };
    find_zero_crossing(&wrapped, jd_start, step, max_steps, 50, 1e-8)
}

/// Convert UtcTime to JD UTC (calendar-only, no LSK).
fn utc_to_jd_utc(utc: &UtcTime) -> f64 {
    let day_frac = utc.day as f64
        + utc.hour as f64 / 24.0
        + utc.minute as f64 / 1440.0
        + utc.second / 86_400.0;
    calendar_to_jd(utc.year, utc.month, day_frac)
}

/// Determine the Tithi (lunar day) for a given date.
///
/// Tithi = which of 30 segments of Moon-Sun elongation (12 deg each) the
/// current moment falls in. Returns tithi with start/end UTC times.
pub fn tithi_for_date(engine: &Engine, utc: &UtcTime) -> Result<TithiInfo, SearchError> {
    let jd = utc.to_jd_tdb(engine.lsk());
    let elong = elongation_at(engine, jd)?;
    tithi_at(engine, jd, elong)
}

/// Determine the Tithi from a pre-computed elongation.
///
/// Accepts the Moon-Sun elongation in degrees [0, 360) at `jd_tdb`.
/// The engine is still needed for boundary bisection (finding start/end times).
pub fn tithi_at(
    engine: &Engine,
    jd_tdb: f64,
    elongation_deg: f64,
) -> Result<TithiInfo, SearchError> {
    let pos = tithi_from_elongation(elongation_deg);

    let start_target = (pos.tithi_index as f64) * TITHI_SEGMENT_DEG;
    let end_target = ((pos.tithi_index as f64) + 1.0) * TITHI_SEGMENT_DEG;

    // Search backward for start boundary, forward for end boundary
    // Step 0.25 days (~12 deg/day relative motion, tithi ~1 day)
    let elong_fn = |t: f64| -> Result<f64, SearchError> { elongation_at(engine, t) };

    let start_jd = find_angle_boundary(&elong_fn, jd_tdb, start_target, -0.25, 20)?
        .ok_or(SearchError::NoConvergence("could not find tithi start"))?;
    let end_jd = find_angle_boundary(&elong_fn, jd_tdb, end_target, 0.25, 20)?
        .ok_or(SearchError::NoConvergence("could not find tithi end"))?;

    Ok(TithiInfo {
        tithi: pos.tithi,
        tithi_index: pos.tithi_index,
        paksha: pos.paksha,
        tithi_in_paksha: pos.tithi_in_paksha,
        start: UtcTime::from_jd_tdb(start_jd, engine.lsk()),
        end: UtcTime::from_jd_tdb(end_jd, engine.lsk()),
    })
}

/// Determine the Karana (half-tithi) for a given date.
///
/// Karana = which of 60 segments of Moon-Sun elongation (6 deg each) the
/// current moment falls in. Uses traditional fixed/movable mapping.
pub fn karana_for_date(engine: &Engine, utc: &UtcTime) -> Result<KaranaInfo, SearchError> {
    let jd = utc.to_jd_tdb(engine.lsk());
    let elong = elongation_at(engine, jd)?;
    karana_at(engine, jd, elong)
}

/// Determine the Karana from a pre-computed elongation.
///
/// Accepts the Moon-Sun elongation in degrees [0, 360) at `jd_tdb`.
/// The engine is still needed for boundary bisection (finding start/end times).
pub fn karana_at(
    engine: &Engine,
    jd_tdb: f64,
    elongation_deg: f64,
) -> Result<KaranaInfo, SearchError> {
    let pos = karana_from_elongation(elongation_deg);

    let start_target = (pos.karana_index as f64) * KARANA_SEGMENT_DEG;
    let end_target = ((pos.karana_index as f64) + 1.0) * KARANA_SEGMENT_DEG;

    let elong_fn = |t: f64| -> Result<f64, SearchError> { elongation_at(engine, t) };

    let start_jd = find_angle_boundary(&elong_fn, jd_tdb, start_target, -0.25, 20)?
        .ok_or(SearchError::NoConvergence("could not find karana start"))?;
    let end_jd = find_angle_boundary(&elong_fn, jd_tdb, end_target, 0.25, 20)?
        .ok_or(SearchError::NoConvergence("could not find karana end"))?;

    Ok(KaranaInfo {
        karana: pos.karana,
        karana_index: pos.karana_index,
        start: UtcTime::from_jd_tdb(start_jd, engine.lsk()),
        end: UtcTime::from_jd_tdb(end_jd, engine.lsk()),
    })
}

/// Determine the Yoga (luni-solar yoga) for a given date.
///
/// Yoga = which of 27 segments of (Moon_sid + Sun_sid) mod 360
/// (~13.33 deg each) the current moment falls in.
/// Requires SankrantiConfig for ayanamsha (sum does not cancel).
pub fn yoga_for_date(
    engine: &Engine,
    utc: &UtcTime,
    config: &SankrantiConfig,
) -> Result<YogaInfo, SearchError> {
    let jd = utc.to_jd_tdb(engine.lsk());
    let sum = sidereal_sum_at(engine, jd, config)?;
    yoga_at(engine, jd, sum, config)
}

/// Determine the Yoga from a pre-computed sidereal sum.
///
/// Accepts (Moon_sid + Sun_sid) mod 360 in degrees [0, 360) at `jd_tdb`.
/// The engine is still needed for boundary bisection (finding start/end times).
pub fn yoga_at(
    engine: &Engine,
    jd_tdb: f64,
    sidereal_sum_deg: f64,
    config: &SankrantiConfig,
) -> Result<YogaInfo, SearchError> {
    let pos = yoga_from_sum(sidereal_sum_deg);

    let start_target = (pos.yoga_index as f64) * YOGA_SEGMENT_DEG;
    let end_target = ((pos.yoga_index as f64) + 1.0) * YOGA_SEGMENT_DEG;

    let sum_fn = |t: f64| -> Result<f64, SearchError> { sidereal_sum_at(engine, t, config) };

    let start_jd = find_angle_boundary(&sum_fn, jd_tdb, start_target, -0.25, 20)?
        .ok_or(SearchError::NoConvergence("could not find yoga start"))?;
    let end_jd = find_angle_boundary(&sum_fn, jd_tdb, end_target, 0.25, 20)?
        .ok_or(SearchError::NoConvergence("could not find yoga end"))?;

    Ok(YogaInfo {
        yoga: pos.yoga,
        yoga_index: pos.yoga_index,
        start: UtcTime::from_jd_tdb(start_jd, engine.lsk()),
        end: UtcTime::from_jd_tdb(end_jd, engine.lsk()),
    })
}

// ---------------------------------------------------------------------------
// Category B: Vaar, Hora, Ghatika (sunrise-based)
// ---------------------------------------------------------------------------

/// Compute the Vedic day sunrise bracket for a given UTC moment.
///
/// Returns (sunrise_jd_tdb, next_sunrise_jd_tdb) defining the Vedic day
/// that contains the given moment. If the moment is before today's sunrise,
/// uses yesterday's sunrise as the start.
pub fn vedic_day_sunrises(
    engine: &Engine,
    eop: &EopKernel,
    utc: &UtcTime,
    location: &GeoLocation,
    riseset_config: &RiseSetConfig,
) -> Result<(f64, f64), SearchError> {
    let jd_utc = utc_to_jd_utc(utc);
    let jd_tdb = utc.to_jd_tdb(engine.lsk());

    // Approximate local noon for today
    let jd_midnight = jd_utc.floor() + 0.5; // 0h UT
    let jd_noon = approximate_local_noon_jd(jd_midnight, location.longitude_deg);

    // Today's sunrise
    let today_result = compute_rise_set(
        engine,
        engine.lsk(),
        eop,
        location,
        RiseSetEvent::Sunrise,
        jd_noon,
        riseset_config,
    )
    .map_err(|_| SearchError::NoConvergence("sunrise computation failed"))?;

    let today_sunrise_jd = match today_result {
        RiseSetResult::Event { jd_tdb, .. } => jd_tdb,
        _ => {
            return Err(SearchError::NoConvergence(
                "sun never rises at this location",
            ));
        }
    };

    if jd_tdb >= today_sunrise_jd {
        // Moment is after today's sunrise → vedic day = today sunrise to tomorrow sunrise
        let tomorrow_noon = jd_noon + 1.0;
        let tomorrow_result = compute_rise_set(
            engine,
            engine.lsk(),
            eop,
            location,
            RiseSetEvent::Sunrise,
            tomorrow_noon,
            riseset_config,
        )
        .map_err(|_| SearchError::NoConvergence("next sunrise computation failed"))?;
        let next_sunrise_jd = match tomorrow_result {
            RiseSetResult::Event { jd_tdb, .. } => jd_tdb,
            _ => return Err(SearchError::NoConvergence("sun never rises next day")),
        };
        Ok((today_sunrise_jd, next_sunrise_jd))
    } else {
        // Moment is before today's sunrise → vedic day = yesterday sunrise to today sunrise
        let yesterday_noon = jd_noon - 1.0;
        let yesterday_result = compute_rise_set(
            engine,
            engine.lsk(),
            eop,
            location,
            RiseSetEvent::Sunrise,
            yesterday_noon,
            riseset_config,
        )
        .map_err(|_| SearchError::NoConvergence("previous sunrise computation failed"))?;
        let yesterday_sunrise_jd = match yesterday_result {
            RiseSetResult::Event { jd_tdb, .. } => jd_tdb,
            _ => return Err(SearchError::NoConvergence("sun never rose yesterday")),
        };
        Ok((yesterday_sunrise_jd, today_sunrise_jd))
    }
}

/// Determine the Vaar (weekday) for a given date and location.
///
/// The Vedic day runs from sunrise to next sunrise. The weekday of the
/// sunrise determines the vaar.
pub fn vaar_for_date(
    engine: &Engine,
    eop: &EopKernel,
    utc: &UtcTime,
    location: &GeoLocation,
    riseset_config: &RiseSetConfig,
) -> Result<VaarInfo, SearchError> {
    let (sunrise_jd, next_sunrise_jd) =
        vedic_day_sunrises(engine, eop, utc, location, riseset_config)?;
    Ok(vaar_from_sunrises(
        sunrise_jd,
        next_sunrise_jd,
        engine.lsk(),
    ))
}

/// Determine the Vaar from pre-computed sunrise boundaries.
///
/// Pure arithmetic — no engine queries needed. The `lsk` is used only
/// for converting JD TDB to UTC in the start/end fields.
pub fn vaar_from_sunrises(
    sunrise_jd: f64,
    next_sunrise_jd: f64,
    lsk: &LeapSecondKernel,
) -> VaarInfo {
    let vaar = vaar_from_jd(sunrise_jd);
    VaarInfo {
        vaar,
        start: UtcTime::from_jd_tdb(sunrise_jd, lsk),
        end: UtcTime::from_jd_tdb(next_sunrise_jd, lsk),
    }
}

/// Determine the Hora (planetary hour) for a given date and location.
///
/// The Vedic day is divided into 24 equal horas. The ruling planet
/// follows the Chaldean sequence starting from the day lord.
pub fn hora_for_date(
    engine: &Engine,
    eop: &EopKernel,
    utc: &UtcTime,
    location: &GeoLocation,
    riseset_config: &RiseSetConfig,
) -> Result<HoraInfo, SearchError> {
    let (sunrise_jd, next_sunrise_jd) =
        vedic_day_sunrises(engine, eop, utc, location, riseset_config)?;
    let jd_tdb = utc.to_jd_tdb(engine.lsk());
    Ok(hora_from_sunrises(
        jd_tdb,
        sunrise_jd,
        next_sunrise_jd,
        engine.lsk(),
    ))
}

/// Determine the Hora from pre-computed sunrise boundaries.
///
/// Pure arithmetic — no engine queries needed. The `lsk` is used only
/// for converting JD TDB to UTC in the start/end fields.
pub fn hora_from_sunrises(
    jd_tdb: f64,
    sunrise_jd: f64,
    next_sunrise_jd: f64,
    lsk: &LeapSecondKernel,
) -> HoraInfo {
    let vedic_day_seconds = (next_sunrise_jd - sunrise_jd) * 86400.0;
    let seconds_since_sunrise = (jd_tdb - sunrise_jd) * 86400.0;
    let hora_duration_seconds = vedic_day_seconds / HORA_COUNT as f64;

    let mut hora_index = (seconds_since_sunrise / hora_duration_seconds).floor() as u8;
    if hora_index >= HORA_COUNT {
        hora_index = HORA_COUNT - 1;
    }

    let vaar = vaar_from_jd(sunrise_jd);
    let hora = hora_at(vaar, hora_index);

    let hora_start_jd = sunrise_jd + (hora_index as f64 * hora_duration_seconds) / 86400.0;
    let hora_end_jd = hora_start_jd + hora_duration_seconds / 86400.0;

    HoraInfo {
        hora,
        hora_index,
        start: UtcTime::from_jd_tdb(hora_start_jd, lsk),
        end: UtcTime::from_jd_tdb(hora_end_jd, lsk),
    }
}

/// Determine the Ghatika for a given date and location.
///
/// The Vedic day is divided into 60 equal ghatikas (each ~24 minutes
/// for a standard day). Returns the ghatika number (1-60) with start/end.
pub fn ghatika_for_date(
    engine: &Engine,
    eop: &EopKernel,
    utc: &UtcTime,
    location: &GeoLocation,
    riseset_config: &RiseSetConfig,
) -> Result<GhatikaInfo, SearchError> {
    let (sunrise_jd, next_sunrise_jd) =
        vedic_day_sunrises(engine, eop, utc, location, riseset_config)?;
    let jd_tdb = utc.to_jd_tdb(engine.lsk());
    Ok(ghatika_from_sunrises(
        jd_tdb,
        sunrise_jd,
        next_sunrise_jd,
        engine.lsk(),
    ))
}

/// Determine the Ghatika from pre-computed sunrise boundaries.
///
/// Pure arithmetic — no engine queries needed. The `lsk` is used only
/// for converting JD TDB to UTC in the start/end fields.
pub fn ghatika_from_sunrises(
    jd_tdb: f64,
    sunrise_jd: f64,
    next_sunrise_jd: f64,
    lsk: &LeapSecondKernel,
) -> GhatikaInfo {
    let vedic_day_seconds = (next_sunrise_jd - sunrise_jd) * 86400.0;
    let seconds_since_sunrise = (jd_tdb - sunrise_jd) * 86400.0;

    let pos = ghatika_from_elapsed(seconds_since_sunrise, vedic_day_seconds);
    let ghatika_duration = vedic_day_seconds / 60.0;
    let ghatika_start_jd = sunrise_jd + (pos.index as f64 * ghatika_duration) / 86400.0;
    let ghatika_end_jd = ghatika_start_jd + ghatika_duration / 86400.0;

    GhatikaInfo {
        value: pos.value,
        start: UtcTime::from_jd_tdb(ghatika_start_jd, lsk),
        end: UtcTime::from_jd_tdb(ghatika_end_jd, lsk),
    }
}

// ---------------------------------------------------------------------------
// Combined panchang
// ---------------------------------------------------------------------------

/// Compute all six daily panchang elements (tithi, karana, yoga, vaar, hora,
/// ghatika) for a single moment, sharing intermediate computations.
///
/// This is more efficient than calling the six `_for_date` functions
/// individually because Sun/Moon longitudes are queried once (instead of 3x)
/// and sunrise is computed once (instead of 3x).
///
/// When `include_calendar` is true, also computes masa (lunar month),
/// ayana (solstice period), and varsha (60-year samvatsara cycle).
pub fn panchang_for_date(
    engine: &Engine,
    eop: &EopKernel,
    utc: &UtcTime,
    location: &GeoLocation,
    riseset_config: &RiseSetConfig,
    config: &SankrantiConfig,
    include_calendar: bool,
) -> Result<PanchangInfo, SearchError> {
    let jd = utc.to_jd_tdb(engine.lsk());

    // Category A intermediates: compute body longitudes once
    let elong = elongation_at(engine, jd)?;
    let sum = sidereal_sum_at(engine, jd, config)?;
    let moon_sid = moon_sidereal_longitude_at(engine, jd, config)?;

    let tithi = tithi_at(engine, jd, elong)?;
    let karana = karana_at(engine, jd, elong)?;
    let yoga = yoga_at(engine, jd, sum, config)?;
    let nakshatra = nakshatra_at(engine, jd, moon_sid, config)?;

    // Category B intermediates: compute sunrises once
    let (sunrise_jd, next_sunrise_jd) =
        vedic_day_sunrises(engine, eop, utc, location, riseset_config)?;

    let vaar = vaar_from_sunrises(sunrise_jd, next_sunrise_jd, engine.lsk());
    let hora = hora_from_sunrises(jd, sunrise_jd, next_sunrise_jd, engine.lsk());
    let ghatika = ghatika_from_sunrises(jd, sunrise_jd, next_sunrise_jd, engine.lsk());

    // Calendar elements (expensive — only when requested)
    let (masa, ayana, varsha) = if include_calendar {
        let m = masa_for_date(engine, utc, config)?;
        let a = ayana_for_date(engine, utc, config)?;
        let v = varsha_for_date(engine, utc, config)?;
        (Some(m), Some(a), Some(v))
    } else {
        (None, None, None)
    };

    Ok(PanchangInfo {
        tithi,
        karana,
        yoga,
        vaar,
        hora,
        ghatika,
        nakshatra,
        masa,
        ayana,
        varsha,
    })
}
