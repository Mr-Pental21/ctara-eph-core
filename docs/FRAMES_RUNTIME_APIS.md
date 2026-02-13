# dhruv_frames Runtime API

This is the crate-root callable runtime surface of `dhruv_frames`.

| API | Input | Output | Purpose |
|---|---|---|---|
| `cartesian_to_spherical` | `xyz` | `SphericalCoords` | Cartesian `[x,y,z]` to spherical coordinates. |
| `spherical_to_cartesian` | `s` | `[f64; 3]` | Spherical coordinates back to Cartesian. |
| `cartesian_state_to_spherical_state` | `pos, vel` | `SphericalState` | Convert position+velocity state to spherical form. |
| `icrf_to_ecliptic` | `v` | `[f64; 3]` | Rotate vector from ICRF/J2000 to ecliptic J2000. |
| `ecliptic_to_icrf` | `v` | `[f64; 3]` | Rotate vector from ecliptic J2000 to ICRF/J2000. |
| `fundamental_arguments` | `t` | `[f64; 5]` | Delaunay fundamental arguments (radians). |
| `nutation_iau2000b` | `t` | `(f64, f64)` | IAU 2000B nutation (`Δψ`, `Δε`, arcseconds). |
| `general_precession_longitude_arcsec` | `t` | `f64` | IAU 2006 general precession longitude (arcseconds). |
| `general_precession_longitude_deg` | `t` | `f64` | IAU 2006 general precession longitude (degrees). |
