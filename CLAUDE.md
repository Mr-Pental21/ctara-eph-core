# Claude Agent Instructions (Core Repo)

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

- Do not add dependency edges from core to pro.
- Do not leak proprietary logic into core through flags, generated code, or shared files.

## Dependency Rules

- New dependency requires allowlist verification.
- If uncertain, do not proceed without explicit approval.

## Output Expectations

- Produce policy-compliant patches only.
- If requested work conflicts with policy, refuse that path and propose a compliant alternative.
