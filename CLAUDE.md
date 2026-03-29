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
- Update `THIRD_PARTY_NOTICES.md` when the dependency set changes.
- Keep CI license checks passing (`scripts/ci/license_gate.sh`).
- For significant changes, create a plan file in Claude's plan folder before implementation.
- When implementing a plan, continue until the full plan is implemented and tested unless an issue cannot be resolved without human approval.

## Open Source Scope

- `ctara-dhruv-core` is the primary open-source project scope.
- Do not add proprietary-only logic through flags, generated code, or shared files.

## Dependency Rules

- New dependency requires allowlist verification.
- If uncertain, do not proceed without explicit approval.

## Output Expectations

- Produce policy-compliant patches only.
- If requested work conflicts with policy, refuse that path and propose a compliant alternative.
- Treat the implementation code as the source of truth when docs disagree.
- Prefer one public entry point per logical feature. Express variations through typed request/context inputs and `dhruv_config`-backed config attributes instead of separate ABI/function variations unless a genuinely new feature requires a new entry point.
- Use request/context attributes for alternate inputs or precomputed invocation data, such as UTC vs JD, `with_inputs`, and `with_moon`. Use config attributes for behavior and policy knobs. Do not encode those variations in public function names.
- Do not keep parallel setter-style policy APIs such as `set_*` when the value can live in request/context or config data. Consolidation should remove redundant variant/setter surfaces instead of deprecating and keeping them alongside the main shape.
- Keep all public library surfaces in sync for shared features: C ABI, `dhruv_rs` public APIs, CLIs, and wrappers including Python, Go, Node, Elixir, plus future additions. Language-specific convenience features are allowed, but core library features, request/context shapes, configs, and functions should stay aligned across surfaces.
- When public behavior, CLI flags, public configs, wrapper APIs, or user-facing results change, update both:
  - relevant internal/reference docs (`docs/`, wrapper READMEs, public-surface notes),
  - relevant end-user docs under `docs/end_user/`.
- Do not consider a public-surface change complete until those doc updates are done.

## Stop Conditions

- Stop and ask for human approval if:
  - source/license status is ambiguous,
  - the requested approach requires denylisted/source-available reference code,
  - the change risks violating open-source licensing or provenance policy.

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
- Update relevant documentation after changes, including both internal/reference docs and `docs/end_user/` when the change affects public behavior.
- After having added a functionality in library add C ABIs, CLI, `dhruv_rs`, and other public wrappers unless it is intentionally wrapper-specific.
