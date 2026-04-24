"""Shadbala, Vimsopaka Bala, and Graha Avastha computation.

Wraps the dhruv_ffi_c date-based orchestration APIs for planetary strength
and state analysis.
"""

from __future__ import annotations

from ._ffi import ffi, lib
from ._check import check
from .types import (
    AllGrahaAvasthas,
    AshtakavargaResult,
    BalaBundleResult,
    BhavaBalaEntry,
    BhavaBalaResult,
    BhinnaAshtakavarga,
    GrahaAvasthas,
    KalaBalaBreakdown,
    SarvaAshtakavarga,
    SayanadiResult,
    ShadbalaEntry,
    ShadbalaResult,
    SthanaBalaBreakdown,
    VimsopakaEntry,
    VimsopakaResult,
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


def _make_amsha_selection(amsha_selection):
    if amsha_selection is None:
        return ffi.new("DhruvAmshaSelectionConfig *")
    if isinstance(amsha_selection, ffi.CData):
        return (
            ffi.addressof(amsha_selection)
            if ffi.typeof(amsha_selection) != ffi.typeof("DhruvAmshaSelectionConfig *")
            else amsha_selection
        )
    cfg = ffi.new("DhruvAmshaSelectionConfig *")
    cfg.count = amsha_selection.get("count", 0)
    codes = amsha_selection.get("codes", [])
    variations = amsha_selection.get("variations", [])
    for i, code in enumerate(codes[:40]):
        cfg.codes[i] = code
    for i, variation in enumerate(variations[:40]):
        cfg.variations[i] = variation
    return cfg


def _extract_bhavabala_entry(e):
    return BhavaBalaEntry(
        bhava_number=e.bhava_number,
        cusp_sidereal_lon=e.cusp_sidereal_lon,
        rashi_index=e.rashi_index,
        lord_graha_index=e.lord_graha_index,
        bhavadhipati=e.bhavadhipati,
        dig=e.dig,
        drishti=e.drishti,
        occupation_bonus=e.occupation_bonus,
        rising_bonus=e.rising_bonus,
        total_virupas=e.total_virupas,
        total_rupas=e.total_rupas,
    )


def _extract_bhavabala_result(out):
    return BhavaBalaResult(entries=[_extract_bhavabala_entry(out.entries[i]) for i in range(12)])


def _extract_ashtakavarga_result(out):
    bavs = []
    for i in range(7):
        b = out.bavs[i]
        bavs.append(BhinnaAshtakavarga(
            graha_index=b.graha_index,
            points=[b.points[j] for j in range(12)],
            contributors=[[b.contributors[r][c] for c in range(8)] for r in range(12)],
        ))
    sav = SarvaAshtakavarga(
        total_points=[out.sav.total_points[j] for j in range(12)],
        after_trikona=[out.sav.after_trikona[j] for j in range(12)],
        after_ekadhipatya=[out.sav.after_ekadhipatya[j] for j in range(12)],
    )
    return AshtakavargaResult(bavs=bavs, sav=sav)


def _extract_shadbala_entry(e):
    sthana = SthanaBalaBreakdown(
        uchcha=e.sthana.uchcha,
        saptavargaja=e.sthana.saptavargaja,
        ojhayugma=e.sthana.ojhayugma,
        kendradi=e.sthana.kendradi,
        drekkana=e.sthana.drekkana,
        total=e.sthana.total,
    )
    kala = KalaBalaBreakdown(
        nathonnatha=e.kala.nathonnatha,
        paksha=e.kala.paksha,
        tribhaga=e.kala.tribhaga,
        abda=e.kala.abda,
        masa=e.kala.masa,
        vara=e.kala.vara,
        hora=e.kala.hora,
        ayana=e.kala.ayana,
        yuddha=e.kala.yuddha,
        total=e.kala.total,
    )
    return ShadbalaEntry(
        graha_index=e.graha_index,
        sthana=sthana,
        dig=e.dig,
        kala=kala,
        cheshta=e.cheshta,
        naisargika=e.naisargika,
        drik=e.drik,
        total_shashtiamsas=e.total_shashtiamsas,
        total_rupas=e.total_rupas,
        required_strength=e.required_strength,
        is_strong=bool(e.is_strong),
    )


def _extract_vimsopaka_entry(e):
    return VimsopakaEntry(
        graha_index=e.graha_index,
        shadvarga=e.shadvarga,
        saptavarga=e.saptavarga,
        dashavarga=e.dashavarga,
        shodasavarga=e.shodasavarga,
    )


# ---------------------------------------------------------------------------
# Shadbala
# ---------------------------------------------------------------------------


def shadbala(
    engine,
    lsk,
    eop,
    jd_utc,
    location,
    ayanamsha_system=0,
    use_nutation=1,
    bhava_config=None,
    riseset_config=None,
    amsha_selection=None,
):
    """Compute Shadbala for all 7 sapta grahas (Sun through Saturn).

    Args:
        engine: Engine instance.
        lsk: LSK handle (kept for API uniformity).
        eop: EOP handle.
        jd_utc: UTC time tuple (year, month, day[, hour, min, sec]).
        location: (lat, lon[, alt]) tuple.
        ayanamsha_system: Ayanamsha system code.
        use_nutation: 1=yes, 0=no.
        bhava_config: Optional bhava config dict.
        riseset_config: Optional riseset config dict.

    Returns:
        ShadbalaResult with 7 ShadbalaEntry entries.
    """
    utc = _make_utc(jd_utc)
    loc = _make_location(location)
    bhava_cfg = _make_bhava_config(bhava_config)
    rs_cfg = _make_riseset_config(riseset_config)
    amsha_sel = _make_amsha_selection(amsha_selection)

    out = ffi.new("DhruvShadbalaResult *")
    check(
        lib.dhruv_shadbala_for_date(
            engine._ptr,
            eop,
            utc,
            loc,
            bhava_cfg,
            rs_cfg,
            ayanamsha_system,
            use_nutation,
            amsha_sel,
            out,
        ),
        "shadbala_for_date",
    )

    entries = []
    for i in range(7):
        e = out.entries[i]
        sthana = SthanaBalaBreakdown(
            uchcha=e.sthana.uchcha,
            saptavargaja=e.sthana.saptavargaja,
            ojhayugma=e.sthana.ojhayugma,
            kendradi=e.sthana.kendradi,
            drekkana=e.sthana.drekkana,
            total=e.sthana.total,
        )
        kala = KalaBalaBreakdown(
            nathonnatha=e.kala.nathonnatha,
            paksha=e.kala.paksha,
            tribhaga=e.kala.tribhaga,
            abda=e.kala.abda,
            masa=e.kala.masa,
            vara=e.kala.vara,
            hora=e.kala.hora,
            ayana=e.kala.ayana,
            yuddha=e.kala.yuddha,
            total=e.kala.total,
        )
        entries.append(ShadbalaEntry(
            graha_index=e.graha_index,
            sthana=sthana,
            dig=e.dig,
            kala=kala,
            cheshta=e.cheshta,
            naisargika=e.naisargika,
            drik=e.drik,
            total_shashtiamsas=e.total_shashtiamsas,
            total_rupas=e.total_rupas,
            required_strength=e.required_strength,
            is_strong=bool(e.is_strong),
        ))
    return ShadbalaResult(entries=entries)


def calculate_bhavabala(inputs):
    """Compute Bhava Bala from low-level assembled inputs."""
    cin = ffi.new("DhruvBhavaBalaInputs *")
    for i, value in enumerate(inputs["cusp_sidereal_lons"]):
        cin.cusp_sidereal_lons[i] = value
    cin.ascendant_sidereal_lon = inputs["ascendant_sidereal_lon"]
    cin.meridian_sidereal_lon = inputs["meridian_sidereal_lon"]
    for i, value in enumerate(inputs["graha_bhava_numbers"]):
        cin.graha_bhava_numbers[i] = value
    for i, value in enumerate(inputs["house_lord_strengths"]):
        cin.house_lord_strengths[i] = value
    for gi, row in enumerate(inputs["aspect_virupas"]):
        for bi, value in enumerate(row):
            cin.aspect_virupas[gi][bi] = value
    cin.birth_period = inputs["birth_period"]

    out = ffi.new("DhruvBhavaBalaResult *")
    check(lib.dhruv_calculate_bhavabala(cin, out), "calculate_bhavabala")
    return _extract_bhavabala_result(out)


def bhavabala(
    engine,
    lsk,
    eop,
    jd_utc,
    location,
    ayanamsha_system=0,
    use_nutation=1,
    bhava_config=None,
    riseset_config=None,
):
    """Compute Bhava Bala for all 12 houses."""
    utc = _make_utc(jd_utc)
    loc = _make_location(location)
    bhava_cfg = _make_bhava_config(bhava_config)
    rs_cfg = _make_riseset_config(riseset_config)

    out = ffi.new("DhruvBhavaBalaResult *")
    check(
        lib.dhruv_bhavabala_for_date(
            engine._ptr,
            eop,
            utc,
            loc,
            bhava_cfg,
            rs_cfg,
            ayanamsha_system,
            use_nutation,
            out,
        ),
        "bhavabala_for_date",
    )
    return _extract_bhavabala_result(out)


# ---------------------------------------------------------------------------
# Vimsopaka
# ---------------------------------------------------------------------------


def vimsopaka(
    engine,
    lsk,
    eop,
    jd_utc,
    location,
    ayanamsha_system=0,
    use_nutation=1,
    node_dignity_policy=0,
    amsha_selection=None,
):
    """Compute Vimsopaka Bala for all 9 navagrahas.

    Args:
        engine: Engine instance.
        lsk: LSK handle.
        eop: EOP handle.
        jd_utc: UTC time tuple.
        location: (lat, lon[, alt]) tuple.
        ayanamsha_system: Ayanamsha system code.
        use_nutation: 1=yes, 0=no.
        node_dignity_policy: 0=SignLordBased, 1=AlwaysSama.

    Returns:
        VimsopakaResult with 9 VimsopakaEntry entries.
    """
    utc = _make_utc(jd_utc)
    loc = _make_location(location)
    amsha_sel = _make_amsha_selection(amsha_selection)

    out = ffi.new("DhruvVimsopakaResult *")
    check(
        lib.dhruv_vimsopaka_for_date(
            engine._ptr,
            eop,
            utc,
            loc,
            ayanamsha_system,
            use_nutation,
            node_dignity_policy,
            amsha_sel,
            out,
        ),
        "vimsopaka_for_date",
    )

    entries = []
    for i in range(9):
        e = out.entries[i]
        entries.append(VimsopakaEntry(
            graha_index=e.graha_index,
            shadvarga=e.shadvarga,
            saptavarga=e.saptavarga,
            dashavarga=e.dashavarga,
            shodasavarga=e.shodasavarga,
        ))
    return VimsopakaResult(entries=entries)


def balas(
    engine,
    lsk,
    eop,
    jd_utc,
    location,
    ayanamsha_system=0,
    use_nutation=1,
    node_dignity_policy=0,
    bhava_config=None,
    riseset_config=None,
    amsha_selection=None,
):
    """Compute the bundled bala surfaces for one chart."""
    utc = _make_utc(jd_utc)
    loc = _make_location(location)
    bhava_cfg = _make_bhava_config(bhava_config)
    rs_cfg = _make_riseset_config(riseset_config)
    amsha_sel = _make_amsha_selection(amsha_selection)

    out = ffi.new("DhruvBalaBundleResult *")
    check(
        lib.dhruv_balas_for_date(
            engine._ptr,
            eop,
            utc,
            loc,
            bhava_cfg,
            rs_cfg,
            ayanamsha_system,
            use_nutation,
            node_dignity_policy,
            amsha_sel,
            out,
        ),
        "balas_for_date",
    )
    return BalaBundleResult(
        shadbala=ShadbalaResult(entries=[_extract_shadbala_entry(out.shadbala.entries[i]) for i in range(7)]),
        vimsopaka=VimsopakaResult(entries=[_extract_vimsopaka_entry(out.vimsopaka.entries[i]) for i in range(9)]),
        ashtakavarga=_extract_ashtakavarga_result(out.ashtakavarga),
        bhavabala=_extract_bhavabala_result(out.bhavabala),
    )


# ---------------------------------------------------------------------------
# Avastha
# ---------------------------------------------------------------------------


def avastha(
    engine,
    lsk,
    eop,
    jd_utc,
    location,
    ayanamsha_system=0,
    use_nutation=1,
    node_dignity_policy=0,
    bhava_config=None,
    riseset_config=None,
    amsha_selection=None,
):
    """Compute all 5 avastha categories for all 9 grahas.

    Args:
        engine: Engine instance.
        lsk: LSK handle.
        eop: EOP handle.
        jd_utc: UTC time tuple.
        location: (lat, lon[, alt]) tuple.
        ayanamsha_system: Ayanamsha system code.
        use_nutation: 1=yes, 0=no.
        node_dignity_policy: 0=SignLordBased, 1=AlwaysSama.
        bhava_config: Optional bhava config dict.
        riseset_config: Optional riseset config dict.

    Returns:
        AllGrahaAvasthas with 9 GrahaAvasthas entries.
    """
    utc = _make_utc(jd_utc)
    loc = _make_location(location)
    bhava_cfg = _make_bhava_config(bhava_config)
    rs_cfg = _make_riseset_config(riseset_config)
    amsha_sel = _make_amsha_selection(amsha_selection)

    out = ffi.new("DhruvAllGrahaAvasthas *")
    check(
        lib.dhruv_avastha_for_date(
            engine._ptr,
            eop,
            utc,
            loc,
            bhava_cfg,
            rs_cfg,
            ayanamsha_system,
            use_nutation,
            node_dignity_policy,
            amsha_sel,
            out,
        ),
        "avastha_for_date",
    )

    entries = []
    for i in range(9):
        a = out.entries[i]
        sayanadi = SayanadiResult(
            avastha=a.sayanadi.avastha,
            sub_states=[a.sayanadi.sub_states[j] for j in range(5)],
        )
        entries.append(GrahaAvasthas(
            baladi=a.baladi,
            jagradadi=a.jagradadi,
            deeptadi=a.deeptadi,
            deeptadi_states=[a.deeptadi_states[j] for j in range(a.deeptadi_count)],
            deeptadi_mask=a.deeptadi_mask,
            lajjitadi=a.lajjitadi,
            sayanadi=sayanadi,
        ))
    return AllGrahaAvasthas(entries=entries)
