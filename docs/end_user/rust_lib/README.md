# Rust Library End-User Docs

`dhruv_rs` provides a Rust-friendly surface over the core crates.
It includes:

- `DhruvContext` and request/operation APIs
- canonical request/context-driven jyotish operations such as `upagraha_op`,
  `avastha_op`, and `full_kundali`
- re-exported config/result types used by end users

Amsha variation control is shared across the high-level jyotish surfaces:
standalone shadbala, vimsopaka, balas, and avastha calls accept
`AmshaSelectionConfig`, and `full_kundali(...).amshas` returns the resolved
union of explicit and internally required amshas.

Start here:

- [Rust Reference](./reference.md)
- [Upagraha Configuration](./upagraha_configuration.md)
- `cargo add dhruv_rs` from the unified crates.io release stream

Deeper internal reference:

- [`docs/rust_wrapper.md`](../../rust_wrapper.md)
- [`docs/release_distribution.md`](../../release_distribution.md)
