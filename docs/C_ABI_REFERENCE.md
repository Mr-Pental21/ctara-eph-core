# C ABI Reference

Complete reference for the `dhruv_ffi_c` C-compatible API surface.

**ABI version:** `DHRUV_API_VERSION = 39`

**Library:** `libdhruv_ffi_c` (compiled as `cdylib` + `staticlib`)

---

## Table of Contents

1. [Conventions](#conventions)
2. [Status Codes](#status-codes)
3. [Constants](#constants)
4. [Types](#types)
5. [Functions](#functions)
   - [Versioning](#versioning)
   - [Engine Lifecycle](#engine-lifecycle)
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
   - [Pure-Math Panchang Classifiers](#pure-math-panchang-classifiers)
   - [Graha Sidereal Longitudes](#graha-sidereal-longitudes)
   - [Graha Tropical Longitudes](#graha-tropical-longitudes)
   - [Nakshatra At](#nakshatra-at)
   - [Time Upagraha JD](#time-upagraha-jd)
   - [Pure-Math Ashtakavarga](#pure-math-ashtakavarga)
   - [Pure-Math Drishti](#pure-math-drishti)
   - [Pure-Math Ghatika / Hora](#pure-math-ghatika--hora)
   - [Fixed Stars (Tara)](#fixed-stars-tara)

---

## Conventions

- All functions return `DhruvStatus` (int32) unless stated otherwise.
- Null pointer arguments return `DHRUV_STATUS_NULL_POINTER` (7).
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
    int32_t system;           // DHRUV_BHAVA_* system code (0-9)
    int32_t starting_point;   // -1=Asc, -2=custom deg, or positive NAIF body code
    double  custom_start_deg; // Used only when starting_point == -2
    int32_t reference_mode;   // DHRUV_BHAVA_REF_* constant
} DhruvBhavaConfig;
```

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
} DhruvBhavaResult;
```

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
    double  p1_jd;                  // First penumbral contact
    double  u1_jd;                  // First umbral contact (-1.0 if absent)
    double  u2_jd;                  // Start of totality (-1.0 if absent)
    double  u3_jd;                  // End of totality (-1.0 if absent)
    double  u4_jd;                  // Last umbral contact (-1.0 if absent)
    double  p4_jd;                  // Last penumbral contact
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
    double  c1_jd;                  // First external contact (-1.0 if absent)
    double  c2_jd;                  // First internal contact (-1.0 if absent)
    double  c3_jd;                  // Last internal contact (-1.0 if absent)
    double  c4_jd;                  // Last external contact (-1.0 if absent)
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
DhruvStatus dhruv_query_once(
    const DhruvEngineConfig* config,
    const DhruvQuery*        query,
    DhruvStateVector*        out_state
);
```

One-shot convenience: creates engine, queries, and tears down internally.

```c
DhruvStatus dhruv_query_utc_spherical(
    const DhruvEngineHandle* engine,
    int32_t  target,    // NAIF body code
    int32_t  observer,  // NAIF body code
    int32_t  frame,     // Frame code
    int32_t  year,
    uint32_t month,
    uint32_t day,
    uint32_t hour,
    uint32_t min,
    double   sec,
    DhruvSphericalState* out
);
```

Query from UTC calendar date, returns spherical state (lon/lat/dist + rates). Combines UTC-to-TDB conversion, Cartesian query, and spherical conversion in one call.

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

All ayanamsha functions are pure math (no engine or kernel handles needed).

```c
DhruvStatus dhruv_ayanamsha_mean_deg(
    int32_t system_code,   // 0-19
    double  jd_tdb,
    double* out_deg
);
```

Mean ayanamsha at the given epoch.

```c
DhruvStatus dhruv_ayanamsha_true_deg(
    int32_t system_code,
    double  jd_tdb,
    double  delta_psi_arcsec,  // Nutation in longitude (arcsec)
    double* out_deg
);
```

`true`-mode helper ayanamsha.
`delta_psi_arcsec` is applied to all systems.

```c
DhruvStatus dhruv_ayanamsha_deg(
    int32_t system_code,
    double  jd_tdb,
    uint8_t use_nutation,  // 0 = mean, nonzero = auto-compute nutation
    double* out_deg
);
```

Unified function.
When `use_nutation` is non-zero, nutation in longitude (Δψ) is added for all systems.

```c
uint32_t dhruv_ayanamsha_system_count(void);
```

Returns number of supported systems (currently 20).

#### Star-catalog-aware variants

These functions accept an optional `DhruvTaraCatalogHandle*` parameter.
When non-null, star-anchored systems (TrueLahiri, PushyaPaksha, RohiniPaksha,
Aldebaran15Tau, GalacticCenter0Sag, ChandraHari) use proper-motion-corrected
star positions from the catalog. When null, behavior is identical to the
non-catalog versions.

```c
DhruvStatus dhruv_ayanamsha_mean_deg_with_catalog(
    int32_t system_code,
    double  jd_tdb,
    const DhruvTaraCatalogHandle* catalog,  // may be NULL
    double* out_deg
);
```

```c
DhruvStatus dhruv_ayanamsha_deg_with_catalog(
    int32_t system_code,
    double  jd_tdb,
    uint8_t use_nutation,
    const DhruvTaraCatalogHandle* catalog,  // may be NULL
    double* out_deg
);
```

```c
DhruvStatus dhruv_ayanamsha_deg_with_catalog_utc(
    const DhruvLskHandle* lsk,
    int32_t system_code,
    const DhruvUtcTime*   utc,
    uint8_t use_nutation,
    const DhruvTaraCatalogHandle* catalog,  // may be NULL
    double* out_deg
);
```

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

Compute 12 bhava cusps with Ascendant and MC.

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
DhruvStatus dhruv_next_conjunction(
    const DhruvEngineHandle*      engine,
    int32_t                       body1_code,  // NAIF code
    int32_t                       body2_code,  // NAIF code
    double                        jd_tdb,      // Search start
    const DhruvConjunctionConfig* config,
    DhruvConjunctionEvent*        out_event,
    uint8_t*                      out_found    // 1=found, 0=not found
);
```

Find the next conjunction/aspect event after `jd_tdb`.

```c
DhruvStatus dhruv_prev_conjunction(
    const DhruvEngineHandle*      engine,
    int32_t                       body1_code,
    int32_t                       body2_code,
    double                        jd_tdb,
    const DhruvConjunctionConfig* config,
    DhruvConjunctionEvent*        out_event,
    uint8_t*                      out_found
);
```

Find the previous conjunction/aspect event before `jd_tdb`.

```c
DhruvStatus dhruv_search_conjunctions(
    const DhruvEngineHandle*      engine,
    int32_t                       body1_code,
    int32_t                       body2_code,
    double                        jd_start,
    double                        jd_end,
    const DhruvConjunctionConfig* config,
    DhruvConjunctionEvent*        out_events,   // Array of max_count
    uint32_t                      max_count,
    uint32_t*                     out_count      // Actual count found
);
```

Search for all conjunction/aspect events in a time range.

---

### Chandra Grahan

```c
DhruvGrahanConfig dhruv_grahan_config_default(void);
```

Returns default: `include_penumbral=1`, `include_peak_details=1`.

```c
DhruvStatus dhruv_next_chandra_grahan(
    const DhruvEngineHandle*   engine,
    double                     jd_tdb,
    const DhruvGrahanConfig*   config,
    DhruvChandraGrahanResult*  out_result,
    uint8_t*                   out_found
);
```

Find the next chandra grahan (lunar eclipse) after `jd_tdb`.

```c
DhruvStatus dhruv_prev_chandra_grahan(
    const DhruvEngineHandle*   engine,
    double                     jd_tdb,
    const DhruvGrahanConfig*   config,
    DhruvChandraGrahanResult*  out_result,
    uint8_t*                   out_found
);
```

Find the previous chandra grahan (lunar eclipse) before `jd_tdb`.

```c
DhruvStatus dhruv_search_chandra_grahan(
    const DhruvEngineHandle*   engine,
    double                     jd_start,
    double                     jd_end,
    const DhruvGrahanConfig*   config,
    DhruvChandraGrahanResult*  out_results,  // Array of max_count
    uint32_t                   max_count,
    uint32_t*                  out_count
);
```

Search for all chandra grahan (lunar eclipses) in a time range.

---

### Surya Grahan

```c
DhruvStatus dhruv_next_surya_grahan(
    const DhruvEngineHandle*  engine,
    double                    jd_tdb,
    const DhruvGrahanConfig*  config,
    DhruvSuryaGrahanResult*   out_result,
    uint8_t*                  out_found
);
```

Find the next surya grahan (solar eclipse) after `jd_tdb`.

```c
DhruvStatus dhruv_prev_surya_grahan(
    const DhruvEngineHandle*  engine,
    double                    jd_tdb,
    const DhruvGrahanConfig*  config,
    DhruvSuryaGrahanResult*   out_result,
    uint8_t*                  out_found
);
```

Find the previous surya grahan (solar eclipse) before `jd_tdb`.

```c
DhruvStatus dhruv_search_surya_grahan(
    const DhruvEngineHandle*  engine,
    double                    jd_start,
    double                    jd_end,
    const DhruvGrahanConfig*  config,
    DhruvSuryaGrahanResult*   out_results,  // Array of max_count
    uint32_t                  max_count,
    uint32_t*                 out_count
);
```

Search for all surya grahan (solar eclipses) in a time range.

**Note:** Surya grahan computation is geocentric. Surface-specific effects (lunar parallax ~57') are not modeled.

---

### Stationary Point Search

```c
DhruvStationaryConfig dhruv_stationary_config_default(void);
```

Returns default: `step_size_days=1.0`, `max_iterations=50`, `convergence_days=1e-8`, `numerical_step_days=0.01`.

```c
DhruvStatus dhruv_next_stationary(
    const DhruvEngineHandle*     engine,
    int32_t                      body_code,   // NAIF code (not Sun/Moon/Earth)
    double                       jd_tdb,
    const DhruvStationaryConfig* config,
    DhruvStationaryEvent*        out_event,
    uint8_t*                     out_found
);
```

Find the next stationary point after `jd_tdb`. Returns `InvalidSearchConfig` for Sun, Moon, or Earth.

```c
DhruvStatus dhruv_prev_stationary(
    const DhruvEngineHandle*     engine,
    int32_t                      body_code,
    double                       jd_tdb,
    const DhruvStationaryConfig* config,
    DhruvStationaryEvent*        out_event,
    uint8_t*                     out_found
);
```

Find the previous stationary point before `jd_tdb`.

```c
DhruvStatus dhruv_search_stationary(
    const DhruvEngineHandle*     engine,
    int32_t                      body_code,
    double                       jd_start,
    double                       jd_end,
    const DhruvStationaryConfig* config,
    DhruvStationaryEvent*        out_events,  // Array of max_count
    uint32_t                     max_count,
    uint32_t*                    out_count
);
```

Search for all stationary points in a time range.

---

### Max Speed Search

```c
DhruvStatus dhruv_next_max_speed(
    const DhruvEngineHandle*     engine,
    int32_t                      body_code,   // NAIF code (not Earth)
    double                       jd_tdb,
    const DhruvStationaryConfig* config,
    DhruvMaxSpeedEvent*          out_event,
    uint8_t*                     out_found
);
```

Find the next max-speed event after `jd_tdb`. Sun and Moon are allowed. Returns `InvalidSearchConfig` for Earth.

```c
DhruvStatus dhruv_prev_max_speed(
    const DhruvEngineHandle*     engine,
    int32_t                      body_code,
    double                       jd_tdb,
    const DhruvStationaryConfig* config,
    DhruvMaxSpeedEvent*          out_event,
    uint8_t*                     out_found
);
```

Find the previous max-speed event before `jd_tdb`.

```c
DhruvStatus dhruv_search_max_speed(
    const DhruvEngineHandle*     engine,
    int32_t                      body_code,
    double                       jd_start,
    double                       jd_end,
    const DhruvStationaryConfig* config,
    DhruvMaxSpeedEvent*          out_events,  // Array of max_count
    uint32_t                     max_count,
    uint32_t*                    out_count
);
```

Search for all max-speed events in a time range.

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

### Graha Sidereal Longitudes

```c
typedef struct {
    double longitudes[9];   // Indexed by Graha order (Surya=0 .. Ketu=8)
} DhruvGrahaLongitudes;
```

```c
DhruvStatus dhruv_graha_sidereal_longitudes(
    const Engine*           engine,
    double                  jd_tdb,
    uint32_t                ayanamsha_system,   // 0-19
    uint8_t                 use_nutation,        // 0=false, 1=true
    DhruvGrahaLongitudes*   out
);
```

Query sidereal longitudes (degrees, 0..360) of all 9 grahas at a given JD (TDB). For the 7 physical planets, queries the engine for tropical ecliptic longitude and subtracts ayanamsha. For Rahu/Ketu, uses true node formulas.

---

### Graha Tropical Longitudes

```c
DhruvStatus dhruv_graha_tropical_longitudes(
    const Engine*           engine,
    double                  jd_tdb,
    DhruvGrahaLongitudes*   out
);
```

Query tropical (ecliptic-of-date) longitudes (degrees, 0..360) of all 9 grahas at a given JD (TDB). Returns raw ecliptic-of-date longitudes without ayanamsha subtraction. No `ayanamsha_system` or `use_nutation` parameters — tropical is frame-only. Uses the same `DhruvGrahaLongitudes` output struct as `dhruv_graha_sidereal_longitudes`.

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

Determine the Moon's Nakshatra (27-scheme) from a pre-computed sidereal longitude. The engine is still needed for boundary bisection (finding start/end times). Returns nakshatra index, pada, and start/end times (UTC). Useful when the Moon's sidereal longitude has already been computed (e.g., from `dhruv_graha_sidereal_longitudes`).

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

Calculate BAV (Bhinna Ashtakavarga) for a single graha. Returns `DHRUV_STATUS_INVALID_QUERY` for `graha_index > 6`.

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

## Function Summary

| # | Function | Engine | LSK | EOP | Pure Math |
|---|----------|--------|-----|-----|-----------|
| 1 | `dhruv_api_version` | | | | yes |
| 2 | `dhruv_engine_new` | creates | | | |
| 3 | `dhruv_engine_free` | destroys | | | |
| 4 | `dhruv_engine_query` | yes | | | |
| 5 | `dhruv_query_once` | internal | | | |
| 6 | `dhruv_query_utc_spherical` | yes | | | |
| 7 | `dhruv_lsk_load` | | creates | | |
| 8 | `dhruv_lsk_free` | | destroys | | |
| 9 | `dhruv_eop_load` | | | creates | |
| 10 | `dhruv_eop_free` | | | destroys | |
| 11 | `dhruv_utc_to_tdb_jd` | | yes | | |
| 12 | `dhruv_jd_tdb_to_utc` | | yes | | |
| 13 | `dhruv_riseset_result_to_utc` | | yes | | |
| 14 | `dhruv_cartesian_to_spherical` | | | | yes |
| 15 | `dhruv_ayanamsha_mean_deg` | | | | yes |
| 16 | `dhruv_ayanamsha_true_deg` | | | | yes |
| 17 | `dhruv_ayanamsha_deg` | | | | yes |
| 18 | `dhruv_ayanamsha_system_count` | | | | yes |
| 19 | `dhruv_nutation_iau2000b` | | | | yes |
| 20 | `dhruv_lunar_node_deg` | | | | yes |
| 21 | `dhruv_lunar_node_count` | | | | yes |
| 22 | `dhruv_riseset_config_default` | | | | yes |
| 23 | `dhruv_compute_rise_set` | yes | yes | yes | |
| 24 | `dhruv_compute_all_events` | yes | yes | yes | |
| 25 | `dhruv_approximate_local_noon_jd` | | | | yes |
| 26 | `dhruv_bhava_config_default` | | | | yes |
| 27 | `dhruv_bhava_system_count` | | | | yes |
| 28 | `dhruv_compute_bhavas` | yes | yes | yes | |
| 29 | `dhruv_lagna_deg` | | yes | yes | |
| 30 | `dhruv_mc_deg` | | yes | yes | |
| 31 | `dhruv_conjunction_config_default` | | | | yes |
| 32 | `dhruv_next_conjunction` | yes | | | |
| 33 | `dhruv_prev_conjunction` | yes | | | |
| 34 | `dhruv_search_conjunctions` | yes | | | |
| 35 | `dhruv_grahan_config_default` | | | | yes |
| 36 | `dhruv_next_chandra_grahan` | yes | | | |
| 37 | `dhruv_prev_chandra_grahan` | yes | | | |
| 38 | `dhruv_search_chandra_grahan` | yes | | | |
| 39 | `dhruv_next_surya_grahan` | yes | | | |
| 40 | `dhruv_prev_surya_grahan` | yes | | | |
| 41 | `dhruv_search_surya_grahan` | yes | | | |
| 42 | `dhruv_stationary_config_default` | | | | yes |
| 43 | `dhruv_next_stationary` | yes | | | |
| 44 | `dhruv_prev_stationary` | yes | | | |
| 45 | `dhruv_search_stationary` | yes | | | |
| 46 | `dhruv_next_max_speed` | yes | | | |
| 47 | `dhruv_prev_max_speed` | yes | | | |
| 48 | `dhruv_search_max_speed` | yes | | | |
| 49 | `dhruv_graha_sidereal_longitudes` | yes | | | |
| 50 | `dhruv_graha_tropical_longitudes` | yes | | | |
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
| 75 | `dhruv_dasha_hierarchy_utc` | yes | | | |
| 76 | `dhruv_dasha_snapshot_utc` | yes | | | |
| 77 | `dhruv_dasha_hierarchy_level_count` | | | | yes |
| 78 | `dhruv_dasha_hierarchy_period_count` | | | | yes |
| 79 | `dhruv_dasha_hierarchy_period_at` | | | | yes |
| 80 | `dhruv_dasha_hierarchy_free` | | | | yes |
| 81 | `dhruv_full_kundali_result_free` | | | | yes |
| 82 | `dhruv_full_kundali_config_default` | | | | yes |
| 83 | `dhruv_tara_catalog_load` | | | | yes |
| 84 | `dhruv_tara_catalog_free` | | | | yes |
| 85 | `dhruv_tara_position_equatorial` | | | | yes |
| 86 | `dhruv_tara_position_equatorial_ex` | | | | yes |
| 87 | `dhruv_tara_position_ecliptic` | | | | yes |
| 88 | `dhruv_tara_position_ecliptic_ex` | | | | yes |
| 89 | `dhruv_tara_sidereal_longitude` | | | | yes |
| 90 | `dhruv_tara_sidereal_longitude_ex` | | | | yes |
| 91 | `dhruv_tara_galactic_center_ecliptic` | | | | yes |
| 92 | `dhruv_ayanamsha_mean_deg_with_catalog` | | | | yes |
| 93 | `dhruv_ayanamsha_deg_with_catalog` | | | | yes |
| 94 | `dhruv_ayanamsha_deg_with_catalog_utc` | | yes | | |

**Total exported symbols: 82 functions**

---

## Dasha Types

### `DhruvDashaSelectionConfig`

```c
struct DhruvDashaSelectionConfig {
    uint8_t count;           // number of valid entries in systems (0..8)
    uint8_t systems[8];      // DashaSystem codes (0xFF = unused)
    uint8_t max_level;       // hierarchy depth (0-4, default 2)
    uint8_t level_methods[5]; // per-level sub-period method (0xFF = default)
    uint8_t yogini_scheme;   // 0 = default
    uint8_t use_abhijit;     // 1 = yes, 0 = no
    uint8_t has_snapshot_jd; // 0 = no snapshot, 1 = snapshot_jd is valid
    double  snapshot_jd;     // JD UTC, only read when has_snapshot_jd == 1
};
```

### `DhruvDashaPeriod`

```c
struct DhruvDashaPeriod {
    uint8_t  entity_type;  // 0=Graha, 1=Rashi, 2=Yogini
    uint8_t  entity_index; // Graha (0-8), Rashi (0-11), Yogini (0-7)
    double   start_jd;     // JD UTC, inclusive
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
    double            query_jd;   // echoed query JD UTC
    uint8_t           count;      // number of valid periods (0-5)
    DhruvDashaPeriod  periods[5]; // one per level
};
```

## JD Timescale Notes (Dasha)

All JD values in dasha APIs use **JD UTC** (not TDB):
- `DhruvDashaSelectionConfig.snapshot_jd` — query time (only when `has_snapshot_jd == 1`)
- `DhruvDashaPeriod.start_jd`, `.end_jd` — period boundaries
- `DhruvDashaSnapshot.query_jd` — echoed query time

## Ownership & Lifetime Table (Dasha)

| Handle/Resource | Allocated by | Freed by | Notes |
|-----------------|-------------|----------|-------|
| `DhruvDashaHierarchyHandle` (standalone) | `dhruv_dasha_hierarchy_utc` | `dhruv_dasha_hierarchy_free` | Caller owns. Must free exactly once. |
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
DhruvStatus dhruv_tara_position_equatorial(
    const DhruvTaraCatalogHandle* handle,
    int32_t                       tara_id,
    double                        jd_tdb,
    DhruvEquatorialPosition*      out
);
```

Compute equatorial position (Astrometric, no parallax). Returns RA/Dec/distance
at the given TDB epoch.

```c
DhruvStatus dhruv_tara_position_equatorial_ex(
    const DhruvTaraCatalogHandle* handle,
    int32_t                       tara_id,
    double                        jd_tdb,
    const DhruvTaraConfig*        config,
    const DhruvEarthState*        earth_state,  // may be null for Astrometric without parallax
    DhruvEquatorialPosition*      out
);
```

Compute equatorial position with config. Pass `earth_state` for Apparent tier
or when `config.apply_parallax == 1`.

```c
DhruvStatus dhruv_tara_position_ecliptic(
    const DhruvTaraCatalogHandle* handle,
    int32_t                       tara_id,
    double                        jd_tdb,
    DhruvSphericalCoords*         out
);
```

Compute ecliptic position of date (Astrometric, no parallax). Output in
`DhruvSphericalCoords` (lon_deg, lat_deg, distance_km).

```c
DhruvStatus dhruv_tara_position_ecliptic_ex(
    const DhruvTaraCatalogHandle* handle,
    int32_t                       tara_id,
    double                        jd_tdb,
    const DhruvTaraConfig*        config,
    const DhruvEarthState*        earth_state,  // may be null for Astrometric without parallax
    DhruvSphericalCoords*         out
);
```

Compute ecliptic position with config.

```c
DhruvStatus dhruv_tara_sidereal_longitude(
    const DhruvTaraCatalogHandle* handle,
    int32_t                       tara_id,
    double                        jd_tdb,
    double                        ayanamsha_deg,
    double*                       out
);
```

Compute sidereal longitude (Astrometric, no parallax). Result is
`tropical_ecliptic_longitude - ayanamsha_deg`, normalized to [0, 360).
Caller computes `ayanamsha_deg` separately (e.g., via `dhruv_ayanamsha_deg`).

```c
DhruvStatus dhruv_tara_sidereal_longitude_ex(
    const DhruvTaraCatalogHandle* handle,
    int32_t                       tara_id,
    double                        jd_tdb,
    double                        ayanamsha_deg,
    const DhruvTaraConfig*        config,
    const DhruvEarthState*        earth_state,  // may be null for Astrometric without parallax
    double*                       out
);
```

Compute sidereal longitude with config.

```c
DhruvStatus dhruv_tara_galactic_center_ecliptic(
    const DhruvTaraCatalogHandle* handle,
    double                        jd_tdb,
    DhruvSphericalCoords*         out
);
```

Compute ecliptic position of the Galactic Center (IAU 2000 ICRS direction,
no proper motion). Equivalent to calling `dhruv_tara_position_ecliptic` with
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

**v37**: Added fixed star (tara) support. New types: `DhruvEquatorialPosition`, `DhruvEarthState`, `DhruvTaraConfig`. New functions: `dhruv_tara_catalog_load`, `dhruv_tara_catalog_free`, `dhruv_tara_position_equatorial`, `dhruv_tara_position_equatorial_ex`, `dhruv_tara_position_ecliptic`, `dhruv_tara_position_ecliptic_ex`, `dhruv_tara_sidereal_longitude`, `dhruv_tara_sidereal_longitude_ex`, `dhruv_tara_galactic_center_ecliptic`. Catalog loaded from JSON file (opaque handle). Two accuracy tiers: Astrometric (default) and Apparent (aberration + light deflection + nutation, requires `DhruvEarthState`). Optional parallax correction.

**v35.1** (Rust API): `NodeMode::default()` now returns `NodeMode::True` (was `NodeMode::Mean`). This aligns the Rust enum default with the jyotish pipeline which already used true nodes. The C ABI is unaffected — `dhruv_lunar_node_deg` requires an explicit `mode_code` parameter. CLI `lunar-node --mode` default changed from `mean` to `true`.

**v35**: Added `include_bhava_cusps` field to `DhruvFullKundaliConfig` (first field, default: 1). Added `ayanamsha_deg` (`double`), `bhava_cusps_valid` (`uint8_t`), `bhava_cusps` (`DhruvBhavaResult`) fields to `DhruvFullKundaliResult` (prepended before `graha_positions_valid`). `bhava_cusps_valid` is 1 only when `include_bhava_cusps` was non-zero and computation succeeded; 0 otherwise. Bhava cusps are only computed when `include_bhava_cusps` is non-zero, so panchang/calendar-only requests are not affected by house-system failures at high latitudes. Added `dhruv_full_kundali_config_default()` constructor that returns a config with core include flags (`include_bhava_cusps`, `include_graha_positions`, `include_bindus`, `include_drishti`, `include_ashtakavarga`, `include_upagrahas`, `include_special_lagnas`) set to 1, and optional sections (`include_amshas`, `include_shadbala`, `include_vimsopaka`, `include_avastha`, `include_panchang`, `include_calendar`, `include_dasha`) set to 0. C callers should use this instead of zero-initializing.

**v34**: Added `include_panchang`, `include_calendar` fields to `DhruvFullKundaliConfig`. Added `panchang_valid`, `panchang` (`DhruvPanchangInfo`) fields to `DhruvFullKundaliResult`. When `include_panchang` or `include_calendar` is non-zero, result includes panchang data. `include_calendar` implies `include_panchang`. Existing fields and offsets of prior struct members are unchanged (new fields appended).

**v33**: Added dasha integration to `DhruvFullKundaliConfig` / `DhruvFullKundaliResult`. Added `DhruvDashaSelectionConfig`, `DhruvDashaSnapshot`, `DhruvDashaPeriod` types. Added standalone `dhruv_dasha_hierarchy_utc`, `dhruv_dasha_snapshot_utc`, accessor, and free functions.
