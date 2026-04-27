# Python Reference

This page summarizes the public Python wrapper by module, using
`bindings/python-open/src/ctara_dhruv/` as the source of truth.

## Top-Level Package Surface

`ctara_dhruv.__init__` intentionally re-exports a compact surface:

- engine lifecycle: `Engine`, `init`, `engine`, `lsk`, `eop`
- runtime SPK replacement: `replace_spks`, `SpkReplaceReport`, `LoadedSpkInfo`
- selected enums from `enums`
- selected dataclasses from `types`
- ephemeris helpers: `query`, `body_ecliptic_lon_lat`, `cartesian_to_spherical`
- time helpers: `utc_to_jd_tdb`, `jd_tdb_to_utc`, `nutation`
  `utc_to_jd_tdb` now takes a typed `UtcToTdbRequest` and returns `UtcToTdbResult`, including `TimeDiagnostics` / `TimeWarning`.
- ayanamsha helpers: `ayanamsha`, `system_count`, `reference_plane_default`
- tara: `TaraCatalog`
- dasha classes and helpers

The fuller public surface is intentionally module-based.

## Public Modules

- `ctara_dhruv.engine`
- `ctara_dhruv.ephemeris`
- `ctara_dhruv.time`
- `ctara_dhruv.ayanamsha`
- `ctara_dhruv.vedic`
- `ctara_dhruv.panchang`
- `ctara_dhruv.kundali`
- `ctara_dhruv.shadbala`
- `ctara_dhruv.amsha`
- `ctara_dhruv.dasha`
- `ctara_dhruv.search`
- `ctara_dhruv.tara`
- public support modules: `ctara_dhruv.enums`, `ctara_dhruv.types`

## Public Module Inventory

`engine`:

- `Engine`
- `SpkReplaceReport`
- `LoadedSpkInfo`
- `init`
- `engine`
- `lsk`
- `eop`
- `replace_spks`

`Engine.replace_spks(spk_paths)` atomically replaces the active SPK set on a
long-lived engine, and `Engine.list_spks()` returns active SPKs in query order.
The module-level `replace_spks(spk_paths)` helper applies to the initialized
singleton engine. CLI commands are short-lived and do not expose runtime SPK
replacement.

`ephemeris`:

- `query`
- `query_once`
- `body_ecliptic_lon_lat`
- `cartesian_to_spherical`

`time`:

- `utc_to_jd_tdb`
- `jd_tdb_to_utc`
- `nutation`
- `nutation_utc`
- `approximate_local_noon_jd`
- typed UTC-conversion support types from `types`:
  `UtcToTdbRequest`, `UtcToTdbResult`, `TimePolicy`, `TimeConversionOptions`, `TimeDiagnostics`, `TimeWarning`

`ayanamsha`:

- `ayanamsha`
- `system_count`
- `reference_plane_default`
- `sidereal_sum_at`

`vedic`:

- rise/set defaults and helpers:
  - `riseset_config_default`
  - `compute_rise_set`
  - `sunrise`
  - `sunset`
  - `compute_all_events`
  - `vedic_day_sunrises`
  - `compute_rise_set_utc`
  - `compute_all_events_utc`
  - `approximate_local_noon_jd`
  - `riseset_result_to_utc`
- bhava and lagna helpers:
  - `bhava_config_default`
  - `bhava_system_count`
  - `compute_bhavas`
  - `compute_bhavas_utc`
  - `lagna_deg`
  - `mc_deg`
  - `ramc_deg`
  - `lagna_deg_utc`
  - `mc_deg_utc`
  - `ramc_deg_utc`
- rashi, nakshatra, names, and pure classifiers:
  - `rashi_from_longitude`
  - `nakshatra_from_longitude`
  - `nakshatra28_from_longitude`
  - `rashi_from_tropical`
  - `nakshatra_from_tropical`
  - `nakshatra28_from_tropical`
  - `rashi_from_tropical_utc`
  - `nakshatra_from_tropical_utc`
  - `nakshatra28_from_tropical_utc`
  - `deg_to_dms`
  - `rashi_count`
  - `nakshatra_count`
  - `rashi_lord`
  - `rashi_name`
  - `nakshatra_name`
  - `nakshatra28_name`
  - `graha_name`
  - `yogini_name`
  - `tithi_name`
  - `karana_name`
  - `yoga_name`
  - `vaar_name`
  - `hora_name`
  - `masa_name`
  - `ayana_name`
  - `samvatsara_name`
  - `sphuta_name`
  - `special_lagna_name`
  - `arudha_pada_name`
  - `upagraha_name`
  - `nth_rashi_from`
  - `tithi_from_elongation`
  - `karana_from_elongation`
  - `yoga_from_sum`
  - `vaar_from_jd`
  - `masa_from_rashi_index`
  - `ayana_from_sidereal_longitude`
  - `samvatsara_from_year`
  - `ghatika_from_elapsed`
  - `ghatikas_since_sunrise`
  - `hora_at`
- lunar node helpers:
  - `lunar_node_deg`
  - `lunar_node_deg_with_engine`
  - `lunar_node_compute_ex`
  - `lunar_node_count`
  - `lunar_node_deg_utc`
  - `lunar_node_deg_utc_with_engine`
- sphuta and special-lagna helpers:
  - `all_sphutas`
  - `bhrigu_bindu`
  - `prana_sphuta`
  - `deha_sphuta`
  - `mrityu_sphuta`
  - `tithi_sphuta`
  - `yoga_sphuta`
  - `yoga_sphuta_normalized`
  - `rahu_tithi_sphuta`
  - `kshetra_sphuta`
  - `beeja_sphuta`
  - `trisphuta`
  - `chatussphuta`
  - `panchasphuta`
  - `sookshma_trisphuta`
  - `avayoga_sphuta`
  - `kunda`
  - `special_lagnas_for_date`
  - `bhava_lagna`
  - `hora_lagna`
  - `ghati_lagna`
  - `vighati_lagna`
  - `varnada_lagna`
  - `sree_lagna`
  - `pranapada_lagna`
  - `indu_lagna`
  - `arudha_pada`
  - `arudha_padas_for_date`
  - `sun_based_upagrahas`
- time-based upagraha helpers:
  - `time_upagraha_config_default`
  - `time_upagraha_jd`
  - `time_upagraha_jd_utc`
  - `all_upagrahas_for_date`
- graha relationship, combustion, dignity, and classification helpers:
  - `exaltation_degree`
  - `debilitation_degree`
  - `moolatrikone_range`
  - `combustion_threshold`
  - `is_combust`
  - `all_combustion_status`
  - `naisargika_maitri`
  - `tatkalika_maitri`
  - `panchadha_maitri`
  - `dignity_in_rashi`
  - `dignity_in_rashi_with_positions`
  - `node_dignity_in_rashi`
  - `natural_benefic_malefic`
  - `moon_benefic_nature`
  - `graha_gender`
  - `hora_lord`
  - `masa_lord`
  - `samvatsara_lord`
- ashtakavarga and drishti:
  - `calculate_bav`
  - `calculate_all_bav`
  - `calculate_sav`
  - `calculate_ashtakavarga`
  - `trikona_sodhana`
  - `ekadhipatya_sodhana`
  - `ashtakavarga_for_date`
  - `graha_drishti`
  - `graha_drishti_matrix`
  - `drishti_for_date`

`panchang`:

- include-mask constants:
  - `INCLUDE_TITHI`
  - `INCLUDE_KARANA`
  - `INCLUDE_YOGA`
  - `INCLUDE_VAAR`
  - `INCLUDE_HORA`
  - `INCLUDE_GHATIKA`
  - `INCLUDE_NAKSHATRA`
  - `INCLUDE_MASA`
  - `INCLUDE_AYANA`
  - `INCLUDE_VARSHA`
  - `INCLUDE_ALL_CORE`
  - `INCLUDE_ALL_CALENDAR`
  - `INCLUDE_ALL`
- functions:
  - `panchang`
  - `tithi_for_date`
  - `karana_for_date`
  - `yoga_for_date`
  - `nakshatra_for_date`
  - `vaar_for_date`
  - `hora_for_date`
  - `ghatika_for_date`
  - `masa_for_date`
  - `ayana_for_date`
  - `varsha_for_date`
  - `nakshatra_at`
  - `samvatsara_from_year`
  - `elongation_at`
  - `tithi_at`
  - `karana_at`
  - `yoga_at`
  - `vaar_from_sunrises`
  - `hora_from_sunrises`
  - `ghatika_from_sunrises`

`kundali`:

- `graha_longitudes`
  Accepts optional `GrahaLongitudesConfig` with `GrahaLongitudeKind` / `PrecessionModel` selectors, or the default sidereal settings via keyword args.
  Results keep `longitudes` as the 9 navagrahas and expose Uranus, Neptune,
  and Pluto separately as `outer_planets`.
- `moving_osculating_apogees_for_date`
  Returns moving heliocentric osculating apogees for graha indices 2..6
  (`Mangal,Buddh,Guru,Shukra,Shani`) with sidereal longitude, ayanamsha, and
  reference-plane longitude.
- `graha_positions`
  Results keep `grahas` as the 9 navagrahas and expose positional-only
  Uranus, Neptune, and Pluto separately as `outer_planets`.
- `core_bindus`
- `charakaraka_for_date`
- `full_kundali_config_default`
- `full_kundali`

`shadbala`:

- `shadbala`
- `calculate_bhavabala`
- `bhavabala`
- `vimsopaka`
- `balas`
- `avastha`

The standalone bala helpers accept an `amsha_selection` argument. Embedded
`full_kundali(...).amshas` returns the full resolved amsha union used by the
call. Use `amsha_variations` and `amsha_variations_many` to discover valid
variation codes, names, labels, and defaults for each amsha.

Avastha entries expose `deeptadi` as the primary compatibility index and
`deeptadi_states` / `deeptadi_mask` as the full set of Deeptadi states that
apply to the graha. They also expose `lajjitadi`, `lajjitadi_states`, and
`lajjitadi_mask`; `lajjitadi is None` when no Lajjitadi condition applies.

`amsha`:

- `amsha_longitude`
- `amsha_longitudes`
- `amsha_rashi_info`
- `amsha_chart_for_date`
  Amsha charts keep `grahas` as 9 entries and expose transformed
  `outer_planets` separately when the scope includes them.
- `amsha_variations`
- `amsha_variations_many`

`dasha`:

- `DashaLevel`
- `DashaHierarchy`
- `dasha_selection_config_default`
- `dasha_variation_config_default`
- `dasha_hierarchy`
- `dasha_snapshot`
- `dasha_level0`
- `dasha_level0_entity`
- `dasha_children`
- `dasha_child_period`
- `dasha_complete_level`

Dasha period objects expose `entity_name` with the exact canonical Sanskrit
entity name, plus structured `start_utc` / `end_utc` alongside `start_jd` /
`end_jd`. Dasha snapshots similarly expose `query_utc` alongside `query_jd`.
Python dasha calls can use either UTC/location birth context or `birth_jd` plus
precomputed `inputs` on the same main functions.

Chara-style dasha periods use dual lordship for Kumbha (`Shani`/`Rahu`) and
Vrischika (`Mangal`/`Ketu`). Rahu owns Kumbha and Ketu owns Vrischika for the
default sign-lord-based node dignity policy.

`search`:

- `conjunction_config_default`
- `next_conjunction`
- `prev_conjunction`
- `search_conjunctions`
- `grahan_config_default`
- `next_lunar_eclipse`
- `prev_lunar_eclipse`
- `next_solar_eclipse`
- `prev_solar_eclipse`
- `search_lunar_eclipses`
- `search_solar_eclipses`
- `stationary_config_default`
- `next_stationary`
- `prev_stationary`
- `search_stationary`
- `next_max_speed`
- `prev_max_speed`
- `search_max_speeds`
- `next_purnima`
- `prev_purnima`
- `next_amavasya`
- `prev_amavasya`
- `search_lunar_phases`
- `sankranti_config_default`

Python high-level search results expose structured Gregorian UTC wherever the
result itself represents an event time. Conjunction, grahan, stationary, and
max-speed results now carry UTC alongside the existing JD values; sankranti and
lunar-phase results continue to expose UTC directly.

The same main Python search functions accept either `UtcTime` inputs or numeric
JD/TDB transport on their existing parameters; no separate `*_utc` helper
families are required.
- `next_sankranti`
- `prev_sankranti`
- `specific_sankranti`
- `search_sankrantis`

Python range-search helpers auto-expand their internal buffers until the full
result set is returned. `max_results` is only the initial internal chunk size,
not a public truncation cap.

`tara`:

- `TaraCatalog`
- `tara_compute`
- `propagate_position`
- `apply_aberration`
- `apply_light_deflection`
- `galactic_anticenter_icrs`
- `galactic_center_ecliptic`

## Public Config Families

Common dict-style or struct-style config inputs:

- `bhava_config`
- `riseset_config`
- `sankranti_config`
- `bindus_config`
- `drishti_config`
- `variation_config`
- amsha `scope`
- search configs returned by:
  - `conjunction_config_default`
  - `grahan_config_default`
  - `stationary_config_default`
  - `sankranti_config_default`
- full-kundali config returned by `full_kundali_config_default`
- dasha configs returned by `dasha_selection_config_default` and `dasha_variation_config_default`

For embedded full-kundali dasha snapshots, the CFFI dasha selection config now
uses `snapshot_time` with `time_kind`, `utc`, and `jd_utc`. Prefer
`time_kind = DHRUV_DASHA_TIME_UTC` plus `snapshot_time.utc` on high-level
wrapper calls.

Time-based upagraha config fields:

- `gulika_point`
- `maandi_point`
- `other_point`
- `gulika_planet`
- `maandi_planet`

Accepted upagraha values:

- points: `"start"`, `"middle"`, `"end"`
- planets: `"rahu"`, `"saturn"`

## Notes

- The package exposes some symbols at top level in `ctara_dhruv.__init__`, but the fuller surface is module-based.
- Config names are mostly `snake_case`.
- Public enums live in `ctara_dhruv.enums`; public dataclasses and result objects live in `ctara_dhruv.types`.
- For wrapper setup and build notes, see [`bindings/python-open/README.md`](../../../bindings/python-open/README.md).

## Rashi-Bhava Bhava Config

Python `bhava_config` dictionaries may set `use_rashi_bhava_for_bala_avastha`, `include_rashi_bhava_results`, and `include_special_bhavabala_rules`; all default to `1`. They may also set `include_node_aspects_for_drik_bala`, defaulting to `0`, to include Rahu/Ketu incoming aspects in Shadbala Drik Bala and Bhava Bala Drishti Bala. `divide_guru_buddh_drishti_by_4_for_drik_bala` defaults to `1`; set it to `0` to add Guru/Buddh incoming aspects at full signed strength instead of through the divided Drik Bala balance. `chandra_benefic_rule` defaults to `0` for the 72-degree brightness rule; set it to `1` for the 0..=180-degree waxing arc rule. The same rule is used by Buddh's association-based nature in Shadbala Drik Bala and Bhava Bala Drishti Bala. `sayanadi_ghatika_rounding` defaults to `0` for floor; set it to `1` for ceil. Existing fields such as `bhava_cusps` and `bhava_number` remain configured-system outputs. New sibling fields such as `rashi_bhava_cusps`, `rashi_bhava_number`, and `graha_to_rashi_bhava` expose the rashi-bhava/equal-house basis.
