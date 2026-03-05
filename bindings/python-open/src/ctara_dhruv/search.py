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


# ---------------------------------------------------------------------------
# Internal helpers
# ---------------------------------------------------------------------------


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


def _conjunction_event(e) -> ConjunctionEvent:
    return ConjunctionEvent(
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
        greatest_grahan_jd=r.greatest_grahan_jd,
        p1_jd=r.p1_jd,
        u1_jd=r.u1_jd,
        u2_jd=r.u2_jd,
        u3_jd=r.u3_jd,
        u4_jd=r.u4_jd,
        p4_jd=r.p4_jd,
        moon_ecliptic_lat_deg=r.moon_ecliptic_lat_deg,
        angular_separation_deg=r.angular_separation_deg,
    )


def _surya_grahan(r) -> SuryaGrahanResult:
    return SuryaGrahanResult(
        grahan_type=r.grahan_type,
        magnitude=r.magnitude,
        greatest_grahan_jd=r.greatest_grahan_jd,
        c1_jd=r.c1_jd,
        c2_jd=r.c2_jd,
        c3_jd=r.c3_jd,
        c4_jd=r.c4_jd,
        moon_ecliptic_lat_deg=r.moon_ecliptic_lat_deg,
        angular_separation_deg=r.angular_separation_deg,
    )


def _stationary_event(e) -> StationaryEvent:
    return StationaryEvent(
        jd_tdb=e.jd_tdb,
        body_code=e.body_code,
        longitude_deg=e.longitude_deg,
        latitude_deg=e.latitude_deg,
        station_type=e.station_type,
    )


def _max_speed_event(e) -> MaxSpeedEvent:
    return MaxSpeedEvent(
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
    after_jd_tdb: float,
    config=None,
) -> Optional[ConjunctionEvent]:
    """Find next conjunction after *after_jd_tdb*. Returns ConjunctionEvent or None."""
    req = ffi.new("DhruvConjunctionSearchRequest *")
    req.body1_code = body1_code
    req.body2_code = body2_code
    req.query_mode = _CONJUNCTION_NEXT
    req.at_jd_tdb = after_jd_tdb
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
    before_jd_tdb: float,
    config=None,
) -> Optional[ConjunctionEvent]:
    """Find previous conjunction before *before_jd_tdb*."""
    req = ffi.new("DhruvConjunctionSearchRequest *")
    req.body1_code = body1_code
    req.body2_code = body2_code
    req.query_mode = _CONJUNCTION_PREV
    req.at_jd_tdb = before_jd_tdb
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
    start_jd: float,
    end_jd: float,
    config=None,
    max_results: int = 100,
) -> list[ConjunctionEvent]:
    """Search for conjunctions in [start_jd, end_jd]. Returns list of events."""
    req = ffi.new("DhruvConjunctionSearchRequest *")
    req.body1_code = body1_code
    req.body2_code = body2_code
    req.query_mode = _CONJUNCTION_RANGE
    req.start_jd_tdb = start_jd
    req.end_jd_tdb = end_jd
    req.config = config if config is not None else lib.dhruv_conjunction_config_default()

    out_events = ffi.new("DhruvConjunctionEvent[]", max_results)
    out_count = ffi.new("uint32_t *")
    check(
        lib.dhruv_conjunction_search_ex(
            engine, req,
            ffi.NULL, ffi.NULL,
            out_events, max_results, out_count,
        ),
        "conjunction_search_ex(range)",
    )
    return [_conjunction_event(out_events[i]) for i in range(out_count[0])]


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


def _grahan_single(engine, grahan_kind: int, query_mode: int, jd: float, config):
    """Internal: single grahan search (NEXT/PREV)."""
    req = ffi.new("DhruvGrahanSearchRequest *")
    req.grahan_kind = grahan_kind
    req.query_mode = query_mode
    req.at_jd_tdb = jd
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


def next_lunar_eclipse(engine, after_jd: float, config=None) -> Optional[ChandraGrahanResult]:
    """Find the next lunar eclipse after *after_jd*."""
    return _grahan_single(engine, _GRAHAN_CHANDRA, _GRAHAN_NEXT, after_jd, config)


def prev_lunar_eclipse(engine, before_jd: float, config=None) -> Optional[ChandraGrahanResult]:
    """Find the previous lunar eclipse before *before_jd*."""
    return _grahan_single(engine, _GRAHAN_CHANDRA, _GRAHAN_PREV, before_jd, config)


def next_solar_eclipse(engine, after_jd: float, config=None) -> Optional[SuryaGrahanResult]:
    """Find the next solar eclipse after *after_jd*."""
    return _grahan_single(engine, _GRAHAN_SURYA, _GRAHAN_NEXT, after_jd, config)


def prev_solar_eclipse(engine, before_jd: float, config=None) -> Optional[SuryaGrahanResult]:
    """Find the previous solar eclipse before *before_jd*."""
    return _grahan_single(engine, _GRAHAN_SURYA, _GRAHAN_PREV, before_jd, config)


def search_lunar_eclipses(
    engine,
    start_jd: float,
    end_jd: float,
    config=None,
    max_results: int = 50,
) -> list[ChandraGrahanResult]:
    """Search for lunar eclipses in [start_jd, end_jd]."""
    req = ffi.new("DhruvGrahanSearchRequest *")
    req.grahan_kind = _GRAHAN_CHANDRA
    req.query_mode = _GRAHAN_RANGE
    req.start_jd_tdb = start_jd
    req.end_jd_tdb = end_jd
    req.config = config if config is not None else lib.dhruv_grahan_config_default()

    out_chandra = ffi.new("DhruvChandraGrahanResult[]", max_results)
    out_count = ffi.new("uint32_t *")
    check(
        lib.dhruv_grahan_search_ex(
            engine, req,
            ffi.NULL, ffi.NULL, ffi.NULL,
            out_chandra, ffi.NULL, max_results, out_count,
        ),
        "grahan_search_ex(chandra_range)",
    )
    return [_chandra_grahan(out_chandra[i]) for i in range(out_count[0])]


def search_solar_eclipses(
    engine,
    start_jd: float,
    end_jd: float,
    config=None,
    max_results: int = 50,
) -> list[SuryaGrahanResult]:
    """Search for solar eclipses in [start_jd, end_jd]."""
    req = ffi.new("DhruvGrahanSearchRequest *")
    req.grahan_kind = _GRAHAN_SURYA
    req.query_mode = _GRAHAN_RANGE
    req.start_jd_tdb = start_jd
    req.end_jd_tdb = end_jd
    req.config = config if config is not None else lib.dhruv_grahan_config_default()

    out_surya = ffi.new("DhruvSuryaGrahanResult[]", max_results)
    out_count = ffi.new("uint32_t *")
    check(
        lib.dhruv_grahan_search_ex(
            engine, req,
            ffi.NULL, ffi.NULL, ffi.NULL,
            ffi.NULL, out_surya, max_results, out_count,
        ),
        "grahan_search_ex(surya_range)",
    )
    return [_surya_grahan(out_surya[i]) for i in range(out_count[0])]


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


def _motion_single_stationary(engine, query_mode: int, body_code: int, jd: float, config):
    """Internal: single stationary search."""
    req = ffi.new("DhruvMotionSearchRequest *")
    req.body_code = body_code
    req.motion_kind = _MOTION_STATIONARY
    req.query_mode = query_mode
    req.at_jd_tdb = jd
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


def _motion_single_max_speed(engine, query_mode: int, body_code: int, jd: float, config):
    """Internal: single max-speed search."""
    req = ffi.new("DhruvMotionSearchRequest *")
    req.body_code = body_code
    req.motion_kind = _MOTION_MAX_SPEED
    req.query_mode = query_mode
    req.at_jd_tdb = jd
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
    engine, body_code: int, after_jd: float, config=None
) -> Optional[StationaryEvent]:
    """Find next stationary point after *after_jd*."""
    return _motion_single_stationary(engine, _MOTION_NEXT, body_code, after_jd, config)


def prev_stationary(
    engine, body_code: int, before_jd: float, config=None
) -> Optional[StationaryEvent]:
    """Find previous stationary point before *before_jd*."""
    return _motion_single_stationary(engine, _MOTION_PREV, body_code, before_jd, config)


def search_stationary(
    engine,
    body_code: int,
    start_jd: float,
    end_jd: float,
    config=None,
    max_results: int = 100,
) -> list[StationaryEvent]:
    """Search for stationary points in [start_jd, end_jd]."""
    req = ffi.new("DhruvMotionSearchRequest *")
    req.body_code = body_code
    req.motion_kind = _MOTION_STATIONARY
    req.query_mode = _MOTION_RANGE
    req.start_jd_tdb = start_jd
    req.end_jd_tdb = end_jd
    req.config = config if config is not None else lib.dhruv_stationary_config_default()

    out_events = ffi.new("DhruvStationaryEvent[]", max_results)
    out_count = ffi.new("uint32_t *")
    check(
        lib.dhruv_motion_search_ex(
            engine, req,
            ffi.NULL, ffi.NULL, ffi.NULL,
            out_events, ffi.NULL, max_results, out_count,
        ),
        "motion_search_ex(stationary_range)",
    )
    return [_stationary_event(out_events[i]) for i in range(out_count[0])]


def next_max_speed(
    engine, body_code: int, after_jd: float, config=None
) -> Optional[MaxSpeedEvent]:
    """Find next max-speed event after *after_jd*."""
    return _motion_single_max_speed(engine, _MOTION_NEXT, body_code, after_jd, config)


def prev_max_speed(
    engine, body_code: int, before_jd: float, config=None
) -> Optional[MaxSpeedEvent]:
    """Find previous max-speed event before *before_jd*."""
    return _motion_single_max_speed(engine, _MOTION_PREV, body_code, before_jd, config)


def search_max_speeds(
    engine,
    body_code: int,
    start_jd: float,
    end_jd: float,
    config=None,
    max_results: int = 100,
) -> list[MaxSpeedEvent]:
    """Search for max-speed events in [start_jd, end_jd]."""
    req = ffi.new("DhruvMotionSearchRequest *")
    req.body_code = body_code
    req.motion_kind = _MOTION_MAX_SPEED
    req.query_mode = _MOTION_RANGE
    req.start_jd_tdb = start_jd
    req.end_jd_tdb = end_jd
    req.config = config if config is not None else lib.dhruv_stationary_config_default()

    out_events = ffi.new("DhruvMaxSpeedEvent[]", max_results)
    out_count = ffi.new("uint32_t *")
    check(
        lib.dhruv_motion_search_ex(
            engine, req,
            ffi.NULL, ffi.NULL, ffi.NULL,
            ffi.NULL, out_events, max_results, out_count,
        ),
        "motion_search_ex(max_speed_range)",
    )
    return [_max_speed_event(out_events[i]) for i in range(out_count[0])]


# ---------------------------------------------------------------------------
# Lunar phase search (dhruv_lunar_phase_search_ex)
# ---------------------------------------------------------------------------

_LUNAR_PHASE_AMAVASYA = 0
_LUNAR_PHASE_PURNIMA = 1
_LUNAR_PHASE_NEXT = 0
_LUNAR_PHASE_PREV = 1
_LUNAR_PHASE_RANGE = 2


def _lunar_phase_single(engine, phase_kind: int, query_mode: int, jd: float):
    """Internal: single lunar-phase search."""
    req = ffi.new("DhruvLunarPhaseSearchRequest *")
    req.phase_kind = phase_kind
    req.query_mode = query_mode
    req.at_jd_tdb = jd

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


def next_purnima(engine, after_jd: float) -> Optional[LunarPhaseEvent]:
    """Find the next Purnima (full moon) after *after_jd*."""
    return _lunar_phase_single(engine, _LUNAR_PHASE_PURNIMA, _LUNAR_PHASE_NEXT, after_jd)


def prev_purnima(engine, before_jd: float) -> Optional[LunarPhaseEvent]:
    """Find the previous Purnima (full moon) before *before_jd*."""
    return _lunar_phase_single(engine, _LUNAR_PHASE_PURNIMA, _LUNAR_PHASE_PREV, before_jd)


def next_amavasya(engine, after_jd: float) -> Optional[LunarPhaseEvent]:
    """Find the next Amavasya (new moon) after *after_jd*."""
    return _lunar_phase_single(engine, _LUNAR_PHASE_AMAVASYA, _LUNAR_PHASE_NEXT, after_jd)


def prev_amavasya(engine, before_jd: float) -> Optional[LunarPhaseEvent]:
    """Find the previous Amavasya (new moon) before *before_jd*."""
    return _lunar_phase_single(engine, _LUNAR_PHASE_AMAVASYA, _LUNAR_PHASE_PREV, before_jd)


def search_lunar_phases(
    engine,
    phase_kind: int,
    start_jd: float,
    end_jd: float,
    max_results: int = 50,
) -> list[LunarPhaseEvent]:
    """Search for lunar phase events in [start_jd, end_jd].

    *phase_kind*: 0=Amavasya, 1=Purnima.
    """
    req = ffi.new("DhruvLunarPhaseSearchRequest *")
    req.phase_kind = phase_kind
    req.query_mode = _LUNAR_PHASE_RANGE
    req.start_jd_tdb = start_jd
    req.end_jd_tdb = end_jd

    out_events = ffi.new("DhruvLunarPhaseEvent[]", max_results)
    out_count = ffi.new("uint32_t *")
    check(
        lib.dhruv_lunar_phase_search_ex(
            engine, req,
            ffi.NULL, ffi.NULL,
            out_events, max_results, out_count,
        ),
        "lunar_phase_search_ex(range)",
    )
    return [_lunar_phase_event(out_events[i]) for i in range(out_count[0])]


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
    engine, after_jd: float, config=None
) -> Optional[SankrantiEvent]:
    """Find the next sankranti (any rashi) after *after_jd*."""
    req = ffi.new("DhruvSankrantiSearchRequest *")
    req.target_kind = _SANKRANTI_TARGET_ANY
    req.query_mode = _SANKRANTI_NEXT
    req.at_jd_tdb = after_jd
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
    engine, before_jd: float, config=None
) -> Optional[SankrantiEvent]:
    """Find the previous sankranti (any rashi) before *before_jd*."""
    req = ffi.new("DhruvSankrantiSearchRequest *")
    req.target_kind = _SANKRANTI_TARGET_ANY
    req.query_mode = _SANKRANTI_PREV
    req.at_jd_tdb = before_jd
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


def next_specific_sankranti(
    engine, after_jd: float, rashi_index: int, config=None
) -> Optional[SankrantiEvent]:
    """Find the next sankranti into a specific rashi after *after_jd*.

    *rashi_index*: 0-based (0=Mesha .. 11=Meena).
    """
    req = ffi.new("DhruvSankrantiSearchRequest *")
    req.target_kind = _SANKRANTI_TARGET_SPECIFIC
    req.query_mode = _SANKRANTI_NEXT
    req.rashi_index = rashi_index
    req.at_jd_tdb = after_jd
    req.config = config if config is not None else lib.dhruv_sankranti_config_default()

    out_event = ffi.new("DhruvSankrantiEvent *")
    out_found = ffi.new("uint8_t *")
    check(
        lib.dhruv_sankranti_search_ex(
            engine, req,
            out_event, out_found,
            ffi.NULL, 0, ffi.NULL,
        ),
        "sankranti_search_ex(specific_next)",
    )
    if out_found[0] == 0:
        return None
    return _sankranti_event(out_event[0])


def search_sankrantis(
    engine,
    start_jd: float,
    end_jd: float,
    config=None,
    max_results: int = 50,
) -> list[SankrantiEvent]:
    """Search for sankrantis in [start_jd, end_jd]."""
    req = ffi.new("DhruvSankrantiSearchRequest *")
    req.target_kind = _SANKRANTI_TARGET_ANY
    req.query_mode = _SANKRANTI_RANGE
    req.start_jd_tdb = start_jd
    req.end_jd_tdb = end_jd
    req.config = config if config is not None else lib.dhruv_sankranti_config_default()

    out_events = ffi.new("DhruvSankrantiEvent[]", max_results)
    out_count = ffi.new("uint32_t *")
    check(
        lib.dhruv_sankranti_search_ex(
            engine, req,
            ffi.NULL, ffi.NULL,
            out_events, max_results, out_count,
        ),
        "sankranti_search_ex(range)",
    )
    return [_sankranti_event(out_events[i]) for i in range(out_count[0])]
