"""Shadbala, Vimsopaka Bala, and Graha Avastha computation.

Wraps the dhruv_ffi_c date-based orchestration APIs for planetary strength
and state analysis.
"""

from __future__ import annotations

from ._ffi import ffi, lib
from ._check import check
from .types import (
    AllGrahaAvasthas,
    GrahaAvasthas,
    KalaBalaBreakdown,
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
    return cfg


def _make_riseset_config(riseset_config):
    if riseset_config is None:
        return ffi.NULL
    cfg = ffi.new("DhruvRiseSetConfig *")
    cfg.use_refraction = riseset_config.get("use_refraction", 1)
    cfg.sun_limb = riseset_config.get("sun_limb", 0)
    cfg.altitude_correction = riseset_config.get("altitude_correction", 0)
    return cfg


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
            lajjitadi=a.lajjitadi,
            sayanadi=sayanadi,
        ))
    return AllGrahaAvasthas(entries=entries)
