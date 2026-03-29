# `dhruv_rs` — Rust Wrapper

## Purpose

`dhruv_rs` is the context-first Rust facade over the ctara-dhruv crates.

The intended public shape is:

- explicit reusable `DhruvContext`
- typed request structs in `ops.rs`
- selected re-exported config/result/helper types that are intentionally part
  of the high-level Rust contract

For usage-first end-user docs, start with
[`docs/end_user/rust_lib/README.md`](end_user/rust_lib/README.md).

## Quick Start

```rust
use std::path::PathBuf;
use dhruv_rs::*;

let engine_config = EngineConfig::with_single_spk(
    PathBuf::from("kernels/data/de442s.bsp"),
    PathBuf::from("kernels/data/naif0012.tls"),
    256,
    true,
);

let ctx = DhruvContext::new(engine_config).expect("context init");
let eop = EopKernel::load("kernels/data/finals2000A.all").expect("eop");

let request = UpagrahaRequest {
    at: TimeInput::Utc(UtcDate::new(2024, 1, 15, 12, 0, 0.0)),
    location: GeoLocation::new(28.6139, 77.2090, 0.0),
    riseset_config: Some(RiseSetConfig::default()),
    sankranti_config: Some(SankrantiConfig::default_lahiri()),
    upagraha_config: Some(TimeUpagrahaConfig::default()),
};

let upagrahas = upagraha_op(&ctx, &eop, &request).expect("upagraha");
assert!(upagrahas.gulika >= 0.0 && upagrahas.gulika < 360.0);
```

## Public API Shape

### Context Lifecycle

- `DhruvContext::new(config)`
- `DhruvContext::with_resolver(config, resolver)`
- `DhruvContext::engine()`
- `DhruvContext::resolver()`
- `DhruvContext::set_resolver(...)`
- `DhruvContext::set_time_conversion_policy(...)`
- `DhruvContext::time_conversion_policy()`

### Request-Based Ops

- search/event requests:
  `ConjunctionRequest`, `GrahanRequest`, `MotionRequest`,
  `LunarPhaseRequest`, `SankrantiRequest`
- scalar/value requests:
  `AyanamshaRequest`, `NodeRequest`
- assembled workflow requests:
  `PanchangRequest`, `TaraRequest`, `CharakarakaRequest`,
  `UpagrahaRequest`, `AvasthaRequest`, `FullKundaliRequest`

The corresponding entrypoints are:

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
- `full_kundali`

### Re-Export Policy

`dhruv_rs` intentionally re-exports a selected set of high-level config/result
types so callers can stay on the facade for common workflows. It is not meant
to be a full umbrella crate for every low-level Rust API in the workspace.

Low-level engine, time, frame, and extension-trait surfaces that are not part
of the stable high-level contract should be used from their source crates:

- `dhruv_core`
- `dhruv_time`
- `dhruv_frames`
- `dhruv_search`
- `dhruv_vedic_base`

## Configuration Rules

- Invocation-specific data belongs in the request or context:
  UTC vs JD(TDB), locations, target graha selection, range bounds, and other
  per-call inputs.
- Behavior and policy knobs belong in config structs:
  `SankrantiConfig`, `RiseSetConfig`, `FullKundaliConfig`,
  `TimeUpagrahaConfig`, and similar families.
- If a `DhruvContext` has a `ConfigResolver`, omitted config fields are resolved
  from layered config before built-in defaults.

## Notes

- `dhruv_rs` does not use public global singleton APIs.
- Reusable `DhruvContext` ownership is the intended replacement for process-wide
  wrapper state.
