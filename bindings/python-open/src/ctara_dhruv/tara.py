"""Fixed star (Tara) catalog and position computation.

Wraps the dhruv_ffi_c tara APIs for fixed star position lookup in
equatorial, ecliptic, or sidereal coordinate systems.
"""

from __future__ import annotations

from ._ffi import ffi, lib
from ._check import check
from .types import (
    EquatorialPosition,
    SphericalCoords,
    TaraComputeResult,
)


# ---------------------------------------------------------------------------
# Tara Catalog
# ---------------------------------------------------------------------------


class TaraCatalog:
    """Opaque handle for a loaded fixed-star catalog.

    Use as a context manager for automatic cleanup::

        with TaraCatalog("/path/to/stars.json") as catalog:
            result = tara_compute(catalog, tara_id=1, jd_tdb=2451545.0)
    """

    def __init__(self, path: str):
        handle = ffi.new("DhruvTaraCatalogHandle **")
        path_bytes = path.encode("utf-8")
        check(
            lib.dhruv_tara_catalog_load(path_bytes, len(path_bytes), handle),
            "tara_catalog_load",
        )
        self._handle = handle[0]

    @property
    def _ptr(self):
        if self._handle == ffi.NULL:
            raise RuntimeError("TaraCatalog is closed")
        return self._handle

    def close(self):
        """Release the catalog handle."""
        if self._handle != ffi.NULL:
            lib.dhruv_tara_catalog_free(self._handle)
            self._handle = ffi.NULL

    def __enter__(self):
        return self

    def __exit__(self, *args):
        self.close()

    def __del__(self):
        pass  # Don't rely on __del__


# ---------------------------------------------------------------------------
# Tara Compute
# ---------------------------------------------------------------------------

# Output kind constants (match DHRUV_TARA_OUTPUT_*)
TARA_OUTPUT_EQUATORIAL = 0
TARA_OUTPUT_ECLIPTIC = 1
TARA_OUTPUT_SIDEREAL = 2


def tara_compute(
    catalog,
    tara_id,
    jd_tdb,
    output_kind=TARA_OUTPUT_ECLIPTIC,
    ayanamsha_deg=0.0,
    config=None,
    earth_state=None,
):
    """Compute a fixed star position.

    Args:
        catalog: TaraCatalog instance.
        tara_id: Star identifier code (TaraId).
        jd_tdb: Epoch as JD TDB.
        output_kind: 0=equatorial, 1=ecliptic, 2=sidereal.
        ayanamsha_deg: Ayanamsha in degrees (used only for sidereal output).
        config: Optional dict with ``accuracy`` (0=astrometric, 1=apparent)
                and ``apply_parallax`` (0/1).
        earth_state: Optional dict with ``position_au`` (3 floats) and
                     ``velocity_au_day`` (3 floats) for apparent/parallax modes.

    Returns:
        TaraComputeResult dataclass.
    """
    req = ffi.new("DhruvTaraComputeRequest *")
    req.tara_id = tara_id
    req.output_kind = output_kind
    req.jd_tdb = jd_tdb
    req.ayanamsha_deg = ayanamsha_deg

    if config is not None:
        req.config.accuracy = config.get("accuracy", 0)
        req.config.apply_parallax = config.get("apply_parallax", 0)
    else:
        req.config.accuracy = 0
        req.config.apply_parallax = 0

    if earth_state is not None:
        req.earth_state_valid = 1
        pos = earth_state["position_au"]
        vel = earth_state["velocity_au_day"]
        for i in range(3):
            req.earth_state.position_au[i] = pos[i]
            req.earth_state.velocity_au_day[i] = vel[i]
    else:
        req.earth_state_valid = 0

    out = ffi.new("DhruvTaraComputeResult *")
    check(
        lib.dhruv_tara_compute_ex(catalog._ptr, req, out),
        "tara_compute_ex",
    )

    equatorial = None
    ecliptic = None
    sidereal_longitude_deg = None

    if out.output_kind == TARA_OUTPUT_EQUATORIAL:
        equatorial = EquatorialPosition(
            ra_deg=out.equatorial.ra_deg,
            dec_deg=out.equatorial.dec_deg,
            distance_au=out.equatorial.distance_au,
        )
    elif out.output_kind == TARA_OUTPUT_ECLIPTIC:
        ecliptic = SphericalCoords(
            lon_deg=out.ecliptic.lon_deg,
            lat_deg=out.ecliptic.lat_deg,
            distance_km=out.ecliptic.distance_km,
        )
    elif out.output_kind == TARA_OUTPUT_SIDEREAL:
        sidereal_longitude_deg = out.sidereal_longitude_deg

    return TaraComputeResult(
        output_kind=out.output_kind,
        equatorial=equatorial,
        ecliptic=ecliptic,
        sidereal_longitude_deg=sidereal_longitude_deg,
    )


# ---------------------------------------------------------------------------
# Galactic Center
# ---------------------------------------------------------------------------


def galactic_center_ecliptic(catalog, jd_tdb):
    """Get ecliptic position of the Galactic Center.

    Args:
        catalog: TaraCatalog instance.
        jd_tdb: Epoch as JD TDB.

    Returns:
        SphericalCoords dataclass with ecliptic lon/lat/distance.
    """
    out = ffi.new("DhruvSphericalCoords *")
    check(
        lib.dhruv_tara_galactic_center_ecliptic(catalog._ptr, jd_tdb, out),
        "tara_galactic_center_ecliptic",
    )
    return SphericalCoords(
        lon_deg=out.lon_deg,
        lat_deg=out.lat_deg,
        distance_km=out.distance_km,
    )


def propagate_position(
    ra_deg: float,
    dec_deg: float,
    parallax_mas: float,
    pm_ra_mas_yr: float,
    pm_dec_mas_yr: float,
    rv_km_s: float,
    dt_years: float,
) -> EquatorialPosition:
    """Propagate a fixed star from raw astrometric parameters."""
    out = ffi.new("DhruvEquatorialPosition *")
    check(
        lib.dhruv_tara_propagate_position(
            ra_deg,
            dec_deg,
            parallax_mas,
            pm_ra_mas_yr,
            pm_dec_mas_yr,
            rv_km_s,
            dt_years,
            out,
        ),
        "dhruv_tara_propagate_position",
    )
    return EquatorialPosition(
        ra_deg=out.ra_deg,
        dec_deg=out.dec_deg,
        distance_au=out.distance_au,
    )


def apply_aberration(direction: tuple[float, float, float], earth_vel_au_day: tuple[float, float, float]) -> tuple[float, float, float]:
    """Apply annual aberration to a unit direction vector."""
    direction_buf = ffi.new("double[3]", list(direction))
    velocity_buf = ffi.new("double[3]", list(earth_vel_au_day))
    out = ffi.new("double[3]")
    check(
        lib.dhruv_tara_apply_aberration(direction_buf, velocity_buf, out),
        "dhruv_tara_apply_aberration",
    )
    return (out[0], out[1], out[2])


def apply_light_deflection(
    direction: tuple[float, float, float],
    sun_to_observer: tuple[float, float, float],
    sun_observer_distance_au: float,
) -> tuple[float, float, float]:
    """Apply solar light deflection to a unit direction vector."""
    direction_buf = ffi.new("double[3]", list(direction))
    observer_buf = ffi.new("double[3]", list(sun_to_observer))
    out = ffi.new("double[3]")
    check(
        lib.dhruv_tara_apply_light_deflection(
            direction_buf, observer_buf, sun_observer_distance_au, out
        ),
        "dhruv_tara_apply_light_deflection",
    )
    return (out[0], out[1], out[2])


def galactic_anticenter_icrs() -> tuple[float, float, float]:
    """Return the galactic anticenter ICRS unit direction vector."""
    out = ffi.new("double[3]")
    check(lib.dhruv_tara_galactic_anticenter_icrs(out), "dhruv_tara_galactic_anticenter_icrs")
    return (out[0], out[1], out[2])
