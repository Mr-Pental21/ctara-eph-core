"""Position queries: Cartesian, spherical, and ecliptic coordinates."""

from __future__ import annotations

from ._ffi import ffi, lib
from ._check import check
from .types import SphericalCoords, SphericalState, StateVector, UtcTime

_DHRUV_PATH_CAPACITY = 512
_DHRUV_MAX_SPK_PATHS = 8


def _build_engine_config(spk_paths: list[str], lsk_path: str | None):
    """Build a DhruvEngineConfig from Python arguments."""
    cfg = ffi.new("DhruvEngineConfig *")
    cfg.spk_path_count = len(spk_paths)
    for i, p in enumerate(spk_paths):
        p_bytes = p.encode("utf-8")
        ffi.memmove(cfg.spk_paths_utf8[i], p_bytes, len(p_bytes))
    if lsk_path:
        lsk_bytes = lsk_path.encode("utf-8")
        ffi.memmove(cfg.lsk_path_utf8, lsk_bytes, len(lsk_bytes))
    cfg.cache_capacity = 256
    cfg.strict_validation = 1
    return cfg


def _utc_struct(utc: UtcTime):
    """Build a DhruvUtcTime from a UtcTime dataclass."""
    u = ffi.new("DhruvUtcTime *")
    u.year = utc.year
    u.month = utc.month
    u.day = utc.day
    u.hour = utc.hour
    u.minute = utc.minute
    u.second = utc.second
    return u


def _read_state_vector(sv) -> StateVector:
    return StateVector(
        x=sv.position_km[0],
        y=sv.position_km[1],
        z=sv.position_km[2],
        vx=sv.velocity_km_s[0],
        vy=sv.velocity_km_s[1],
        vz=sv.velocity_km_s[2],
    )


def _read_spherical_state(ss) -> SphericalState:
    return SphericalState(
        lon_deg=ss.lon_deg,
        lat_deg=ss.lat_deg,
        distance_km=ss.distance_km,
        lon_speed=ss.lon_speed,
        lat_speed=ss.lat_speed,
        distance_speed=ss.distance_speed,
    )


def query_state(
    engine_handle,
    target: int,
    observer: int,
    jd_tdb: float,
    frame: int = 0,
) -> StateVector:
    """Query the engine for a Cartesian state vector at a given epoch.

    Args:
        engine_handle: DhruvEngineHandle pointer (from Engine._ptr).
        target: NAIF body code of the target.
        observer: NAIF body code of the observer.
        jd_tdb: Julian Date in TDB.
        frame: Frame code (0=J2000/ICRF, 1=ecliptic J2000).

    Returns:
        StateVector with position (km) and velocity (km/s).
    """
    q = ffi.new("DhruvQuery *")
    q.target = target
    q.observer = observer
    q.frame = frame
    q.epoch_tdb_jd = jd_tdb

    out = ffi.new("DhruvStateVector *")
    check(lib.dhruv_engine_query(engine_handle, q, out), "engine_query")
    return _read_state_vector(out)


def query_once(
    spk_paths: list[str],
    lsk_path: str | None,
    target: int,
    observer: int,
    jd_tdb: float,
    frame: int = 0,
) -> StateVector:
    """One-shot query: creates engine, queries, and tears down internally.

    Args:
        spk_paths: List of SPK kernel file paths.
        lsk_path: LSK file path (optional).
        target: NAIF body code of the target.
        observer: NAIF body code of the observer.
        jd_tdb: Julian Date in TDB.
        frame: Frame code (0=J2000/ICRF, 1=ecliptic J2000).

    Returns:
        StateVector with position (km) and velocity (km/s).
    """
    cfg = _build_engine_config(spk_paths, lsk_path)
    q = ffi.new("DhruvQuery *")
    q.target = target
    q.observer = observer
    q.frame = frame
    q.epoch_tdb_jd = jd_tdb

    out = ffi.new("DhruvStateVector *")
    check(lib.dhruv_query_once(cfg, q, out), "query_once")
    return _read_state_vector(out)


def query_utc(
    engine_handle,
    target: int,
    observer: int,
    utc_time: UtcTime,
    frame: int = 0,
) -> SphericalState:
    """Query from a UtcTime dataclass, returns spherical state.

    Args:
        engine_handle: DhruvEngineHandle pointer.
        target: NAIF body code of the target.
        observer: NAIF body code of the observer.
        utc_time: UtcTime dataclass with calendar components.
        frame: Frame code (0=J2000/ICRF, 1=ecliptic J2000).

    Returns:
        SphericalState with lon/lat (deg), distance (km), and rates.
    """
    utc = _utc_struct(utc_time)
    out = ffi.new("DhruvSphericalState *")
    check(
        lib.dhruv_query_utc(engine_handle, target, observer, frame, utc, out),
        "query_utc",
    )
    return _read_spherical_state(out)


def query_utc_spherical(
    engine_handle,
    target: int,
    observer: int,
    frame: int,
    year: int,
    month: int,
    day: int,
    hour: int = 0,
    minute: int = 0,
    sec: float = 0.0,
) -> SphericalState:
    """Query from UTC calendar components, returns spherical state.

    Args:
        engine_handle: DhruvEngineHandle pointer.
        target: NAIF body code of the target.
        observer: NAIF body code of the observer.
        frame: Frame code.
        year, month, day, hour, minute, sec: UTC calendar components.

    Returns:
        SphericalState with lon/lat (deg), distance (km), and rates.
    """
    out = ffi.new("DhruvSphericalState *")
    check(
        lib.dhruv_query_utc_spherical(
            engine_handle, target, observer, frame,
            year, month, day, hour, minute, sec, out,
        ),
        "query_utc_spherical",
    )
    return _read_spherical_state(out)


def body_ecliptic_lon_lat(
    engine_handle, body_code: int, jd_tdb: float
) -> tuple[float, float]:
    """Compute ecliptic longitude and latitude for a body.

    Args:
        engine_handle: DhruvEngineHandle pointer.
        body_code: NAIF body code.
        jd_tdb: Julian Date in TDB.

    Returns:
        Tuple of (lon_deg, lat_deg).
    """
    out_lon = ffi.new("double *")
    out_lat = ffi.new("double *")
    check(
        lib.dhruv_body_ecliptic_lon_lat(
            engine_handle, body_code, jd_tdb, out_lon, out_lat
        ),
        "body_ecliptic_lon_lat",
    )
    return (out_lon[0], out_lat[0])


def cartesian_to_spherical(x: float, y: float, z: float) -> SphericalCoords:
    """Convert Cartesian (km) to spherical coordinates. Pure math.

    Args:
        x, y, z: Cartesian position in km.

    Returns:
        SphericalCoords with lon_deg, lat_deg, distance_km.
    """
    pos = ffi.new("double[3]", [x, y, z])
    out = ffi.new("DhruvSphericalCoords *")
    check(
        lib.dhruv_cartesian_to_spherical(pos, out),
        "cartesian_to_spherical",
    )
    return SphericalCoords(
        lon_deg=out.lon_deg,
        lat_deg=out.lat_deg,
        distance_km=out.distance_km,
    )
