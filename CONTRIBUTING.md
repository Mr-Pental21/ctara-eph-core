# Contributing Guide

All contributions to this project must follow `LICENSE_POLICY.md`.

## Pull Request Requirements

Every PR must include:

- A short summary of changes.
- License review for any new dependency.
- Updates to `THIRD_PARTY_NOTICES.md` if dependency set changed.
- Clean-room/provenance update when adding a new algorithm or data source.

## Mandatory Contributor Declaration

By opening a PR, you confirm:

- This contribution is not derived from denylisted, proprietary, or source-available codebases.
- You did not reference or study denylisted implementations while authoring this change.
- Any AI assistance used complied with `LICENSE_POLICY.md`.

## PR Checklist

- [ ] I reviewed `LICENSE_POLICY.md`.
- [ ] My code is original or copied only from allowlisted-license sources with proper notices.
- [ ] I did not reference denylisted/source-available implementations.
- [ ] Any external algorithm includes provenance metadata.
- [ ] Any external data/table/constant is public domain or allowlisted.
- [ ] New dependencies were license-reviewed and are allowlisted.
- [ ] If license status is unclear, I treated it as disallowed and flagged it.
- [ ] If AI tools were used, prompts/outputs did not request denylisted replication.
- [ ] For major subsystem changes, I updated/added a clean-room `DESIGN.md`.
