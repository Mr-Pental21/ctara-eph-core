# Phase 1 Implementation Plan

## Overview

Phase 1 delivers the first real end-to-end ephemeris query: binary SPK parsing, time-scale conversion, frame transforms, chain resolution, and golden-fixture validation.

**Dependency order:**
1. `jpl_kernel` (no crate dependencies)
2. `dhruv_time` (no crate dependencies, parallel with 1)
3. `dhruv_frames` (no crate dependencies, parallel with 1 and 2)
4. End-to-end wiring in `dhruv_core` (depends on 1 + 2 + 3)
5. Golden test fixtures (depends on 4)

**Crate dependency graph after Phase 1:**
```
dhruv_core  →  jpl_kernel
          →  dhruv_time
          →  dhruv_frames
```

---

## Design Decisions for Forward Compatibility

Phase 2+ features (lunar nodes, eclipses, ayanamshas, sunrise/sunset, house systems) impose constraints on the Phase 1 foundation. The following decisions are locked in now to avoid refactoring later.

### What each future feature needs from the foundation

| Feature | Foundation requirement |
|---|---|
| **Lunar nodes** (Rahu/Ketu) | Moon position in ecliptic frame, Cartesian→Spherical to extract ecliptic lon/lat. Computed from Moon's orbital plane — NOT an SPK body. |
| **Eclipses** | Sun + Moon + Earth positions. Body radii. Root-finding (iterative query at many epochs). |
| **Ayanamshas** | Ecliptic longitude → apply precession offset. Needs Cartesian→ecliptic-spherical conversion. |
| **Sunrise/Sunset** | Sun geocentric position + Earth rotation angle (GMST/ERA) + geographic coordinates + atmospheric refraction. Root-finding over time. |
| **House systems** | Local Sidereal Time (GMST + geographic longitude), obliquity, Ascendant/MC. Geographic coordinates. |

### Decision 1: `dhruv_frames` is a real crate in Phase 1 (not deferred)

Almost every downstream feature needs ecliptic longitude/latitude. Frame transforms must live in `dhruv_frames` from day one — not inline in `dhruv_core`.

`dhruv_frames` provides:
- ICRF ↔ Ecliptic J2000 rotation
- Cartesian ↔ Spherical conversion (longitude, latitude, distance)
- J2000 obliquity constant (and later: obliquity-of-date polynomial)

`dhruv_core` delegates all frame math to `dhruv_frames`.

### Decision 2: `dhruv_time` module structure anticipates sidereal time

Sunrise/sunset and house systems need GMST/ERA. Phase 1 implements UTC→TDB only, but the module layout includes a `sidereal.rs` stub so the API surface is ready.

### Decision 3: Lunar nodes are NOT in the `Body` enum

They are computed points, not SPK targets. They belong in `dhruv_vedic_base` as functions that take Moon/Earth state vectors and return node positions. The existing `DerivedComputation` trait is the extension seam.

The `Body` enum stays SPK-only: physical bodies that exist as segments in the kernel file.

### Decision 4: `Observer` enum stays body-center only

Topocentric observers (geographic lat/lon/alt for sunrise/sunset and houses) are a higher-level concept. The core engine computes geocentric state vectors; topocentric correction is layered on top by calling `Engine::query(observer: Body(Earth))` and then applying Earth rotation + geographic offset.

This matches how SPICE works: SPK gives body-center positions, a separate step applies Earth orientation.

### Decision 5: `DerivedValue` will likely grow

The current `DerivedValue` enum (`Scalar(f64)` / `Vector3([f64; 3])`) is sufficient for Phase 1. Future features may need richer return types (e.g., eclipse geometry with type + magnitude + contact times). This will be addressed when needed — the trait-based extension seam allows backward-compatible evolution.

---

## 1. SPK/DAF Binary Parsing in `jpl_kernel`

### What is DAF?

DAF (Double precision Array File) is NASA/NAIF's binary container format. An SPK file **is** a DAF file — DAF is the container, SPK is the data type stored inside.

### Binary Layout

Every DAF file is composed of **1024-byte records** (128 doubles). The first record is the **file record**:

```
Bytes 0-7:     LOCIDW  — file ID word, e.g. "DAF/SPK " (8 chars, space-padded)
Bytes 8-11:    ND      — number of double components per summary (i32)
Bytes 12-15:   NI      — number of integer components per summary (i32)
Bytes 16-75:   LOCIFN  — internal file name (60 chars)
Bytes 76-79:   FWARD   — record number of first summary record (i32)
Bytes 80-83:   BWARD   — record number of last summary record (i32)
Bytes 84-87:   FREE    — first free address in file (i32)
Bytes 88-95:   LOCFMT  — endianness: "LTL-IEEE" or "BIG-IEEE" (8 chars)
Bytes 96-699:  PRENUL  — zero-fill
Bytes 700-703: FTPSTR  — FTP corruption test string
Bytes 704-1023: zero-fill
```

**Endianness detection**: Read LOCFMT at bytes 88-95. If `"LTL-IEEE"`, the file is little-endian. If `"BIG-IEEE"`, big-endian. The parser must handle both.

For SPK files: **ND = 2, NI = 6**. Summary size SS = `ND + (NI+1)/2 = 2 + 3 = 5` doubles (40 bytes).

### Summary Records (Segment Index)

Summary records form a **doubly-linked list** starting from FWARD. Each summary record:

```
Double 0: NEXT  — record number of next summary record (0.0 if last)
Double 1: PREV  — record number of previous summary record (0.0 if first)
Double 2: NSUM  — number of summaries in this record
Doubles 3..3+NSUM*SS: the summaries, packed contiguously
```

Each SPK summary (40 bytes = 5 doubles):

```
Double 0: start_epoch     — segment start time (TDB seconds past J2000)
Double 1: end_epoch       — segment end time (TDB seconds past J2000)
Integer 0 (i32): target   — NAIF body code of target
Integer 1 (i32): center   — NAIF body code of center
Integer 2 (i32): frame    — reference frame code (1 = J2000/ICRF)
Integer 3 (i32): data_type — SPK data type (2 = Chebyshev position)
Integer 4 (i32): start_addr — first word address of segment data (1-based)
Integer 5 (i32): end_addr   — last word address of segment data (1-based)
```

The 6 integers are packed into 3 doubles — reinterpret the bytes of doubles 2-4 as pairs of i32:

```rust
let d2_bytes = &bytes[offset+16..offset+24];
let target     = i32::from_le_bytes(d2_bytes[0..4]);
let center     = i32::from_le_bytes(d2_bytes[4..8]);
// ... etc for d3, d4
```

### SPK Type 2: Chebyshev Position-Only

DE442 uses **only Type 2** segments. Each segment is divided into fixed-length records covering equal time intervals. At the very end of the segment data, a **descriptor** (last 4 doubles):

```
INIT    — start epoch of first record interval (TDB seconds past J2000)
INTLEN  — length of each interval in seconds
RSIZE   — number of doubles per record (including MID and RADIUS)
N       — number of records in the segment
```

Derived: `n_coeffs = (RSIZE - 2) / 3` — the number of Chebyshev coefficients per axis.

Each record layout:

```
Double 0: MID     — midpoint time of this interval (TDB seconds past J2000)
Double 1: RADIUS  — half-span of this interval in seconds
Doubles 2..2+n_coeffs:             X Chebyshev coefficients [c0, c1, ..., c_{n-1}]
Doubles 2+n_coeffs..2+2*n_coeffs:  Y Chebyshev coefficients
Doubles 2+2*n_coeffs..2+3*n_coeffs: Z Chebyshev coefficients
```

### Record Lookup

Given epoch `t` (TDB seconds past J2000) within a segment's time range:

```
record_index = min(floor((t - INIT) / INTLEN), N - 1)
```

Clamp to `[0, N-1]`. Record data byte offset:

```
record_byte_offset = (start_addr - 1) * 8 + record_index * RSIZE * 8
```

(NAIF word addresses are 1-based, each word = 8 bytes.)

### Chebyshev Evaluation: Clenshaw Recurrence

Given coefficients `[c0, c1, ..., c_{n-1}]` and normalized time `s = (t - MID) / RADIUS` where `s in [-1, 1]`:

**Position** (Clenshaw backward recurrence):

```
b_{n+1} = 0
b_n = 0
for k = n-1 down to 1:
    b_k = 2 * s * b_{k+1} - b_{k+2} + c_k
position = s * b_1 - b_2 + c_0
```

**Velocity** (derivative via forward T_k' recurrence):

```
T_0'(s) = 0,  T_1'(s) = 1
T_k'(s) = 2*T_{k-1}(s) + 2*s*T_{k-1}'(s) - T_{k-2}'(s)

vel_sum = sum(c_k * T_k'(s)) for k=1..n-1
velocity = vel_sum / RADIUS    (km/s)
```

### File Reading Strategy

`unsafe_code = "forbid"` prevents memmap2. Use `std::fs::read()` → `Vec<u8>`. de442s.bsp is ~31MB — acceptable for MVP.

### Module Structure

```
jpl_kernel/src/
├── lib.rs          — public API: SpkKernel, SpkEvaluation
├── error.rs        — KernelError enum
├── daf.rs          — DAF file record, summary record parsing
├── spk.rs          — SPK segment descriptor, Type 2 record layout
└── chebyshev.rs    — Clenshaw position eval, derivative velocity eval
```

### Public API

```rust
pub struct SpkKernel { /* loaded segments, raw data */ }

pub struct SpkSegment {
    pub target: i32,
    pub center: i32,
    pub frame: i32,
    pub data_type: i32,
    pub start_epoch: f64,
    pub end_epoch: f64,
}

pub struct SpkEvaluation {
    pub position_km: [f64; 3],
    pub velocity_km_s: [f64; 3],
}

impl SpkKernel {
    pub fn load(path: &Path) -> Result<Self, KernelError>;
    pub fn segments(&self) -> &[SpkSegment];
    pub fn evaluate(
        &self, target: i32, center: i32, epoch_tdb_s: f64
    ) -> Result<SpkEvaluation, KernelError>;
}
```

`evaluate()` takes **TDB seconds past J2000** (not JD). Conversion from JD is the caller's responsibility.

---

## 2. UTC->TDB Conversion in `dhruv_time`

### The Time Scale Chain

```
UTC  →(+leap seconds)→  TAI  →(+32.184s)→  TT  →(periodic formula)→  TDB
```

### Step 1: UTC -> TAI (Leap Seconds)

TAI = UTC + delta_AT, where delta_AT is the cumulative leap seconds at that UTC epoch. The leap second table comes from `naif0012.tls`.

The LSK file is a NAIF text kernel:

```
\begindata
DELTET/DELTA_T_A = 32.184
DELTET/K         = 1.657D-3
DELTET/EB        = 1.671D-2
DELTET/M         = ( 6.239996  1.99096871D-7 )
DELTET/DELTA_AT  = ( 10,   @1972-JAN-1
                     11,   @1972-JUL-1
                     ...
                     37,   @2017-JAN-1 )
```

**LSK parsing steps:**
1. Scan for `\begindata` marker
2. Parse variable assignments between `\begindata` and `\begintext`/EOF
3. Handle `D` exponent notation (`1.657D-3` → `1.657e-3`)
4. Handle `@YYYY-MON-DD` date literals → seconds past J2000 via calendar-to-JD
5. Build leap second table as `Vec<(f64, f64)>` — (delta_AT, epoch)

**Lookup**: Binary search sorted table. Last entry where `epoch <= utc_s`.

### Step 2: TAI -> TT

`TT = TAI + 32.184` seconds (exact by IAU definition, the `DELTA_T_A` value from LSK).

### Step 3: TT -> TDB

```
M = M0 + M1 * tt_seconds_past_J2000
E = M + EB * sin(M)
TDB = TT + K * sin(E)
```

Constants from LSK:
- K = 0.001657 s (amplitude)
- EB = 0.01671 (Earth orbital eccentricity)
- M0 = 6.239996 rad (mean anomaly at J2000)
- M1 = 1.99096871e-7 rad/s (mean anomaly rate)

Maximum correction: ~1.66 ms. Accuracy vs full relativistic treatment: ~30 us.

TT is used as proxy for TDB in computing M — the circular dependency is negligible (~30us error).

### Calendar-to-JD Algorithm

```
If month <= 2: y = year - 1, m = month + 12
Else: y = year, m = month

a = floor(y / 100)
b = 2 - a + floor(a / 4)

JD = floor(365.25 * (y + 4716))
   + floor(30.6001 * (m + 1))
   + day + b - 1524.5
```

Where `day` can be fractional.

### Internal Representation

`f64` seconds past J2000.0 TDB. Rationale:
- JD: only ~86us precision (large number + small fraction)
- Seconds past J2000: ~0.7us precision at 100-year offsets
- Matches NAIF/SPICE convention and SPK epoch format

### Module Structure

```
dhruv_time/src/
├── lib.rs          — public API: Epoch, LeapSecondKernel
├── lsk.rs          — LSK file parser
├── julian.rs       — calendar_to_jd, jd_to_calendar
├── scales.rs       — UTC->TAI->TT->TDB conversion functions
└── sidereal.rs     — GMST/ERA (stub in Phase 1, implemented in Phase 2)
```

Note: `sidereal.rs` is a stub in Phase 1. It will be implemented in Phase 2 for sunrise/sunset and house systems. The module is created now so the crate's public API surface is stable.

### Public API

```rust
pub struct Epoch {
    tdb_seconds: f64,  // TDB seconds past J2000.0
}

impl Epoch {
    pub fn from_jd_tdb(jd: f64) -> Self;
    pub fn from_utc(y: i32, mo: u32, d: u32, h: u32, mi: u32, s: f64, lsk: &LeapSecondKernel) -> Self;
    pub fn as_tdb_seconds(&self) -> f64;
    pub fn as_jd_tdb(&self) -> f64;
}

pub struct LeapSecondKernel { /* parsed LSK data */ }

impl LeapSecondKernel {
    pub fn load(path: &Path) -> Result<Self, TimeError>;
}
```

---

## 3. Frame Transforms in `dhruv_frames`

### Why this is a Phase 1 deliverable

Ecliptic longitude/latitude extraction is needed by nearly every downstream feature:
- **Ayanamsha**: ecliptic longitude + precession offset
- **Lunar nodes**: ecliptic latitude zero-crossing of Moon's orbit
- **House systems**: ecliptic longitude of Ascendant/MC
- **Vedic charts**: all planetary positions expressed in sidereal ecliptic longitude

Putting frame math in a standalone crate ensures `dhruv_core`, `dhruv_vedic_base`, and `dhruv_pro` all share the same rotation/conversion code without circular dependencies.

### Module Structure

```
dhruv_frames/src/
├── lib.rs          — re-exports
├── rotation.rs     — ICRF ↔ Ecliptic J2000 rotation
├── spherical.rs    — Cartesian ↔ Spherical (lon, lat, distance)
└── obliquity.rs    — J2000 obliquity constant, later: obliquity of date
```

### ICRF ↔ Ecliptic Rotation

Obliquity at J2000: epsilon = 23.4392911111 deg = 0.4090928042223 rad (IAU 1976 value used by DE kernels).

```
R_icrf_to_ecl = | 1    0          0         |
                | 0    cos(eps)   sin(eps)  |
                | 0   -sin(eps)   cos(eps)  |

R_ecl_to_icrf = transpose(R_icrf_to_ecl)
```

Apply to both position and velocity 3-vectors.

```rust
/// Rotate a 3-vector from ICRF/J2000 to Ecliptic J2000.
pub fn icrf_to_ecliptic(v: &[f64; 3]) -> [f64; 3];

/// Rotate a 3-vector from Ecliptic J2000 to ICRF/J2000.
pub fn ecliptic_to_icrf(v: &[f64; 3]) -> [f64; 3];
```

### Cartesian ↔ Spherical

This is the critical conversion for all downstream astrology/astronomy features.

```rust
pub struct SphericalCoords {
    /// Longitude in degrees [0, 360)
    pub lon_deg: f64,
    /// Latitude in degrees [-90, 90]
    pub lat_deg: f64,
    /// Distance in km
    pub distance_km: f64,
}

/// Convert Cartesian [x, y, z] to spherical (longitude, latitude, distance).
/// Longitude is measured in the x-y plane from +x toward +y.
/// Latitude is elevation from the x-y plane.
pub fn cartesian_to_spherical(xyz: &[f64; 3]) -> SphericalCoords;

/// Convert spherical back to Cartesian.
pub fn spherical_to_cartesian(s: &SphericalCoords) -> [f64; 3];
```

**Usage pattern for ecliptic longitude** (the most common downstream need):
```rust
use dhruv_frames::{icrf_to_ecliptic, cartesian_to_spherical};

let ecl_pos = icrf_to_ecliptic(&state.position_km);
let spherical = cartesian_to_spherical(&ecl_pos);
let ecliptic_longitude_deg = spherical.lon_deg;
// Apply ayanamsha for sidereal longitude:
// let sidereal_lon = ecliptic_longitude_deg - ayanamsha_deg;
```

### Obliquity

```rust
/// Mean obliquity of the ecliptic at J2000.0 (IAU 1976), in radians.
pub const OBLIQUITY_J2000_RAD: f64 = 0.4090928042223;

/// Mean obliquity of the ecliptic at J2000.0, in degrees.
pub const OBLIQUITY_J2000_DEG: f64 = 23.4392911111;

// Phase 2: obliquity_of_date(t_centuries_j2000) -> f64
// Uses the IAU 2006 polynomial for precession-corrected obliquity.
```

---

## 4. End-to-End Query Wiring in `dhruv_core`

### The Query Pipeline

```
1. Convert epoch (JD TDB) → TDB seconds past J2000
2. Resolve chain: target → SSB, observer → SSB
3. For each link, evaluate the SPK segment
4. Accumulate vectors: state_target_ssb, state_observer_ssb
5. Subtract: result = target_ssb - observer_ssb
6. If frame != ICRF, apply rotation via dhruv_frames
7. Return StateVector
```

### Chain Resolution: Body Hierarchy

DE442s stores positions in a tree rooted at SSB (0):

```
SSB (0)
├── Mercury Barycenter (1) ← Mercury (199)
├── Venus Barycenter (2) ← Venus (299)
├── Earth-Moon Barycenter (3)
│   ├── Earth (399)
│   └── Moon (301)
├── Mars Barycenter (4) ← Mars (499)
├── Jupiter Barycenter (5) ← Jupiter (599)
├── Saturn Barycenter (6) ← Saturn (699)
├── Uranus Barycenter (7) ← Uranus (799)
├── Neptune Barycenter (8) ← Neptune (899)
├── Pluto Barycenter (9) ← Pluto (999)
└── Sun (10)
```

Each SPK segment stores a `(target, center)` pair. Build a lookup table at load time from the segment descriptors.

**Resolution algorithm:**

```rust
fn resolve_to_ssb(body_code: i32, kernel: &SpkKernel, epoch: f64) -> [f64; 6] {
    let mut code = body_code;
    let mut accumulated = [0.0; 6];  // [x, y, z, vx, vy, vz]
    while code != 0 {  // 0 = SSB
        let eval = kernel.evaluate(code, /* center from segment */, epoch)?;
        // Add position and velocity components
        accumulated[0..3] += eval.position_km;
        accumulated[3..6] += eval.velocity_km_s;
        code = segment.center;  // move up the tree
    }
    accumulated
}
```

**Example — Mars relative to Earth:**

```
Mars (499) → Mars Bary (4) → SSB (0)
  mars_ssb = seg(499→4) + seg(4→0)

Earth (399) → EMB (3) → SSB (0)
  earth_ssb = seg(399→3) + seg(3→0)

result = mars_ssb - earth_ssb
```

### Engine Changes

```rust
use jpl_kernel::SpkKernel;
use dhruv_time::LeapSecondKernel;
use dhruv_frames;

pub struct Engine {
    config: EngineConfig,
    spk: SpkKernel,
    lsk: LeapSecondKernel,
}

impl Engine {
    pub fn new(config: EngineConfig) -> Result<Self, EngineError> {
        config.validate()?;
        let spk = SpkKernel::load(&config.spk_path)
            .map_err(|e| EngineError::KernelLoad(e.to_string()))?;
        let lsk = LeapSecondKernel::load(&config.lsk_path)
            .map_err(|e| EngineError::KernelLoad(e.to_string()))?;
        Ok(Self { config, spk, lsk })
    }

    pub fn query(&self, query: Query) -> Result<StateVector, EngineError> {
        // ... validation ...
        let epoch_tdb_s = (query.epoch_tdb_jd - 2_451_545.0) * 86_400.0;
        let target_ssb = self.resolve_to_ssb(query.target.code(), epoch_tdb_s)?;
        let observer_ssb = match query.observer {
            Observer::SolarSystemBarycenter => [0.0; 6],
            Observer::Body(body) => self.resolve_to_ssb(body.code(), epoch_tdb_s)?,
        };
        let mut state = StateVector {
            position_km: [
                target_ssb[0] - observer_ssb[0],
                target_ssb[1] - observer_ssb[1],
                target_ssb[2] - observer_ssb[2],
            ],
            velocity_km_s: [
                target_ssb[3] - observer_ssb[3],
                target_ssb[4] - observer_ssb[4],
                target_ssb[5] - observer_ssb[5],
            ],
        };
        // Frame rotation delegated to dhruv_frames
        if query.frame == Frame::EclipticJ2000 {
            state.position_km = dhruv_frames::icrf_to_ecliptic(&state.position_km);
            state.velocity_km_s = dhruv_frames::icrf_to_ecliptic(&state.velocity_km_s);
        }
        Ok(state)
    }
}
```

### Caching

Not needed for MVP. de442s has ~15 segments, evaluation is O(n_coefficients) — microsecond-scale. Optional later: cache most recently evaluated segment + record.

---

## 5. Golden Test Fixtures from JPL Horizons

### Purpose

1. **Same-kernel validation**: Chebyshev evaluator vs CSPICE with same DE442s (tight: <1e-10 km)
2. **Cross-kernel sanity**: Engine vs Horizons/DE441 (loose: <0.01 km inner, <0.1 km outer)

### Horizons API

REST endpoint: `https://ssd.jpl.nasa.gov/api/horizons.api`

Key parameters:

| Parameter | Value | Purpose |
|---|---|---|
| `COMMAND` | `'499'` | Target NAIF ID |
| `CENTER` | `'500@399'` | Observer body center |
| `EPHEM_TYPE` | `'VECTORS'` | State vectors |
| `REF_FRAME` | `'ICRF'` | Reference frame |
| `VEC_TABLE` | `'2'` | Position + velocity |
| `OUT_UNITS` | `'KM-S'` | km and km/s |
| `CSV_FORMAT` | `'YES'` | Machine-readable |
| `TIME_TYPE` | `'TDB'` | TDB time scale |
| `TLIST` | `'2451545.0,2460000.5'` | Specific JD TDB epochs |

**Important**: Horizons uses DE441, not DE442. Meter-level differences are expected.

### Epoch Selection

| Epoch JD TDB | Calendar TDB | Rationale |
|---|---|---|
| 2451545.0 | 2000-Jan-01 12:00 | J2000.0 reference |
| 2460000.5 | 2023-Feb-25 12:00 | Mid-range, matches bootstrap |
| 2458849.5 | 2020-Jan-01 12:00 | Recent historical |
| 2460676.5 | 2024-Dec-01 12:00 | Near-current |
| 2440423.5 | 1969-Jul-20 12:00 | Historical (Apollo era) |
| 2451545.00069 | J2000 + ~1 min | Sub-day precision test |
| 2451544.5 | 2000-Jan-01 00:00 | Midnight boundary |

### Test Matrix

- **Bodies**: All 11 from `Body` enum
- **Observers**: SSB (0), Sun (10), Earth (399)
- **Total**: ~231 test cases (minus self-referencing)

### Fixture File Format

JSON in `testdata/horizons_golden/`, one per observer center:

```json
{
  "metadata": {
    "source": "JPL Horizons REST API",
    "horizons_de_kernel": "DE441",
    "comparison_kernel": "DE442s",
    "reference_frame": "ICRF/J2000.0",
    "units": { "position": "km", "velocity": "km/s", "epoch": "JD TDB" },
    "generated_utc": "2025-01-15T10:30:00Z"
  },
  "tolerances": {
    "same_kernel":        { "position_km": 1e-10,  "velocity_km_s": 1e-13 },
    "cross_kernel":       { "position_km": 0.01,   "velocity_km_s": 1e-5 },
    "cross_kernel_outer": { "position_km": 0.1,    "velocity_km_s": 1e-4 }
  },
  "test_cases": [
    {
      "name": "mars_earth_icrf_2460000.5",
      "target": { "name": "Mars", "naif_id": 499 },
      "center": { "name": "Earth", "naif_id": 399 },
      "frame": "IcrfJ2000",
      "epoch": { "jd_tdb": 2460000.5 },
      "state": {
        "x_km": -1.452003247363175e+08,
        "y_km":  1.212809701823288e+07,
        "z_km":  6.861975338506435e+06,
        "vx_km_s": -5.840598884668782e+00,
        "vy_km_s": -3.018044457094731e+01,
        "vz_km_s": -1.309152984861156e+01
      },
      "tolerance_class": "cross_kernel"
    }
  ]
}
```

### Tolerance Budget

| Comparison Type | Position | Velocity | When |
|---|---|---|---|
| Same-kernel (DE442s) | 1e-10 km (0.1 nm) | 1e-13 km/s | CSPICE with same .bsp |
| Cross-kernel inner | 0.01 km (10 m) | 1e-5 km/s | Horizons, inner planets |
| Cross-kernel outer | 0.1 km (100 m) | 1e-4 km/s | Horizons, outer planets |

Always use **absolute per-component** tolerance (not relative) — components pass through zero during orbital motion.

### Automation Script

Python script at `scripts/golden/fetch_horizons_vectors.py`:
1. Iterate body x observer x epoch matrix
2. Call Horizons API with `TLIST` (batch epochs per request)
3. Parse `$$SOE`/`$$EOE`-delimited CSV
4. Write JSON fixture files
5. Rate-limit ~2 req/s

Fixture files are committed. Tests never call Horizons at runtime.

### Integration Test

```rust
// crates/dhruv_core/tests/golden_horizons.rs
#[test]
fn golden_horizons_ssb() {
    let fixtures = load_fixtures("de441_ssb_icrf.json");
    let engine = Engine::new(config_with_real_kernels())?;
    for tc in &fixtures.test_cases {
        let result = engine.query(build_query(tc))?;
        assert_state_within_tolerance(&tc.name, &result, &tc.state, tol);
    }
}
```

Gate with `#[ignore]` when kernel files aren't present.

---

## Implementation Sequence

```
Week 1:  jpl_kernel  (DAF parsing + SPK Type 2 + Chebyshev eval)
         dhruv_time    (LSK parsing + UTC->TDB chain + sidereal stub)  [parallel]
         dhruv_frames  (rotation + spherical + obliquity)              [parallel]
Week 2:  dhruv_core    (chain resolution + Engine wiring, depends on week 1)
         Golden fixtures (Horizons fetch + JSON + integration tests)
```

---

## Appendix: Phase 2+ Feature Routing

This table shows where each future feature will be implemented, using Phase 1 foundations:

| Feature | Crate | Uses from Phase 1 |
|---|---|---|
| Lunar nodes (Rahu/Ketu) | `dhruv_vedic_base` | `Engine::query(Moon)` + `dhruv_frames::cartesian_to_spherical` |
| Ayanamsha (Lahiri, etc.) | `dhruv_vedic_base` | `dhruv_frames::cartesian_to_spherical` for ecliptic lon |
| Eclipses | `dhruv_core` or `dhruv_vedic_base` | `Engine::query()` for Sun/Moon/Earth positions |
| Sunrise/Sunset | `dhruv_vedic_base` or new crate | `Engine::query(Sun, observer: Earth)` + `dhruv_time::sidereal` (Phase 2) |
| House systems | `dhruv_vedic_base` | `dhruv_time::sidereal` + `dhruv_frames::obliquity` |
| Topocentric observer | New layer above `dhruv_core` | `Engine::query(observer: Earth)` + Earth rotation |
