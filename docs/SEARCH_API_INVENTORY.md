# dhruv_search API Inventory

This document lists the public function surface used by `dhruv_search` callers,
with inputs, outputs, and behavior.

Notes:
- Most operational APIs return `Result<..., SearchError>`.
- Time arguments are either `UtcTime` (UTC-facing API) or `f64` Julian Date TDB (`jd_tdb`).
- Many input/output types come from `dhruv_core`, `dhruv_time`, and `dhruv_vedic_base`.

## Related Docs

- `docs/clean_room_conjunction.md`
- `docs/clean_room_grahan.md`
- `docs/clean_room_stationary.md`
- `docs/clean_room_panchang.md`
- `docs/clean_room_tithi_karana_yoga.md`
- `docs/clean_room_ashtakavarga.md`
- `docs/clean_room_drishti.md`
- `docs/clean_room_upagraha.md`
- C ABI mapping (for wrapper parity): `docs/C_ABI_REFERENCE.md`

## Error Type

`SearchError` (`crates/dhruv_search/src/error.rs`) has:
- `Engine(EngineError)`
- `InvalidConfig(&'static str)`
- `NoConvergence(&'static str)`

## Conjunction APIs

Source: `crates/dhruv_search/src/conjunction.rs`, `crates/dhruv_search/src/conjunction_types.rs`

| Function | Inputs | Output | What it does |
|---|---|---|---|
| `body_ecliptic_lon_lat` | `engine`, `body`, `jd_tdb` | `Result<(f64, f64), SearchError>` | Queries geocentric ecliptic longitude/latitude of a body (degrees). |
| `next_conjunction` | `engine`, `body1`, `body2`, `jd_tdb`, `config` | `Result<Option<ConjunctionEvent>, SearchError>` | Finds next event where body separation hits target angle in `config`. |
| `prev_conjunction` | `engine`, `body1`, `body2`, `jd_tdb`, `config` | `Result<Option<ConjunctionEvent>, SearchError>` | Finds previous target-separation event. |
| `search_conjunctions` | `engine`, `body1`, `body2`, `jd_start`, `jd_end`, `config` | `Result<Vec<ConjunctionEvent>, SearchError>` | Finds all target-separation events in range. |
| `ConjunctionConfig::conjunction` | `step_size_days` | `ConjunctionConfig` | Factory for 0 degree separation search. |
| `ConjunctionConfig::opposition` | `step_size_days` | `ConjunctionConfig` | Factory for 180 degree separation search. |
| `ConjunctionConfig::aspect` | `target_deg`, `step_size_days` | `ConjunctionConfig` | Factory for arbitrary aspect angle search. |

## Lunar Phase APIs

Source: `crates/dhruv_search/src/lunar_phase.rs`, `crates/dhruv_search/src/lunar_phase_types.rs`

| Function | Inputs | Output | What it does |
|---|---|---|---|
| `next_purnima` | `engine`, `utc` | `Result<Option<LunarPhaseEvent>, SearchError>` | Next full moon after UTC instant. |
| `prev_purnima` | `engine`, `utc` | `Result<Option<LunarPhaseEvent>, SearchError>` | Previous full moon before UTC instant. |
| `next_amavasya` | `engine`, `utc` | `Result<Option<LunarPhaseEvent>, SearchError>` | Next new moon after UTC instant. |
| `prev_amavasya` | `engine`, `utc` | `Result<Option<LunarPhaseEvent>, SearchError>` | Previous new moon before UTC instant. |
| `search_purnimas` | `engine`, `start`, `end` | `Result<Vec<LunarPhaseEvent>, SearchError>` | All full moons in UTC range. |
| `search_amavasyas` | `engine`, `start`, `end` | `Result<Vec<LunarPhaseEvent>, SearchError>` | All new moons in UTC range. |
| `LunarPhase::name` | `self` | `&'static str` | Returns display name (`"Amavasya"` or `"Purnima"`). |

## Grahan (Eclipse) APIs

Source: `crates/dhruv_search/src/grahan.rs`, `crates/dhruv_search/src/grahan_types.rs`

| Function | Inputs | Output | What it does |
|---|---|---|---|
| `next_chandra_grahan` | `engine`, `jd_tdb`, `config` | `Result<Option<ChandraGrahan>, SearchError>` | Next lunar eclipse candidate after `jd_tdb`, classified + contacts. |
| `prev_chandra_grahan` | `engine`, `jd_tdb`, `config` | `Result<Option<ChandraGrahan>, SearchError>` | Previous lunar eclipse before `jd_tdb`. |
| `search_chandra_grahan` | `engine`, `jd_start`, `jd_end`, `config` | `Result<Vec<ChandraGrahan>, SearchError>` | All lunar eclipses in range. |
| `next_surya_grahan` | `engine`, `jd_tdb`, `config` | `Result<Option<SuryaGrahan>, SearchError>` | Next geocentric solar eclipse after `jd_tdb`. |
| `prev_surya_grahan` | `engine`, `jd_tdb`, `config` | `Result<Option<SuryaGrahan>, SearchError>` | Previous geocentric solar eclipse before `jd_tdb`. |
| `search_surya_grahan` | `engine`, `jd_start`, `jd_end`, `config` | `Result<Vec<SuryaGrahan>, SearchError>` | All geocentric solar eclipses in range. |
| `GeoLocation::new` | `latitude_deg`, `longitude_deg`, `altitude_m` | `GeoLocation` | Constructor for grahan location struct. |
| `GeoLocation::latitude_rad` | `self` | `f64` | Latitude in radians. |
| `GeoLocation::longitude_rad` | `self` | `f64` | Longitude in radians. |

## Sankranti APIs

Source: `crates/dhruv_search/src/sankranti.rs`, `crates/dhruv_search/src/sankranti_types.rs`

| Function | Inputs | Output | What it does |
|---|---|---|---|
| `next_sankranti` | `engine`, `utc`, `config` | `Result<Option<SankrantiEvent>, SearchError>` | Next Sun-entry into any rashi after UTC time. |
| `prev_sankranti` | `engine`, `utc`, `config` | `Result<Option<SankrantiEvent>, SearchError>` | Previous Sun-entry into any rashi before UTC time. |
| `search_sankrantis` | `engine`, `start`, `end`, `config` | `Result<Vec<SankrantiEvent>, SearchError>` | All sankrantis in UTC range. |
| `next_specific_sankranti` | `engine`, `utc`, `rashi`, `config` | `Result<Option<SankrantiEvent>, SearchError>` | Next entry into a specific rashi. |
| `prev_specific_sankranti` | `engine`, `utc`, `rashi`, `config` | `Result<Option<SankrantiEvent>, SearchError>` | Previous entry into a specific rashi. |
| `SankrantiConfig::new` | `ayanamsha_system`, `use_nutation` | `SankrantiConfig` | Constructor with default scan parameters. |
| `SankrantiConfig::default_lahiri` | none | `SankrantiConfig` | Factory using Lahiri ayanamsha. |
| `SankrantiConfig::validate` | `&self` | `Result<(), &'static str>` | Validates search parameter ranges. |

## Stationary and Max-Speed APIs

Source: `crates/dhruv_search/src/stationary.rs`, `crates/dhruv_search/src/stationary_types.rs`

| Function | Inputs | Output | What it does |
|---|---|---|---|
| `next_stationary` | `engine`, `body`, `jd_tdb`, `config` | `Result<Option<StationaryEvent>, SearchError>` | Next station (velocity sign-crossing) after `jd_tdb`. |
| `prev_stationary` | `engine`, `body`, `jd_tdb`, `config` | `Result<Option<StationaryEvent>, SearchError>` | Previous station before `jd_tdb`. |
| `search_stationary` | `engine`, `body`, `jd_start`, `jd_end`, `config` | `Result<Vec<StationaryEvent>, SearchError>` | All stations in range. |
| `next_max_speed` | `engine`, `body`, `jd_tdb`, `config` | `Result<Option<MaxSpeedEvent>, SearchError>` | Next local speed extremum after `jd_tdb`. |
| `prev_max_speed` | `engine`, `body`, `jd_tdb`, `config` | `Result<Option<MaxSpeedEvent>, SearchError>` | Previous speed extremum before `jd_tdb`. |
| `search_max_speed` | `engine`, `body`, `jd_start`, `jd_end`, `config` | `Result<Vec<MaxSpeedEvent>, SearchError>` | All speed extrema in range. |
| `StationaryConfig::inner_planet` | none | `StationaryConfig` | Preset config for inner planets. |
| `StationaryConfig::outer_planet` | none | `StationaryConfig` | Preset config for outer planets. |

## Panchang APIs

Source: `crates/dhruv_search/src/panchang.rs`

| Function | Inputs | Output | What it does |
|---|---|---|---|
| `masa_for_date` | `engine`, `utc`, `sankranti_config` | `Result<MasaInfo, SearchError>` | Computes amanta lunar month + adhika flag + boundaries. |
| `ayana_for_date` | `engine`, `utc`, `sankranti_config` | `Result<AyanaInfo, SearchError>` | Computes current ayana and its start/end transitions. |
| `varsha_for_date` | `engine`, `utc`, `sankranti_config` | `Result<VarshaInfo, SearchError>` | Computes samvatsara position and Vedic year boundaries. |
| `elongation_at` | `engine`, `jd_tdb` | `Result<f64, SearchError>` | Computes `(Moon_lon - Sun_lon) mod 360` (tropical). |
| `sidereal_sum_at` | `engine`, `jd_tdb`, `sankranti_config` | `Result<f64, SearchError>` | Computes `(Moon_sid + Sun_sid) mod 360`. |
| `moon_sidereal_longitude_at` | `engine`, `jd_tdb`, `sankranti_config` | `Result<f64, SearchError>` | Computes Moon sidereal longitude. |
| `nakshatra_for_date` | `engine`, `utc`, `sankranti_config` | `Result<PanchangNakshatraInfo, SearchError>` | Computes current nakshatra/pada with start/end. |
| `nakshatra_at` | `engine`, `jd_tdb`, `moon_sidereal_deg`, `sankranti_config` | `Result<PanchangNakshatraInfo, SearchError>` | Same as above using precomputed Moon sidereal longitude. |
| `tithi_for_date` | `engine`, `utc` | `Result<TithiInfo, SearchError>` | Computes tithi + paksha + start/end. |
| `tithi_at` | `engine`, `jd_tdb`, `elongation_deg` | `Result<TithiInfo, SearchError>` | Same as above using precomputed elongation. |
| `karana_for_date` | `engine`, `utc` | `Result<KaranaInfo, SearchError>` | Computes karana with start/end. |
| `karana_at` | `engine`, `jd_tdb`, `elongation_deg` | `Result<KaranaInfo, SearchError>` | Same as above using precomputed elongation. |
| `yoga_for_date` | `engine`, `utc`, `sankranti_config` | `Result<YogaInfo, SearchError>` | Computes yoga with start/end. |
| `yoga_at` | `engine`, `jd_tdb`, `sidereal_sum_deg`, `sankranti_config` | `Result<YogaInfo, SearchError>` | Same as above using precomputed sidereal sum. |
| `vedic_day_sunrises` | `engine`, `eop`, `utc`, `location`, `riseset_config` | `Result<(f64, f64), SearchError>` | Returns sunrise and next-sunrise JD bounds for the Vedic day. |
| `vaar_for_date` | `engine`, `eop`, `utc`, `location`, `riseset_config` | `Result<VaarInfo, SearchError>` | Computes Vedic weekday with sunrise boundaries. |
| `vaar_from_sunrises` | `sunrise_jd`, `next_sunrise_jd`, `lsk` | `VaarInfo` | Pure arithmetic weekday result from sunrise pair. |
| `hora_for_date` | `engine`, `eop`, `utc`, `location`, `riseset_config` | `Result<HoraInfo, SearchError>` | Computes planetary hour with start/end. |
| `hora_from_sunrises` | `jd_tdb`, `sunrise_jd`, `next_sunrise_jd`, `lsk` | `HoraInfo` | Pure arithmetic hora classification from sunrise pair. |
| `ghatika_for_date` | `engine`, `eop`, `utc`, `location`, `riseset_config` | `Result<GhatikaInfo, SearchError>` | Computes ghatika number (1..60) with start/end. |
| `ghatika_from_sunrises` | `jd_tdb`, `sunrise_jd`, `next_sunrise_jd`, `lsk` | `GhatikaInfo` | Pure arithmetic ghatika classification from sunrise pair. |
| `panchang_for_date` | `engine`, `eop`, `utc`, `location`, `riseset_config`, `sankranti_config`, `include_calendar` | `Result<PanchangInfo, SearchError>` | Combined one-shot daily panchang (7 limbs + optional masa/ayana/varsha). |

## Jyotish Orchestration APIs

Source: `crates/dhruv_search/src/jyotish.rs`, `crates/dhruv_search/src/jyotish_types.rs`

| Function | Inputs | Output | What it does |
|---|---|---|---|
| `graha_sidereal_longitudes` | `engine`, `jd_tdb`, `ayanamsha_system`, `use_nutation` | `Result<GrahaLongitudes, SearchError>` | Computes 9 graha sidereal longitudes (includes node handling for Rahu/Ketu). |
| `special_lagnas_for_date` | `engine`, `eop`, `utc`, `location`, `riseset_config`, `aya_config` | `Result<AllSpecialLagnas, SearchError>` | Computes all special lagnas via engine + pure math orchestration. |
| `arudha_padas_for_date` | `engine`, `eop`, `utc`, `location`, `bhava_config`, `aya_config` | `Result<[ArudhaResult; 12], SearchError>` | Computes arudha padas for all 12 houses. |
| `all_upagrahas_for_date` | `engine`, `eop`, `utc`, `location`, `riseset_config`, `aya_config` | `Result<AllUpagrahas, SearchError>` | Computes all 11 upagrahas (time-based and sun-based). |
| `graha_positions` | `engine`, `eop`, `utc`, `location`, `bhava_config`, `aya_config`, `config` | `Result<GrahaPositions, SearchError>` | Central graha position API with optional lagna/nakshatra/bhava/outer planets. |
| `ashtakavarga_for_date` | `engine`, `eop`, `utc`, `location`, `aya_config` | `Result<AshtakavargaResult, SearchError>` | Computes full ashtakavarga (BAV/SAV/sodhana) for date/location. |
| `core_bindus` | `engine`, `eop`, `utc`, `location`, `bhava_config`, `riseset_config`, `aya_config`, `config` | `Result<BindusResult, SearchError>` | Computes curated bindu points (arudha set + lagnas + gulika/maandi etc.). |
| `drishti_for_date` | `engine`, `eop`, `utc`, `location`, `bhava_config`, `riseset_config`, `aya_config`, `config` | `Result<DrishtiResult, SearchError>` | Computes graha drishti matrix and optional bhava/lagna/bindu projections. |
| `GrahaLongitudes::longitude` | `&self`, `graha` | `f64` | Reads one graha sidereal longitude from stored array. |
| `GrahaLongitudes::rashi_index` | `&self`, `graha` | `u8` | Computes 0-based rashi index for one graha. |
| `GrahaLongitudes::all_rashi_indices` | `&self` | `[u8; 9]` | Computes rashi indices for all 9 grahas. |
| `GrahaEntry::sentinel` | none | `GrahaEntry` | Returns sentinel/zeroed entry used when optional fields are not requested. |

## API Surface Included Via Re-exports

The crate root (`crates/dhruv_search/src/lib.rs`) re-exports all operational search/orchestration functions above, plus the main input/output structs and enums. This lets callers use `dhruv_search::...` directly.
