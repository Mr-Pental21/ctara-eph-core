# jpl_kernel Runtime API

This is the crate-root callable runtime surface of `jpl_kernel`.

| API | Input | Output | Purpose |
|---|---|---|---|
| `planet_body_to_barycenter` | `code` | `i32` | Map planet body code (`x99`) to barycenter code (`x`). |
| `SpkKernel::load` | `path` | `Result<SpkKernel, KernelError>` | Load SPK from file path. |
| `SpkKernel::from_bytes` | `data` | `Result<SpkKernel, KernelError>` | Load SPK from in-memory bytes. |
| `SpkKernel::segments` | `&self` | `&[SpkSegment]` | Read indexed SPK segments. |
| `SpkKernel::evaluate` | `target, center, epoch_tdb_s` | `Result<SpkEvaluation, KernelError>` | Evaluate one segment at epoch. |
| `SpkKernel::center_for` | `target` | `Option<i32>` | Find center body for target. |
| `SpkKernel::resolve_to_ssb` | `body_code, epoch_tdb_s` | `Result<[f64; 6], KernelError>` | Resolve body chain to SSB state vector. |
