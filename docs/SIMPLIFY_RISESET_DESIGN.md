# Design: Simplify Rise/Set Config & Add IAU 2000B Nutation

## Context

The current rise/set C ABI requires users to pass raw numeric values for `refraction_arcmin`, `semidiameter_arcmin`, and `delta_psi_arcsec`. Users shouldn't need to know these values — they should be computed internally with simple flags to enable/disable them.

**Goals:**
1. Add IAU 2000B nutation model so `delta_psi` is computed internally
2. Replace numeric `refraction_arcmin` / `semidiameter_arcmin` with boolean/enum flags
3. Add `SunLimb` enum to control which part of the sun disk defines sunrise/sunset
4. Compute solar semidiameter dynamically from Earth-Sun distance (already available)

---

## Step 1: IAU 2000B Nutation Model

**New file:** `crates/dhruv_frames/src/nutation.rs`
**Modify:** `crates/dhruv_frames/src/lib.rs` (register module, re-export)

Implements the IAU 2000B truncated nutation model (77 lunisolar terms).

**Source:** IERS Conventions 2010, Chapter 5, Table 5.3b. Public domain (IAU standard).

### Functions

```rust
/// Five Delaunay fundamental arguments (l, l', F, D, Ω) in radians.
fn fundamental_arguments(t: f64) -> [f64; 5]

/// IAU 2000B nutation: returns (delta_psi_arcsec, delta_epsilon_arcsec).
pub fn nutation_iau2000b(t: f64) -> (f64, f64)
```

- `t` = Julian centuries of TDB since J2000.0
- 77 terms, each: `[nl, nl', nF, nD, nΩ, S_i, S'_i, C_i, C'_i]`
- Δψ = Σ (S_i + S'_i·T) · sin(nl·l + nl'·l' + nF·F + nD·D + nΩ·Ω)
- Δε = Σ (C_i + C'_i·T) · cos(...)

**Pattern:** Follows `precession.rs` — module-level doc citing paper, arcsec variant, polynomial eval.

### Tests (unit, in nutation.rs)
- `zero_at_j2000_is_finite` — sanity check returns are finite
- `typical_amplitude` — |Δψ| < 20″, |Δε| < 10″ at T=0.24 (~2024)
- `known_value_2024` — cross-check against IAU reference (Δψ ≈ -14″ ± 2″ at 2024.0)

---

## Step 2: SunLimb Enum + Simplified RiseSetConfig

**Modify:** `crates/dhruv_vedic_base/src/riseset_types.rs`

### New Enum

```rust
/// Which part of the solar disk defines the sunrise/sunset event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum SunLimb {
    /// Sunrise = upper limb appears; Sunset = lower limb disappears.
    #[default]
    UpperLimb,
    /// Sunrise/sunset defined by center of disk.
    Center,
    /// Sunrise = lower limb appears; Sunset = upper limb disappears.
    LowerLimb,
}
```

### Updated RiseSetConfig

```rust
pub struct RiseSetConfig {
    /// Apply standard atmospheric refraction (34 arcmin). Default: true.
    pub use_refraction: bool,
    /// Which solar limb defines sunrise/sunset. Default: UpperLimb.
    pub sun_limb: SunLimb,
    /// Apply geometric dip for observer altitude. Default: true.
    pub altitude_correction: bool,
}
```

**Removed:** `refraction_arcmin` (hardcoded 34.0 when enabled), `semidiameter_arcmin` (computed dynamically).

### New Method: `target_altitude_deg`

Replaces `horizon_depression_deg`. Takes the event type and dynamic semidiameter:

```rust
impl RiseSetConfig {
    /// Target altitude for the center of the Sun at the event.
    pub fn target_altitude_deg(
        &self,
        event: RiseSetEvent,
        semidiameter_arcmin: f64,
        altitude_m: f64,
    ) -> f64
}
```

**Logic for sunrise/sunset:**

| SunLimb    | Rising event h0            | Setting event h0           |
|------------|---------------------------|---------------------------|
| UpperLimb  | -(R + S) / 60             | -(R - S) / 60             |
| Center     | -R / 60                   | -R / 60                   |
| LowerLimb  | -(R - S) / 60             | -(R + S) / 60             |

Where R = 34.0 if `use_refraction`, else 0.0. S = dynamic semidiameter arcmin. Dip added for altitude.

**Logic for twilight:** Fixed IAU angles (6°/12°/18°), no refraction or semidiameter.

### Update `RiseSetEvent::depression_deg`

Keep only twilight cases. Remove sunrise/sunset arm (now handled by config).

### Tests to update
- `depression_sunrise` — remove (no longer a fixed property)
- `default_config` — update for new fields
- `depression_sea_level`, `depression_1000m`, `depression_no_altitude_correction` — rewrite using `target_altitude_deg`
- Add: `target_altitude_upper_limb_rising/setting`, `target_altitude_center`, `target_altitude_lower_limb`, `target_altitude_no_refraction`

---

## Step 3: Dynamic Semidiameter + Updated Riseset Algorithm

**Modify:** `crates/dhruv_vedic_base/src/riseset.rs`

### Change `sun_equatorial_ra_dec` to return distance

Currently returns `(ra_rad, dec_rad)`, discards distance. Change to:

```rust
fn sun_equatorial_ra_dec(engine: &Engine, jd_tdb: f64) -> Result<(f64, f64, f64), VedicError>
```

Returns `(ra_rad, dec_rad, distance_km)`. The distance is already computed by `cartesian_to_spherical()` — just return `sph.distance_km` alongside `sph.lon_deg` and `sph.lat_deg` (converting to radians for the RA/Dec context).

### Compute semidiameter from distance

```rust
const SUN_RADIUS_KM: f64 = 696_000.0; // IAU 2015 nominal

let semidiameter_arcmin = (SUN_RADIUS_KM / distance_km).asin().to_degrees() * 60.0;
// Varies: ~15.7' (aphelion) to ~16.3' (perihelion)
```

### Update `compute_rise_set`

1. At initial query, get `(ra, dec, dist)` instead of `(ra, dec)`
2. Compute `semidiameter_arcmin` from `dist`
3. Use `config.target_altitude_deg(event, semidiameter_arcmin, altitude_m)` instead of `config.horizon_depression_deg(altitude_m)`
4. In the iteration loop, recompute `(ra_i, dec_i, dist_i)` and update semidiameter
5. No change for twilight events (semidiameter passed as 0.0 for twilight, but `target_altitude_deg` ignores it anyway)

### Tests
- Existing integration tests (`sunrise_new_delhi`, `polar_never_sets`) — update to use new config
- Add: `sunrise_lower_limb_later_than_upper` — verify UpperLimb sunrise < LowerLimb sunrise
- Add: `center_mode_between_limbs` — Center sunrise is between Upper and Lower

---

## Step 4: Unified Ayanamsha Function

**Modify:** `crates/dhruv_vedic_base/src/ayanamsha.rs`

### New function

```rust
/// Compute ayanamsha, optionally with nutation correction.
///
/// When `use_nutation` is true and the system uses the true equinox
/// (currently only TrueLahiri), nutation in longitude is computed
/// internally via IAU 2000B and added to the mean value.
pub fn ayanamsha_deg(
    system: AyanamshaSystem,
    t_centuries: f64,
    use_nutation: bool,
) -> f64
```

Calls `dhruv_frames::nutation_iau2000b(t)` internally when needed.

**Existing functions remain** for backward compatibility — `ayanamsha_mean_deg` and `ayanamsha_true_deg` are unchanged.

### Tests
- `ayanamsha_deg_without_nutation_matches_mean` — same as `ayanamsha_mean_deg`
- `ayanamsha_deg_with_nutation_true_lahiri` — differs from mean by ~Δψ/3600

---

## Step 5: C ABI Updates

**Modify:** `crates/dhruv_ffi_c/src/lib.rs`

### Updated `DhruvRiseSetConfig`

```c
typedef struct {
    uint8_t use_refraction;      // 1 = apply 34' refraction, 0 = no
    int32_t sun_limb;            // 0=UpperLimb, 1=Center, 2=LowerLimb
    uint8_t altitude_correction; // 1 = apply dip, 0 = no
} DhruvRiseSetConfig;
```

### New constants

```c
#define DHRUV_SUN_LIMB_UPPER   0
#define DHRUV_SUN_LIMB_CENTER  1
#define DHRUV_SUN_LIMB_LOWER   2
```

### Updated `dhruv_riseset_config_default`

Returns: `use_refraction=1, sun_limb=DHRUV_SUN_LIMB_UPPER, altitude_correction=1`

### New FFI functions

```c
// Unified ayanamsha (computes nutation internally when needed)
DhruvStatus dhruv_ayanamsha_deg(
    int32_t system_code,
    double  jd_tdb,
    uint8_t use_nutation,  // 0 = mean, 1 = true (auto-computes nutation)
    double* out_deg
);

// Standalone nutation (for consumers who need raw values)
DhruvStatus dhruv_nutation_iau2000b(
    double  jd_tdb,
    double* out_dpsi_arcsec,
    double* out_deps_arcsec
);
```

**Existing functions remain:** `dhruv_ayanamsha_mean_deg` and `dhruv_ayanamsha_true_deg` unchanged for backward compatibility.

### Conversion helper update

The internal conversion from `DhruvRiseSetConfig` to Rust `RiseSetConfig` maps `sun_limb` int to `SunLimb` enum (0→UpperLimb, 1→Center, 2→LowerLimb, else InvalidQuery).

### Tests
- Update `ffi_riseset_config_default_values` for new fields
- Add `ffi_ayanamsha_deg_mean_matches_old`
- Add `ffi_ayanamsha_deg_true_lahiri_with_nutation`
- Add `ffi_nutation_iau2000b_at_j2000`
- Update integration tests: `ffi_sunrise_new_delhi`, `ffi_polar_never_sets_tromso`, `ffi_all_events_new_delhi`

---

## Step 6: Clean-Room Documentation

**New file:** `docs/clean_room_nutation.md`

Document:
- Source: IERS Conventions 2010, Chapter 5, Table 5.3b
- 5 Delaunay fundamental arguments (polynomial coefficients from IERS)
- 77 lunisolar term table with amplitudes
- No code-level reference to any copyleft implementation
- Solar radius: IAU 2015 Resolution B3, nominal value 696,000 km

---

## Files Modified (Summary)

| File | Change |
|------|--------|
| `crates/dhruv_frames/src/nutation.rs` | **NEW** — IAU 2000B nutation (77 terms) |
| `crates/dhruv_frames/src/lib.rs` | Register nutation module, re-export |
| `crates/dhruv_vedic_base/src/riseset_types.rs` | SunLimb enum, simplified RiseSetConfig, target_altitude_deg |
| `crates/dhruv_vedic_base/src/riseset.rs` | Return distance from sun query, compute semidiameter, use new config |
| `crates/dhruv_vedic_base/src/ayanamsha.rs` | Add `ayanamsha_deg()` unified function |
| `crates/dhruv_ffi_c/src/lib.rs` | Updated DhruvRiseSetConfig, new ayanamsha/nutation FFI |
| `crates/dhruv_ffi_c/tests/ffi_integration.rs` | Update integration tests for new config |
| `docs/clean_room_nutation.md` | **NEW** — clean-room provenance for nutation |

---

## Verification

1. `cargo test -p dhruv_frames` — nutation unit tests pass
2. `cargo test -p dhruv_vedic_base` — updated riseset_types + ayanamsha tests pass
3. `cargo test -p dhruv_ffi_c --lib` — updated unit tests pass
4. `cargo test -p dhruv_ffi_c --test ffi_integration` — updated integration tests pass
5. `cargo test --workspace` — all tests pass
6. `cargo clippy --workspace` — no warnings
