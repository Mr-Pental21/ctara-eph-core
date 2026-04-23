# Rust Library Reference

This page summarizes the intended public `dhruv_rs` surface from
`crates/dhruv_rs/src/`.

## Primary API Styles

- Explicit reusable context ownership via `DhruvContext`
- Request-based operation APIs in `ops.rs`
- Amsha helpers in `amsha.rs`

`dhruv_rs` should be used through explicit `DhruvContext` ownership rather than
global singleton state. A `DhruvContext` owns an initialized engine and is
meant to be reused across many operations, not recreated for every call.

## Context APIs

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
- `DhruvContext::time_conversion_policy`

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
- `UpagrahaRequest`
- `AvasthaTarget`, `AvasthaRequest`, `AvasthaResult`
- `FullKundaliRequest`

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
- `upagraha_op`
- `avastha_op`

High-level time-bearing search results default to structured Gregorian UTC on
their main result types while retaining numeric JD/TDB alongside UTC where the
numeric transport remains part of the public contract.

The corresponding high-level search request types use `TimeInput`, so the same
main operations accept either structured Gregorian UTC or numeric JD/TDB
without separate `*_utc` entrypoints.
- `full_kundali`

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
- `TimeConversionPolicy`
- `TimeConversionOptions`
- `Graha`
- `AyanamshaSystem`
- `NodeDignityPolicy`
- `GrahaPositionsConfig`

`BhavaConfig` defaults `use_rashi_bhava_for_bala_avastha=true`,
`include_rashi_bhava_results=true`, and
`include_node_aspects_for_drik_bala=false`. Set
`include_node_aspects_for_drik_bala=true` when Shadbala Drik Bala should
include Rahu/Ketu incoming aspects; standalone drishti matrices are unaffected.
- `BindusConfig`
- `DrishtiConfig`
- `TimeUpagrahaConfig`
- `TimeUpagrahaPoint`
- `GulikaMaandiPlanet`
- `FullKundaliConfig`
- `FullKundaliResult`
- `DashaSnapshotTime`
- `DashaTimeExt`
- `DashaSnapshotTimeExt`
- `AllUpagrahas`
- `AllGrahaAvasthas`
- `GrahaAvasthas`
- `DashaVariationConfig`
- `TaraConfig`

## Selected Direct Re-Exports

`dhruv_rs` still re-exports a selected set of lower-level helpers and result
types for Rust callers, including:

- amsha helpers such as `amsha_longitude`, `amsha_chart_for_date`, and
  `amsha_charts_for_date`
- full-kundali, shadbala, vimsopaka, and dasha result/config families
- pure jyotish math helpers such as `calculate_ashtakavarga`,
  `calculate_bhava_bala`, `calculate_bav`, `calculate_sav`, and
  `calculate_all_bav`

The standalone shadbala, vimsopaka, balas, and avastha surfaces now share
`AmshaSelectionConfig`, and embedded `full_kundali(...).amshas` returns the
resolved amsha union used by the call.

For low-level engine, time, frame, and extension-trait surfaces that are not
explicitly re-exported here, depend on the source crates directly:

- `dhruv_core`
- `dhruv_time`
- `dhruv_frames`
- `dhruv_search`
- `dhruv_vedic_base`

## Notes

- Use request/context attributes for invocation-specific inputs such as UTC vs
  JD(TDB), locations, and per-call selectors.
- Use config objects for behavior and policy knobs.
- `dhruv_rs` no longer carries public singleton or convenience-wrapper layers.
