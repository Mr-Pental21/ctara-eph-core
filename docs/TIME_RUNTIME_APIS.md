# dhruv_time Runtime API

This is the crate-root callable runtime surface of `dhruv_time`.

## Free Functions

| API | Input | Output | Purpose |
|---|---|---|---|
| `calendar_to_jd` | `year, month, day` | `f64` | Gregorian calendar to Julian Date. |
| `jd_to_calendar` | `jd` | `(i32, u32, f64)` | Julian Date to Gregorian calendar tuple. |
| `jd_to_tdb_seconds` | `jd` | `f64` | Julian Date (TDB) to seconds past J2000. |
| `tdb_seconds_to_jd` | `tdb_s` | `f64` | Seconds past J2000 to Julian Date (TDB). |
| `earth_rotation_angle_rad` | `jd_ut1` | `f64` | Earth rotation angle in radians. |
| `gmst_rad` | `jd_ut1` | `f64` | Greenwich Mean Sidereal Time in radians. |
| `local_sidereal_time_rad` | `gmst, longitude_east_rad` | `f64` | Local sidereal time in radians. |

## `LeapSecondKernel`

| API | Input | Output | Purpose |
|---|---|---|---|
| `LeapSecondKernel::load` | `path` | `Result<LeapSecondKernel, TimeError>` | Load LSK from file path. |
| `LeapSecondKernel::parse` | `content` | `Result<LeapSecondKernel, TimeError>` | Parse LSK from text. |
| `LeapSecondKernel::data` | `&self` | `&LskData` | Read parsed LSK payload. |
| `LeapSecondKernel::utc_to_tdb` | `utc_s` | `f64` | UTC seconds past J2000 to TDB seconds. |
| `LeapSecondKernel::tdb_to_utc` | `tdb_s` | `f64` | TDB seconds past J2000 to UTC seconds. |

## `Epoch`

| API | Input | Output | Purpose |
|---|---|---|---|
| `Epoch::from_tdb_seconds` | `s` | `Epoch` | Construct from TDB seconds past J2000. |
| `Epoch::from_jd_tdb` | `jd` | `Epoch` | Construct from Julian Date TDB. |
| `Epoch::from_utc` | `year, month, day, hour, min, sec, lsk` | `Epoch` | Construct from UTC calendar using LSK. |
| `Epoch::as_tdb_seconds` | `self` | `f64` | Read TDB seconds past J2000. |
| `Epoch::as_jd_tdb` | `self` | `f64` | Read Julian Date TDB. |
