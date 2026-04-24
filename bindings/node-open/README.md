# Node Wrapper (`node-open`)

Open-source Node.js bindings for `ctara-dhruv-core`, implemented against the canonical C ABI (`dhruv_ffi_c`).

## Status

- ABI target: `DHRUV_API_VERSION=62`
- Binding strategy: Native Node-API addon (`native/dhruv_node.cc`) over `crates/dhruv_ffi_c/include/dhruv.h`
- Package: `bindings/node-open`
- Primary distribution: npm package with bundled platform prebuilds from unified `vX.Y.Z` tags

## End-User Docs

Usage-first documentation for this wrapper lives in
[`../../docs/end_user/node/README.md`](../../docs/end_user/node/README.md).

## Install

Published installs:

```bash
npm install ctara-dhruv-node-open
```

The published npm tarball bundles native prebuilds for the required release
targets and loads them automatically at runtime.

## Prerequisites For Local Development

- Node.js 20+
- C++ compiler (Linux/macOS currently)
- Rust toolchain (`cargo`)

## Build

From `bindings/node-open`:

```bash
npm run build
```

This script builds `dhruv_ffi_c` in release mode, compiles `dhruv_node.node`, and copies the shared `dhruv_ffi_c` library next to the addon.

## Test

```bash
npm test
```

Integration tests skip gracefully when kernel files are absent.

## Quickstart

```js
const dhruv = require('./index');

dhruv.verifyAbi();

const engine = dhruv.Engine.create({
  spkPaths: ['/abs/path/to/de442s.bsp'],
  lskPath: '/abs/path/to/naif0012.tls',
  cacheCapacity: 64,
  strictValidation: false,
});

const result = engine.query({
  target: 301,
  observer: 399,
  frame: 1,
  epochTdbJd: 2451545.0,
});

console.log(result.state);
engine.close();
```

## Coverage

Public modules included in this wrapper:

- engine/config/LSK/EOP lifecycle
- time conversions, nutation, ayanamsha, and lunar-node APIs
- unified search APIs (conjunction/grahan/motion/lunar phase/sankranti)
- panchang/date APIs (`compute_rise_set*`, `compute_all_events*`, `compute_bhavas*`, `lagna/mc/ramc`, `tithi`, `karana`, `yoga`, `nakshatra`, `vaar`, `hora`, `ghatika`, `masa`, `ayana`, `varsha`)
- jyotish/rashi/nakshatra helpers (`grahaLongitudes`, longitude classifiers, special lagnas, arudha/upagraha date APIs)
- charakaraka date API (`charakarakaForDate`) with selectable schemes (`8`, `7-no-pitri`, `7-pk-merged-mk`, `mixed-parashara`)
- extras/composable APIs (panchang intermediates, sphuta/special-lagna scalar helpers, ashtakavarga, drishti, graha positions, bindus, amsha)
- low-level graha relationship/combustion/dignity helpers in `extras`
- shadbala/vimsopaka/avastha and full-kundali summary
- dasha hierarchy and snapshot, with `entityName` on returned period objects for the exact canonical Sanskrit entity name
- tara catalog load/compute helpers plus low-level propagation/correction primitives

## Time-Based Upagraha Config

The Node wrapper accepts an optional `upagrahaConfig` object in:

- `jyotish.allUpagrahasForDate(...)`
- `extras.timeUpagrahaJd(...)`
- `extras.timeUpagrahaJdUtc(...)`
- `extras.coreBindusForDate(...)`
- `shadbala.fullKundaliForDate(...)`

Object fields are:

- `gulikaPoint`, `maandiPoint`, `otherPoint`
- `gulikaPlanet`, `maandiPlanet`

Value mappings are numeric:

- points: `0=start`, `1=middle`, `2=end`
- planets: `0=rahu`, `1=saturn`

## Amsha Notes

The Node wrapper exposes the amsha family through:

- `amshaLongitude`
- `amshaRashiInfo`
- `amshaLongitudes`
- `amshaChartForDate`
- `amshaVariations`
- `amshaVariationsMany`
- `fullKundaliConfigDefault`
- `fullKundaliForDate`

`fullKundaliConfigDefault()` returns amsha config fields as:

- `amshaScope`
- `amshaSelection`

The standalone bala helpers accept the same `amshaSelection` object:

- `shadbalaForDate(..., amshaSelection)`
- `vimsopakaForDate(..., amshaSelection)`
- `balasForDate(..., amshaSelection)`
- `avasthaForDate(..., amshaSelection)`

Direct amsha charts and embedded `fullKundaliForDate(...).amshas` now include
the optional scoped arrays when requested and available:

- `bhavaCusps`
- `arudhaPadas`
- `upagrahas`
- `sphutas`
- `specialLagnas`

Embedded `fullKundaliForDate(...).amshas` now returns the full resolved amsha
union used by the call, not only the explicitly requested subset. Numeric
`variationCode` values are interpreted per amsha; use `amshaVariations*` to
discover the valid codes and names for a given amsha.

## Library Loading

- Optional addon override: `DHRUV_NODE_ADDON_PATH=/abs/path/to/dhruv_node.node`
- Published packages load bundled prebuilds from `prebuilds/<platform>-<arch>/`.
- Local development builds still use `build/Release/`.

## Notes

- Windows build path is implemented in `scripts/build-addon.mjs` using MSVC `cl`.
- If `cl`/`node.lib` discovery differs in your environment, set `NODE_INCLUDE_DIR` and use the package scripts from a Developer Command Prompt.
