# Kernel Inputs

Expected kernel artifacts for initial development:
- `de442s.bsp` (primary SPK for MVP)
- `naif0012.tls` (NAIF leap-seconds kernel for UTC conversions)

## Acquisition

Use the lockfile-driven download script:

```bash
./scripts/kernels/fetch_kernels.sh
```

Default behavior:
- reads `kernels/manifest/de442s.lock`,
- downloads into `kernels/data/`,
- verifies file checksums from the lockfile before accepting files.

Optional overrides:

```bash
./scripts/kernels/fetch_kernels.sh --lock-file kernels/manifest/de442s.lock --dest-dir kernels/data
```

## Provenance

Lockfile entries include:
- canonical source URL for each kernel file,
- pinned checksum,
- checksum source reference URL.

Current checksum source is the NAIF official checksum listing:
- `https://naif.jpl.nasa.gov/pub/naif/generic_kernels/aa_checksums.txt`

Do not commit proprietary or license-incompatible data.

## DE441/DE442 Split Kernels

The split manifest is `kernels/manifest/de441_de442_splits.tsv`. It records
the generated filename, parent kernel, exact TDB coverage, source URL, byte
size, MD5 checksum, precedence, and usage note.

Generate local split BSPs with NAIF `SPKMERGE`:

```bash
./scripts/kernels/generate_split_kernels.sh \
  --spkmerge /path/to/spkmerge \
  --update-manifest
```

The generated `.bsp` files are written to `kernels/data/` and are intentionally
gitignored. Verify manifest checksums with:

```bash
./scripts/kernels/verify_split_kernel_manifest.py
cargo test -p dhruv_core --test split_kernel_integration
```

Use DE442 first for its central range (`1549 DEC 31` through `2650 JAN 25`
TDB). Use DE441 split kernels only as long-range fallback before and after that
range; in overlap zones, load/order DE442 before DE441.
