# Claude Agent Instructions (`ctara-dhruv-core`)

This repository is clean-room and permissive-license only.

## Read First

- `LICENSE_POLICY.md`
- `CONTRIBUTING.md`
- `AGENTS.md`

## Non-Negotiable Constraints

- Use only allowlisted-license code and public/open papers/specifications.
- Never inspect or derive from denylisted/source-available implementations.
- Treat ambiguous or custom licenses as disallowed.
- Black-box I/O comparison is permitted; code-level reference to denylisted projects is not.

## Implementation Rules

- Prefer original implementation from papers/specs.
- Track provenance for algorithms and data.
- Keep updates compatible with CI license enforcement.
- For major subsystem work, maintain clean-room documentation from template.

## Core/Pro Separation

- `ctara-dhruv-core` must not depend on `ctara-dhruv-pro`.
- Do not leak proprietary logic into core through flags, generated code, or shared files.

## Dependency Rules

- New dependency requires allowlist verification.
- If uncertain, do not proceed without explicit approval.

## Output Expectations

- Produce policy-compliant patches only.
- If requested work conflicts with policy, refuse that path and propose a compliant alternative.

## Naming Convention

- Crate names: `dhruv_*` prefix (e.g., `dhruv_core`, `dhruv_time`). Exception: `jpl_kernel` (domain-specific, not project-branded).
- FFI constants: `DHRUV_*` (e.g., `DHRUV_API_VERSION`)
- FFI types: `Dhruv*` (e.g., `DhruvStatus`, `DhruvEngineConfig`)
- FFI functions: `dhruv_*` (e.g., `dhruv_engine_new`, `dhruv_lsk_load`)

## Testing

- **Unit tests** (`src/`): pure logic, no external files. Examples: math, config validation, null-pointer rejection.
- **Integration tests** (`tests/`): anything that loads kernel files (`.bsp`, `.tls`) or other external data from disk.
- Never put kernel-dependent tests in `src/`. Always place them in the crate's `tests/` directory.
- Integration tests that need kernels should skip gracefully (early return) when files are absent.
- After all Rust code changes, run the full test suite (`cargo test`) and report the number of passing tests. Do not claim tests pass without actually running them.


## Git
- Commit after changes are made and tested.
- Short commit messages, imperative mood.
- No boilerplate, signatures, or Co-Authored-By lines


## General Rules 
- Always verify changes against the actual codebase before making claims. Never fabricate CI status, tool versions, or test results. If you can't verify something, say so explicitly.
- Update relevant documentation after changes.
- After having added a functionality in library Add C ABIs, CLI and rust(dhruv_rs) bindings.