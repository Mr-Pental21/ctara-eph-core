# jpl_kernel API Inventory

This document tracks the public API surface of `jpl_kernel`.

## Crate-Root Runtime API

See `docs/JPL_KERNEL_RUNTIME_APIS.md`.

## Public Types

- `SpkKernel`
- `SpkSegment`
- `SpkEvaluation`
- `KernelError`

## Public Low-Level Helpers (module APIs)

These are public for testing and composability, even though most callers should
prefer `SpkKernel` methods:

| API | Input | Output | Purpose |
|---|---|---|---|
| `clenshaw` | `coeffs, s` | `f64` | Evaluate Chebyshev series. |
| `clenshaw_derivative` | `coeffs, s` | `f64` | Evaluate derivative of Chebyshev series. |
| `parse_file_record` | `data` | `Result<FileRecord, KernelError>` | Parse DAF file record block. |
| `read_summaries` | `data, file_record` | `Result<Vec<DafSummary>, KernelError>` | Read linked summary records. |
| `segment_from_summary` | `summary` | `Result<SpkSegment, KernelError>` | Build typed SPK segment descriptor. |
| `evaluate_type2` | `data, segment, epoch_tdb_s, endian` | `Result<SpkEvaluation, KernelError>` | Evaluate SPK Type 2 record. |
