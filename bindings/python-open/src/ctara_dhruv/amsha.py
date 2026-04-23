"""Amsha (divisional chart) computation.

Pure-math amsha transforms and engine-backed amsha chart orchestration.
"""

from __future__ import annotations

from ._ffi import ffi, lib
from ._check import check
from .types import (
    AmshaChart,
    AmshaEntry,
    AmshaVariationCatalog,
    AmshaVariationInfo,
    Dms,
    RashiInfo,
)


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
    cfg.use_rashi_bhava_for_bala_avastha = bhava_config.get(
        "use_rashi_bhava_for_bala_avastha", 1
    )
    cfg.include_node_aspects_for_drik_bala = bhava_config.get(
        "include_node_aspects_for_drik_bala", 0
    )
    cfg.divide_guru_buddh_drishti_by_4_for_drik_bala = bhava_config.get(
        "divide_guru_buddh_drishti_by_4_for_drik_bala", 1
    )
    cfg.chandra_benefic_rule = bhava_config.get("chandra_benefic_rule", 0)
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


def _extract_amsha_entry(e):
    return AmshaEntry(
        sidereal_longitude=e.sidereal_longitude,
        rashi_index=e.rashi_index,
        dms_degrees=e.dms_degrees,
        dms_minutes=e.dms_minutes,
        dms_seconds=e.dms_seconds,
        degrees_in_rashi=e.degrees_in_rashi,
    )


def _decode_c_string(buf):
    return ffi.string(buf).decode("utf-8")


def _extract_amsha_variation_catalog(catalog):
    variations = []
    for i in range(catalog.count):
        info = catalog.variations[i]
        variations.append(
            AmshaVariationInfo(
                amsha_code=info.amsha_code,
                variation_code=info.variation_code,
                name=_decode_c_string(info.name),
                label=_decode_c_string(info.label),
                is_default=bool(info.is_default),
                description=_decode_c_string(info.description),
            )
        )
    return AmshaVariationCatalog(
        amsha_code=catalog.amsha_code,
        default_variation_code=catalog.default_variation_code,
        variations=variations,
    )


# ---------------------------------------------------------------------------
# Pure math: single longitude
# ---------------------------------------------------------------------------


def amsha_longitude(sidereal_lon_deg, amsha_number, variation=0):
    """Compute amsha longitude for a single sidereal longitude.

    Args:
        sidereal_lon_deg: Sidereal longitude in degrees [0, 360).
        amsha_number: D-number (e.g. 9 for Navamsha, 12 for Dwadashamsha).
        variation: amsha-specific variation code; 0=default for that amsha.

    Returns:
        Amsha longitude in degrees [0, 360).
    """
    out = ffi.new("double *")
    check(
        lib.dhruv_amsha_longitude(sidereal_lon_deg, amsha_number, variation, out),
        "amsha_longitude",
    )
    return out[0]


# ---------------------------------------------------------------------------
# Pure math: batch longitudes
# ---------------------------------------------------------------------------


def amsha_longitudes(sidereal_lons, amsha_codes, variation_codes=None):
    """Compute amsha longitudes for multiple points and/or multiple amshas.

    This is a low-level batch function. For each index i, it transforms
    sidereal_lons[0] through amsha_codes[i] (the FFI function takes a single
    longitude and array of codes).

    For 9 grahas + lagna, call once per amsha with individual longitudes,
    or use amsha_chart_for_date for the full orchestration.

    Args:
        sidereal_lons: Single sidereal longitude in degrees (scalar float).
        amsha_codes: List of D-numbers (u16).
        variation_codes: Optional list of amsha-specific variation codes (u8),
                        same length as amsha_codes. None = all default.

    Returns:
        List of amsha longitudes (one per amsha_code).
    """
    count = len(amsha_codes)
    c_codes = ffi.new("uint16_t[]", count)
    for i, code in enumerate(amsha_codes):
        c_codes[i] = code

    c_variations = ffi.NULL
    if variation_codes is not None:
        c_variations = ffi.new("uint8_t[]", count)
        for i, vc in enumerate(variation_codes):
            c_variations[i] = vc

    c_out = ffi.new("double[]", count)
    check(
        lib.dhruv_amsha_longitudes(sidereal_lons, c_codes, c_variations, count, c_out),
        "amsha_longitudes",
    )
    return [c_out[i] for i in range(count)]


# ---------------------------------------------------------------------------
# Pure math: rashi info
# ---------------------------------------------------------------------------


def amsha_rashi_info(sidereal_lon_deg, amsha_number, variation=0):
    """Get rashi info for an amsha longitude.

    Args:
        sidereal_lon_deg: Sidereal longitude in degrees.
        amsha_number: D-number.
        variation: Amsha-specific variation code.

    Returns:
        RashiInfo dataclass.
    """
    out = ffi.new("DhruvRashiInfo *")
    check(
        lib.dhruv_amsha_rashi_info(sidereal_lon_deg, amsha_number, variation, out),
        "amsha_rashi_info",
    )
    return RashiInfo(
        rashi_index=out.rashi_index,
        degrees_in_rashi=out.degrees_in_rashi,
        dms=Dms(
            degrees=out.dms.degrees,
            minutes=out.dms.minutes,
            seconds=out.dms.seconds,
        ),
    )


# ---------------------------------------------------------------------------
# Orchestration: amsha chart for date
# ---------------------------------------------------------------------------


def amsha_chart_for_date(
    engine,
    lsk,
    eop,
    jd_utc,
    location,
    amsha_code,
    variation=0,
    ayanamsha_system=0,
    use_nutation=1,
    scope=None,
    bhava_config=None,
    riseset_config=None,
):
    """Compute a single amsha (divisional) chart for a date and location.

    Args:
        engine: Engine instance.
        lsk: LSK handle.
        eop: EOP handle.
        jd_utc: UTC time tuple (year, month, day[, hour, min, sec]).
        location: (lat, lon[, alt]) tuple.
        amsha_code: D-number (e.g. 9 for Navamsha).
        variation: Amsha-specific variation code (0=default for that amsha).
        ayanamsha_system: Ayanamsha system code.
        use_nutation: 1=yes, 0=no.
        scope: Optional dict with include_bhava_cusps, include_arudha_padas,
               include_upagrahas, include_sphutas, include_special_lagnas (u8).
        bhava_config: Optional bhava config dict.
        riseset_config: Optional riseset config dict.

    Returns:
        AmshaChart dataclass.
    """
    utc = _make_utc(jd_utc)
    loc = _make_location(location)
    bhava_cfg = _make_bhava_config(bhava_config)
    rs_cfg = _make_riseset_config(riseset_config)

    scope_c = ffi.new("DhruvAmshaChartScope *")
    if scope is not None:
        scope_c.include_bhava_cusps = scope.get("include_bhava_cusps", 0)
        scope_c.include_arudha_padas = scope.get("include_arudha_padas", 0)
        scope_c.include_upagrahas = scope.get("include_upagrahas", 0)
        scope_c.include_sphutas = scope.get("include_sphutas", 0)
        scope_c.include_special_lagnas = scope.get("include_special_lagnas", 0)
    # else all zero (no optional sections)

    out = ffi.new("DhruvAmshaChart *")
    check(
        lib.dhruv_amsha_chart_for_date(
            engine._ptr,
            eop,
            utc,
            loc,
            bhava_cfg,
            rs_cfg,
            ayanamsha_system,
            use_nutation,
            amsha_code,
            variation,
            scope_c,
            out,
        ),
        "amsha_chart_for_date",
    )

    grahas = [_extract_amsha_entry(out.grahas[i]) for i in range(9)]
    lagna = _extract_amsha_entry(out.lagna)

    bhava_cusps = None
    if out.bhava_cusps_valid:
        bhava_cusps = [_extract_amsha_entry(out.bhava_cusps[i]) for i in range(12)]

    rashi_bhava_cusps = None
    if out.rashi_bhava_cusps_valid:
        rashi_bhava_cusps = [_extract_amsha_entry(out.rashi_bhava_cusps[i]) for i in range(12)]

    arudha_padas = None
    if out.arudha_padas_valid:
        arudha_padas = [_extract_amsha_entry(out.arudha_padas[i]) for i in range(12)]

    rashi_bhava_arudha_padas = None
    if out.rashi_bhava_arudha_padas_valid:
        rashi_bhava_arudha_padas = [
            _extract_amsha_entry(out.rashi_bhava_arudha_padas[i]) for i in range(12)
        ]

    upagrahas = None
    if out.upagrahas_valid:
        upagrahas = [_extract_amsha_entry(out.upagrahas[i]) for i in range(11)]

    sphutas = None
    if out.sphutas_valid:
        sphutas = [_extract_amsha_entry(out.sphutas[i]) for i in range(16)]

    special_lagnas = None
    if out.special_lagnas_valid:
        special_lagnas = [_extract_amsha_entry(out.special_lagnas[i]) for i in range(8)]

    return AmshaChart(
        amsha_code=out.amsha_code,
        variation_code=out.variation_code,
        grahas=grahas,
        lagna=lagna,
        bhava_cusps=bhava_cusps,
        rashi_bhava_cusps=rashi_bhava_cusps,
        arudha_padas=arudha_padas,
        rashi_bhava_arudha_padas=rashi_bhava_arudha_padas,
        upagrahas=upagrahas,
        sphutas=sphutas,
        special_lagnas=special_lagnas,
    )


def amsha_variations(amsha_code):
    """List supported variations for a single amsha."""
    out = ffi.new("DhruvAmshaVariationList *")
    check(lib.dhruv_amsha_variations(amsha_code, out), "amsha_variations")
    return _extract_amsha_variation_catalog(out[0])


def amsha_variations_many(amsha_codes):
    """List supported variations for multiple amshas."""
    count = len(amsha_codes)
    c_codes = ffi.new("uint16_t[]", count)
    for i, code in enumerate(amsha_codes):
        c_codes[i] = code
    out = ffi.new("DhruvAmshaVariationCatalogs *")
    check(
        lib.dhruv_amsha_variations_many(c_codes, count, out),
        "amsha_variations_many",
    )
    return [_extract_amsha_variation_catalog(out.lists[i]) for i in range(out.count)]
