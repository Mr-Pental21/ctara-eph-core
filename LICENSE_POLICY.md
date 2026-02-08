# License Policy

This project accepts only permissively licensed code and original implementations derived from public/open research papers.

## Allowed Licenses (Default Allowlist)

- MIT
- Apache-2.0
- BSD-2-Clause
- BSD-3-Clause
- ISC
- Zlib

## Disallowed Licenses (Denylist)

- AGPL (all versions)
- GPL (all versions)
- LGPL (all versions)
- SSPL
- BUSL/BSL
- Any license with strong copyleft, network copyleft, or production-use restrictions

## Source Intake Rules

- Public papers may be used for formulas and algorithm design.
- Code must be original implementation unless copied source is under an allowed license.
- Any direct code reuse must preserve required notices and attribution.
- If a source license is unclear, custom, or ambiguous, treat it as disallowed until explicitly approved.

## No-Taint Rule

- Contributors must not reference, study, or derive implementations from denylisted or source-available codebases (for example Swiss Ephemeris and GPL astrology libraries).
- Black-box testing (input/output comparison) is allowed.
- Implementation guidance must come only from papers, public-domain sources, or clean specifications.

## Data Provenance Rules

- Precomputed tables, constants, correction values, and datasets must be public domain or under an allowlisted license.
- No tables or constants may be transcribed from denylisted or source-available projects, even if no code is copied.

## Clean-Room Record

- Major subsystems must maintain a short clean-room design note (for example `DESIGN.md`) that lists:
  - conceptual sources used,
  - references consulted,
  - confirmation that denylisted codebases were not reviewed.

## Dependency Rules

- New dependencies require license review in PR.
- CI must fail if a denylisted license is detected.
- `THIRD_PARTY_NOTICES.md` must be updated when dependencies change.
- SPDX license identifiers are required in source file headers where practical.

## Contributor Declaration

- Contributors must confirm their changes are not derived from denylisted, proprietary, or non-approved sources.

## AI Assistance Policy

- AI-generated code is treated as authored code and must comply with this policy.
- Contributors must not ask AI systems to study, summarize, or replicate denylisted/source-available codebases.

## Core / Pro Separation Rule

- `ctara-eph-core` must not depend on `ctara-eph-pro`.
- Proprietary logic must not be introduced into core via shared files, feature flags, conditional compilation, build scripts, or generated artifacts.

## Provenance Record

For each external algorithm/source, record:
- Source URL
- License
- What was reimplemented
- Any copied files and their notices (if applicable)
