# Go Wrapper (`go-open`)

Open-source Go bindings for `ctara-dhruv-core`, implemented against the canonical C ABI (`dhruv_ffi_c`).

## Status

- ABI target: `DHRUV_API_VERSION=44`
- Binding strategy: `cgo` over `crates/dhruv_ffi_c/include/dhruv.h`
- Package: `ctara-dhruv-core/bindings/go-open/dhruv`

## Prerequisites

- Go (1.24+)
- Rust toolchain (`cargo`)

## Build C ABI

From repository root:

```bash
cargo build -p dhruv_ffi_c --release
```

This produces:

- Linux: `target/release/libdhruv_ffi_c.so`
- macOS: `target/release/libdhruv_ffi_c.dylib`
- Windows: `target/release/dhruv_ffi_c.dll`

## Run Tests

From `bindings/go-open`:

```bash
GOCACHE=/tmp/go-build go test ./...
```

Kernel-dependent tests auto-skip when required files are absent.

## Quickstart

See `examples/basic/main.go`.

```bash
export DHRUV_SPK_PATH=/abs/path/to/de442s.bsp
export DHRUV_LSK_PATH=/abs/path/to/naif0012.tls
cd bindings/go-open
GOCACHE=/tmp/go-build go run ./examples/basic
```

## Library Loading

The wrapper links against `target/release` by default via cgo linker flags.

If runtime loading fails:

- Linux: add `target/release` to `LD_LIBRARY_PATH`
- macOS: add `target/release` to `DYLD_LIBRARY_PATH`
- Windows: add `target/release` to `PATH`

## Coverage

Low-level coverage in `internal/cabi` maps all currently exported `dhruv_ffi_c`
symbols from `dhruv.h` (ABI v44).

The public `dhruv` package includes wrappers for:

- engine/config/LSK/EOP lifecycle
- ephemeris query and UTC query helpers
- time conversion and nutation
- ayanamsha and lunar-node APIs
- riseset/bhava APIs
- unified search APIs (conjunction/grahan/motion/lunar phase/sankranti)
- panchang and calendar date APIs
- panchang/classifier/math helper APIs
- graha longitude and jyotish date APIs
- shadbala, vimsopaka, and avastha date APIs
- drishti, ashtakavarga, core bindus, and amsha APIs
- dasha hierarchy/snapshot APIs
- full-kundali summary API (default C ABI config)
- tara catalog and compute APIs
