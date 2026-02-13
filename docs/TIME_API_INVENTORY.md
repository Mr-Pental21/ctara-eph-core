# dhruv_time API Inventory

This document tracks the public API surface of `dhruv_time`.

## Crate-Root Runtime Surface

See `docs/TIME_RUNTIME_APIS.md` for callable crate-root APIs.

## Public Time Data Types

- `UtcTime`: typed UTC calendar struct used by higher-level crates.
- `LskData`: parsed leap-second kernel payload.
- `EopData`: parsed Earth orientation (DUT1) table.
- `EopKernel`: loaded EOP handle.
- `Epoch`: typed TDB epoch wrapper.
- `TimeError`: crate error enum.

## EOP APIs (`eop.rs`)

| API | Input | Output | Purpose |
|---|---|---|---|
| `EopData::parse_finals` | `content` | `Result<EopData, TimeError>` | Parse `finals2000A.all` content. |
| `EopData::len` | `&self` | `usize` | Number of EOP rows. |
| `EopData::is_empty` | `&self` | `bool` | Whether table has no rows. |
| `EopData::range` | `&self` | `(f64, f64)` | Covered MJD range. |
| `EopData::dut1_at_mjd` | `mjd` | `Result<f64, TimeError>` | Interpolated DUT1 at MJD. |
| `EopData::utc_to_ut1_jd` | `jd_utc` | `Result<f64, TimeError>` | UTC JD to UT1 JD conversion. |
| `EopKernel::load` | `path` | `Result<EopKernel, TimeError>` | Load EOP file from disk. |
| `EopKernel::parse` | `content` | `Result<EopKernel, TimeError>` | Parse EOP content from text. |
| `EopKernel::data` | `&self` | `&EopData` | Access parsed EOP rows. |
| `EopKernel::utc_to_ut1_jd` | `jd_utc` | `Result<f64, TimeError>` | UTC JD to UT1 JD conversion via kernel handle. |

## UTC APIs (`utc_time.rs`)

| API | Input | Output | Purpose |
|---|---|---|---|
| `UtcTime::new` | `year, month, day, hour, minute, second` | `UtcTime` | Construct UTC timestamp value. |
| `UtcTime::to_jd_tdb` | `&self, lsk` | `f64` | UTC calendar to Julian Date TDB. |
| `UtcTime::from_jd_tdb` | `jd_tdb, lsk` | `UtcTime` | Julian Date TDB to UTC calendar. |

## Scale/LSK Helpers (Public Module APIs)

| API | Input | Output | Purpose |
|---|---|---|---|
| `parse_lsk` | `content` | `Result<LskData, TimeError>` | Parse LSK text payload. |
| `lookup_delta_at` | `utc_seconds, lsk` | `f64` | Lookup cumulative leap seconds. |
| `utc_to_tai` | `utc_s, lsk` | `f64` | UTC -> TAI seconds past J2000. |
| `tai_to_tt` | `tai_s, lsk` | `f64` | TAI -> TT seconds past J2000. |
| `tt_to_tdb` | `tt_s, lsk` | `f64` | TT -> TDB seconds past J2000. |
| `tdb_to_tt` | `tdb_s, lsk` | `f64` | TDB -> TT seconds past J2000. |
| `tt_to_tai` | `tt_s, lsk` | `f64` | TT -> TAI seconds past J2000. |
| `tdb_to_utc` | `tdb_s, lsk` | `f64` | TDB -> UTC seconds past J2000 (full inverse chain). |
| `utc_to_tdb` | `utc_s, lsk` | `f64` | UTC -> TDB seconds past J2000 (full forward chain). |
| `month_from_abbrev` | `abbrev` | `Option<u32>` | Month abbreviation lookup. |
