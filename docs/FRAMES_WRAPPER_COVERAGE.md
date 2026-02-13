# dhruv_frames C ABI Coverage

Scope: crate-root runtime APIs from `dhruv_frames` (9 functions).

Direct C ABI coverage: `2 / 9`.

| Runtime API | C ABI export(s) | Status |
|---|---|---|
| `cartesian_to_spherical` | `dhruv_cartesian_to_spherical` | Wrapped |
| `nutation_iau2000b` | `dhruv_nutation_iau2000b`, `dhruv_nutation_iau2000b_utc` | Wrapped |
| `cartesian_state_to_spherical_state` | `-` | Missing direct wrapper |
| `ecliptic_to_icrf` | `-` | Missing direct wrapper |
| `fundamental_arguments` | `-` | Missing direct wrapper |
| `general_precession_longitude_arcsec` | `-` | Missing direct wrapper |
| `general_precession_longitude_deg` | `-` | Missing direct wrapper |
| `icrf_to_ecliptic` | `-` | Missing direct wrapper |
| `spherical_to_cartesian` | `-` | Missing direct wrapper |
