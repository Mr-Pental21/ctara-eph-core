# Architecture Overview

## Goal
`ctara-dhruv-core` is a clean-room Rust ephemeris engine that uses JPL/NAIF kernels and exposes a stable C ABI.

## Planned Crates
- `crates/jpl_kernel`: SPK/DAF parsing and interpolation primitives.
- `crates/dhruv_time`: UTC/TAI/TT/TDB conversion and leap-second handling.
- `crates/dhruv_frames`: frame conversion helpers.
- `crates/dhruv_core`: query engine, computation DAG, memoization, caching.
- `crates/dhruv_rs`: Rust convenience wrapper (global singleton, UTC input, spherical output).
- `crates/dhruv_vedic_base`: open derived Vedic calculations built on core results.
- `crates/dhruv_ffi_c`: C ABI facade with versioned contract.
- `crates/dhruv_cli`: diagnostics and developer tooling.

## Architectural Constraints
- No proprietary dependencies or closed-source coupling.
- No denylisted-source derivation.
- Deterministic outputs under documented numeric tolerances.
- Thread-safe query execution.

## Next Design Deliverables
1. Define canonical time representation and conversion API in `dhruv_time`.
2. Define engine query contract and error model in `dhruv_core`.
3. Define extension traits that downstream crates can implement without tight coupling.
4. Define C ABI ownership and error semantics in `dhruv_ffi_c`.
