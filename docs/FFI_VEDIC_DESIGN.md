# C ABI Design: Ayanamsha and Sunrise/Sunset

## Context

The `dhruv_vedic_base` crate has full Rust implementations for ayanamsha (20 systems)
and sunrise/sunset (with twilight). The `dhruv_ffi_c` crate provides C ABI bindings for
the core engine but has no vedic base bindings. This document specifies C ABI functions
for ayanamsha and rise/set, following the existing FFI patterns.

---

## Files to Modify

| File | Change |
|------|--------|
| `crates/dhruv_ffi_c/Cargo.toml` | Add `dhruv_vedic_base` dependency |
| `crates/dhruv_ffi_c/src/lib.rs` | Add new types, status codes, and FFI functions |
| `crates/dhruv_ffi_c/tests/ffi_integration.rs` | Add integration tests |

---

## New Status Codes

Extend `DhruvStatus` enum (existing codes 0-7, 255):

```rust
EopLoad = 8,          // EOP file failed to load
EopOutOfRange = 9,    // Epoch outside EOP table range
InvalidLocation = 10, // Bad lat/lon/alt
NoConvergence = 11,   // Rise/set iteration didn't converge
```

Add `From<&VedicError>` impl mapping to these codes.

---

## New Opaque Handle: EOP

Follow the same pattern as `DhruvLskHandle`:

```c
typedef struct DhruvEopHandle DhruvEopHandle;

// Load EOP from NUL-terminated file path, returns opaque handle
DhruvStatus dhruv_eop_load(const uint8_t* path, DhruvEopHandle** out);

// Destroy EOP handle (null-safe)
DhruvStatus dhruv_eop_free(DhruvEopHandle* eop);
```

---

## Ayanamsha ABI

Ayanamsha is pure math — no engine, no handles needed. Only needs a system code and epoch.

### System Codes

Integer codes 0-19 matching `AyanamshaSystem` enum order:

```c
// Ayanamsha system codes
#define DHRUV_AYANAMSHA_LAHIRI              0
#define DHRUV_AYANAMSHA_TRUE_LAHIRI         1
#define DHRUV_AYANAMSHA_KP                  2
#define DHRUV_AYANAMSHA_RAMAN               3
#define DHRUV_AYANAMSHA_FAGAN_BRADLEY       4
#define DHRUV_AYANAMSHA_PUSHYA_PAKSHA       5
#define DHRUV_AYANAMSHA_ROHINI_PAKSHA       6
#define DHRUV_AYANAMSHA_DELUCE              7
#define DHRUV_AYANAMSHA_DJWAL_KHUL          8
#define DHRUV_AYANAMSHA_HIPPARCHOS          9
#define DHRUV_AYANAMSHA_SASSANIAN           10
#define DHRUV_AYANAMSHA_DEVA_DUTTA          11
#define DHRUV_AYANAMSHA_USHA_SHASHI         12
#define DHRUV_AYANAMSHA_YUKTESHWAR          13
#define DHRUV_AYANAMSHA_JN_BHASIN           14
#define DHRUV_AYANAMSHA_CHANDRA_HARI        15
#define DHRUV_AYANAMSHA_JAGGANATHA          16
#define DHRUV_AYANAMSHA_SURYA_SIDDHANTA     17
#define DHRUV_AYANAMSHA_GALACTIC_CENTER_0SAG 18
#define DHRUV_AYANAMSHA_ALDEBARAN_15TAU     19
```

### Functions

```c
// Mean ayanamsha at JD TDB (pure math, no engine needed)
// Writes result to *out_deg
DhruvStatus dhruv_ayanamsha_mean_deg(
    int32_t system_code,    // 0-19, see codes above
    double  jd_tdb,         // Julian Date in TDB
    double* out_deg         // output: ayanamsha in degrees
);

// True (nutation-corrected) ayanamsha
// For TrueLahiri, adds delta_psi to mean value
// For all other systems, ignores delta_psi and returns mean
DhruvStatus dhruv_ayanamsha_true_deg(
    int32_t system_code,
    double  jd_tdb,
    double  delta_psi_arcsec,  // nutation in longitude (arcsec)
    double* out_deg
);

// Number of supported ayanamsha systems (returns 20)
uint32_t dhruv_ayanamsha_system_count(void);
```

### Rust Internal

```rust
fn ayanamsha_system_from_code(code: i32) -> Option<AyanamshaSystem>
```

Maps integer 0..19 to enum variants via `AyanamshaSystem::all()` array index.
Returns `None` for out-of-range codes, which maps to `DhruvStatus::InvalidQuery`.

---

## Rise/Set ABI

### C-Compatible Structs

```c
// Geographic location
typedef struct {
    double latitude_deg;    // [-90, 90], north positive
    double longitude_deg;   // [-180, 180], east positive
    double altitude_m;      // meters above sea level
} DhruvGeoLocation;

// Rise/set configuration
typedef struct {
    double  refraction_arcmin;    // atmospheric refraction (default 34.0)
    double  semidiameter_arcmin;  // solar semi-diameter (default 16.0)
    uint8_t altitude_correction;  // apply dip correction (default 1 = true)
} DhruvRiseSetConfig;

// Rise/set result
typedef struct {
    int32_t result_type;    // 0=Event, 1=NeverRises, 2=NeverSets
    int32_t event_code;     // which event (valid when result_type==0)
    double  jd_tdb;         // event time in JD TDB (valid when result_type==0)
} DhruvRiseSetResult;
```

### Event Codes

```c
#define DHRUV_EVENT_SUNRISE             0
#define DHRUV_EVENT_SUNSET              1
#define DHRUV_EVENT_CIVIL_DAWN          2
#define DHRUV_EVENT_CIVIL_DUSK          3
#define DHRUV_EVENT_NAUTICAL_DAWN       4
#define DHRUV_EVENT_NAUTICAL_DUSK       5
#define DHRUV_EVENT_ASTRONOMICAL_DAWN   6
#define DHRUV_EVENT_ASTRONOMICAL_DUSK   7
```

### Result Type Codes

```c
#define DHRUV_RISESET_EVENT        0  // Event occurred, jd_tdb is valid
#define DHRUV_RISESET_NEVER_RISES  1  // Polar night — Sun stays below horizon
#define DHRUV_RISESET_NEVER_SETS   2  // Midnight sun — Sun stays above horizon
```

### Functions

```c
// Returns default config: refraction=34', semidiameter=16', altitude_correction=1
DhruvRiseSetConfig dhruv_riseset_config_default(void);

// Compute a single rise/set event
// engine: ephemeris engine for Sun position queries
// lsk:    leap second kernel for UTC-TDB conversion
// eop:    IERS EOP for UTC-UT1 conversion
DhruvStatus dhruv_compute_rise_set(
    const DhruvEngineHandle*  engine,
    const DhruvLskHandle*     lsk,
    const DhruvEopHandle*     eop,
    const DhruvGeoLocation*   location,
    int32_t                   event_code,     // 0-7
    double                    jd_utc_noon,    // approximate local noon (UTC JD)
    const DhruvRiseSetConfig* config,
    DhruvRiseSetResult*       out_result
);

// Compute all 8 events for a day
// Caller must provide array of at least 8 DhruvRiseSetResult
// Order: AstroDawn, NautDawn, CivilDawn, Sunrise,
//        Sunset, CivilDusk, NautDusk, AstroDusk
DhruvStatus dhruv_compute_all_events(
    const DhruvEngineHandle*  engine,
    const DhruvLskHandle*     lsk,
    const DhruvEopHandle*     eop,
    const DhruvGeoLocation*   location,
    double                    jd_utc_noon,
    const DhruvRiseSetConfig* config,
    DhruvRiseSetResult*       out_results     // array of 8
);

// Utility: approximate local noon JD from 0h UT JD and longitude
// Pure math, no handles needed
double dhruv_approximate_local_noon_jd(
    double jd_ut_midnight,
    double longitude_deg
);
```

---

## Unit Tests (in `src/lib.rs`)

| Test | Assertion |
|------|-----------|
| `ffi_ayanamsha_rejects_invalid_code` | code=99 → InvalidQuery |
| `ffi_ayanamsha_rejects_null_output` | null out → NullPointer |
| `ffi_ayanamsha_lahiri_at_j2000` | code=0, jd=2451545.0 → ~23.853° |
| `ffi_riseset_config_default` | refraction=34.0, semidiameter=16.0, correction=1 |
| `ffi_eop_load_rejects_null` | null path → NullPointer |
| `ffi_compute_rise_set_rejects_null` | null engine → NullPointer |
| `ffi_local_noon` | lon=0 → JD_0h + 0.5 |

## Integration Tests (in `tests/ffi_integration.rs`, skip if kernels absent)

| Test | Assertion |
|------|-----------|
| `ffi_ayanamsha_mean_lahiri_j2000` | system=0, jd=2451545.0 → 23.85° ± 0.01° |
| `ffi_eop_lifecycle` | load + free succeeds, no leak |
| `ffi_sunrise_new_delhi` | 2024-03-20 sunrise ~00:48 UTC ±6 min |
| `ffi_polar_never_sets` | Tromso 2024-06-21 sunrise → result_type=2 (NeverSets) |

---

## C Usage Example

```c
#include "dhruv.h"
#include <stdio.h>

int main(void) {
    DhruvStatus st;

    // ========================================
    // AYANAMSHA (pure math, no engine needed)
    // ========================================

    double lahiri_deg;
    st = dhruv_ayanamsha_mean_deg(
        DHRUV_AYANAMSHA_LAHIRI,   // system code
        2460310.5,                // JD TDB for 2024-Jan-01
        &lahiri_deg               // output
    );
    if (st == DHRUV_STATUS_OK) {
        printf("Lahiri ayanamsha at 2024: %.3f deg\n", lahiri_deg);
        // Output: ~24.19 deg
    }

    // True Lahiri with nutation correction
    double true_lahiri;
    st = dhruv_ayanamsha_true_deg(
        DHRUV_AYANAMSHA_TRUE_LAHIRI,
        2460310.5,
        -12.5,           // delta_psi in arcseconds (from nutation model)
        &true_lahiri
    );
    // true_lahiri ≈ lahiri - 12.5/3600

    // Loop over all systems
    uint32_t count = dhruv_ayanamsha_system_count();
    for (int i = 0; i < (int)count; i++) {
        double val;
        dhruv_ayanamsha_mean_deg(i, 2451545.0, &val);
        printf("System %d at J2000: %.3f deg\n", i, val);
    }

    // ========================================
    // SUNRISE / SUNSET
    // ========================================

    // Step 1: Load engine, LSK, and EOP
    DhruvEngineConfig config = { /* ... fill spk/lsk paths ... */ };
    DhruvEngineHandle* engine = NULL;
    DhruvLskHandle*    lsk    = NULL;
    DhruvEopHandle*    eop    = NULL;

    dhruv_engine_new(&config, &engine);
    dhruv_lsk_load("naif0012.tls\0", &lsk);
    dhruv_eop_load("finals2000A.all\0", &eop);

    // Step 2: Set location (New Delhi)
    DhruvGeoLocation loc = {
        .latitude_deg  = 28.6139,
        .longitude_deg = 77.209,
        .altitude_m    = 0.0
    };

    // Step 3: Compute local noon for 2024-03-20
    double jd_0h = 2460388.5;  // 2024-03-20 0h UT
    double noon  = dhruv_approximate_local_noon_jd(jd_0h, loc.longitude_deg);

    // Step 4: Compute single event (sunrise)
    DhruvRiseSetConfig rs_config = dhruv_riseset_config_default();
    DhruvRiseSetResult result;

    st = dhruv_compute_rise_set(
        engine, lsk, eop,
        &loc,
        DHRUV_EVENT_SUNRISE,
        noon,
        &rs_config,
        &result
    );

    if (st == DHRUV_STATUS_OK) {
        switch (result.result_type) {
        case DHRUV_RISESET_EVENT:
            printf("Sunrise at JD TDB: %.6f\n", result.jd_tdb);
            break;
        case DHRUV_RISESET_NEVER_RISES:
            printf("Polar night — Sun never rises\n");
            break;
        case DHRUV_RISESET_NEVER_SETS:
            printf("Midnight sun — Sun never sets\n");
            break;
        }
    }

    // Step 5: Compute all 8 events for the day
    DhruvRiseSetResult all_events[8];
    st = dhruv_compute_all_events(
        engine, lsk, eop,
        &loc,
        noon,
        &rs_config,
        all_events
    );

    if (st == DHRUV_STATUS_OK) {
        const char* names[] = {
            "Astro Dawn", "Naut Dawn", "Civil Dawn", "Sunrise",
            "Sunset", "Civil Dusk", "Naut Dusk", "Astro Dusk"
        };
        for (int i = 0; i < 8; i++) {
            if (all_events[i].result_type == DHRUV_RISESET_EVENT) {
                printf("%-12s JD TDB: %.6f\n", names[i], all_events[i].jd_tdb);
            } else {
                printf("%-12s (no event)\n", names[i]);
            }
        }
    }

    // Step 6: Cleanup
    dhruv_eop_free(eop);
    dhruv_lsk_free(lsk);
    dhruv_engine_free(engine);
    return 0;
}
```

---

## UTC Time Output

Rise/set results are returned as JD TDB. For consumers who need human-readable UTC,
provide a struct and conversion function.

### UTC Time Struct

```c
// Broken-down UTC calendar time
typedef struct {
    int32_t  year;
    uint32_t month;    // 1-12
    uint32_t day;      // 1-31
    uint32_t hour;     // 0-23
    uint32_t minute;   // 0-59
    double   second;   // 0.0-59.999...
} DhruvUtcTime;
```

### Functions

```c
// Convert a rise/set result (JD TDB) to UTC calendar components.
// Only valid when result->result_type == DHRUV_RISESET_EVENT.
// Returns InvalidQuery if result_type is NeverRises or NeverSets.
DhruvStatus dhruv_riseset_result_to_utc(
    const DhruvLskHandle*      lsk,
    const DhruvRiseSetResult*  result,
    DhruvUtcTime*              out_utc
);

// General-purpose: convert any JD TDB to UTC calendar components.
// Pure conversion, no engine needed — only uses LSK for TDB→UTC offset.
DhruvStatus dhruv_jd_tdb_to_utc(
    const DhruvLskHandle*  lsk,
    double                 jd_tdb,
    DhruvUtcTime*          out_utc
);
```

### Rust Internal

Conversion chain: `jd_tdb` → `jd_to_tdb_seconds()` → `lsk.tdb_to_utc()` →
`tdb_seconds_to_jd()` → `jd_to_calendar()` → extract h/m/s from fractional day.

```rust
fn fractional_day_to_hms(day_frac: f64) -> (u32, u32, f64) {
    let frac = day_frac.fract();
    let total_seconds = frac * 86400.0;
    let hour = (total_seconds / 3600.0).floor() as u32;
    let minute = ((total_seconds % 3600.0) / 60.0).floor() as u32;
    let second = total_seconds % 60.0;
    (hour, minute, second)
}
```

### Unit Tests

| Test | Assertion |
|------|-----------|
| `ffi_jd_tdb_to_utc_rejects_null` | null lsk or null out → NullPointer |
| `ffi_riseset_result_to_utc_never_rises` | result_type=1 → InvalidQuery |
| `ffi_riseset_result_to_utc_event` | known JD TDB → expected UTC y/m/d/h/m/s |

### Integration Tests

| Test | Assertion |
|------|-----------|
| `ffi_sunrise_utc_new_delhi` | 2024-03-20 sunrise → ~2024-03-20 00:48 UTC ± 6 min |

### C Usage

```c
DhruvRiseSetResult result;
dhruv_compute_rise_set(engine, lsk, eop, &loc,
    DHRUV_EVENT_SUNRISE, noon, &rs_config, &result);

if (result.result_type == DHRUV_RISESET_EVENT) {
    DhruvUtcTime utc;
    dhruv_riseset_result_to_utc(lsk, &result, &utc);
    printf("Sunrise: %04d-%02u-%02u %02u:%02u:%05.2f UTC\n",
           utc.year, utc.month, utc.day,
           utc.hour, utc.minute, utc.second);
}
```

---

## Verification

1. `cargo test -p dhruv_ffi_c --lib` — unit tests (null checks, defaults, pure math)
2. `cargo test -p dhruv_ffi_c --test ffi_integration` — integration tests with kernels
3. `cargo clippy --workspace` — no new warnings
4. `cargo test --workspace` — all existing 162+ tests still pass
