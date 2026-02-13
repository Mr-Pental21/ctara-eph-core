# dhruv_core API Inventory

This document tracks the public crate-root API of `dhruv_core`.

## Primary Types

- `EngineConfig`: startup configuration (SPK paths, LSK path, cache config).
- `Body`: supported physical ephemeris bodies.
- `Observer`: observer target (`SolarSystemBarycenter` or `Body(...)`).
- `Frame`: output frame (`IcrfJ2000`, `EclipticJ2000`).
- `Query`: input payload for one engine query.
- `StateVector`: query output (`position_km`, `velocity_km_s`).
- `Engine`: runtime entry point.
- `QueryStats`: telemetry counters.
- `EngineError`: non-exhaustive core error enum.
- `DerivedValue`: extension output (`Scalar` or `Vector3`).
- `DerivedComputation`: extension trait for downstream derived models.

## Public Methods and Functions

Full callable list is documented in `docs/CORE_RUNTIME_APIS.md`.

## Design Notes

- The crate intentionally keeps runtime access on `Engine` methods instead of
  crate-level free functions.
- Downstream crates (`dhruv_search`, `dhruv_vedic_base`) layer domain logic
  on top of this contract.
