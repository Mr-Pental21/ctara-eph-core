# Clean-Room Implementation Record

## Subsystem

- Name: Charakaraka assignment (Jaimini/Parashara variants)
- Owner: ctara-dhruv maintainers
- Date: 2026-03-05

## Scope

- What is being implemented:
  - Charakaraka role assignment from graha sidereal longitudes.
  - Selectable scheme support: 8-karaka, 7-no-pitri, 7 PK-merged-MK, mixed Parashara.
  - Surface integration in C ABI, CLI, Rust, Python, Go, and Node wrappers.
- Public API surface impacted:
  - `dhruv_vedic_base`, `dhruv_search`, `dhruv_ffi_c`, `dhruv_cli`, `dhruv_rs`.
  - Wrapper APIs in `bindings/python-open`, `bindings/go-open`, `bindings/node-open`.

## Conceptual Sources

- Paper/spec/public-domain source URL:
  - Public-domain Jyotisha concept sources for Chara Karaka role names and ordering.
  - Traditional role sets used in widely published astrological literature (Atma, Amatya, Bhratri, Matri, Pitri, Putra, Gnati, Dara).
- License/status:
  - Conceptual/traditional domain knowledge; no copyrighted implementation text reused.
- What concept or formula was used:
  - Rank planets by degrees traversed in the current sign.
  - Rahu handled with reversed in-sign traversal (`30 - degrees_in_sign`) when included.
  - Scheme-dependent role assignment and count.
  - Mixed Parashara selector based on integer-degree tie predicate among classical planets.

## Explicitly Excluded Sources

- Denylisted projects reviewed: `None`
- Source-available/proprietary projects reviewed: `None`

## Data Provenance

- Tables/constants/datasets used:
  - No external numeric tables or copyrighted datasets.
  - Only enum role/scheme code assignments defined in this repository.
- Source URL:
  - N/A
- License/status:
  - N/A
- Evidence this source is public domain or allowlisted:
  - No third-party data ingestion.

## Implementation Notes

- Key algorithm choices:
  - Deterministic sorting by effective in-sign degree (desc), then raw in-sign degree (desc), then graha index (asc).
  - Mixed Parashara switches to 8-karaka mode when same-sign integer-degree ties are detected among classical planets; otherwise uses 7 PK-merged-MK.
- Numerical assumptions:
  - In-sign degree domain normalized to `[0, 30)`.
  - Floating-point comparison for mixed scheme tie predicate uses floored integer degree.
- Edge cases handled:
  - Invalid/non-finite longitude inputs rejected.
  - Scheme validation enforced at FFI boundary.
  - Output bounded by `DHRUV_MAX_CHARAKARAKA_ENTRIES`.

## Validation

- Black-box references used (I/O comparison only):
  - Behavioral parity checks against user-specified scheme expectations.
- Golden test vectors added:
  - Scheme-specific unit tests for 8/7/mixed behavior and Rahu reversal.
- Error tolerance used:
  - Exact discrete role/rank matching for tests.

## Contributor Declaration

- I confirm this implementation is clean-room and does not derive from denylisted/source-available code.
- Name: Codex AI agent
- Date: 2026-03-05
