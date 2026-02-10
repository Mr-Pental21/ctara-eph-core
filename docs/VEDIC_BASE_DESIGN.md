# Vedic Base Design: Ayanamsha + Sunrise/Sunset

## Overview

Add two feature subsystems to the `dhruv_vedic_base` crate:

1. **Ayanamsha** — sidereal zodiac offset for 20+ reference systems
2. **Sunrise/Sunset** — geometric + atmospheric refraction, with twilight variants

Both are derived computations layered on top of the existing ephemeris engine.
Three prerequisites must be added to existing crates first: IERS EOP parsing
for proper UT1 conversion, IAU 2006 precession polynomial, and GMST/ERA.

---

## Prerequisites

### A. IAU 2006 General Precession Polynomial

**Crate**: `dhruv_frames`
**New file**: `crates/dhruv_frames/src/precession.rs`
**Edit**: `crates/dhruv_frames/src/lib.rs` — add `pub mod precession;` and re-exports

The general precession in longitude (p_A) measures the accumulated
westward motion of the vernal equinox along the ecliptic since J2000.0.
This is the foundational formula for all ayanamsha calculations.

**Source**: Capitaine, Wallace & Chapront 2003, _Astronomy & Astrophysics_
412, 567-586. Coefficients also published in IERS Conventions 2010, Chapter 5.
Public domain (IAU standard).

**Polynomial** (T = Julian centuries of TDB since J2000.0):

```
p_A(T) = 5028.796195″·T
       +    1.1054348″·T²
       +    0.00007964″·T³
       -    0.000023857″·T⁴
       -    0.0000000383″·T⁵
```

Returns arcseconds. The dominant linear term gives ~1.3969°/century.

**Functions**:

```rust
/// IAU 2006 general precession in ecliptic longitude, in arcseconds.
///
/// T = Julian centuries of TDB since J2000.0.
/// Source: Capitaine et al. 2003 Table 1 / IERS Conventions 2010 Ch. 5.
pub fn general_precession_longitude_arcsec(t: f64) -> f64

/// Same as above, in degrees.
pub fn general_precession_longitude_deg(t: f64) -> f64
```

**Unit tests** (in `src/precession.rs`, no kernel dependency):

| Test | Expected |
|------|----------|
| `p_a_at_j2000_is_zero` | `p_A(0.0) == 0.0` |
| `p_a_one_century_approx` | `p_A(1.0) ≈ 5028.80″` (within 1″) |
| `p_a_negative_century` | `p_A(-1.0) < 0` |
| `p_a_rate_per_year` | `p_A(0.01) ≈ 50.29″` (1 year) |

---

### B. IERS Earth Orientation Parameters (UT1−UTC)

**Crate**: `dhruv_time`
**New file**: `crates/dhruv_time/src/eop.rs`
**Edit**: `crates/dhruv_time/src/lib.rs` — add `pub mod eop;` and re-exports
**Edit**: `crates/dhruv_time/src/error.rs` — add `EopParse(String)`, `EopOutOfRange` variants

GMST requires UT1 (not UTC). We load IERS Earth Orientation Parameters from
the standard finals2000A.all file to obtain the DUT1 = UT1−UTC correction.
This follows the same file-loading pattern as `LeapSecondKernel`.

**Data file**: IERS finals2000A.all
- Fixed-width text, ~3MB, public domain (government/intergovernmental data)
- Available from https://datacenter.iers.org/ and https://maia.usno.navy.mil/ser7/
- Updated weekly by IERS Rapid Service/Prediction Center
- Contains daily EOP values from 1973-Jan-02 onward + ~1 year of predictions
- User downloads and places alongside other kernel files (de442s.bsp, naif0012.tls)

**File format** (relevant columns, 1-indexed):

```
Col  1-2:   Year (I2, last two digits)
Col  3-4:   Month (I2)
Col  5-6:   Day (I2)
Col  8-15:  Modified Julian Date (F8.2, MJD = JD − 2400000.5)
Col 58:     Flag — 'I' (IERS final) or 'P' (prediction)
Col 59-68:  UT1−UTC in seconds (F10.7)
```

**Data structures**:

```rust
/// Parsed IERS Earth Orientation Parameters (DUT1 lookup table).
pub struct EopData {
    /// Daily (MJD, DUT1) pairs, sorted ascending by MJD.
    entries: Vec<(f64, f64)>,  // (mjd, dut1_seconds)
}

/// Loaded EOP file, ready for UT1 conversions.
/// Follows the same pattern as LeapSecondKernel.
pub struct EopKernel {
    data: EopData,
}
```

**Functions**:

```rust
impl EopData {
    /// Parse IERS finals2000A.all fixed-width format.
    /// Extracts MJD (col 8-15) and DUT1 (col 59-68) from each line.
    /// Skips lines with blank or unparseable DUT1 fields.
    pub fn parse_finals(content: &str) -> Result<Self, TimeError>

    /// DUT1 (UT1−UTC) in seconds at a given MJD, linearly interpolated
    /// between adjacent daily values.
    pub fn dut1_at_mjd(&self, mjd: f64) -> Result<f64, TimeError>

    /// Convert UTC Julian Date to UT1 Julian Date.
    /// jd_ut1 = jd_utc + dut1 / 86400.0
    pub fn utc_to_ut1_jd(&self, jd_utc: f64) -> Result<f64, TimeError>

    /// MJD range covered by the table: (first, last).
    pub fn range(&self) -> (f64, f64)
}

impl EopKernel {
    /// Load finals2000A.all from disk.
    pub fn load(path: &Path) -> Result<Self, TimeError>
    /// Parse from string content.
    pub fn parse(content: &str) -> Result<Self, TimeError>
    /// Access parsed data.
    pub fn data(&self) -> &EopData
    /// Convenience: UTC JD → UT1 JD.
    pub fn utc_to_ut1_jd(&self, jd_utc: f64) -> Result<f64, TimeError>
}
```

**Edge cases**:
- Date before table start → `TimeError::EopOutOfRange`
- Date after last entry → `TimeError::EopOutOfRange`
- Predictions (flag 'P') are included and usable (less accurate, but valid)
- Lines with blank DUT1 field (col 59-68 all spaces) → skipped during parse
- Malformed lines → skipped (don't abort entire parse for one bad line)

**Unit tests** (in `src/eop.rs`, inline test data — no external file):

| Test | Expected |
|------|----------|
| `parse_small_snippet` | Hand-crafted 3-line finals excerpt parses correctly |
| `interpolation_midpoint` | DUT1 at MJD halfway between two entries = average |
| `interpolation_exact` | DUT1 at exact entry MJD = that entry's value |
| `out_of_range_before` | MJD before table start → `EopOutOfRange` |
| `out_of_range_after` | MJD after table end → `EopOutOfRange` |
| `blank_dut1_skipped` | Lines with empty col 59-68 produce no entries |
| `utc_to_ut1_offset` | `jd_ut1 − jd_utc` equals `dut1 / 86400` |

**Integration tests** (in `tests/eop_integration.rs`, skip gracefully if file absent):

| Test | Expected |
|------|----------|
| `load_real_file` | `EopKernel::load()` succeeds, entries > 10000 |
| `dut1_within_bounds` | All DUT1 values satisfy \|DUT1\| < 0.9s |
| `known_date_value` | DUT1 at specific date matches IERS Bulletin B |

---

### C. Greenwich Mean Sidereal Time / Earth Rotation Angle

**Crate**: `dhruv_time`
**Edit**: `crates/dhruv_time/src/sidereal.rs` (replace existing stub)
**Edit**: `crates/dhruv_time/src/lib.rs` — add re-exports

Needed for sunrise/sunset: convert Sun's right ascension + observer longitude
to local hour angle. The chain is: EOP(UTC→UT1) → ERA → GMST → LST → hour angle.

**Source**: IERS Conventions 2010, Eq. 5.15 (ERA) and Capitaine et al. 2003
Table 2 (GMST–ERA polynomial). Public domain (IAU standard).

**UT1 handling**: All functions take `jd_ut1` (not `jd_utc`). Callers convert
UTC→UT1 using `EopKernel::utc_to_ut1_jd()` before calling GMST. This mirrors
the existing pattern where callers convert UTC→TDB using `LeapSecondKernel`.

**Formulas**:

Earth Rotation Angle (radians):
```
θ(JD_UT1) = 2π × (0.7790572732640 + 1.00273781191135448 × Du)
Du = JD_UT1 − 2451545.0
Result normalized to [0, 2π)
```

GMST from ERA (radians):
```
GMST = θ + polynomial(T)
polynomial(T) = (0.014506″ + 4612.156534″·T + 1.3915817″·T²
                − 0.00000044″·T³ − 0.000029956″·T⁴
                − 0.0000000368″·T⁵) × (π / 648000)
T = (JD_UT1 − 2451545.0) / 36525.0
```

**Functions**:

```rust
/// Earth Rotation Angle at a given UT1 Julian Date.
/// Source: IERS Conventions 2010, Eq. 5.15.
pub fn earth_rotation_angle_rad(jd_ut1: f64) -> f64

/// Greenwich Mean Sidereal Time at a given UT1 Julian Date.
/// Source: Capitaine et al. 2003 Table 2.
pub fn gmst_rad(jd_ut1: f64) -> f64

/// Local Sidereal Time = GMST + east longitude.
pub fn local_sidereal_time_rad(gmst: f64, longitude_east_rad: f64) -> f64
```

**Unit tests** (in `src/sidereal.rs`, no kernel dependency):

| Test | Expected |
|------|----------|
| `era_at_j2000_noon` | `θ(2451545.0) ≈ 280.46°` (known) |
| `gmst_j2000_midnight` | `GMST(2451544.5) ≈ 6h39m51s = 99.968°` (Meeus) |
| `gmst_monotonic` | `GMST(JD+1) > GMST(JD)` (modulo 2π handled) |
| `lst_east_offset` | `LST(gmst, +90°) = gmst + π/2` |
| `era_range` | Result always in `[0, 2π)` |

---

## Ayanamsha Module

**Crate**: `dhruv_vedic_base`
**New file**: `crates/dhruv_vedic_base/src/ayanamsha.rs`

### Concept

The ayanamsha is the angular offset between the tropical zodiac (defined by
the vernal equinox) and a sidereal zodiac (defined by fixed stars). As the
equinox precesses westward, the ayanamsha increases over time.

Each ayanamsha system is defined by anchoring a specific star or sidereal
point at a specific ecliptic longitude. The differences between systems
reduce to a single parameter: the ayanamsha value at the J2000.0 epoch.

### Core Formula

```
ayanamsha(T) = reference_value_at_J2000 + p_A(T) / 3600.0  [degrees]
```

Where:
- `T` = Julian centuries of TDB since J2000.0
- `p_A(T)` = IAU 2006 general precession in longitude (arcseconds)
- At T=0 (J2000.0), `p_A = 0`, so the formula returns the reference value

### AyanamshaSystem Enum

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AyanamshaSystem {
    /// Lahiri (Chitrapaksha): Spica (α Vir) at 0° Libra sidereal.
    /// Indian government standard, adopted by N.C. Lahiri and the
    /// Indian Calendar Reform Committee (1957).
    Lahiri,

    /// True Lahiri (True Chitrapaksha): same anchor as Lahiri but
    /// uses the true (nutation-corrected) equinox instead of the mean equinox.
    /// Requires nutation-in-longitude (Δψ) parameter.
    TrueLahiri,

    /// Krishnamurti Paddhati (KP): system used in KP astrology,
    /// slightly offset from Lahiri.
    KP,

    /// B.V. Raman: defined in B.V. Raman's "Hindu Predictive Astrology".
    /// Zero ayanamsha year approximately 397 CE.
    Raman,

    /// Fagan-Bradley: primary Western sidereal system.
    /// Defined by Cyril Fagan, refined by Donald Bradley.
    /// Based on Spica at 29° Virgo (opposite of Lahiri's convention).
    FaganBradley,

    /// Pushya Paksha: Pushya nakshatra (δ Cancri) at 16° Cancer = 106° sidereal.
    PushyaPaksha,

    /// Rohini Paksha: Aldebaran (α Tau) at Rohini nakshatra ~15°47' Taurus.
    RohiniPaksha,

    /// Robert DeLuce ayanamsha (1930s).
    DeLuce,

    /// Djwal Khul: esoteric astrology system (Alice Bailey tradition).
    DjwalKhul,

    /// Hipparchos: derived from Hipparchus's observations (~128 BCE).
    Hipparchos,

    /// Sassanian: based on Sassanid-era Persian astronomical tradition.
    Sassanian,

    /// Deva-Dutta ayanamsha.
    DevaDutta,

    /// Usha-Shashi ayanamsha.
    UshaShashi,

    /// Sri Yukteshwar: defined in "The Holy Science" (1894) by
    /// Swami Sri Yukteshwar Giri. Zero ayanamsha year ~499 CE.
    Yukteshwar,

    /// J.N. Bhasin ayanamsha.
    JnBhasin,

    /// Chandra Hari ayanamsha.
    ChandraHari,

    /// Jagganatha ayanamsha.
    Jagganatha,

    /// Surya Siddhanta: based on the ancient Indian astronomical treatise.
    /// Uses IAU precession (not the traditional fixed 54″/year rate)
    /// for mathematical consistency. The reference value is back-computed
    /// from the traditional definition.
    SuryaSiddhanta,

    /// Galactic Center at 0° Sagittarius sidereal (0°Sg0).
    /// Defines the Galactic Center as the origin of Sagittarius.
    GalacticCenter0Sag,

    /// Aldebaran at 15° Taurus sidereal (15°Ta0).
    /// Star-anchored system placing Aldebaran at 15° Taurus.
    Aldebaran15Tau,
}
```

### Reference Values at J2000.0

Each system is characterized by its ayanamsha value at J2000.0 (degrees).
These values are derived from published system definitions by:

1. Identifying the system's defining anchor (star + sidereal longitude, or
   zero-ayanamsha epoch)
2. Computing the ayanamsha at J2000.0 using the IAU 2006 precession
   polynomial

**Provenance**: Values must be independently computed from the published
definition of each system. They must NOT be copied from any denylisted
source code. The clean-room record (`docs/clean_room_ayanamsha.md`) must
document the derivation for each system.

```rust
impl AyanamshaSystem {
    /// Reference ayanamsha at J2000.0 in degrees.
    pub const fn reference_j2000_deg(self) -> f64 { ... }

    /// Whether this system uses the true (nutation-corrected) equinox.
    pub const fn uses_true_equinox(self) -> bool { ... }

    /// All defined systems.
    pub const fn all() -> &'static [AyanamshaSystem] { ... }
}
```

### Public API

```rust
/// Mean ayanamsha in degrees at a given epoch.
///
/// T = Julian centuries of TDB since J2000.0.
/// Uses the IAU 2006 general precession polynomial to extrapolate
/// from the system's J2000.0 reference value.
pub fn ayanamsha_mean_deg(system: AyanamshaSystem, t_centuries: f64) -> f64

/// True (nutation-corrected) ayanamsha in degrees.
///
/// For TrueLahiri, adds delta_psi (nutation in longitude) to the mean value.
/// For all other systems, returns the mean value unchanged.
///
/// delta_psi_arcsec: nutation in longitude in arcseconds.
/// A full nutation model (IAU 2000B) is a separate future module.
/// Callers who have nutation from an external source can pass it here.
pub fn ayanamsha_true_deg(
    system: AyanamshaSystem,
    t_centuries: f64,
    delta_psi_arcsec: f64,
) -> f64

/// Convert JD TDB to Julian centuries since J2000.0.
pub fn jd_tdb_to_centuries(jd_tdb: f64) -> f64

/// Convert TDB seconds past J2000.0 to Julian centuries.
pub fn tdb_seconds_to_centuries(tdb_s: f64) -> f64
```

### DerivedComputation Adapter (Optional)

```rust
/// Adapter wrapping AyanamshaSystem as a DerivedComputation for use with
/// Engine::query_with_derived(). The ayanamsha depends only on epoch,
/// not on the target body — the StateVector is ignored.
pub struct AyanamshaComputation {
    system: AyanamshaSystem,
}

impl DerivedComputation for AyanamshaComputation {
    fn name(&self) -> &'static str { "ayanamsha" }
    fn compute(&self, _engine, query, _state) -> Result<DerivedValue> {
        let t = jd_tdb_to_centuries(query.epoch_tdb_jd);
        Ok(DerivedValue::Scalar(ayanamsha_mean_deg(self.system, t)))
    }
}
```

### Ayanamsha Unit Tests (in `src/ayanamsha.rs`)

| Test | Check |
|------|-------|
| `all_systems_count` | `AyanamshaSystem::all().len() == 20` |
| `lahiri_at_j2000` | `ayanamsha_mean_deg(Lahiri, 0.0) == reference_j2000_deg()` |
| `precession_forward` | `Lahiri at T=1 > Lahiri at T=0` (by ~1.40°) |
| `precession_backward` | `Lahiri at T=-1 < Lahiri at T=0` |
| `true_lahiri_zero_nutation` | `true_deg(TrueLahiri, T, 0.0) == mean_deg(TrueLahiri, T)` |
| `true_lahiri_nutation_offset` | `true_deg(TrueLahiri, 0, 17.0) ≈ ref + 0.00472°` |
| `non_true_ignores_nutation` | `true_deg(Lahiri, 0, 999.0) == mean_deg(Lahiri, 0)` |
| `reference_values_in_range` | All references between 19° and 28° |
| `century_conversion_roundtrip` | `jd_tdb_to_centuries` ↔ inverse consistency |

---

## Sunrise/Sunset Module

### Types

**New file**: `crates/dhruv_vedic_base/src/riseset_types.rs`

#### GeoLocation

```rust
/// Geographic location on Earth's surface.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GeoLocation {
    /// Geodetic latitude in degrees, north positive. Range: [-90, 90].
    pub latitude_deg: f64,
    /// Geodetic longitude in degrees, east positive. Range: [-180, 180].
    pub longitude_deg: f64,
    /// Altitude above mean sea level in meters.
    pub altitude_m: f64,
}

impl GeoLocation {
    pub fn new(lat: f64, lon: f64, alt: f64) -> Self { ... }
    pub fn latitude_rad(&self) -> f64 { ... }
    pub fn longitude_rad(&self) -> f64 { ... }
}
```

#### RiseSetEvent

```rust
/// Rise/set event types, including twilight variants.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RiseSetEvent {
    Sunrise,              // Upper limb at horizon
    Sunset,
    CivilDawn,            // Sun center at −6°
    CivilDusk,
    NauticalDawn,         // Sun center at −12°
    NauticalDusk,
    AstronomicalDawn,     // Sun center at −18°
    AstronomicalDusk,
}
```

Methods:

```rust
impl RiseSetEvent {
    /// Depression angle (degrees below geometric horizon).
    /// Sunrise/Sunset: 0.8333° = 50' (34' refraction + 16' semidiameter).
    /// Twilight: 6°, 12°, 18°.
    pub fn depression_deg(self) -> f64

    /// Whether this is a rising (morning) event.
    pub fn is_rising(self) -> bool
}
```

#### RiseSetConfig

```rust
/// Configurable parameters for rise/set computation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RiseSetConfig {
    /// Atmospheric refraction at horizon (arcminutes). Default: 34.0.
    pub refraction_arcmin: f64,
    /// Solar angular semi-diameter (arcminutes). Default: 16.0.
    pub semidiameter_arcmin: f64,
    /// Apply geometric dip correction for observer altitude.
    /// Dip ≈ √(2h/R_earth) radians. Default: true.
    pub altitude_correction: bool,
}
```

Methods:

```rust
impl RiseSetConfig {
    /// Total horizon depression for sunrise/sunset, accounting for
    /// refraction, semidiameter, and (optionally) altitude dip.
    ///
    /// h0 = −(refraction + semidiameter)/60 − dip
    pub fn horizon_depression_deg(&self, altitude_m: f64) -> f64
}

impl Default for RiseSetConfig {
    fn default() -> Self {
        Self {
            refraction_arcmin: 34.0,
            semidiameter_arcmin: 16.0,
            altitude_correction: true,
        }
    }
}
```

#### RiseSetResult

```rust
/// Result of a rise/set computation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RiseSetResult {
    /// Event occurs at the given Julian Date (TDB).
    Event { jd_tdb: f64, event: RiseSetEvent },
    /// Sun never rises during this solar day (polar night).
    NeverRises,
    /// Sun never sets during this solar day (midnight sun).
    NeverSets,
}
```

### Algorithm

**New file**: `crates/dhruv_vedic_base/src/riseset.rs`

The algorithm finds the time when the Sun's geocentric altitude equals
a target depression angle, for a given observer location and date. It is
an iterative method based on standard spherical astronomy formulas.

**Source**: Standard astronomical spherical trigonometry. The hour angle
formula and iterative rise/set method are published in numerous public
textbooks (e.g., Meeus "Astronomical Algorithms", US Naval Observatory
publications, Montenbruck & Pfleger "Astronomy on the Personal Computer").
This is an original implementation from the fundamental formulas.

#### Step-by-step

1. **Estimate local noon**: `JD_noon = JD_midnight + 0.5 − longitude/360`
2. **Query Sun position**: `engine.query(Sun, Earth, ICRF, JD_noon)` →
   `cartesian_to_spherical()` → (RA, Dec) in equatorial J2000
3. **Convert UTC→UT1**: `eop.utc_to_ut1_jd(JD_noon)`, then compute `gmst_rad(JD_ut1_noon)`
4. **Determine target altitude h₀**:
   - Sunrise/Sunset: `−config.horizon_depression_deg(altitude)`
   - Twilight: `−event.depression_deg()`
5. **Hour angle formula**:
   ```
   cos(H₀) = [sin(h₀) − sin(φ)·sin(δ)] / [cos(φ)·cos(δ)]
   ```
   - φ = observer latitude, δ = Sun declination, h₀ = target altitude
6. **Polar check**: If `|cos(H₀)| > 1`, return NeverRises or NeverSets
7. **Estimate event time**:
   - Transit: when Sun's hour angle = 0
   - Rise = transit − H₀/(2π) days
   - Set = transit + H₀/(2π) days
8. **Iterate** (up to 5 times, converge to ~0.1 second):
   - Recompute Sun RA/Dec at estimated event time
   - Recompute GMST and hour angle at event time
   - Compute correction and apply
9. Return `RiseSetResult::Event` or polar result

**Constants**:
```rust
const MAX_ITERATIONS: usize = 5;
const CONVERGENCE_DAYS: f64 = 1.0e-6;  // ~0.086 seconds
```

#### Internal Helper

```rust
/// Compute Sun's geocentric equatorial RA and Dec at a given JD TDB.
/// Returns (ra_rad, dec_rad) in ICRF/J2000.
fn sun_equatorial_ra_dec(engine: &Engine, jd_tdb: f64)
    -> Result<(f64, f64), VedicError>
```

#### Public API

```rust
/// Compute rise or set time for a specific event.
///
/// jd_utc_noon: approximate local noon on the desired date (UTC JD).
/// Use approximate_local_noon_jd() to compute from calendar date + longitude.
/// eop: IERS Earth Orientation Parameters for proper UTC→UT1 conversion.
pub fn compute_rise_set(
    engine: &Engine,
    eop: &EopKernel,
    location: &GeoLocation,
    event: RiseSetEvent,
    jd_utc_noon: f64,
    config: &RiseSetConfig,
) -> Result<RiseSetResult, VedicError>

/// Compute all 8 events for a day, in chronological order:
/// AstronomicalDawn, NauticalDawn, CivilDawn, Sunrise,
/// Sunset, CivilDusk, NauticalDusk, AstronomicalDusk.
pub fn compute_all_events(
    engine: &Engine,
    eop: &EopKernel,
    location: &GeoLocation,
    jd_utc_noon: f64,
    config: &RiseSetConfig,
) -> Result<Vec<RiseSetResult>, VedicError>

/// Approximate local solar noon JD from 0h UT JD and longitude.
/// JD_noon ≈ JD_0h + 0.5 − longitude_deg/360
pub fn approximate_local_noon_jd(jd_ut_midnight: f64, longitude_deg: f64) -> f64
```

### Rise/Set Unit Tests (in `src/riseset_types.rs` and `src/riseset.rs`)

| Test | Check |
|------|-------|
| `depression_sunrise` | `Sunrise.depression_deg() ≈ 0.8333` |
| `depression_civil` | `CivilDawn.depression_deg() == 6.0` |
| `is_rising_correct` | Sunrise/CivilDawn/etc = true, Sunset/etc = false |
| `default_config_values` | refraction=34, semidiameter=16, altitude=true |
| `horizon_depression_sea_level` | `(34+16)/60 = 0.833°` (no dip at 0m) |
| `horizon_depression_with_altitude` | 1000m adds ~1.01° dip |
| `cos_h_equator_equinox` | Known δ=0, φ=0, h0=-0.833° → cos(H) calculable |
| `cos_h_polar_never_rises` | φ=70°, δ=-23° → cos(H) > 1 |
| `cos_h_polar_never_sets` | φ=70°, δ=+23° → cos(H) < -1 |
| `local_noon_greenwich` | lon=0 → JD_noon = JD_0h + 0.5 |
| `local_noon_east` | lon=90E → JD_noon = JD_0h + 0.25 |

---

## Error Type

**New file**: `crates/dhruv_vedic_base/src/error.rs`

```rust
use std::error::Error;
use std::fmt;
use dhruv_core::EngineError;
use dhruv_time::TimeError;

#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum VedicError {
    /// Error propagated from the ephemeris engine.
    Engine(EngineError),
    /// Error propagated from time conversions (e.g., EOP out of range).
    Time(TimeError),
    /// Invalid geographic location parameters.
    InvalidLocation(&'static str),
    /// Rise/set iteration did not converge.
    NoConvergence(&'static str),
}

impl fmt::Display for VedicError { ... }
impl Error for VedicError { ... }
impl From<EngineError> for VedicError { ... }
impl From<TimeError> for VedicError { ... }
```

---

## Crate Root

**Edit**: `crates/dhruv_vedic_base/src/lib.rs`

```rust
//! Open derived Vedic calculations built on core ephemeris outputs.
//!
//! This crate provides:
//! - Ayanamsha computation for 20 sidereal reference systems
//! - Sunrise/sunset and twilight calculations
//!
//! All implementations are clean-room, derived from IAU standards
//! and public astronomical formulas.

pub mod ayanamsha;
pub mod error;
pub mod riseset;
pub mod riseset_types;

pub use ayanamsha::{
    AyanamshaComputation, AyanamshaSystem,
    ayanamsha_mean_deg, ayanamsha_true_deg,
    jd_tdb_to_centuries, tdb_seconds_to_centuries,
};
pub use error::VedicError;
pub use riseset::{approximate_local_noon_jd, compute_all_events, compute_rise_set};
pub use riseset_types::{GeoLocation, RiseSetConfig, RiseSetEvent, RiseSetResult};
```

**Edit**: `crates/dhruv_vedic_base/Cargo.toml`

```toml
[package]
name = "dhruv_vedic_base"
version = "0.1.0"
license = "Apache-2.0"
edition = "2024"

[dependencies]
dhruv_core   = { path = "../dhruv_core" }
dhruv_time   = { path = "../dhruv_time" }
dhruv_frames = { path = "../dhruv_frames" }
```

---

## Integration Tests

### Ayanamsha Golden Tests

**New file**: `crates/dhruv_vedic_base/tests/ayanamsha_golden.rs`

These do NOT require kernel files (ayanamsha is computed from precession
polynomial alone, no engine queries needed). However, they validate against
published almanac values.

| System | Epoch | Expected (°) | Tolerance | Source |
|--------|-------|--------------|-----------|--------|
| Lahiri | J2000.0 (2000-01-01.5 TDB) | ~23.85 | 0.01° | Indian Astronomical Ephemeris |
| Lahiri | 2024-01-01 | ~24.17 | 0.02° | Rashtriya Panchang 2024 |
| Lahiri | 1950-01-01 | ~23.15 | 0.02° | Published tables |
| FaganBradley | J2000.0 | ~24.74 | 0.02° | Published WS tables |
| Raman | J2000.0 | ~22.37 | 0.02° | B.V. Raman tables |

### Sunrise/Sunset Golden Tests

**New file**: `crates/dhruv_vedic_base/tests/riseset_golden.rs`

Requires kernel files (de442s.bsp, naif0012.tls) AND IERS EOP file
(finals2000A.all). Skip gracefully if any are absent.
Golden values obtained via black-box comparison with USNO Solar Calculator.

| Location | Date | Event | Expected (UTC) | Tolerance |
|----------|------|-------|----------------|-----------|
| New Delhi (28.6139°N, 77.209°E) | 2024-03-20 | Sunrise | ~00:48 UTC (06:18 IST) | 2 min |
| New Delhi | 2024-03-20 | Sunset | ~12:55 UTC (18:25 IST) | 2 min |
| London (51.5074°N, 0.1278°W) | 2024-06-21 | Sunrise | ~03:43 UTC | 2 min |
| London | 2024-06-21 | Sunset | ~20:21 UTC | 2 min |
| Tromso (69.65°N, 18.96°E) | 2024-06-21 | Sunrise | NeverSets | exact |
| Tromso | 2024-12-21 | Sunrise | NeverRises | exact |
| Sydney (33.87°S, 151.21°E) | 2024-03-20 | Sunrise | ~20:07 UTC (prev day) | 2 min |

---

## Implementation Order

| Step | What | Files | Depends On |
|------|------|-------|------------|
| 1 | Precession polynomial | `dhruv_frames/src/precession.rs`, `lib.rs` | — |
| 2 | IERS EOP parser (DUT1) | `dhruv_time/src/eop.rs`, `lib.rs`, `error.rs` | — |
| 3 | GMST/ERA | `dhruv_time/src/sidereal.rs`, `lib.rs` | 2 |
| 4 | Cargo.toml deps | `dhruv_vedic_base/Cargo.toml` | 1, 3 |
| 5 | Error type | `dhruv_vedic_base/src/error.rs` | 4 |
| 6 | Ayanamsha | `dhruv_vedic_base/src/ayanamsha.rs` | 1, 5 |
| 7 | Rise/set types | `dhruv_vedic_base/src/riseset_types.rs` | 5 |
| 8 | Rise/set algorithm | `dhruv_vedic_base/src/riseset.rs` | 3, 7 |
| 9 | Crate root | `dhruv_vedic_base/src/lib.rs` | 5–8 |
| 10 | Integration tests | `dhruv_vedic_base/tests/`, `dhruv_time/tests/` | 9 |
| 11 | Clean-room records | `docs/clean_room_*.md` | 10 |

Steps 1 & 2 are independent (parallel). Steps 6 & 7 are independent (parallel).
Step 3 (GMST) depends on step 2 (EOP) because callers need EOP to convert UTC→UT1 before calling GMST.

---

## Verification Checklist

1. `cargo test -p dhruv_frames` — precession unit tests pass
2. `cargo test -p dhruv_time` — EOP parser + sidereal unit tests pass
3. `cargo test -p dhruv_vedic_base` — all unit tests pass
4. `cargo test -p dhruv_vedic_base --test ayanamsha_golden` — golden values match
5. `cargo test -p dhruv_vedic_base --test riseset_golden` — sunrise/sunset within 2 min
6. `cargo test -p dhruv_time --test eop_integration` — real EOP file loads correctly
7. `cargo test --workspace` — full workspace (128+ tests) still passes
8. `cargo clippy --workspace` — no new warnings

---

## Design Decisions

1. **Proper UT1 via IERS EOP**: Load finals2000A.all via `EopKernel` (same
   pattern as `LeapSecondKernel`). GMST takes `jd_ut1`; callers convert via
   `eop.utc_to_ut1_jd()`. No UTC≈UT1 approximation.

2. **Precession in `dhruv_frames`**, **GMST+EOP in `dhruv_time`**: These are
   general astronomy concepts, not Vedic-specific. Other crates may need them.

3. **Ayanamsha is standalone functions**, not DerivedComputation: Ayanamsha depends
   only on epoch (not body query). The `DerivedComputation` adapter is optional.

4. **Sunrise/sunset takes `&Engine` + `&EopKernel`**: It needs multiple iterative
   engine queries, proper UT1 conversion, geographic location, and returns time.

5. **Polar cases return NeverRises/NeverSets**: These are valid astronomical
   conditions, not errors.

6. **TrueLahiri nutation is deferred**: The API accepts `delta_psi_arcsec` so
   callers with an external nutation source can use it. A full IAU 2000B
   nutation module (~77 terms) is a separate follow-up.

7. **Surya Siddhanta uses IAU precession**: For mathematical consistency across
   all systems. The traditional fixed 54″/year rate would diverge significantly
   from the IAU model over centuries.

8. **EOP out-of-range is an error**: No silent fallback to DUT1=0. Caller must
   provide valid EOP data covering their date range.

---

## Clean-Room Compliance

Two clean-room records must be created:

- **`docs/clean_room_ayanamsha.md`**: IAU 2006 precession source, derivation of
  each ayanamsha reference value, star catalog sources (Hipparcos FK5),
  declaration that no denylisted sources were referenced.

- **`docs/clean_room_riseset.md`**: IERS EOP file format (public domain),
  GMST/ERA formula source (IERS 2010), rise/set algorithm source (standard
  spherical astronomy), atmospheric refraction model, black-box validation
  methodology (USNO calculator).

No dependency on Swiss Ephemeris, no GPL/AGPL sources, no copyleft code.
All formulas from IAU standards and public astronomical literature.
