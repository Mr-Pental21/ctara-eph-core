"""ctara-dhruv: Python bindings for the ctara-dhruv-core ephemeris engine.

Usage::

    import ctara_dhruv as cd

    with cd.Engine(["de442s.bsp"], "naif0012.tls") as eng:
        state = cd.query_state(eng._ptr, target=cd.Body.MARS, observer=cd.Body.SSB, jd_tdb=2451545.0)
        print(state.x, state.y, state.z)
"""

# Engine lifecycle
from ctara_dhruv.engine import Engine, init, engine, lsk, eop

# Enums
from ctara_dhruv.enums import (
    Body,
    DhruvStatus,
    AyanamshaSystem,
    AyanamshaMode,
    BhavaSystem,
    Graha,
    SunLimb,
    RiseSetEvent,
    RiseSetResultType,
    StationType,
    MaxSpeedType,
    DashaSystem,
    ReferencePlane,
    SearchQueryMode,
    GrahanKind,
    MotionKind,
    LunarPhaseKind,
    SankrantiTargetKind,
    ChandraGrahanType,
    SuryaGrahanType,
    CharakarakaScheme,
    CharakarakaRole,
    TaraOutputKind,
)

# Core types
from ctara_dhruv.types import (
    StateVector,
    SphericalCoords,
    SphericalState,
    UtcTime,
    GeoLocation,
    Dms,
    RashiInfo,
    NakshatraInfo,
    Nakshatra28Info,
    BhavaEntry,
    BhavaResult,
    ConjunctionEvent,
    ChandraGrahanResult,
    SuryaGrahanResult,
    StationaryEvent,
    MaxSpeedEvent,
    LunarPhaseEvent,
    SankrantiEvent,
    GrahaEntry,
    GrahaPositions,
    CharakarakaEntry,
    CharakarakaResult,
    DashaPeriod,
    DashaSnapshot,
)

# Errors
from ctara_dhruv._check import DhruvError

# Ephemeris
from ctara_dhruv.ephemeris import (
    query_state,
    query_utc_spherical,
    body_ecliptic_lon_lat,
    cartesian_to_spherical,
)

# Time
from ctara_dhruv.time import utc_to_jd_tdb, jd_tdb_to_utc, nutation

# Ayanamsha
from ctara_dhruv.ayanamsha import ayanamsha, system_count, reference_plane_default

# Tara
from ctara_dhruv.tara import TaraCatalog

# Dasha
from ctara_dhruv.dasha import (
    DashaLevel,
    DashaHierarchy,
    dasha_selection_config_default,
    dasha_variation_config_default,
    dasha_hierarchy,
    dasha_snapshot,
    dasha_level0,
    dasha_level0_entity,
    dasha_children,
    dasha_child_period,
    dasha_complete_level,
)

__all__ = [
    # Engine
    "Engine", "init", "engine", "lsk", "eop",
    # Enums
    "Body", "DhruvStatus", "AyanamshaSystem", "AyanamshaMode",
    "BhavaSystem", "Graha",
    "SunLimb", "RiseSetEvent", "RiseSetResultType",
    "StationType", "MaxSpeedType", "DashaSystem", "ReferencePlane",
    "SearchQueryMode", "GrahanKind", "MotionKind", "LunarPhaseKind",
    "SankrantiTargetKind", "ChandraGrahanType", "SuryaGrahanType",
    "CharakarakaScheme", "CharakarakaRole", "TaraOutputKind",
    # Types
    "StateVector", "SphericalCoords", "SphericalState", "UtcTime",
    "GeoLocation", "Dms", "RashiInfo", "NakshatraInfo", "Nakshatra28Info",
    "BhavaEntry", "BhavaResult", "ConjunctionEvent",
    "ChandraGrahanResult", "SuryaGrahanResult",
    "StationaryEvent", "MaxSpeedEvent",
    "LunarPhaseEvent", "SankrantiEvent",
    "GrahaEntry", "GrahaPositions", "CharakarakaEntry", "CharakarakaResult", "DashaPeriod",
    "DashaSnapshot",
    # Errors
    "DhruvError",
    # Functions
    "query_state", "query_utc_spherical", "body_ecliptic_lon_lat",
    "cartesian_to_spherical",
    "utc_to_jd_tdb", "jd_tdb_to_utc", "nutation",
    "ayanamsha", "system_count", "reference_plane_default",
    "TaraCatalog",
    "DashaLevel", "DashaHierarchy",
    "dasha_selection_config_default", "dasha_variation_config_default",
    "dasha_hierarchy", "dasha_snapshot",
    "dasha_level0", "dasha_level0_entity",
    "dasha_children", "dasha_child_period", "dasha_complete_level",
]
