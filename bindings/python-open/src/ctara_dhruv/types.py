"""Frozen dataclasses mirroring the C ABI result types from dhruv_ffi_c.

Every type is ``@dataclass(frozen=True)`` so instances are immutable and
hashable.  Field names follow the C struct field names converted to
``snake_case`` where practical.
"""

from __future__ import annotations

from dataclasses import dataclass, field
from datetime import datetime
from typing import TYPE_CHECKING, Optional

if TYPE_CHECKING:
    from .dasha import DashaHierarchy


# ---------------------------------------------------------------------------
# Core types
# ---------------------------------------------------------------------------


@dataclass(frozen=True)
class StateVector:
    """Cartesian state vector (km and km/s)."""

    x: float
    y: float
    z: float
    vx: float
    vy: float
    vz: float


@dataclass(frozen=True)
class SphericalCoords:
    """Spherical position: longitude, latitude (degrees), distance (km)."""

    lon_deg: float
    lat_deg: float
    distance_km: float


@dataclass(frozen=True)
class SphericalState:
    """Spherical state with angular velocities.

    Speeds: ``lon_speed`` and ``lat_speed`` in deg/day,
    ``distance_speed`` in km/s.
    """

    lon_deg: float
    lat_deg: float
    distance_km: float
    lon_speed: float
    lat_speed: float
    distance_speed: float


@dataclass(frozen=True)
class UtcTime:
    """Broken-down UTC calendar time matching ``DhruvUtcTime``."""

    year: int
    month: int
    day: int
    hour: int
    minute: int
    second: float

    def to_datetime(self) -> datetime:
        """Convert to a ``datetime.datetime``, truncating to microseconds."""
        whole_sec = int(self.second)
        microsecond = int((self.second - whole_sec) * 1_000_000)
        return datetime(
            self.year,
            self.month,
            self.day,
            self.hour,
            self.minute,
            whole_sec,
            microsecond,
        )

    @classmethod
    def from_datetime(cls, dt: datetime) -> UtcTime:
        """Create a ``UtcTime`` from a ``datetime.datetime``."""
        sec = dt.second + dt.microsecond / 1_000_000.0
        return cls(dt.year, dt.month, dt.day, dt.hour, dt.minute, sec)


@dataclass(frozen=True)
class GeoLocation:
    """Observer geographic location."""

    lat_deg: float
    lon_deg: float
    alt_m: float = 0.0


@dataclass(frozen=True)
class Dms:
    """Degrees-minutes-seconds representation."""

    degrees: int
    minutes: int
    seconds: float


# ---------------------------------------------------------------------------
# Rashi / Nakshatra
# ---------------------------------------------------------------------------


@dataclass(frozen=True)
class RashiInfo:
    """Rashi (zodiac sign) classification.

    ``rashi_index``: 0-based (0=Mesha .. 11=Meena).
    ``degrees_in_rashi``: decimal degrees within the rashi [0, 30).
    ``dms``: position within rashi as DMS.
    """

    rashi_index: int
    degrees_in_rashi: float
    dms: Dms


@dataclass(frozen=True)
class NakshatraInfo:
    """Nakshatra (lunar mansion) classification, 27-scheme.

    ``nakshatra_index``: 0-based (0=Ashwini .. 26=Revati).
    ``pada``: quarter 1-4.
    ``degrees_in_nakshatra``: decimal degrees within the nakshatra.
    ``degrees_in_pada``: decimal degrees within the pada.
    """

    nakshatra_index: int
    pada: int
    degrees_in_nakshatra: float
    degrees_in_pada: float


@dataclass(frozen=True)
class Nakshatra28Info:
    """Nakshatra classification, 28-scheme (with Abhijit).

    ``nakshatra_index``: 0-based (0=Ashwini, 21=Abhijit, 27=Revati).
    ``pada``: quarter 1-4 (0 for Abhijit).
    """

    nakshatra_index: int
    pada: int
    degrees_in_nakshatra: float


# ---------------------------------------------------------------------------
# Bhava (House Systems)
# ---------------------------------------------------------------------------


@dataclass(frozen=True)
class BhavaEntry:
    """A single bhava (house).

    ``number``: bhava number 1-12.
    ``cusp_deg``: cusp longitude [0, 360).
    ``start_deg`` / ``end_deg``: span in degrees.
    """

    number: int
    cusp_deg: float
    start_deg: float
    end_deg: float


@dataclass(frozen=True)
class BhavaResult:
    """Complete bhava computation result with 12 houses plus lagna and MC."""

    bhavas: list[BhavaEntry]
    lagna_deg: float
    mc_deg: float


# ---------------------------------------------------------------------------
# Rise / Set
# ---------------------------------------------------------------------------


@dataclass(frozen=True)
class RiseSetResult:
    """Single rise/set event result.

    ``result_type``: 0=event, 1=never rises, 2=never sets.
    ``event_code``: DHRUV_EVENT_* constant (valid when result_type==0).
    ``jd_tdb``: event time in JD TDB (valid when result_type==0).
    """

    result_type: int
    event_code: int
    jd_tdb: float


# ---------------------------------------------------------------------------
# Search results
# ---------------------------------------------------------------------------


@dataclass(frozen=True)
class ConjunctionEvent:
    """Conjunction / aspect event.

    ``body1_code`` / ``body2_code``: NAIF body codes.
    """

    jd_tdb: float
    actual_separation_deg: float
    body1_longitude_deg: float
    body2_longitude_deg: float
    body1_latitude_deg: float
    body2_latitude_deg: float
    body1_code: int
    body2_code: int


@dataclass(frozen=True)
class ChandraGrahanResult:
    """Lunar eclipse (Chandra Grahan) result.

    ``grahan_type``: 0=penumbral, 1=partial, 2=total.
    Contact JDs use ``DHRUV_JD_ABSENT`` (-1.0) when not applicable.
    """

    grahan_type: int
    magnitude: float
    penumbral_magnitude: float
    greatest_grahan_jd: float
    p1_jd: float
    u1_jd: float
    u2_jd: float
    u3_jd: float
    u4_jd: float
    p4_jd: float
    moon_ecliptic_lat_deg: float
    angular_separation_deg: float


@dataclass(frozen=True)
class SuryaGrahanResult:
    """Solar eclipse (Surya Grahan) result.

    ``grahan_type``: 0=partial, 1=annular, 2=total, 3=hybrid.
    """

    grahan_type: int
    magnitude: float
    greatest_grahan_jd: float
    c1_jd: float
    c2_jd: float
    c3_jd: float
    c4_jd: float
    moon_ecliptic_lat_deg: float
    angular_separation_deg: float


@dataclass(frozen=True)
class StationaryEvent:
    """Planetary station event.

    ``station_type``: 0=retrograde, 1=direct.
    """

    jd_tdb: float
    body_code: int
    longitude_deg: float
    latitude_deg: float
    station_type: int


@dataclass(frozen=True)
class MaxSpeedEvent:
    """Peak-speed event.

    ``speed_type``: 0=direct, 1=retrograde.
    """

    jd_tdb: float
    body_code: int
    longitude_deg: float
    latitude_deg: float
    speed_deg_per_day: float
    speed_type: int


@dataclass(frozen=True)
class LunarPhaseEvent:
    """Lunar phase event (Purnima / Amavasya).

    ``phase``: DHRUV_LUNAR_PHASE_NEW_MOON or _FULL_MOON.
    """

    utc: UtcTime
    phase: int
    moon_longitude_deg: float
    sun_longitude_deg: float


@dataclass(frozen=True)
class SankrantiEvent:
    """Sankranti (solar ingress) event.

    ``rashi_index``: 0-based (0=Mesha .. 11=Meena).
    """

    utc: UtcTime
    rashi_index: int
    sun_sidereal_longitude_deg: float
    sun_tropical_longitude_deg: float


# ---------------------------------------------------------------------------
# Pure-math Panchang classifiers
# ---------------------------------------------------------------------------


@dataclass(frozen=True)
class TithiPosition:
    """Tithi from elongation (pure math).

    ``tithi_index``: 0-based (0..29).
    ``paksha``: 0=Shukla, 1=Krishna.
    ``tithi_in_paksha``: 1-based (1..15).
    ``degrees_in_tithi``: [0, 12).
    """

    tithi_index: int
    paksha: int
    tithi_in_paksha: int
    degrees_in_tithi: float


@dataclass(frozen=True)
class KaranaPosition:
    """Karana from elongation (pure math).

    ``karana_index``: 0-based (0..59).
    ``degrees_in_karana``: [0, 6).
    """

    karana_index: int
    degrees_in_karana: float


@dataclass(frozen=True)
class YogaPosition:
    """Yoga from sidereal sum (pure math).

    ``yoga_index``: 0-based (0..26).
    ``degrees_in_yoga``: [0, 13.333...).
    """

    yoga_index: int
    degrees_in_yoga: float


@dataclass(frozen=True)
class SamvatsaraResult:
    """Jovian year (samvatsara) result.

    ``samvatsara_index``: 0-based (0..59).
    ``cycle_position``: 1-based (1..60).
    """

    samvatsara_index: int
    cycle_position: int


# ---------------------------------------------------------------------------
# Panchang (engine-computed, with time boundaries)
# ---------------------------------------------------------------------------


@dataclass(frozen=True)
class TithiInfo:
    """Tithi with time boundaries.

    ``tithi_index``: 0-based (0=Shukla Pratipada .. 29=Amavasya).
    ``paksha``: 0=Shukla, 1=Krishna.
    ``tithi_in_paksha``: 1-based (1-15).
    """

    tithi_index: int
    paksha: int
    tithi_in_paksha: int
    start: UtcTime
    end: UtcTime


@dataclass(frozen=True)
class KaranaInfo:
    """Karana with time boundaries.

    ``karana_index``: 0-based sequence index (0-59) within the synodic month.
    ``karana_name_index``: name index in ALL_KARANAS (0=Bava .. 10=Kinstugna).
    """

    karana_index: int
    karana_name_index: int
    start: UtcTime
    end: UtcTime


@dataclass(frozen=True)
class YogaInfo:
    """Yoga with time boundaries.

    ``yoga_index``: 0-based (0=Vishkumbha .. 26=Vaidhriti).
    """

    yoga_index: int
    start: UtcTime
    end: UtcTime


@dataclass(frozen=True)
class VaarInfo:
    """Vaar (weekday) with time boundaries.

    ``vaar_index``: 0=Ravivaar(Sunday) .. 6=Shanivaar(Saturday).
    """

    vaar_index: int
    start: UtcTime
    end: UtcTime


@dataclass(frozen=True)
class HoraInfo:
    """Hora with time boundaries.

    ``hora_index``: Chaldean sequence lord index (0=Surya .. 6=Mangal).
    ``hora_position``: 0-based position within the Vedic day (0-23).
    """

    hora_index: int
    hora_position: int
    start: UtcTime
    end: UtcTime


@dataclass(frozen=True)
class GhatikaInfo:
    """Ghatika with time boundaries.

    ``value``: ghatika number (1-60).
    """

    value: int
    start: UtcTime
    end: UtcTime


@dataclass(frozen=True)
class PanchangNakshatraInfo:
    """Moon's nakshatra with time boundaries.

    ``nakshatra_index``: 0-based (0=Ashwini .. 26=Revati).
    ``pada``: quarter 1-4.
    """

    nakshatra_index: int
    pada: int
    start: UtcTime
    end: UtcTime


@dataclass(frozen=True)
class MasaInfo:
    """Lunar month (masa) with time boundaries.

    ``masa_index``: 0-based (0=Chaitra .. 11=Phalguna).
    ``adhika``: True if intercalary month.
    """

    masa_index: int
    adhika: bool
    start: UtcTime
    end: UtcTime


@dataclass(frozen=True)
class AyanaInfo:
    """Ayana with time boundaries.

    ``ayana``: 0=Uttarayana, 1=Dakshinayana.
    """

    ayana: int
    start: UtcTime
    end: UtcTime


@dataclass(frozen=True)
class VarshaInfo:
    """Varsha (Jovian year) with time boundaries.

    ``samvatsara_index``: 0-based (0=Prabhava .. 59=Akshaya).
    ``order``: 1-based position in the 60-year cycle (1-60).
    """

    samvatsara_index: int
    order: int
    start: UtcTime
    end: UtcTime


@dataclass(frozen=True)
class PanchangResult:
    """Combined panchang result with optional calendar fields.

    Each field is ``None`` when not requested or not computed.
    """

    tithi: Optional[TithiInfo] = None
    karana: Optional[KaranaInfo] = None
    yoga: Optional[YogaInfo] = None
    vaar: Optional[VaarInfo] = None
    hora: Optional[HoraInfo] = None
    ghatika: Optional[GhatikaInfo] = None
    nakshatra: Optional[PanchangNakshatraInfo] = None
    masa: Optional[MasaInfo] = None
    ayana: Optional[AyanaInfo] = None
    varsha: Optional[VarshaInfo] = None


@dataclass(frozen=True)
class PanchangInfo:
    """Combined panchang info with all seven daily elements plus optional calendar.

    Matches ``DhruvPanchangInfo``.
    """

    tithi: TithiInfo
    karana: KaranaInfo
    yoga: YogaInfo
    vaar: VaarInfo
    hora: HoraInfo
    ghatika: GhatikaInfo
    nakshatra: PanchangNakshatraInfo
    calendar_valid: bool
    masa: Optional[MasaInfo] = None
    ayana: Optional[AyanaInfo] = None
    varsha: Optional[VarshaInfo] = None


# ---------------------------------------------------------------------------
# Sphuta
# ---------------------------------------------------------------------------


@dataclass(frozen=True)
class SphutalResult:
    """All 16 sphuta longitudes (indexed 0-15 matching ALL_SPHUTAS order)."""

    longitudes: list[float]


# ---------------------------------------------------------------------------
# Special Lagnas
# ---------------------------------------------------------------------------


@dataclass(frozen=True)
class SpecialLagnas:
    """All 8 special lagnas (sidereal degrees)."""

    bhava_lagna: float
    hora_lagna: float
    ghati_lagna: float
    vighati_lagna: float
    varnada_lagna: float
    sree_lagna: float
    pranapada_lagna: float
    indu_lagna: float


# ---------------------------------------------------------------------------
# Arudha Padas
# ---------------------------------------------------------------------------


@dataclass(frozen=True)
class ArudhaResult:
    """Single arudha pada result."""

    bhava_number: int
    longitude_deg: float
    rashi_index: int


# ---------------------------------------------------------------------------
# Upagrahas
# ---------------------------------------------------------------------------


@dataclass(frozen=True)
class AllUpagrahas:
    """All 11 upagraha sidereal longitudes."""

    gulika: float
    maandi: float
    kaala: float
    mrityu: float
    artha_prahara: float
    yama_ghantaka: float
    dhooma: float
    vyatipata: float
    parivesha: float
    indra_chapa: float
    upaketu: float


# ---------------------------------------------------------------------------
# Ashtakavarga
# ---------------------------------------------------------------------------


@dataclass(frozen=True)
class BhinnaAshtakavarga:
    """Bhinna Ashtakavarga for a single graha.

    ``graha_index``: 0=Sun through 6=Saturn.
    ``points``: benefic points per rashi (12 entries, max 8 each).
    ``contributors``: attribution matrix ``[rashi][contributor]`` (12x8, 0/1).
      Contributor order: Sun, Moon, Mars, Mercury, Jupiter, Venus, Saturn, Lagna.
    """

    graha_index: int
    points: list[int]
    contributors: list[list[int]] = field(default_factory=lambda: [[0] * 8 for _ in range(12)])


@dataclass(frozen=True)
class SarvaAshtakavarga:
    """Sarva Ashtakavarga with sodhana.

    ``total_points``: SAV per rashi (sum of all 7 BAVs).
    ``after_trikona``: after Trikona Sodhana.
    ``after_ekadhipatya``: after Ekadhipatya Sodhana.
    """

    total_points: list[int]
    after_trikona: list[int]
    after_ekadhipatya: list[int]


@dataclass(frozen=True)
class AshtakavargaResult:
    """Complete ashtakavarga result."""

    bavs: list[BhinnaAshtakavarga]
    sav: SarvaAshtakavarga


# ---------------------------------------------------------------------------
# Drishti (Planetary Aspects)
# ---------------------------------------------------------------------------


@dataclass(frozen=True)
class DrishtiEntry:
    """Single drishti (aspect) measurement between two points."""

    angular_distance: float
    base_virupa: float
    special_virupa: float
    total_virupa: float


@dataclass(frozen=True)
class GrahaDrishtiMatrix:
    """9x9 graha-to-graha drishti matrix."""

    matrix: list[list[DrishtiEntry]]


@dataclass(frozen=True)
class DrishtiResult:
    """Complete drishti result.

    ``graha_to_graha``: 9x9 matrix.
    ``graha_to_bhava``: 9x12 matrix.
    ``graha_to_lagna``: 9 entries.
    ``graha_to_bindus``: 9x19 matrix.
    """

    graha_to_graha: list[list[DrishtiEntry]]
    graha_to_bhava: list[list[DrishtiEntry]]
    graha_to_lagna: list[DrishtiEntry]
    graha_to_bindus: list[list[DrishtiEntry]]


# ---------------------------------------------------------------------------
# Graha Positions
# ---------------------------------------------------------------------------


@dataclass(frozen=True)
class GrahaEntry:
    """Single graha position entry.

    ``sidereal_longitude``: degrees [0, 360).
    ``rashi_index``: 0-based (0-11).
    ``nakshatra_index``: 0-based (0-26), 255 if not computed.
    ``pada``: 1-4, 0 if not computed.
    ``bhava_number``: 1-12, 0 if not computed.
    """

    sidereal_longitude: float
    rashi_index: int
    nakshatra_index: int
    pada: int
    bhava_number: int


@dataclass(frozen=True)
class GrahaPositions:
    """Comprehensive graha positions result.

    ``grahas``: 9 Vedic grahas indexed by graha index 0-8.
    ``lagna``: lagna entry (sentinel if not computed).
    ``outer_planets``: [Uranus, Neptune, Pluto].
    """

    grahas: list[GrahaEntry]
    lagna: GrahaEntry
    outer_planets: list[GrahaEntry]


# ---------------------------------------------------------------------------
# Core Bindus
# ---------------------------------------------------------------------------


@dataclass(frozen=True)
class BindusResult:
    """Curated sensitive points (bindus) result.

    Contains 12 arudha padas and 7 special points, each as a
    ``GrahaEntry`` with optional nakshatra/bhava enrichment.
    """

    arudha_padas: list[GrahaEntry]
    bhrigu_bindu: GrahaEntry
    pranapada_lagna: GrahaEntry
    gulika: GrahaEntry
    maandi: GrahaEntry
    hora_lagna: GrahaEntry
    ghati_lagna: GrahaEntry
    sree_lagna: GrahaEntry


# ---------------------------------------------------------------------------
# Amsha (Divisional Charts)
# ---------------------------------------------------------------------------


@dataclass(frozen=True)
class AmshaEntry:
    """Position in a divisional chart.

    ``sidereal_longitude``: degrees [0, 360).
    ``rashi_index``: 0-based (0-11).
    ``dms_degrees`` / ``dms_minutes`` / ``dms_seconds``: DMS within rashi.
    ``degrees_in_rashi``: decimal degrees within rashi [0, 30).
    """

    sidereal_longitude: float
    rashi_index: int
    dms_degrees: int
    dms_minutes: int
    dms_seconds: float
    degrees_in_rashi: float


@dataclass(frozen=True)
class AmshaChart:
    """Single amsha (divisional) chart result.

    ``amsha_code``: D-number of this chart.
    ``variation_code``: 0=default, 1=HoraCancerLeoOnly.
    """

    amsha_code: int
    variation_code: int
    grahas: list[AmshaEntry]
    lagna: AmshaEntry
    bhava_cusps: Optional[list[AmshaEntry]] = None
    arudha_padas: Optional[list[AmshaEntry]] = None
    upagrahas: Optional[list[AmshaEntry]] = None
    sphutas: Optional[list[AmshaEntry]] = None
    special_lagnas: Optional[list[AmshaEntry]] = None


# ---------------------------------------------------------------------------
# Shadbala
# ---------------------------------------------------------------------------


@dataclass(frozen=True)
class SthanaBalaBreakdown:
    """Sthana Bala sub-components."""

    uchcha: float
    saptavargaja: float
    ojhayugma: float
    kendradi: float
    drekkana: float
    total: float


@dataclass(frozen=True)
class KalaBalaBreakdown:
    """Kala Bala sub-components."""

    nathonnatha: float
    paksha: float
    tribhaga: float
    abda: float
    masa: float
    vara: float
    hora: float
    ayana: float
    yuddha: float
    total: float


@dataclass(frozen=True)
class ShadbalaEntry:
    """Shadbala for a single sapta graha.

    ``graha_index``: 0-6 (Sun through Saturn).
    ``is_strong``: True if total meets required strength.
    """

    graha_index: int
    sthana: SthanaBalaBreakdown
    dig: float
    kala: KalaBalaBreakdown
    cheshta: float
    naisargika: float
    drik: float
    total_shashtiamsas: float
    total_rupas: float
    required_strength: float
    is_strong: bool


@dataclass(frozen=True)
class ShadbalaResult:
    """Shadbala result for all 7 sapta grahas."""

    entries: list[ShadbalaEntry]


@dataclass(frozen=True)
class BhavaBalaEntry:
    """Bhava Bala for a single house."""

    bhava_number: int
    cusp_sidereal_lon: float
    rashi_index: int
    lord_graha_index: int
    bhavadhipati: float
    dig: float
    drishti: float
    occupation_bonus: float
    rising_bonus: float
    total_virupas: float
    total_rupas: float


@dataclass(frozen=True)
class BhavaBalaResult:
    """Bhava Bala result for all 12 houses."""

    entries: list[BhavaBalaEntry]


# ---------------------------------------------------------------------------
# Vimsopaka
# ---------------------------------------------------------------------------


@dataclass(frozen=True)
class VimsopakaEntry:
    """Vimsopaka Bala for a single graha.

    ``graha_index``: 0-8 (all 9 navagrahas).
    Scores for 4 varga groupings (each out of 20).
    """

    graha_index: int
    shadvarga: float
    saptavarga: float
    dashavarga: float
    shodasavarga: float


@dataclass(frozen=True)
class VimsopakaResult:
    """Vimsopaka result for all 9 navagrahas."""

    entries: list[VimsopakaEntry]


@dataclass(frozen=True)
class BalaBundleResult:
    """Combined bala surfaces for one chart."""

    shadbala: ShadbalaResult
    vimsopaka: VimsopakaResult
    ashtakavarga: AshtakavargaResult
    bhavabala: BhavaBalaResult


# ---------------------------------------------------------------------------
# Avastha (Planetary State)
# ---------------------------------------------------------------------------


@dataclass(frozen=True)
class SayanadiResult:
    """Sayanadi avastha for a single graha.

    ``avastha``: SayanadiAvastha index (0-11).
    ``sub_states``: 5 sub-state indices (Ka/Cha/Ta-retroflex/Ta-dental/Pa).
    """

    avastha: int
    sub_states: list[int]


@dataclass(frozen=True)
class GrahaAvasthas:
    """All avasthas for a single graha.

    ``baladi``: BaladiAvastha index (0-4).
    ``jagradadi``: JagradadiAvastha index (0-2).
    ``deeptadi``: DeeptadiAvastha index (0-8).
    ``lajjitadi``: LajjitadiAvastha index (0-5).
    """

    baladi: int
    jagradadi: int
    deeptadi: int
    lajjitadi: int
    sayanadi: SayanadiResult


@dataclass(frozen=True)
class AllGrahaAvasthas:
    """Avasthas for all 9 grahas."""

    entries: list[GrahaAvasthas]


# ---------------------------------------------------------------------------
# Dasha
# ---------------------------------------------------------------------------


@dataclass(frozen=True)
class DashaPeriod:
    """Single dasha period.

    ``entity_type``: 0=Graha, 1=Rashi, 2=Yogini.
    ``entity_index``: Graha (0-8), rashi (0-11), or yogini (0-7).
    ``entity_name``: exact canonical entity name when available.
    ``level``: hierarchical level (0-4).
    ``start_jd`` / ``end_jd``: JD UTC, [start, end) interval.
    ``order``: 1-indexed position among siblings.
    ``parent_idx``: index into parent level's array (0 for level 0).
    """

    entity_type: int
    entity_index: int
    start_jd: float
    end_jd: float
    level: int
    order: int
    parent_idx: int
    entity_name: Optional[str] = None


@dataclass(frozen=True)
class DashaSnapshot:
    """Dasha snapshot at a point in time (max 5 levels).

    ``system``: DashaSystem code.
    ``query_jd``: query JD UTC.
    ``periods``: one period per active level.
    """

    system: int
    query_jd: float
    periods: list[DashaPeriod]


# ---------------------------------------------------------------------------
# Charakaraka
# ---------------------------------------------------------------------------


@dataclass(frozen=True)
class CharakarakaEntry:
    """Single charakaraka assignment entry."""

    role_code: int
    graha_index: int
    rank: int
    longitude_deg: float
    degrees_in_rashi: float
    effective_degrees_in_rashi: float


@dataclass(frozen=True)
class CharakarakaResult:
    """Charakaraka assignment result for one scheme."""

    scheme: int
    used_eight_karakas: bool
    entries: list[CharakarakaEntry]


# ---------------------------------------------------------------------------
# Tara (Fixed Stars)
# ---------------------------------------------------------------------------


@dataclass(frozen=True)
class EquatorialPosition:
    """Equatorial position of a fixed star."""

    ra_deg: float
    dec_deg: float
    distance_au: float


@dataclass(frozen=True)
class EarthState:
    """Earth state vector in AU and AU/day."""

    position_au: list[float]
    velocity_au_day: list[float]


@dataclass(frozen=True)
class TaraComputeResult:
    """Unified tara (fixed star) computation result.

    ``output_kind``: 0=equatorial, 1=ecliptic, 2=sidereal.
    Only the field matching ``output_kind`` is meaningful.
    """

    output_kind: int
    equatorial: Optional[EquatorialPosition] = None
    ecliptic: Optional[SphericalCoords] = None
    sidereal_longitude_deg: Optional[float] = None


# ---------------------------------------------------------------------------
# Graha Longitudes
# ---------------------------------------------------------------------------


@dataclass(frozen=True)
class GrahaLongitudes:
    """Sidereal or tropical longitudes for all 9 grahas.

    Indexed by Graha order: Surya=0, Chandra=1, Mangal=2, Buddh=3,
    Guru=4, Shukra=5, Shani=6, Rahu=7, Ketu=8.
    """

    longitudes: list[float]


# ---------------------------------------------------------------------------
# Full Kundali
# ---------------------------------------------------------------------------


@dataclass(frozen=True)
class FullKundaliResult:
    """Complete kundali (birth chart) result.

    Each section is ``None`` when not requested or computation failed.
    """

    ayanamsha_deg: float
    bhava_cusps: Optional[BhavaResult] = None
    graha_positions: Optional[GrahaPositions] = None
    bindus: Optional[BindusResult] = None
    drishti: Optional[DrishtiResult] = None
    ashtakavarga: Optional[AshtakavargaResult] = None
    upagrahas: Optional[AllUpagrahas] = None
    sphutas: Optional[SphutalResult] = None
    special_lagnas: Optional[SpecialLagnas] = None
    amshas: Optional[list[AmshaChart]] = None
    shadbala: Optional[ShadbalaResult] = None
    bhavabala: Optional[BhavaBalaResult] = None
    vimsopaka: Optional[VimsopakaResult] = None
    avastha: Optional[AllGrahaAvasthas] = None
    charakaraka: Optional[CharakarakaResult] = None
    panchang: Optional[PanchangInfo] = None
    dasha: Optional[list[DashaHierarchy]] = None
    dasha_snapshots: Optional[list[DashaSnapshot]] = None
