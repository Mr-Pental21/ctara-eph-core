# Amsha Parity Contract

Scope: amsha-related parity only.

This document defines the canonical amsha contract used for wrapper work across:

- C ABI
- `dhruv_rs`
- CLI
- Python
- Go
- Node
- Elixir

It is the Phase 1 acceptance reference for the amsha-first parity plan in
`~/.codex/plans/2026-03-18_amsha_wrapper_cli_rs_parity_plan.md`.

## Canonical Source

The canonical external contract is the C ABI in `crates/dhruv_ffi_c`.

Primary source files:

- `crates/dhruv_ffi_c/src/lib.rs`
- `crates/dhruv_search/src/jyotish_types.rs`
- `crates/dhruv_vedic_base/src/amsha.rs`

Current ABI constants relevant to amsha:

- `DHRUV_API_VERSION = 49`
- `DHRUV_MAX_AMSHA_REQUESTS = 40`

## Canonical Concepts

### Supported amshas

The supported amsha set is the `ALL_AMSHAS` list in `dhruv_vedic_base`:

`D1, D2, D3, D4, D5, D6, D7, D8, D9, D10, D11, D12, D15, D16, D18, D20, D21, D22, D24, D25, D27, D28, D30, D36, D40, D45, D48, D50, D54, D60, D72, D81, D108, D144`

The standard shodashavarga subset is:

`D1, D2, D3, D4, D7, D9, D10, D12, D16, D20, D24, D27, D30, D40, D45, D60`

### Variations

Variation codes are amsha-scoped. The current catalog entries are:

- `D2`: `0=default`, `1=cancer-leo-only`
- every other supported amsha currently exposes `0=default`

For `D2` default, odd signs map halves as `start,start+1` and even signs map
halves as `start+1,start`, where `start = (rashi * 2) % 12`.

Use the amsha variation discovery helpers as the authoritative source for the
valid codes, names, labels, and defaults for any given amsha.

### Request model

Canonical request/config concepts:

- `Amsha`
- `AmshaRequest`
- `AmshaChartScope`
- `AmshaSelectionConfig`

### Output model

Canonical result concepts:

- `RashiInfo`
- `AmshaEntry`
- `AmshaChart`
- `AmshaResult`

Optional `AmshaChart` sections:

- `bhava_cusps`
- `arudha_padas`
- `upagrahas`
- `sphutas`
- `special_lagnas`

Grahas and lagna are always present.

## Canonical C ABI Surface

### Direct pure-amsha transforms

- `dhruv_amsha_longitude`
- `dhruv_amsha_rashi_info`
- `dhruv_amsha_longitudes`

### Date/location-backed amsha chart orchestration

- `dhruv_amsha_chart_for_date`

### Full-kundali embedded amsha support

- `dhruv_full_kundali_config_default`
- `dhruv_full_kundali_for_date`
- `DhruvFullKundaliConfig.amsha_scope`
- `DhruvFullKundaliConfig.amsha_selection`
- `DhruvFullKundaliResult.amshas`

### Canonical amsha-related C structs

- `DhruvAmshaChartScope`
- `DhruvAmshaSelectionConfig`
- `DhruvAmshaEntry`
- `DhruvAmshaChart`
- `DhruvFullKundaliConfig`
- `DhruvFullKundaliResult`

## Validation Contract

All wrappers must preserve these semantics.

### Code validation

- Unknown amsha code: reject.
- Unknown variation code: reject.
- Selection count above `40`: reject.

### Variation validation

- Unknown variation code for the selected amsha: reject.

### Defaulting

- Missing variation means that amsha's default variation.
- Missing `AmshaChartScope` means all optional-section flags are false.

### Output guarantees

- Every returned amsha longitude is normalized to `[0, 360)`.
- Every returned `rashi_index` is in `0..=11`.
- In `AmshaChart`, `grahas` always has length `9`.
- In `AmshaChart`, `lagna` is always present.

## Full-Kundali Dependency Contract

Embedded amsha charts in `full_kundali` depend on the relevant root sections
already being computed.

Required dependencies:

- amsha charts in general require graha positions with lagna
- `amsha_scope.include_bhava_cusps` depends on `include_bhava_cusps`
- `amsha_scope.include_arudha_padas` depends on `include_bindus`
- `amsha_scope.include_upagrahas` depends on `include_upagrahas`
- `amsha_scope.include_sphutas` depends on `include_sphutas`
- `amsha_scope.include_special_lagnas` depends on `include_special_lagnas`

Wrappers may satisfy this either by:

1. exposing the raw config and documenting these dependencies clearly, or
2. auto-promoting the dependent root flags when amsha scope requests them.

Either approach is acceptable, but silent omission of requested amsha sections
is not.

## Wrapper Expectations

### `dhruv_rs`

Expected public surface:

- low-level pure helpers for:
  - single amsha longitude
  - batch amsha longitudes
  - amsha rashi info
- date-backed amsha chart helper
- root re-exports for the amsha type family
- `FullKundaliConfig` access to `amsha_selection` and `amsha_scope`

### CLI

Expected public surface:

- direct pure transform command: `amsha`
- date-backed chart command: `amsha-chart`
- `kundali` flags for:
  - enabling amshas
  - selecting amsha requests
  - selecting amsha scope
- printed output for optional amsha sections when requested and present

### Python

Expected public surface:

- `ctara_dhruv.amsha.amsha_longitude`
- `ctara_dhruv.amsha.amsha_rashi_info`
- `ctara_dhruv.amsha.amsha_longitudes`
- `ctara_dhruv.amsha.amsha_chart_for_date`
- `ctara_dhruv.kundali.full_kundali_config_default`
- `ctara_dhruv.kundali.full_kundali` with usable `amsha_selection` and `amsha_scope`
- extraction of all optional `AmshaChart` sections

### Go

Expected public surface:

- `AmshaLongitude`
- `AmshaRashiInfo`
- `AmshaLongitudes`
- `(*Engine).AmshaChartForDate`
- `FullKundaliConfig.AmshaSelection`
- `FullKundaliConfig.AmshaScope`
- extraction of all optional `AmshaChart` sections

### Node

Expected public surface:

- `amshaLongitude`
- `amshaRashiInfo`
- `amshaLongitudes`
- `amshaChartForDate`
- `fullKundaliConfigDefault`
- `fullKundaliForDate` with `amshaSelection` and `amshaScope`
- extraction of all optional `AmshaChart` sections

### Elixir

Expected public surface:

- `CtaraDhruv.Jyotish.amsha/2`
- `CtaraDhruv.Jyotish.full_kundali/2`
- request/config support for:
  - amsha request codes
  - variation codes
  - `amsha_scope`
  - `amsha_selection`
- result maps that expose all optional amsha sections when present

## Wrapper Checklist

Use this checklist as the acceptance gate for any wrapper claiming amsha parity.

### Shared checklist

- supports the canonical amsha code set from `ALL_AMSHAS`
- supports variation code `0`
- supports variation code `1`
- rejects unknown amsha codes
- rejects unknown variation codes for the selected amsha
- preserves the default variation behavior
- preserves the optional-section scope behavior

### Per-wrapper checklist

#### `dhruv_rs`

- exposes pure single-longitude amsha transform
- exposes pure batch amsha transform
- exposes amsha rashi info helper
- exposes date-backed amsha chart helper
- re-exports the amsha type family
- exposes `full_kundali` amsha selection/scope config

#### CLI

- `amsha` supports longitude-only output
- `amsha` supports rashi-info output
- `amsha` supports machine-readable batch output
- `amsha-chart` supports one or more amsha requests
- `amsha-chart` supports amsha scope flags
- `kundali` supports explicit amsha selection
- `kundali` supports explicit amsha scope

#### Python

- direct amsha helpers are present
- date-backed amsha chart helper is present
- `full_kundali` config exposes `amsha_selection`
- `full_kundali` config exposes `amsha_scope`
- optional amsha chart sections are extracted

#### Go

- direct amsha helpers are present
- date-backed amsha chart helper is present
- `FullKundaliConfig` exposes `AmshaSelection`
- `FullKundaliConfig` exposes `AmshaScope`
- optional amsha chart sections are extracted

#### Node

- direct amsha helpers are present
- date-backed amsha chart helper is present
- `fullKundaliConfigDefault()` exposes `amshaSelection`
- `fullKundaliConfigDefault()` exposes `amshaScope`
- optional amsha chart sections are extracted

#### Elixir

- dedicated amsha request path accepts caller-controlled scope
- full kundali config exposes `amsha_selection`
- full kundali config exposes `amsha_scope`
- optional amsha chart sections are present in result maps

## Notes

- This document defines expected surface and behavior, not language-specific API
  style.
- Where wrapper naming differs from the C ABI, capability parity matters more
  than exact spelling.
- This document is intentionally amsha-only; it does not define parity policy
  for unrelated API families.
