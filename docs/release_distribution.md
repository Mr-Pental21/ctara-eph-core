# Release And Distribution Guide

This repository now uses one release tag stream for all public surfaces:

- release tag format: `vX.Y.Z`
- dry-run path: `workflow_dispatch`
- publish path: tag push plus the matching registry/release credentials

## Distribution Matrix

Public distribution channels:

- Rust crates: crates.io publish stack culminating in `dhruv_rs`
- Python: PyPI
- Node: npm, with bundled native prebuilds inside the published tarball
- Elixir: Hex, with source-built Rustler NIFs
- CLI: GitHub Releases zipped binaries
- C ABI: GitHub Releases zipped bundles (`include/`, `lib/`, checksums)
- Go: Git tag/module release plus validated C ABI linkage in CI

Platform policy:

- required green: Linux x64, macOS x64, macOS arm64, Windows x64
- best effort: Windows ARM64

Current runtime expectations:

- Python ships wheels on the main supported targets and falls back to source-build smoke on Windows ARM64 until wheel support is proven green.
- Node ships bundled prebuilds for the required targets and attempts Windows ARM64 prebuilds without blocking the release.
- Elixir publishes a Hex package, but the NIF is still compiled from source on install.
- Go remains a source-consumed module; CI validates the wrapper on release tags and the released C ABI bundles stay the canonical native artifact.

## Maintainer Release Flow

1. Update all surface versions together to the same `X.Y.Z`.
2. Update `docs/RELEASE_NOTES.md`.
3. Push tag `vX.Y.Z`.
4. Watch:
   - `Python Wheels`
   - `Unified Release`
5. Confirm registry publishes:
   - PyPI
   - npm
   - Hex
  - crates.io Rust crate stack
6. Confirm GitHub Release assets:
   - CLI zip per target
   - C ABI bundle per target
   - Node npm tarball
   - Hex package tarball

## Required Secrets And Trusted Publishing

- PyPI: trusted publishing for the `publish-pypi` job
- npm: `NPM_TOKEN`
- Hex: `HEX_API_KEY`
- crates.io: `CARGO_REGISTRY_TOKEN`

Run `scripts/ci/license_gate.sh` cleanly before attempting a publish rerun.

## Retry Guidance

- If only one publish job fails, rerun that failed job instead of cutting a new tag.
- If GitHub Release asset upload fails, rerun `github-release`; it consumes uploaded artifacts from the same run.
- If a registry publish partially succeeds, do not retag. Fix credentials or metadata, then rerun the failed workflow/job.
- If Windows ARM64 fails, treat it as non-blocking for the initial rollout unless the failure also reproduces on a required target.
