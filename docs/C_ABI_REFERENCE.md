# C ABI Reference

Complete reference for the `dhruv_ffi_c` C-compatible API surface.

**ABI version:** `DHRUV_API_VERSION = 61`

**Library:** `libdhruv_ffi_c` (compiled as `cdylib` + `staticlib`)

**Release build artifacts:**
- GitHub Releases ship zipped C ABI bundles per supported target.
- Each bundle contains `include/dhruv.h`, platform libraries under `lib/`, and `SHA256SUMS.txt`.
- Local development build command: `cargo build -p dhruv_ffi_c --release`

---

## Table of Contents

1. [Conventions](#conventions)
2. [Status Codes](#status-codes)
3. [Constants](#constants)
4. [Types](#types)
5. [Functions](#functions)
   - [Versioning](#versioning)
   - [Engine Lifecycle](#engine-lifecycle)
   - [Layered Config](#layered-config)
   - [Ephemeris Query](#ephemeris-query)
   - [Time Conversion](#time-conversion)
   - [Coordinate Conversion](#coordinate-conversion)
   - [LSK / EOP Handles](#lsk--eop-handles)
   - [Ayanamsha](#ayanamsha)
   - [Nutation](#nutation)
   - [Lunar Nodes](#lunar-nodes)
   - [Sunrise / Sunset](#sunrise--sunset)
   - [Bhava (House Systems)](#bhava-house-systems)
   - [Conjunction / Aspect Search](#conjunction--aspect-search)
   - [Chandra Grahan](#chandra-grahan)
   - [Surya Grahan](#surya-grahan)
   - [Stationary Point Search](#stationary-point-search)
   - [Max Speed Search](#max-speed-search)
   - [RAMC](#ramc)
   - [Unified Panchang Compute](#unified-panchang-compute)
   - [Pure-Math Panchang Classifiers](#pure-math-panchang-classifiers)
   - [Graha Sidereal Longitudes](#graha-sidereal-longitudes)
   - [Graha Tropical Longitudes](#graha-tropical-longitudes)
   - [Nakshatra At](#nakshatra-at)
   - [Time Upagraha JD](#time-upagraha-jd)
   - [Pure-Math Ashtakavarga](#pure-math-ashtakavarga)
   - [Pure-Math Drishti](#pure-math-drishti)
   - [Pure-Math Ghatika / Hora](#pure-math-ghatika--hora)
   - [Amsha (Divisional Charts)](#amsha-divisional-charts)
   - [Fixed Stars (Tara)](#fixed-stars-tara)

---

## Conventions

- All functions return `DhruvStatus` (int32) unless stated otherwise.
- Required null pointer arguments return `DHRUV_STATUS_NULL_POINTER` (7).
- For operation config pointers, `NULL` is accepted and resolved through layered config fallback.
- Out-of-range enum codes return `DHRUV_STATUS_INVALID_QUERY` (2).
- Opaque handles are heap-allocated; always pair `*_load`/`*_new` with `*_free`.
- No panics cross the FFI boundary (caught by `catch_unwind`).
- Caller allocates all output buffers.
- Path strings are NUL-terminated UTF-8.
- Time epochs are Julian Date in TDB unless noted otherwise.
- Angles are in degrees unless a `_rad` suffix is present.
- Optional JD fields use sentinel `DHRUV_JD_ABSENT = -1.0` when absent.

---

## Status Codes

```c
enum DhruvStatus {
    DHRUV_STATUS_OK                = 0,
    DHRUV_STATUS_INVALID_CONFIG    = 1,
    DHRUV_STATUS_INVALID_QUERY     = 2,
    DHRUV_STATUS_KERNEL_LOAD       = 3,
    DHRUV_STATUS_TIME_CONVERSION   = 4,
    DHRUV_STATUS_UNSUPPORTED_QUERY = 5,
    DHRUV_STATUS_EPOCH_OUT_OF_RANGE= 6,
    DHRUV_STATUS_NULL_POINTER      = 7,
    DHRUV_STATUS_EOP_LOAD          = 8,
    DHRUV_STATUS_EOP_OUT_OF_RANGE  = 9,
    DHRUV_STATUS_INVALID_LOCATION  = 10,
    DHRUV_STATUS_NO_CONVERGENCE    = 11,
    DHRUV_STATUS_INVALID_SEARCH_CONFIG = 12,
    DHRUV_STATUS_INTERNAL          = 255,
};
```

---

## Constants

### Path Limits

| Constant | Value | Description |
|----------|-------|-------------|
| `DHRUV_PATH_CAPACITY` | 512 | Max bytes per path field (including NUL) |
| `DHRUV_MAX_SPK_PATHS` | 8 | Max number of SPK kernel paths |

### Sun Limb

| Constant | Value | Description |
|----------|-------|-------------|
| `DHRUV_SUN_LIMB_UPPER` | 0 | Upper limb (conventional sunrise/sunset) |
| `DHRUV_SUN_LIMB_CENTER` | 1 | Center of solar disk |
| `DHRUV_SUN_LIMB_LOWER` | 2 | Lower limb |

### Rise/Set Result Types

| Constant | Value | Description |
|----------|-------|-------------|
| `DHRUV_RISESET_EVENT` | 0 | Event occurred, `jd_tdb` is valid |
| `DHRUV_RISESET_NEVER_RISES` | 1 | Polar night (Sun never rises) |
| `DHRUV_RISESET_NEVER_SETS` | 2 | Midnight sun (Sun never sets) |

### Rise/Set Event Codes

| Constant | Value |
|----------|-------|
| `DHRUV_EVENT_SUNRISE` | 0 |
| `DHRUV_EVENT_SUNSET` | 1 |
| `DHRUV_EVENT_CIVIL_DAWN` | 2 |
| `DHRUV_EVENT_CIVIL_DUSK` | 3 |
| `DHRUV_EVENT_NAUTICAL_DAWN` | 4 |
| `DHRUV_EVENT_NAUTICAL_DUSK` | 5 |
| `DHRUV_EVENT_ASTRONOMICAL_DAWN` | 6 |
| `DHRUV_EVENT_ASTRONOMICAL_DUSK` | 7 |

### Lunar Node Codes

| Constant | Value | Description |
|----------|-------|-------------|
| `DHRUV_NODE_RAHU` | 0 | Ascending node |
| `DHRUV_NODE_KETU` | 1 | Descending node |
| `DHRUV_NODE_MODE_MEAN` | 0 | Mean (polynomial only) |
| `DHRUV_NODE_MODE_TRUE` | 1 | True node mode (osculating in engine-aware APIs; 50-term fitted series in pure APIs) |

### Bhava (House) System Codes

| Constant | Value | System |
|----------|-------|--------|
| `DHRUV_BHAVA_EQUAL` | 0 | Equal house |
| `DHRUV_BHAVA_SURYA_SIDDHANTA` | 1 | Surya Siddhanta |
| `DHRUV_BHAVA_SRIPATI` | 2 | Sripati |
| `DHRUV_BHAVA_KP` | 3 | KP (Placidus) |
| `DHRUV_BHAVA_KOCH` | 4 | Koch |
| `DHRUV_BHAVA_REGIOMONTANUS` | 5 | Regiomontanus |
| `DHRUV_BHAVA_CAMPANUS` | 6 | Campanus |
| `DHRUV_BHAVA_AXIAL_ROTATION` | 7 | Axial Rotation |
| `DHRUV_BHAVA_TOPOCENTRIC` | 8 | Topocentric |
| `DHRUV_BHAVA_ALCABITUS` | 9 | Alcabitus |

### Bhava Reference Mode

| Constant | Value | Description |
|----------|-------|-------------|
| `DHRUV_BHAVA_REF_START` | 0 | Starting point is start of first bhava |
| `DHRUV_BHAVA_REF_MIDDLE` | 1 | Starting point is middle of first bhava |

### Bhava Starting Point

| Constant | Value | Description |
|----------|-------|-------------|
| `DHRUV_BHAVA_START_LAGNA` | -1 | Use Lagna (Ascendant) as starting point |
| `DHRUV_BHAVA_START_CUSTOM` | -2 | Use `custom_start_deg` field |
| *(positive value)* | NAIF code | Use ecliptic longitude of specified body |

### Ayanamsha System Codes

| Code | System |
|------|--------|
| 0 | Lahiri |
| 1 | True Lahiri |
| 2 | KP (Krishnamurti) |
| 3 | Raman |
| 4 | Fagan-Bradley |
| 5 | Pushya Paksha |
| 6 | Rohini Paksha |
| 7 | DeLuce |
| 8 | Djwal Khul |
| 9 | Hipparchos |
| 10 | Sassanian |
| 11 | Deva Dutta |
| 12 | Usha Shashi |
| 13 | Yukteshwar |
| 14 | JN Bhasin |
| 15 | Chandra Hari |
| 16 | Jagganatha |
| 17 | Surya Siddhanta |
| 18 | Galactic Center 0 Sag |
| 19 | Aldebaran 15 Tau |

### Grahan Type Codes

**Chandra Grahan:**

| Constant | Value |
|----------|-------|
| `DHRUV_CHANDRA_GRAHAN_PENUMBRAL` | 0 |
| `DHRUV_CHANDRA_GRAHAN_PARTIAL` | 1 |
| `DHRUV_CHANDRA_GRAHAN_TOTAL` | 2 |

**Surya Grahan:**

| Constant | Value |
|----------|-------|
| `DHRUV_SURYA_GRAHAN_PARTIAL` | 0 |
| `DHRUV_SURYA_GRAHAN_ANNULAR` | 1 |
| `DHRUV_SURYA_GRAHAN_TOTAL` | 2 |
| `DHRUV_SURYA_GRAHAN_HYBRID` | 3 |

### Stationary Point Type Codes

| Constant | Value | Description |
|----------|-------|-------------|
| `DHRUV_STATION_RETROGRADE` | 0 | Planet begins retrograde motion |
| `DHRUV_STATION_DIRECT` | 1 | Planet resumes direct motion |

### Max Speed Type Codes

| Constant | Value | Description |
|----------|-------|-------------|
| `DHRUV_MAX_SPEED_DIRECT` | 0 | Peak forward (direct) speed |
| `DHRUV_MAX_SPEED_RETROGRADE` | 1 | Peak retrograde speed |

### Sentinel Values

| Constant | Value | Description |
|----------|-------|-------------|
| `DHRUV_JD_ABSENT` | -1.0 | Absent optional JD field |

---

## Types

### Opaque Handles

```c
typedef struct DhruvEngineHandle DhruvEngineHandle;  // Ephemeris engine
typedef struct DhruvLskHandle    DhruvLskHandle;     // Leap second kernel
typedef struct DhruvEopHandle    DhruvEopHandle;     // IERS Earth orientation parameters
```

### DhruvEngineConfig

```c
typedef struct {
    uint32_t spk_path_count;
    uint8_t  spk_paths_utf8[DHRUV_MAX_SPK_PATHS][DHRUV_PATH_CAPACITY];
    uint8_t  lsk_path_utf8[DHRUV_PATH_CAPACITY];
    uint64_t cache_capacity;
    uint8_t  strict_validation;  // 0 = false, 1 = true
} DhruvEngineConfig;
```

### DhruvQuery

```c
typedef struct {
    int32_t target;       // NAIF body code
    int32_t observer;     // NAIF body code
    int32_t frame;        // Frame code (0 = J2000/ICRF, 1 = ecliptic J2000)
    double  epoch_tdb_jd; // Julian Date in TDB
} DhruvQuery;
```

### DhruvStateVector

```c
typedef struct {
    double position_km[3];
    double velocity_km_s[3];
} DhruvStateVector;
```

### DhruvSphericalCoords

```c
typedef struct {
    double lon_deg;      // Longitude [0, 360)
    double lat_deg;      // Latitude [-90, 90]
    double distance_km;
} DhruvSphericalCoords;
```

### DhruvSphericalState

```c
typedef struct {
    double lon_deg;         // Longitude [0, 360)
    double lat_deg;         // Latitude [-90, 90]
    double distance_km;
    double lon_speed;       // deg/day
    double lat_speed;       // deg/day
    double distance_speed;  // km/s
} DhruvSphericalState;
```

### DhruvUtcTime

```c
typedef struct {
    int32_t  year;
    uint32_t month;    // 1-12
    uint32_t day;      // 1-31
    uint32_t hour;     // 0-23
    uint32_t minute;   // 0-59
    double   second;   // 0.0-59.999...
} DhruvUtcTime;
```

### DhruvGeoLocation

```c
typedef struct {
    double latitude_deg;    // [-90, 90], north positive
    double longitude_deg;   // [-180, 180], east positive
    double altitude_m;      // Meters above sea level
} DhruvGeoLocation;
```

### DhruvRiseSetConfig

```c
typedef struct {
    uint8_t use_refraction;      // 1 = apply 34' atmospheric refraction
    int32_t sun_limb;            // DHRUV_SUN_LIMB_* constant
    uint8_t altitude_correction; // 1 = apply dip correction
} DhruvRiseSetConfig;
```

### DhruvRiseSetResult

```c
typedef struct {
    int32_t result_type;  // DHRUV_RISESET_* constant
    int32_t event_code;   // DHRUV_EVENT_* constant (valid when result_type == 0)
    double  jd_tdb;       // Event time in JD TDB (valid when result_type == 0)
} DhruvRiseSetResult;
```

### DhruvBhavaConfig

```c
typedef struct {
    int32_t system;            // DHRUV_BHAVA_* system code (0-9)
    int32_t starting_point;    // -1=Asc, -2=custom deg, or positive NAIF body code
    double  custom_start_deg;  // Used only when starting_point == -2
    int32_t reference_mode;    // DHRUV_BHAVA_REF_* constant
    int32_t output_mode;       // 0=tropical, 1=sidereal
    int32_t ayanamsha_system;  // used when output_mode == sidereal
    uint8_t use_nutation;
    int32_t reference_plane;
    uint8_t use_rashi_bhava_for_bala_avastha; // default 1
    uint8_t include_node_aspects_for_drik_bala; // default 0
    uint8_t divide_guru_buddh_drishti_by_4_for_drik_bala; // default 1
    int32_t chandra_benefic_rule; // default 0 = DHRUV_CHANDRA_BENEFIC_RULE_BRIGHTNESS_72
    uint8_t include_rashi_bhava_results;      // default 1
} DhruvBhavaConfig;
```

Existing bhava result fields keep the configured bhava-system meaning. High-level jyotish result structs add rashi-bhava sibling fields where applicable, such as `rashi_bhava_number`, `rashi_bhava_cusps`, `graha_to_rashi_bhava`, and rashi-bhava amsha cusp/arudha arrays. See `docs/dual_rashi_bhava_outputs.md`.

`include_node_aspects_for_drik_bala` controls only Shadbala Drik Bala. It
defaults to `0`, so Rahu/Ketu incoming aspects are excluded from the Shadbala
Drik Bala balance unless explicitly enabled. Drishti matrix endpoints are
unchanged and continue to expose node aspects.

`divide_guru_buddh_drishti_by_4_for_drik_bala` also controls only Shadbala
Drik Bala. It defaults to `1`, so Guru and Buddh incoming aspects participate
in the divided `(benefic - malefic) / 4` balance. Set it to `0` to add their
signed incoming aspects at full strength after the divided balance.

`chandra_benefic_rule` controls Chandra's dynamic benefic/malefic
classification for Shadbala nature-dependent calculations. Code `0`
(`DHRUV_CHANDRA_BENEFIC_RULE_BRIGHTNESS_72`) is the default 72-degree
brightness rule; code `1` (`DHRUV_CHANDRA_BENEFIC_RULE_WAXING_180`) uses the
0..=180-degree waxing arc rule.

### DhruvBhava

```c
typedef struct {
    uint8_t number;     // Bhava number (1-12)
    double  cusp_deg;   // Cusp longitude in degrees [0, 360)
    double  start_deg;  // Start of bhava in degrees
    double  end_deg;    // End of bhava in degrees
} DhruvBhava;
```

### DhruvBhavaResult

```c
typedef struct {
    DhruvBhava bhavas[12];
    double     lagna_deg;      // Lagna (Ascendant) longitude in degrees
    double     mc_deg;         // MC (Midheaven) longitude in degrees
    uint8_t    rashi_bhava_valid;
    DhruvBhava rashi_bhava_bhavas[12];
    double     rashi_bhava_lagna_deg;
    double     rashi_bhava_mc_deg;
} DhruvBhavaResult;
```

`bhavas`, `lagna_deg`, and `mc_deg` are always the configured bhava-system output. `dhruv_compute_bhavas*` also fills the rashi-bhava sibling fields when `include_rashi_bhava_results` is non-zero.

### DhruvConjunctionConfig

```c
typedef struct {
    double   target_separation_deg; // 0=conjunction, 180=opposition, 90=square
    double   step_size_days;        // Coarse scan step (default 0.5)
    uint32_t max_iterations;        // Max bisection iterations (default 50)
    double   convergence_days;      // Convergence threshold (default 1e-8)
} DhruvConjunctionConfig;
```

### DhruvConjunctionEvent

```c
typedef struct {
    double  jd_tdb;                // Event time (JD TDB)
    DhruvUtcTime utc;              // Structured Gregorian UTC alongside JD
    double  actual_separation_deg; // Actual separation at peak
    double  body1_longitude_deg;   // Body 1 ecliptic longitude
    double  body2_longitude_deg;   // Body 2 ecliptic longitude
    double  body1_latitude_deg;    // Body 1 ecliptic latitude
    double  body2_latitude_deg;    // Body 2 ecliptic latitude
    int32_t body1_code;            // Body 1 NAIF code
    int32_t body2_code;            // Body 2 NAIF code
} DhruvConjunctionEvent;
```

### DhruvGrahanConfig

```c
typedef struct {
    uint8_t include_penumbral;    // 1 = include penumbral-only chandra grahan
    uint8_t include_peak_details; // 1 = include lat/separation at peak
} DhruvGrahanConfig;
```

### DhruvChandraGrahanResult

```c
typedef struct {
    int32_t grahan_type;            // DHRUV_CHANDRA_GRAHAN_* constant
    double  magnitude;              // Umbral magnitude
    double  penumbral_magnitude;
    double  greatest_grahan_jd;     // JD TDB
    DhruvUtcTime greatest_grahan_utc;
    double  p1_jd;                  // First penumbral contact
    DhruvUtcTime p1_utc;
    double  u1_jd;                  // First umbral contact (-1.0 if absent)
    DhruvUtcTime u1_utc;            // Zeroed when corresponding JD is absent
    double  u2_jd;                  // Start of totality (-1.0 if absent)
    DhruvUtcTime u2_utc;            // Zeroed when corresponding JD is absent
    double  u3_jd;                  // End of totality (-1.0 if absent)
    DhruvUtcTime u3_utc;            // Zeroed when corresponding JD is absent
    double  u4_jd;                  // Last umbral contact (-1.0 if absent)
    DhruvUtcTime u4_utc;            // Zeroed when corresponding JD is absent
    double  p4_jd;                  // Last penumbral contact
    DhruvUtcTime p4_utc;
    double  moon_ecliptic_lat_deg;  // Moon lat at greatest grahan
    double  angular_separation_deg; // Separation at greatest grahan
} DhruvChandraGrahanResult;
```

### DhruvSuryaGrahanResult

```c
typedef struct {
    int32_t grahan_type;            // DHRUV_SURYA_GRAHAN_* constant
    double  magnitude;              // Moon/Sun apparent diameter ratio
    double  greatest_grahan_jd;     // JD TDB
    DhruvUtcTime greatest_grahan_utc;
    double  c1_jd;                  // First external contact (-1.0 if absent)
    DhruvUtcTime c1_utc;            // Zeroed when corresponding JD is absent
    double  c2_jd;                  // First internal contact (-1.0 if absent)
    DhruvUtcTime c2_utc;            // Zeroed when corresponding JD is absent
    double  c3_jd;                  // Last internal contact (-1.0 if absent)
    DhruvUtcTime c3_utc;            // Zeroed when corresponding JD is absent
    double  c4_jd;                  // Last external contact (-1.0 if absent)
    DhruvUtcTime c4_utc;            // Zeroed when corresponding JD is absent
    double  moon_ecliptic_lat_deg;  // Moon lat at greatest grahan
    double  angular_separation_deg; // Separation at greatest grahan
} DhruvSuryaGrahanResult;
```

### DhruvStationaryConfig

```c
typedef struct {
    double   step_size_days;        // Coarse scan step (default 1.0)
    uint32_t max_iterations;        // Max bisection iterations (default 50)
    double   convergence_days;      // Convergence threshold (default 1e-8)
    double   numerical_step_days;   // Central difference step (default 0.01)
} DhruvStationaryConfig;
```

### DhruvStationaryEvent

```c
typedef struct {
    double  jd_tdb;          // Event time (JD TDB)
    DhruvUtcTime utc;        // Structured Gregorian UTC alongside JD
    int32_t body_code;       // NAIF body code
    double  longitude_deg;   // Ecliptic longitude at station
    double  latitude_deg;    // Ecliptic latitude at station
    int32_t station_type;    // DHRUV_STATION_* constant
} DhruvStationaryEvent;
```

### DhruvMaxSpeedEvent

```c
typedef struct {
    double  jd_tdb;              // Event time (JD TDB)
    DhruvUtcTime utc;            // Structured Gregorian UTC alongside JD
    int32_t body_code;           // NAIF body code
    double  longitude_deg;       // Ecliptic longitude at peak speed
    double  latitude_deg;        // Ecliptic latitude at peak speed
    double  speed_deg_per_day;   // Longitude speed at peak (deg/day)
    int32_t speed_type;          // DHRUV_MAX_SPEED_* constant
} DhruvMaxSpeedEvent;
```

### DhruvSankrantiConfig

```c
typedef struct {
    int32_t  ayanamsha_system;   // System code (0-19)
    uint8_t  use_nutation;       // 0=false, 1=true
    int32_t  reference_plane;    // 0=Ecliptic, 1=Invariable, -1=system default
    double   step_size_days;     // Coarse scan step (default 1.0)
    uint32_t max_iterations;     // Max bisection iterations (default 50)
    double   convergence_days;   // Convergence threshold (default 1e-8)
} DhruvSankrantiConfig;
```

Configuration for Sankranti search, panchang, dasha, and related functions.
The `reference_plane` field controls which plane longitudes and ayanamsha are
measured on. Set to -1 to use the system's default (Ecliptic for most systems,
Invariable for Jagganatha). Obtain defaults via `dhruv_sankranti_config_default()`.

### DhruvSankrantiEvent

```c
typedef struct {
    DhruvUtcTime utc;                    // Event time (UTC)
    int32_t      rashi_index;            // 0-based (0=Mesha .. 11=Meena)
    double       sun_sidereal_longitude_deg;  // On configured reference plane
    double       sun_tropical_longitude_deg;  // Always ecliptic tropical
} DhruvSankrantiEvent;
```

### DhruvLunarPhaseEvent

```c
typedef struct {
    DhruvUtcTime utc;               // Event time (UTC)
    int32_t      phase;             // DHRUV_LUNAR_PHASE_NEW_MOON or _FULL_MOON
    double       moon_longitude_deg;
    double       sun_longitude_deg;
} DhruvLunarPhaseEvent;
```

### DhruvLunarPhaseSearchRequest

```c
typedef struct {
    int32_t      phase_kind;     // DHRUV_LUNAR_PHASE_KIND_*
    int32_t      query_mode;     // DHRUV_LUNAR_PHASE_QUERY_MODE_*
    int32_t      time_kind;      // DHRUV_SEARCH_TIME_*
    double       at_jd_tdb;      // NEXT/PREV when time_kind=JD_TDB
    double       start_jd_tdb;   // RANGE when time_kind=JD_TDB
    double       end_jd_tdb;     // RANGE when time_kind=JD_TDB
    DhruvUtcTime at_utc;         // NEXT/PREV when time_kind=UTC
    DhruvUtcTime start_utc;      // RANGE when time_kind=UTC
    DhruvUtcTime end_utc;        // RANGE when time_kind=UTC
} DhruvLunarPhaseSearchRequest;
```

### DhruvSankrantiSearchRequest

```c
typedef struct {
    int32_t              target_kind;   // DHRUV_SANKRANTI_TARGET_*
    int32_t              query_mode;    // DHRUV_SANKRANTI_QUERY_MODE_*
    int32_t              rashi_index;   // 0..11 for TARGET_SPECIFIC
    int32_t              time_kind;     // DHRUV_SEARCH_TIME_*
    double               at_jd_tdb;     // NEXT/PREV when time_kind=JD_TDB
    double               start_jd_tdb;  // RANGE when time_kind=JD_TDB
    double               end_jd_tdb;    // RANGE when time_kind=JD_TDB
    DhruvUtcTime         at_utc;        // NEXT/PREV when time_kind=UTC
    DhruvUtcTime         start_utc;     // RANGE when time_kind=UTC
    DhruvUtcTime         end_utc;       // RANGE when time_kind=UTC
    DhruvSankrantiConfig config;
} DhruvSankrantiSearchRequest;
```

### DhruvPanchangComputeRequest

```c
typedef struct {
    int32_t              time_kind;      // DHRUV_PANCHANG_TIME_*
    double               jd_tdb;         // used for TIME_JD_TDB
    DhruvUtcTime         utc;            // used for TIME_UTC
    uint32_t             include_mask;   // DHRUV_PANCHANG_INCLUDE_* bitset
    DhruvGeoLocation     location;
    DhruvRiseSetConfig   riseset_config;
    DhruvSankrantiConfig sankranti_config;
} DhruvPanchangComputeRequest;
```

### DhruvPanchangOperationResult

```c
typedef struct {
    uint8_t                     tithi_valid;
    DhruvTithiInfo              tithi;
    uint8_t                     karana_valid;
    DhruvKaranaInfo             karana;
    uint8_t                     yoga_valid;
    DhruvYogaInfo               yoga;
    uint8_t                     vaar_valid;
    DhruvVaarInfo               vaar;
    uint8_t                     hora_valid;
    DhruvHoraInfo               hora;
    uint8_t                     ghatika_valid;
    DhruvGhatikaInfo            ghatika;
    uint8_t                     nakshatra_valid;
    DhruvPanchangNakshatraInfo  nakshatra;
    uint8_t                     masa_valid;
    DhruvMasaInfo               masa;
    uint8_t                     ayana_valid;
    DhruvAyanaInfo              ayana;
    uint8_t                     varsha_valid;
    DhruvVarshaInfo             varsha;
} DhruvPanchangOperationResult;
```

---

## Functions

### Versioning

```c
uint32_t dhruv_api_version(void);
```

Returns the ABI version number.

---

### Engine Lifecycle

```c
DhruvStatus dhruv_engine_new(
    const DhruvEngineConfig* config,
    DhruvEngineHandle**      out_engine
);
```

Create an engine handle from configuration. Loads SPK and LSK kernels from disk.

```c
DhruvStatus dhruv_engine_free(DhruvEngineHandle* engine);
```

Destroy an engine handle. Null-safe (no-op if null).

---

### Layered Config

```c
typedef struct DhruvConfigHandle DhruvConfigHandle;

DhruvStatus dhruv_config_load(
    const char* path_utf8,            // nullable: auto-discovery when null
    int32_t defaults_mode,            // 0=recommended, 1=none
    DhruvConfigHandle** out_handle
);

DhruvStatus dhruv_config_free(DhruvConfigHandle* handle);
DhruvStatus dhruv_config_clear_active(void);
```

Behavior:

- `dhruv_config_load` parses TOML/JSON config and activates resolver fallback for nullable config pointers.
- Non-null config pointers remain highest-priority explicit overrides.
- Null config pointers resolve as:
  1. operation config section
  2. common config section
  3. recommended defaults (when enabled)

This applies to conjunction, grahan, stationary, sankranti, riseset, bhava, graha-positions, bindus, drishti, and full-kundali family calls.

---

### Ephemeris Query

```c
DhruvStatus dhruv_engine_query(
    const DhruvEngineHandle* engine,
    const DhruvQuery*        query,
    DhruvStateVector*        out_state
);
```

Query the engine for a Cartesian state vector (position + velocity) at a given epoch.

```c
DhruvStatus dhruv_engine_query_request(
    const DhruvEngineHandle* engine,
    const DhruvQueryRequest* request,
    DhruvQueryResult*        out
);
```

Unified query entrypoint: carry JD(TDB)-vs-UTC input and cartesian-vs-spherical output selection through `DhruvQueryRequest`.

```c
DhruvStatus dhruv_query_once(
    const DhruvEngineConfig* config,
    const DhruvQuery*        query,
    DhruvStateVector*        out_state
);
```

One-shot convenience: creates engine, queries, and tears down internally.


---

### Time Conversion

```c
DhruvStatus dhruv_utc_to_tdb_jd(
    const DhruvLskHandle* lsk,
    int32_t  year,
    uint32_t month,
    uint32_t day,
    uint32_t hour,
    uint32_t min,
    double   sec,
    double*  out_jd_tdb
);
```

Convert UTC calendar date to JD TDB using a standalone LSK handle.

```c
DhruvStatus dhruv_jd_tdb_to_utc(
    const DhruvLskHandle* lsk,
    double                jd_tdb,
    DhruvUtcTime*         out_utc
);
```

Convert JD TDB to broken-down UTC calendar time.

```c
DhruvStatus dhruv_riseset_result_to_utc(
    const DhruvLskHandle*     lsk,
    const DhruvRiseSetResult* result,
    DhruvUtcTime*             out_utc
);
```

Convert a rise/set result to UTC. Returns `InvalidQuery` if `result_type` is not `DHRUV_RISESET_EVENT`.

---

### Coordinate Conversion

```c
DhruvStatus dhruv_cartesian_to_spherical(
    const double[3]       position_km,
    DhruvSphericalCoords* out_spherical
);
```

Convert Cartesian [x,y,z] (km) to spherical (lon_deg, lat_deg, distance_km). Pure math, no engine needed.

---

### LSK / EOP Handles

```c
DhruvStatus dhruv_lsk_load(
    const uint8_t*    lsk_path_utf8,  // NUL-terminated
    DhruvLskHandle**  out_lsk
);
```

Load a NAIF leap second kernel (.tls) file.

```c
DhruvStatus dhruv_lsk_free(DhruvLskHandle* lsk);
```

Destroy an LSK handle. Null-safe.

```c
DhruvStatus dhruv_eop_load(
    const uint8_t*    eop_path_utf8,  // NUL-terminated
    DhruvEopHandle**  out_eop
);
```

Load an IERS EOP file (finals2000A.all format).

```c
DhruvStatus dhruv_eop_free(DhruvEopHandle* eop);
```

Destroy an EOP handle. Null-safe.

---

### Ayanamsha

Use `dhruv_ayanamsha_compute_ex` as the single entrypoint for all ayanamsha
queries (mean/true/unified, JD-TDB/UTC, optional catalog).

```c
uint32_t dhruv_ayanamsha_system_count(void);
```

Returns number of supported systems (currently 20).

#### Unified request-based ayanamsha API

```c
#define DHRUV_AYANAMSHA_MODE_MEAN      0
#define DHRUV_AYANAMSHA_MODE_TRUE      1
#define DHRUV_AYANAMSHA_MODE_UNIFIED   2

#define DHRUV_AYANAMSHA_TIME_JD_TDB    0
#define DHRUV_AYANAMSHA_TIME_UTC       1

typedef struct {
    int32_t      system_code;       // 0..19
    int32_t      mode;              // DHRUV_AYANAMSHA_MODE_*
    int32_t      time_kind;         // DHRUV_AYANAMSHA_TIME_*
    double       jd_tdb;            // when time_kind=JD_TDB
    DhruvUtcTime utc;               // when time_kind=UTC
    uint8_t      use_nutation;      // for mode=UNIFIED
    double       delta_psi_arcsec;  // for mode=TRUE
} DhruvAyanamshaComputeRequest;

DhruvStatus dhruv_ayanamsha_compute_ex(
    const DhruvLskHandle*                 lsk,      // required for time_kind=UTC
    const DhruvAyanamshaComputeRequest*   request,
    const DhruvTaraCatalogHandle*         catalog,  // optional
    double*                               out_deg
);
```

Single request API for mode + time-base variants:
- `mode=MEAN`: mean ayanamsha (catalog-aware for star-anchored systems).
- `mode=TRUE`: true ayanamsha from `delta_psi_arcsec`.
- `mode=UNIFIED`: unified ayanamsha using `use_nutation`.
- `time_kind=JD_TDB`: uses `jd_tdb`.
- `time_kind=UTC`: uses `utc` and requires `lsk`.

---

### Reference Plane

```c
enum DhruvReferencePlane {
    DHRUV_REFERENCE_PLANE_ECLIPTIC   = 0,
    DHRUV_REFERENCE_PLANE_INVARIABLE = 1,
};
```

Reference plane for longitude measurements. Most ayanamsha systems use the
ecliptic (0). The Jagganatha system uses the invariable plane (1).

```c
int32_t dhruv_reference_plane_default(int32_t system_code);
```

Returns the default reference plane code for a given ayanamsha system.
Returns 0 (Ecliptic) for all systems except Jagganatha (code 16), which
returns 1 (Invariable). Returns -1 for invalid system codes.

```c
DhruvSankrantiConfig dhruv_sankranti_config_default(void);
```

Returns a default `DhruvSankrantiConfig` (Lahiri, no nutation,
`reference_plane = -1` for system default, standard search parameters).

---

### Nutation

```c
DhruvStatus dhruv_nutation_iau2000b(
    double  jd_tdb,
    double* out_dpsi_arcsec,  // Nutation in longitude
    double* out_deps_arcsec   // Nutation in obliquity
);
```

Standalone IAU 2000B nutation computation. Pure math. Returns nutation in longitude and obliquity in arcseconds.

---

### Lunar Nodes

```c
DhruvStatus dhruv_lunar_node_deg(
    int32_t node_code,  // 0=Rahu, 1=Ketu
    int32_t mode_code,  // 0=Mean, 1=True
    double  jd_tdb,
    double* out_deg     // Longitude in degrees [0, 360)
);
```

Compute lunar node longitude. Pure math, no engine needed.
For `mode_code=1` (True), the 50-term perturbation series was fitted over 1900–2100;
accuracy degrades outside that interval. Prefer the `_with_engine` variant for
production use.

```c
DhruvStatus dhruv_lunar_node_deg_with_engine(
    const DhruvEngineHandle* engine,
    int32_t node_code,  // 0=Rahu, 1=Ketu
    int32_t mode_code,  // 0=Mean, 1=True
    double  jd_tdb,
    double* out_deg     // Longitude in degrees [0, 360)
);
```

Compute lunar node longitude using an engine handle. For `mode_code=1`, this
uses an osculating node from Moon state vectors.

```c
DhruvStatus dhruv_lunar_node_deg_utc_with_engine(
    const DhruvEngineHandle* engine,
    int32_t node_code,  // 0=Rahu, 1=Ketu
    int32_t mode_code,  // 0=Mean, 1=True
    const DhruvUtcTime* utc,
    double* out_deg     // Longitude in degrees [0, 360)
);
```

UTC convenience variant of the engine-aware lunar node API.

```c
#define DHRUV_NODE_BACKEND_ANALYTIC 0
#define DHRUV_NODE_BACKEND_ENGINE   1
#define DHRUV_NODE_TIME_JD_TDB      0
#define DHRUV_NODE_TIME_UTC         1

typedef struct {
    int32_t      node_code;   // DHRUV_NODE_RAHU or DHRUV_NODE_KETU
    int32_t      mode_code;   // DHRUV_NODE_MODE_*
    int32_t      backend;     // DHRUV_NODE_BACKEND_*
    int32_t      time_kind;   // DHRUV_NODE_TIME_*
    double       jd_tdb;      // when time_kind=JD_TDB
    DhruvUtcTime utc;         // when time_kind=UTC
} DhruvLunarNodeRequest;

DhruvStatus dhruv_lunar_node_compute_ex(
    const DhruvEngineHandle*     engine,   // required for backend=ENGINE
    const DhruvLskHandle*        lsk,      // required for time_kind=UTC
    const DhruvLunarNodeRequest* request,
    double*                      out_deg
);
```

Unified node entrypoint:
- `backend=ANALYTIC` uses the pure-math model backend.
- `backend=ENGINE` uses the engine-backed osculating-node backend.
- `time_kind=JD_TDB` uses `jd_tdb`.
- `time_kind=UTC` uses `utc` and requires `lsk`.

```c
uint32_t dhruv_lunar_node_count(void);
```

Returns number of node variants (currently 2: Rahu, Ketu).

---

### Sunrise / Sunset

```c
DhruvRiseSetConfig dhruv_riseset_config_default(void);
```

Returns default config: `use_refraction=1`, `sun_limb=UPPER`, `altitude_correction=1`.

```c
DhruvStatus dhruv_compute_rise_set(
    const DhruvEngineHandle*  engine,
    const DhruvLskHandle*     lsk,
    const DhruvEopHandle*     eop,
    const DhruvGeoLocation*   location,
    int32_t                   event_code,     // 0-7
    double                    jd_utc_noon,    // Approximate local noon (JD UTC)
    const DhruvRiseSetConfig* config,
    DhruvRiseSetResult*       out_result
);
```

Compute a single rise/set event for a given day and location.

```c
DhruvStatus dhruv_compute_all_events(
    const DhruvEngineHandle*  engine,
    const DhruvLskHandle*     lsk,
    const DhruvEopHandle*     eop,
    const DhruvGeoLocation*   location,
    double                    jd_utc_noon,
    const DhruvRiseSetConfig* config,
    DhruvRiseSetResult*       out_results     // Array of 8
);
```

Compute all 8 events for a day. Output order: AstroDawn, NautDawn, CivilDawn, Sunrise, Sunset, CivilDusk, NautDusk, AstroDusk.

```c
double dhruv_approximate_local_noon_jd(
    double jd_ut_midnight,
    double longitude_deg
);
```

Utility: approximate local noon JD from 0h UT JD and longitude. Pure math.

---

### Bhava (House Systems)

```c
DhruvBhavaConfig dhruv_bhava_config_default(void);
```

Returns default: Equal house, Ascendant start, StartOfFirst reference.

```c
uint32_t dhruv_bhava_system_count(void);
```

Returns number of supported house systems (currently 10).

```c
DhruvStatus dhruv_compute_bhavas(
    const DhruvEngineHandle* engine,
    const DhruvLskHandle*    lsk,
    const DhruvEopHandle*    eop,
    const DhruvGeoLocation*  location,
    double                   jd_utc,
    const DhruvBhavaConfig*  config,
    DhruvBhavaResult*        out_result
);
```

Compute 12 configured bhava cusps with Ascendant and MC, plus optional rashi-bhava sibling cusps when requested by `DhruvBhavaConfig`.

**Note:** KP, Koch, Topocentric, and Alcabitus systems require `|latitude| <= 66.5 deg`.

```c
DhruvStatus dhruv_lagna_deg(
    const DhruvLskHandle*   lsk,
    const DhruvEopHandle*   eop,
    const DhruvGeoLocation* location,
    double                  jd_utc,
    double*                 out_deg
);
```

Compute Ascendant ecliptic longitude in degrees. Uses apparent (GAST) sidereal
time and true obliquity. No engine needed.

```c
DhruvStatus dhruv_mc_deg(
    const DhruvLskHandle*   lsk,
    const DhruvEopHandle*   eop,
    const DhruvGeoLocation* location,
    double                  jd_utc,
    double*                 out_deg
);
```

Compute MC (Midheaven) ecliptic longitude in degrees. Uses apparent (GAST)
sidereal time and true obliquity. No engine needed.

---

### RAMC

```c
DhruvStatus dhruv_ramc_deg(
    const DhruvLskHandle*   lsk,
    const DhruvEopHandle*   eop,
    const DhruvGeoLocation* location,
    double                  jd_utc,
    double*                 out_deg
);
```

Compute the RAMC (Right Ascension of the MC / apparent Local Sidereal Time) in
degrees. Uses apparent (GAST) sidereal time. No engine needed.

```c
DhruvStatus dhruv_ramc_deg_utc(
    const DhruvLskHandle*   lsk,
    const DhruvEopHandle*   eop,
    const DhruvGeoLocation* location,
    const DhruvUtcTime*     utc,
    double*                 out_deg
);
```

UTC variant of `dhruv_ramc_deg`. Uses apparent (GAST) sidereal time.

---

### Conjunction / Aspect Search

```c
DhruvConjunctionConfig dhruv_conjunction_config_default(void);
```

Returns default: `target_separation_deg=0`, `step_size_days=0.5`, `max_iterations=50`, `convergence_days=1e-8`.

```c
#define DHRUV_CONJUNCTION_QUERY_MODE_NEXT  0
#define DHRUV_CONJUNCTION_QUERY_MODE_PREV  1
#define DHRUV_CONJUNCTION_QUERY_MODE_RANGE 2

typedef struct {
    int32_t                body1_code;
    int32_t                body2_code;
    int32_t                query_mode;
    int32_t                time_kind;
    double                 at_jd_tdb;
    double                 start_jd_tdb;
    double                 end_jd_tdb;
    DhruvUtcTime           at_utc;
    DhruvUtcTime           start_utc;
    DhruvUtcTime           end_utc;
    DhruvConjunctionConfig config;
} DhruvConjunctionSearchRequest;

DhruvStatus dhruv_conjunction_search_ex(
    const DhruvEngineHandle*                 engine,
    const DhruvConjunctionSearchRequest*     request,
    DhruvConjunctionEvent*                   out_event,   // NEXT/PREV
    uint8_t*                                 out_found,   // NEXT/PREV
    DhruvConjunctionEvent*                   out_events,  // RANGE
    uint32_t                                 max_count,   // RANGE
    uint32_t*                                out_count    // RANGE
);
```

Unified conjunction entrypoint:
- `time_kind=JD_TDB` uses `at_jd_tdb` or `start_jd_tdb/end_jd_tdb`.
- `time_kind=UTC` uses `at_utc` or `start_utc/end_utc`.
- `query_mode=NEXT/PREV` writes to `out_event/out_found`.
- `query_mode=RANGE` writes to `out_events/out_count`.

---

### Chandra Grahan

```c
DhruvGrahanConfig dhruv_grahan_config_default(void);
```

Returns default: `include_penumbral=1`, `include_peak_details=1`.

```c
#define DHRUV_GRAHAN_KIND_CHANDRA      0
#define DHRUV_GRAHAN_KIND_SURYA        1
#define DHRUV_GRAHAN_QUERY_MODE_NEXT   0
#define DHRUV_GRAHAN_QUERY_MODE_PREV   1
#define DHRUV_GRAHAN_QUERY_MODE_RANGE  2

typedef struct {
    int32_t           grahan_kind;    // DHRUV_GRAHAN_KIND_*
    int32_t           query_mode;     // DHRUV_GRAHAN_QUERY_MODE_*
    int32_t           time_kind;      // DHRUV_SEARCH_TIME_*
    double            at_jd_tdb;
    double            start_jd_tdb;
    double            end_jd_tdb;
    DhruvUtcTime      at_utc;
    DhruvUtcTime      start_utc;
    DhruvUtcTime      end_utc;
    DhruvGrahanConfig config;
} DhruvGrahanSearchRequest;

DhruvStatus dhruv_grahan_search_ex(
    const DhruvEngineHandle*               engine,
    const DhruvGrahanSearchRequest*        request,
    DhruvChandraGrahanResult*              out_chandra_single, // CHANDRA + NEXT/PREV
    DhruvSuryaGrahanResult*                out_surya_single,   // SURYA + NEXT/PREV
    uint8_t*                               out_found,          // NEXT/PREV
    DhruvChandraGrahanResult*              out_chandra_many,   // CHANDRA + RANGE
    DhruvSuryaGrahanResult*                out_surya_many,     // SURYA + RANGE
    uint32_t                               max_count,          // RANGE
    uint32_t*                              out_count           // RANGE
);
```

Unified grahan entrypoint:
- `grahan_kind` selects chandra vs surya result family.
- `time_kind=JD_TDB` uses `at_jd_tdb` or `start_jd_tdb/end_jd_tdb`.
- `time_kind=UTC` uses `at_utc` or `start_utc/end_utc`.
- `query_mode=NEXT/PREV` uses `out_found` and a single-result pointer.
- `query_mode=RANGE` uses array output pointer + `out_count`.

**Note:** Legacy split grahan wrappers were removed in v42. Use `dhruv_grahan_search_ex`.

---

### Stationary Point Search

```c
DhruvStationaryConfig dhruv_stationary_config_default(void);
```

Returns default: `step_size_days=1.0`, `max_iterations=50`, `convergence_days=1e-8`, `numerical_step_days=0.01`.

```c
#define DHRUV_MOTION_KIND_STATIONARY       0
#define DHRUV_MOTION_KIND_MAX_SPEED        1
#define DHRUV_MOTION_QUERY_MODE_NEXT       0
#define DHRUV_MOTION_QUERY_MODE_PREV       1
#define DHRUV_MOTION_QUERY_MODE_RANGE      2

typedef struct {
    int32_t               body_code;     // NAIF code
    int32_t               motion_kind;   // DHRUV_MOTION_KIND_*
    int32_t               query_mode;    // DHRUV_MOTION_QUERY_MODE_*
    int32_t               time_kind;     // DHRUV_SEARCH_TIME_*
    double                at_jd_tdb;
    double                start_jd_tdb;
    double                end_jd_tdb;
    DhruvUtcTime          at_utc;
    DhruvUtcTime          start_utc;
    DhruvUtcTime          end_utc;
    DhruvStationaryConfig config;
} DhruvMotionSearchRequest;

DhruvStatus dhruv_motion_search_ex(
    const DhruvEngineHandle*              engine,
    const DhruvMotionSearchRequest*       request,
    DhruvStationaryEvent*                 out_stationary_single, // STATIONARY + NEXT/PREV
    DhruvMaxSpeedEvent*                   out_max_speed_single,  // MAX_SPEED + NEXT/PREV
    uint8_t*                              out_found,             // NEXT/PREV
    DhruvStationaryEvent*                 out_stationary_many,   // STATIONARY + RANGE
    DhruvMaxSpeedEvent*                   out_max_speed_many,    // MAX_SPEED + RANGE
    uint32_t                              max_count,             // RANGE
    uint32_t*                             out_count              // RANGE
);
```

Unified motion entrypoint:
- `motion_kind` selects stationary vs max-speed family.
- `time_kind=JD_TDB` uses `at_jd_tdb` or `start_jd_tdb/end_jd_tdb`.
- `time_kind=UTC` uses `at_utc` or `start_utc/end_utc`.
- `query_mode=NEXT/PREV` uses `out_found` and a single-result pointer.
- `query_mode=RANGE` uses array output pointer + `out_count`.

**Note:** Legacy split motion wrappers were removed in v42. Use `dhruv_motion_search_ex`.

---

### Lunar Phase Search

```c
#define DHRUV_LUNAR_PHASE_KIND_AMAVASYA      0
#define DHRUV_LUNAR_PHASE_KIND_PURNIMA       1
#define DHRUV_LUNAR_PHASE_QUERY_MODE_NEXT    0
#define DHRUV_LUNAR_PHASE_QUERY_MODE_PREV    1
#define DHRUV_LUNAR_PHASE_QUERY_MODE_RANGE   2

DhruvStatus dhruv_lunar_phase_search_ex(
    const DhruvEngineHandle*                engine,
    const DhruvLunarPhaseSearchRequest*     request,
    DhruvLunarPhaseEvent*                   out_event,   // NEXT/PREV
    uint8_t*                                out_found,   // NEXT/PREV
    DhruvLunarPhaseEvent*                   out_events,  // RANGE
    uint32_t                                max_count,   // RANGE
    uint32_t*                               out_count    // RANGE
);
```

Unified lunar-phase entrypoint:
- `phase_kind` selects amavasya vs purnima family.
- `time_kind=JD_TDB` uses `at_jd_tdb` or `start_jd_tdb/end_jd_tdb`.
- `time_kind=UTC` uses `at_utc` or `start_utc/end_utc`.
- `query_mode=NEXT/PREV` writes `out_event/out_found`.
- `query_mode=RANGE` writes `out_events/out_count`.

---

### Sankranti Search

```c
#define DHRUV_SANKRANTI_TARGET_ANY           0
#define DHRUV_SANKRANTI_TARGET_SPECIFIC      1
#define DHRUV_SANKRANTI_QUERY_MODE_NEXT      0
#define DHRUV_SANKRANTI_QUERY_MODE_PREV      1
#define DHRUV_SANKRANTI_QUERY_MODE_RANGE     2

DhruvStatus dhruv_sankranti_search_ex(
    const DhruvEngineHandle*               engine,
    const DhruvSankrantiSearchRequest*     request,
    DhruvSankrantiEvent*                   out_event,   // NEXT/PREV
    uint8_t*                               out_found,   // NEXT/PREV
    DhruvSankrantiEvent*                   out_events,  // RANGE
    uint32_t                               max_count,   // RANGE
    uint32_t*                              out_count    // RANGE
);
```

Unified sankranti entrypoint:
- `target_kind=ANY` covers all rashis.
- `target_kind=SPECIFIC` filters to `rashi_index`.
- `time_kind=JD_TDB` uses `at_jd_tdb` or `start_jd_tdb/end_jd_tdb`.
- `time_kind=UTC` uses `at_utc` or `start_utc/end_utc`.
- `query_mode=NEXT/PREV` writes `out_event/out_found`.
- `query_mode=RANGE` writes `out_events/out_count`.

---

### Unified Panchang Compute

```c
#define DHRUV_PANCHANG_TIME_JD_TDB      0
#define DHRUV_PANCHANG_TIME_UTC         1

#define DHRUV_PANCHANG_INCLUDE_TITHI       (1u << 0)
#define DHRUV_PANCHANG_INCLUDE_KARANA      (1u << 1)
#define DHRUV_PANCHANG_INCLUDE_YOGA        (1u << 2)
#define DHRUV_PANCHANG_INCLUDE_VAAR        (1u << 3)
#define DHRUV_PANCHANG_INCLUDE_HORA        (1u << 4)
#define DHRUV_PANCHANG_INCLUDE_GHATIKA     (1u << 5)
#define DHRUV_PANCHANG_INCLUDE_NAKSHATRA   (1u << 6)
#define DHRUV_PANCHANG_INCLUDE_MASA        (1u << 7)
#define DHRUV_PANCHANG_INCLUDE_AYANA       (1u << 8)
#define DHRUV_PANCHANG_INCLUDE_VARSHA      (1u << 9)
#define DHRUV_PANCHANG_INCLUDE_ALL_CORE     0x7fu
#define DHRUV_PANCHANG_INCLUDE_ALL_CALENDAR 0x380u
#define DHRUV_PANCHANG_INCLUDE_ALL          0x3ffu

DhruvStatus dhruv_panchang_compute_ex(
    const DhruvEngineHandle*         engine,
    const DhruvEopHandle*            eop,
    const DhruvLskHandle*            lsk,      // required for TIME_JD_TDB
    const DhruvPanchangComputeRequest* request,
    DhruvPanchangOperationResult*    out
);
```

Unified panchang entrypoint:
- `time_kind=TIME_UTC` reads `request.utc`.
- `time_kind=TIME_JD_TDB` reads `request.jd_tdb` and requires non-null `lsk`.
- `include_mask` selects returned fields; each output slot has a `<field>_valid` flag.
- `sankranti_config` is used for sidereal/calendar-dependent elements.
- `riseset_config` and `location` are used for `vaar`, `hora`, `ghatika`.

---

### Pure-Math Panchang Classifiers

These functions classify raw angular/temporal values into Vedic categories. No engine or kernel needed.

```c
typedef struct {
    int32_t tithi_index;        // 0-based (0..29)
    int32_t paksha;             // 0=Shukla, 1=Krishna
    int32_t tithi_in_paksha;    // 1-based (1..15)
    double  degrees_in_tithi;   // [0, 12)
} DhruvTithiPosition;

DhruvStatus dhruv_tithi_from_elongation(double elongation_deg, DhruvTithiPosition* out);
```

Determine Tithi from Moon-Sun elongation (degrees, 0..360).

```c
typedef struct {
    int32_t karana_index;       // 0-based (0..59)
    double  degrees_in_karana;  // [0, 6)
} DhruvKaranaPosition;

DhruvStatus dhruv_karana_from_elongation(double elongation_deg, DhruvKaranaPosition* out);
```

Determine Karana from Moon-Sun elongation (degrees).

```c
typedef struct {
    int32_t yoga_index;         // 0-based (0..26)
    double  degrees_in_yoga;    // [0, 13.333...)
} DhruvYogaPosition;

DhruvStatus dhruv_yoga_from_sum(double sum_deg, DhruvYogaPosition* out);
```

Determine Yoga from Sun+Moon sidereal longitude sum (degrees).

```c
int32_t dhruv_vaar_from_jd(double jd);
```

Determine Vaar (weekday) from Julian Date. Returns 0=Ravivaar(Sunday) .. 6=Shanivaar(Saturday).

```c
int32_t dhruv_masa_from_rashi_index(uint32_t rashi_index);
```

Determine Masa (lunar month) from 0-based rashi index. Returns 0=Chaitra .. 11=Phalguna, or -1 for invalid input.

```c
int32_t dhruv_ayana_from_sidereal_longitude(double lon_deg);
```

Determine Ayana from sidereal longitude. Returns 0=Uttarayana, 1=Dakshinayana.

```c
typedef struct {
    int32_t samvatsara_index;   // 0-based (0..59)
    int32_t cycle_position;     // 1-based (1..60)
} DhruvSamvatsaraResult;

DhruvStatus dhruv_samvatsara_from_year(int32_t ce_year, DhruvSamvatsaraResult* out);
```

Determine Samvatsara (Jovian year) from a CE year.

```c
int32_t dhruv_nth_rashi_from(uint32_t rashi_index, uint32_t offset);
```

Compute the rashi index that is `offset` signs from `rashi_index`. Returns 0-based index, or -1 for invalid input.

---

### Graha Longitudes

```c
typedef struct {
    double longitudes[9];   // Indexed by Graha order (Surya=0 .. Ketu=8)
} DhruvGrahaLongitudes;
```

```c
typedef struct {
    int32_t kind;               // DHRUV_GRAHA_LONGITUDE_KIND_*
    uint32_t ayanamsha_system;  // 0-19
    uint8_t use_nutation;       // 0=false, 1=true
    int32_t precession_model;   // DHRUV_PRECESSION_MODEL_*
    int32_t reference_plane;    // DHRUV_REFERENCE_PLANE_*
} DhruvGrahaLongitudesConfig;
```

```c
DhruvGrahaLongitudesConfig dhruv_graha_longitudes_config_default(void);

DhruvStatus dhruv_graha_longitudes(
    const Engine*                         engine,
    double                                jd_tdb,
    const DhruvGrahaLongitudesConfig*     config,
    DhruvGrahaLongitudes*                 out
);
```

Query graha longitudes (degrees, 0..360) of all 9 grahas at a given JD (TDB). `config->kind` selects sidereal vs tropical/reference-plane output. The same config carries ayanamsha choice, nutation, precession model, and reference-plane selection instead of splitting those variations across separate symbol names.

```c
typedef struct {
    uint8_t graha_index;
    double sidereal_longitude;
    double ayanamsha_deg;
    double reference_plane_longitude;
} DhruvMovingOsculatingApogeeEntry;

typedef struct {
    uint8_t count;
    DhruvMovingOsculatingApogeeEntry entries[DHRUV_MAX_OSCULATING_APOGEE_REQUESTS];
} DhruvMovingOsculatingApogees;

DhruvStatus dhruv_moving_osculating_apogees_for_date(
    const DhruvEngineHandle*              engine,
    const DhruvEopHandle*                 eop,
    const DhruvUtcTime*                   utc,
    const uint8_t*                        graha_indices,
    uint8_t                               graha_count,
    const DhruvGrahaLongitudesConfig*     config,
    DhruvMovingOsculatingApogees*         out
);
```

Batch heliocentric moving osculating apogees for Mangal=2, Buddh=3, Guru=4,
Shukra=5, and Shani=6. Entries preserve caller order and duplicate
multiplicity. Surya, Chandra, Rahu, and Ketu return invalid input.

---

### Nakshatra At

```c
DhruvStatus dhruv_nakshatra_at(
    const DhruvEngineHandle*    engine,
    double                      jd_tdb,
    double                      moon_sidereal_deg,  // [0, 360)
    const DhruvSankrantiConfig* config,
    DhruvPanchangNakshatraInfo* out
);
```

Determine the Moon's Nakshatra (27-scheme) from a pre-computed sidereal longitude. The engine is still needed for boundary bisection (finding start/end times). Returns nakshatra index, pada, and start/end times (UTC). Useful when the Moon's sidereal longitude has already been computed (e.g., from `dhruv_graha_longitudes` with sidereal config).

---

### Time Upagraha JD

```c
DhruvStatus dhruv_time_upagraha_jd(
    uint32_t    upagraha_index,    // 0=Gulika, 1=Maandi, 2=Kaala, 3=Mrityu, 4=ArthaPrahara, 5=YamaGhantaka
    uint32_t    weekday,           // 0=Sunday .. 6=Saturday
    uint8_t     is_day,            // 1=daytime, 0=nighttime
    double      sunrise_jd,
    double      sunset_jd,
    double      next_sunrise_jd,
    double*     out_jd
);
```

Compute the JD at which to evaluate a time-based upagraha's lagna. Pure math — accepts pre-computed sunrise/sunset/next-sunrise JDs. Returns `DHRUV_STATUS_INVALID_QUERY` for `upagraha_index >= 6` or `weekday > 6`.

```c
DhruvStatus dhruv_time_upagraha_jd_utc(
    const DhruvEngineHandle*   engine,
    const DhruvEopHandle*      eop,
    const DhruvUtcTime*        utc,
    const DhruvGeoLocation*    location,
    const DhruvRiseSetConfig*  riseset_config,
    uint32_t                   upagraha_index,
    double*                    out_jd
);
```

Compute the JD for a time-based upagraha from a UTC date and location. Computes sunrise, sunset, and next sunrise internally from the engine, EOP, and location. Automatically determines weekday and day/night status from the computed rise/set times.

---

### Pure-Math Ashtakavarga

These expose the individual building blocks of ashtakavarga computation. All are pure math — no engine or kernel needed. Callers supply pre-computed rashi positions.

```c
DhruvStatus dhruv_calculate_bav(
    uint8_t        graha_index,      // 0=Sun through 6=Saturn
    const uint8_t* graha_rashis,     // 7 entries: 0-based rashi for Sun..Saturn
    uint8_t        lagna_rashi,      // 0-based rashi of Ascendant
    DhruvBhinnaAshtakavarga* out
);
```

Calculate BAV (Bhinna Ashtakavarga) for a single graha. The `DhruvBhinnaAshtakavarga`
output includes both aggregate `points[12]` and contributor attribution matrix
`contributors[12][8]` (Sun, Moon, Mars, Mercury, Jupiter, Venus, Saturn, Lagna).
Returns `DHRUV_STATUS_INVALID_QUERY` for `graha_index > 6`.

```c
DhruvStatus dhruv_calculate_all_bav(
    const uint8_t*           graha_rashis,   // 7 entries
    uint8_t                  lagna_rashi,
    DhruvBhinnaAshtakavarga* out             // caller allocates array of 7
);
```

Calculate BAV for all 7 grahas at once.

```c
DhruvStatus dhruv_calculate_sav(
    const DhruvBhinnaAshtakavarga* bavs,   // 7 entries (from dhruv_calculate_all_bav)
    DhruvSarvaAshtakavarga*        out
);
```

Calculate SAV (Sarva Ashtakavarga) from 7 BAVs. Returns total points, after trikona sodhana, and after ekadhipatya sodhana.

```c
DhruvStatus dhruv_trikona_sodhana(
    const uint8_t* totals,    // 12 rashi totals
    uint8_t*       out        // 12 values after trikona reduction
);
```

Apply Trikona Sodhana: subtract the minimum value from each trine group (fire, earth, air, water).

```c
DhruvStatus dhruv_ekadhipatya_sodhana(
    const uint8_t* after_trikona,   // 12 values (from dhruv_trikona_sodhana)
    uint8_t*       out              // 12 values after ekadhipatya reduction
);
```

Apply Ekadhipatya Sodhana: subtract the minimum from same-lord pairs (Mercury: Mithuna/Kanya, Jupiter: Dhanu/Meena). Typically called on the output of `dhruv_trikona_sodhana`.

---

### Pure-Math Drishti

```c
DhruvStatus dhruv_graha_drishti(
    uint32_t          graha_index,   // 0=Surya .. 8=Ketu
    double            source_lon,    // sidereal longitude (degrees)
    double            target_lon,    // sidereal longitude (degrees)
    DhruvDrishtiEntry* out
);
```

Compute drishti (planetary aspect) from a single graha to a single sidereal point. Returns angular distance, base virupa, special virupa, and total virupa. Returns `DHRUV_STATUS_INVALID_QUERY` for `graha_index > 8`.

```c
DhruvStatus dhruv_graha_drishti_matrix(
    const double*             longitudes,   // 9 sidereal longitudes (Sun..Ketu)
    DhruvGrahaDrishtiMatrix*  out
);
```

Compute the full 9×9 graha drishti matrix from pre-computed sidereal longitudes. Self-aspect (diagonal) entries are zeroed.

---

### Pure-Math Ghatika / Hora

```c
DhruvStatus dhruv_ghatika_from_elapsed(
    double   seconds_since_sunrise,
    double   vedic_day_duration_seconds,
    uint8_t* out_value,    // ghatika 1-60
    uint8_t* out_index     // 0-based index 0-59
);
```

Determine the ghatika from elapsed seconds since sunrise. One Vedic day = 60 ghatikas.

```c
DhruvStatus dhruv_ghatikas_since_sunrise(
    double   jd_moment,
    double   jd_sunrise,
    double   jd_next_sunrise,
    double*  out_ghatikas
);
```

Compute fractional ghatikas elapsed since sunrise. Result can exceed 60 if `jd_moment` is past the next sunrise.

```c
int32_t dhruv_hora_at(
    uint32_t vaar_index,    // 0=Sunday .. 6=Saturday
    uint32_t hora_index     // 0=first hora at sunrise .. 23=last
);
```

Determine the hora lord for a given weekday and hora position. Returns the lord's Chaldean index (0=Surya, 1=Shukra, 2=Buddh, 3=Chandra, 4=Shani, 5=Guru, 6=Mangal), or -1 on invalid input.

---

### Amsha (Divisional Charts)

Canonical amsha types and validation rules are described in
`docs/AMSHA_PARITY_CONTRACT.md`.

Direct amsha entry points:

```c
DhruvStatus dhruv_amsha_longitude(
    double    sidereal_lon,     // sidereal longitude in degrees
    uint16_t  amsha_code,       // D-number (1, 2, 3, ... 144)
    uint8_t   variation_code,   // interpreted in the namespace of amsha_code
    double*   out
);
```

Transform a single sidereal longitude through one amsha division.

```c
DhruvStatus dhruv_amsha_rashi_info(
    double           sidereal_lon,
    uint16_t         amsha_code,
    uint8_t          variation_code,
    DhruvRashiInfo*  out
);
```

Transform a single sidereal longitude through one amsha division and return
full rashi information for the transformed longitude.

```c
DhruvStatus dhruv_amsha_longitudes(
    double           sidereal_lon,
    const uint16_t*  amsha_codes,       // count items
    const uint8_t*   variation_codes,   // count items or NULL for all default
    uint32_t         count,
    double*          out                // count items
);
```

Transform one sidereal longitude through multiple amshas in one call.

```c
DhruvStatus dhruv_amsha_chart_for_date(
    const DhruvEngineHandle*      engine,
    const DhruvEopHandle*         eop,
    const DhruvUtcTime*           utc,
    const DhruvGeoLocation*       location,
    const DhruvBhavaConfig*       bhava_config,     // nullable: defaults resolved
    const DhruvRiseSetConfig*     riseset_config,   // nullable: defaults resolved
    uint32_t                      ayanamsha_system,
    uint8_t                       use_nutation,
    uint16_t                      amsha_code,
    uint8_t                       variation_code,
    const DhruvAmshaChartScope*   scope,
    DhruvAmshaChart*              out
);
```

Compute one amsha chart for a date and location. `scope` controls optional
sections inside the returned `DhruvAmshaChart`.

Relevant config/result shapes:

```c
struct DhruvAmshaChartScope {
    uint8_t include_bhava_cusps;
    uint8_t include_arudha_padas;
    uint8_t include_upagrahas;
    uint8_t include_sphutas;
    uint8_t include_special_lagnas;
};

struct DhruvAmshaSelectionConfig {
    uint8_t  count;                          // 0..=40
    uint16_t codes[DHRUV_MAX_AMSHA_REQUESTS];
    uint8_t  variations[DHRUV_MAX_AMSHA_REQUESTS]; // each value resolved against codes[i]
};

struct DhruvAmshaVariationInfo {
    uint16_t amsha_code;
    uint8_t  variation_code;
    char     name[DHRUV_AMSHA_VARIATION_NAME_CAPACITY];
    char     label[DHRUV_AMSHA_VARIATION_LABEL_CAPACITY];
    uint8_t  is_default;
    char     description[DHRUV_AMSHA_VARIATION_DESCRIPTION_CAPACITY];
};

struct DhruvAmshaVariationList {
    uint16_t                 amsha_code;
    uint8_t                  default_variation_code;
    uint8_t                  count;
    DhruvAmshaVariationInfo  variations[DHRUV_MAX_AMSHA_VARIATIONS];
};

struct DhruvAmshaVariationCatalogs {
    uint32_t                count;
    DhruvAmshaVariationList lists[DHRUV_MAX_AMSHA_REQUESTS];
};

DhruvStatus dhruv_amsha_variations(
    uint16_t amsha_code,
    DhruvAmshaVariationList *out
);

DhruvStatus dhruv_amsha_variations_many(
    const uint16_t *amsha_codes,
    uint32_t count,
    DhruvAmshaVariationCatalogs *out
);
```

Full-kundali embeds amsha configuration here:

- `DhruvFullKundaliConfig.include_amshas`
- `DhruvFullKundaliConfig.amsha_scope`
- `DhruvFullKundaliConfig.amsha_selection`

and returns amsha charts here:

- `DhruvFullKundaliResult.amshas_valid`
- `DhruvFullKundaliResult.amshas_count`
- `DhruvFullKundaliResult.amshas`

Validation notes:

- unknown `amsha_code` returns `DHRUV_STATUS_INVALID_SEARCH_CONFIG`
- unknown `variation_code` for that amsha returns `DHRUV_STATUS_INVALID_SEARCH_CONFIG`
- `variation_codes == NULL` in `dhruv_amsha_longitudes` means all requests use
  the default variation for each requested amsha

Dependency notes for full-kundali amsha scope:

- `include_bhava_cusps` amsha output depends on root `include_bhava_cusps`
- `include_arudha_padas` amsha output depends on root `include_bindus`
- `include_upagrahas` amsha output depends on root `include_upagrahas`
- `include_sphutas` amsha output depends on root `include_sphutas`
- `include_special_lagnas` amsha output depends on root `include_special_lagnas`

---

## Function Summary

The summary table below predates some newer amsha helper exports. For current
amsha entry points, use the dedicated section above.

Note: as of ABI v42, legacy split `dhruv_next_*` / `dhruv_prev_*` /
`dhruv_search_*` wrappers are no longer exported C ABI symbols.
Use the unified `*_search_ex` / `*_compute_ex` entries documented above.

| # | Function | Engine | LSK | EOP | Pure Math |
|---|----------|--------|-----|-----|-----------|
| 1 | `dhruv_api_version` | | | | yes |
| 2 | `dhruv_engine_new` | creates | | | |
| 3 | `dhruv_engine_free` | destroys | | | |
| 4 | `dhruv_engine_query` | yes | | | |
| 5 | `dhruv_engine_query_request` | yes | | | |
| 6 | `dhruv_query_once` | internal | | | |
| 7 | `dhruv_lsk_load` | | creates | | |
| 8 | `dhruv_lsk_free` | | destroys | | |
| 9 | `dhruv_eop_load` | | | creates | |
| 10 | `dhruv_eop_free` | | | destroys | |
| 11 | `dhruv_utc_to_tdb_jd` | | yes | | |
| 12 | `dhruv_jd_tdb_to_utc` | | yes | | |
| 13 | `dhruv_riseset_result_to_utc` | | yes | | |
| 14 | `dhruv_cartesian_to_spherical` | | | | yes |
| 15 | `dhruv_ayanamsha_compute_ex` | | conditional | | yes* |
| 16 | `dhruv_ayanamsha_system_count` | | | | yes |
| 17 | `dhruv_reference_plane_default` | | | | yes |
| 18 | `dhruv_nutation_iau2000b` | | | | yes |
| 19 | `dhruv_lunar_node_deg` | | | | yes |
| 20 | `dhruv_lunar_node_count` | | | | yes |
| 21 | `dhruv_riseset_config_default` | | | | yes |
| 23 | `dhruv_compute_rise_set` | yes | yes | yes | |
| 24 | `dhruv_compute_all_events` | yes | yes | yes | |
| 25 | `dhruv_approximate_local_noon_jd` | | | | yes |
| 26 | `dhruv_bhava_config_default` | | | | yes |
| 27 | `dhruv_bhava_system_count` | | | | yes |
| 28 | `dhruv_compute_bhavas` | yes | yes | yes | |
| 29 | `dhruv_lagna_deg` | | yes | yes | |
| 30 | `dhruv_mc_deg` | | yes | yes | |
| 31 | `dhruv_conjunction_config_default` | | | | yes |
| 35 | `dhruv_grahan_config_default` | | | | yes |
| 42 | `dhruv_stationary_config_default` | | | | yes |
| 49 | `dhruv_graha_longitudes_config_default` | yes | | | |
| 50 | `dhruv_graha_longitudes` | yes | | | |
| 51 | `dhruv_nakshatra_at` | yes | | | |
| 52 | `dhruv_ramc_deg` | | yes | yes | |
| 53 | `dhruv_ramc_deg_utc` | | yes | yes | |
| 54 | `dhruv_tithi_from_elongation` | | | | yes |
| 55 | `dhruv_karana_from_elongation` | | | | yes |
| 56 | `dhruv_yoga_from_sum` | | | | yes |
| 57 | `dhruv_vaar_from_jd` | | | | yes |
| 58 | `dhruv_masa_from_rashi_index` | | | | yes |
| 59 | `dhruv_ayana_from_sidereal_longitude` | | | | yes |
| 60 | `dhruv_samvatsara_from_year` | | | | yes |
| 61 | `dhruv_nth_rashi_from` | | | | yes |
| 62 | `dhruv_time_upagraha_jd` | | | | yes |
| 63 | `dhruv_time_upagraha_jd_utc` | yes | | yes | |
| 64 | `dhruv_calculate_bav` | | | | yes |
| 65 | `dhruv_calculate_all_bav` | | | | yes |
| 66 | `dhruv_calculate_sav` | | | | yes |
| 67 | `dhruv_trikona_sodhana` | | | | yes |
| 68 | `dhruv_ekadhipatya_sodhana` | | | | yes |
| 69 | `dhruv_graha_drishti` | | | | yes |
| 70 | `dhruv_graha_drishti_matrix` | | | | yes |
| 71 | `dhruv_ghatika_from_elapsed` | | | | yes |
| 72 | `dhruv_ghatikas_since_sunrise` | | | | yes |
| 73 | `dhruv_hora_at` | | | | yes |
| 74 | `dhruv_dasha_selection_config_default` | | | | yes |
| 75 | `dhruv_dasha_hierarchy` | yes | | | |
| 76 | `dhruv_dasha_snapshot` | yes | | | |
| 77 | `dhruv_dasha_hierarchy_level_count` | | | | yes |
| 78 | `dhruv_dasha_hierarchy_period_count` | | | | yes |
| 79 | `dhruv_dasha_hierarchy_period_at` | | | | yes |
| 80 | `dhruv_dasha_hierarchy_free` | | | | yes |
| 81 | `dhruv_full_kundali_result_free` | | | | yes |
| 82 | `dhruv_full_kundali_config_default` | | | | yes |
| 83 | `dhruv_tara_catalog_load` | | | | yes |
| 84 | `dhruv_tara_catalog_free` | | | | yes |
| 85 | `dhruv_tara_compute_ex` | | | | yes |
| 86 | `dhruv_tara_galactic_center_ecliptic` | | | | yes |
| 87 | `dhruv_panchang_compute_ex` | yes | conditional | yes | |

This summary table is not an authoritative symbol count. For current amsha
exports and config/result shapes, use the dedicated sections above.

---

## Dasha Types

### `DhruvDashaSelectionConfig`

```c
struct DhruvDashaSnapshotTime {
    int32_t      time_kind; // DHRUV_DASHA_TIME_NONE / JD_UTC / UTC
    double       jd_utc;    // only when time_kind == DHRUV_DASHA_TIME_JD_UTC
    DhruvUtcTime utc;       // only when time_kind == DHRUV_DASHA_TIME_UTC
};
```

```c
struct DhruvDashaSelectionConfig {
    uint8_t count;           // number of valid entries in systems (0..8)
    uint8_t systems[8];      // DashaSystem codes (0xFF = unused)
    uint8_t max_levels[8];   // per-system hierarchy depth (0-4, 0xFF = use max_level)
    uint8_t max_level;       // hierarchy depth (0-4, default 2)
    uint8_t level_methods[5]; // per-level sub-period method (0xFF = default)
    uint8_t yogini_scheme;   // 0 = default
    uint8_t use_abhijit;     // 1 = yes, 0 = no
    DhruvDashaSnapshotTime snapshot_time;
};
```

### `DhruvDashaInputs`

```c
struct DhruvDashaInputs {
    uint8_t               has_moon_sid_lon;
    double                moon_sid_lon;
    uint8_t               has_rashi_inputs;
    DhruvRashiDashaInputs rashi_inputs;
    uint8_t               has_sunrise_sunset;
    double                sunrise_jd;
    double                sunset_jd;
};
```

### `DhruvDashaBirthContext`

```c
struct DhruvDashaBirthContext {
    int32_t              time_kind;      // DHRUV_DASHA_TIME_JD_UTC or DHRUV_DASHA_TIME_UTC
    double               birth_jd;       // read when time_kind == JD_UTC
    DhruvUtcTime         birth_utc;      // read when time_kind == UTC
    uint8_t              has_location;
    DhruvGeoLocation     location;
    DhruvBhavaConfig     bhava_config;
    DhruvRiseSetConfig   riseset_config;
    DhruvSankrantiConfig sankranti_config;
    uint8_t              has_inputs;
    DhruvDashaInputs     inputs;
};
```

### Dasha Request Structs

The standalone dasha entrypoints are request-based and use:

- `DhruvDashaHierarchyRequest`
- `DhruvDashaSnapshotRequest`
- `DhruvDashaLevel0Request`
- `DhruvDashaLevel0EntityRequest`
- `DhruvDashaChildrenRequest`
- `DhruvDashaChildPeriodRequest`
- `DhruvDashaCompleteLevelRequest`

These all carry one `DhruvDashaBirthContext` plus the feature-specific fields
for `system`, `max_level`, `variation`, `parent`, `child_entity_*`, or
`child_level`.

### `DhruvDashaPeriod`

```c
struct DhruvDashaPeriod {
    uint8_t  entity_type;  // 0=Graha, 1=Rashi, 2=Yogini
    uint8_t  entity_index; // Graha (0-8), Rashi (0-11), Yogini (0-7)
    const char *entity_name; // canonical static UTF-8 name
    DhruvUtcTime start_utc; // Gregorian UTC, inclusive
    double   start_jd;     // JD UTC, inclusive
    DhruvUtcTime end_utc;   // Gregorian UTC, exclusive
    double   end_jd;       // JD UTC, exclusive
    uint8_t  level;        // 0-4
    uint16_t order;        // 1-indexed position among siblings
    uint32_t parent_idx;   // index into parent level (0 for level 0)
};
```

### `DhruvDashaSnapshot`

```c
struct DhruvDashaSnapshot {
    uint8_t           system;     // DashaSystem code
    DhruvUtcTime      query_utc;  // echoed query Gregorian UTC
    double            query_jd;   // echoed query JD UTC
    uint8_t           count;      // number of valid periods (0-5)
    DhruvDashaPeriod  periods[5]; // one per level
};
```

## JD Timescale Notes (Dasha)

All JD values in dasha APIs use **JD UTC** (not TDB):
- `DhruvDashaSelectionConfig.snapshot_time.jd_utc` — query time when `time_kind == DHRUV_DASHA_TIME_JD_UTC`
- `DhruvDashaPeriod.entity_name` — canonical static entity name
- `DhruvDashaPeriod.start_utc`, `.end_utc` — default structured Gregorian UTC boundaries
- `DhruvDashaPeriod.start_jd`, `.end_jd` — numeric JD UTC boundaries kept alongside UTC
- `DhruvDashaSnapshot.query_utc` — default structured Gregorian UTC query time
- `DhruvDashaSnapshot.query_jd` — echoed numeric JD UTC query time

## Ownership & Lifetime Table (Dasha)

| Handle/Resource | Allocated by | Freed by | Notes |
|-----------------|-------------|----------|-------|
| `DhruvDashaHierarchyHandle` (standalone) | `dhruv_dasha_hierarchy` | `dhruv_dasha_hierarchy_free` | Caller owns. Must free exactly once. |
| `DhruvDashaHierarchyHandle` (in kundali) | `dhruv_full_kundali_for_date` | `dhruv_full_kundali_result_free` | Result owns. Do NOT call `dhruv_dasha_hierarchy_free` on these. |
| `DhruvFullKundaliResult` | Caller stack/heap | `dhruv_full_kundali_result_free` | **Move-only:** do NOT memcpy the struct and free both copies — copied handles become dangling after the first free. Exactly one `result_free` call per `dhruv_full_kundali_for_date` invocation. |

---

### Fixed Stars (Tara)

Fixed star position computation. Stars are loaded from a JSON catalog file.
All time inputs are JD TDB. Positions are computed via space-motion-vector
propagation (proper motion, parallax, radial velocity) with optional apparent-place
corrections (aberration, gravitational light deflection, nutation).

#### Tara Types

##### `DhruvEquatorialPosition`

```c
struct DhruvEquatorialPosition {
    double ra_deg;       // Right ascension (degrees, 0-360)
    double dec_deg;      // Declination (degrees, -90 to +90)
    double distance_au;  // Distance (AU)
};
```

##### `DhruvEarthState`

```c
struct DhruvEarthState {
    double position_au[3];      // ICRS barycentric position (AU)
    double velocity_au_day[3];  // ICRS barycentric velocity (AU/day)
};
```

Required for Apparent tier and parallax. Caller obtains these from the
ephemeris engine (e.g., Earth relative to SSB).

##### `DhruvTaraConfig`

```c
struct DhruvTaraConfig {
    int32_t accuracy;       // 0 = Astrometric (default), 1 = Apparent
    uint8_t apply_parallax; // 1 = apply parallax correction, 0 = don't
};
```

- **Astrometric** (0): geometric/catalog place. Space motion propagation + frame rotation + precession. Optionally adds parallax if `apply_parallax == 1`.
- **Apparent** (1): adds annual aberration, gravitational light deflection, and nutation. Requires non-null `earth_state`.

##### `DhruvTaraCatalogHandle`

Opaque handle to a loaded star catalog. Created by `dhruv_tara_catalog_load`,
freed by `dhruv_tara_catalog_free`.

##### `DhruvTaraComputeRequest`

```c
struct DhruvTaraComputeRequest {
    int32_t          tara_id;           // TaraId code
    int32_t          output_kind;       // DHRUV_TARA_OUTPUT_*
    double           jd_tdb;            // epoch
    double           ayanamsha_deg;     // used for SIDEREAL output
    DhruvTaraConfig  config;
    uint8_t          earth_state_valid; // 0/1
    DhruvEarthState  earth_state;       // read only when earth_state_valid == 1
};
```

##### `DhruvTaraComputeResult`

```c
struct DhruvTaraComputeResult {
    int32_t                 output_kind;            // echoed DHRUV_TARA_OUTPUT_*
    DhruvEquatorialPosition equatorial;             // valid for EQUATORIAL
    DhruvSphericalCoords    ecliptic;               // valid for ECLIPTIC
    double                  sidereal_longitude_deg; // valid for SIDEREAL
};
```

##### `TaraId` codes

Stars are identified by `int32_t` codes. Ranges:
- 0-27: Nakshatra yogataras (e.g., 0=Ashwini, 13=Chitra/Spica)
- 100+: Rashi constellation stars
- 200+: Special Vedic stars (e.g., 200=Polaris, 201=Arcturus)
- 300+: Galactic reference points (300=GalacticCenter, 301=GalacticAntiCenter)

#### Tara Functions

```c
DhruvStatus dhruv_tara_catalog_load(
    const uint8_t* path_utf8,   // catalog JSON file path (UTF-8, not null-terminated)
    uint32_t       path_len,    // byte length of path
    DhruvTaraCatalogHandle** out_handle
);
```

Load a star catalog from a JSON file. On success, `*out_handle` is set to an
opaque handle. Caller must free with `dhruv_tara_catalog_free`.

```c
void dhruv_tara_catalog_free(
    DhruvTaraCatalogHandle* handle  // may be null (no-op)
);
```

Free a catalog handle. Safe to call with null.

```c
#define DHRUV_TARA_OUTPUT_EQUATORIAL 0
#define DHRUV_TARA_OUTPUT_ECLIPTIC   1
#define DHRUV_TARA_OUTPUT_SIDEREAL   2

DhruvStatus dhruv_tara_compute_ex(
    const DhruvTaraCatalogHandle*   handle,
    const DhruvTaraComputeRequest*  request,
    DhruvTaraComputeResult*         out
);
```

Unified tara entrypoint:
- `output_kind=EQUATORIAL` populates `out->equatorial`.
- `output_kind=ECLIPTIC` populates `out->ecliptic`.
- `output_kind=SIDEREAL` populates `out->sidereal_longitude_deg`.
- `earth_state_valid=1` passes `earth_state` for apparent/parallax modes.

Legacy per-output tara entrypoints were removed in ABI v41. Use
`dhruv_tara_compute_ex` for equatorial/ecliptic/sidereal outputs.

```c
DhruvStatus dhruv_tara_galactic_center_ecliptic(
    const DhruvTaraCatalogHandle* handle,
    double                        jd_tdb,
    DhruvSphericalCoords*         out
);
```

Compute ecliptic position of the Galactic Center (IAU 2000 ICRS direction,
no proper motion). Equivalent to requesting ecliptic output for
`TaraId::GalacticCenter` (code 300).

#### Ownership & Lifetime (Tara)

| Handle | Allocated by | Freed by | Notes |
|--------|-------------|----------|-------|
| `DhruvTaraCatalogHandle` | `dhruv_tara_catalog_load` | `dhruv_tara_catalog_free` | Caller owns. Must free exactly once. |

#### Error Mapping (Tara)

| Condition | DhruvStatus |
|-----------|-------------|
| Null pointer argument | `NullPointer` (7) |
| Invalid `tara_id` code | `InvalidQuery` (2) |
| Star not found in catalog | `InvalidQuery` (2) |
| Apparent/parallax without `earth_state` | `InvalidConfig` (5) |
| Catalog file load failure | `KernelLoad` (1) |

---

## Changelog

**v63**: `DhruvGrahaAvasthas` now exposes every applicable Lajjitadi state via
`lajjitadi_valid`, `lajjitadi_mask`, `lajjitadi_count`, and
`lajjitadi_states[6]`. The existing `lajjitadi` field remains the primary
compatibility state when valid, or `255` when no Lajjitadi condition applies.

**v62**: `DhruvGrahaAvasthas` now exposes every applicable Deeptadi state via
`deeptadi_mask`, `deeptadi_count`, and `deeptadi_states[9]`. The existing
`deeptadi` field remains the primary compatibility state.

**v55**: Standalone `dhruv_shadbala_for_date`, `dhruv_vimsopaka_for_date`,
`dhruv_balas_for_date`, and `dhruv_avastha_for_date` now accept
`const DhruvAmshaSelectionConfig *amsha_selection`. Embedded
`DhruvFullKundaliResult.amshas` now returns the resolved union of explicit
`amsha_selection` and any internally required bala/avastha amshas, using
caller-selected variation codes when present and default variations otherwise.

**v54**: Extended the unified high-level search request structs with `time_kind`
plus structured UTC fields (`at_utc`, `start_utc`, `end_utc`) so conjunction,
grahan, motion, lunar-phase, and sankranti searches can use Gregorian UTC or
numeric JD/TDB through the same main request shape.

**v52**: Extended `DhruvDashaPeriod` with structured Gregorian UTC `start_utc` and `end_utc` alongside numeric `start_jd` and `end_jd`. Extended `DhruvDashaSnapshot` with structured Gregorian UTC `query_utc` alongside numeric `query_jd`.

**v53**: Extended high-level time-bearing search/event results so the main C ABI structs now carry structured Gregorian UTC alongside numeric JD/TDB. This includes `DhruvConjunctionEvent`, `DhruvStationaryEvent`, `DhruvMaxSpeedEvent`, `DhruvChandraGrahanResult`, and `DhruvSuryaGrahanResult`.

**v47**: Added dedicated amsha C ABI entry points
`dhruv_amsha_longitude`, `dhruv_amsha_rashi_info`,
`dhruv_amsha_longitudes`, and `dhruv_amsha_chart_for_date`. Added
`DHRUV_MAX_AMSHA_REQUESTS`, `DhruvAmshaChartScope`,
`DhruvAmshaSelectionConfig`, and `DhruvAmshaChart`. Extended
`DhruvFullKundaliConfig` with `amsha_scope` and `amsha_selection`, and
extended `DhruvFullKundaliResult` with embedded amsha charts.

**v45**: Extended `DhruvBhinnaAshtakavarga` with contributor attribution matrix
`contributors[12][8]` (0/1 flags per rashi and contributor). Affects all APIs
that produce/consume BAV values: `dhruv_calculate_bav`,
`dhruv_calculate_all_bav`, `dhruv_calculate_ashtakavarga`,
`dhruv_ashtakavarga_for_date`, and full-kundali ashtakavarga fields.

**v48**: Removed `dhruv_graha_english_name` from the exported C ABI. Added `dhruv_yogini_name`. Extended `DhruvDashaPeriod` with `entity_name`, a canonical static UTF-8 pointer so dasha outputs carry exact entity names directly.

**v43**: Added charakaraka API surface. New constants: `DHRUV_MAX_CHARAKARAKA_ENTRIES`, `DHRUV_CHARAKARAKA_SCHEME_*`, `DHRUV_CHARAKARAKA_ROLE_*`. New types: `DhruvCharakarakaEntry`, `DhruvCharakarakaResult`. New function: `dhruv_charakaraka_for_date`. Extended `DhruvFullKundaliConfig` with `include_charakaraka` and `charakaraka_scheme`; extended `DhruvFullKundaliResult` with `charakaraka_valid` and `charakaraka`.

**v42**: Removed legacy split `dhruv_next_*`, `dhruv_prev_*`, `dhruv_search_*` (and `_utc`) wrappers from exported C ABI symbols for conjunction/grahan/motion/lunar-phase/sankranti families. Use unified `dhruv_conjunction_search_ex`, `dhruv_grahan_search_ex`, `dhruv_motion_search_ex`, `dhruv_lunar_phase_search_ex`, and `dhruv_sankranti_search_ex`.

**v41**: Removed legacy split Ayanamsha/Tara/Panchang entrypoints in favor of unified request-based APIs. Removed: `dhruv_ayanamsha_mean_deg`, `dhruv_ayanamsha_true_deg`, `dhruv_ayanamsha_deg`, `dhruv_ayanamsha_mean_deg_with_catalog`, `dhruv_ayanamsha_deg_with_catalog`, `dhruv_ayanamsha_mean_deg_utc`, `dhruv_ayanamsha_true_deg_utc`, `dhruv_ayanamsha_deg_utc`, `dhruv_ayanamsha_deg_with_catalog_utc`, `dhruv_tara_position_equatorial`, `dhruv_tara_position_equatorial_ex`, `dhruv_tara_position_ecliptic`, `dhruv_tara_position_ecliptic_ex`, `dhruv_tara_sidereal_longitude`, `dhruv_tara_sidereal_longitude_ex`, `dhruv_panchang_for_date`. Use `dhruv_ayanamsha_compute_ex`, `dhruv_tara_compute_ex`, and `dhruv_panchang_compute_ex`.

**v40**: Added unified operation APIs for panchang and tara. Panchang: new constants `DHRUV_PANCHANG_TIME_*`, `DHRUV_PANCHANG_INCLUDE_*`; new types `DhruvPanchangComputeRequest`, `DhruvPanchangOperationResult`; new function `dhruv_panchang_compute_ex`. Tara: new constants `DHRUV_TARA_OUTPUT_*`; new types `DhruvTaraComputeRequest`, `DhruvTaraComputeResult`; new function `dhruv_tara_compute_ex`.

**v37**: Added fixed star (tara) support. New types: `DhruvEquatorialPosition`, `DhruvEarthState`, `DhruvTaraConfig`. New functions: `dhruv_tara_catalog_load`, `dhruv_tara_catalog_free`, `dhruv_tara_position_equatorial`, `dhruv_tara_position_equatorial_ex`, `dhruv_tara_position_ecliptic`, `dhruv_tara_position_ecliptic_ex`, `dhruv_tara_sidereal_longitude`, `dhruv_tara_sidereal_longitude_ex`, `dhruv_tara_galactic_center_ecliptic`. Catalog loaded from JSON file (opaque handle). Two accuracy tiers: Astrometric (default) and Apparent (aberration + light deflection + nutation, requires `DhruvEarthState`). Optional parallax correction.

**v35.1** (Rust API): `NodeMode::default()` now returns `NodeMode::True` (was `NodeMode::Mean`). This aligns the Rust enum default with the jyotish pipeline which already used true nodes. The C ABI is unaffected — `dhruv_lunar_node_deg` requires an explicit `mode_code` parameter. CLI `lunar-node --mode` default changed from `mean` to `true`.

**v35**: Added `include_bhava_cusps` field to `DhruvFullKundaliConfig` (first field, default: 1). Added `ayanamsha_deg` (`double`), `bhava_cusps_valid` (`uint8_t`), `bhava_cusps` (`DhruvBhavaResult`) fields to `DhruvFullKundaliResult` (prepended before `graha_positions_valid`). `bhava_cusps_valid` is 1 only when `include_bhava_cusps` was non-zero and computation succeeded; 0 otherwise. Bhava cusps are only computed when `include_bhava_cusps` is non-zero, so panchang/calendar-only requests are not affected by house-system failures at high latitudes. Added `dhruv_full_kundali_config_default()` constructor that returns a config with core include flags (`include_bhava_cusps`, `include_graha_positions`, `include_bindus`, `include_drishti`, `include_ashtakavarga`, `include_upagrahas`, `include_special_lagnas`) set to 1, and optional sections (`include_amshas`, `include_shadbala`, `include_vimsopaka`, `include_avastha`, `include_panchang`, `include_calendar`, `include_dasha`) set to 0. C callers should use this instead of zero-initializing.

**v34**: Added `include_panchang`, `include_calendar` fields to `DhruvFullKundaliConfig`. Added `panchang_valid`, `panchang` (`DhruvPanchangInfo`) fields to `DhruvFullKundaliResult`. When `include_panchang` or `include_calendar` is non-zero, result includes panchang data. `include_calendar` implies `include_panchang`. Existing fields and offsets of prior struct members are unchanged (new fields appended).

**v33**: Added dasha integration to `DhruvFullKundaliConfig` / `DhruvFullKundaliResult`. Added `DhruvDashaSelectionConfig`, `DhruvDashaSnapshot`, `DhruvDashaPeriod` types. Added standalone request-based dasha entrypoints, accessor functions, and free functions.
