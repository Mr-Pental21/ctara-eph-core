"""ctara-dhruv: Python bindings for the ctara-dhruv-core ephemeris engine.

Usage::

    import ctara_dhruv as cd

    with cd.Engine(["de442s.bsp"], "naif0012.tls") as eng:
        result = cd.query(
            eng._ptr,
            cd.QueryRequest(
                target=cd.Body.MARS,
                observer=cd.Body.SSB,
                epoch_tdb_jd=2451545.0,
                output_mode=cd.QUERY_OUTPUT_CARTESIAN,
            ),
        )
        print(result.state.x, result.state.y, result.state.z)
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
    PrecessionModel,
    GrahaLongitudeKind,
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
    QUERY_OUTPUT_BOTH,
    QUERY_OUTPUT_CARTESIAN,
    QUERY_OUTPUT_SPHERICAL,
    DELTA_T_MODEL_LEGACY_ESPENAK_MEEUS_2006,
    DELTA_T_MODEL_SMH2016_WITH_PRE720_QUADRATIC,
    FUTURE_DELTA_T_TRANSITION_BRIDGE_FROM_MODERN_ENDPOINT,
    FUTURE_DELTA_T_TRANSITION_LEGACY_TT_UTC_BLEND,
    QUERY_TIME_JD_TDB,
    QUERY_TIME_UTC,
    SMH_FUTURE_FAMILY_ADDENDUM_2020_PIECEWISE,
    SMH_FUTURE_FAMILY_CONSTANT_C_MINUS15P32,
    SMH_FUTURE_FAMILY_CONSTANT_C_MINUS17P52,
    SMH_FUTURE_FAMILY_CONSTANT_C_MINUS20,
    SMH_FUTURE_FAMILY_STEPHENSON_1997,
    SMH_FUTURE_FAMILY_STEPHENSON_2016,
    TIME_POLICY_HYBRID_DELTA_T,
    TIME_POLICY_STRICT_LSK,
    TIME_WARNING_DELTA_T_MODEL_USED,
    TIME_WARNING_EOP_FUTURE_FROZEN,
    TIME_WARNING_EOP_PRE_RANGE_FALLBACK,
    TIME_WARNING_LSK_FUTURE_FROZEN,
    TIME_WARNING_LSK_PRE_RANGE_FALLBACK,
    TT_UTC_SOURCE_DELTA_T_MODEL,
    TT_UTC_SOURCE_LSK_DELTA_AT,
    QueryRequest,
    QueryResult,
    StateVector,
    SphericalCoords,
    SphericalState,
    TimeConversionOptions,
    TimeDiagnostics,
    TimePolicy,
    TimeWarning,
    UtcTime,
    UtcToTdbRequest,
    UtcToTdbResult,
    GrahaLongitudesConfig,
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
    MovingOsculatingApogeeEntry,
    MovingOsculatingApogees,
    AmshaVariationCatalog,
    AmshaVariationInfo,
    CharakarakaEntry,
    CharakarakaResult,
    DashaPeriod,
    DashaSnapshot,
)

# Errors
from ctara_dhruv._check import DhruvError

# Ephemeris
from ctara_dhruv.ephemeris import (
    query,
    body_ecliptic_lon_lat,
    cartesian_to_spherical,
)

# Time
from ctara_dhruv.time import utc_to_jd_tdb, jd_tdb_to_utc, nutation

# Ayanamsha
from ctara_dhruv.ayanamsha import ayanamsha, system_count, reference_plane_default

# Tara
from ctara_dhruv.tara import TaraCatalog

# Amsha
from ctara_dhruv.amsha import (
    amsha_longitude,
    amsha_longitudes,
    amsha_rashi_info,
    amsha_chart_for_date,
    amsha_variations,
    amsha_variations_many,
)

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
    "StationType", "MaxSpeedType", "DashaSystem", "ReferencePlane", "PrecessionModel",
    "GrahaLongitudeKind",
    "SearchQueryMode", "GrahanKind", "MotionKind", "LunarPhaseKind",
    "SankrantiTargetKind", "ChandraGrahanType", "SuryaGrahanType",
    "CharakarakaScheme", "CharakarakaRole", "TaraOutputKind",
    "QUERY_TIME_JD_TDB", "QUERY_TIME_UTC",
    "TIME_POLICY_STRICT_LSK", "TIME_POLICY_HYBRID_DELTA_T",
    "DELTA_T_MODEL_LEGACY_ESPENAK_MEEUS_2006", "DELTA_T_MODEL_SMH2016_WITH_PRE720_QUADRATIC",
    "FUTURE_DELTA_T_TRANSITION_LEGACY_TT_UTC_BLEND", "FUTURE_DELTA_T_TRANSITION_BRIDGE_FROM_MODERN_ENDPOINT",
    "SMH_FUTURE_FAMILY_ADDENDUM_2020_PIECEWISE", "SMH_FUTURE_FAMILY_CONSTANT_C_MINUS20",
    "SMH_FUTURE_FAMILY_CONSTANT_C_MINUS17P52", "SMH_FUTURE_FAMILY_CONSTANT_C_MINUS15P32",
    "SMH_FUTURE_FAMILY_STEPHENSON_1997", "SMH_FUTURE_FAMILY_STEPHENSON_2016",
    "TT_UTC_SOURCE_LSK_DELTA_AT", "TT_UTC_SOURCE_DELTA_T_MODEL",
    "TIME_WARNING_LSK_FUTURE_FROZEN", "TIME_WARNING_LSK_PRE_RANGE_FALLBACK",
    "TIME_WARNING_EOP_FUTURE_FROZEN", "TIME_WARNING_EOP_PRE_RANGE_FALLBACK",
    "TIME_WARNING_DELTA_T_MODEL_USED",
    "QUERY_OUTPUT_CARTESIAN", "QUERY_OUTPUT_SPHERICAL", "QUERY_OUTPUT_BOTH",
    # Types
    "QueryRequest", "QueryResult",
    "StateVector", "SphericalCoords", "SphericalState", "UtcTime",
    "TimeConversionOptions", "TimePolicy", "TimeWarning", "TimeDiagnostics",
    "UtcToTdbRequest", "UtcToTdbResult", "GrahaLongitudesConfig",
    "GeoLocation", "Dms", "RashiInfo", "NakshatraInfo", "Nakshatra28Info",
    "BhavaEntry", "BhavaResult", "ConjunctionEvent",
    "ChandraGrahanResult", "SuryaGrahanResult",
    "StationaryEvent", "MaxSpeedEvent",
    "LunarPhaseEvent", "SankrantiEvent",
    "GrahaEntry", "GrahaPositions", "MovingOsculatingApogeeEntry", "MovingOsculatingApogees",
    "CharakarakaEntry", "CharakarakaResult", "DashaPeriod",
    "DashaSnapshot", "AmshaVariationCatalog", "AmshaVariationInfo",
    # Errors
    "DhruvError",
    # Functions
    "query", "body_ecliptic_lon_lat",
    "cartesian_to_spherical",
    "utc_to_jd_tdb", "jd_tdb_to_utc", "nutation",
    "ayanamsha", "system_count", "reference_plane_default",
    "amsha_longitude", "amsha_longitudes", "amsha_rashi_info", "amsha_chart_for_date",
    "amsha_variations", "amsha_variations_many",
    "TaraCatalog",
    "DashaLevel", "DashaHierarchy",
    "dasha_selection_config_default", "dasha_variation_config_default",
    "dasha_hierarchy", "dasha_snapshot",
    "dasha_level0", "dasha_level0_entity",
    "dasha_children", "dasha_child_period", "dasha_complete_level",
]
