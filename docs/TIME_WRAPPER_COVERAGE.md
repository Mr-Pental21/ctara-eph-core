# dhruv_time C ABI Coverage

Scope: crate-root runtime free functions from `dhruv_time` (7 functions).

Direct C ABI coverage: `0 / 7`.

| Runtime API | C ABI export(s) | Status |
|---|---|---|
| `calendar_to_jd` | `-` | Missing direct wrapper |
| `jd_to_calendar` | `-` | Missing direct wrapper |
| `jd_to_tdb_seconds` | `-` | Missing direct wrapper |
| `tdb_seconds_to_jd` | `-` | Missing direct wrapper |
| `earth_rotation_angle_rad` | `-` | Missing direct wrapper |
| `gmst_rad` | `-` | Missing direct wrapper |
| `local_sidereal_time_rad` | `-` | Missing direct wrapper |

Related C ABI time conversions (composed APIs):
- `dhruv_utc_to_tdb_jd`
- `dhruv_jd_tdb_to_utc`
