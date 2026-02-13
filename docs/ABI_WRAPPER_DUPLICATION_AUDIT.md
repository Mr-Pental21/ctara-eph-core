# C ABI, Rust Wrapper, and CLI Duplication Audit

## Purpose

Audit high-level wrappers that trigger repeated internal calculations for the same inputs, and identify where low-level/composable APIs can reduce duplicate work.

## Scope

- C ABI surface in `crates/dhruv_ffi_c/src/lib.rs`
- Rust convenience wrappers in `crates/dhruv_rs/src/convenience.rs`
- CLI command orchestration in `crates/dhruv_cli/src/main.rs`
- Search orchestration internals in `crates/dhruv_search/src/panchang.rs`, `crates/dhruv_search/src/jyotish.rs`, and `crates/dhruv_search/src/conjunction.rs`
- Engine batching/concurrency capability in `crates/dhruv_core/src/lib.rs`

## Executive Summary

1. The panchang stack already exposes enough low-level functions to avoid repeated work, but callers can still accidentally recompute by calling high-level functions separately.
2. `drishti_for_date(... include_bindus=true)` currently recomputes several expensive intermediates inside `core_bindus`.
3. `graha_sidereal_longitudes` and conjunction internals do same-epoch per-body queries serially instead of using `Engine::query_batch`.
4. C ABI does not expose batch query APIs, which limits throughput and shared memoization benefits for Go and other FFI consumers.
5. Rust convenience wrappers have at least one clear same-input repeat conversion/query pattern (`sidereal_longitude` path).
6. For a strict stateless core goal, reusable cross-call state should live in wrappers, not in core/ABI context handles.
7. `ashtakavarga_for_date` should be treated as part of duplication analysis because typical kundali workflows call overlapping orchestration functions for the same input.

## Detailed Findings

### 1) Panchang: separate high-level calls can duplicate work

High-level per-limb APIs:

- `dhruv_tithi_for_date`: `crates/dhruv_ffi_c/src/lib.rs:5731`
- `dhruv_karana_for_date`: `crates/dhruv_ffi_c/src/lib.rs:5765`
- `dhruv_yoga_for_date`: `crates/dhruv_ffi_c/src/lib.rs:5800`
- `dhruv_vaar_for_date`: `crates/dhruv_ffi_c/src/lib.rs:5877`
- `dhruv_hora_for_date`: `crates/dhruv_ffi_c/src/lib.rs:5935`
- `dhruv_ghatika_for_date`: `crates/dhruv_ffi_c/src/lib.rs:5994`

Combined API:

- `dhruv_panchang_for_date`: `crates/dhruv_ffi_c/src/lib.rs:6087`

Rust search internals confirm duplication for separate calls and sharing in combined call:

- separate: `tithi_for_date` (`crates/dhruv_search/src/panchang.rs:324`), `karana_for_date` (`crates/dhruv_search/src/panchang.rs:367`), `yoga_for_date` (`crates/dhruv_search/src/panchang.rs:407`)
- combined shared path: `panchang_for_date` (`crates/dhruv_search/src/panchang.rs:682`)

What is duplicated when called separately:

- Moon-Sun elongation (`tithi` + `karana`)
- sidereal sum (`yoga`)
- sunrise bracket (`vaar` + `hora` + `ghatika`)

### 2) Panchang low-level/composable APIs already exist (good)

Already exposed in C ABI:

- `dhruv_elongation_at`: `crates/dhruv_ffi_c/src/lib.rs:6395`
- `dhruv_sidereal_sum_at`: `crates/dhruv_ffi_c/src/lib.rs:6424`
- `dhruv_vedic_day_sunrises`: `crates/dhruv_ffi_c/src/lib.rs:6459`
- `dhruv_tithi_at`: `crates/dhruv_ffi_c/src/lib.rs:6550`
- `dhruv_karana_at`: `crates/dhruv_ffi_c/src/lib.rs:6584`
- `dhruv_yoga_at`: `crates/dhruv_ffi_c/src/lib.rs:6617`
- `dhruv_vaar_from_sunrises`: `crates/dhruv_ffi_c/src/lib.rs:6654`
- `dhruv_hora_from_sunrises`: `crates/dhruv_ffi_c/src/lib.rs:6682`
- `dhruv_ghatika_from_sunrises`: `crates/dhruv_ffi_c/src/lib.rs:6712`
- `dhruv_nakshatra_at`: `crates/dhruv_ffi_c/src/lib.rs:8448`

Conclusion: no new ABI is required for panchang deduplication itself. Callers can already compute shared intermediates once and fan out.

### 3) Jyotish internal duplication: `drishti` + `core_bindus`

In `drishti_for_date`, these are computed:

- graha longitudes: `crates/dhruv_search/src/jyotish.rs:610`
- optional bhavas: `crates/dhruv_search/src/jyotish.rs:637`
- optional bindus path invokes `core_bindus`: `crates/dhruv_search/src/jyotish.rs:657`

Inside `core_bindus`, these are recomputed:

- graha longitudes: `crates/dhruv_search/src/jyotish.rs:475`
- bhavas: `crates/dhruv_search/src/jyotish.rs:490`
- sunrise pair: `crates/dhruv_search/src/jyotish.rs:516`
- sunset search: `crates/dhruv_search/src/jyotish.rs:522`

Impact:

- If `drishti` is called with `include_bindus=true`, expensive same-input work repeats in one request.

### 4) Same-epoch query batching opportunity not used in key paths

Engine supports same-epoch batch sharing:

- `Engine::query_batch`: `crates/dhruv_core/src/lib.rs:506`

But `graha_sidereal_longitudes` currently loops and queries one graha at a time:

- `crates/dhruv_search/src/jyotish.rs:63`
- per-graha query call through `body_ecliptic_lon_lat`: `crates/dhruv_search/src/jyotish.rs:76`
- underlying single query: `crates/dhruv_search/src/conjunction.rs:32`

Conjunction separation also fetches two bodies serially per timestamp:

- `crates/dhruv_search/src/conjunction.rs:67`
- `crates/dhruv_search/src/conjunction.rs:68`

Practical note:

- This optimization should be benchmark-gated; expected gain may be modest for small fixed-size sets.

### 5) Rust convenience wrapper repeat pattern

`sidereal_longitude` does:

- `longitude(...)` first (which already queries using UTC->TDB): `crates/dhruv_rs/src/convenience.rs:171`
- then another UTC->TDB conversion for ayanamsha time argument: `crates/dhruv_rs/src/convenience.rs:172`

The UTC conversion helper also re-fetches global engine:

- `utc_to_jd_tdb`: `crates/dhruv_rs/src/convenience.rs:29`

This is a clear candidate for one-pass helper using one `eng` + one `jd`.

### 6) C ABI UTC conversion repeats across many wrappers

`to_jd_tdb(...)` appears across many `_utc` wrappers (29 call sites total in file):

- examples in Group A: `crates/dhruv_ffi_c/src/lib.rs:4335`, `crates/dhruv_ffi_c/src/lib.rs:4387`, `crates/dhruv_ffi_c/src/lib.rs:4442`, `crates/dhruv_ffi_c/src/lib.rs:4443`
- examples in Group C: `crates/dhruv_ffi_c/src/lib.rs:5344`, `crates/dhruv_ffi_c/src/lib.rs:5372`, `crates/dhruv_ffi_c/src/lib.rs:5400`, `crates/dhruv_ffi_c/src/lib.rs:5424`, `crates/dhruv_ffi_c/src/lib.rs:5460`

`dhruv_utc_to_tdb_jd` already exists:

- `crates/dhruv_ffi_c/src/lib.rs:482`

So callers can avoid repeated conversion if they choose lower-level JD-based APIs.

### 7) CLI behavior

CLI has both separate and combined panchang commands:

- separate: tithi/karana/yoga/vaar/hora/ghatika handlers (`crates/dhruv_cli/src/main.rs:1886`, `crates/dhruv_cli/src/main.rs:1910`, `crates/dhruv_cli/src/main.rs:1933`, `crates/dhruv_cli/src/main.rs:1992`, `crates/dhruv_cli/src/main.rs:2022`, `crates/dhruv_cli/src/main.rs:2056`)
- combined: `Panchang` handler (`crates/dhruv_cli/src/main.rs:2271`) using shared `panchang_for_date` (`crates/dhruv_cli/src/main.rs:2293`)

CLI itself is not forcing duplication in one command invocation, but user workflows that call multiple commands separately will duplicate work.

### 8) Cross-function duplication in typical kundali workloads

`ashtakavarga_for_date` calls `graha_positions(...)` internally:

- `crates/dhruv_search/src/jyotish.rs:399`
- `crates/dhruv_search/src/jyotish.rs:413`

Typical consumers request multiple results for the same timestamp/location (for example graha positions, drishti, bindus, ashtakavarga, upagrahas). Without a combined kundali-level API, shared intermediates are recomputed across separate calls.

## Do We Need New ABIs?

### Not needed for panchang deduplication

Already possible using existing composable C APIs listed in Finding 2.

### Needed for high-throughput FFI and Go concurrency

New ABIs would materially help:

1. Batch query ABI (`engine query many`) to reduce cgo crossings and exploit same-epoch memoization.
2. Optional batched UTC variant (`query many with UTC inputs`) for ergonomic wrappers.
3. A combined kundali API can offer bigger practical gains than low-level batch APIs for many real integrations.

### Not recommended under stateless-core policy

- Do not add persistent shared-context handle ABIs (`*_context_new/get/free`) in core C ABI.
- Keep any reusable state/caches in language wrappers (Go/Rust/Python wrapper layers), with explicit lifecycle and memory bounds.

## Go Concurrency Implications

What already helps:

- Core engine is `Send + Sync` in Rust (`crates/dhruv_core/src/lib.rs:650`), indicating concurrency support at engine layer.

Current limitation for Go:

- Without C ABI batch APIs, Go parallelism means many separate cgo calls, which increases boundary overhead and misses cross-query sharing opportunities.

What lower-level ABIs would unlock:

- Group same-epoch queries in Go and submit one batch call.
- Precompute shared intermediates once (`elongation`, `sidereal_sum`, `sunrise pair`) and fan out derived calculations cheaply.
- Better multi-goroutine scheduling with fewer FFI call sites per request.
- Wrapper-owned per-request/per-worker caches without introducing core-library statefulness.

Profiling caveat:

- If Go integrations mostly use combined orchestration APIs (for example `dhruv_graha_positions` at `crates/dhruv_ffi_c/src/lib.rs:8008`), generic batch-query ABI may be lower priority than a `full_kundali_for_date` API.

## Revised Priority

1. Fix `drishti` + `core_bindus` duplicate compute via intra-call local intermediates.
2. Add a combined `full_kundali_for_date` API (Rust first, then C ABI/wrapper surface).
3. Apply wrapper cleanup for `sidereal_longitude`.
4. Evaluate `query_batch` migration only if benchmark gain is meaningful (for example >10% on representative workloads).
5. Add generic batch-query C ABI only if Go profiling shows cgo-call overhead is the main bottleneck.

## Risk Notes

- This audit is structural/code-level and does not yet include measured before/after perf deltas.
- Functional equivalence is expected for refactors, but benchmark and regression tests should gate rollout.
