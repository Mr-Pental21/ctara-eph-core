"""IntEnum classes mirroring C ABI constants from dhruv_ffi_c.

Every value here MUST match the C ABI exactly.  See docs/C_ABI_REFERENCE.md
and crates/dhruv_ffi_c/src/lib.rs for the canonical definitions.
"""

from enum import IntEnum

__all__ = [
    "Body",
    "DhruvStatus",
    "AyanamshaSystem",
    "BhavaSystem",
    "Graha",
    "SunLimb",
    "RiseSetEvent",
    "RiseSetResultType",
    "StationType",
    "MaxSpeedType",
    "DashaSystem",
    "ReferencePlane",
    "PrecessionModel",
    "GrahaLongitudeKind",
    "SearchQueryMode",
    "GrahanKind",
    "MotionKind",
    "LunarPhaseKind",
    "SankrantiTargetKind",
    "NakshatraScheme",
    "CharakarakaScheme",
    "CharakarakaRole",
    "ChandraGrahanType",
    "SuryaGrahanType",
    "LunarNodeType",
    "LunarNodeMode",
    "AyanamshaMode",
    "TaraOutputKind",
]


# ---------------------------------------------------------------------------
# SPK body codes (NAIF convention)
# ---------------------------------------------------------------------------

class Body(IntEnum):
    """NAIF/SPK body codes used by the ephemeris engine."""

    SSB = 0
    MERCURY_BARYCENTER = 1
    VENUS_BARYCENTER = 2
    EARTH_BARYCENTER = 3
    MARS_BARYCENTER = 4
    JUPITER_BARYCENTER = 5
    SATURN_BARYCENTER = 6
    URANUS_BARYCENTER = 7
    NEPTUNE_BARYCENTER = 8
    PLUTO_BARYCENTER = 9
    SUN = 10
    MOON = 301
    MERCURY = 199
    VENUS = 299
    EARTH = 399
    MARS_BODY = 499
    JUPITER_BODY = 599
    SATURN_BODY = 699
    URANUS_BODY = 799
    NEPTUNE_BODY = 899
    PLUTO_BODY = 999

    # DE442s resolves Mars/Jupiter/Saturn to barycenters
    MARS = 4
    JUPITER = 5
    SATURN = 6


# ---------------------------------------------------------------------------
# Status codes
# ---------------------------------------------------------------------------

class DhruvStatus(IntEnum):
    """C ABI return codes (DhruvStatus repr(i32))."""

    OK = 0
    INVALID_CONFIG = 1
    INVALID_QUERY = 2
    KERNEL_LOAD = 3
    TIME_CONVERSION = 4
    UNSUPPORTED_QUERY = 5
    EPOCH_OUT_OF_RANGE = 6
    NULL_POINTER = 7
    EOP_LOAD = 8
    EOP_OUT_OF_RANGE = 9
    INVALID_LOCATION = 10
    NO_CONVERGENCE = 11
    INVALID_SEARCH_CONFIG = 12
    INVALID_INPUT = 13
    INTERNAL = 255


# ---------------------------------------------------------------------------
# Ayanamsha systems  (index into AyanamshaSystem::all())
# ---------------------------------------------------------------------------

class AyanamshaSystem(IntEnum):
    """20 ayanamsha systems.  Codes match C ABI (0-19)."""

    LAHIRI = 0
    TRUE_LAHIRI = 1
    KP = 2
    RAMAN = 3
    FAGAN_BRADLEY = 4
    PUSHYA_PAKSHA = 5
    ROHINI_PAKSHA = 6
    DELUCE = 7
    DJWAL_KHUL = 8
    HIPPARCHOS = 9
    SASSANIAN = 10
    DEVA_DUTTA = 11
    USHA_SHASHI = 12
    YUKTESHWAR = 13
    JN_BHASIN = 14
    CHANDRA_HARI = 15
    JAGGANATHA = 16
    SURYA_SIDDHANTA = 17
    GALACTIC_CENTER_0_SAG = 18
    ALDEBARAN_15_TAU = 19


# ---------------------------------------------------------------------------
# Ayanamsha mode  (DHRUV_AYANAMSHA_MODE_*)
# ---------------------------------------------------------------------------

class AyanamshaMode(IntEnum):
    """Ayanamsha computation mode."""

    MEAN = 0
    TRUE = 1
    UNIFIED = 2


# ---------------------------------------------------------------------------
# Bhava (house) systems  (DHRUV_BHAVA_* constants)
# ---------------------------------------------------------------------------

class BhavaSystem(IntEnum):
    """10 bhava (house) systems.  Codes match C ABI (0-9)."""

    EQUAL = 0
    SURYA_SIDDHANTA = 1
    SRIPATI = 2
    KP = 3
    KOCH = 4
    REGIOMONTANUS = 5
    CAMPANUS = 6
    AXIAL_ROTATION = 7
    TOPOCENTRIC = 8
    ALCABITUS = 9


# ---------------------------------------------------------------------------
# Graha  (navagraha, 0-8)
# ---------------------------------------------------------------------------

class Graha(IntEnum):
    """9 grahas in traditional order (Surya..Ketu)."""

    SURYA = 0
    CHANDRA = 1
    MANGAL = 2
    BUDDH = 3
    GURU = 4
    SHUKRA = 5
    SHANI = 6
    RAHU = 7
    KETU = 8

    # Western-name aliases
    SUN = 0
    MOON = 1
    MARS = 2
    MERCURY = 3
    JUPITER = 4
    VENUS = 5
    SATURN = 6


# ---------------------------------------------------------------------------
# Sun limb  (DHRUV_SUN_LIMB_*)
# ---------------------------------------------------------------------------

class SunLimb(IntEnum):
    """Which part of the solar disk defines rise/set."""

    UPPER = 0
    CENTER = 1
    LOWER = 2


# ---------------------------------------------------------------------------
# Rise/set event codes  (DHRUV_EVENT_*)
# ---------------------------------------------------------------------------

class RiseSetEvent(IntEnum):
    """Rise/set event types."""

    SUNRISE = 0
    SUNSET = 1
    CIVIL_DAWN = 2
    CIVIL_DUSK = 3
    NAUTICAL_DAWN = 4
    NAUTICAL_DUSK = 5
    ASTRONOMICAL_DAWN = 6
    ASTRONOMICAL_DUSK = 7


# ---------------------------------------------------------------------------
# Rise/set result type  (DHRUV_RISESET_*)
# ---------------------------------------------------------------------------

class RiseSetResultType(IntEnum):
    """Outcome of a rise/set computation."""

    EVENT = 0
    NEVER_RISES = 1
    NEVER_SETS = 2


# ---------------------------------------------------------------------------
# Station type  (DHRUV_STATION_*)
# ---------------------------------------------------------------------------

class StationType(IntEnum):
    """Stationary-point direction."""

    RETROGRADE = 0
    DIRECT = 1


# ---------------------------------------------------------------------------
# Max speed type  (DHRUV_MAX_SPEED_*)
# ---------------------------------------------------------------------------

class MaxSpeedType(IntEnum):
    """Peak speed direction."""

    DIRECT = 0
    RETROGRADE = 1


# ---------------------------------------------------------------------------
# Dasha systems  (DashaSystem repr(u8), 0-22)
# ---------------------------------------------------------------------------

class DashaSystem(IntEnum):
    """23 dasha systems.  Codes match DashaSystem repr(u8)."""

    # Nakshatra-based (10)
    VIMSHOTTARI = 0
    ASHTOTTARI = 1
    SHODSOTTARI = 2
    DWADASHOTTARI = 3
    PANCHOTTARI = 4
    SHATABDIKA = 5
    CHATURASHITI = 6
    DWISAPTATI_SAMA = 7
    SHASHTIHAYANI = 8
    SHAT_TRIMSHA_SAMA = 9
    # Yogini (1)
    YOGINI = 10
    # Rashi-based (7)
    CHARA = 11
    STHIRA = 12
    YOGARDHA = 13
    DRIGA = 14
    SHOOLA = 15
    MANDOOKA = 16
    CHAKRA = 17
    # Graha-based (1)
    KALA = 18
    # Special (1)
    KAAL_CHAKRA = 19
    # Kendradi variants (3)
    KENDRADI = 20
    KARAKA_KENDRADI = 21
    KARAKA_KENDRADI_GRAHA = 22


# ---------------------------------------------------------------------------
# Charakaraka scheme  (DHRUV_CHARAKARAKA_SCHEME_*)
# ---------------------------------------------------------------------------

class CharakarakaScheme(IntEnum):
    """Charakaraka assignment scheme."""

    EIGHT = 0
    SEVEN_NO_PITRI = 1
    SEVEN_PK_MERGED_MK = 2
    MIXED_PARASHARA = 3


# ---------------------------------------------------------------------------
# Charakaraka role  (DHRUV_CHARAKARAKA_ROLE_*)
# ---------------------------------------------------------------------------

class CharakarakaRole(IntEnum):
    """Charakaraka role code."""

    ATMA = 0
    AMATYA = 1
    BHRATRI = 2
    MATRI = 3
    PITRI = 4
    PUTRA = 5
    GNATI = 6
    DARA = 7
    MATRI_PUTRA = 8


# ---------------------------------------------------------------------------
# Reference plane  (DhruvReferencePlane repr(i32))
# ---------------------------------------------------------------------------

class ReferencePlane(IntEnum):
    """Reference plane for positional measurements."""

    ECLIPTIC = 0
    INVARIABLE = 1


class PrecessionModel(IntEnum):
    """Precession-model selector."""

    NEWCOMB1895 = 0
    LIESKE1977 = 1
    IAU2006 = 2
    VONDRAK2011 = 3


class GrahaLongitudeKind(IntEnum):
    """Graha longitude output selector."""

    SIDEREAL = 0
    TROPICAL = 1


# ---------------------------------------------------------------------------
# Unified search query mode
# ---------------------------------------------------------------------------

class SearchQueryMode(IntEnum):
    """Direction/range for search queries."""

    NEXT = 0
    PREV = 1
    RANGE = 2


# ---------------------------------------------------------------------------
# Grahan (eclipse) kind  (DHRUV_GRAHAN_KIND_*)
# ---------------------------------------------------------------------------

class GrahanKind(IntEnum):
    """Eclipse family."""

    CHANDRA = 0
    SURYA = 1


# ---------------------------------------------------------------------------
# Chandra grahan subtype  (DHRUV_CHANDRA_GRAHAN_*)
# ---------------------------------------------------------------------------

class ChandraGrahanType(IntEnum):
    """Lunar eclipse subtype."""

    PENUMBRAL = 0
    PARTIAL = 1
    TOTAL = 2


# ---------------------------------------------------------------------------
# Surya grahan subtype  (DHRUV_SURYA_GRAHAN_*)
# ---------------------------------------------------------------------------

class SuryaGrahanType(IntEnum):
    """Solar eclipse subtype."""

    PARTIAL = 0
    ANNULAR = 1
    TOTAL = 2
    HYBRID = 3


# ---------------------------------------------------------------------------
# Motion kind  (DHRUV_MOTION_KIND_*)
# ---------------------------------------------------------------------------

class MotionKind(IntEnum):
    """Stationary vs max-speed search."""

    STATIONARY = 0
    MAX_SPEED = 1


# ---------------------------------------------------------------------------
# Lunar phase kind  (DHRUV_LUNAR_PHASE_KIND_*)
# ---------------------------------------------------------------------------

class LunarPhaseKind(IntEnum):
    """Full/new Moon target."""

    AMAVASYA = 0
    PURNIMA = 1


# ---------------------------------------------------------------------------
# Sankranti target kind  (DHRUV_SANKRANTI_TARGET_*)
# ---------------------------------------------------------------------------

class SankrantiTargetKind(IntEnum):
    """Sankranti search scope."""

    ANY = 0
    SPECIFIC = 1


# ---------------------------------------------------------------------------
# Nakshatra scheme
# ---------------------------------------------------------------------------

class NakshatraScheme(IntEnum):
    """27- or 28-nakshatra scheme (with/without Abhijit)."""

    NAKSHATRA_27 = 0
    NAKSHATRA_28 = 1


# ---------------------------------------------------------------------------
# Lunar node type  (DHRUV_NODE_*)
# ---------------------------------------------------------------------------

class LunarNodeType(IntEnum):
    """Ascending or descending node."""

    RAHU = 0
    KETU = 1


# ---------------------------------------------------------------------------
# Lunar node mode  (DHRUV_NODE_MODE_*)
# ---------------------------------------------------------------------------

class LunarNodeMode(IntEnum):
    """Mean vs true node computation."""

    MEAN = 0
    TRUE = 1


# ---------------------------------------------------------------------------
# Tara output kind  (DHRUV_TARA_OUTPUT_*)
# ---------------------------------------------------------------------------

class TaraOutputKind(IntEnum):
    """Coordinate system for tara (fixed star) output."""

    EQUATORIAL = 0
    ECLIPTIC = 1
    SIDEREAL = 2
