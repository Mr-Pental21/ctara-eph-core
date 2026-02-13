# FFI Benchmark Plan

This plan tracks `dhruv_ffi_c` benchmark coverage for C ABIs and the Rust
functions they wrap.

## Implemented (Runtime Wrappers Covered)

Bench file: `crates/dhruv_ffi_c/benches/ffi_bench.rs`

- Query baseline:
  - `dhruv_engine_query_internal` vs core query path
  - `dhruv_query_once_internal` vs one-shot path
- Classifier/value wrappers:
  - `dhruv_tithi_from_elongation`
  - `dhruv_karana_from_elongation`
  - `dhruv_yoga_from_sum`
  - `dhruv_vaar_from_jd`
  - `dhruv_masa_from_rashi_index`
  - `dhruv_ayana_from_sidereal_longitude`
  - `dhruv_samvatsara_from_year`
  - `dhruv_nth_rashi_from`
  - `dhruv_rashi_lord`
- Ashtakavarga math wrappers:
  - `dhruv_calculate_bav`
  - `dhruv_calculate_all_bav`
  - `dhruv_calculate_sav`
  - `dhruv_trikona_sodhana`
  - `dhruv_ekadhipatya_sodhana`
- Drishti wrappers:
  - `dhruv_graha_drishti`
  - `dhruv_graha_drishti_matrix`
- Ghatika/hora wrappers:
  - `dhruv_ghatika_from_elapsed`
  - `dhruv_ghatikas_since_sunrise`
  - `dhruv_hora_at`
- Upagraha wrapper:
  - `dhruv_time_upagraha_jd`
- Search wrappers:
  - `dhruv_graha_sidereal_longitudes`
  - `dhruv_nakshatra_at`
- Core ABI wrappers:
  - `dhruv_engine_query`
  - `dhruv_query_once`
  - `dhruv_query_utc`
  - `dhruv_query_utc_spherical`
- Time/frame wrappers:
  - `dhruv_utc_to_tdb_jd`
  - `dhruv_jd_tdb_to_utc`
  - `dhruv_cartesian_to_spherical`
- Vedic primitives:
  - `dhruv_ayanamsha_mean_deg`
  - `dhruv_ayanamsha_true_deg`
  - `dhruv_ayanamsha_deg`
  - `dhruv_nutation_iau2000b`
  - `dhruv_lunar_node_deg`
  - `dhruv_rashi_from_longitude`
  - `dhruv_nakshatra_from_longitude`
  - `dhruv_nakshatra28_from_longitude`
  - `dhruv_rashi_from_tropical`
  - `dhruv_nakshatra_from_tropical`
- Search event wrappers:
  - `dhruv_next_conjunction`, `dhruv_prev_conjunction`, `dhruv_search_conjunctions`
  - `dhruv_next_chandra_grahan`, `dhruv_prev_chandra_grahan`, `dhruv_search_chandra_grahan`
  - `dhruv_next_surya_grahan`, `dhruv_prev_surya_grahan`, `dhruv_search_surya_grahan`
  - `dhruv_next_stationary`, `dhruv_prev_stationary`, `dhruv_search_stationary`
  - `dhruv_next_max_speed`, `dhruv_prev_max_speed`, `dhruv_search_max_speed`
  - UTC variants benchmarked:
    - `dhruv_next_conjunction_utc`, `dhruv_prev_conjunction_utc`, `dhruv_search_conjunctions_utc`
    - `dhruv_next_chandra_grahan_utc`, `dhruv_prev_chandra_grahan_utc`,
      `dhruv_search_chandra_grahan_utc`
    - `dhruv_next_surya_grahan_utc`, `dhruv_prev_surya_grahan_utc`,
      `dhruv_search_surya_grahan_utc`
    - `dhruv_next_stationary_utc`, `dhruv_prev_stationary_utc`, `dhruv_search_stationary_utc`
    - `dhruv_next_max_speed_utc`, `dhruv_prev_max_speed_utc`, `dhruv_search_max_speed_utc`
- Lunar phase + sankranti/date wrappers:
  - `dhruv_next_purnima`, `dhruv_prev_purnima`, `dhruv_search_purnimas`
  - `dhruv_next_amavasya`, `dhruv_prev_amavasya`, `dhruv_search_amavasyas`
  - `dhruv_next_sankranti`, `dhruv_prev_sankranti`, `dhruv_search_sankrantis`
  - `dhruv_next_specific_sankranti`, `dhruv_prev_specific_sankranti`
  - `dhruv_masa_for_date`, `dhruv_ayana_for_date`, `dhruv_varsha_for_date`
- Rise/set + bhava wrappers:
  - `dhruv_compute_rise_set`, `dhruv_compute_all_events`
  - `dhruv_compute_rise_set_utc`, `dhruv_compute_all_events_utc`
  - `dhruv_compute_bhavas`, `dhruv_compute_bhavas_utc`
  - `dhruv_lagna_deg`, `dhruv_mc_deg`, `dhruv_ramc_deg`
  - `dhruv_lagna_deg_utc`, `dhruv_mc_deg_utc`, `dhruv_ramc_deg_utc`
- UTC math wrappers:
  - `dhruv_ayanamsha_mean_deg_utc`
  - `dhruv_ayanamsha_true_deg_utc`
  - `dhruv_ayanamsha_deg_utc`
  - `dhruv_nutation_iau2000b_utc`
  - `dhruv_lunar_node_deg_utc`
  - `dhruv_rashi_from_tropical_utc`
  - `dhruv_nakshatra_from_tropical_utc`
  - `dhruv_nakshatra28_from_tropical_utc`
- Additional scalar/runtime wrappers:
  - `dhruv_approximate_local_noon_jd`
  - `dhruv_deg_to_dms`
  - `dhruv_riseset_result_to_utc`
  - `dhruv_arudha_pada`
  - `dhruv_bhrigu_bindu`, `dhruv_prana_sphuta`, `dhruv_deha_sphuta`,
    `dhruv_mrityu_sphuta`, `dhruv_tithi_sphuta`, `dhruv_yoga_sphuta`,
    `dhruv_yoga_sphuta_normalized`, `dhruv_rahu_tithi_sphuta`, `dhruv_kshetra_sphuta`,
    `dhruv_beeja_sphuta`, `dhruv_trisphuta`, `dhruv_chatussphuta`,
    `dhruv_panchasphuta`, `dhruv_sookshma_trisphuta`, `dhruv_avayoga_sphuta`,
    `dhruv_kunda`
  - `dhruv_bhava_lagna`, `dhruv_hora_lagna`, `dhruv_ghati_lagna`,
    `dhruv_vighati_lagna`, `dhruv_varnada_lagna`, `dhruv_sree_lagna`,
    `dhruv_pranapada_lagna`, `dhruv_indu_lagna`

Each benchmark runs as a pair:

- `.../rust`: direct Rust call
- `.../ffi`: C ABI call

## Remaining (Intentional)

Unbenchmarked exports are intentionally excluded categories:

- lifecycle/resource APIs (`*_new`, `*_load`, `*_free`)
- static name/count/version lookup APIs

## Exclusions (Intentional)

These should not be benchmarked as ABI-vs-wrapped-function pairs:

- pure resource lifecycle glue (`*_load`, `*_free`)
- static name/count lookup APIs
- helpers explicitly treated as internals in wrapper-coverage docs

## Run

```bash
cargo bench -p dhruv_ffi_c --bench ffi_bench
```

## Saved Outputs

Benchmark logs and sorted summaries are saved under `docs/benchmarks/`:

- Raw run output: `docs/benchmarks/ffi_bench_<timestamp>.txt`
- Slowest-first summary: `docs/benchmarks/ffi_bench_<timestamp>_sorted.tsv`
