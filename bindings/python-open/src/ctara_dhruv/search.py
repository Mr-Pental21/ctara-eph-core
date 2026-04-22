"""Unified search APIs for conjunction, eclipse, motion, lunar phase, and sankranti.

All functions use the unified ``*_search_ex`` FFI entrypoints introduced in ABI v42.
"""

from __future__ import annotations

from typing import Optional

from ._ffi import ffi, lib
from ._check import check
from .types import (
    ConjunctionEvent,
    ChandraGrahanResult,
    SuryaGrahanResult,
    StationaryEvent,
    MaxSpeedEvent,
    LunarPhaseEvent,
    SankrantiEvent,
    UtcTime,
)

_SEARCH_TIME_JD_TDB = 0
_SEARCH_TIME_UTC = 1
_JD_ABSENT = -1.0


# ---------------------------------------------------------------------------
# Internal helpers
# ---------------------------------------------------------------------------


def _normalize_search_capacity(capacity: int) -> int:
    return max(1, int(capacity))


def _collect_full_range(fetch, initial_capacity: int):
    capacity = _normalize_search_capacity(initial_capacity)
    while True:
        items, count = fetch(capacity)
        if count < capacity:
            return items
        capacity *= 2


def _utc_from_c(u) -> UtcTime:
    """Convert a DhruvUtcTime C struct to a Python UtcTime."""
    return UtcTime(
        year=u.year,
        month=u.month,
        day=u.day,
        hour=u.hour,
        minute=u.minute,
        second=u.second,
    )


def _utc_struct(utc: UtcTime):
    out = ffi.new("DhruvUtcTime *")
    out.year = utc.year
    out.month = utc.month
    out.day = utc.day
    out.hour = utc.hour
    out.minute = utc.minute
    out.second = utc.second
    return out


def _set_single_search_time(req, when, *, arg_name: str) -> None:
    if isinstance(when, UtcTime):
        req.time_kind = _SEARCH_TIME_UTC
        req.at_utc = _utc_struct(when)[0]
        return
    if when is None:
        raise ValueError(f"{arg_name} is required")
    req.time_kind = _SEARCH_TIME_JD_TDB
    req.at_jd_tdb = float(when)


def _set_range_search_time(req, start, end, *, start_name: str, end_name: str) -> None:
    if start is None or end is None:
        missing = start_name if start is None else end_name
        raise ValueError(f"{missing} is required")
    start_is_utc = isinstance(start, UtcTime)
    end_is_utc = isinstance(end, UtcTime)
    if start_is_utc != end_is_utc:
        raise TypeError(f"{start_name} and {end_name} must use the same time input form")
    if start_is_utc:
        req.time_kind = _SEARCH_TIME_UTC
        req.start_utc = _utc_struct(start)[0]
        req.end_utc = _utc_struct(end)[0]
        return
    req.time_kind = _SEARCH_TIME_JD_TDB
    req.start_jd_tdb = float(start)
    req.end_jd_tdb = float(end)


def _conjunction_event(e) -> ConjunctionEvent:
    return ConjunctionEvent(
        utc=_utc_from_c(e.utc),
        jd_tdb=e.jd_tdb,
        actual_separation_deg=e.actual_separation_deg,
        body1_longitude_deg=e.body1_longitude_deg,
        body2_longitude_deg=e.body2_longitude_deg,
        body1_latitude_deg=e.body1_latitude_deg,
        body2_latitude_deg=e.body2_latitude_deg,
        body1_code=e.body1_code,
        body2_code=e.body2_code,
    )


def _chandra_grahan(r) -> ChandraGrahanResult:
    return ChandraGrahanResult(
        grahan_type=r.grahan_type,
        magnitude=r.magnitude,
        penumbral_magnitude=r.penumbral_magnitude,
        greatest_grahan_utc=_utc_from_c(r.greatest_grahan_utc),
        greatest_grahan_jd=r.greatest_grahan_jd,
        p1_utc=_utc_from_c(r.p1_utc),
        p1_jd=r.p1_jd,
        u1_utc=None if r.u1_jd == _JD_ABSENT else _utc_from_c(r.u1_utc),
        u1_jd=r.u1_jd,
        u2_utc=None if r.u2_jd == _JD_ABSENT else _utc_from_c(r.u2_utc),
        u2_jd=r.u2_jd,
        u3_utc=None if r.u3_jd == _JD_ABSENT else _utc_from_c(r.u3_utc),
        u3_jd=r.u3_jd,
        u4_utc=None if r.u4_jd == _JD_ABSENT else _utc_from_c(r.u4_utc),
        u4_jd=r.u4_jd,
        p4_utc=_utc_from_c(r.p4_utc),
        p4_jd=r.p4_jd,
        moon_ecliptic_lat_deg=r.moon_ecliptic_lat_deg,
        angular_separation_deg=r.angular_separation_deg,
    )


def _surya_grahan(r) -> SuryaGrahanResult:
    return SuryaGrahanResult(
        grahan_type=r.grahan_type,
        magnitude=r.magnitude,
        greatest_grahan_utc=_utc_from_c(r.greatest_grahan_utc),
        greatest_grahan_jd=r.greatest_grahan_jd,
        c1_utc=None if r.c1_jd == _JD_ABSENT else _utc_from_c(r.c1_utc),
        c1_jd=r.c1_jd,
        c2_utc=None if r.c2_jd == _JD_ABSENT else _utc_from_c(r.c2_utc),
        c2_jd=r.c2_jd,
        c3_utc=None if r.c3_jd == _JD_ABSENT else _utc_from_c(r.c3_utc),
        c3_jd=r.c3_jd,
        c4_utc=None if r.c4_jd == _JD_ABSENT else _utc_from_c(r.c4_utc),
        c4_jd=r.c4_jd,
        moon_ecliptic_lat_deg=r.moon_ecliptic_lat_deg,
        angular_separation_deg=r.angular_separation_deg,
    )


def _stationary_event(e) -> StationaryEvent:
    return StationaryEvent(
        utc=_utc_from_c(e.utc),
        jd_tdb=e.jd_tdb,
        body_code=e.body_code,
        longitude_deg=e.longitude_deg,
        latitude_deg=e.latitude_deg,
        station_type=e.station_type,
    )


def _max_speed_event(e) -> MaxSpeedEvent:
    return MaxSpeedEvent(
        utc=_utc_from_c(e.utc),
        jd_tdb=e.jd_tdb,
        body_code=e.body_code,
        longitude_deg=e.longitude_deg,
        latitude_deg=e.latitude_deg,
        speed_deg_per_day=e.speed_deg_per_day,
        speed_type=e.speed_type,
    )


def _lunar_phase_event(e) -> LunarPhaseEvent:
    return LunarPhaseEvent(
        utc=_utc_from_c(e.utc),
        phase=e.phase,
        moon_longitude_deg=e.moon_longitude_deg,
        sun_longitude_deg=e.sun_longitude_deg,
    )


def _sankranti_event(e) -> SankrantiEvent:
    return SankrantiEvent(
        utc=_utc_from_c(e.utc),
        rashi_index=e.rashi_index,
        sun_sidereal_longitude_deg=e.sun_sidereal_longitude_deg,
        sun_tropical_longitude_deg=e.sun_tropical_longitude_deg,
    )


# ---------------------------------------------------------------------------
# Conjunction search (dhruv_conjunction_search_ex)
# ---------------------------------------------------------------------------

# Query mode constants
_CONJUNCTION_NEXT = 0
_CONJUNCTION_PREV = 1
_CONJUNCTION_RANGE = 2


def conjunction_config_default():
    """Return default DhruvConjunctionConfig."""
    return lib.dhruv_conjunction_config_default()


def next_conjunction(
    engine,
    body1_code: int,
    body2_code: int,
    after_jd_tdb,
    config=None,
) -> Optional[ConjunctionEvent]:
    """Find next conjunction after a ``UtcTime`` or JD(TDB) anchor."""
    req = ffi.new("DhruvConjunctionSearchRequest *")
    req.body1_code = body1_code
    req.body2_code = body2_code
    req.query_mode = _CONJUNCTION_NEXT
    _set_single_search_time(req, after_jd_tdb, arg_name="after_jd_tdb")
    req.config = config if config is not None else lib.dhruv_conjunction_config_default()

    out_event = ffi.new("DhruvConjunctionEvent *")
    out_found = ffi.new("uint8_t *")
    check(
        lib.dhruv_conjunction_search_ex(
            engine, req, out_event, out_found,
            ffi.NULL, 0, ffi.NULL,
        ),
        "conjunction_search_ex(next)",
    )
    if out_found[0] == 0:
        return None
    return _conjunction_event(out_event[0])


def prev_conjunction(
    engine,
    body1_code: int,
    body2_code: int,
    before_jd_tdb,
    config=None,
) -> Optional[ConjunctionEvent]:
    """Find previous conjunction before a ``UtcTime`` or JD(TDB) anchor."""
    req = ffi.new("DhruvConjunctionSearchRequest *")
    req.body1_code = body1_code
    req.body2_code = body2_code
    req.query_mode = _CONJUNCTION_PREV
    _set_single_search_time(req, before_jd_tdb, arg_name="before_jd_tdb")
    req.config = config if config is not None else lib.dhruv_conjunction_config_default()

    out_event = ffi.new("DhruvConjunctionEvent *")
    out_found = ffi.new("uint8_t *")
    check(
        lib.dhruv_conjunction_search_ex(
            engine, req, out_event, out_found,
            ffi.NULL, 0, ffi.NULL,
        ),
        "conjunction_search_ex(prev)",
    )
    if out_found[0] == 0:
        return None
    return _conjunction_event(out_event[0])


def search_conjunctions(
    engine,
    body1_code: int,
    body2_code: int,
    start_jd,
    end_jd,
    config=None,
    max_results: int = 100,
) -> list[ConjunctionEvent]:
    """Search for conjunctions in a UTC or JD(TDB) range."""
    req = ffi.new("DhruvConjunctionSearchRequest *")
    req.body1_code = body1_code
    req.body2_code = body2_code
    req.query_mode = _CONJUNCTION_RANGE
    _set_range_search_time(req, start_jd, end_jd, start_name="start_jd", end_name="end_jd")
    req.config = config if config is not None else lib.dhruv_conjunction_config_default()

    def fetch(capacity: int):
        out_events = ffi.new("DhruvConjunctionEvent[]", capacity)
        out_count = ffi.new("uint32_t *")
        check(
            lib.dhruv_conjunction_search_ex(
                engine, req,
                ffi.NULL, ffi.NULL,
                out_events, capacity, out_count,
            ),
            "conjunction_search_ex(range)",
        )
        count = int(out_count[0])
        return ([_conjunction_event(out_events[i]) for i in range(count)], count)

    return _collect_full_range(fetch, max_results)


# ---------------------------------------------------------------------------
# Eclipse search (dhruv_grahan_search_ex)
# ---------------------------------------------------------------------------

_GRAHAN_CHANDRA = 0
_GRAHAN_SURYA = 1
_GRAHAN_NEXT = 0
_GRAHAN_PREV = 1
_GRAHAN_RANGE = 2


def grahan_config_default():
    """Return default DhruvGrahanConfig."""
    return lib.dhruv_grahan_config_default()


def _grahan_single(engine, grahan_kind: int, query_mode: int, when, config):
    """Internal: single grahan search (NEXT/PREV)."""
    req = ffi.new("DhruvGrahanSearchRequest *")
    req.grahan_kind = grahan_kind
    req.query_mode = query_mode
    _set_single_search_time(req, when, arg_name="jd")
    req.config = config if config is not None else lib.dhruv_grahan_config_default()

    out_chandra = ffi.new("DhruvChandraGrahanResult *")
    out_surya = ffi.new("DhruvSuryaGrahanResult *")
    out_found = ffi.new("uint8_t *")
    check(
        lib.dhruv_grahan_search_ex(
            engine, req,
            out_chandra, out_surya, out_found,
            ffi.NULL, ffi.NULL, 0, ffi.NULL,
        ),
        "grahan_search_ex(single)",
    )
    if out_found[0] == 0:
        return None
    if grahan_kind == _GRAHAN_CHANDRA:
        return _chandra_grahan(out_chandra[0])
    return _surya_grahan(out_surya[0])


def next_lunar_eclipse(engine, after_jd, config=None) -> Optional[ChandraGrahanResult]:
    """Find the next lunar eclipse after a ``UtcTime`` or JD(TDB) anchor."""
    return _grahan_single(engine, _GRAHAN_CHANDRA, _GRAHAN_NEXT, after_jd, config)


def prev_lunar_eclipse(engine, before_jd, config=None) -> Optional[ChandraGrahanResult]:
    """Find the previous lunar eclipse before a ``UtcTime`` or JD(TDB) anchor."""
    return _grahan_single(engine, _GRAHAN_CHANDRA, _GRAHAN_PREV, before_jd, config)


def next_solar_eclipse(engine, after_jd, config=None) -> Optional[SuryaGrahanResult]:
    """Find the next solar eclipse after a ``UtcTime`` or JD(TDB) anchor."""
    return _grahan_single(engine, _GRAHAN_SURYA, _GRAHAN_NEXT, after_jd, config)


def prev_solar_eclipse(engine, before_jd, config=None) -> Optional[SuryaGrahanResult]:
    """Find the previous solar eclipse before a ``UtcTime`` or JD(TDB) anchor."""
    return _grahan_single(engine, _GRAHAN_SURYA, _GRAHAN_PREV, before_jd, config)


def search_lunar_eclipses(
    engine,
    start_jd,
    end_jd,
    config=None,
    max_results: int = 50,
) -> list[ChandraGrahanResult]:
    """Search for lunar eclipses in a UTC or JD(TDB) range."""
    req = ffi.new("DhruvGrahanSearchRequest *")
    req.grahan_kind = _GRAHAN_CHANDRA
    req.query_mode = _GRAHAN_RANGE
    _set_range_search_time(req, start_jd, end_jd, start_name="start_jd", end_name="end_jd")
    req.config = config if config is not None else lib.dhruv_grahan_config_default()

    def fetch(capacity: int):
        out_chandra = ffi.new("DhruvChandraGrahanResult[]", capacity)
        out_count = ffi.new("uint32_t *")
        check(
            lib.dhruv_grahan_search_ex(
                engine, req,
                ffi.NULL, ffi.NULL, ffi.NULL,
                out_chandra, ffi.NULL, capacity, out_count,
            ),
            "grahan_search_ex(chandra_range)",
        )
        count = int(out_count[0])
        return ([_chandra_grahan(out_chandra[i]) for i in range(count)], count)

    return _collect_full_range(fetch, max_results)


def search_solar_eclipses(
    engine,
    start_jd,
    end_jd,
    config=None,
    max_results: int = 50,
) -> list[SuryaGrahanResult]:
    """Search for solar eclipses in a UTC or JD(TDB) range."""
    req = ffi.new("DhruvGrahanSearchRequest *")
    req.grahan_kind = _GRAHAN_SURYA
    req.query_mode = _GRAHAN_RANGE
    _set_range_search_time(req, start_jd, end_jd, start_name="start_jd", end_name="end_jd")
    req.config = config if config is not None else lib.dhruv_grahan_config_default()

    def fetch(capacity: int):
        out_surya = ffi.new("DhruvSuryaGrahanResult[]", capacity)
        out_count = ffi.new("uint32_t *")
        check(
            lib.dhruv_grahan_search_ex(
                engine, req,
                ffi.NULL, ffi.NULL, ffi.NULL,
                ffi.NULL, out_surya, capacity, out_count,
            ),
            "grahan_search_ex(surya_range)",
        )
        count = int(out_count[0])
        return ([_surya_grahan(out_surya[i]) for i in range(count)], count)

    return _collect_full_range(fetch, max_results)


# ---------------------------------------------------------------------------
# Motion search (dhruv_motion_search_ex)
# ---------------------------------------------------------------------------

_MOTION_STATIONARY = 0
_MOTION_MAX_SPEED = 1
_MOTION_NEXT = 0
_MOTION_PREV = 1
_MOTION_RANGE = 2


def stationary_config_default():
    """Return default DhruvStationaryConfig."""
    return lib.dhruv_stationary_config_default()


def _motion_single_stationary(engine, query_mode: int, body_code: int, when, config):
    """Internal: single stationary search."""
    req = ffi.new("DhruvMotionSearchRequest *")
    req.body_code = body_code
    req.motion_kind = _MOTION_STATIONARY
    req.query_mode = query_mode
    _set_single_search_time(req, when, arg_name="jd")
    req.config = config if config is not None else lib.dhruv_stationary_config_default()

    out_event = ffi.new("DhruvStationaryEvent *")
    out_found = ffi.new("uint8_t *")
    check(
        lib.dhruv_motion_search_ex(
            engine, req,
            out_event, ffi.NULL, out_found,
            ffi.NULL, ffi.NULL, 0, ffi.NULL,
        ),
        "motion_search_ex(stationary_single)",
    )
    if out_found[0] == 0:
        return None
    return _stationary_event(out_event[0])


def _motion_single_max_speed(engine, query_mode: int, body_code: int, when, config):
    """Internal: single max-speed search."""
    req = ffi.new("DhruvMotionSearchRequest *")
    req.body_code = body_code
    req.motion_kind = _MOTION_MAX_SPEED
    req.query_mode = query_mode
    _set_single_search_time(req, when, arg_name="jd")
    req.config = config if config is not None else lib.dhruv_stationary_config_default()

    out_event = ffi.new("DhruvMaxSpeedEvent *")
    out_found = ffi.new("uint8_t *")
    check(
        lib.dhruv_motion_search_ex(
            engine, req,
            ffi.NULL, out_event, out_found,
            ffi.NULL, ffi.NULL, 0, ffi.NULL,
        ),
        "motion_search_ex(max_speed_single)",
    )
    if out_found[0] == 0:
        return None
    return _max_speed_event(out_event[0])


def next_stationary(
    engine, body_code: int, after_jd, config=None
) -> Optional[StationaryEvent]:
    """Find next stationary point after a ``UtcTime`` or JD(TDB) anchor."""
    return _motion_single_stationary(engine, _MOTION_NEXT, body_code, after_jd, config)


def prev_stationary(
    engine, body_code: int, before_jd, config=None
) -> Optional[StationaryEvent]:
    """Find previous stationary point before a ``UtcTime`` or JD(TDB) anchor."""
    return _motion_single_stationary(engine, _MOTION_PREV, body_code, before_jd, config)


def search_stationary(
    engine,
    body_code: int,
    start_jd,
    end_jd,
    config=None,
    max_results: int = 100,
) -> list[StationaryEvent]:
    """Search for stationary points in a UTC or JD(TDB) range."""
    req = ffi.new("DhruvMotionSearchRequest *")
    req.body_code = body_code
    req.motion_kind = _MOTION_STATIONARY
    req.query_mode = _MOTION_RANGE
    _set_range_search_time(req, start_jd, end_jd, start_name="start_jd", end_name="end_jd")
    req.config = config if config is not None else lib.dhruv_stationary_config_default()

    def fetch(capacity: int):
        out_events = ffi.new("DhruvStationaryEvent[]", capacity)
        out_count = ffi.new("uint32_t *")
        check(
            lib.dhruv_motion_search_ex(
                engine, req,
                ffi.NULL, ffi.NULL, ffi.NULL,
                out_events, ffi.NULL, capacity, out_count,
            ),
            "motion_search_ex(stationary_range)",
        )
        count = int(out_count[0])
        return ([_stationary_event(out_events[i]) for i in range(count)], count)

    return _collect_full_range(fetch, max_results)


def next_max_speed(
    engine, body_code: int, after_jd, config=None
) -> Optional[MaxSpeedEvent]:
    """Find next max-speed event after a ``UtcTime`` or JD(TDB) anchor."""
    return _motion_single_max_speed(engine, _MOTION_NEXT, body_code, after_jd, config)


def prev_max_speed(
    engine, body_code: int, before_jd, config=None
) -> Optional[MaxSpeedEvent]:
    """Find previous max-speed event before a ``UtcTime`` or JD(TDB) anchor."""
    return _motion_single_max_speed(engine, _MOTION_PREV, body_code, before_jd, config)


def search_max_speeds(
    engine,
    body_code: int,
    start_jd,
    end_jd,
    config=None,
    max_results: int = 100,
) -> list[MaxSpeedEvent]:
    """Search for max-speed events in a UTC or JD(TDB) range."""
    req = ffi.new("DhruvMotionSearchRequest *")
    req.body_code = body_code
    req.motion_kind = _MOTION_MAX_SPEED
    req.query_mode = _MOTION_RANGE
    _set_range_search_time(req, start_jd, end_jd, start_name="start_jd", end_name="end_jd")
    req.config = config if config is not None else lib.dhruv_stationary_config_default()

    def fetch(capacity: int):
        out_events = ffi.new("DhruvMaxSpeedEvent[]", capacity)
        out_count = ffi.new("uint32_t *")
        check(
            lib.dhruv_motion_search_ex(
                engine, req,
                ffi.NULL, ffi.NULL, ffi.NULL,
                ffi.NULL, out_events, capacity, out_count,
            ),
            "motion_search_ex(max_speed_range)",
        )
        count = int(out_count[0])
        return ([_max_speed_event(out_events[i]) for i in range(count)], count)

    return _collect_full_range(fetch, max_results)


# ---------------------------------------------------------------------------
# Lunar phase search (dhruv_lunar_phase_search_ex)
# ---------------------------------------------------------------------------

_LUNAR_PHASE_AMAVASYA = 0
_LUNAR_PHASE_PURNIMA = 1
_LUNAR_PHASE_NEXT = 0
_LUNAR_PHASE_PREV = 1
_LUNAR_PHASE_RANGE = 2


def _lunar_phase_single(engine, phase_kind: int, query_mode: int, when):
    """Internal: single lunar-phase search."""
    req = ffi.new("DhruvLunarPhaseSearchRequest *")
    req.phase_kind = phase_kind
    req.query_mode = query_mode
    _set_single_search_time(req, when, arg_name="jd")

    out_event = ffi.new("DhruvLunarPhaseEvent *")
    out_found = ffi.new("uint8_t *")
    check(
        lib.dhruv_lunar_phase_search_ex(
            engine, req,
            out_event, out_found,
            ffi.NULL, 0, ffi.NULL,
        ),
        "lunar_phase_search_ex(single)",
    )
    if out_found[0] == 0:
        return None
    return _lunar_phase_event(out_event[0])


def next_purnima(engine, after_jd) -> Optional[LunarPhaseEvent]:
    """Find the next Purnima after a ``UtcTime`` or JD(TDB) anchor."""
    return _lunar_phase_single(engine, _LUNAR_PHASE_PURNIMA, _LUNAR_PHASE_NEXT, after_jd)


def prev_purnima(engine, before_jd) -> Optional[LunarPhaseEvent]:
    """Find the previous Purnima before a ``UtcTime`` or JD(TDB) anchor."""
    return _lunar_phase_single(engine, _LUNAR_PHASE_PURNIMA, _LUNAR_PHASE_PREV, before_jd)


def next_amavasya(engine, after_jd) -> Optional[LunarPhaseEvent]:
    """Find the next Amavasya after a ``UtcTime`` or JD(TDB) anchor."""
    return _lunar_phase_single(engine, _LUNAR_PHASE_AMAVASYA, _LUNAR_PHASE_NEXT, after_jd)


def prev_amavasya(engine, before_jd) -> Optional[LunarPhaseEvent]:
    """Find the previous Amavasya before a ``UtcTime`` or JD(TDB) anchor."""
    return _lunar_phase_single(engine, _LUNAR_PHASE_AMAVASYA, _LUNAR_PHASE_PREV, before_jd)


def search_lunar_phases(
    engine,
    phase_kind: int,
    start_jd,
    end_jd,
    max_results: int = 50,
) -> list[LunarPhaseEvent]:
    """Search for lunar phase events in a UTC or JD(TDB) range.

    *phase_kind*: 0=Amavasya, 1=Purnima.
    """
    req = ffi.new("DhruvLunarPhaseSearchRequest *")
    req.phase_kind = phase_kind
    req.query_mode = _LUNAR_PHASE_RANGE
    _set_range_search_time(req, start_jd, end_jd, start_name="start_jd", end_name="end_jd")

    def fetch(capacity: int):
        out_events = ffi.new("DhruvLunarPhaseEvent[]", capacity)
        out_count = ffi.new("uint32_t *")
        check(
            lib.dhruv_lunar_phase_search_ex(
                engine, req,
                ffi.NULL, ffi.NULL,
                out_events, capacity, out_count,
            ),
            "lunar_phase_search_ex(range)",
        )
        count = int(out_count[0])
        return ([_lunar_phase_event(out_events[i]) for i in range(count)], count)

    return _collect_full_range(fetch, max_results)


# ---------------------------------------------------------------------------
# Sankranti search (dhruv_sankranti_search_ex)
# ---------------------------------------------------------------------------

_SANKRANTI_TARGET_ANY = 0
_SANKRANTI_TARGET_SPECIFIC = 1
_SANKRANTI_NEXT = 0
_SANKRANTI_PREV = 1
_SANKRANTI_RANGE = 2


def sankranti_config_default():
    """Return default DhruvSankrantiConfig."""
    return lib.dhruv_sankranti_config_default()


def next_sankranti(
    engine, after_jd, config=None
) -> Optional[SankrantiEvent]:
    """Find the next sankranti after a ``UtcTime`` or JD(TDB) anchor."""
    req = ffi.new("DhruvSankrantiSearchRequest *")
    req.target_kind = _SANKRANTI_TARGET_ANY
    req.query_mode = _SANKRANTI_NEXT
    _set_single_search_time(req, after_jd, arg_name="after_jd")
    req.config = config if config is not None else lib.dhruv_sankranti_config_default()

    out_event = ffi.new("DhruvSankrantiEvent *")
    out_found = ffi.new("uint8_t *")
    check(
        lib.dhruv_sankranti_search_ex(
            engine, req,
            out_event, out_found,
            ffi.NULL, 0, ffi.NULL,
        ),
        "sankranti_search_ex(next)",
    )
    if out_found[0] == 0:
        return None
    return _sankranti_event(out_event[0])


def prev_sankranti(
    engine, before_jd, config=None
) -> Optional[SankrantiEvent]:
    """Find the previous sankranti before a ``UtcTime`` or JD(TDB) anchor."""
    req = ffi.new("DhruvSankrantiSearchRequest *")
    req.target_kind = _SANKRANTI_TARGET_ANY
    req.query_mode = _SANKRANTI_PREV
    _set_single_search_time(req, before_jd, arg_name="before_jd")
    req.config = config if config is not None else lib.dhruv_sankranti_config_default()

    out_event = ffi.new("DhruvSankrantiEvent *")
    out_found = ffi.new("uint8_t *")
    check(
        lib.dhruv_sankranti_search_ex(
            engine, req,
            out_event, out_found,
            ffi.NULL, 0, ffi.NULL,
        ),
        "sankranti_search_ex(prev)",
    )
    if out_found[0] == 0:
        return None
    return _sankranti_event(out_event[0])


def specific_sankranti(
    engine, at_jd, rashi_index: int, direction: str = "next", config=None
) -> Optional[SankrantiEvent]:
    """Find a direction-specific sankranti into a specific rashi.

    *rashi_index*: 0-based (0=Mesha .. 11=Meena).
    *direction*: ``"next"`` or ``"prev"``.
    """
    if direction == "next":
        query_mode = _SANKRANTI_NEXT
        op_name = "specific_next"
    elif direction == "prev":
        query_mode = _SANKRANTI_PREV
        op_name = "specific_prev"
    else:
        raise ValueError("direction must be 'next' or 'prev'")

    req = ffi.new("DhruvSankrantiSearchRequest *")
    req.target_kind = _SANKRANTI_TARGET_SPECIFIC
    req.query_mode = query_mode
    req.rashi_index = rashi_index
    _set_single_search_time(req, at_jd, arg_name="at_jd")
    req.config = config if config is not None else lib.dhruv_sankranti_config_default()

    out_event = ffi.new("DhruvSankrantiEvent *")
    out_found = ffi.new("uint8_t *")
    check(
        lib.dhruv_sankranti_search_ex(
            engine, req,
            out_event, out_found,
            ffi.NULL, 0, ffi.NULL,
        ),
        f"sankranti_search_ex({op_name})",
    )
    if out_found[0] == 0:
        return None
    return _sankranti_event(out_event[0])


def search_sankrantis(
    engine,
    start_jd,
    end_jd,
    config=None,
    max_results: int = 50,
) -> list[SankrantiEvent]:
    """Search for sankrantis in a UTC or JD(TDB) range."""
    req = ffi.new("DhruvSankrantiSearchRequest *")
    req.target_kind = _SANKRANTI_TARGET_ANY
    req.query_mode = _SANKRANTI_RANGE
    _set_range_search_time(req, start_jd, end_jd, start_name="start_jd", end_name="end_jd")
    req.config = config if config is not None else lib.dhruv_sankranti_config_default()

    def fetch(capacity: int):
        out_events = ffi.new("DhruvSankrantiEvent[]", capacity)
        out_count = ffi.new("uint32_t *")
        check(
            lib.dhruv_sankranti_search_ex(
                engine, req,
                ffi.NULL, ffi.NULL,
                out_events, capacity, out_count,
            ),
            "sankranti_search_ex(range)",
        )
        count = int(out_count[0])
        return ([_sankranti_event(out_events[i]) for i in range(count)], count)

    return _collect_full_range(fetch, max_results)
