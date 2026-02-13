# dhruv_vedic_base C ABI Coverage

Scope: crate-root runtime free functions re-exported by `dhruv_vedic_base`.

Coverage is partial. The C ABI currently prioritizes frequently used query paths
and high-demand jyotish outputs, while many lower-level pure helpers are still
Rust-only.

## Wrapped Directly (Representative, audited)

- Ayanamsha/lunar nodes:
  - `dhruv_ayanamsha_deg`, `dhruv_ayanamsha_mean_deg`, `dhruv_ayanamsha_true_deg`
  - UTC variants for these (`*_utc`)
  - `dhruv_lunar_node_deg` (+ `_utc`)
- Rashi/nakshatra:
  - `dhruv_rashi_from_longitude`, `dhruv_rashi_from_tropical` (+ `_utc`)
  - `dhruv_nakshatra_from_longitude`, `dhruv_nakshatra_from_tropical` (+ `_utc`)
  - `dhruv_nakshatra28_from_longitude`, `dhruv_nakshatra28_from_tropical` (+ `_utc`)
- Rise/set:
  - `dhruv_approximate_local_noon_jd`
  - `dhruv_compute_rise_set`, `dhruv_compute_all_events` (+ `_utc`)
- Bhava/lagna/RAMC:
  - `dhruv_compute_bhavas` (+ `_utc`)
  - `dhruv_lagna_deg` (+ `_utc`) for `lagna_longitude_rad`
  - `dhruv_mc_deg` (+ `_utc`) for `mc_longitude_rad`
  - `dhruv_ramc_deg` (+ `_utc`) for `ramc_rad`
- Sphuta/special lagna family:
  - `dhruv_all_sphutas`, `dhruv_arudha_pada`
  - `dhruv_bhava_lagna`, `dhruv_hora_lagna`, `dhruv_ghati_lagna`,
    `dhruv_indu_lagna`, `dhruv_pranapada_lagna`, `dhruv_sree_lagna`,
    `dhruv_varnada_lagna`, `dhruv_vighati_lagna`
  - `dhruv_prana_sphuta`, `dhruv_deha_sphuta`, `dhruv_mrityu_sphuta`,
    `dhruv_tithi_sphuta`, `dhruv_yoga_sphuta`, `dhruv_yoga_sphuta_normalized`,
    `dhruv_rahu_tithi_sphuta`, `dhruv_bhrigu_bindu`, `dhruv_beeja_sphuta`,
    `dhruv_kshetra_sphuta`, `dhruv_chatussphuta`, `dhruv_panchasphuta`,
    `dhruv_trisphuta`, `dhruv_sookshma_trisphuta`, `dhruv_avayoga_sphuta`,
    `dhruv_kunda`
- Misc:
  - `dhruv_calculate_ashtakavarga`
  - `dhruv_deg_to_dms`
  - `dhruv_sun_based_upagrahas`
- Ashtakavarga APIs:
  - `dhruv_calculate_bav` for `calculate_bav`
  - `dhruv_calculate_all_bav` for `calculate_all_bav`
  - `dhruv_calculate_sav` for `calculate_sav`
  - `dhruv_trikona_sodhana` for `trikona_sodhana`
  - `dhruv_ekadhipatya_sodhana` for `ekadhipatya_sodhana`
- Drishti APIs:
  - `dhruv_graha_drishti` for `graha_drishti`
  - `dhruv_graha_drishti_matrix` for `graha_drishti_matrix`
- Classification/value helpers:
  - `dhruv_tithi_from_elongation` for `tithi_from_elongation`
  - `dhruv_karana_from_elongation` for `karana_from_elongation`
  - `dhruv_yoga_from_sum` for `yoga_from_sum`
  - `dhruv_vaar_from_jd` for `vaar_from_jd`
  - `dhruv_masa_from_rashi_index` for `masa_from_rashi_index`
  - `dhruv_ayana_from_sidereal_longitude` for `ayana_from_sidereal_longitude`
  - `dhruv_samvatsara_from_year` for `samvatsara_from_year`
  - `dhruv_rashi_lord` for `rashi_lord_by_index`
  - `dhruv_nth_rashi_from` for `nth_rashi_from`
  - `dhruv_ghatika_from_elapsed` for `ghatika_from_elapsed`
  - `dhruv_ghatikas_since_sunrise` for `ghatikas_since_sunrise`
  - `dhruv_hora_at` for `hora_at`
- Upagraha public API:
  - `dhruv_time_upagraha_jd` (+ `_utc`) for `time_upagraha_jd`

## Bhava/Lagna Coverage Detail

| Rust function | C ABI (JD variant) | C ABI (UTC variant) | Status |
|---|---|---|---|
| `compute_bhavas` | `dhruv_compute_bhavas` | `dhruv_compute_bhavas_utc` | Directly wrapped |
| `lagna_longitude_rad` | `dhruv_lagna_deg` | `dhruv_lagna_deg_utc` | Directly wrapped |
| `mc_longitude_rad` | `dhruv_mc_deg` | `dhruv_mc_deg_utc` | Directly wrapped |
| `lagna_and_mc_rad` | `dhruv_lagna_deg` + `dhruv_mc_deg` | Same with `_utc` variants | Functionally covered (no single-call ABI) |
| `ramc_rad` | `dhruv_ramc_deg` | `dhruv_ramc_deg_utc` | Directly wrapped |

## Missing Direct Wrappers (Important Gaps)

- Convenience gap:
  - `lagna_and_mc_rad` has no single-call C ABI helper.
- Intentionally not wrapped (internal helpers):
  - `day_portion_index`, `night_portion_index`, `portion_jd_range`,
    `time_upagraha_planet`
- Intentionally not wrapped (internal/math helpers):
  - `base_virupa`, `special_virupa`, `tdb_seconds_to_centuries`,
    `jd_tdb_to_centuries`, `normalize_360`

## Interpretation

- Current C ABI is usable for high-level runtime workflows.
- For intended external usage, coverage is effectively complete.
- Remaining non-covered items are either intentional internals or a convenience
  optimization (`lagna_and_mc_rad` single-call ABI).
