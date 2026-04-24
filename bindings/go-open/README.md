# Go Wrapper (`go-open`)

Open-source Go bindings for `ctara-dhruv-core`, implemented against the canonical C ABI (`dhruv_ffi_c`).

## Status

- ABI target: `DHRUV_API_VERSION=62`
- Binding strategy: `cgo` over `crates/dhruv_ffi_c/include/dhruv.h`
- Package: `ctara-dhruv-core/bindings/go-open/dhruv`
- Distribution model: tagged Go module plus validated C ABI release artifacts

## End-User Docs

Usage-first documentation for this wrapper lives in
[`../../docs/end_user/go/README.md`](../../docs/end_user/go/README.md).

## Prerequisites

- Go (1.24+)
- Rust toolchain (`cargo`)

## Install

```bash
go get ctara-dhruv-core/bindings/go-open/dhruv@v0.1.0
```

The Go wrapper remains a source-consumed module. It expects a matching
`dhruv_ffi_c` library at build/runtime.

## Build Or Download The C ABI

From repository root:

```bash
cargo build -p dhruv_ffi_c --release
```

This produces:

- Linux: `target/release/libdhruv_ffi_c.so`
- macOS: `target/release/libdhruv_ffi_c.dylib`
- Windows: `target/release/dhruv_ffi_c.dll`

Release tags also publish C ABI bundles on GitHub Releases for the main
platform matrix. Linux and macOS consumers can point `CGO_LDFLAGS` and runtime
library paths at those bundles. Windows Go support remains source-build first
in the initial rollout.

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
symbols from `dhruv.h` (ABI v61).

Dasha periods returned through the Go wrapper now carry `EntityName`, the exact
canonical Sanskrit entity name alongside the numeric kind/index fields.

The public `dhruv` package includes wrappers for:

- engine/config/LSK/EOP lifecycle
- unified ephemeris query requests with selectable JD-vs-UTC input and cartesian-vs-spherical output
- time conversion and nutation
- ayanamsha and lunar-node APIs
- riseset/bhava APIs
- unified search APIs (conjunction/grahan/motion/lunar phase/sankranti)
  with structured UTC on the high-level time-bearing result objects alongside JD
- panchang and calendar date APIs
- panchang/classifier/math helper APIs
- graha longitude and jyotish date APIs
- shadbala, vimsopaka, and avastha date APIs
- drishti, ashtakavarga, core bindus, and amsha APIs
- dasha hierarchy/snapshot APIs
- full-kundali summary and full-result APIs, including root sphutas and dasha hierarchies
- tara catalog and compute APIs
- low-level graha relationship/combustion/dignity helpers
- low-level tara propagation and correction primitives

## Time-Based Upagraha Config

The Go wrapper exposes configurable time-based upagrahas through:

- `TimeUpagrahaConfigDefault()`
- `(*Engine).AllUpagrahasForDateWithConfig(...)`
- `BindusConfig.UpagrahaConfig`
- `FullKundaliConfig.UpagrahaConfig`

Public value constants are:

- `UpagrahaPointStart`, `UpagrahaPointMiddle`, `UpagrahaPointEnd`
- `GulikaMaandiPlanetRahu`, `GulikaMaandiPlanetSaturn`

## Amsha Notes

The Go wrapper exposes the amsha surface through:

- `AmshaLongitude`
- `AmshaRashiInfo`
- `AmshaLongitudes`
- `AmshaVariations`
- `AmshaVariationsMany`
- `(*Engine).AmshaChartForDate`
- `FullKundaliConfig.AmshaSelection`
- `FullKundaliConfig.AmshaScope`

Standalone bala helpers take the same selection shape:

- `(*Engine).ShadbalaForDate(..., amshaSelection)`
- `(*Engine).VimsopakaForDate(..., amshaSelection)`
- `(*Engine).BalasForDate(..., amshaSelection)`
- `(*Engine).AvasthaForDate(..., amshaSelection)`

`AmshaChart` now carries optional scoped sections directly:

- `BhavaCusps`
- `ArudhaPadas`
- `Upagrahas`
- `Sphutas`
- `SpecialLagnas`

For embedded amsha charts in `FullKundaliForDate`, the relevant root sections
must also be enabled in the full-kundali config, or the wrapper caller must use
a higher-level helper that promotes those dependencies. Returned
`FullKundaliResult.Amshas` now contains the resolved union of explicit
`AmshaSelection` and any internally required bala/avastha amshas. Numeric
variation codes are amsha-scoped; use `AmshaVariations` or
`AmshaVariationsMany` to discover valid codes and names for each amsha.
