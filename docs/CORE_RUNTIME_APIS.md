# dhruv_core Runtime API

This is the callable public runtime surface for `dhruv_core`.
The crate is method-centric around `Engine`.

## Engine Configuration

| API | Input | Output | Purpose |
|---|---|---|---|
| `EngineConfig::with_single_spk` | `spk_path, lsk_path, cache_capacity, strict_validation` | `EngineConfig` | Convenience constructor for one SPK + one LSK setup. |

## Body / Observer / Frame Conversions

| API | Input | Output | Purpose |
|---|---|---|---|
| `Body::code` | `self` | `i32` | Convert a `Body` enum to NAIF-style code. |
| `Body::from_code` | `code` | `Option<Body>` | Convert NAIF-style code back to `Body`. |
| `Observer::code` | `self` | `i32` | Convert observer to compact code (0 = SSB). |
| `Observer::from_code` | `code` | `Option<Observer>` | Convert compact code back to observer. |
| `Frame::code` | `self` | `i32` | Convert frame to compact code. |
| `Frame::from_code` | `code` | `Option<Frame>` | Convert compact code back to frame. |

## Engine Lifecycle / Accessors

| API | Input | Output | Purpose |
|---|---|---|---|
| `Engine::new` | `config` | `Result<Engine, EngineError>` | Load kernels and create an engine instance. |
| `Engine::config` | `&self` | `EngineConfig` | Read active engine configuration. |
| `Engine::replace_spk_paths` | `spk_paths` | `Result<SpkReplaceReport, EngineError>` | Atomically replace the active SPK set. |
| `Engine::spk_infos` | `&self` | `Vec<LoadedSpkInfo>` | List active SPKs in query order. |
| `Engine::spk_generation` | `&self` | `u64` | Current SPK-set generation. |
| `Engine::lsk` | `&self` | `&LeapSecondKernel` | Access loaded leap-second kernel. |

SPK replacement is copy-on-write: new kernels are loaded before the active set
is swapped, matching kernels are reused by canonical path + file size + mtime,
and failed replacements leave the old set active. LSK remains engine-lifetime
state and requires recreating the engine to change.

## Query Execution

| API | Input | Output | Purpose |
|---|---|---|---|
| `Engine::query` | `query` | `Result<StateVector, EngineError>` | Execute one ephemeris query. |
| `Engine::query_with_stats` | `query` | `Result<(StateVector, QueryStats), EngineError>` | Execute one query and return telemetry. |
| `Engine::query_batch` | `queries` | `Vec<Result<StateVector, EngineError>>` | Execute many queries with per-request memoization. |
| `Engine::query_batch_with_stats` | `queries` | `(Vec<Result<StateVector, EngineError>>, QueryStats)` | Batch query plus aggregate telemetry. |
| `Engine::query_with_derived` | `query, derived` | `Result<(StateVector, DerivedValue), EngineError>` | Run core query and derived extension computation together. |
