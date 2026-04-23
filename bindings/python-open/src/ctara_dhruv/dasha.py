"""Dasha (planetary period) hierarchy and snapshot computation.

Wraps the dhruv_ffi_c dasha APIs for birth-chart period analysis.
"""

from __future__ import annotations

from dataclasses import dataclass
from typing import Optional

from ._ffi import ffi, lib
from ._check import check
from .types import DashaPeriod, DashaSnapshot, UtcTime


DHRUV_DASHA_TIME_JD_UTC = 0
DHRUV_DASHA_TIME_UTC = 1


# ---------------------------------------------------------------------------
# Python result types
# ---------------------------------------------------------------------------


@dataclass(frozen=True)
class DashaLevel:
    """All periods at a single hierarchical level.

    ``level``: 0=Maha, 1=Antar, 2=Pratyantar, 3=Sookshma, 4=Prana.
    ``periods``: list of DashaPeriod at this level.
    """

    level: int
    periods: list[DashaPeriod]


@dataclass(frozen=True)
class DashaHierarchy:
    """Complete dasha hierarchy with all computed levels.

    ``levels``: list of DashaLevel (one per computed depth).
    """

    levels: list[DashaLevel]
    system: Optional[int] = None


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def _make_utc(jd_utc):
    utc = ffi.new("DhruvUtcTime *")
    utc.year = jd_utc[0]
    utc.month = jd_utc[1]
    utc.day = jd_utc[2]
    utc.hour = jd_utc[3] if len(jd_utc) > 3 else 0
    utc.minute = jd_utc[4] if len(jd_utc) > 4 else 0
    utc.second = jd_utc[5] if len(jd_utc) > 5 else 0.0
    return utc


def _utc_from_c(u):
    return UtcTime(
        year=u.year,
        month=u.month,
        day=u.day,
        hour=u.hour,
        minute=u.minute,
        second=u.second,
    )


def _make_location(location):
    loc = ffi.new("DhruvGeoLocation *")
    loc.latitude_deg = location[0]
    loc.longitude_deg = location[1]
    loc.altitude_m = location[2] if len(location) > 2 else 0.0
    return loc


def _make_bhava_config(bhava_config):
    if bhava_config is None:
        return ffi.NULL
    cfg = ffi.new("DhruvBhavaConfig *")
    cfg.system = bhava_config.get("system", 0)
    cfg.starting_point = bhava_config.get("starting_point", -1)
    cfg.custom_start_deg = bhava_config.get("custom_start_deg", 0.0)
    cfg.reference_mode = bhava_config.get("reference_mode", 0)
    cfg.use_rashi_bhava_for_bala_avastha = bhava_config.get(
        "use_rashi_bhava_for_bala_avastha", 1
    )
    cfg.include_node_aspects_for_drik_bala = bhava_config.get(
        "include_node_aspects_for_drik_bala", 0
    )
    cfg.include_rashi_bhava_results = bhava_config.get("include_rashi_bhava_results", 1)
    return cfg


def _make_riseset_config(riseset_config):
    if riseset_config is None:
        return ffi.NULL
    cfg = ffi.new("DhruvRiseSetConfig *")
    cfg.use_refraction = riseset_config.get("use_refraction", 1)
    cfg.sun_limb = riseset_config.get("sun_limb", 0)
    cfg.altitude_correction = riseset_config.get("altitude_correction", 0)
    return cfg


def _extract_period(p):
    return DashaPeriod(
        entity_type=p.entity_type,
        entity_index=p.entity_index,
        start_utc=_utc_from_c(p.start_utc),
        end_utc=_utc_from_c(p.end_utc),
        start_jd=p.start_jd,
        end_jd=p.end_jd,
        level=p.level,
        order=p.order,
        parent_idx=p.parent_idx,
        entity_name=ffi.string(p.entity_name).decode("utf-8") if p.entity_name != ffi.NULL else None,
    )


def _make_period(period):
    out = ffi.new("DhruvDashaPeriod *")
    out.entity_type = period.entity_type
    out.entity_index = period.entity_index
    out.entity_name = ffi.NULL
    out.start_jd = period.start_jd
    out.end_jd = period.end_jd
    if getattr(period, "start_utc", None) is not None:
        out.start_utc.year = period.start_utc.year
        out.start_utc.month = period.start_utc.month
        out.start_utc.day = period.start_utc.day
        out.start_utc.hour = period.start_utc.hour
        out.start_utc.minute = period.start_utc.minute
        out.start_utc.second = period.start_utc.second
    if getattr(period, "end_utc", None) is not None:
        out.end_utc.year = period.end_utc.year
        out.end_utc.month = period.end_utc.month
        out.end_utc.day = period.end_utc.day
        out.end_utc.hour = period.end_utc.hour
        out.end_utc.minute = period.end_utc.minute
        out.end_utc.second = period.end_utc.second
    out.level = period.level
    out.order = period.order
    out.parent_idx = period.parent_idx
    return out


def _make_variation_config(variation_config):
    default = lib.dhruv_dasha_variation_config_default()
    cfg = ffi.new("DhruvDashaVariationConfig *", default)
    if variation_config is None:
        return cfg
    cfg.level_methods = default.level_methods
    cfg.yogini_scheme = default.yogini_scheme
    cfg.use_abhijit = default.use_abhijit
    for idx, method in enumerate(variation_config.get("level_methods", [])):
        if idx >= 5:
            break
        cfg.level_methods[idx] = method if method is not None else 0xFF
    if "yogini_scheme" in variation_config:
        cfg.yogini_scheme = variation_config["yogini_scheme"]
    if "use_abhijit" in variation_config:
        cfg.use_abhijit = 1 if variation_config["use_abhijit"] else 0
    return cfg


def _utc_tuple_to_jd_utc(jd_utc):
    year = jd_utc[0]
    month = jd_utc[1]
    day = jd_utc[2] + (jd_utc[3] if len(jd_utc) > 3 else 0) / 24.0
    day += (jd_utc[4] if len(jd_utc) > 4 else 0) / 1440.0
    day += (jd_utc[5] if len(jd_utc) > 5 else 0.0) / 86400.0
    if month <= 2:
        year -= 1
        month += 12
    a = int(year / 100)
    b = 2 - a + int(a / 4)
    return int(365.25 * (year + 4716)) + int(30.6001 * (month + 1)) + day + b - 1524.5


def _make_dasha_inputs(inputs):
    raw = ffi.new("DhruvDashaInputs *")
    if inputs is None:
        return raw

    if "moon_sid_lon" in inputs and inputs["moon_sid_lon"] is not None:
        raw.has_moon_sid_lon = 1
        raw.moon_sid_lon = inputs["moon_sid_lon"]

    rashi_inputs = inputs.get("rashi_inputs")
    graha_sidereal_lons = inputs.get("graha_sidereal_lons")
    lagna_sidereal_lon = inputs.get("lagna_sidereal_lon")
    if rashi_inputs is not None:
        graha_sidereal_lons = rashi_inputs.get("graha_sidereal_lons", graha_sidereal_lons)
        lagna_sidereal_lon = rashi_inputs.get("lagna_sidereal_lon", lagna_sidereal_lon)
    if graha_sidereal_lons is not None or lagna_sidereal_lon is not None:
        if graha_sidereal_lons is None or lagna_sidereal_lon is None:
            raise ValueError("rashi dasha inputs require graha_sidereal_lons and lagna_sidereal_lon")
        if len(graha_sidereal_lons) != 9:
            raise ValueError("graha_sidereal_lons must contain exactly 9 values")
        raw.has_rashi_inputs = 1
        for idx, value in enumerate(graha_sidereal_lons):
            raw.rashi_inputs.graha_sidereal_lons[idx] = value
        raw.rashi_inputs.lagna_sidereal_lon = lagna_sidereal_lon

    sunrise_sunset = inputs.get("sunrise_sunset")
    if sunrise_sunset is not None:
        if len(sunrise_sunset) != 2:
            raise ValueError("sunrise_sunset must be a 2-tuple of JD UTC values")
        raw.has_sunrise_sunset = 1
        raw.sunrise_jd = sunrise_sunset[0]
        raw.sunset_jd = sunrise_sunset[1]
    elif "sunrise_jd" in inputs or "sunset_jd" in inputs:
        if "sunrise_jd" not in inputs or "sunset_jd" not in inputs:
            raise ValueError("sunrise_jd and sunset_jd must be provided together")
        raw.has_sunrise_sunset = 1
        raw.sunrise_jd = inputs["sunrise_jd"]
        raw.sunset_jd = inputs["sunset_jd"]

    return raw


def _make_dasha_birth_context(
    jd_utc_birth,
    location,
    ayanamsha_system,
    use_nutation,
    bhava_config,
    riseset_config,
    birth_jd,
    inputs,
):
    ctx = ffi.new("DhruvDashaBirthContext *")
    ctx.bhava_config = lib.dhruv_bhava_config_default()
    ctx.riseset_config = lib.dhruv_riseset_config_default()
    ctx.sankranti_config = lib.dhruv_sankranti_config_default()
    ctx.sankranti_config.ayanamsha_system = ayanamsha_system
    ctx.sankranti_config.use_nutation = use_nutation

    if bhava_config is not None:
        ctx.bhava_config = _make_bhava_config(bhava_config)[0]
    if riseset_config is not None:
        ctx.riseset_config = _make_riseset_config(riseset_config)[0]
    if location is not None:
        ctx.has_location = 1
        ctx.location = _make_location(location)[0]
    if inputs is not None:
        ctx.has_inputs = 1
        ctx.inputs = _make_dasha_inputs(inputs)[0]
        ctx.time_kind = DHRUV_DASHA_TIME_JD_UTC
        if birth_jd is None:
            if jd_utc_birth is None:
                raise ValueError("birth_jd or jd_utc_birth is required when using dasha inputs")
            birth_jd = _utc_tuple_to_jd_utc(jd_utc_birth)
        ctx.birth_jd = birth_jd
        if jd_utc_birth is not None:
            ctx.birth_utc = _make_utc(jd_utc_birth)[0]
    else:
        if jd_utc_birth is None or location is None:
            raise ValueError("jd_utc_birth and location are required when inputs are not provided")
        ctx.time_kind = DHRUV_DASHA_TIME_UTC
        ctx.birth_utc = _make_utc(jd_utc_birth)[0]

    return ctx


def _extract_period_list(handle):
    try:
        count_out = ffi.new("uint32_t *")
        check(lib.dhruv_dasha_period_list_count(handle, count_out), "period_list_count")
        period_out = ffi.new("DhruvDashaPeriod *")
        periods = []
        for idx in range(count_out[0]):
            check(lib.dhruv_dasha_period_list_at(handle, idx, period_out), "period_list_at")
            periods.append(_extract_period(period_out))
        return periods
    finally:
        lib.dhruv_dasha_period_list_free(handle)


def _parse_entity(entity):
    if isinstance(entity, dict):
        return entity["type"], entity["index"]
    if isinstance(entity, tuple) and len(entity) == 2:
        return entity[0], entity[1]
    raise TypeError("entity must be {'type': ..., 'index': ...} or (type, index)")


# ---------------------------------------------------------------------------
# Dasha selection config
# ---------------------------------------------------------------------------


def dasha_selection_config_default():
    """Return a default DhruvDashaSelectionConfig C struct.

    count=0 (no systems selected), max_level=2, no snapshot.
    """
    return lib.dhruv_dasha_selection_config_default()


def dasha_variation_config_default():
    """Return a default DhruvDashaVariationConfig C struct."""
    return lib.dhruv_dasha_variation_config_default()


# ---------------------------------------------------------------------------
# Dasha hierarchy
# ---------------------------------------------------------------------------


def dasha_hierarchy(
    engine,
    lsk,
    eop,
    jd_utc_birth=None,
    location=None,
    system=0,
    max_level=2,
    ayanamsha_system=0,
    use_nutation=1,
    bhava_config=None,
    riseset_config=None,
    variation_config=None,
    *,
    birth_jd=None,
    inputs=None,
):
    """Compute a full dasha hierarchy for a birth chart.

    Args:
        engine: Engine instance.
        lsk: LSK handle.
        eop: EOP handle.
        jd_utc_birth: Birth UTC time tuple (year, month, day[, hour, min, sec]).
        location: Birth location (lat, lon[, alt]) tuple.
        system: DashaSystem code (0=Vimshottari, etc.).
        max_level: Maximum depth (0-4, default 2).
        ayanamsha_system: Ayanamsha system code.
        use_nutation: 1=yes, 0=no.
        bhava_config: Optional bhava config dict.
        riseset_config: Optional riseset config dict.

    Returns:
        DashaHierarchy with all periods extracted.
    """
    request = ffi.new("DhruvDashaHierarchyRequest *")
    request.birth = _make_dasha_birth_context(
        jd_utc_birth,
        location,
        ayanamsha_system,
        use_nutation,
        bhava_config,
        riseset_config,
        birth_jd,
        inputs,
    )[0]
    request.system = system
    request.max_level = max_level
    request.variation = _make_variation_config(variation_config)[0]
    handle = ffi.new("void **")
    check(
        lib.dhruv_dasha_hierarchy(
            engine._ptr,
            eop,
            request,
            handle,
        ),
        "dasha_hierarchy",
    )
    h = handle[0]

    try:
        # Get number of levels
        level_count_out = ffi.new("uint8_t *")
        check(lib.dhruv_dasha_hierarchy_level_count(h, level_count_out), "level_count")
        level_count = level_count_out[0]

        levels = []
        for lvl in range(level_count):
            # Get period count at this level
            period_count_out = ffi.new("uint32_t *")
            check(
                lib.dhruv_dasha_hierarchy_period_count(h, lvl, period_count_out),
                "period_count",
            )
            period_count = period_count_out[0]

            # Extract all periods
            periods = []
            period_out = ffi.new("DhruvDashaPeriod *")
            for idx in range(period_count):
                check(
                    lib.dhruv_dasha_hierarchy_period_at(h, lvl, idx, period_out),
                    "period_at",
                )
                periods.append(_extract_period(period_out))

            levels.append(DashaLevel(level=lvl, periods=periods))

        return DashaHierarchy(levels=levels, system=system)
    finally:
        lib.dhruv_dasha_hierarchy_free(h)


# ---------------------------------------------------------------------------
# Dasha snapshot
# ---------------------------------------------------------------------------


def dasha_snapshot(
    engine,
    lsk,
    eop,
    jd_utc_birth=None,
    jd_utc_query=None,
    location=None,
    system=0,
    max_level=2,
    ayanamsha_system=0,
    use_nutation=1,
    bhava_config=None,
    riseset_config=None,
    variation_config=None,
    *,
    birth_jd=None,
    query_jd=None,
    inputs=None,
):
    """Get dasha snapshot (active periods) at a specific query time.

    Args:
        engine: Engine instance.
        lsk: LSK handle.
        eop: EOP handle.
        jd_utc_birth: Birth UTC time tuple.
        jd_utc_query: Query UTC time tuple.
        location: Birth location tuple.
        system: DashaSystem code (0=Vimshottari).
        max_level: Maximum depth (0-4).
        ayanamsha_system: Ayanamsha system code.
        use_nutation: 1=yes, 0=no.
        bhava_config: Optional bhava config dict.
        riseset_config: Optional riseset config dict.

    Returns:
        DashaSnapshot with active periods.
    """
    request = ffi.new("DhruvDashaSnapshotRequest *")
    request.birth = _make_dasha_birth_context(
        jd_utc_birth,
        location,
        ayanamsha_system,
        use_nutation,
        bhava_config,
        riseset_config,
        birth_jd,
        inputs,
    )[0]
    request.system = system
    request.max_level = max_level
    request.variation = _make_variation_config(variation_config)[0]
    if query_jd is not None:
        request.query_time_kind = DHRUV_DASHA_TIME_JD_UTC
        request.query_jd = query_jd
    else:
        request.query_time_kind = DHRUV_DASHA_TIME_UTC
        request.query_utc = _make_utc(jd_utc_query)[0]
    out = ffi.new("DhruvDashaSnapshot *")
    check(
        lib.dhruv_dasha_snapshot(
            engine._ptr,
            eop,
            request,
            out,
        ),
        "dasha_snapshot",
    )

    periods = [_extract_period(out.periods[i]) for i in range(out.count)]
    return DashaSnapshot(
        system=out.system,
        query_utc=_utc_from_c(out.query_utc),
        query_jd=out.query_jd,
        periods=periods,
    )


def dasha_level0(
    engine,
    lsk,
    eop,
    jd_utc_birth=None,
    location=None,
    system=0,
    ayanamsha_system=0,
    use_nutation=1,
    bhava_config=None,
    riseset_config=None,
    *,
    birth_jd=None,
    inputs=None,
):
    request = ffi.new("DhruvDashaLevel0Request *")
    request.birth = _make_dasha_birth_context(
        jd_utc_birth,
        location,
        ayanamsha_system,
        use_nutation,
        bhava_config,
        riseset_config,
        birth_jd,
        inputs,
    )[0]
    request.system = system
    handle = ffi.new("void **")
    check(
        lib.dhruv_dasha_level0(
            engine._ptr,
            eop,
            request,
            handle,
        ),
        "dasha_level0",
    )
    return _extract_period_list(handle[0])


def dasha_level0_entity(
    engine,
    lsk,
    eop,
    jd_utc_birth=None,
    location=None,
    entity=None,
    system=0,
    ayanamsha_system=0,
    use_nutation=1,
    bhava_config=None,
    riseset_config=None,
    *,
    birth_jd=None,
    inputs=None,
):
    if entity is None:
        raise ValueError("entity is required")
    entity_type, entity_index = _parse_entity(entity)
    request = ffi.new("DhruvDashaLevel0EntityRequest *")
    request.birth = _make_dasha_birth_context(
        jd_utc_birth,
        location,
        ayanamsha_system,
        use_nutation,
        bhava_config,
        riseset_config,
        birth_jd,
        inputs,
    )[0]
    request.system = system
    request.entity_type = entity_type
    request.entity_index = entity_index
    found = ffi.new("uint8_t *")
    out = ffi.new("DhruvDashaPeriod *")
    check(
        lib.dhruv_dasha_level0_entity(
            engine._ptr,
            eop,
            request,
            found,
            out,
        ),
        "dasha_level0_entity",
    )
    return _extract_period(out) if found[0] else None


def dasha_children(
    engine,
    lsk,
    eop,
    jd_utc_birth=None,
    location=None,
    parent=None,
    system=0,
    ayanamsha_system=0,
    use_nutation=1,
    bhava_config=None,
    riseset_config=None,
    variation_config=None,
    *,
    birth_jd=None,
    inputs=None,
):
    if parent is None:
        raise ValueError("parent is required")
    request = ffi.new("DhruvDashaChildrenRequest *")
    request.birth = _make_dasha_birth_context(
        jd_utc_birth,
        location,
        ayanamsha_system,
        use_nutation,
        bhava_config,
        riseset_config,
        birth_jd,
        inputs,
    )[0]
    request.system = system
    request.variation = _make_variation_config(variation_config)[0]
    request.parent = _make_period(parent)[0]
    handle = ffi.new("void **")
    check(
        lib.dhruv_dasha_children(
            engine._ptr,
            eop,
            request,
            handle,
        ),
        "dasha_children",
    )
    return _extract_period_list(handle[0])


def dasha_child_period(
    engine,
    lsk,
    eop,
    jd_utc_birth=None,
    location=None,
    parent=None,
    entity=None,
    system=0,
    ayanamsha_system=0,
    use_nutation=1,
    bhava_config=None,
    riseset_config=None,
    variation_config=None,
    *,
    birth_jd=None,
    inputs=None,
):
    if parent is None:
        raise ValueError("parent is required")
    if entity is None:
        raise ValueError("entity is required")
    entity_type, entity_index = _parse_entity(entity)
    request = ffi.new("DhruvDashaChildPeriodRequest *")
    request.birth = _make_dasha_birth_context(
        jd_utc_birth,
        location,
        ayanamsha_system,
        use_nutation,
        bhava_config,
        riseset_config,
        birth_jd,
        inputs,
    )[0]
    request.system = system
    request.variation = _make_variation_config(variation_config)[0]
    request.parent = _make_period(parent)[0]
    request.child_entity_type = entity_type
    request.child_entity_index = entity_index
    found = ffi.new("uint8_t *")
    out = ffi.new("DhruvDashaPeriod *")
    check(
        lib.dhruv_dasha_child_period(
            engine._ptr,
            eop,
            request,
            found,
            out,
        ),
        "dasha_child_period",
    )
    return _extract_period(out) if found[0] else None


def dasha_complete_level(
    engine,
    lsk,
    eop,
    jd_utc_birth=None,
    location=None,
    parent_periods=None,
    child_level=None,
    system=0,
    ayanamsha_system=0,
    use_nutation=1,
    bhava_config=None,
    riseset_config=None,
    variation_config=None,
    *,
    birth_jd=None,
    inputs=None,
):
    if parent_periods is None:
        raise ValueError("parent_periods is required")
    if child_level is None:
        raise ValueError("child_level is required")
    request = ffi.new("DhruvDashaCompleteLevelRequest *")
    request.birth = _make_dasha_birth_context(
        jd_utc_birth,
        location,
        ayanamsha_system,
        use_nutation,
        bhava_config,
        riseset_config,
        birth_jd,
        inputs,
    )[0]
    request.system = system
    request.variation = _make_variation_config(variation_config)[0]
    request.child_level = child_level
    parent_buf = ffi.new("DhruvDashaPeriod[]", len(parent_periods))
    for idx, period in enumerate(parent_periods):
        parent_buf[idx] = _make_period(period)[0]
    handle = ffi.new("void **")
    check(
        lib.dhruv_dasha_complete_level(
            engine._ptr,
            eop,
            request,
            parent_buf,
            len(parent_periods),
            handle,
        ),
        "dasha_complete_level",
    )
    return _extract_period_list(handle[0])
