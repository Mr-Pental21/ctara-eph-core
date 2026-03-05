"""Graha positions, core bindus, and full kundali computation.

Wraps the dhruv_ffi_c orchestration APIs for comprehensive birth-chart
computation.
"""

from __future__ import annotations

from ._ffi import ffi, lib
from ._check import check
from .types import (
    AmshaChart,
    AmshaEntry,
    AshtakavargaResult,
    AllGrahaAvasthas,
    AllUpagrahas,
    AyanaInfo,
    BhavaEntry,
    BhavaResult,
    BhinnaAshtakavarga,
    BindusResult,
    DashaPeriod,
    DashaSnapshot,
    DrishtiEntry,
    DrishtiResult,
    FullKundaliResult,
    GhatikaInfo,
    GrahaAvasthas,
    GrahaEntry,
    GrahaLongitudes,
    GrahaPositions,
    HoraInfo,
    KaranaInfo,
    MasaInfo,
    PanchangInfo,
    PanchangNakshatraInfo,
    SarvaAshtakavarga,
    SayanadiResult,
    ShadbalaEntry,
    ShadbalaResult,
    SpecialLagnas,
    SthanaBalaBreakdown,
    KalaBalaBreakdown,
    TithiInfo,
    UtcTime,
    VaarInfo,
    VarshaInfo,
    VimsopakaEntry,
    VimsopakaResult,
    YogaInfo,
)


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def _make_utc(jd_utc):
    """Build a DhruvUtcTime C struct from a (year, month, day, hour, min, sec) tuple."""
    utc = ffi.new("DhruvUtcTime *")
    utc.year = jd_utc[0]
    utc.month = jd_utc[1]
    utc.day = jd_utc[2]
    utc.hour = jd_utc[3] if len(jd_utc) > 3 else 0
    utc.minute = jd_utc[4] if len(jd_utc) > 4 else 0
    utc.second = jd_utc[5] if len(jd_utc) > 5 else 0.0
    return utc


def _make_location(location):
    """Build a DhruvGeoLocation C struct from a (lat, lon, alt_m) tuple."""
    loc = ffi.new("DhruvGeoLocation *")
    loc.latitude_deg = location[0]
    loc.longitude_deg = location[1]
    loc.altitude_m = location[2] if len(location) > 2 else 0.0
    return loc


def _make_bhava_config(bhava_config):
    """Build DhruvBhavaConfig from a dict or return NULL for defaults."""
    if bhava_config is None:
        return ffi.NULL
    cfg = ffi.new("DhruvBhavaConfig *")
    cfg.system = bhava_config.get("system", 0)
    cfg.starting_point = bhava_config.get("starting_point", -1)
    cfg.custom_start_deg = bhava_config.get("custom_start_deg", 0.0)
    cfg.reference_mode = bhava_config.get("reference_mode", 0)
    return cfg


def _make_riseset_config(riseset_config):
    """Build DhruvRiseSetConfig from a dict or return NULL for defaults."""
    if riseset_config is None:
        return ffi.NULL
    cfg = ffi.new("DhruvRiseSetConfig *")
    cfg.use_refraction = riseset_config.get("use_refraction", 1)
    cfg.sun_limb = riseset_config.get("sun_limb", 0)
    cfg.altitude_correction = riseset_config.get("altitude_correction", 0)
    return cfg


def _make_sankranti_config(sankranti_config):
    """Build DhruvSankrantiConfig from a dict or return NULL for defaults."""
    if sankranti_config is None:
        return ffi.NULL
    cfg = ffi.new("DhruvSankrantiConfig *")
    cfg.ayanamsha_system = sankranti_config.get("ayanamsha_system", 0)
    cfg.use_nutation = sankranti_config.get("use_nutation", 1)
    cfg.reference_plane = sankranti_config.get("reference_plane", -1)
    cfg.step_size_days = sankranti_config.get("step_size_days", 1.0)
    cfg.max_iterations = sankranti_config.get("max_iterations", 50)
    cfg.convergence_days = sankranti_config.get("convergence_days", 1e-10)
    return cfg


def _graha_entry_from_ffi(e):
    """Convert a DhruvGrahaEntry to a GrahaEntry dataclass."""
    return GrahaEntry(
        sidereal_longitude=e.sidereal_longitude,
        rashi_index=e.rashi_index,
        nakshatra_index=e.nakshatra_index,
        pada=e.pada,
        bhava_number=e.bhava_number,
    )


def _utc_from_ffi(u):
    """Convert a DhruvUtcTime to a UtcTime dataclass."""
    return UtcTime(
        year=u.year, month=u.month, day=u.day,
        hour=u.hour, minute=u.minute, second=u.second,
    )


# ---------------------------------------------------------------------------
# Graha Longitudes (JD TDB based, no location needed)
# ---------------------------------------------------------------------------


def graha_longitudes(engine, jd_tdb, ayanamsha_system=0, use_nutation=1):
    """Return sidereal longitudes of 9 grahas as a list of 9 floats.

    Args:
        engine: Engine instance (use engine._ptr).
        jd_tdb: Julian date in TDB.
        ayanamsha_system: Ayanamsha system code (0=Lahiri default).
        use_nutation: Whether to apply nutation (1=yes, 0=no).

    Returns:
        GrahaLongitudes with a list of 9 sidereal longitudes.
    """
    out = ffi.new("DhruvGrahaLongitudes *")
    check(
        lib.dhruv_graha_sidereal_longitudes(
            engine._ptr, jd_tdb, ayanamsha_system, use_nutation, out
        ),
        "graha_sidereal_longitudes",
    )
    return GrahaLongitudes(longitudes=[out.longitudes[i] for i in range(9)])


def graha_tropical_longitudes(engine, jd_tdb):
    """Return tropical longitudes of 9 grahas as a list of 9 floats.

    Args:
        engine: Engine instance.
        jd_tdb: Julian date in TDB.

    Returns:
        GrahaLongitudes with a list of 9 tropical longitudes.
    """
    out = ffi.new("DhruvGrahaLongitudes *")
    check(
        lib.dhruv_graha_tropical_longitudes(engine._ptr, jd_tdb, out),
        "graha_tropical_longitudes",
    )
    return GrahaLongitudes(longitudes=[out.longitudes[i] for i in range(9)])


# ---------------------------------------------------------------------------
# Graha Positions (UTC-based, needs location)
# ---------------------------------------------------------------------------


def graha_positions(
    engine,
    lsk,
    eop,
    jd_utc,
    location,
    ayanamsha_system=0,
    use_nutation=1,
    config=None,
    bhava_config=None,
    sankranti_config=None,
):
    """Compute positions of all 9 grahas + optional lagna/outer planets.

    Args:
        engine: Engine instance.
        lsk: LSK handle (unused by this FFI call but kept for API uniformity).
        eop: EOP handle.
        jd_utc: UTC time as (year, month, day[, hour, min, sec]) tuple.
        location: (lat_deg, lon_deg[, alt_m]) tuple.
        ayanamsha_system: Ayanamsha system code.
        use_nutation: 1=apply nutation, 0=skip.
        config: Optional dict with keys include_nakshatra, include_lagna,
                include_outer_planets, include_bhava (all u8 0/1).
        bhava_config: Optional dict for bhava system config.
        sankranti_config: Optional dict for sankranti config.

    Returns:
        GrahaPositions dataclass.
    """
    utc = _make_utc(jd_utc)
    loc = _make_location(location)
    bhava_cfg = _make_bhava_config(bhava_config)

    if config is not None:
        cfg = ffi.new("DhruvGrahaPositionsConfig *")
        cfg.include_nakshatra = config.get("include_nakshatra", 0)
        cfg.include_lagna = config.get("include_lagna", 0)
        cfg.include_outer_planets = config.get("include_outer_planets", 0)
        cfg.include_bhava = config.get("include_bhava", 0)
    else:
        cfg = ffi.NULL

    out = ffi.new("DhruvGrahaPositions *")
    check(
        lib.dhruv_graha_positions(
            engine._ptr,
            eop,
            utc,
            loc,
            bhava_cfg,
            ayanamsha_system,
            use_nutation,
            cfg,
            out,
        ),
        "graha_positions",
    )

    grahas = [_graha_entry_from_ffi(out.grahas[i]) for i in range(9)]
    lagna = _graha_entry_from_ffi(out.lagna)
    outer_planets = [_graha_entry_from_ffi(out.outer_planets[i]) for i in range(3)]
    return GrahaPositions(grahas=grahas, lagna=lagna, outer_planets=outer_planets)


# ---------------------------------------------------------------------------
# Core Bindus
# ---------------------------------------------------------------------------


def core_bindus(
    engine,
    lsk,
    eop,
    jd_utc,
    location,
    ayanamsha_system=0,
    use_nutation=1,
    bhava_config=None,
    riseset_config=None,
    bindus_config=None,
):
    """Compute 19 curated sensitive points (12 arudha padas + 7 special).

    Args:
        engine: Engine instance.
        lsk: LSK handle (kept for API uniformity).
        eop: EOP handle.
        jd_utc: UTC time tuple.
        location: (lat, lon[, alt]) tuple.
        ayanamsha_system: Ayanamsha system code.
        use_nutation: 1=yes, 0=no.
        bhava_config: Optional bhava config dict.
        riseset_config: Optional riseset config dict.
        bindus_config: Optional dict with include_nakshatra, include_bhava (u8).

    Returns:
        BindusResult dataclass.
    """
    utc = _make_utc(jd_utc)
    loc = _make_location(location)
    bhava_cfg = _make_bhava_config(bhava_config)
    rs_cfg = _make_riseset_config(riseset_config)

    if bindus_config is not None:
        bcfg = ffi.new("DhruvBindusConfig *")
        bcfg.include_nakshatra = bindus_config.get("include_nakshatra", 0)
        bcfg.include_bhava = bindus_config.get("include_bhava", 0)
    else:
        bcfg = ffi.NULL

    out = ffi.new("DhruvBindusResult *")
    check(
        lib.dhruv_core_bindus(
            engine._ptr,
            eop,
            utc,
            loc,
            bhava_cfg,
            rs_cfg,
            ayanamsha_system,
            use_nutation,
            bcfg,
            out,
        ),
        "core_bindus",
    )

    arudha_padas = [_graha_entry_from_ffi(out.arudha_padas[i]) for i in range(12)]
    return BindusResult(
        arudha_padas=arudha_padas,
        bhrigu_bindu=_graha_entry_from_ffi(out.bhrigu_bindu),
        pranapada_lagna=_graha_entry_from_ffi(out.pranapada_lagna),
        gulika=_graha_entry_from_ffi(out.gulika),
        maandi=_graha_entry_from_ffi(out.maandi),
        hora_lagna=_graha_entry_from_ffi(out.hora_lagna),
        ghati_lagna=_graha_entry_from_ffi(out.ghati_lagna),
        sree_lagna=_graha_entry_from_ffi(out.sree_lagna),
    )


# ---------------------------------------------------------------------------
# Full Kundali
# ---------------------------------------------------------------------------


def full_kundali_config_default():
    """Return a default DhruvFullKundaliConfig as a cffi struct.

    Core sections (bhava, graha, bindus, drishti, ashtakavarga, upagrahas,
    special_lagnas) default to enabled. Optional sections (amshas, shadbala,
    vimsopaka, avastha, panchang, calendar, dasha) default to disabled.
    """
    return lib.dhruv_full_kundali_config_default()


def _extract_drishti_entry(e):
    return DrishtiEntry(
        angular_distance=e.angular_distance,
        base_virupa=e.base_virupa,
        special_virupa=e.special_virupa,
        total_virupa=e.total_virupa,
    )


def _extract_amsha_entry(e):
    return AmshaEntry(
        sidereal_longitude=e.sidereal_longitude,
        rashi_index=e.rashi_index,
        dms_degrees=e.dms_degrees,
        dms_minutes=e.dms_minutes,
        dms_seconds=e.dms_seconds,
        degrees_in_rashi=e.degrees_in_rashi,
    )


def _extract_amsha_chart(c):
    grahas = [_extract_amsha_entry(c.grahas[i]) for i in range(9)]
    lagna = _extract_amsha_entry(c.lagna)

    bhava_cusps = None
    if c.bhava_cusps_valid:
        bhava_cusps = [_extract_amsha_entry(c.bhava_cusps[i]) for i in range(12)]

    arudha_padas = None
    if c.arudha_padas_valid:
        arudha_padas = [_extract_amsha_entry(c.arudha_padas[i]) for i in range(12)]

    upagrahas = None
    if c.upagrahas_valid:
        upagrahas = [_extract_amsha_entry(c.upagrahas[i]) for i in range(11)]

    sphutas = None
    if c.sphutas_valid:
        sphutas = [_extract_amsha_entry(c.sphutas[i]) for i in range(16)]

    special_lagnas = None
    if c.special_lagnas_valid:
        special_lagnas = [_extract_amsha_entry(c.special_lagnas[i]) for i in range(8)]

    return AmshaChart(
        amsha_code=c.amsha_code,
        variation_code=c.variation_code,
        grahas=grahas,
        lagna=lagna,
        bhava_cusps=bhava_cusps,
        arudha_padas=arudha_padas,
        upagrahas=upagrahas,
        sphutas=sphutas,
        special_lagnas=special_lagnas,
    )


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


def _extract_sayanadi(s):
    return SayanadiResult(
        avastha=s.avastha,
        sub_states=[s.sub_states[i] for i in range(5)],
    )


def _extract_graha_avastha(a):
    return GrahaAvasthas(
        baladi=a.baladi,
        jagradadi=a.jagradadi,
        deeptadi=a.deeptadi,
        lajjitadi=a.lajjitadi,
        sayanadi=_extract_sayanadi(a.sayanadi),
    )


def _extract_dasha_period(p):
    return DashaPeriod(
        entity_type=p.entity_type,
        entity_index=p.entity_index,
        start_jd=p.start_jd,
        end_jd=p.end_jd,
        level=p.level,
        order=p.order,
        parent_idx=p.parent_idx,
    )


def _extract_panchang_info(p):
    tithi = TithiInfo(
        tithi_index=p.tithi.tithi_index,
        paksha=p.tithi.paksha,
        tithi_in_paksha=p.tithi.tithi_in_paksha,
        start=_utc_from_ffi(p.tithi.start),
        end=_utc_from_ffi(p.tithi.end),
    )
    karana = KaranaInfo(
        karana_index=p.karana.karana_index,
        karana_name_index=p.karana.karana_name_index,
        start=_utc_from_ffi(p.karana.start),
        end=_utc_from_ffi(p.karana.end),
    )
    yoga = YogaInfo(
        yoga_index=p.yoga.yoga_index,
        start=_utc_from_ffi(p.yoga.start),
        end=_utc_from_ffi(p.yoga.end),
    )
    vaar = VaarInfo(
        vaar_index=p.vaar.vaar_index,
        start=_utc_from_ffi(p.vaar.start),
        end=_utc_from_ffi(p.vaar.end),
    )
    hora = HoraInfo(
        hora_index=p.hora.hora_index,
        hora_position=p.hora.hora_position,
        start=_utc_from_ffi(p.hora.start),
        end=_utc_from_ffi(p.hora.end),
    )
    ghatika = GhatikaInfo(
        value=p.ghatika.value,
        start=_utc_from_ffi(p.ghatika.start),
        end=_utc_from_ffi(p.ghatika.end),
    )
    nakshatra = PanchangNakshatraInfo(
        nakshatra_index=p.nakshatra.nakshatra_index,
        pada=p.nakshatra.pada,
        start=_utc_from_ffi(p.nakshatra.start),
        end=_utc_from_ffi(p.nakshatra.end),
    )

    masa = None
    ayana = None
    varsha = None
    if p.calendar_valid:
        masa = MasaInfo(
            masa_index=p.masa.masa_index,
            adhika=bool(p.masa.adhika),
            start=_utc_from_ffi(p.masa.start),
            end=_utc_from_ffi(p.masa.end),
        )
        ayana = AyanaInfo(
            ayana=p.ayana.ayana,
            start=_utc_from_ffi(p.ayana.start),
            end=_utc_from_ffi(p.ayana.end),
        )
        varsha = VarshaInfo(
            samvatsara_index=p.varsha.samvatsara_index,
            order=p.varsha.order,
            start=_utc_from_ffi(p.varsha.start),
            end=_utc_from_ffi(p.varsha.end),
        )

    return PanchangInfo(
        tithi=tithi,
        karana=karana,
        yoga=yoga,
        vaar=vaar,
        hora=hora,
        ghatika=ghatika,
        nakshatra=nakshatra,
        calendar_valid=bool(p.calendar_valid),
        masa=masa,
        ayana=ayana,
        varsha=varsha,
    )


def full_kundali(
    engine,
    lsk,
    eop,
    jd_utc,
    location,
    ayanamsha_system=0,
    use_nutation=1,
    config=None,
    bhava_config=None,
    riseset_config=None,
):
    """Compute full kundali with all sections.

    Args:
        engine: Engine instance.
        lsk: LSK handle (kept for uniformity).
        eop: EOP handle.
        jd_utc: UTC time tuple.
        location: (lat, lon[, alt]) tuple.
        ayanamsha_system: Ayanamsha system code.
        use_nutation: 1=yes, 0=no.
        config: DhruvFullKundaliConfig (from full_kundali_config_default()) or None.
        bhava_config: Optional bhava config dict.
        riseset_config: Optional riseset config dict.

    Returns:
        FullKundaliResult dataclass with all sections populated per config.
    """
    utc = _make_utc(jd_utc)
    loc = _make_location(location)
    bhava_cfg = _make_bhava_config(bhava_config)
    rs_cfg = _make_riseset_config(riseset_config)

    if config is None:
        cfg_ptr = ffi.NULL
    elif isinstance(config, ffi.CData):
        # Already a C struct (e.g. from full_kundali_config_default())
        cfg_ptr = ffi.addressof(config) if ffi.typeof(config) != ffi.typeof("DhruvFullKundaliConfig *") else config
    else:
        cfg_ptr = config

    out = ffi.new("DhruvFullKundaliResult *")
    check(
        lib.dhruv_full_kundali_for_date(
            engine._ptr,
            eop,
            utc,
            loc,
            bhava_cfg,
            rs_cfg,
            ayanamsha_system,
            use_nutation,
            cfg_ptr,
            out,
        ),
        "full_kundali_for_date",
    )

    try:
        # Extract all sections
        ayanamsha_deg = out.ayanamsha_deg

        # Bhava cusps
        bhava_cusps = None
        if out.bhava_cusps_valid:
            bhavas = []
            for i in range(12):
                b = out.bhava_cusps.bhavas[i]
                bhavas.append(BhavaEntry(
                    number=b.number, cusp_deg=b.cusp_deg,
                    start_deg=b.start_deg, end_deg=b.end_deg,
                ))
            bhava_cusps = BhavaResult(
                bhavas=bhavas,
                lagna_deg=out.bhava_cusps.lagna_deg,
                mc_deg=out.bhava_cusps.mc_deg,
            )

        # Graha positions
        graha_pos = None
        if out.graha_positions_valid:
            grahas = [_graha_entry_from_ffi(out.graha_positions.grahas[i]) for i in range(9)]
            lagna_entry = _graha_entry_from_ffi(out.graha_positions.lagna)
            outer = [_graha_entry_from_ffi(out.graha_positions.outer_planets[i]) for i in range(3)]
            graha_pos = GrahaPositions(grahas=grahas, lagna=lagna_entry, outer_planets=outer)

        # Bindus
        bindus = None
        if out.bindus_valid:
            arudha_padas = [_graha_entry_from_ffi(out.bindus.arudha_padas[i]) for i in range(12)]
            bindus = BindusResult(
                arudha_padas=arudha_padas,
                bhrigu_bindu=_graha_entry_from_ffi(out.bindus.bhrigu_bindu),
                pranapada_lagna=_graha_entry_from_ffi(out.bindus.pranapada_lagna),
                gulika=_graha_entry_from_ffi(out.bindus.gulika),
                maandi=_graha_entry_from_ffi(out.bindus.maandi),
                hora_lagna=_graha_entry_from_ffi(out.bindus.hora_lagna),
                ghati_lagna=_graha_entry_from_ffi(out.bindus.ghati_lagna),
                sree_lagna=_graha_entry_from_ffi(out.bindus.sree_lagna),
            )

        # Drishti
        drishti = None
        if out.drishti_valid:
            g2g = [
                [_extract_drishti_entry(out.drishti.graha_to_graha[i][j]) for j in range(9)]
                for i in range(9)
            ]
            g2b = [
                [_extract_drishti_entry(out.drishti.graha_to_bhava[i][j]) for j in range(12)]
                for i in range(9)
            ]
            g2l = [_extract_drishti_entry(out.drishti.graha_to_lagna[i]) for i in range(9)]
            g2bi = [
                [_extract_drishti_entry(out.drishti.graha_to_bindus[i][j]) for j in range(19)]
                for i in range(9)
            ]
            drishti = DrishtiResult(
                graha_to_graha=g2g,
                graha_to_bhava=g2b,
                graha_to_lagna=g2l,
                graha_to_bindus=g2bi,
            )

        # Ashtakavarga
        ashtakavarga = None
        if out.ashtakavarga_valid:
            bavs = []
            for i in range(7):
                b = out.ashtakavarga.bavs[i]
                bavs.append(BhinnaAshtakavarga(
                    graha_index=b.graha_index,
                    points=[b.points[j] for j in range(12)],
                ))
            sav = SarvaAshtakavarga(
                total_points=[out.ashtakavarga.sav.total_points[j] for j in range(12)],
                after_trikona=[out.ashtakavarga.sav.after_trikona[j] for j in range(12)],
                after_ekadhipatya=[out.ashtakavarga.sav.after_ekadhipatya[j] for j in range(12)],
            )
            ashtakavarga = AshtakavargaResult(bavs=bavs, sav=sav)

        # Upagrahas
        upagrahas = None
        if out.upagrahas_valid:
            u = out.upagrahas
            upagrahas = AllUpagrahas(
                gulika=u.gulika, maandi=u.maandi, kaala=u.kaala,
                mrityu=u.mrityu, artha_prahara=u.artha_prahara,
                yama_ghantaka=u.yama_ghantaka, dhooma=u.dhooma,
                vyatipata=u.vyatipata, parivesha=u.parivesha,
                indra_chapa=u.indra_chapa, upaketu=u.upaketu,
            )

        # Special lagnas
        special_lagnas = None
        if out.special_lagnas_valid:
            sl = out.special_lagnas
            special_lagnas = SpecialLagnas(
                bhava_lagna=sl.bhava_lagna, hora_lagna=sl.hora_lagna,
                ghati_lagna=sl.ghati_lagna, vighati_lagna=sl.vighati_lagna,
                varnada_lagna=sl.varnada_lagna, sree_lagna=sl.sree_lagna,
                pranapada_lagna=sl.pranapada_lagna, indu_lagna=sl.indu_lagna,
            )

        # Amshas
        amshas = None
        if out.amshas_valid and out.amshas_count > 0:
            amshas = [_extract_amsha_chart(out.amshas[i]) for i in range(out.amshas_count)]

        # Shadbala
        shadbala = None
        if out.shadbala_valid:
            shadbala = ShadbalaResult(
                entries=[_extract_shadbala_entry(out.shadbala.entries[i]) for i in range(7)]
            )

        # Vimsopaka
        vimsopaka = None
        if out.vimsopaka_valid:
            vimsopaka = VimsopakaResult(
                entries=[_extract_vimsopaka_entry(out.vimsopaka.entries[i]) for i in range(9)]
            )

        # Avastha
        avastha = None
        if out.avastha_valid:
            avastha = AllGrahaAvasthas(
                entries=[_extract_graha_avastha(out.avastha.entries[i]) for i in range(9)]
            )

        # Panchang
        panchang = None
        if out.panchang_valid:
            panchang = _extract_panchang_info(out.panchang)

        # Dasha snapshots (from the full kundali result, not hierarchy handles)
        dasha_snapshots = None
        if out.dasha_snapshot_count > 0:
            dasha_snapshots = []
            for i in range(out.dasha_snapshot_count):
                snap = out.dasha_snapshots[i]
                periods = [_extract_dasha_period(snap.periods[j]) for j in range(snap.count)]
                dasha_snapshots.append(DashaSnapshot(
                    system=snap.system,
                    query_jd=snap.query_jd,
                    periods=periods,
                ))

        return FullKundaliResult(
            ayanamsha_deg=ayanamsha_deg,
            bhava_cusps=bhava_cusps,
            graha_positions=graha_pos,
            bindus=bindus,
            drishti=drishti,
            ashtakavarga=ashtakavarga,
            upagrahas=upagrahas,
            special_lagnas=special_lagnas,
            amshas=amshas,
            shadbala=shadbala,
            vimsopaka=vimsopaka,
            avastha=avastha,
            panchang=panchang,
            dasha_snapshots=dasha_snapshots,
        )
    finally:
        lib.dhruv_full_kundali_result_free(out)
