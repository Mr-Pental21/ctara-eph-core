# Python Wrapper (`python-open`)

Open-source Python bindings for `ctara-dhruv-core`, implemented against the
canonical C ABI (`dhruv_ffi_c`) via `cffi`.

## Status

- ABI target: `DHRUV_API_VERSION=61`
- Package root: `bindings/python-open`
- Runtime dependency: `cffi`
- Primary distribution: PyPI wheels plus sdist from unified `vX.Y.Z` tags

## End-User Docs

Usage-first documentation for this wrapper lives in
[`../../docs/end_user/python/README.md`](../../docs/end_user/python/README.md).

## Install

Published installs:

```bash
pip install ctara-dhruv
```

Local development from `bindings/python-open`:

```bash
pip install -e .
```

The shared `dhruv_ffi_c` library must be built from the repository root:

```bash
cargo build -p dhruv_ffi_c --release
```

Or use the repository helper to refresh the optimized local binaries, bundled
Python shared library, and local CLI/C ABI archives without cutting a release:

```bash
./scripts/ci/build_local_native_binaries.sh
```

Supported prebuilt wheel targets are Linux, macOS, and Windows x64 on the main
release matrix. Windows ARM64 remains best-effort until wheel support is proven
green in CI.

## Time-Based Upagraha Config

Dasha periods exposed by the Python wrapper now include `entity_name`, the
exact canonical Sanskrit entity name alongside the numeric entity fields.

The Python wrapper exposes configurable time-based upagrahas through:

- `ctara_dhruv.vedic.time_upagraha_config_default()`
- `ctara_dhruv.vedic.all_upagrahas_for_date(..., upagraha_config=...)`
- `ctara_dhruv.kundali.core_bindus(..., bindus_config={"upagraha_config": ...})`
- `ctara_dhruv.kundali.full_kundali(..., config=...)`

Accepted dict values are:

- points: `"start"`, `"middle"`, `"end"`
- planets: `"rahu"`, `"saturn"`

## Amsha Surface

Direct amsha helpers:

- `ctara_dhruv.amsha.amsha_longitude`
- `ctara_dhruv.amsha.amsha_rashi_info`
- `ctara_dhruv.amsha.amsha_longitudes`
- `ctara_dhruv.amsha.amsha_chart_for_date`
- `ctara_dhruv.amsha.amsha_variations`
- `ctara_dhruv.amsha.amsha_variations_many`

Embedded amsha support:

- `ctara_dhruv.kundali.full_kundali_config_default`
- `ctara_dhruv.kundali.full_kundali`

Relevant full-kundali config fields:

- `config.include_amshas`
- `config.amsha_selection`
- `config.amsha_scope`

Standalone bala helpers also accept `amsha_selection`:

- `ctara_dhruv.shadbala.shadbala`
- `ctara_dhruv.shadbala.vimsopaka`
- `ctara_dhruv.shadbala.balas`
- `ctara_dhruv.shadbala.avastha`

Optional amsha chart sections extracted by the wrapper:

- `bhava_cusps`
- `arudha_padas`
- `upagrahas`
- `sphutas`
- `special_lagnas`

## Example

```python
import ctara_dhruv
from ctara_dhruv.engine import engine, lsk, eop
from ctara_dhruv.amsha import amsha_chart_for_date
from ctara_dhruv.kundali import full_kundali, full_kundali_config_default

ctara_dhruv.init(
    ["../../kernels/data/de442s.bsp"],
    "../../kernels/data/naif0012.tls",
    "../../kernels/data/finals2000A.all",
)

chart = amsha_chart_for_date(
    engine(), lsk(), eop(),
    jd_utc=(2024, 1, 15, 6, 0, 0.0),
    location=(28.6139, 77.2090),
    amsha_code=9,
    scope={
        "include_sphutas": 1,
        "include_special_lagnas": 1,
    },
)

cfg = full_kundali_config_default()
cfg.include_sphutas = 1
cfg.include_special_lagnas = 1
cfg.include_amshas = 1
cfg.amsha_selection.count = 1
cfg.amsha_selection.codes[0] = 9
cfg.amsha_scope.include_sphutas = 1
cfg.amsha_scope.include_special_lagnas = 1

kundali = full_kundali(
    engine(), lsk(), eop(),
    jd_utc=(2024, 1, 15, 6, 0, 0.0),
    location=(28.6139, 77.2090),
    config=cfg,
)
```

For embedded amsha sections in `full_kundali`, remember that scoped amsha
sub-sections depend on the corresponding root full-kundali sections also being
enabled. Returned `kundali.amshas` charts now reflect the full resolved amsha
set used by the call: explicit `config.amsha_selection` first, then any
internally required amshas for shadbala, vimsopaka, or avastha.
Variation codes are amsha-scoped; use `amsha_variations*` to discover the
valid codes, names, labels, and defaults for a given amsha.

## Low-Level Helper Coverage

`ctara_dhruv.vedic` now also exposes the intended low-level helper family:

- graha relationship and dignity helpers such as `naisargika_maitri`,
  `tatkalika_maitri`, `panchadha_maitri`, `dignity_in_rashi`, and
  `node_dignity_in_rashi`
- combustion helpers such as `combustion_threshold`, `is_combust`, and
  `all_combustion_status`
- classification/lord helpers such as `natural_benefic_malefic`,
  `moon_benefic_nature`, `graha_gender`, `hora_lord`, `masa_lord`, and
  `samvatsara_lord`

`ctara_dhruv.tara` also exposes the low-level tara primitives:

- `propagate_position`
- `apply_aberration`
- `apply_light_deflection`
- `galactic_anticenter_icrs`
