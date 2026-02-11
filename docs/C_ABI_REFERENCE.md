# C ABI Reference

Complete reference for the `dhruv_ffi_c` C-compatible API surface.

**ABI version:** `DHRUV_API_VERSION = 7`

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
   - [Lunar Eclipse](#lunar-eclipse)
   - [Solar Eclipse](#solar-eclipse)

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
| `DHRUV_NODE_MODE_TRUE` | 1 | True (mean + perturbation corrections) |

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
| `DHRUV_BHAVA_START_ASCENDANT` | -1 | Use Ascendant as starting point |
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

### Eclipse Type Codes

**Lunar:**

| Constant | Value |
|----------|-------|
| `DHRUV_LUNAR_ECLIPSE_PENUMBRAL` | 0 |
| `DHRUV_LUNAR_ECLIPSE_PARTIAL` | 1 |
| `DHRUV_LUNAR_ECLIPSE_TOTAL` | 2 |

**Solar:**

| Constant | Value |
|----------|-------|
| `DHRUV_SOLAR_ECLIPSE_PARTIAL` | 0 |
| `DHRUV_SOLAR_ECLIPSE_ANNULAR` | 1 |
| `DHRUV_SOLAR_ECLIPSE_TOTAL` | 2 |
| `DHRUV_SOLAR_ECLIPSE_HYBRID` | 3 |

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
    double lon_rad;      // Longitude [0, 2*pi)
    double lat_rad;      // Latitude [-pi/2, pi/2]
    double distance_km;
} DhruvSphericalCoords;
```

### DhruvSphericalState

```c
typedef struct {
    double lon_rad;         // Longitude [0, 2*pi)
    double lat_rad;         // Latitude [-pi/2, pi/2]
    double distance_km;
    double lon_speed;       // rad/s
    double lat_speed;       // rad/s
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
    double     ascendant_deg;  // Ascendant longitude in degrees
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

### DhruvEclipseConfig

```c
typedef struct {
    uint8_t include_penumbral;    // 1 = include penumbral-only lunar eclipses
    uint8_t include_peak_details; // 1 = include lat/separation at peak
} DhruvEclipseConfig;
```

### DhruvLunarEclipseResult

```c
typedef struct {
    int32_t eclipse_type;           // DHRUV_LUNAR_ECLIPSE_* constant
    double  magnitude;              // Umbral magnitude
    double  penumbral_magnitude;
    double  greatest_eclipse_jd;    // JD TDB
    double  p1_jd;                  // First penumbral contact
    double  u1_jd;                  // First umbral contact (-1.0 if absent)
    double  u2_jd;                  // Start of totality (-1.0 if absent)
    double  u3_jd;                  // End of totality (-1.0 if absent)
    double  u4_jd;                  // Last umbral contact (-1.0 if absent)
    double  p4_jd;                  // Last penumbral contact
    double  moon_ecliptic_lat_deg;  // Moon lat at greatest eclipse
    double  angular_separation_deg; // Separation at greatest eclipse
} DhruvLunarEclipseResult;
```

### DhruvSolarEclipseResult

```c
typedef struct {
    int32_t eclipse_type;           // DHRUV_SOLAR_ECLIPSE_* constant
    double  magnitude;              // Moon/Sun apparent diameter ratio
    double  greatest_eclipse_jd;    // JD TDB
    double  c1_jd;                  // First external contact (-1.0 if absent)
    double  c2_jd;                  // First internal contact (-1.0 if absent)
    double  c3_jd;                  // Last internal contact (-1.0 if absent)
    double  c4_jd;                  // Last external contact (-1.0 if absent)
    double  moon_ecliptic_lat_deg;  // Moon lat at greatest eclipse
    double  angular_separation_deg; // Separation at greatest eclipse
} DhruvSolarEclipseResult;
```

---

## Functions

### Versioning

```c
uint32_t dhruv_api_version(void);
```

Returns the ABI version number (currently 7).

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

Convert Cartesian [x,y,z] (km) to spherical (lon_rad, lat_rad, distance_km). Pure math, no engine needed.

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

True (nutation-corrected) ayanamsha. Only affects TrueLahiri (code 1); others return mean.

```c
DhruvStatus dhruv_ayanamsha_deg(
    int32_t system_code,
    double  jd_tdb,
    uint8_t use_nutation,  // 0 = mean, nonzero = auto-compute nutation
    double* out_deg
);
```

Unified function. When `use_nutation != 0`, automatically computes IAU 2000B nutation and applies it (only relevant for TrueLahiri).

```c
uint32_t dhruv_ayanamsha_system_count(void);
```

Returns number of supported systems (currently 20).

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
DhruvStatus dhruv_ascendant_deg(
    const DhruvLskHandle*   lsk,
    const DhruvEopHandle*   eop,
    const DhruvGeoLocation* location,
    double                  jd_utc,
    double*                 out_deg
);
```

Compute Ascendant ecliptic longitude in degrees. No engine needed.

```c
DhruvStatus dhruv_mc_deg(
    const DhruvLskHandle*   lsk,
    const DhruvEopHandle*   eop,
    const DhruvGeoLocation* location,
    double                  jd_utc,
    double*                 out_deg
);
```

Compute MC (Midheaven) ecliptic longitude in degrees. No engine needed.

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

### Lunar Eclipse

```c
DhruvEclipseConfig dhruv_eclipse_config_default(void);
```

Returns default: `include_penumbral=1`, `include_peak_details=1`.

```c
DhruvStatus dhruv_next_lunar_eclipse(
    const DhruvEngineHandle*  engine,
    double                    jd_tdb,
    const DhruvEclipseConfig* config,
    DhruvLunarEclipseResult*  out_result,
    uint8_t*                  out_found
);
```

Find the next lunar eclipse after `jd_tdb`.

```c
DhruvStatus dhruv_prev_lunar_eclipse(
    const DhruvEngineHandle*  engine,
    double                    jd_tdb,
    const DhruvEclipseConfig* config,
    DhruvLunarEclipseResult*  out_result,
    uint8_t*                  out_found
);
```

Find the previous lunar eclipse before `jd_tdb`.

```c
DhruvStatus dhruv_search_lunar_eclipses(
    const DhruvEngineHandle*  engine,
    double                    jd_start,
    double                    jd_end,
    const DhruvEclipseConfig* config,
    DhruvLunarEclipseResult*  out_results,  // Array of max_count
    uint32_t                  max_count,
    uint32_t*                 out_count
);
```

Search for all lunar eclipses in a time range.

---

### Solar Eclipse

```c
DhruvStatus dhruv_next_solar_eclipse(
    const DhruvEngineHandle*  engine,
    double                    jd_tdb,
    const DhruvEclipseConfig* config,
    DhruvSolarEclipseResult*  out_result,
    uint8_t*                  out_found
);
```

Find the next solar eclipse after `jd_tdb`.

```c
DhruvStatus dhruv_prev_solar_eclipse(
    const DhruvEngineHandle*  engine,
    double                    jd_tdb,
    const DhruvEclipseConfig* config,
    DhruvSolarEclipseResult*  out_result,
    uint8_t*                  out_found
);
```

Find the previous solar eclipse before `jd_tdb`.

```c
DhruvStatus dhruv_search_solar_eclipses(
    const DhruvEngineHandle*  engine,
    double                    jd_start,
    double                    jd_end,
    const DhruvEclipseConfig* config,
    DhruvSolarEclipseResult*  out_results,  // Array of max_count
    uint32_t                  max_count,
    uint32_t*                 out_count
);
```

Search for all solar eclipses in a time range.

**Note:** Solar eclipse computation is geocentric. Surface-specific effects (lunar parallax ~57') are not modeled.

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
| 29 | `dhruv_ascendant_deg` | | yes | yes | |
| 30 | `dhruv_mc_deg` | | yes | yes | |
| 31 | `dhruv_conjunction_config_default` | | | | yes |
| 32 | `dhruv_next_conjunction` | yes | | | |
| 33 | `dhruv_prev_conjunction` | yes | | | |
| 34 | `dhruv_search_conjunctions` | yes | | | |
| 35 | `dhruv_eclipse_config_default` | | | | yes |
| 36 | `dhruv_next_lunar_eclipse` | yes | | | |
| 37 | `dhruv_prev_lunar_eclipse` | yes | | | |
| 38 | `dhruv_search_lunar_eclipses` | yes | | | |
| 39 | `dhruv_next_solar_eclipse` | yes | | | |
| 40 | `dhruv_prev_solar_eclipse` | yes | | | |
| 41 | `dhruv_search_solar_eclipses` | yes | | | |

**Total exported symbols: 41 functions**
