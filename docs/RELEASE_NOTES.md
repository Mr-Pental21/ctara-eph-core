# Release Notes

## Unreleased

- Added unified `vX.Y.Z` release automation across Python, Node, Go verification,
  Elixir, `dhruv_rs`, CLI, and the C ABI.
- Added GitHub Release packaging for CLI and C ABI bundles, npm prebuild
  packaging, PyPI publish, and Hex publish wiring.
- Expanded CI coverage to explicitly include Linux/macOS/Windows required
  targets plus best-effort Windows ARM64 jobs.
- Time policy default changed to `hybrid-deltat` (future freeze enabled by default).
- Date-driven command paths in `dhruv_cli`, `dhruv_search`, and `dhruv_rs` now share policy-aware UTC->TDB handling by default.
- Added optional staleness warnings:
  - `--stale-lsk-threshold-days`
  - `--stale-eop-threshold-days`
- Added model-agnostic future Delta-T transition strategies:
  - `legacy-tt-utc-blend` (default frozen-compatible behavior).
  - `bridge-modern-endpoint` (100-year bridge to selected asymptotic family).
- Removed user-facing `--no-freeze-future`; use `--future-delta-t-transition` instead.
- Added `stephenson1997` support in `--smh-future-family` under bridge strategy.
- Added `stephenson2016` support in `--smh-future-family` under bridge strategy, using:
  - `ΔT = -320.0 + 32.5 * ((year - 1825.0) / 100.0)^2`
