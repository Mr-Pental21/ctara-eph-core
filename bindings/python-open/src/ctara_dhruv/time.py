"""Time conversions: UTC/TDB, nutation, local noon."""

from __future__ import annotations

from ._ffi import ffi, lib
from ._check import check
from .types import UtcTime


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


def utc_to_jd_tdb(
    lsk_handle,
    year: int,
    month: int,
    day: int,
    hour: int = 0,
    minute: int = 0,
    sec: float = 0.0,
) -> float:
    """Convert UTC calendar date to JD TDB using a standalone LSK handle.

    Args:
        lsk_handle: DhruvLskHandle pointer.
        year, month, day, hour, minute, sec: UTC calendar components.

    Returns:
        Julian Date in TDB.
    """
    out = ffi.new("double *")
    check(
        lib.dhruv_utc_to_tdb_jd(lsk_handle, year, month, day, hour, minute, sec, out),
        "utc_to_tdb_jd",
    )
    return out[0]


def jd_tdb_to_utc(lsk_handle, jd_tdb: float) -> UtcTime:
    """Convert JD TDB to broken-down UTC calendar time.

    Args:
        lsk_handle: DhruvLskHandle pointer.
        jd_tdb: Julian Date in TDB.

    Returns:
        UtcTime dataclass.
    """
    out = ffi.new("DhruvUtcTime *")
    check(lib.dhruv_jd_tdb_to_utc(lsk_handle, jd_tdb, out), "jd_tdb_to_utc")
    return UtcTime(
        year=out.year,
        month=out.month,
        day=out.day,
        hour=out.hour,
        minute=out.minute,
        second=out.second,
    )


def nutation(jd_tdb: float) -> tuple[float, float]:
    """Compute IAU 2000B nutation. Pure math.

    Args:
        jd_tdb: Julian Date in TDB.

    Returns:
        Tuple of (dpsi_arcsec, deps_arcsec).
    """
    dpsi = ffi.new("double *")
    deps = ffi.new("double *")
    check(lib.dhruv_nutation_iau2000b(jd_tdb, dpsi, deps), "nutation_iau2000b")
    return (dpsi[0], deps[0])


def nutation_utc(lsk_handle, utc: UtcTime) -> tuple[float, float]:
    """Compute IAU 2000B nutation from UTC time.

    Args:
        lsk_handle: DhruvLskHandle pointer.
        utc: UtcTime dataclass.

    Returns:
        Tuple of (dpsi_arcsec, deps_arcsec).
    """
    utc_c = _utc_struct(utc)
    dpsi = ffi.new("double *")
    deps = ffi.new("double *")
    check(
        lib.dhruv_nutation_iau2000b_utc(lsk_handle, utc_c, dpsi, deps),
        "nutation_iau2000b_utc",
    )
    return (dpsi[0], deps[0])


def approximate_local_noon_jd(
    jd_ut_midnight: float, longitude_deg: float
) -> float:
    """Approximate local noon JD from 0h UT JD and longitude. Pure math.

    Args:
        jd_ut_midnight: Julian Date at 0h UT.
        longitude_deg: Observer longitude in degrees (east positive).

    Returns:
        Approximate JD of local noon.
    """
    return lib.dhruv_approximate_local_noon_jd(jd_ut_midnight, longitude_deg)
