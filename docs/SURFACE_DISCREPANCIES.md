# Surface Discrepancy Audit

## Scope and authority

This audit compares the library's authoritative Rust implementation against every public surface in the repository.

Authoritative core crates used as the reference:

- `crates/dhruv_core`
- `crates/dhruv_time`
- `crates/dhruv_frames`
- `crates/dhruv_vedic_base`
- `crates/dhruv_vedic_ops`
- `crates/dhruv_search`
- `crates/dhruv_tara`
- `crates/dhruv_rs`

Public surfaces audited:

- CLI: `crates/dhruv_cli`
- Rust crate facade: `crates/dhruv_rs/src/lib.rs`
- C ABI: `crates/dhruv_ffi_c/include/dhruv.h`
- Python: `bindings/python-open`
- Node.js: `bindings/node-open`
- Go: `bindings/go-open`
- Elixir: `bindings/elixir-open`

Other wrappers:

- `bindings/README.md` lists only `python-open`, `node-open`, `go-open`, and `elixir-open`. There are no additional wrapper directories to audit.

## Design note on request/context and `_with_*` APIs

The desired public-API pattern is one stable entry point per logical feature, where optional behavior is expressed through typed request/context and config attributes rather than separate function names.

- Existing `*_with_model`, `*_with_config`, and `*_with_inputs` symbols are treated here as current implementation details, not as the target API shape.
- Existing setter-style policy APIs such as `set_time_conversion_policy` and `set_time_policy` are also treated as current implementation details when the same value can be carried by request/context or config data.
- Existing naming and presentation variants such as `*_english_name`, `western_name`, or parallel `*_name` helper families for the same logical identifier are also treated as current implementation details when the variation can be carried by config attributes or result metadata instead of separate symbol names.
- The preferred design is to move those knobs into request/context and config objects on the main symbols. Notable examples of the intended pattern are `dhruv_search::operations::*Operation`, `dhruv_rs::ops::*Request`, and the C ABI's `DhruvTaraComputeRequest`.
- Request/context attributes should carry alternate inputs or precomputed invocation data, such as UTC vs JD inputs, `with_inputs`, or `with_moon` style variations.
- Config attributes should carry behavior, policy, and presentation knobs, such as model selection, defaults, tolerances, feature options, locale, naming style, or output-format preferences.

This audit therefore does not treat a missing `_with_*` symbol as a discrepancy by itself. The real discrepancy is when a surface cannot reach the same behavior through the primary symbol plus typed request/context and config attributes. When an area already has a request/context-based entrypoint, that is considered the preferred shape. There should be no long-term deprecation layer for unwanted public surfaces: once a request/context/config-driven replacement exists, redundant variants, setters, and naming-style helper symbols should be removed rather than kept around as aliases.

## Cross-surface discrepancies

### 1. `dhruv_rs` hides a large implemented API

- Missing or wrong:
  This discrepancy was previously driven by dormant `convenience.rs` and
  `global.rs` surfaces that were not reachable through the live facade. Those
  files have now been removed instead of being re-surfaced. The remaining
  requirement is simply to keep extending `dhruv_rs` through canonical
  request/context entrypoints and intentional re-exports, rather than adding a
  second parallel helper layer.
- Affected surfaces:
  Rust public API.
- Correct behavior:
  Keep `dhruv_rs` as a single context-first/request-based Rust facade. Add
  new high-level capabilities through `ops` entrypoints or intentional
  re-exports, not through a second singleton or convenience-wrapper layer.
- Evidence:
  `crates/dhruv_rs/src/lib.rs`, `crates/dhruv_rs/src/ops.rs`.

### 2. The Rust facade does not expose the full low-level core/query surface

- Missing or wrong:
  `dhruv_rs` currently mixes a high-level context/request facade with a partial set of low-level re-exports. Missing low-level pieces fall into several different roles:
  - runtime/engine types such as `Engine` and `LeapSecondKernel`,
  - low-level input/value types such as `Query`, `UtcTime`, and `Epoch`,
  - low-level result/error/diagnostic types such as `EngineError`, `QueryStats`, `DerivedValue`, `TimeDiagnostics`, and `TimeWarning`,
  - low-level policy/config enums such as `DeltaTModel`,
  - Rust-only extension traits such as `DerivedComputation`,
  - low-level helper APIs from `dhruv_frames`, `dhruv_search`, and `dhruv_vedic_base`.
  As a result, `dhruv_rs` is neither a clean high-level facade nor a complete umbrella crate for Rust consumers.
- Affected surfaces:
  Rust public API.
- Correct behavior:
  Keep `dhruv_rs` as the high-level Rust facade rather than turning it into a full umbrella crate. Document that low-level engine, time, frame, diagnostics, and extension-trait surfaces live in `dhruv_core`, `dhruv_time`, `dhruv_frames`, `dhruv_search`, and `dhruv_vedic_base`. Re-export low-level types or APIs from `dhruv_rs` only when they are intentionally part of the stable high-level Rust contract.
- Evidence:
  `crates/dhruv_rs/src/lib.rs`, `crates/dhruv_core/src/lib.rs`, `crates/dhruv_time/src/lib.rs`, `crates/dhruv_frames/src/lib.rs`, `crates/dhruv_search/src/lib.rs`.

### 3. Batch query, query telemetry, and derived-computation hooks are intentionally Rust-only

- Missing or wrong:
  Core exposes `Engine::query_batch`, `Engine::query_batch_with_stats`, `Engine::query_with_stats`, and `Engine::query_with_derived`, plus `QueryStats` and the `DerivedComputation` extension seam.
- Affected surfaces:
  CLI, C ABI, Python, Node.js, Go, Elixir.
- Correct behavior:
  Treat these as Rust-only low-level engine APIs. Do not treat the absence of wrapper/ABI/CLI surfaces for them as a discrepancy unless the project later decides to make them part of the cross-language contract.
- Evidence:
  `crates/dhruv_core/src/lib.rs`, `crates/dhruv_ffi_c/include/dhruv.h`.

### 4. The C ABI hard-limits engine configuration in ways the core does not

- Missing or wrong:
  Core `EngineConfig` accepts an arbitrary `Vec<PathBuf>` for `spk_paths`. The C ABI fixes this to `DHRUV_MAX_SPK_PATHS == 8` and `DHRUV_PATH_CAPACITY == 512`.
- Affected surfaces:
  C ABI, Python, Node.js, Go.
- Correct behavior:
  Treat this as an intentional ABI transport limitation unless project requirements change. The important requirement is to document it clearly as an ABI-specific limit imposed by the C transport layer rather than by core engine behavior.
- Evidence:
  `crates/dhruv_core/src/lib.rs`, `crates/dhruv_ffi_c/include/dhruv.h`.

### 5. UTC query support is weaker than the core engine

- Missing or wrong:
  This used to be split across JD-only main queries plus separate UTC helper names. The canonical shape is the main query request/context surface itself: callers should express JD-vs-UTC input and cartesian-vs-spherical output through request attributes, not suffixed helper entrypoints.
- Affected surfaces:
  C ABI, Python, Node.js, Go.
- Correct behavior:
  Expose the full UTC-query behavior through the main query request/context shape in the ABI and wrappers, so callers can select UTC vs JD inputs and cartesian vs spherical outputs without separate suffixed entry points. Any remaining split helper names should be removed rather than kept alongside the unified query surface.
- Evidence:
  `crates/dhruv_core/src/lib.rs`, `crates/dhruv_ffi_c/include/dhruv.h`, `bindings/python-open/src/ctara_dhruv/ephemeris.py`, `bindings/node-open/src/engine.js`, `bindings/go-open/dhruv/engine.go`.

### 6. Time diagnostics and most time-policy controls stop at the Rust crates

- Status:
  Resolved for the shared UTC-conversion contract.
- Current behavior:
  The C ABI now exposes one typed UTC-conversion request/result transport carrying `TimeConversionPolicy`, `TimeConversionOptions`, diagnostics, warnings, and the model-selection enums needed for fallback behavior. Python, Node.js, and Go consume that same shape. The SMH reconstruction parse/install helpers remain internal, and `dhruv_time` auto-loads the bundled reconstruction asset when the SMH model is selected.
- Affected surfaces:
  C ABI, Python, Node.js, Go.
- Evidence:
  `crates/dhruv_time/src/lib.rs`, `crates/dhruv_time/src/scales.rs`, `crates/dhruv_ffi_c/include/dhruv.h`.

### 7. CLI and Elixir expose only part of the time-policy surface

- Status:
  Resolved for the current CLI and Elixir time surfaces.
- Current behavior:
  CLI flags now carry the full `TimeConversionOptions` shape, including `warn_on_fallback` and `pre_range_dut1`, while retaining normal flag-plus-config layering. Elixir now carries time policy through engine/config data and optional `Time.utc_to_jd_tdb/2` request attributes, returns diagnostics on UTC conversion, and no longer exposes a separate mutable `engine_set_time_policy` public surface.
- Affected surfaces:
  CLI, Elixir.
- Evidence:
  `crates/dhruv_cli/src/main.rs`, `bindings/elixir-open/native/dhruv_elixir_nif/src/lib.rs`, `crates/dhruv_time/src/scales.rs`.

### 8. Frame/precession/invariable-plane options are not carried consistently through public configs

- Missing or wrong:
  The real gap is not the absence of standalone helper exports. The gap is that frame/precession/invariable-plane choices are not carried consistently through the config/request shapes of the higher-level features that depend on them:
  - model-selection concepts such as `PrecessionModel`,
  - reference-plane choices including invariable-plane variants,
  - related precession/frame-selection knobs used by higher-level longitude, position, or ayanamsha flows.
  Outside Rust, only a very small subset of these choices is broadly configurable today, and some surfaces still expose them only through narrow special cases such as the CLI's tropical precession path.
- Affected surfaces:
  CLI, C ABI, Python, Node.js, Go, Elixir.
- Correct behavior:
  Do not add standalone public helper APIs for these low-level transforms. Instead, ensure every higher-level public surface that uses frame/precession/invariable-plane behavior carries the necessary options through typed config/request attributes.
- Evidence:
  `crates/dhruv_frames/src/lib.rs`, `crates/dhruv_ffi_c/include/dhruv.h`, `crates/dhruv_cli/src/main.rs`.

### 9. Longitude-only APIs should stay unified under config attributes

- Current status:
  The primary longitude-only surface is now the single `graha_longitudes` API family with typed config across Rust, C ABI, Python, Node.js, Go, Elixir, and CLI. Sidereal vs tropical/reference-plane output, ayanamsha choice, nutation, precession model, and reference-plane selection are all carried through config attributes rather than separate symbol names.
- Affected surfaces:
  Rust public API, C ABI, Python, Node.js, Go, Elixir, CLI.
- Required behavior:
  Keep this surface unified. Do not reintroduce public `*_with_model`, sidereal-only, or tropical-only variants for longitude-only queries; new variations belong in the shared config shape.
- Evidence:
  `crates/dhruv_search/src/jyotish.rs`, `crates/dhruv_search/src/jyotish_types.rs`, `crates/dhruv_ffi_c/include/dhruv.h`, `crates/dhruv_cli/src/main.rs`, `bindings/go-open/dhruv/jyotish.go`, `bindings/node-open/src/jyotish.js`, `bindings/python-open/src/ctara_dhruv/kundali.py`, `bindings/elixir-open/native/dhruv_elixir_nif/src/lib.rs`.

### 10. Search range APIs can truncate outside Rust

- Status:
  Resolved for the current public wrappers.
- Affected surfaces:
  C ABI, Python, Node.js, Go.
- Current behavior:
  The C ABI remains a fixed-buffer transport, but Python, Node.js, and Go now auto-expand internal range buffers until the full result set fits. Small initial capacities no longer truncate the public wrapper result set.
- Evidence:
  `crates/dhruv_search/src/lib.rs`, `crates/dhruv_ffi_c/include/dhruv.h`, `bindings/node-open/src/search.js`, `bindings/python-open/src/ctara_dhruv/search.py`, `bindings/go-open/dhruv/search.go`.

### 11. Input-based dasha constructors remain Rust-only

- Missing or wrong:
  Resolved for the current public dasha surfaces.
- Affected surfaces:
  Rust public API, CLI, C ABI, Python, Node.js, Go, Elixir.
- Correct behavior:
  Keep one request/context-driven dasha surface per feature. Alternate invocation data such as precomputed Moon longitude, rashi inputs, sunrise/sunset, birth JD, and query JD should flow through typed birth/input request attributes rather than parallel `_with_*` or `_utc` public symbols.
- Evidence:
  `crates/dhruv_search/src/dasha.rs`, `crates/dhruv_search/src/lib.rs`, `crates/dhruv_vedic_ops/src/lib.rs`, `crates/dhruv_cli/src/main.rs`, `crates/dhruv_ffi_c/include/dhruv.h`, `bindings/python-open/src/ctara_dhruv/dasha.py`, `bindings/node-open/src/dasha.js`, `bindings/go-open/dhruv/dasha.go`, `bindings/elixir-open/native/dhruv_elixir_nif/src/lib.rs`.

### 12. Most low-level graha relationship and combustion math is Rust-only, and some naming variants are still split into parallel APIs

- Missing or wrong:
  This area has two separate issues:
  - `dhruv_vedic_base` publicly exposes `combustion`, `graha_relationships`, dignity helpers, benefic/malefic classification, lord lookups, and related primitives. Outside Rust, only a very small subset is surfaced (`rashi_lord`, `hora_at`, and a few helper lookups).
  - Some naming and presentation helpers are still modeled as parallel APIs or methods for the same logical identifier surface, such as separate Sanskrit/English/western name helpers. Those variations should not become permanent parallel public surfaces.
- Affected surfaces:
  CLI, C ABI, Python, Node.js, Go, Elixir.
- Correct behavior:
  The relationship, combustion, dignity, and classification primitives are intended public library features and should be available across the supported public surfaces. Expose dedicated wrapper/ABI/CLI APIs for them instead of forcing downstream users onto Rust crates. For naming or presentation differences, use one identifier/lookup surface and carry locale/style/output preferences through config attributes or result metadata instead of keeping separate `*_english_name`-style APIs.
- Evidence:
  `crates/dhruv_vedic_base/src/lib.rs`, `crates/dhruv_vedic_base/src/graha_relationships.rs`, `crates/dhruv_vedic_base/src/graha.rs`, `crates/dhruv_vedic_base/src/rashi.rs`, `crates/dhruv_vedic_base/src/vaar.rs`, `crates/dhruv_ffi_c/include/dhruv.h`, `bindings/python-open/src/ctara_dhruv/vedic.py`, `bindings/go-open/internal/cabi/extras.go`.

## CLI-specific discrepancies

### 13. The CLI intentionally has no surface for batch/stats/derived-query features

- Missing or wrong:
  There is no command for `query_batch`, `query_with_stats`, or `query_with_derived`.
- Affected surfaces:
  CLI.
- Correct behavior:
  Keep these Rust-only unless the project explicitly decides that low-level engine telemetry or derived-query hooks belong in the CLI contract.
- Evidence:
  `crates/dhruv_core/src/lib.rs`, `crates/dhruv_cli/src/main.rs`.

### 14. The CLI does not expose most low-level time/frame APIs

- Missing or wrong:
  There are no commands for `TimeDiagnostics`, `CalendarPolicy`, `Epoch`, `month_from_abbrev`, invariable-plane transforms, or raw precession/obliquity helpers.
- Affected surfaces:
  CLI.
- Correct behavior:
  If the CLI is expected to mirror more of the broader library, carry these capabilities through coherent diagnostic/utility command surfaces with typed flags/config rather than a pile of one-off helper commands. Otherwise explicitly scope the CLI to end-user workflows only.
- Evidence:
  `crates/dhruv_time/src/lib.rs`, `crates/dhruv_frames/src/lib.rs`, `crates/dhruv_cli/src/main.rs`.

## Python-specific discrepancies

### 15. The package root exports only a small subset of the implemented Python binding

- Status:
  Resolved by explicit documentation of the intended root surface.
- Affected surfaces:
  Python bindings.
- Current behavior:
  `ctara_dhruv.__init__` is intentionally a compact convenience surface. The fuller public API is module-based, and the end-user docs now describe that explicitly instead of implying the root is the whole binding.
- Evidence:
  `bindings/python-open/src/ctara_dhruv/__init__.py`, `docs/end_user/python/reference.md`.

### 16. The Python wrapper has no access to time-policy configuration

- Status:
  Resolved for the UTC-conversion surface.
- Current behavior:
  Python exposes typed `UtcToTdbRequest`, `UtcToTdbResult`, `TimePolicy`, `TimeConversionOptions`, `TimeDiagnostics`, and `TimeWarning` through `ctara_dhruv.time.utc_to_jd_tdb(...)`.
- Affected surfaces:
  Python bindings.
- Evidence:
  `bindings/python-open/src/ctara_dhruv/engine.py`, `bindings/python-open/src/ctara_dhruv/time.py`, `crates/dhruv_time/src/scales.rs`.

### 17. Python lacks a bidirectional specific-sankranti search shape

- Status:
  Resolved for the current Python bindings.
- Affected surfaces:
  Python bindings.
- Current behavior:
  Python now exposes one `specific_sankranti(...)` helper with a direction-bearing argument instead of a one-off `next_specific_sankranti` helper.
- Evidence:
  `bindings/python-open/src/ctara_dhruv/search.py`, `crates/dhruv_ffi_c/include/dhruv.h`.

### 18. Python range-search helpers can silently truncate results

- Status:
  Resolved for the current Python bindings.
- Affected surfaces:
  Python bindings.
- Current behavior:
  Python range-search helpers auto-expand their internal buffers until the full result set is returned. `max_results` is now only the initial internal chunk size.
- Evidence:
  `bindings/python-open/src/ctara_dhruv/search.py`, `crates/dhruv_search/src/lib.rs`.

## Node.js-specific discrepancies

### 19. The Node wrapper has no time-policy or diagnostics surface

- Status:
  Resolved for the UTC-conversion surface.
- Current behavior:
  Node exposes typed `timePolicy` request attributes, policy/model constants, and UTC-conversion diagnostics through `utcToTdbJd(...)`.
- Affected surfaces:
  Node.js bindings.
- Evidence:
  `bindings/node-open/src/time.js`, `bindings/node-open/src/engine.js`, `crates/dhruv_time/src/scales.rs`.

### 20. Node range-search defaults are too small and can truncate

- Status:
  Resolved for the current Node.js bindings.
- Affected surfaces:
  Node.js bindings.
- Current behavior:
  Node range-search calls auto-expand internal buffers until the full result set is returned. The optional third argument is now only the initial internal chunk size.
- Evidence:
  `bindings/node-open/src/search.js`, `crates/dhruv_search/src/lib.rs`.

## Go-specific discrepancies

### 21. Go config loading cannot use discovery mode or choose `DefaultsMode`

- Status:
  Resolved for the current Go bindings.
- Affected surfaces:
  Go bindings.
- Current behavior:
  Go now exposes a typed `ConfigLoadOptions` request with nullable `Path` and explicit `DefaultsMode`, matching the main C ABI config-loading contract instead of trapping discovery/defaults selection behind a path-only helper.
- Evidence:
  `crates/dhruv_ffi_c/include/dhruv.h`, `bindings/go-open/dhruv/engine.go`, `bindings/go-open/internal/cabi/cabi.go`.

### 22. The Go wrapper has no time-policy or diagnostics surface

- Status:
  Resolved for the UTC-conversion surface.
- Current behavior:
  Go now exposes typed `UtcToTdbRequest`, `UtcToTdbResult`, `TimePolicy`, `TimeConversionOptions`, and `TimeDiagnostics` through `UTCToTdbJD(...)`.
- Affected surfaces:
  Go bindings.
- Evidence:
  `bindings/go-open/dhruv/time.go`, `crates/dhruv_time/src/scales.rs`.

### 23. Go range-search wrappers can truncate without signaling it

- Status:
  Resolved for the current Go bindings.
- Affected surfaces:
  Go bindings.
- Current behavior:
  Go range-search methods now auto-expand their internal buffers until the full result set is returned. The optional final argument is now only the initial internal chunk size.
- Evidence:
  `bindings/go-open/dhruv/search.go`, `crates/dhruv_ffi_c/include/dhruv.h`.

## Elixir-specific discrepancies

### 24. Elixir injects ad-hoc engine defaults that are not defined by the core

- Missing or wrong:
  The Elixir engine constructor diverges in two ways:
  - it injects implicit defaults even though core `EngineConfig` has no default for those fields,
  - the chosen values (`cache_capacity = 64`, `strict_validation = false`) differ from the established wrapper/CLI convention of `256` and `true`.
  This creates wrapper-specific behavior that is not anchored in the core contract.
- Affected surfaces:
  Elixir bindings.
- Correct behavior:
  Either require both fields explicitly, or align the implicit defaults with the established wrapper behavior. This point is an inference from surrounding surfaces because core itself does not define defaults.
- Evidence:
  `bindings/elixir-open/native/dhruv_elixir_nif/src/lib.rs`, `bindings/python-open/src/ctara_dhruv/engine.py`, `crates/dhruv_cli/src/main.rs`.

### 25. Elixir config loading cannot use discovery mode or choose `DefaultsMode`

- Status:
  Resolved for the current Elixir bindings.
- Affected surfaces:
  Elixir bindings.
- Current behavior:
  Elixir `load_config/2` now accepts the same main request shape as the shared loader contract: optional `path` plus explicit `defaults_mode`. Discovery mode is reachable when `path` is omitted, and the string-path form remains only as a small convenience wrapper.
- Evidence:
  `bindings/elixir-open/native/dhruv_elixir_nif/src/lib.rs`, `crates/dhruv_cli/src/main.rs`, `bindings/python-open/src/ctara_dhruv/engine.py`, `bindings/node-open/src/engine.js`.

### 26. Elixir only exposes a partial time-policy shape

- Status:
  Resolved for the current Elixir time/config surface.
- Current behavior:
  Elixir accepts the full time-policy shape through engine config and optional `Time.utc_to_jd_tdb/2` request attributes, returns diagnostics on UTC conversion, and no longer keeps a standalone setter-style policy API.
- Affected surfaces:
  Elixir bindings.
- Evidence:
  `bindings/elixir-open/native/dhruv_elixir_nif/src/lib.rs`, `crates/dhruv_time/src/scales.rs`.

### 27. Elixir time helpers are much thinner than the implemented library

- Missing or wrong:
  Elixir time currently exposes only a very small subset:
  - basic conversions: `utc_to_jd_tdb`, `jd_tdb_to_utc`,
  - one frame/time helper: `nutation`.
  It does not expose:
  - system/default metadata such as ayanamsha system count or reference-plane defaults,
  - lunar-node and local-noon helpers,
  - delta-T and reconstruction helpers,
  - diagnostics or time-policy result objects.
- Affected surfaces:
  Elixir bindings.
- Correct behavior:
  Expand `CtaraDhruv.Time` if it is intended to represent the library's time surface.
- Evidence:
  `bindings/elixir-open/lib/ctara_dhruv/time.ex`, `bindings/elixir-open/native/dhruv_elixir_nif/src/lib.rs`, `crates/dhruv_time/src/lib.rs`.

### 28. Elixir omits the composable panchang/intermediate APIs

- Missing or wrong:
  The missing Elixir panchang surface includes several composable intermediate layers:
  - low-level astronomical intermediates such as `elongation_at`, `sidereal_sum_at`, `vedic_day_sunrises`, and `body_ecliptic_lon_lat`,
  - direct calendar-factor evaluators such as `tithi_at`, `karana_at`, `yoga_at`, and `nakshatra_at`,
  - sunrise-derived helpers such as `vaar_from_sunrises`, `hora_from_sunrises`, and `ghatika_from_sunrises`,
  - elapsed-time helpers such as `ghatika_from_elapsed` and `ghatikas_since_sunrise`.
- Affected surfaces:
  Elixir bindings.
- Correct behavior:
  Expose the same composable intermediate APIs that Python, Node, Go, CLI, and the Rust crates already support.
- Evidence:
  `bindings/elixir-open/lib/ctara_dhruv/panchang.ex`, `bindings/elixir-open/native/dhruv_elixir_nif/src/lib.rs`, `crates/dhruv_ffi_c/include/dhruv.h`.

### 29. Elixir omits most pure-math helper APIs

- Missing or wrong:
  The missing Elixir pure-math helper surface spans several utility families:
  - formatting/classification helpers such as DMS conversion and rashi/nakshatra classifiers,
  - lookup helpers such as name lookups,
  - formula helpers such as sphuta and special-lagna calculations,
  - upagraha and Ashtakavarga pure-math helpers,
  - low-level graha-drishti primitives.
- Affected surfaces:
  Elixir bindings.
- Correct behavior:
  Add a utility module mirroring the low-level helper surface already exposed in the C ABI and other bindings.
- Evidence:
  `bindings/elixir-open/lib/ctara_dhruv/*.ex`, `bindings/elixir-open/native/dhruv_elixir_nif/src/lib.rs`, `crates/dhruv_ffi_c/include/dhruv.h`.

### 30. Elixir jyotish longitude behavior should remain config-driven

- Current status:
  Elixir now uses the shared `graha_longitudes` surface and carries longitude kind through the request/config path. The NIF transport also carries `precession_model`, so direct longitude calls and higher-level jyotish requests can select non-default model behavior without separate symbol names.
- Affected surfaces:
  Elixir bindings.
- Required behavior:
  Keep the Elixir longitude surface aligned with the shared Rust/C ABI contract. Do not reintroduce separate tropical or model-specific public names; future variations belong in the same request/config transport.
- Evidence:
  `bindings/elixir-open/lib/ctara_dhruv/jyotish.ex`, `bindings/elixir-open/native/dhruv_elixir_nif/src/lib.rs`, `crates/dhruv_search/src/jyotish.rs`.

### 31. Elixir dasha coverage stops at hierarchy and snapshot

- Missing or wrong:
  Elixir exposes only `hierarchy/2` and `snapshot/2`. It does not expose `level0`, `level0_entity`, `children`, `child_period`, or `complete_level`, even though those are available in other wrappers and the C ABI.
- Affected surfaces:
  Elixir bindings.
- Correct behavior:
  Add the low-tier dasha APIs so Elixir can inspect and paginate dasha trees incrementally instead of always materializing the full hierarchy.
- Evidence:
  `bindings/elixir-open/lib/ctara_dhruv/dasha.ex`, `crates/dhruv_ffi_c/include/dhruv.h`.

## Tara-specific wrapper discrepancies

### 32. Tara already uses a request/config pattern; the real remaining gap is the missing low-level primitives

- Missing or wrong:
  This area mixes one intentional non-gap and one real gap:
  - intentional non-gap: non-Rust surfaces do not expose `position_equatorial_with_config`, `position_ecliptic_with_config`, or `sidereal_longitude_with_config` as separate symbols because request/config-driven replacements already exist through `dhruv_search::TaraOperation`, `dhruv_rs::ops::TaraRequest`, and the C ABI's `DhruvTaraComputeRequest`,
  - real gap: lower-level primitives such as `propagate_position`, `apply_aberration`, `apply_light_deflection`, and `galactic_anticenter_icrs` remain Rust-only.
  The important distinction is between redundant named variants, which should not be added, and genuinely missing low-level building blocks.
- Affected surfaces:
  C ABI, Python, Node.js, Go, Elixir, CLI.
- Correct behavior:
  Do not count the absence of one-for-one tara `*_with_config` symbols as a discrepancy when `tara_compute_ex`/`TaraOperation` is already present, because the single request/config-driven entrypoint is the preferred shape. If low-level star propagation or correction composition is intended to be public outside Rust, add dedicated wrappers for the remaining primitives.
- Evidence:
  `crates/dhruv_tara/src/lib.rs`, `crates/dhruv_search/src/operations.rs`, `crates/dhruv_rs/src/ops.rs`, `crates/dhruv_ffi_c/include/dhruv.h`, `bindings/python-open/src/ctara_dhruv/tara.py`, `bindings/node-open/src/tara.js`, `bindings/go-open/dhruv/tara.go`, `bindings/elixir-open/native/dhruv_elixir_nif/src/lib.rs`, `crates/dhruv_cli/src/main.rs`.

## Summary

The main structural choke points are:

- `dhruv_rs` hides much of its own intended high-level API.
- The C ABI is broad for assembled jyotish/search workflows, but narrow for low-level engine, time, frame, and tara primitives.
- Python's package root is much smaller than the implemented binding behind it.
- Node and Go are close to the C ABI, but inherit its missing time/frame surfaces and range-search truncation risk.
- Elixir diverges the most: partial time-policy support, thinner panchang/time/jyotish coverage, and wrapper-specific defaults that do not come from core behavior.
