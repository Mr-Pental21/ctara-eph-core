# dhruv_core C ABI Coverage

Scope: crate-root runtime free functions from `dhruv_core`.

`dhruv_core` is method-centric (`Engine`, `Query`, `Body`, etc.) and does not export
crate-root runtime free functions, so coverage for this scope is:

- Directly wrapped runtime free functions: `0 / 0`

Related C ABI entry points for this crate are handle-oriented and live in
`dhruv_ffi_c`:

- `dhruv_engine_new`
- `dhruv_engine_free`
- `dhruv_engine_query`
