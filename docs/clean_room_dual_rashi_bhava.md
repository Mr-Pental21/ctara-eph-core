# Clean-Room Implementation Record: Dual Rashi-Bhava Outputs

## Subsystem

- Name: Dual bhava outputs and bala/avastha bhava-basis selection
- Owner: Codex
- Date: 2026-04-19

## Scope

- What is being implemented: rashi-bhava/equal-house companion outputs, configurable bala/avastha bhava basis, and exact-longitude Shadbala Dig Bala.
- Public API surface impacted: Rust/search, `dhruv_rs` via shared types, C ABI, CLI, Python, Go, Node, and Elixir wrappers.

## Conceptual Sources

- Paper/spec/public-domain source URL: existing project clean-room bhava and shadbala records in `docs/clean_room_bhava.md` and `docs/clean_room_shadbala.md`.
- License/status: project-authored Apache-2.0/MIT repository documentation and elementary spherical/zodiac arithmetic.
- What concept or formula was used: whole-sign offset from lagna rashi, equal-house cusps preserving lagna degree, and exact angular separation on a 360-degree circle.

## Explicitly Excluded Sources

- Denylisted projects reviewed: `None`
- Source-available/proprietary projects reviewed: `None`

## Data Provenance

- Tables/constants/datasets used: Dig Bala max-strength cusp mapping already present in Dhruv shadbala code and documentation.
- Source URL: local project clean-room record `docs/clean_room_shadbala.md`.
- License/status: project-authored under repository license.
- Evidence this source is public domain or allowlisted: no external tables were introduced in this change.

## Implementation Notes

- Key algorithm choices: rashi-bhava cusps are derived from sidereal lagna; bhava N is `(lagna_rashi + N - 1) % 12` at the same degree within the sign. Bala and avastha read either rashi-bhava or configured bhava from a shared per-call context.
- Numerical assumptions: all rashi-bhava longitudes are normalized to `[0, 360)`. Dig Bala uses smaller angular separation clamped to `[0, 180]`.
- Edge cases handled: configured bhava outputs remain unchanged; rashi-bhava siblings are suppressed when `include_rashi_bhava_results=false`; standalone bala/avastha defaults preserve the new rashi-bhava mode but can explicitly use configured bhavas.

## Validation

- Black-box references used (I/O comparison only): none.
- Golden test vectors added: rashi-bhava cusp sequence and whole-sign bhava-number unit tests; Dig Bala exact-distance unit tests.
- Error tolerance used: `1e-12` for rashi-bhava helper tests and existing shadbala epsilon for Dig Bala tests.

## Contributor Declaration

- I confirm this implementation is clean-room and does not derive from denylisted/source-available code.
- Name: Codex
- Date: 2026-04-19
