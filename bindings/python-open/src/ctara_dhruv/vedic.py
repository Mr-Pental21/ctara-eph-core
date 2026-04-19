"""Vedic base FFI wrappers.

Wraps all vedic-base functions from dhruv_ffi_c: rise/set, bhava, rashi,
nakshatra, name lookups, lunar nodes, sphutas, special lagnas, arudha padas,
upagrahas, ashtakavarga, drishti, and pure-math classifiers.
"""

from __future__ import annotations

from dataclasses import dataclass
from typing import Optional

from ._ffi import ffi, lib
from ._check import check
from .types import (
    ArudhaResult,
    AllUpagrahas,
    AshtakavargaResult,
    BhavaEntry,
    BhavaResult,
    BhinnaAshtakavarga,
    Dms,
    DrishtiEntry,
    DrishtiResult,
    GeoLocation,
    GrahaDrishtiMatrix,
    KaranaPosition,
    Nakshatra28Info,
    NakshatraInfo,
    PanchangNakshatraInfo,
    RashiInfo,
    RiseSetResult,
    SamvatsaraResult,
    SarvaAshtakavarga,
    SpecialLagnas,
    SphutalResult,
    TithiPosition,
    UtcTime,
    YogaPosition,
)


@dataclass(frozen=True, slots=True)
class RiseSetResultUtc:
    """Rise/set result with UTC time instead of JD TDB.

    ``result_type``: 0=event, 1=never rises, 2=never sets.
    ``event_code``: DHRUV_EVENT_* constant (valid when result_type==0).
    ``utc``: event time as UtcTime (valid when result_type==0).
    """

    result_type: int
    event_code: int
    utc: Optional[UtcTime]


# ---------------------------------------------------------------------------
# Helpers: Python types -> FFI structs
# ---------------------------------------------------------------------------


def _make_geo(loc: GeoLocation):
    """Create a DhruvGeoLocation FFI struct from a Python GeoLocation."""
    g = ffi.new("DhruvGeoLocation *")
    g.latitude_deg = loc.lat_deg
    g.longitude_deg = loc.lon_deg
    g.altitude_m = loc.alt_m
    return g


def _make_utc(utc: UtcTime):
    """Create a DhruvUtcTime FFI struct from a Python UtcTime."""
    t = ffi.new("DhruvUtcTime *")
    t.year = utc.year
    t.month = utc.month
    t.day = utc.day
    t.hour = utc.hour
    t.minute = utc.minute
    t.second = utc.second
    return t


def _str_or_none(ptr) -> Optional[str]:
    """Decode a C string pointer to Python str, or None if NULL."""
    if ptr == ffi.NULL:
        return None
    return ffi.string(ptr).decode("utf-8")


def _read_utc(t) -> UtcTime:
    """Read a DhruvUtcTime FFI struct into a Python UtcTime."""
    return UtcTime(
        year=t.year,
        month=t.month,
        day=t.day,
        hour=t.hour,
        minute=t.minute,
        second=t.second,
    )


def _drishti_entry(e) -> DrishtiEntry:
    """Convert a DhruvDrishtiEntry FFI struct to Python DrishtiEntry."""
    return DrishtiEntry(
        angular_distance=e.angular_distance,
        base_virupa=e.base_virupa,
        special_virupa=e.special_virupa,
        total_virupa=e.total_virupa,
    )


_UPAGRAHA_POINT_CODES = {
    "start": 0,
    "middle": 1,
    "end": 2,
}

_GULIKA_MAANDI_PLANET_CODES = {
    "rahu": 0,
    "saturn": 1,
}


def _normalize_upagraha_point(value) -> int:
    if isinstance(value, str):
        key = value.strip().lower().replace("_", "-")
        if key in _UPAGRAHA_POINT_CODES:
            return _UPAGRAHA_POINT_CODES[key]
        raise ValueError(f"invalid upagraha point: {value}")
    return int(value)


def _normalize_gulika_maandi_planet(value) -> int:
    if isinstance(value, str):
        key = value.strip().lower().replace("_", "-")
        if key in _GULIKA_MAANDI_PLANET_CODES:
            return _GULIKA_MAANDI_PLANET_CODES[key]
        raise ValueError(f"invalid Gulika/Maandi planet: {value}")
    return int(value)


def _make_time_upagraha_config(config):
    if config is None:
        return ffi.NULL
    if isinstance(config, ffi.CData):
        return ffi.addressof(config) if ffi.typeof(config) != ffi.typeof("DhruvTimeUpagrahaConfig *") else config

    cfg = ffi.new("DhruvTimeUpagrahaConfig *")
    cfg[0] = lib.dhruv_time_upagraha_config_default()
    cfg.gulika_point = _normalize_upagraha_point(config.get("gulika_point", cfg.gulika_point))
    cfg.maandi_point = _normalize_upagraha_point(config.get("maandi_point", cfg.maandi_point))
    cfg.other_point = _normalize_upagraha_point(config.get("other_point", cfg.other_point))
    cfg.gulika_planet = _normalize_gulika_maandi_planet(config.get("gulika_planet", cfg.gulika_planet))
    cfg.maandi_planet = _normalize_gulika_maandi_planet(config.get("maandi_planet", cfg.maandi_planet))
    return cfg


# ---------------------------------------------------------------------------
# 1. Rise / Set
# ---------------------------------------------------------------------------


def riseset_config_default():
    """Return the default DhruvRiseSetConfig (FFI struct, by value)."""
    return lib.dhruv_riseset_config_default()


def compute_rise_set(engine, lsk, eop, location: GeoLocation,
                     event_code: int, jd_utc_noon: float,
                     config=None) -> RiseSetResult:
    """Compute a single rise/set event.

    Args:
        engine: DhruvEngineHandle pointer.
        lsk: DhruvLskHandle pointer.
        eop: DhruvEopHandle pointer.
        location: Observer location.
        event_code: DHRUV_EVENT_* constant (0-7).
        jd_utc_noon: Approximate local noon (JD UTC).
        config: Optional DhruvRiseSetConfig pointer (NULL uses defaults).

    Returns:
        RiseSetResult with result_type, event_code, and jd_tdb.
    """
    geo = _make_geo(location)
    out = ffi.new("DhruvRiseSetResult *")
    cfg = config if config is not None else ffi.NULL
    status = lib.dhruv_compute_rise_set(engine, lsk, eop, geo, event_code,
                                        jd_utc_noon, cfg, out)
    check(status, "dhruv_compute_rise_set")
    return RiseSetResult(
        result_type=out.result_type,
        event_code=out.event_code,
        jd_tdb=out.jd_tdb,
    )


def sunrise(engine, lsk, eop, location: GeoLocation,
            jd_utc_noon: float, config=None) -> RiseSetResult:
    """Compute sunrise (event_code=0)."""
    return compute_rise_set(engine, lsk, eop, location, 0, jd_utc_noon, config)


def sunset(engine, lsk, eop, location: GeoLocation,
           jd_utc_noon: float, config=None) -> RiseSetResult:
    """Compute sunset (event_code=1)."""
    return compute_rise_set(engine, lsk, eop, location, 1, jd_utc_noon, config)


def compute_all_events(engine, lsk, eop, location: GeoLocation,
                       jd_utc_noon: float,
                       config=None) -> list[RiseSetResult]:
    """Compute all 8 rise/set events for a day.

    Returns list of 8 RiseSetResult in order: AstroDawn, NautDawn,
    CivilDawn, Sunrise, Sunset, CivilDusk, NautDusk, AstroDusk.
    """
    geo = _make_geo(location)
    out = ffi.new("DhruvRiseSetResult[8]")
    cfg = config if config is not None else ffi.NULL
    status = lib.dhruv_compute_all_events(engine, lsk, eop, geo,
                                          jd_utc_noon, cfg, out)
    check(status, "dhruv_compute_all_events")
    return [
        RiseSetResult(
            result_type=out[i].result_type,
            event_code=out[i].event_code,
            jd_tdb=out[i].jd_tdb,
        )
        for i in range(8)
    ]


def vedic_day_sunrises(engine, eop, utc: UtcTime, location: GeoLocation,
                       config=None) -> tuple[float, float]:
    """Compute Vedic day sunrise bracket.

    Returns (sunrise_jd_tdb, next_sunrise_jd_tdb).
    """
    t = _make_utc(utc)
    geo = _make_geo(location)
    sr = ffi.new("double *")
    nsr = ffi.new("double *")
    cfg = config if config is not None else ffi.NULL
    status = lib.dhruv_vedic_day_sunrises(engine, eop, t, geo, cfg, sr, nsr)
    check(status, "dhruv_vedic_day_sunrises")
    return (sr[0], nsr[0])


def compute_rise_set_utc(engine, lsk, eop, location: GeoLocation,
                         event_code: int, utc: UtcTime,
                         config=None) -> RiseSetResultUtc:
    """Compute a single rise/set event with UTC input/output.

    Args:
        engine: DhruvEngineHandle pointer.
        lsk: DhruvLskHandle pointer.
        eop: DhruvEopHandle pointer.
        location: Observer location.
        event_code: DHRUV_EVENT_* constant (0-7).
        utc: Approximate date as UtcTime.
        config: Optional DhruvRiseSetConfig pointer.

    Returns:
        RiseSetResultUtc with result_type, event_code, and utc.
    """
    geo = _make_geo(location)
    t = _make_utc(utc)
    out = ffi.new("DhruvRiseSetResultUtc *")
    cfg = config if config is not None else ffi.NULL
    status = lib.dhruv_compute_rise_set_utc(engine, lsk, eop, geo,
                                            event_code, t, cfg, out)
    check(status, "dhruv_compute_rise_set_utc")
    utc_out = _read_utc(out.utc) if out.result_type == 0 else None
    return RiseSetResultUtc(
        result_type=out.result_type,
        event_code=out.event_code,
        utc=utc_out,
    )


def compute_all_events_utc(engine, lsk, eop, location: GeoLocation,
                           utc: UtcTime,
                           config=None) -> list[RiseSetResultUtc]:
    """Compute all 8 rise/set events for a day with UTC input/output."""
    geo = _make_geo(location)
    t = _make_utc(utc)
    out = ffi.new("DhruvRiseSetResultUtc[8]")
    cfg = config if config is not None else ffi.NULL
    status = lib.dhruv_compute_all_events_utc(engine, lsk, eop, geo,
                                              t, cfg, out)
    check(status, "dhruv_compute_all_events_utc")
    return [
        RiseSetResultUtc(
            result_type=out[i].result_type,
            event_code=out[i].event_code,
            utc=_read_utc(out[i].utc) if out[i].result_type == 0 else None,
        )
        for i in range(8)
    ]


def approximate_local_noon_jd(jd_ut_midnight: float,
                              longitude_deg: float) -> float:
    """Approximate local noon JD from 0h UT JD and longitude. Pure math."""
    return lib.dhruv_approximate_local_noon_jd(jd_ut_midnight, longitude_deg)


def riseset_result_to_utc(lsk, result) -> UtcTime:
    """Convert a DhruvRiseSetResult (JD-based) event time to UTC.

    Args:
        lsk: DhruvLskHandle pointer.
        result: DhruvRiseSetResult pointer (from compute_rise_set).

    Returns:
        UtcTime of the event.
    """
    out = ffi.new("DhruvUtcTime *")
    status = lib.dhruv_riseset_result_to_utc(lsk, result, out)
    check(status, "dhruv_riseset_result_to_utc")
    return _read_utc(out[0])


# ---------------------------------------------------------------------------
# 2. Bhava (House Systems)
# ---------------------------------------------------------------------------


def bhava_config_default():
    """Return the default DhruvBhavaConfig (FFI struct, by value)."""
    return lib.dhruv_bhava_config_default()


def bhava_system_count() -> int:
    """Return number of supported bhava systems (currently 10)."""
    return lib.dhruv_bhava_system_count()


def compute_bhavas(engine, lsk, eop, location: GeoLocation,
                   jd_utc: float, config=None) -> BhavaResult:
    """Compute 12 bhava cusps with lagna and MC.

    Args:
        config: Optional DhruvBhavaConfig pointer. Set output fields to request
            sidereal longitude output.
    """
    geo = _make_geo(location)
    out = ffi.new("DhruvBhavaResult *")
    cfg = config if config is not None else ffi.NULL
    status = lib.dhruv_compute_bhavas(engine, lsk, eop, geo, jd_utc, cfg, out)
    check(status, "dhruv_compute_bhavas")
    bhavas = [
        BhavaEntry(
            number=out.bhavas[i].number,
            cusp_deg=out.bhavas[i].cusp_deg,
            start_deg=out.bhavas[i].start_deg,
            end_deg=out.bhavas[i].end_deg,
        )
        for i in range(12)
    ]
    return BhavaResult(bhavas=bhavas, lagna_deg=out.lagna_deg, mc_deg=out.mc_deg)


def lagna_deg(lsk, eop, location: GeoLocation, jd_utc: float, config=None) -> float:
    """Compute Ascendant longitude in degrees.

    Args:
        config: Optional DhruvBhavaConfig pointer for sidereal output.
    """
    geo = _make_geo(location)
    out = ffi.new("double *")
    if config is None:
        status = lib.dhruv_lagna_deg(lsk, eop, geo, jd_utc, out)
        check(status, "dhruv_lagna_deg")
    else:
        status = lib.dhruv_lagna_deg_with_config(lsk, eop, geo, jd_utc, config, out)
        check(status, "dhruv_lagna_deg_with_config")
    return out[0]


def mc_deg(lsk, eop, location: GeoLocation, jd_utc: float, config=None) -> float:
    """Compute MC (Midheaven) longitude in degrees.

    Args:
        config: Optional DhruvBhavaConfig pointer for sidereal output.
    """
    geo = _make_geo(location)
    out = ffi.new("double *")
    if config is None:
        status = lib.dhruv_mc_deg(lsk, eop, geo, jd_utc, out)
        check(status, "dhruv_mc_deg")
    else:
        status = lib.dhruv_mc_deg_with_config(lsk, eop, geo, jd_utc, config, out)
        check(status, "dhruv_mc_deg_with_config")
    return out[0]


def ramc_deg(lsk, eop, location: GeoLocation, jd_utc: float) -> float:
    """Compute RAMC in degrees. No engine needed."""
    geo = _make_geo(location)
    out = ffi.new("double *")
    status = lib.dhruv_ramc_deg(lsk, eop, geo, jd_utc, out)
    check(status, "dhruv_ramc_deg")
    return out[0]


def ramc_deg_utc(lsk, eop, location: GeoLocation, utc: UtcTime) -> float:
    """Compute RAMC in degrees from UTC. No engine needed."""
    geo = _make_geo(location)
    t = _make_utc(utc)
    out = ffi.new("double *")
    status = lib.dhruv_ramc_deg_utc(lsk, eop, geo, t, out)
    check(status, "dhruv_ramc_deg_utc")
    return out[0]


def compute_bhavas_utc(engine, lsk, eop, location: GeoLocation,
                       utc: UtcTime, config=None) -> BhavaResult:
    """Compute 12 bhava cusps with lagna and MC from UTC input.

    Args:
        config: Optional DhruvBhavaConfig pointer. Set output fields to request
            sidereal longitude output.
    """
    geo = _make_geo(location)
    t = _make_utc(utc)
    out = ffi.new("DhruvBhavaResult *")
    cfg = config if config is not None else ffi.NULL
    status = lib.dhruv_compute_bhavas_utc(engine, lsk, eop, geo, t, cfg, out)
    check(status, "dhruv_compute_bhavas_utc")
    bhavas = [
        BhavaEntry(
            number=out.bhavas[i].number,
            cusp_deg=out.bhavas[i].cusp_deg,
            start_deg=out.bhavas[i].start_deg,
            end_deg=out.bhavas[i].end_deg,
        )
        for i in range(12)
    ]
    return BhavaResult(bhavas=bhavas, lagna_deg=out.lagna_deg, mc_deg=out.mc_deg)


def lagna_deg_utc(lsk, eop, location: GeoLocation, utc: UtcTime, config=None) -> float:
    """Compute Ascendant longitude in degrees from UTC.

    Args:
        config: Optional DhruvBhavaConfig pointer for sidereal output.
    """
    geo = _make_geo(location)
    t = _make_utc(utc)
    out = ffi.new("double *")
    if config is None:
        status = lib.dhruv_lagna_deg_utc(lsk, eop, geo, t, out)
        check(status, "dhruv_lagna_deg_utc")
    else:
        status = lib.dhruv_lagna_deg_utc_with_config(lsk, eop, geo, t, config, out)
        check(status, "dhruv_lagna_deg_utc_with_config")
    return out[0]


def mc_deg_utc(lsk, eop, location: GeoLocation, utc: UtcTime, config=None) -> float:
    """Compute MC (Midheaven) longitude in degrees from UTC.

    Args:
        config: Optional DhruvBhavaConfig pointer for sidereal output.
    """
    geo = _make_geo(location)
    t = _make_utc(utc)
    out = ffi.new("double *")
    if config is None:
        status = lib.dhruv_mc_deg_utc(lsk, eop, geo, t, out)
        check(status, "dhruv_mc_deg_utc")
    else:
        status = lib.dhruv_mc_deg_utc_with_config(lsk, eop, geo, t, config, out)
        check(status, "dhruv_mc_deg_utc_with_config")
    return out[0]


# ---------------------------------------------------------------------------
# 3. Rashi / Nakshatra
# ---------------------------------------------------------------------------


def rashi_from_longitude(sidereal_lon_deg: float) -> RashiInfo:
    """Classify sidereal longitude into rashi (pure math)."""
    out = ffi.new("DhruvRashiInfo *")
    status = lib.dhruv_rashi_from_longitude(sidereal_lon_deg, out)
    check(status, "dhruv_rashi_from_longitude")
    return RashiInfo(
        rashi_index=out.rashi_index,
        degrees_in_rashi=out.degrees_in_rashi,
        dms=Dms(
            degrees=out.dms.degrees,
            minutes=out.dms.minutes,
            seconds=out.dms.seconds,
        ),
    )


def nakshatra_from_longitude(sidereal_lon_deg: float) -> NakshatraInfo:
    """Classify sidereal longitude into nakshatra, 27-scheme (pure math)."""
    out = ffi.new("DhruvNakshatraInfo *")
    status = lib.dhruv_nakshatra_from_longitude(sidereal_lon_deg, out)
    check(status, "dhruv_nakshatra_from_longitude")
    return NakshatraInfo(
        nakshatra_index=out.nakshatra_index,
        pada=out.pada,
        degrees_in_nakshatra=out.degrees_in_nakshatra,
        degrees_in_pada=out.degrees_in_pada,
    )


def nakshatra28_from_longitude(sidereal_lon_deg: float) -> Nakshatra28Info:
    """Classify sidereal longitude into nakshatra, 28-scheme (pure math)."""
    out = ffi.new("DhruvNakshatra28Info *")
    status = lib.dhruv_nakshatra28_from_longitude(sidereal_lon_deg, out)
    check(status, "dhruv_nakshatra28_from_longitude")
    return Nakshatra28Info(
        nakshatra_index=out.nakshatra_index,
        pada=out.pada,
        degrees_in_nakshatra=out.degrees_in_nakshatra,
    )


def rashi_from_tropical(tropical_lon: float, aya_system: int,
                        jd_tdb: float, use_nutation: int = 1) -> RashiInfo:
    """Classify tropical longitude into rashi (applies ayanamsha)."""
    out = ffi.new("DhruvRashiInfo *")
    status = lib.dhruv_rashi_from_tropical(tropical_lon, aya_system, jd_tdb,
                                           use_nutation, out)
    check(status, "dhruv_rashi_from_tropical")
    return RashiInfo(
        rashi_index=out.rashi_index,
        degrees_in_rashi=out.degrees_in_rashi,
        dms=Dms(
            degrees=out.dms.degrees,
            minutes=out.dms.minutes,
            seconds=out.dms.seconds,
        ),
    )


def nakshatra_from_tropical(tropical_lon: float, aya_system: int,
                            jd_tdb: float,
                            use_nutation: int = 1) -> NakshatraInfo:
    """Classify tropical longitude into nakshatra (applies ayanamsha)."""
    out = ffi.new("DhruvNakshatraInfo *")
    status = lib.dhruv_nakshatra_from_tropical(tropical_lon, aya_system,
                                               jd_tdb, use_nutation, out)
    check(status, "dhruv_nakshatra_from_tropical")
    return NakshatraInfo(
        nakshatra_index=out.nakshatra_index,
        pada=out.pada,
        degrees_in_nakshatra=out.degrees_in_nakshatra,
        degrees_in_pada=out.degrees_in_pada,
    )


def rashi_from_tropical_utc(lsk, tropical_lon: float, aya_system: int,
                            utc: UtcTime,
                            use_nutation: int = 1) -> RashiInfo:
    """Classify tropical longitude into rashi from UTC (applies ayanamsha)."""
    t = _make_utc(utc)
    out = ffi.new("DhruvRashiInfo *")
    status = lib.dhruv_rashi_from_tropical_utc(lsk, tropical_lon, aya_system,
                                               t, use_nutation, out)
    check(status, "dhruv_rashi_from_tropical_utc")
    return RashiInfo(
        rashi_index=out.rashi_index,
        degrees_in_rashi=out.degrees_in_rashi,
        dms=Dms(
            degrees=out.dms.degrees,
            minutes=out.dms.minutes,
            seconds=out.dms.seconds,
        ),
    )


def nakshatra_from_tropical_utc(lsk, tropical_lon: float, aya_system: int,
                                utc: UtcTime,
                                use_nutation: int = 1) -> NakshatraInfo:
    """Classify tropical longitude into nakshatra from UTC."""
    t = _make_utc(utc)
    out = ffi.new("DhruvNakshatraInfo *")
    status = lib.dhruv_nakshatra_from_tropical_utc(lsk, tropical_lon,
                                                   aya_system, t,
                                                   use_nutation, out)
    check(status, "dhruv_nakshatra_from_tropical_utc")
    return NakshatraInfo(
        nakshatra_index=out.nakshatra_index,
        pada=out.pada,
        degrees_in_nakshatra=out.degrees_in_nakshatra,
        degrees_in_pada=out.degrees_in_pada,
    )


def nakshatra28_from_tropical(tropical_lon: float, aya_system: int,
                               jd_tdb: float,
                               use_nutation: int = 1) -> Nakshatra28Info:
    """Classify tropical longitude into 28-scheme nakshatra (applies ayanamsha)."""
    out = ffi.new("DhruvNakshatra28Info *")
    status = lib.dhruv_nakshatra28_from_tropical(tropical_lon, aya_system,
                                                  jd_tdb, use_nutation, out)
    check(status, "dhruv_nakshatra28_from_tropical")
    return Nakshatra28Info(
        nakshatra_index=out.nakshatra_index,
        pada=out.pada,
        degrees_in_nakshatra=out.degrees_in_nakshatra,
    )


def nakshatra28_from_tropical_utc(lsk, tropical_lon: float, aya_system: int,
                                   utc: UtcTime,
                                   use_nutation: int = 1) -> Nakshatra28Info:
    """Classify tropical longitude into 28-scheme nakshatra from UTC."""
    t = _make_utc(utc)
    out = ffi.new("DhruvNakshatra28Info *")
    status = lib.dhruv_nakshatra28_from_tropical_utc(lsk, tropical_lon,
                                                      aya_system, t,
                                                      use_nutation, out)
    check(status, "dhruv_nakshatra28_from_tropical_utc")
    return Nakshatra28Info(
        nakshatra_index=out.nakshatra_index,
        pada=out.pada,
        degrees_in_nakshatra=out.degrees_in_nakshatra,
    )


def deg_to_dms(degrees: float) -> Dms:
    """Convert decimal degrees to degrees-minutes-seconds."""
    out = ffi.new("DhruvDms *")
    status = lib.dhruv_deg_to_dms(degrees, out)
    check(status, "dhruv_deg_to_dms")
    return Dms(degrees=out.degrees, minutes=out.minutes, seconds=out.seconds)


def rashi_count() -> int:
    """Return number of rashis (12)."""
    return lib.dhruv_rashi_count()


def nakshatra_count(scheme: int = 27) -> int:
    """Return number of nakshatras for scheme (27 or 28)."""
    return lib.dhruv_nakshatra_count(scheme)


def rashi_lord(rashi_index: int) -> int:
    """Return graha index (0-8) of the lord of a rashi. -1 for invalid."""
    return lib.dhruv_rashi_lord(rashi_index)


def nakshatra_at(engine, jd_tdb: float, moon_sidereal_deg: float,
                 config=None) -> PanchangNakshatraInfo:
    """Compute nakshatra with time boundaries using engine.

    Args:
        engine: DhruvEngineHandle pointer.
        jd_tdb: Julian Date in TDB.
        moon_sidereal_deg: Sidereal longitude of the Moon in degrees.
        config: Optional DhruvSankrantiConfig pointer.

    Returns:
        PanchangNakshatraInfo with nakshatra_index, pada, start, end.
    """
    out = ffi.new("DhruvPanchangNakshatraInfo *")
    cfg = config if config is not None else ffi.NULL
    status = lib.dhruv_nakshatra_at(engine, jd_tdb, moon_sidereal_deg,
                                    cfg, out)
    check(status, "dhruv_nakshatra_at")
    return PanchangNakshatraInfo(
        nakshatra_index=out.nakshatra_index,
        pada=out.pada,
        start=_read_utc(out.start),
        end=_read_utc(out.end),
    )


# ---------------------------------------------------------------------------
# 4. Name lookups
# ---------------------------------------------------------------------------


def rashi_name(index: int) -> Optional[str]:
    """Return rashi name by 0-based index, or None."""
    return _str_or_none(lib.dhruv_rashi_name(index))


def nakshatra_name(index: int) -> Optional[str]:
    """Return nakshatra name (27-scheme) by 0-based index, or None."""
    return _str_or_none(lib.dhruv_nakshatra_name(index))


def nakshatra28_name(index: int) -> Optional[str]:
    """Return nakshatra name (28-scheme) by 0-based index, or None."""
    return _str_or_none(lib.dhruv_nakshatra28_name(index))


def graha_name(index: int) -> Optional[str]:
    """Return graha Sanskrit name by index (0-8), or None."""
    return _str_or_none(lib.dhruv_graha_name(index))


def yogini_name(index: int) -> Optional[str]:
    """Return Yogini dasha entity name by index (0-7), or None."""
    return _str_or_none(lib.dhruv_yogini_name(index))


def tithi_name(index: int) -> Optional[str]:
    """Return tithi name by 0-based index (0-29), or None."""
    return _str_or_none(lib.dhruv_tithi_name(index))


def karana_name(index: int) -> Optional[str]:
    """Return karana name by index, or None."""
    return _str_or_none(lib.dhruv_karana_name(index))


def yoga_name(index: int) -> Optional[str]:
    """Return yoga name by 0-based index (0-26), or None."""
    return _str_or_none(lib.dhruv_yoga_name(index))


def vaar_name(index: int) -> Optional[str]:
    """Return vaar (weekday) name by index (0=Sunday..6=Saturday), or None."""
    return _str_or_none(lib.dhruv_vaar_name(index))


def hora_name(index: int) -> Optional[str]:
    """Return hora lord name by Chaldean index, or None."""
    return _str_or_none(lib.dhruv_hora_name(index))


def masa_name(index: int) -> Optional[str]:
    """Return masa (lunar month) name by index (0-11), or None."""
    return _str_or_none(lib.dhruv_masa_name(index))


def ayana_name(index: int) -> Optional[str]:
    """Return ayana name by index (0=Uttarayana, 1=Dakshinayana), or None."""
    return _str_or_none(lib.dhruv_ayana_name(index))


def samvatsara_name(index: int) -> Optional[str]:
    """Return samvatsara name by 0-based index (0-59), or None."""
    return _str_or_none(lib.dhruv_samvatsara_name(index))


def sphuta_name(index: int) -> Optional[str]:
    """Return sphuta name by index (0-15), or None."""
    return _str_or_none(lib.dhruv_sphuta_name(index))


def special_lagna_name(index: int) -> Optional[str]:
    """Return special lagna name by index (0-7), or None."""
    return _str_or_none(lib.dhruv_special_lagna_name(index))


def arudha_pada_name(index: int) -> Optional[str]:
    """Return arudha pada name by index (0-11), or None."""
    return _str_or_none(lib.dhruv_arudha_pada_name(index))


def upagraha_name(index: int) -> Optional[str]:
    """Return upagraha name by index (0-10), or None."""
    return _str_or_none(lib.dhruv_upagraha_name(index))


def nth_rashi_from(rashi_index: int, offset: int) -> int:
    """Compute rashi index that is offset signs from rashi_index. -1 on error."""
    return lib.dhruv_nth_rashi_from(rashi_index, offset)


# ---------------------------------------------------------------------------
# 5. Lunar Nodes
# ---------------------------------------------------------------------------


def lunar_node_deg(node_code: int, mode_code: int,
                   jd_tdb: float) -> float:
    """Compute lunar node longitude (pure math, no engine).

    Args:
        node_code: 0=Rahu, 1=Ketu.
        mode_code: 0=Mean, 1=True (50-term fitted series).
        jd_tdb: Julian Date in TDB.
    """
    out = ffi.new("double *")
    status = lib.dhruv_lunar_node_deg(node_code, mode_code, jd_tdb, out)
    check(status, "dhruv_lunar_node_deg")
    return out[0]


def lunar_node_deg_with_engine(engine, node_code: int, mode_code: int,
                               jd_tdb: float) -> float:
    """Compute lunar node longitude using engine (osculating for true mode)."""
    out = ffi.new("double *")
    status = lib.dhruv_lunar_node_deg_with_engine(engine, node_code,
                                                  mode_code, jd_tdb, out)
    check(status, "dhruv_lunar_node_deg_with_engine")
    return out[0]


def lunar_node_compute_ex(engine, lsk, request) -> float:
    """Unified lunar node computation via DhruvLunarNodeRequest.

    Args:
        engine: DhruvEngineHandle pointer (required for backend=ENGINE).
        lsk: DhruvLskHandle pointer (required for time_kind=UTC).
        request: DhruvLunarNodeRequest pointer.
    """
    out = ffi.new("double *")
    status = lib.dhruv_lunar_node_compute_ex(engine, lsk, request, out)
    check(status, "dhruv_lunar_node_compute_ex")
    return out[0]


def lunar_node_count() -> int:
    """Return number of supported lunar node types (Rahu, Ketu)."""
    return lib.dhruv_lunar_node_count()


def lunar_node_deg_utc(lsk, node_code: int, mode_code: int,
                       utc: UtcTime) -> float:
    """Compute lunar node longitude from UTC (pure math, no engine).

    Args:
        lsk: DhruvLskHandle pointer.
        node_code: 0=Rahu, 1=Ketu.
        mode_code: 0=Mean, 1=True (50-term fitted series).
        utc: UTC time.
    """
    t = _make_utc(utc)
    out = ffi.new("double *")
    status = lib.dhruv_lunar_node_deg_utc(lsk, node_code, mode_code, t, out)
    check(status, "dhruv_lunar_node_deg_utc")
    return out[0]


def lunar_node_deg_utc_with_engine(engine, lsk, node_code: int,
                                   mode_code: int, utc: UtcTime) -> float:
    """Compute lunar node longitude from UTC using engine (osculating for true mode).

    Args:
        engine: DhruvEngineHandle pointer.
        lsk: DhruvLskHandle pointer.
        node_code: 0=Rahu, 1=Ketu.
        mode_code: 0=Mean, 1=True (osculating).
        utc: UTC time.
    """
    t = _make_utc(utc)
    out = ffi.new("double *")
    status = lib.dhruv_lunar_node_deg_utc_with_engine(
        engine, lsk, node_code, mode_code, t, out
    )
    check(status, "dhruv_lunar_node_deg_utc_with_engine")
    return out[0]


# ---------------------------------------------------------------------------
# 6. Sphutas
# ---------------------------------------------------------------------------


def all_sphutas(sun: float, moon: float, mars: float, jupiter: float,
                venus: float, rahu: float, lagna: float,
                eighth_lord: float, gulika: float) -> SphutalResult:
    """Compute all 16 sphutas from sidereal longitudes (pure math).

    Returns SphutalResult with 16 longitudes indexed by sphuta order.
    """
    inp = ffi.new("DhruvSphutalInputs *")
    inp.sun = sun
    inp.moon = moon
    inp.mars = mars
    inp.jupiter = jupiter
    inp.venus = venus
    inp.rahu = rahu
    inp.lagna = lagna
    inp.eighth_lord = eighth_lord
    inp.gulika = gulika
    out = ffi.new("DhruvSphutalResult *")
    status = lib.dhruv_all_sphutas(inp, out)
    check(status, "dhruv_all_sphutas")
    return SphutalResult(longitudes=[out.longitudes[i] for i in range(16)])


# --- Individual sphuta functions (pure math) ---


def bhrigu_bindu(rahu: float, moon: float) -> float:
    """Bhrigu Bindu = midpoint of Rahu and Moon (pure math)."""
    return lib.dhruv_bhrigu_bindu(rahu, moon)


def prana_sphuta(lagna: float, moon: float) -> float:
    """Prana Sphuta (pure math)."""
    return lib.dhruv_prana_sphuta(lagna, moon)


def deha_sphuta(moon: float, lagna: float) -> float:
    """Deha Sphuta (pure math)."""
    return lib.dhruv_deha_sphuta(moon, lagna)


def mrityu_sphuta(eighth_lord: float, lagna: float) -> float:
    """Mrityu Sphuta (pure math)."""
    return lib.dhruv_mrityu_sphuta(eighth_lord, lagna)


def tithi_sphuta(moon: float, sun: float, lagna: float) -> float:
    """Tithi Sphuta (pure math)."""
    return lib.dhruv_tithi_sphuta(moon, sun, lagna)


def yoga_sphuta(sun: float, moon: float) -> float:
    """Yoga Sphuta = Sun + Moon (raw, may exceed 360, pure math)."""
    return lib.dhruv_yoga_sphuta(sun, moon)


def yoga_sphuta_normalized(sun: float, moon: float) -> float:
    """Yoga Sphuta normalized to [0, 360) (pure math)."""
    return lib.dhruv_yoga_sphuta_normalized(sun, moon)


def rahu_tithi_sphuta(rahu: float, sun: float, lagna: float) -> float:
    """Rahu Tithi Sphuta (pure math)."""
    return lib.dhruv_rahu_tithi_sphuta(rahu, sun, lagna)


def kshetra_sphuta(moon: float, mars: float, jupiter: float,
                   venus: float, lagna: float) -> float:
    """Kshetra Sphuta (pure math)."""
    # Keep Python API argument order stable while mapping to C ABI order.
    return lib.dhruv_kshetra_sphuta(venus, moon, mars, jupiter, lagna)


def beeja_sphuta(sun: float, venus: float, jupiter: float) -> float:
    """Beeja Sphuta (pure math)."""
    return lib.dhruv_beeja_sphuta(sun, venus, jupiter)


def trisphuta(lagna: float, moon: float, gulika: float) -> float:
    """Trisphuta (pure math)."""
    return lib.dhruv_trisphuta(lagna, moon, gulika)


def chatussphuta(trisphuta_val: float, sun: float) -> float:
    """Chatussphuta (pure math)."""
    return lib.dhruv_chatussphuta(trisphuta_val, sun)


def panchasphuta(chatussphuta_val: float, rahu: float) -> float:
    """Panchasphuta (pure math)."""
    return lib.dhruv_panchasphuta(chatussphuta_val, rahu)


def sookshma_trisphuta(lagna: float, moon: float,
                       gulika: float, sun: float) -> float:
    """Sookshma Trisphuta (pure math)."""
    return lib.dhruv_sookshma_trisphuta(lagna, moon, gulika, sun)


def avayoga_sphuta(sun: float, moon: float) -> float:
    """Avayoga Sphuta (pure math)."""
    return lib.dhruv_avayoga_sphuta(sun, moon)


def kunda(lagna: float, moon: float, mars: float) -> float:
    """Kunda Sphuta (pure math)."""
    return lib.dhruv_kunda(lagna, moon, mars)


# ---------------------------------------------------------------------------
# 7. Special Lagnas
# ---------------------------------------------------------------------------


def special_lagnas_for_date(engine, eop, utc: UtcTime,
                           location: GeoLocation,
                           ayanamsha_system: int = 0,
                           use_nutation: int = 1,
                           riseset_config=None) -> SpecialLagnas:
    """Compute all 8 special lagnas for a date and location."""
    t = _make_utc(utc)
    geo = _make_geo(location)
    out = ffi.new("DhruvSpecialLagnas *")
    cfg = riseset_config if riseset_config is not None else ffi.NULL
    status = lib.dhruv_special_lagnas_for_date(
        engine, eop, t, geo, cfg, ayanamsha_system, use_nutation, out
    )
    check(status, "dhruv_special_lagnas_for_date")
    return SpecialLagnas(
        bhava_lagna=out.bhava_lagna,
        hora_lagna=out.hora_lagna,
        ghati_lagna=out.ghati_lagna,
        vighati_lagna=out.vighati_lagna,
        varnada_lagna=out.varnada_lagna,
        sree_lagna=out.sree_lagna,
        pranapada_lagna=out.pranapada_lagna,
        indu_lagna=out.indu_lagna,
    )


# --- Individual special lagna functions (pure math) ---


def bhava_lagna(sun_lon: float, ghatikas: float) -> float:
    """Bhava Lagna from Sun longitude and ghatikas since sunrise (pure math)."""
    return lib.dhruv_bhava_lagna(sun_lon, ghatikas)


def hora_lagna(sun_lon: float, ghatikas: float) -> float:
    """Hora Lagna from Sun longitude and ghatikas since sunrise (pure math)."""
    return lib.dhruv_hora_lagna(sun_lon, ghatikas)


def ghati_lagna(sun_lon: float, ghatikas: float) -> float:
    """Ghati Lagna from Sun longitude and ghatikas since sunrise (pure math)."""
    return lib.dhruv_ghati_lagna(sun_lon, ghatikas)


def vighati_lagna(lagna_lon: float, vighatikas: float) -> float:
    """Vighati Lagna from lagna longitude and vighatikas (pure math)."""
    return lib.dhruv_vighati_lagna(lagna_lon, vighatikas)


def varnada_lagna(lagna_lon: float, hora_lagna_lon: float) -> float:
    """Varnada Lagna from lagna and hora lagna longitudes (pure math)."""
    return lib.dhruv_varnada_lagna(lagna_lon, hora_lagna_lon)


def sree_lagna(moon_lon: float, lagna_lon: float) -> float:
    """Sree Lagna from Moon and lagna longitudes (pure math)."""
    return lib.dhruv_sree_lagna(moon_lon, lagna_lon)


def pranapada_lagna(sun_lon: float, ghatikas: float) -> float:
    """Pranapada Lagna from Sun longitude and ghatikas (pure math)."""
    return lib.dhruv_pranapada_lagna(sun_lon, ghatikas)


def indu_lagna(moon_lon: float, lagna_lord: int, moon_9th_lord: int) -> float:
    """Indu Lagna from Moon longitude and lord indices (pure math).

    Args:
        moon_lon: Moon's sidereal longitude in degrees.
        lagna_lord: Graha index of the lagna lord.
        moon_9th_lord: Graha index of the 9th-from-Moon lord.
    """
    return lib.dhruv_indu_lagna(moon_lon, lagna_lord, moon_9th_lord)


# ---------------------------------------------------------------------------
# 8. Arudha Padas
# ---------------------------------------------------------------------------


def arudha_pada(bhava_cusp_lon: float,
                lord_lon: float) -> tuple[float, int]:
    """Compute single arudha pada (pure math).

    Returns (longitude_deg, rashi_index).
    """
    out_rashi = ffi.new("uint8_t *")
    lon = lib.dhruv_arudha_pada(bhava_cusp_lon, lord_lon, out_rashi)
    return (lon, out_rashi[0])


def arudha_padas_for_date(engine, eop, utc: UtcTime,
                          location: GeoLocation,
                          ayanamsha_system: int = 0,
                          use_nutation: int = 1) -> list[ArudhaResult]:
    """Compute all 12 arudha padas for a date and location."""
    t = _make_utc(utc)
    geo = _make_geo(location)
    out = ffi.new("DhruvArudhaResult[12]")
    status = lib.dhruv_arudha_padas_for_date(
        engine, eop, t, geo, ayanamsha_system, use_nutation, out
    )
    check(status, "dhruv_arudha_padas_for_date")
    return [
        ArudhaResult(
            bhava_number=out[i].bhava_number,
            longitude_deg=out[i].longitude_deg,
            rashi_index=out[i].rashi_index,
        )
        for i in range(12)
    ]


# ---------------------------------------------------------------------------
# 9. Upagrahas
# ---------------------------------------------------------------------------


def sun_based_upagrahas(sun_sid_lon: float) -> AllUpagrahas:
    """Compute the 5 sun-based upagrahas from sidereal Sun longitude (pure math).

    Only dhooma, vyatipata, parivesha, indra_chapa, upaketu are valid.
    Time-based fields (gulika..yama_ghantaka) are uninitialized.
    """
    out = ffi.new("DhruvAllUpagrahas *")
    status = lib.dhruv_sun_based_upagrahas(sun_sid_lon, out)
    check(status, "dhruv_sun_based_upagrahas")
    return AllUpagrahas(
        gulika=out.gulika,
        maandi=out.maandi,
        kaala=out.kaala,
        mrityu=out.mrityu,
        artha_prahara=out.artha_prahara,
        yama_ghantaka=out.yama_ghantaka,
        dhooma=out.dhooma,
        vyatipata=out.vyatipata,
        parivesha=out.parivesha,
        indra_chapa=out.indra_chapa,
        upaketu=out.upaketu,
    )


def time_upagraha_config_default():
    """Return the default DhruvTimeUpagrahaConfig (FFI struct, by value)."""
    return lib.dhruv_time_upagraha_config_default()


def time_upagraha_jd(index: int, weekday: int, is_day: int,
                     sunrise_jd: float, sunset_jd: float,
                     next_sunrise_jd: float, upagraha_config=None) -> float:
    """Compute JD for a time-based upagraha (pure math).

    Args:
        index: 0=Gulika, 1=Maandi, 2=Kaala, 3=Mrityu,
               4=ArthaPrahara, 5=YamaGhantaka.
        weekday: 0=Sunday .. 6=Saturday.
        is_day: 1=daytime, 0=nighttime.
        upagraha_config: Optional DhruvTimeUpagrahaConfig or dict with
            gulika_point, maandi_point, other_point, gulika_planet,
            maandi_planet. Point values accept start/middle/end; planet values
            accept rahu/saturn.
    """
    out = ffi.new("double *")
    cfg = _make_time_upagraha_config(upagraha_config)
    if cfg == ffi.NULL:
        status = lib.dhruv_time_upagraha_jd(index, weekday, is_day,
                                            sunrise_jd, sunset_jd,
                                            next_sunrise_jd, out)
        check(status, "dhruv_time_upagraha_jd")
    else:
        status = lib.dhruv_time_upagraha_jd_with_config(
            index, weekday, is_day, sunrise_jd, sunset_jd, next_sunrise_jd, cfg, out
        )
        check(status, "dhruv_time_upagraha_jd_with_config")
    return out[0]


def time_upagraha_jd_utc(engine, eop, utc: UtcTime,
                         location: GeoLocation,
                         index: int,
                         riseset_config=None,
                         upagraha_config=None) -> float:
    """Compute JD for a time-based upagraha from UTC date and location."""
    t = _make_utc(utc)
    geo = _make_geo(location)
    out = ffi.new("double *")
    cfg = riseset_config if riseset_config is not None else ffi.NULL
    upa_cfg = _make_time_upagraha_config(upagraha_config)
    if upa_cfg == ffi.NULL:
        status = lib.dhruv_time_upagraha_jd_utc(engine, eop, t, geo, cfg,
                                                index, out)
        check(status, "dhruv_time_upagraha_jd_utc")
    else:
        status = lib.dhruv_time_upagraha_jd_utc_with_config(
            engine, eop, t, geo, cfg, index, upa_cfg, out
        )
        check(status, "dhruv_time_upagraha_jd_utc_with_config")
    return out[0]


def all_upagrahas_for_date(engine, eop, utc: UtcTime,
                           location: GeoLocation,
                           ayanamsha_system: int = 0,
                           use_nutation: int = 1,
                           upagraha_config=None) -> AllUpagrahas:
    """Compute all 11 upagrahas for a date and location."""
    t = _make_utc(utc)
    geo = _make_geo(location)
    out = ffi.new("DhruvAllUpagrahas *")
    cfg = _make_time_upagraha_config(upagraha_config)
    if cfg == ffi.NULL:
        status = lib.dhruv_all_upagrahas_for_date(
            engine, eop, t, geo, ayanamsha_system, use_nutation, out
        )
        check(status, "dhruv_all_upagrahas_for_date")
    else:
        status = lib.dhruv_all_upagrahas_for_date_with_config(
            engine, eop, t, geo, ayanamsha_system, use_nutation, cfg, out
        )
        check(status, "dhruv_all_upagrahas_for_date_with_config")
    return AllUpagrahas(
        gulika=out.gulika,
        maandi=out.maandi,
        kaala=out.kaala,
        mrityu=out.mrityu,
        artha_prahara=out.artha_prahara,
        yama_ghantaka=out.yama_ghantaka,
        dhooma=out.dhooma,
        vyatipata=out.vyatipata,
        parivesha=out.parivesha,
        indra_chapa=out.indra_chapa,
        upaketu=out.upaketu,
    )


# ---------------------------------------------------------------------------
# 10. Ashtakavarga
# ---------------------------------------------------------------------------


def calculate_bav(graha_index: int, graha_rashis: list[int],
                  lagna_rashi: int) -> BhinnaAshtakavarga:
    """Calculate BAV for a single graha (pure math).

    Args:
        graha_index: 0=Sun through 6=Saturn.
        graha_rashis: 7 rashi indices (0-based) for Sun..Saturn.
        lagna_rashi: 0-based rashi of Ascendant.
    """
    rashis_buf = ffi.new("uint8_t[7]", graha_rashis)
    out = ffi.new("DhruvBhinnaAshtakavarga *")
    status = lib.dhruv_calculate_bav(graha_index, rashis_buf, lagna_rashi, out)
    check(status, "dhruv_calculate_bav")
    return BhinnaAshtakavarga(
        graha_index=out.graha_index,
        points=[out.points[i] for i in range(12)],
        contributors=[[out.contributors[i][j] for j in range(8)] for i in range(12)],
    )


def calculate_all_bav(graha_rashis: list[int],
                      lagna_rashi: int) -> list[BhinnaAshtakavarga]:
    """Calculate BAV for all 7 grahas (pure math)."""
    rashis_buf = ffi.new("uint8_t[7]", graha_rashis)
    out = ffi.new("DhruvBhinnaAshtakavarga[7]")
    status = lib.dhruv_calculate_all_bav(rashis_buf, lagna_rashi, out)
    check(status, "dhruv_calculate_all_bav")
    return [
        BhinnaAshtakavarga(
            graha_index=out[i].graha_index,
            points=[out[i].points[j] for j in range(12)],
            contributors=[[out[i].contributors[r][c] for c in range(8)] for r in range(12)],
        )
        for i in range(7)
    ]


def calculate_sav(bavs: list[BhinnaAshtakavarga]) -> SarvaAshtakavarga:
    """Calculate SAV from 7 BAVs (pure math)."""
    bav_buf = ffi.new("DhruvBhinnaAshtakavarga[7]")
    for i, bav in enumerate(bavs):
        bav_buf[i].graha_index = bav.graha_index
        for j in range(12):
            bav_buf[i].points[j] = bav.points[j]
            for k in range(8):
                bav_buf[i].contributors[j][k] = bav.contributors[j][k]
    out = ffi.new("DhruvSarvaAshtakavarga *")
    status = lib.dhruv_calculate_sav(bav_buf, out)
    check(status, "dhruv_calculate_sav")
    return SarvaAshtakavarga(
        total_points=[out.total_points[i] for i in range(12)],
        after_trikona=[out.after_trikona[i] for i in range(12)],
        after_ekadhipatya=[out.after_ekadhipatya[i] for i in range(12)],
    )


def calculate_ashtakavarga(graha_rashis: list[int],
                           lagna_rashi: int) -> AshtakavargaResult:
    """Calculate all BAVs and SAV in one call (pure math)."""
    rashis_buf = ffi.new("uint8_t[7]", graha_rashis)
    out = ffi.new("DhruvAshtakavargaResult *")
    status = lib.dhruv_calculate_ashtakavarga(rashis_buf, lagna_rashi, out)
    check(status, "dhruv_calculate_ashtakavarga")
    bavs = [
        BhinnaAshtakavarga(
            graha_index=out.bavs[i].graha_index,
            points=[out.bavs[i].points[j] for j in range(12)],
            contributors=[[out.bavs[i].contributors[r][c] for c in range(8)] for r in range(12)],
        )
        for i in range(7)
    ]
    sav = SarvaAshtakavarga(
        total_points=[out.sav.total_points[i] for i in range(12)],
        after_trikona=[out.sav.after_trikona[i] for i in range(12)],
        after_ekadhipatya=[out.sav.after_ekadhipatya[i] for i in range(12)],
    )
    return AshtakavargaResult(bavs=bavs, sav=sav)


def trikona_sodhana(totals: list[int]) -> list[int]:
    """Apply Trikona Sodhana to 12 rashi totals (pure math)."""
    src = ffi.new("uint8_t[12]", totals)
    dst = ffi.new("uint8_t[12]")
    status = lib.dhruv_trikona_sodhana(src, dst)
    check(status, "dhruv_trikona_sodhana")
    return [dst[i] for i in range(12)]


def ekadhipatya_sodhana(after_trikona: list[int]) -> list[int]:
    """Apply Ekadhipatya Sodhana to 12 rashi totals (pure math)."""
    src = ffi.new("uint8_t[12]", after_trikona)
    dst = ffi.new("uint8_t[12]")
    status = lib.dhruv_ekadhipatya_sodhana(src, dst)
    check(status, "dhruv_ekadhipatya_sodhana")
    return [dst[i] for i in range(12)]


def ashtakavarga_for_date(engine, eop, utc: UtcTime,
                          location: GeoLocation,
                          ayanamsha_system: int = 0,
                          use_nutation: int = 1) -> AshtakavargaResult:
    """Compute complete ashtakavarga for a date (orchestration)."""
    t = _make_utc(utc)
    geo = _make_geo(location)
    out = ffi.new("DhruvAshtakavargaResult *")
    status = lib.dhruv_ashtakavarga_for_date(
        engine, eop, t, geo, ayanamsha_system, use_nutation, out
    )
    check(status, "dhruv_ashtakavarga_for_date")
    bavs = [
        BhinnaAshtakavarga(
            graha_index=out.bavs[i].graha_index,
            points=[out.bavs[i].points[j] for j in range(12)],
            contributors=[[out.bavs[i].contributors[r][c] for c in range(8)] for r in range(12)],
        )
        for i in range(7)
    ]
    sav = SarvaAshtakavarga(
        total_points=[out.sav.total_points[i] for i in range(12)],
        after_trikona=[out.sav.after_trikona[i] for i in range(12)],
        after_ekadhipatya=[out.sav.after_ekadhipatya[i] for i in range(12)],
    )
    return AshtakavargaResult(bavs=bavs, sav=sav)


# ---------------------------------------------------------------------------
# 11. Drishti (Planetary Aspects)
# ---------------------------------------------------------------------------


def graha_drishti(graha_index: int, source_lon: float,
                  target_lon: float) -> DrishtiEntry:
    """Compute drishti from a single graha to a single sidereal point (pure math).

    Args:
        graha_index: 0=Surya .. 8=Ketu.
        source_lon: sidereal longitude of source (degrees).
        target_lon: sidereal longitude of target (degrees).
    """
    out = ffi.new("DhruvDrishtiEntry *")
    status = lib.dhruv_graha_drishti(graha_index, source_lon, target_lon, out)
    check(status, "dhruv_graha_drishti")
    return _drishti_entry(out)


def graha_drishti_matrix(longitudes_9: list[float]) -> GrahaDrishtiMatrix:
    """Compute full 9x9 graha drishti matrix from sidereal longitudes (pure math).

    Args:
        longitudes_9: 9 sidereal longitudes [Sun..Ketu] in degrees.
    """
    lons = ffi.new("double[9]", longitudes_9)
    out = ffi.new("DhruvGrahaDrishtiMatrix *")
    status = lib.dhruv_graha_drishti_matrix(lons, out)
    check(status, "dhruv_graha_drishti_matrix")
    matrix = [
        [_drishti_entry(out.entries[i][j]) for j in range(9)]
        for i in range(9)
    ]
    return GrahaDrishtiMatrix(matrix=matrix)


def drishti_for_date(engine, eop, utc: UtcTime, location: GeoLocation,
                     ayanamsha_system: int = 0, use_nutation: int = 1,
                     bhava_config=None, riseset_config=None,
                     include_bhava: bool = True,
                     include_lagna: bool = True,
                     include_bindus: bool = True) -> DrishtiResult:
    """Compute full drishti for a date (orchestration).

    Returns DrishtiResult with graha_to_graha (9x9), graha_to_bhava (9x12),
    graha_to_lagna (9), and graha_to_bindus (9x19).

    Args:
        engine: DhruvEngineHandle pointer.
        eop: DhruvEopHandle pointer.
        utc: Date/time.
        location: Observer location.
        ayanamsha_system: AyanamshaSystem code.
        use_nutation: 1=apply nutation, 0=skip.
        bhava_config: Optional DhruvBhavaConfig pointer.
        riseset_config: Optional DhruvRiseSetConfig pointer.
        include_bhava: Compute graha-to-bhava drishti.
        include_lagna: Compute graha-to-lagna drishti.
        include_bindus: Compute graha-to-core-bindus drishti.
    """
    t = _make_utc(utc)
    geo = _make_geo(location)
    bcfg = bhava_config if bhava_config is not None else ffi.NULL
    rcfg = riseset_config if riseset_config is not None else ffi.NULL
    dcfg = ffi.new("DhruvDrishtiConfig *")
    dcfg.include_bhava = 1 if include_bhava else 0
    dcfg.include_lagna = 1 if include_lagna else 0
    dcfg.include_bindus = 1 if include_bindus else 0
    out = ffi.new("DhruvDrishtiResult *")
    status = lib.dhruv_drishti(engine, eop, t, geo, bcfg, rcfg,
                               ayanamsha_system, use_nutation, dcfg, out)
    check(status, "dhruv_drishti")
    g2g = [
        [_drishti_entry(out.graha_to_graha[i][j]) for j in range(9)]
        for i in range(9)
    ]
    g2b = [
        [_drishti_entry(out.graha_to_bhava[i][j]) for j in range(12)]
        for i in range(9)
    ]
    g2rb = [
        [_drishti_entry(out.graha_to_rashi_bhava[i][j]) for j in range(12)]
        for i in range(9)
    ]
    g2l = [_drishti_entry(out.graha_to_lagna[i]) for i in range(9)]
    g2bindus = [
        [_drishti_entry(out.graha_to_bindus[i][j]) for j in range(19)]
        for i in range(9)
    ]
    return DrishtiResult(
        graha_to_graha=g2g,
        graha_to_bhava=g2b,
        graha_to_rashi_bhava=g2rb,
        graha_to_lagna=g2l,
        graha_to_bindus=g2bindus,
    )


# ---------------------------------------------------------------------------
# 12. Pure-math classifiers
# ---------------------------------------------------------------------------


def tithi_from_elongation(elongation_deg: float) -> TithiPosition:
    """Determine Tithi from Moon-Sun elongation (pure math)."""
    out = ffi.new("DhruvTithiPosition *")
    status = lib.dhruv_tithi_from_elongation(elongation_deg, out)
    check(status, "dhruv_tithi_from_elongation")
    return TithiPosition(
        tithi_index=out.tithi_index,
        paksha=out.paksha,
        tithi_in_paksha=out.tithi_in_paksha,
        degrees_in_tithi=out.degrees_in_tithi,
    )


def karana_from_elongation(elongation_deg: float) -> KaranaPosition:
    """Determine Karana from Moon-Sun elongation (pure math)."""
    out = ffi.new("DhruvKaranaPosition *")
    status = lib.dhruv_karana_from_elongation(elongation_deg, out)
    check(status, "dhruv_karana_from_elongation")
    return KaranaPosition(
        karana_index=out.karana_index,
        degrees_in_karana=out.degrees_in_karana,
    )


def yoga_from_sum(sum_deg: float) -> YogaPosition:
    """Determine Yoga from Sun+Moon sidereal longitude sum (pure math)."""
    out = ffi.new("DhruvYogaPosition *")
    status = lib.dhruv_yoga_from_sum(sum_deg, out)
    check(status, "dhruv_yoga_from_sum")
    return YogaPosition(
        yoga_index=out.yoga_index,
        degrees_in_yoga=out.degrees_in_yoga,
    )


def vaar_from_jd(jd: float) -> int:
    """Determine Vaar (weekday) from JD. Returns 0=Sunday..6=Saturday."""
    return lib.dhruv_vaar_from_jd(jd)


def masa_from_rashi_index(rashi_index: int) -> int:
    """Determine Masa from 0-based rashi index. Returns 0=Chaitra..11, or -1."""
    return lib.dhruv_masa_from_rashi_index(rashi_index)


def ayana_from_sidereal_longitude(lon_deg: float) -> int:
    """Determine Ayana from sidereal longitude. 0=Uttarayana, 1=Dakshinayana."""
    return lib.dhruv_ayana_from_sidereal_longitude(lon_deg)


def samvatsara_from_year(ce_year: int) -> SamvatsaraResult:
    """Determine Samvatsara (Jovian year) from a CE year."""
    out = ffi.new("DhruvSamvatsaraResult *")
    status = lib.dhruv_samvatsara_from_year(ce_year, out)
    check(status, "dhruv_samvatsara_from_year")
    return SamvatsaraResult(
        samvatsara_index=out.samvatsara_index,
        cycle_position=out.cycle_position,
    )


def ghatika_from_elapsed(seconds: float,
                         vedic_day_seconds: float) -> tuple[int, int]:
    """Determine ghatika from elapsed seconds since sunrise (pure math).

    Returns (value 1-60, index 0-59).
    """
    val = ffi.new("uint8_t *")
    idx = ffi.new("uint8_t *")
    status = lib.dhruv_ghatika_from_elapsed(seconds, vedic_day_seconds,
                                            val, idx)
    check(status, "dhruv_ghatika_from_elapsed")
    return (val[0], idx[0])


def ghatikas_since_sunrise(jd_moment: float, jd_sunrise: float,
                           jd_next_sunrise: float) -> float:
    """Compute fractional ghatikas elapsed since sunrise (pure math)."""
    out = ffi.new("double *")
    status = lib.dhruv_ghatikas_since_sunrise(jd_moment, jd_sunrise,
                                             jd_next_sunrise, out)
    check(status, "dhruv_ghatikas_since_sunrise")
    return out[0]


def hora_at(vaar_index: int, hora_index: int) -> int:
    """Determine hora lord for a weekday and position. Returns Chaldean index or -1."""
    return lib.dhruv_hora_at(vaar_index, hora_index)


_NAISARGIKA_LABELS = {
    0: "friend",
    1: "enemy",
    2: "neutral",
}

_TATKALIKA_LABELS = {
    0: "friend",
    1: "enemy",
}

_PANCHADHA_LABELS = {
    0: "adhi_shatru",
    1: "shatru",
    2: "sama",
    3: "mitra",
    4: "adhi_mitra",
}

_DIGNITY_LABELS = {
    0: "exalted",
    1: "moolatrikone",
    2: "own_sign",
    3: "adhi_mitra",
    4: "mitra",
    5: "sama",
    6: "shatru",
    7: "adhi_shatru",
    8: "debilitated",
}

_BENEFIC_LABELS = {
    0: "benefic",
    1: "malefic",
}

_GENDER_LABELS = {
    0: "male",
    1: "female",
    2: "neuter",
}


def _label(code: int, mapping: dict[int, str], op_name: str) -> str:
    if code not in mapping:
        raise ValueError(f"{op_name} returned unknown code {code}")
    return mapping[code]


def _optional_degree(status_name: str, fn, *args):
    has_value = ffi.new("uint8_t *")
    value = ffi.new("double *")
    check(fn(*args, has_value, value), status_name)
    return value[0] if has_value[0] else None


def exaltation_degree(graha_index: int):
    """Return the exaltation degree for a graha, or ``None`` when undefined."""
    return _optional_degree("dhruv_exaltation_degree", lib.dhruv_exaltation_degree, graha_index)


def debilitation_degree(graha_index: int):
    """Return the debilitation degree for a graha, or ``None`` when undefined."""
    return _optional_degree("dhruv_debilitation_degree", lib.dhruv_debilitation_degree, graha_index)


def moolatrikone_range(graha_index: int):
    """Return ``(rashi_index, start_deg, end_deg)`` or ``None`` when undefined."""
    has_value = ffi.new("uint8_t *")
    rashi_index = ffi.new("uint8_t *")
    start_deg = ffi.new("double *")
    end_deg = ffi.new("double *")
    check(
        lib.dhruv_moolatrikone_range(
            graha_index, has_value, rashi_index, start_deg, end_deg
        ),
        "dhruv_moolatrikone_range",
    )
    if not has_value[0]:
        return None
    return (int(rashi_index[0]), float(start_deg[0]), float(end_deg[0]))


def combustion_threshold(graha_index: int, is_retrograde: bool = False):
    """Return the combustion threshold in degrees, or ``None`` when undefined."""
    return _optional_degree(
        "dhruv_combustion_threshold",
        lib.dhruv_combustion_threshold,
        graha_index,
        1 if is_retrograde else 0,
    )


def is_combust(
    graha_index: int,
    graha_sid_lon: float,
    sun_sid_lon: float,
    is_retrograde: bool = False,
) -> bool:
    """Return whether the graha is combust for the given longitudes."""
    out = ffi.new("uint8_t *")
    check(
        lib.dhruv_is_combust(
            graha_index,
            graha_sid_lon,
            sun_sid_lon,
            1 if is_retrograde else 0,
            out,
        ),
        "dhruv_is_combust",
    )
    return bool(out[0])


def all_combustion_status(sidereal_lons_9: list[float], retrograde_flags_9: list[bool]) -> list[bool]:
    """Return combustion flags for the 9 grahas."""
    if len(sidereal_lons_9) != 9 or len(retrograde_flags_9) != 9:
        raise ValueError("expected 9 longitudes and 9 retrograde flags")
    lon_buf = ffi.new("double[9]", sidereal_lons_9)
    retro_buf = ffi.new("uint8_t[9]", [1 if value else 0 for value in retrograde_flags_9])
    out = ffi.new("uint8_t[9]")
    check(
        lib.dhruv_all_combustion_status(lon_buf, retro_buf, out),
        "dhruv_all_combustion_status",
    )
    return [bool(out[i]) for i in range(9)]


def naisargika_maitri(graha_index: int, other_index: int) -> str:
    """Return the natural relationship label for two grahas."""
    out = ffi.new("int32_t *")
    check(lib.dhruv_naisargika_maitri(graha_index, other_index, out), "dhruv_naisargika_maitri")
    return _label(out[0], _NAISARGIKA_LABELS, "dhruv_naisargika_maitri")


def tatkalika_maitri(graha_rashi_index: int, other_rashi_index: int) -> str:
    """Return the temporal relationship label for two rashi positions."""
    out = ffi.new("int32_t *")
    check(lib.dhruv_tatkalika_maitri(graha_rashi_index, other_rashi_index, out), "dhruv_tatkalika_maitri")
    return _label(out[0], _TATKALIKA_LABELS, "dhruv_tatkalika_maitri")


def panchadha_maitri(naisargika_code: int, tatkalika_code: int) -> str:
    """Combine natural and temporal relationship codes into the five-fold relationship."""
    out = ffi.new("int32_t *")
    check(lib.dhruv_panchadha_maitri(naisargika_code, tatkalika_code, out), "dhruv_panchadha_maitri")
    return _label(out[0], _PANCHADHA_LABELS, "dhruv_panchadha_maitri")


def dignity_in_rashi(graha_index: int, sidereal_lon: float, rashi_index: int) -> str:
    """Return the dignity label in the target rashi without temporal context."""
    out = ffi.new("int32_t *")
    check(lib.dhruv_dignity_in_rashi(graha_index, sidereal_lon, rashi_index, out), "dhruv_dignity_in_rashi")
    return _label(out[0], _DIGNITY_LABELS, "dhruv_dignity_in_rashi")


def dignity_in_rashi_with_positions(
    graha_index: int, sidereal_lon: float, rashi_index: int, sapta_rashi_indices_7: list[int]
) -> str:
    """Return the dignity label using sapta-graha positions for compound friendship."""
    if len(sapta_rashi_indices_7) != 7:
        raise ValueError("expected 7 sapta-graha rashi indices")
    positions = ffi.new("uint8_t[7]", sapta_rashi_indices_7)
    out = ffi.new("int32_t *")
    check(
        lib.dhruv_dignity_in_rashi_with_positions(
            graha_index, sidereal_lon, rashi_index, positions, out
        ),
        "dhruv_dignity_in_rashi_with_positions",
    )
    return _label(out[0], _DIGNITY_LABELS, "dhruv_dignity_in_rashi_with_positions")


def node_dignity_in_rashi(
    graha_index: int, rashi_index: int, graha_rashi_indices_9: list[int], policy_code: int = 0
) -> str:
    """Return the node dignity label for Rahu/Ketu using the selected policy code."""
    if len(graha_rashi_indices_9) != 9:
        raise ValueError("expected 9 graha rashi indices")
    positions = ffi.new("uint8_t[9]", graha_rashi_indices_9)
    out = ffi.new("int32_t *")
    check(
        lib.dhruv_node_dignity_in_rashi(
            graha_index, rashi_index, positions, policy_code, out
        ),
        "dhruv_node_dignity_in_rashi",
    )
    return _label(out[0], _DIGNITY_LABELS, "dhruv_node_dignity_in_rashi")


def natural_benefic_malefic(graha_index: int) -> str:
    """Return whether the graha is naturally benefic or malefic."""
    out = ffi.new("int32_t *")
    check(lib.dhruv_natural_benefic_malefic(graha_index, out), "dhruv_natural_benefic_malefic")
    return _label(out[0], _BENEFIC_LABELS, "dhruv_natural_benefic_malefic")


def moon_benefic_nature(moon_sun_elongation: float) -> str:
    """Return the Moon's benefic/malefic nature from elongation."""
    out = ffi.new("int32_t *")
    check(lib.dhruv_moon_benefic_nature(moon_sun_elongation, out), "dhruv_moon_benefic_nature")
    return _label(out[0], _BENEFIC_LABELS, "dhruv_moon_benefic_nature")


def graha_gender(graha_index: int) -> str:
    """Return the graha gender label."""
    out = ffi.new("int32_t *")
    check(lib.dhruv_graha_gender(graha_index, out), "dhruv_graha_gender")
    return _label(out[0], _GENDER_LABELS, "dhruv_graha_gender")


def hora_lord(vaar_index: int, hora_index: int) -> int:
    """Return the graha index of the hora lord, or ``-1`` for invalid input."""
    return lib.dhruv_hora_lord(vaar_index, hora_index)


def masa_lord(masa_index: int) -> int:
    """Return the graha index of the masa lord, or ``-1`` for invalid input."""
    return lib.dhruv_masa_lord(masa_index)


def samvatsara_lord(samvatsara_index: int) -> int:
    """Return the graha index of the samvatsara lord, or ``-1`` for invalid input."""
    return lib.dhruv_samvatsara_lord(samvatsara_index)
