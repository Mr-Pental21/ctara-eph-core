"""Dasha (planetary period) hierarchy and snapshot computation.

Wraps the dhruv_ffi_c dasha APIs for birth-chart period analysis.
"""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import Optional

from ._ffi import ffi, lib
from ._check import check
from .types import DashaPeriod, DashaSnapshot


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
        start_jd=p.start_jd,
        end_jd=p.end_jd,
        level=p.level,
        order=p.order,
        parent_idx=p.parent_idx,
    )


# ---------------------------------------------------------------------------
# Dasha selection config
# ---------------------------------------------------------------------------


def dasha_selection_config_default():
    """Return a default DhruvDashaSelectionConfig C struct.

    count=0 (no systems selected), max_level=2, no snapshot.
    """
    return lib.dhruv_dasha_selection_config_default()


# ---------------------------------------------------------------------------
# Dasha hierarchy
# ---------------------------------------------------------------------------


def dasha_hierarchy(
    engine,
    lsk,
    eop,
    jd_utc_birth,
    location,
    system=0,
    max_level=2,
    ayanamsha_system=0,
    use_nutation=1,
    bhava_config=None,
    riseset_config=None,
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
    birth_utc = _make_utc(jd_utc_birth)
    loc = _make_location(location)
    bhava_cfg = _make_bhava_config(bhava_config)
    rs_cfg = _make_riseset_config(riseset_config)

    handle = ffi.new("void **")
    check(
        lib.dhruv_dasha_hierarchy_utc(
            engine._ptr,
            eop,
            birth_utc,
            loc,
            bhava_cfg,
            rs_cfg,
            ayanamsha_system,
            use_nutation,
            system,
            max_level,
            handle,
        ),
        "dasha_hierarchy_utc",
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
    jd_utc_birth,
    jd_utc_query,
    location,
    system=0,
    max_level=2,
    ayanamsha_system=0,
    use_nutation=1,
    bhava_config=None,
    riseset_config=None,
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
    birth_utc = _make_utc(jd_utc_birth)
    query_utc = _make_utc(jd_utc_query)
    loc = _make_location(location)
    bhava_cfg = _make_bhava_config(bhava_config)
    rs_cfg = _make_riseset_config(riseset_config)

    out = ffi.new("DhruvDashaSnapshot *")
    check(
        lib.dhruv_dasha_snapshot_utc(
            engine._ptr,
            eop,
            birth_utc,
            query_utc,
            loc,
            bhava_cfg,
            rs_cfg,
            ayanamsha_system,
            use_nutation,
            system,
            max_level,
            out,
        ),
        "dasha_snapshot_utc",
    )

    periods = [_extract_period(out.periods[i]) for i in range(out.count)]
    return DashaSnapshot(
        system=out.system,
        query_jd=out.query_jd,
        periods=periods,
    )
