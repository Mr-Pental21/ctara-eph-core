# CLI End-User Docs

`dhruv_cli` is the broadest public surface in the repository. It exposes:

- engine/config inspection
- ephemeris and sidereal longitude queries
- time, rise/set, lagna, and bhava tools
- panchang and jyotish chart calculations
- pure-math classifiers and scalar helpers
- unified search commands for conjunction, grahan, lunar phase, sankranti, and motion
- dasha and tara commands

Start here:

- [CLI Reference](./reference.md)
- [Upagraha Configuration](./upagraha_configuration.md)

Install path:

- release binaries are attached to GitHub Releases for each `vX.Y.Z` tag
- local development still uses `cargo build -p dhruv_cli --release`

Deeper internal reference:

- [`docs/cli_reference.md`](../../cli_reference.md)
