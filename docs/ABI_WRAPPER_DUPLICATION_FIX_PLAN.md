# C ABI and Wrapper Deduplication Fix Plan

## Goal

Reduce repeated calculations across C ABI, Rust wrappers, and orchestration layers by:

- reusing same-input intermediates once per request
- using lower-level APIs where they already exist
- adding new ABIs only where profiling justifies them

## Non-Goals

- No algorithmic behavior changes.
- No licensing/dependency changes.
- No breaking C ABI removals in this plan.
- No persistent cross-call state inside core library or C ABI.

## Stateless Policy Constraints

- Keep core and C ABI request/response style (stateless across calls).
- Allow only intra-call local reuse (local variables/helpers) for deduplication.
- Place reusable cross-call state (caches, pools, memo maps) in wrappers only.

## Baseline and Gating

Collect perf baselines first so improvements are measurable.

1. Benchmark `drishti(include_bindus=true)` current path.
2. Benchmark same-input multi-call kundali workloads (graha positions + ashtakavarga + drishti + bindus + upagrahas).
3. Benchmark `sidereal_longitude` convenience path.
4. Benchmark `query_batch` candidate paths before changing them.
5. Profile Go workloads to confirm whether cgo crossing overhead is a primary bottleneck.

Suggested benchmark locations:

- `crates/dhruv_search/benches/*`
- `crates/dhruv_ffi_c/benches/ffi_bench.rs`

## Workstream A: Core Refactors (Stateless, Highest Priority)

### A1. Fix `drishti` + `core_bindus` duplicate compute with local `JyotishContext`

Use a function-local precomputed intermediates struct to avoid threading many parameters and to avoid recomputation in one request path.

```rust
struct JyotishContext {
    jd_tdb: f64,
    jd_utc: f64,
    ayanamsha: f64,
    graha_lons: [f64; 9],
    bhava_result: Option<BhavaResult>,
    sunrise_pair: Option<(f64, f64)>,
    sunset_jd: Option<f64>,
}
```

Implementation notes:

- Build `JyotishContext` once inside `drishti_for_date` and reuse it for bindus computation.
- Keep it demand-driven: compute `bhava`, `sunrise`, `sunset` only when feature flags require them.
- Keep public function signatures unchanged.
- Do not persist this struct across calls.

Target functions:

- `core_bindus`: `crates/dhruv_search/src/jyotish.rs:451`
- `drishti_for_date`: `crates/dhruv_search/src/jyotish.rs:594`

### A2. Add a combined `full_kundali_for_date` API

Introduce one high-level orchestration entrypoint that computes common kundali outputs once and returns a combined result.

Initial scope:

- graha positions
- bindus
- drishti
- ashtakavarga
- upagrahas
- optional flags for expensive optional sections

Design constraints:

- One-shot stateless call only.
- No context handles.
- Reuse local intermediates internally (`JyotishContext`).

Rollout shape:

1. Rust API in `dhruv_search` (for internal sharing and wrapper adoption).
2. C ABI export (for Go and other FFI consumers).
3. `dhruv_rs` convenience wrapper.

### A3. Clean up Rust convenience repeat conversion path

Target:

- `sidereal_longitude`: `crates/dhruv_rs/src/convenience.rs:164`

Plan:

- use one engine acquisition + one JD conversion in that path
- avoid repeated conversion/query plumbing from helper chaining

### A4. Evaluate `query_batch` migration only if benchmark gain is material

Targets:

- `graha_sidereal_longitudes`: `crates/dhruv_search/src/jyotish.rs:52`
- conjunction separation fetch path: `crates/dhruv_search/src/conjunction.rs:60`

Rule:

- proceed only if representative benchmark gain is meaningful (for example >10%)
- otherwise defer for complexity/maintenance balance

## Workstream B: ABI Additions (Profiling-Gated)

### B1. Generic batch query C ABI (optional)

Proposed additive ABI:

- `dhruv_engine_query_batch(engine, queries, query_count, out_states, out_statuses)`
- optional: `dhruv_engine_query_batch_with_stats(...)`

Proceed only when:

- Go profiling shows call-boundary overhead is material
- combined orchestration APIs do not cover key workload patterns

### B2. UTC batch query C ABI (optional)

Proposed additive ABI:

- `dhruv_query_batch_utc(engine, query_utc_items, count, out_states, out_statuses)`

Proceed only when:

- consumers need low-level flexibility with UTC inputs
- and B1 evidence already justifies batch ABI direction

## Workstream C: Wrapper-Level State and Guidance (Lower Priority)

Wrapper policy:

- keep reusable caches in wrapper-owned structs (worker-scoped or request-scoped)
- bound memory explicitly
- make lifecycle explicit in wrapper APIs

Usage guidance:

- prefer `panchang_for_date` over separate panchang limb calls
- prefer `full_kundali_for_date` once available over many separate kundali orchestration calls

## Go Concurrency Plan

### Immediate with current ABI

1. Reuse engine/eop/lsk handles across goroutines.
2. Convert UTC to JD once when multiple JD-based functions are needed.
3. Prefer combined APIs over many small calls.
4. Keep caches/memo state in wrapper-owned structs, not core ABI.

### After `full_kundali_for_date`

1. Use one call per kundali workload unit where possible.
2. Use goroutine parallelism at request level, not micro-call level.
3. Reduce cgo crossings and duplicated intermediate computation naturally.

## Revised Priority

1. A1: `JyotishContext` local refactor for `drishti` + `core_bindus`.
2. A2: `full_kundali_for_date` combined API.
3. A3: `sidereal_longitude` convenience cleanup.
4. A4: `query_batch` migration only if benchmarked gain is meaningful.
5. B1/B2: batch-query ABIs only if Go profiling confirms boundary overhead bottleneck.
6. C: wrapper docs/guidance and wrapper-owned caching policy hardening.

## Verification Matrix

For each changed function/API:

1. Golden-output equivalence tests against previous behavior.
2. Null-pointer and boundary checks for new C APIs.
3. Benchmarks showing reduced duplicated computation and/or latency.
4. Concurrency stress tests for shared engine handle usage.

## Compatibility and Release Notes

- All proposed ABI additions are additive.
- Existing symbols remain unchanged.
- Increment `DHRUV_API_VERSION` only when introducing new public C symbols.
- Update docs:
  - `docs/C_ABI_REFERENCE.md`
  - `docs/rust_wrapper.md`
  - `docs/cli_reference.md`

## Acceptance Criteria

1. No functional regressions in existing test suite.
2. `drishti(include_bindus=true)` no longer recomputes graha/bhava/sunrise in one request path.
3. `ashtakavarga`-related repeated work is reduced in combined kundali workloads.
4. `full_kundali_for_date` exists and reuses intermediates in a single stateless call.
5. `query_batch` and batch ABIs are only adopted when backed by benchmark/profile evidence.

