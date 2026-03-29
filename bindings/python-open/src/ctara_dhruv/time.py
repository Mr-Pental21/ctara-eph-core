"""Time conversions: UTC/TDB, nutation, local noon."""

from __future__ import annotations

from ._ffi import ffi, lib
from ._check import check
from .types import (
    TimeConversionOptions,
    TimeDiagnostics,
    TimePolicy,
    TimeWarning,
    UtcTime,
    UtcToTdbRequest,
    UtcToTdbResult,
)


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


def _time_policy_struct(policy: TimePolicy):
    opts = ffi.new("DhruvTimeConversionOptions *")
    opts.warn_on_fallback = 1 if policy.options.warn_on_fallback else 0
    opts.delta_t_model = policy.options.delta_t_model
    opts.freeze_future_dut1 = 1 if policy.options.freeze_future_dut1 else 0
    opts.pre_range_dut1 = policy.options.pre_range_dut1
    opts.future_delta_t_transition = policy.options.future_delta_t_transition
    opts.future_transition_years = policy.options.future_transition_years
    opts.smh_future_family = policy.options.smh_future_family

    out = ffi.new("DhruvTimePolicy *")
    out.mode = policy.mode
    out.options = opts[0]
    return out


def _decode_time_warning(value) -> TimeWarning:
    return TimeWarning(
        kind=value.kind,
        utc_seconds=value.utc_seconds,
        first_entry_utc_seconds=value.first_entry_utc_seconds,
        last_entry_utc_seconds=value.last_entry_utc_seconds,
        used_delta_at_seconds=value.used_delta_at_seconds,
        mjd=value.mjd,
        first_entry_mjd=value.first_entry_mjd,
        last_entry_mjd=value.last_entry_mjd,
        used_dut1_seconds=value.used_dut1_seconds,
        delta_t_model=value.delta_t_model,
        delta_t_segment=value.delta_t_segment,
    )


def _decode_time_diagnostics(value) -> TimeDiagnostics:
    warnings = [_decode_time_warning(value.warnings[i]) for i in range(value.warning_count)]
    return TimeDiagnostics(
        source=value.source,
        tt_minus_utc_s=value.tt_minus_utc_s,
        warnings=warnings,
    )


def utc_to_jd_tdb(lsk_handle, request: UtcToTdbRequest, eop_handle=None) -> UtcToTdbResult:
    """Convert UTC calendar date to JD TDB using typed policy and diagnostics."""
    out = ffi.new("DhruvUtcToTdbResult *")
    req = ffi.new("DhruvUtcToTdbRequest *")
    req.utc = _utc_struct(request.utc)[0]
    req.policy = _time_policy_struct(request.time_policy)[0]
    eop_ptr = ffi.NULL if eop_handle is None else eop_handle
    check(
        lib.dhruv_utc_to_tdb_jd(lsk_handle, eop_ptr, req, out),
        "utc_to_tdb_jd",
    )
    return UtcToTdbResult(
        jd_tdb=out.jd_tdb,
        diagnostics=_decode_time_diagnostics(out.diagnostics),
    )


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
