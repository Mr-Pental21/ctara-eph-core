# `dhruv_rs` — Rust Convenience Wrapper

## Purpose

`dhruv_rs` provides a high-level Rust API over the ctara-dhruv engine crates. It removes boilerplate by managing a global engine singleton, accepting UTC dates directly, and returning spherical coordinates.

Users only need `use dhruv_rs::*` — all core types are re-exported.

## Quick Start

```rust
use std::path::PathBuf;
use dhruv_rs::*;

// 1. Initialize once at startup
let config = EngineConfig::with_single_spk(
    PathBuf::from("kernels/data/de442s.bsp"),
    PathBuf::from("kernels/data/naif0012.tls"),
    256,
    true,
);
init(config).expect("engine init");

// 2. Query with UTC dates
let date: UtcDate = "2024-03-20T12:00:00Z".parse().unwrap();
let lon = longitude(Body::Mars, Observer::Body(Body::Earth), date).unwrap();
println!("Mars ecliptic longitude: {:.4}°", lon.to_degrees());
```

## API Reference

### Initialization

| Function | Description |
|---|---|
| `init(config: EngineConfig) -> Result<(), DhruvError>` | Initialize the global engine. Must be called once before queries. |
| `is_initialized() -> bool` | Check whether the engine has been initialized. |

### Date Input

`UtcDate` is a UTC calendar date with sub-second precision:

```rust
// Constructor
let d = UtcDate::new(2024, 3, 20, 12, 0, 0.0);

// ISO 8601 parsing (subset: YYYY-MM-DDTHH:MM:SS[.f]Z)
let d: UtcDate = "2024-03-20T12:30:45.5Z".parse().unwrap();
```

### Ephemeris Queries

| Function | Returns | Description |
|---|---|---|
| `position(target, observer, date)` | `SphericalCoords` | Ecliptic J2000 position (lon, lat, distance) |
| `position_full(target, observer, date)` | `SphericalState` | Position + angular velocities |
| `longitude(target, observer, date)` | `f64` | Ecliptic longitude in radians |
| `query(target, observer, frame, date)` | `StateVector` | Cartesian position + velocity in specified frame |
| `query_batch(requests)` | `Vec<Result<StateVector>>` | Batched queries with memoization |

### Vedic Astrological Functions

#### Ayanamsha / Nutation / Lunar Nodes

| Function | Returns | Description |
|---|---|---|
| `ayanamsha(date, system, nutation)` | `f64` | Ayanamsha in degrees |
| `nutation(date)` | `(f64, f64)` | IAU 2000B nutation (dpsi, deps) in arcsec |
| `lunar_node(date, node, mode)` | `f64` | Rahu/Ketu longitude in degrees |
| `sidereal_longitude(target, observer, date, system, nutation)` | `f64` | Sidereal longitude in degrees |

#### Rashi / Nakshatra

| Function | Returns | Description |
|---|---|---|
| `rashi(sidereal_lon)` | `RashiInfo` | Rashi from sidereal longitude |
| `nakshatra(sidereal_lon)` | `NakshatraInfo` | 27-nakshatra from sidereal longitude |
| `nakshatra28(sidereal_lon)` | `Nakshatra28Info` | 28-nakshatra from sidereal longitude |

#### Lagna / MC / Bhava

| Function | Returns | Description |
|---|---|---|
| `lagna(date, eop, location)` | `f64` | Lagna (Ascendant) in degrees |
| `mc(date, eop, location)` | `f64` | Midheaven in degrees |
| `ramc(date, eop, location)` | `f64` | Right Ascension of MC in degrees |
| `bhavas(date, eop, location, config)` | `BhavaResult` | House cusps (10 systems) |

#### Sunrise / Sunset

| Function | Returns | Description |
|---|---|---|
| `sunrise(date, eop, location)` | `RiseSetResult` | Sunrise JD |
| `sunset(date, eop, location)` | `RiseSetResult` | Sunset JD |
| `all_rise_set_events(date, eop, location)` | `Vec<RiseSetEvent>` | All twilight events |

#### Graha Positions / Longitudes

| Function | Returns | Description |
|---|---|---|
| `graha_longitudes(date, system, nutation)` | `GrahaLongitudes` | All 9 sidereal longitudes |
| `graha_positions(date, eop, location, system, nutation, config)` | `GrahaPositions` | Full graha positions with optional nakshatra/bhava/lagna |

#### Panchang (combined)

| Function | Returns | Description |
|---|---|---|
| `panchang(date, eop, location, system, nutation, calendar)` | `PanchangInfo` | Tithi, karana, yoga, vaar, hora, ghatika + optional calendar |
| `tithi(date)` | `TithiInfo` | Current tithi with start/end times |
| `karana(date)` | `KaranaInfo` | Current karana with start/end times |
| `yoga(date, system, nutation)` | `YogaInfo` | Current yoga with start/end times |
| `moon_nakshatra(date, system, nutation)` | `PanchangNakshatraInfo` | Moon's nakshatra with start/end times |
| `vaar(date, eop, location)` | `VaarInfo` | Vedic weekday with sunrise bounds |
| `hora(date, eop, location)` | `HoraInfo` | Current hora lord |
| `ghatika(date, eop, location)` | `GhatikaInfo` | Current ghatika (1-60) |
| `masa(date, system, nutation)` | `MasaInfo` | Lunar month |
| `ayana(date, system, nutation)` | `AyanaInfo` | Uttarayana/Dakshinayana |
| `varsha(date, system, nutation)` | `VarshaInfo` | 60-year samvatsara cycle |

#### Jyotish Building Blocks

| Function | Returns | Description |
|---|---|---|
| `sphutas(inputs)` | `[(Sphuta, f64); 16]` | All 16 sphutas |
| `special_lagnas(date, eop, location, system, nutation)` | `AllSpecialLagnas` | All 8 special lagnas |
| `arudha_padas(date, eop, location, system, nutation)` | `[ArudhaResult; 12]` | All 12 arudha padas |
| `ashtakavarga(date, system, nutation)` | `AshtakavargaResult` | Full BAV + SAV |
| `upagrahas(date, eop, location, system, nutation)` | `AllUpagrahas` | All 11 upagrahas |
| `core_bindus(date, eop, location, system, nutation, nak, bhava)` | `Vec<CoreBindu>` | 19 curated sensitive points |
| `drishti(date, eop, location, system, nutation, config)` | `DrishtiResult` | Graha drishti with optional bhava/lagna/bindus |

#### Search Functions

| Function | Returns | Description |
|---|---|---|
| `next_purnima(date)` | `LunarPhaseEvent` | Next full moon |
| `prev_purnima(date)` | `LunarPhaseEvent` | Previous full moon |
| `next_amavasya(date)` | `LunarPhaseEvent` | Next new moon |
| `prev_amavasya(date)` | `LunarPhaseEvent` | Previous new moon |
| `search_purnimas(start, end)` | `Vec<LunarPhaseEvent>` | Full moons in range |
| `search_amavasyas(start, end)` | `Vec<LunarPhaseEvent>` | New moons in range |
| `next_sankranti(date, system, nutation)` | `SankrantiEvent` | Next solar ingress |
| `prev_sankranti(date, system, nutation)` | `SankrantiEvent` | Previous solar ingress |
| `search_sankrantis(start, end, system, nutation)` | `Vec<SankrantiEvent>` | Solar ingresses in range |
| `next_specific_sankranti(date, rashi, system, nutation)` | `SankrantiEvent` | Next entry into specific rashi |
| `prev_specific_sankranti(date, rashi, system, nutation)` | `SankrantiEvent` | Previous entry into specific rashi |
| `next_conjunction(date, body1, body2, config)` | `ConjunctionEvent` | Next conjunction |
| `prev_conjunction(date, body1, body2, config)` | `ConjunctionEvent` | Previous conjunction |
| `search_conjunctions(start, end, body1, body2, config)` | `Vec<ConjunctionEvent>` | Conjunctions in range |
| `next_chandra_grahan(date)` | `Option<ChandraGrahan>` | Next lunar eclipse |
| `prev_chandra_grahan(date)` | `Option<ChandraGrahan>` | Previous lunar eclipse |
| `search_chandra_grahan(start, end)` | `Vec<ChandraGrahan>` | Lunar eclipses in range |
| `next_surya_grahan(date)` | `Option<SuryaGrahan>` | Next solar eclipse |
| `prev_surya_grahan(date)` | `Option<SuryaGrahan>` | Previous solar eclipse |
| `search_surya_grahan(start, end)` | `Vec<SuryaGrahan>` | Solar eclipses in range |
| `next_stationary(date, body, config)` | `StationaryEvent` | Next stationary point |
| `prev_stationary(date, body, config)` | `StationaryEvent` | Previous stationary point |
| `search_stationary(start, end, body, config)` | `Vec<StationaryEvent>` | Stationary points in range |
| `next_max_speed(date, body, config)` | `MaxSpeedEvent` | Next max-speed event |
| `prev_max_speed(date, body, config)` | `MaxSpeedEvent` | Previous max-speed event |
| `search_max_speed(start, end, body, config)` | `Vec<MaxSpeedEvent>` | Max-speed events in range |

### Individual Sphuta Formulas (pure math)

All take sidereal longitudes in degrees and return degrees. No engine required.

| Function | Formula |
|---|---|
| `bhrigu_bindu(rahu, moon)` | Midpoint of Rahu and Moon |
| `prana_sphuta(lagna, moon)` | Lagna + Moon |
| `deha_sphuta(moon, lagna)` | Moon + Lagna |
| `mrityu_sphuta(eighth_lord, lagna)` | 8th lord + Lagna |
| `tithi_sphuta(moon, sun, lagna)` | (Moon - Sun) + Lagna |
| `yoga_sphuta(sun, moon)` | Sun + Moon (raw sum) |
| `yoga_sphuta_normalized(sun, moon)` | (Sun + Moon) mod 360 |
| `rahu_tithi_sphuta(rahu, sun, lagna)` | (Rahu - Sun) + Lagna |
| `kshetra_sphuta(venus, moon, mars, jupiter, lagna)` | Venus + Moon + Mars + Jupiter + Lagna |
| `beeja_sphuta(sun, venus, jupiter)` | Sun + Venus + Jupiter |
| `trisphuta(lagna, moon, gulika)` | Lagna + Moon + Gulika |
| `chatussphuta(trisphuta_val, sun)` | TriSphuta + Sun |
| `panchasphuta(chatussphuta_val, rahu)` | ChatusSphuta + Rahu |
| `sookshma_trisphuta(lagna, moon, gulika, sun)` | Lagna + Moon + Gulika + Sun |
| `avayoga_sphuta(sun, moon)` | Avayoga formula |
| `kunda(lagna, moon, mars)` | Lagna + Moon + Mars |

### Individual Special Lagna Formulas (pure math)

All take sidereal longitudes in degrees and return degrees. No engine required.

| Function | Inputs |
|---|---|
| `bhava_lagna(sun_lon, ghatikas)` | Sun longitude + elapsed ghatikas |
| `hora_lagna(sun_lon, ghatikas)` | Sun longitude + elapsed ghatikas |
| `ghati_lagna(sun_lon, ghatikas)` | Sun longitude + elapsed ghatikas |
| `vighati_lagna(lagna_lon, vighatikas)` | Lagna longitude + elapsed vighatikas |
| `varnada_lagna(lagna_lon, hora_lagna_lon)` | Lagna + Hora Lagna longitudes |
| `sree_lagna(moon_lon, lagna_lon)` | Moon + Lagna longitudes |
| `pranapada_lagna(sun_lon, ghatikas)` | Sun longitude + elapsed ghatikas |
| `indu_lagna(moon_lon, lagna_lord, moon_9th_lord)` | Moon longitude + two Graha lords |

### Utility Primitives (pure math)

No engine required.

| Function | Returns | Description |
|---|---|---|
| `tithi_from_elongation(elongation_deg)` | `TithiPosition` | Tithi from Moon-Sun elongation |
| `karana_from_elongation(elongation_deg)` | `KaranaPosition` | Karana from Moon-Sun elongation |
| `yoga_from_sum(sum_deg)` | `YogaPosition` | Yoga from sidereal Sun+Moon sum |
| `vaar_from_jd(jd)` | `Vaar` | Weekday from Julian Date |
| `masa_from_rashi_index(rashi_index)` | `Masa` | Lunar month from rashi (0-11) |
| `ayana_from_sidereal_longitude(lon_deg)` | `Ayana` | Uttarayana/Dakshinayana from longitude |
| `samvatsara_from_year(year)` | `(Samvatsara, u8)` | 60-year cycle name + index |
| `nth_rashi_from(rashi_index, offset)` | `u8` | Rashi index N signs away |
| `rashi_lord(rashi_index)` | `Option<Graha>` | Lord of a rashi |
| `hora_at(vaar, hora_index)` | `Hora` | Hora lord at (weekday, index) |
| `ghatika_from_elapsed(secs, day_secs)` | `GhatikaPosition` | Ghatika from elapsed time |
| `normalize_360(deg)` | `f64` | Normalize angle to [0, 360) |
| `approximate_local_noon_jd(jd_midnight, lon)` | `f64` | Approximate local noon JD |
| `arudha_pada(cusp_lon, lord_lon)` | `(f64, u8)` | Single arudha pada longitude + rashi |
| `sun_based_upagrahas(sun_sid_lon)` | `SunBasedUpagrahas` | 5 sun-based upagrahas |

### Panchang Intermediates (engine required)

Building blocks for custom panchang computation. Require the global engine.

| Function | Returns | Description |
|---|---|---|
| `elongation_at(date)` | `f64` | Moon-Sun elongation in degrees |
| `sidereal_sum_at(date, system, nutation)` | `f64` | Sidereal Sun+Moon sum in degrees |
| `vedic_day_sunrises(date, eop, location)` | `(f64, f64)` | Today's + next sunrise JD |
| `body_ecliptic_lon_lat(body, date)` | `(f64, f64)` | Ecliptic longitude + latitude |
| `tithi_at(date, elongation)` | `TithiInfo` | Tithi from pre-computed elongation |
| `karana_at(date, elongation)` | `KaranaInfo` | Karana from pre-computed elongation |
| `yoga_at(date, sum, system, nutation)` | `YogaInfo` | Yoga from pre-computed sum |
| `nakshatra_at(date, moon_sid, system, nutation)` | `PanchangNakshatraInfo` | Nakshatra from pre-computed Moon longitude |
| `ghatikas_since_sunrise(date, eop, location)` | `f64` | Elapsed ghatikas since sunrise |

### Low-Level Ashtakavarga (pure math)

Array-based computation on rashi positions. No engine required.

| Function | Returns | Description |
|---|---|---|
| `calculate_ashtakavarga(graha_rashis, lagna_rashi)` | `AshtakavargaResult` | Full BAV + SAV from rashi positions |
| `calculate_bav(graha_index, graha_rashis, lagna_rashi)` | `BhinnaAshtakavarga` | Single BAV for one graha |
| `calculate_all_bav(graha_rashis, lagna_rashi)` | `[BhinnaAshtakavarga; 7]` | All 7 BAVs |
| `calculate_sav(bavs)` | `SarvaAshtakavarga` | SAV from all BAVs |
| `trikona_sodhana(totals)` | `[u8; 12]` | Trine reduction |
| `ekadhipatya_sodhana(after_trikona)` | `[u8; 12]` | Same-lord reduction |

### Low-Level Drishti (pure math)

| Function | Returns | Description |
|---|---|---|
| `graha_drishti(graha, source_lon, target_lon)` | `DrishtiEntry` | Single drishti between two points |
| `graha_drishti_matrix(longitudes)` | `GrahaDrishtiMatrix` | Full 9x9 aspect matrix |

---

## C-Only FFI Patterns (not wrapped in dhruv_rs or CLI)

The C ABI (`dhruv_ffi_c`) exports ~196 functions. Some categories exist solely to
bridge C's lack of constructors, destructors, enums-with-methods, and ownership
semantics. These are not wrapped in `dhruv_rs` or the CLI because Rust provides
the underlying features natively.

### Engine Lifecycle (9 FFI functions)

C callers must manually create, use, and destroy opaque handles:

```c
// C: manual handle lifecycle
DhruvEngineHandle* engine = NULL;
DhruvStatus s = dhruv_engine_new(&config, &engine);   // heap alloc via Box::into_raw
s = dhruv_engine_query(engine, &query, &state);        // borrow handle
dhruv_engine_free(engine);                             // manual free via Box::from_raw + drop

DhruvLskHandle* lsk = NULL;
dhruv_lsk_load("/path/naif0012.tls\0", &lsk);          // heap alloc
// ... use lsk ...
dhruv_lsk_free(lsk);                                    // manual free

DhruvEopHandle* eop = NULL;
dhruv_eop_load("/path/finals2000A.all\0", &eop);       // heap alloc
// ... use eop ...
dhruv_eop_free(eop);                                    // manual free
```

| FFI Function | Signature | Purpose |
|---|---|---|
| `dhruv_api_version()` | `-> u32` | Returns ABI version constant |
| `dhruv_engine_new(config, out)` | `(*DhruvEngineConfig, *mut *mut Handle) -> Status` | Heap-allocate engine from config |
| `dhruv_engine_query(engine, query, out)` | `(*Handle, *DhruvQuery, *mut StateVec) -> Status` | Query an existing engine handle |
| `dhruv_engine_free(engine)` | `(*mut Handle) -> Status` | Free engine (`Box::from_raw` + `drop`) |
| `dhruv_query_once(config, query, out)` | `(*Config, *Query, *mut State) -> Status` | One-shot: create + query + free |
| `dhruv_lsk_load(path, out)` | `(*u8, *mut *mut LskHandle) -> Status` | Load leap-second kernel to heap |
| `dhruv_lsk_free(lsk)` | `(*mut LskHandle) -> Status` | Free LSK handle |
| `dhruv_eop_load(path, out)` | `(*u8, *mut *mut EopHandle) -> Status` | Load EOP data to heap |
| `dhruv_eop_free(eop)` | `(*mut EopHandle) -> Status` | Free EOP handle |

**Why no Rust wrappers:**

`dhruv_rs` uses a global `OnceLock<Engine>` singleton — `init()` creates it once,
`engine()` borrows it forever, and the process owns it until exit.  The CLI uses
`Engine::new()` on the stack with automatic `Drop`.  `EopKernel::load()` is a
native Rust constructor that returns an owned value — `Drop` handles cleanup.
There is nothing to wrap because Rust's ownership model *is* the lifecycle.

```rust
// dhruv_rs: global singleton (OnceLock)
dhruv_rs::init(config)?;                    // Engine::new() + OnceLock::set()
let pos = dhruv_rs::position(body, obs, d)?; // borrows &'static Engine
// No free needed — OnceLock lives until process exit.

// CLI: stack ownership (RAII)
let engine = Engine::new(config)?;           // owned on stack
let eop = EopKernel::load(&path)?;           // owned on stack
// ... use them ...
// Dropped automatically when they go out of scope.

// Direct Rust API:
let engine = Engine::new(config)?;           // constructor
let state = engine.query(q)?;               // borrow
drop(engine);                                // explicit, or automatic at scope end
```

### Memory Management (0 dedicated FFI functions)

The C ABI uses **static strings** (`*const c_char` pointing to `&'static str`) for
name lookups and **caller-allocated** `#[repr(C)]` structs for all output values.
No heap allocation occurs on the output path, so there are no dedicated `_free`
functions for results.

The only heap operations are the handle lifecycle functions listed above
(`dhruv_engine_free`, `dhruv_lsk_free`, `dhruv_eop_free`), which exist because C
cannot express Rust ownership.

**Why no Rust wrappers:** Rust manages memory automatically via ownership and `Drop`.
There is literally nothing to wrap — there are zero memory-management FFI functions
that aren't already covered by the engine lifecycle section above.

### Name/Count Lookups (22 FFI functions)

C has no enums-with-methods, so the FFI provides index-to-name functions and
count constants for every Vedic enum:

```c
// C: must call FFI to get enum names
const char* name = dhruv_rashi_name(0);       // -> "Mesha"
uint32_t count = dhruv_rashi_count();          // -> 12
const char* graha = dhruv_graha_name(4);       // -> "Guru"
const char* nak = dhruv_nakshatra_name(0);     // -> "Ashwini"
```

| FFI Function | Returns | Description |
|---|---|---|
| `dhruv_rashi_count()` | `u32` | Number of rashis (12) |
| `dhruv_rashi_name(index)` | `*const c_char` | Rashi name by index |
| `dhruv_nakshatra_count(scheme)` | `u32` | 27 or 28 |
| `dhruv_nakshatra_name(index)` | `*const c_char` | 27-nakshatra name |
| `dhruv_nakshatra28_name(index)` | `*const c_char` | 28-nakshatra name |
| `dhruv_tithi_name(index)` | `*const c_char` | Tithi name (0-29) |
| `dhruv_karana_name(index)` | `*const c_char` | Karana name (0-10) |
| `dhruv_yoga_name(index)` | `*const c_char` | Yoga name (0-26) |
| `dhruv_vaar_name(index)` | `*const c_char` | Weekday name (0-6) |
| `dhruv_hora_name(index)` | `*const c_char` | Hora lord name (0-6) |
| `dhruv_masa_name(index)` | `*const c_char` | Lunar month name (0-11) |
| `dhruv_ayana_name(index)` | `*const c_char` | Ayana name (0-1) |
| `dhruv_samvatsara_name(index)` | `*const c_char` | Samvatsara name (0-59) |
| `dhruv_graha_name(index)` | `*const c_char` | Graha Sanskrit name (0-8) |
| `dhruv_graha_english_name(index)` | `*const c_char` | Graha English name (0-8) |
| `dhruv_sphuta_name(index)` | `*const c_char` | Sphuta name (0-15) |
| `dhruv_special_lagna_name(index)` | `*const c_char` | Special lagna name (0-7) |
| `dhruv_arudha_pada_name(index)` | `*const c_char` | Arudha pada name (0-11) |
| `dhruv_upagraha_name(index)` | `*const c_char` | Upagraha name (0-10) |
| `dhruv_ayanamsha_system_count()` | `u32` | Number of ayanamsha systems (20) |
| `dhruv_bhava_system_count()` | `u32` | Number of bhava systems (10) |
| `dhruv_lunar_node_count()` | `u32` | Number of node variants (2) |

**Why no Rust wrappers:** Rust enums have methods. Every Vedic type already
exposes `.name()` directly:

```rust
// Rust: enums have methods — no lookup functions needed
use dhruv_vedic_base::*;

ALL_RASHIS[0].name()                    // "Mesha"
ALL_RASHIS.len()                        // 12
ALL_GRAHAS[4].name()                    // "Guru"
ALL_GRAHAS[4].english_name()            // "Jupiter"
ALL_NAKSHATRAS[0].name()                // "Ashwini"
Sphuta::BhriguBindu.name()             // "Bhrigu Bindu"
SpecialLagna::BhavaLagna.name()         // "Bhava Lagna"
ArudhaPada::A1.name()                   // "A1 (Arudha Lagna)"
Upagraha::Gulika.name()                // "Gulika"

// Counts are just array lengths
ALL_RASHIS.len()                        // 12
ALL_NAKSHATRAS.len()                    // 27
ALL_NAKSHATRAS_28.len()                 // 28
AyanamshaSystem::all().len()            // 20
BhavaSystem::all().len()                // 10
```

### Config Defaults (6 FFI functions)

C has no `Default` trait, so the FFI provides explicit constructors for each
config struct:

```c
// C: explicit default constructors
DhruvRiseSetConfig rs_cfg = dhruv_riseset_config_default();
DhruvBhavaConfig bh_cfg   = dhruv_bhava_config_default();
DhruvConjunctionConfig conj_cfg = dhruv_conjunction_config_default();
```

| FFI Function | Returns | Description |
|---|---|---|
| `dhruv_riseset_config_default()` | `DhruvRiseSetConfig` | Upper limb, standard refraction |
| `dhruv_bhava_config_default()` | `DhruvBhavaConfig` | Equal house, Lagna, StartOfFirst |
| `dhruv_conjunction_config_default()` | `DhruvConjunctionConfig` | 0° target, 0.5-day step |
| `dhruv_grahan_config_default()` | `DhruvGrahanConfig` | Default eclipse search params |
| `dhruv_stationary_config_default()` | `DhruvStationaryConfig` | Inner planet defaults |
| `dhruv_sankranti_config_default()` | `DhruvSankrantiConfig` | Lahiri, no nutation |

**Why no Rust wrappers:** All config structs implement Rust's `Default` trait:

```rust
// Rust: Default trait handles this
let rs_cfg = RiseSetConfig::default();
let bh_cfg = BhavaConfig::default();
let conj_cfg = ConjunctionConfig::default();
let grahan_cfg = GrahanConfig::default();
let stat_cfg = StationaryConfig::default();
let sank_cfg = SankrantiConfig::new(system, nutation); // or SankrantiConfig::default()
```

### UTC Convenience Helpers (4 FFI functions)

The C ABI provides explicit UTC conversion and UTC-input query helpers because
C callers cannot use Rust's `UtcDate` type or the `dhruv_rs` wrappers:

| FFI Function | Description |
|---|---|
| `dhruv_utc_to_tdb_jd(engine, y, m, d, h, min, s, out)` | UTC calendar → JD TDB |
| `dhruv_jd_tdb_to_utc(engine, jd_tdb, out)` | JD TDB → UTC calendar components |
| `dhruv_riseset_result_to_utc(engine, result, out)` | Rise/set JDs → UTC structs |
| `dhruv_query_utc_spherical(engine, body, obs, frame, y, m, d, h, min, s, out)` | UTC-input spherical query |

**Why no Rust wrappers:** `dhruv_rs` already accepts `UtcDate` everywhere and
handles UTC→TDB internally. These FFI functions exist solely because C has no
equivalent of `UtcDate::to_jd_tdb()` or the high-level `position(body, obs, date)`
wrapper:

```rust
// Rust: UtcDate handles all conversions
let date: UtcDate = "2024-03-20T12:00:00Z".parse()?;
let pos = position(Body::Mars, Observer::Body(Body::Earth), date)?;  // UTC in, spherical out
let lon = sidereal_longitude(Body::Mars, AyanamshaSystem::Lahiri, date)?;
```

---

## Error Type

```rust
pub enum DhruvError {
    NotInitialized,        // init() not called
    AlreadyInitialized,    // init() called twice
    DateParse(String),     // invalid ISO 8601 string
    Engine(EngineError),   // error from dhruv_core
    Time(TimeError),       // error from dhruv_time
    Search(SearchError),   // error from dhruv_search
    Vedic(VedicError),     // error from dhruv_vedic_base
}
```

## Re-exported Types

These are re-exported from internal crates so callers only need `use dhruv_rs::*`:

**From `dhruv_core`:** `Body`, `Observer`, `Frame`, `StateVector`, `EngineConfig`

**From `dhruv_frames`:** `SphericalCoords`, `SphericalState`

**From `dhruv_time`:** `EopKernel`

**From `dhruv_vedic_base`:** `AyanamshaSystem`, `BhavaConfig`, `BhavaResult`, `BhavaSystem`, `Bhava`, `LunarNode`, `NodeMode`, `GeoLocation`, `RiseSetConfig`, `RiseSetEvent`, `RiseSetResult`, `SunLimb`, `Graha`, `Sphuta`, `SphutalInputs`, `SpecialLagna`, `AllSpecialLagnas`, `ArudhaPada`, `ArudhaResult`, `Upagraha`, `AllUpagrahas`, `AshtakavargaResult`, `BhinnaAshtakavarga`, `SarvaAshtakavarga`, `DrishtiEntry`, `GrahaDrishtiMatrix`, `SunBasedUpagrahas`, `TithiPosition`, `KaranaPosition`, `YogaPosition`, `GhatikaPosition`, `NakshatraInfo`, `Nakshatra28Info`, `RashiInfo`, `Dms`

**From `dhruv_search`:** `DrishtiConfig`, `DrishtiResult`, `GrahaLongitudes`, `LunarPhaseEvent`, `ConjunctionConfig`, `ConjunctionEvent`, `GrahanConfig`, `ChandraGrahan`, `SuryaGrahan`, `SankrantiConfig`, `SankrantiEvent`, `StationaryConfig`, `StationaryEvent`, `MaxSpeedEvent`

**Vedic name enums (re-exported with aliases):** `AyanaKind`, `HoraLord`, `KaranaName`, `MasaName`, `NakshatraName`, `Nakshatra28Name`, `Paksha`, `RashiName`, `SamvatsaraName`, `TithiName`, `VaarName`, `YogaName`

**From `dhruv_search::panchang_types`:** `PanchangInfo`, `TithiInfo`, `KaranaInfo`, `YogaInfo`, `VaarInfo`, `HoraInfo`, `GhatikaInfo`, `MasaInfo`, `AyanaInfo`, `VarshaInfo`

## Design Notes

- **Global singleton**: Uses `OnceLock<Engine>` — lock-free after initialization since `Engine` is `Send + Sync` and `query()` takes `&self`.
- **No external dependencies**: ISO 8601 parsing is hand-rolled for the supported subset.
- **Ecliptic J2000 default**: `position()`, `position_full()`, and `longitude()` always use ecliptic J2000, which is the standard frame for astrological and most astronomical longitude. Use `query()` for ICRF/J2000.
- **Batch memoization**: `query_batch()` delegates to `Engine::query_batch()`, sharing memoization across same-epoch queries.
- **Pure-math passthrough**: Sphuta formulas, special lagna formulas, utility primitives, and low-level ashtakavarga/drishti functions are thin wrappers that delegate directly to `dhruv_vedic_base` without requiring the engine.

## Module Structure

```
crates/dhruv_rs/
  Cargo.toml
  src/
    lib.rs              # module declarations, re-exports
    error.rs            # DhruvError enum
    date.rs             # UtcDate struct + FromStr
    global.rs           # OnceLock singleton
    convenience.rs      # all wrapper functions (~120)
  tests/
    integration.rs      # kernel-dependent tests
```
