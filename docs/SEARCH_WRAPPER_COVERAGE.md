# dhruv_search C ABI Coverage

Scope: crate-root runtime/query APIs re-exported by `dhruv_search` (58 functions).

Direct C ABI coverage is high: `57 / 58` runtime APIs have an exported
`dhruv_ffi_c` entry point.
Functional coverage is `58 / 58` when `moon_sidereal_longitude_at` is satisfied
via `dhruv_graha_longitudes` with sidereal config (Moon graha index).

## Not Wrapped Directly

This crate-root runtime function does not currently have a direct C export:

- `moon_sidereal_longitude_at`

Functional coverage note:
- `moon_sidereal_longitude_at` is obtainable from
  `dhruv_graha_longitudes` with sidereal config (Moon is one graha entry in that output).

## Wrapped API Families (Direct)

- Conjunction/aspect: `dhruv_body_ecliptic_lon_lat`, `dhruv_next_conjunction`,
  `dhruv_prev_conjunction`, `dhruv_search_conjunctions` (+ `_utc` variants where present)
- Lunar phase: `dhruv_next_purnima`, `dhruv_prev_purnima`, `dhruv_next_amavasya`,
  `dhruv_prev_amavasya`, `dhruv_search_purnimas`, `dhruv_search_amavasyas`
- Grahan: `dhruv_next_*_grahan`, `dhruv_prev_*_grahan`, `dhruv_search_*_grahan`
  (including `_utc` variants)
- Sankranti: `dhruv_next_sankranti`, `dhruv_prev_sankranti`,
  `dhruv_search_sankrantis`, `dhruv_next_specific_sankranti`,
  `dhruv_prev_specific_sankranti`
- Stationary/max-speed: `dhruv_next_stationary`, `dhruv_prev_stationary`,
  `dhruv_search_stationary`, `dhruv_next_max_speed`, `dhruv_prev_max_speed`,
  `dhruv_search_max_speed` (including `_utc` variants)
- Panchang/time slices: `dhruv_masa_for_date`, `dhruv_ayana_for_date`,
  `dhruv_varsha_for_date`, `dhruv_nakshatra_for_date`, `dhruv_tithi_for_date`,
  `dhruv_karana_for_date`, `dhruv_yoga_for_date`, `dhruv_vaar_for_date`,
  `dhruv_hora_for_date`, `dhruv_ghatika_for_date`, `dhruv_panchang_for_date`,
  plus helper exports (`dhruv_elongation_at`, `dhruv_sidereal_sum_at`,
  `dhruv_tithi_at`, `dhruv_karana_at`, `dhruv_yoga_at`,
  `dhruv_vedic_day_sunrises`, `dhruv_vaar_from_sunrises`,
  `dhruv_hora_from_sunrises`, `dhruv_ghatika_from_sunrises`)
- Jyotish orchestrators: `dhruv_special_lagnas_for_date`,
  `dhruv_arudha_padas_for_date`, `dhruv_all_upagrahas_for_date`,
  `dhruv_graha_positions`, `dhruv_ashtakavarga_for_date`, `dhruv_core_bindus`,
  `dhruv_drishti`, `dhruv_graha_longitudes`, `dhruv_nakshatra_at`

## Amsha Parity Status

Amsha parity is tracked separately from the broad runtime-coverage count above.

Canonical C ABI amsha surface:

- `dhruv_amsha_longitude`
- `dhruv_amsha_rashi_info`
- `dhruv_amsha_longitudes`
- `dhruv_amsha_chart_for_date`
- full-kundali amsha config/result fields

Current wrapper status:

| Surface | Status | Notes |
|---|---|---|
| `dhruv_rs` | complete | direct pure helpers, date-backed chart helpers, and full-kundali amsha config exposed |
| CLI | complete | `amsha`, `amsha-chart`, and `kundali` amsha selection/scope flags documented and implemented |
| Python | complete | direct amsha helpers plus full-kundali amsha selection/scope and optional section extraction |
| Go | complete | direct amsha helpers plus full-kundali amsha selection/scope and optional section extraction |
| Node | complete | direct amsha helpers plus full-kundali amsha selection/scope and optional section extraction |
| Elixir | complete | dedicated amsha scope plus full-kundali amsha selection/scope and richer result maps |

Reference:

- `docs/AMSHA_PARITY_CONTRACT.md`
