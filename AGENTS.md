# AI Agent Operating Rules (`ctara-dhruv-core`)

Scope: this file governs AI agent behavior in `ctara-dhruv-core`.

## Mandatory Policy Sources

- Follow `LICENSE_POLICY.md`.
- Follow `CONTRIBUTING.md`.
- Use `CLEAN_ROOM_RECORD_TEMPLATE.md` for major subsystem work.

## License And Source Rules

- Allowed licenses: `MIT`, `Apache-2.0`, `BSD-2-Clause`, `BSD-3-Clause`, `ISC`, `Zlib`.
- Disallowed licenses: `AGPL-*`, `GPL-*`, `LGPL-*`, `SSPL-*`, `BUSL/BSL`, and similarly restrictive/source-available licenses.
- If a license is unclear/custom, treat it as disallowed until explicitly approved.

## No-Taint Rule

- Do not reference, study, summarize, or derive implementations from denylisted/source-available codebases (for example Swiss Ephemeris and GPL astrology libraries).
- Black-box testing via input/output comparison is allowed.
- Implementation guidance must come from papers, public-domain sources, or clean specifications.

## Data Provenance Rule

- Tables/constants/correction values/datasets must be public domain or explicitly allowlisted.
- Do not transcribe tables/constants from denylisted projects even when no code is copied.

## AI-Specific Rule

- AI-generated code is authored code and must satisfy all policy constraints.
- Never prompt tools/models to replicate denylisted implementations.

## Core Boundary Rule

- `ctara-dhruv-core` must not depend on `ctara-dhruv-pro`.
- Do not introduce proprietary behavior through feature flags, shared files, generated artifacts, or build-time hooks.

## Required Workflow For Agent Changes

1. Before adding a dependency, verify license allowlist compliance.
2. Record algorithm/data provenance for external concepts.
3. Update `THIRD_PARTY_NOTICES.md` when dependency set changes.
4. For major subsystem changes, add/update a clean-room record.
5. Keep CI license checks passing (`scripts/ci/license_gate.sh`).

## Stop Conditions

- Stop and ask for human approval if:
  - source/license status is ambiguous,
  - requested approach requires denylisted/source-available reference code,
  - a change risks violating core/pro separation.

## Testing

- **Unit tests** (`src/`): pure logic, no external files. Examples: math, config validation, null-pointer rejection.
- **Integration tests** (`tests/`): anything that loads kernel files (`.bsp`, `.tls`) or other external data from disk.
- Never put kernel-dependent tests in `src/`. Always place them in the crate's `tests/` directory.
- Integration tests that need kernels should skip gracefully (early return) when files are absent.

## Naming Convention

- Crate names: `dhruv_*` prefix (e.g., `dhruv_core`, `dhruv_time`). Exception: `jpl_kernel` (domain-specific, not project-branded).
- FFI constants: `DHRUV_*` (e.g., `DHRUV_API_VERSION`)
- FFI types: `Dhruv*` (e.g., `DhruvStatus`, `DhruvEngineConfig`)
- FFI functions: `dhruv_*` (e.g., `dhruv_engine_new`, `dhruv_lsk_load`)

## Git
- Short commit messages, imperative mood
- No boilerplate, signatures, or Co-Authored-By lines