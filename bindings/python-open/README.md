# Python Wrapper (`python-open`)

Open-source Python bindings for `ctara-dhruv-core`, implemented against the
canonical C ABI (`dhruv_ffi_c`) via `cffi`.

## Status

- ABI target: `DHRUV_API_VERSION=47`
- Package root: `bindings/python-open`
- Runtime dependency: `cffi`

## Install For Local Development

From `bindings/python-open`:

```bash
pip install -e .
```

The shared `dhruv_ffi_c` library must be built from the repository root:

```bash
cargo build -p dhruv_ffi_c --release
```

## Amsha Surface

Direct amsha helpers:

- `ctara_dhruv.amsha.amsha_longitude`
- `ctara_dhruv.amsha.amsha_rashi_info`
- `ctara_dhruv.amsha.amsha_longitudes`
- `ctara_dhruv.amsha.amsha_chart_for_date`

Embedded amsha support:

- `ctara_dhruv.kundali.full_kundali_config_default`
- `ctara_dhruv.kundali.full_kundali`

Relevant full-kundali config fields:

- `config.include_amshas`
- `config.amsha_selection`
- `config.amsha_scope`

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
enabled.
