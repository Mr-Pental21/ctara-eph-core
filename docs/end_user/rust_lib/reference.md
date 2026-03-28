# Rust Library Reference

This page summarizes the public `dhruv_rs` surface from code in
`crates/dhruv_rs/src/`.

## Primary API Styles

- Context-first APIs via `DhruvContext`
- Request-based operation APIs in `ops.rs`
- Convenience helpers in `convenience.rs`
- Amsha helpers in `amsha.rs`

## Context APIs

`dhruv_rs` should be used through explicit `DhruvContext` ownership rather than global singleton helpers. A `DhruvContext` owns an initialized engine and is meant to be reused across many operations, not recreated for every call. Dormant `global.rs` helpers are not part of the intended public surface and should be removed rather than used for new code.

Core public types:

- `DhruvContext`
- `UtcDate`
- `DhruvError`

`DhruvContext` methods:

- `DhruvContext::new`
- `DhruvContext::with_resolver`
- `DhruvContext::engine`
- `DhruvContext::resolver`
- `DhruvContext::set_resolver`
- `DhruvContext::set_time_conversion_policy`

## Request-Based Ops API

Public request/query types in `ops.rs`:

- `TimeInput`
- `ConjunctionRequestQuery`, `ConjunctionRequest`
- `GrahanRequestQuery`, `GrahanRequest`
- `MotionRequestQuery`, `MotionRequest`
- `LunarPhaseRequestQuery`, `LunarPhaseRequest`
- `SankrantiRequestQuery`, `SankrantiRequest`
- `AyanamshaRequestMode`, `AyanamshaRequest`
- `NodeRequest`
- `PanchangRequest`
- `TaraRequest`
- `CharakarakaRequest`

Request-driven functions:

- `conjunction`
- `grahan`
- `motion`
- `lunar_phase`
- `sankranti`
- `ayanamsha_op`
- `lunar_node_op`
- `panchang_op`
- `tara_op`
- `charakaraka`

## Common Public Types And Configs

Frequently used config and result families re-exported from `dhruv_rs::*`:

- `EngineConfig`
- `GeoLocation`
- `EopKernel`
- `RiseSetConfig`
- `BhavaConfig`
- `SankrantiConfig`
- `ConjunctionConfig`
- `GrahanConfig`
- `StationaryConfig`
- `GrahaPositionsConfig`
- `BindusConfig`
- `DrishtiConfig`
- `TimeUpagrahaConfig`
- `TimeUpagrahaPoint`
- `GulikaMaandiPlanet`
- `AmshaChartScope`
- `AmshaSelectionConfig`
- `FullKundaliConfig`
- `DashaSelectionConfig`
- `DashaVariationConfig`
- `TaraConfig`

## Convenience Function Inventory

Ephemeris and longitude helpers:

- `position`
- `position_full`
- `query`
- `query_batch`
- `longitude`
- `sidereal_longitude`
- `sidereal_longitude_on_plane`
- `body_ecliptic_lon_lat`

Search convenience functions:

- `next_purnima`
- `prev_purnima`
- `next_amavasya`
- `prev_amavasya`
- `search_purnimas`
- `search_amavasyas`
- `next_sankranti`
- `prev_sankranti`
- `search_sankrantis`
- `next_specific_sankranti`
- `prev_specific_sankranti`
- `next_conjunction`
- `prev_conjunction`
- `search_conjunctions`
- `next_chandra_grahan`
- `prev_chandra_grahan`
- `search_chandra_grahan`
- `next_surya_grahan`
- `prev_surya_grahan`
- `search_surya_grahan`
- `next_stationary`
- `prev_stationary`
- `search_stationary`
- `next_max_speed`
- `prev_max_speed`
- `search_max_speed`

Ayanamsha, nutation, and lunar node:

- `ayanamsha`
- `ayanamsha_with_catalog`
- `nutation`
- `lunar_node`

Rashi, nakshatra, and classifier helpers:

- `rashi`
- `nakshatra`
- `nakshatra28`
- `tithi_from_elongation`
- `karana_from_elongation`
- `yoga_from_sum`
- `vaar_from_jd`
- `masa_from_rashi_index`
- `ayana_from_sidereal_longitude`
- `samvatsara_from_year`
- `nth_rashi_from`
- `rashi_lord`
- `hora_at`
- `normalize_360`

Rise/set, lagna, bhava, and panchang:

- `sunrise`
- `sunset`
- `all_rise_set_events`
- `bhavas`
- `lagna`
- `mc`
- `ramc`
- `panchang`
- `tithi`
- `karana`
- `yoga`
- `moon_nakshatra`
- `vaar`
- `hora`
- `ghatika`
- `masa`
- `ayana`
- `varsha`
- `elongation_at`
- `sidereal_sum_at`
- `vedic_day_sunrises`
- `tithi_at`
- `karana_at`
- `yoga_at`
- `nakshatra_at`
- `ghatikas_since_sunrise`

Jyotish and chart helpers:

- `graha_longitudes`
- `graha_tropical_longitudes`
- `graha_positions`
- `special_lagnas`
- `arudha_padas`
- `upagrahas`
- `upagrahas_with_config`
- `core_bindus`
- `drishti`
- `ashtakavarga`
- `full_kundali`

Strength and state:

- `shadbala`
- `shadbala_for_graha`
- `vimsopaka`
- `vimsopaka_for_graha`
- `avastha`
- `avastha_for_graha`
- `dasha_hierarchy`
- `dasha_snapshot`

`DashaEntity::name()` and `DashaPeriod::entity_name()` return the exact
canonical Sanskrit entity names.

Amsha helpers in `convenience.rs` and `amsha.rs`:

- `amsha_chart`
- `amsha_chart_for_date`
- `amsha_charts`
- `amsha_charts_for_date`
- `chart`
- `chart_for_date`
- `charts`
- `charts_for_date`
- `amsha_longitude`
- `amsha_longitudes`
- `amsha_rashi_info`

Pure sphuta and special-lagna math:

- `sphutas`
- `bhrigu_bindu`
- `prana_sphuta`
- `deha_sphuta`
- `mrityu_sphuta`
- `tithi_sphuta`
- `yoga_sphuta`
- `yoga_sphuta_normalized`
- `rahu_tithi_sphuta`
- `kshetra_sphuta`
- `beeja_sphuta`
- `trisphuta`
- `chatussphuta`
- `panchasphuta`
- `sookshma_trisphuta`
- `avayoga_sphuta`
- `kunda`
- `bhava_lagna`
- `hora_lagna`
- `ghati_lagna`
- `vighati_lagna`
- `varnada_lagna`
- `sree_lagna`
- `pranapada_lagna`
- `indu_lagna`
- `arudha_pada`
- `sun_based_upagrahas`

Pure ashtakavarga and drishti helpers:

- `calculate_ashtakavarga`
- `calculate_bav`
- `calculate_all_bav`
- `calculate_sav`
- `trikona_sodhana`
- `ekadhipatya_sodhana`
- `graha_drishti`
- `graha_drishti_matrix`

Tara helpers:

- `tara_position_equatorial`
- `tara_position_ecliptic`
- `tara_sidereal_longitude`
- `tara_position_equatorial_with_config`
- `tara_position_ecliptic_with_config`
- `tara_sidereal_longitude_with_config`

## Public Defaults Worth Knowing

- `upagrahas(...)` uses `TimeUpagrahaConfig::default()`
- `TimeUpagrahaConfig::default()` keeps:
  - Gulika = Rahu + Start
  - Maandi = Rahu + End
  - other time-based upagrahas = Start
- `core_bindus(...)` accepts `BindusConfig` and returns `BindusResult`
- `full_kundali(...)` behavior depends on `FullKundaliConfig` include flags and nested config structs

For longer surface detail, use [`docs/rust_wrapper.md`](../../rust_wrapper.md).
