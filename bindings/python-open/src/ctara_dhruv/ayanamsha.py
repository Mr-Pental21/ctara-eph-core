"""Ayanamsha computation."""

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


def ayanamsha(
    lsk_handle,
    system: int,
    *,
    jd_tdb: float | None = None,
    utc: UtcTime | None = None,
    mode: int = 2,
    use_nutation: int = 1,
    delta_psi: float = 0.0,
    catalog=None,
) -> float:
    """Compute ayanamsha using the unified request API.

    Provide either ``jd_tdb`` (TDB Julian Date) or ``utc`` (UtcTime),
    but not both. When ``utc`` is provided, ``lsk_handle`` must be valid.

    Args:
        lsk_handle: DhruvLskHandle pointer (required for UTC time input).
        system: Ayanamsha system code (0-19).
        jd_tdb: Julian Date in TDB (when using TDB time input).
        utc: UtcTime dataclass (when using UTC time input).
        mode: Computation mode (0=MEAN, 1=TRUE, 2=UNIFIED).
        use_nutation: For UNIFIED mode: 0=false, 1=true.
        delta_psi: For TRUE mode: nutation in longitude (arcsec).
        catalog: DhruvTaraCatalogHandle pointer (optional).

    Returns:
        Ayanamsha value in degrees.
    """
    req = ffi.new("DhruvAyanamshaComputeRequest *")
    req.system_code = system
    req.mode = mode
    req.use_nutation = use_nutation
    req.delta_psi_arcsec = delta_psi

    if utc is not None:
        req.time_kind = 1  # DHRUV_AYANAMSHA_TIME_UTC
        utc_c = _utc_struct(utc)
        req.utc = utc_c[0]
    else:
        req.time_kind = 0  # DHRUV_AYANAMSHA_TIME_JD_TDB
        req.jd_tdb = jd_tdb if jd_tdb is not None else 0.0

    catalog_ptr = catalog if catalog is not None else ffi.NULL
    out = ffi.new("double *")
    check(
        lib.dhruv_ayanamsha_compute_ex(lsk_handle, req, catalog_ptr, out),
        "ayanamsha_compute_ex",
    )
    return out[0]


def system_count() -> int:
    """Return the number of supported ayanamsha systems."""
    return lib.dhruv_ayanamsha_system_count()


def reference_plane_default(system_code: int) -> int:
    """Return the default reference plane for an ayanamsha system.

    Args:
        system_code: Ayanamsha system code (0-19).

    Returns:
        0=Ecliptic, 1=Invariable, -1=invalid code.
    """
    return lib.dhruv_reference_plane_default(system_code)


def sidereal_sum_at(
    engine_handle, jd_tdb: float, config=None
) -> float:
    """Compute the sidereal Sun+Moon longitude sum at a given epoch.

    Args:
        engine_handle: DhruvEngineHandle pointer.
        jd_tdb: Julian Date in TDB.
        config: DhruvSankrantiConfig pointer (optional, uses defaults if NULL).

    Returns:
        Sidereal sum in degrees.
    """
    config_ptr = config if config is not None else ffi.NULL
    out = ffi.new("double *")
    check(
        lib.dhruv_sidereal_sum_at(engine_handle, jd_tdb, config_ptr, out),
        "sidereal_sum_at",
    )
    return out[0]
