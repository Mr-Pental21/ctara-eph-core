# jpl_kernel C ABI Coverage

Scope: crate-root runtime free functions from `jpl_kernel`.

Direct C ABI coverage: `0 / 1`.

| Runtime API | C ABI export(s) | Status |
|---|---|---|
| `planet_body_to_barycenter` | `-` | Missing direct wrapper |

Notes:
- The C ABI currently wraps kernel loading/evaluation through `dhruv_core` engine APIs,
  not the standalone `jpl_kernel::planet_body_to_barycenter` helper.
