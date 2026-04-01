# Elixir Reference

This page summarizes the public Elixir wrapper from the `CtaraDhruv` modules,
using `bindings/elixir-open/lib/` as the source of truth.

## Public Modules

- `CtaraDhruv`
- `CtaraDhruv.Engine`
- `CtaraDhruv.Ephemeris`
- `CtaraDhruv.Time`
- `CtaraDhruv.Math`
- `CtaraDhruv.Vedic`
- `CtaraDhruv.Panchang`
- `CtaraDhruv.Search`
- `CtaraDhruv.Jyotish`
- `CtaraDhruv.Dasha`
- `CtaraDhruv.Tara`

## Exact Function Inventory

`CtaraDhruv`:

- `new_engine/1`

`CtaraDhruv.Engine`:

- `new/1`
- `close/1`
- `load_config/2`
- `clear_config/1`
- `load_eop/2`
- `clear_eop/1`
- `load_tara_catalog/2`
- `reset_tara_catalog/1`

`load_config/2` accepts either:

- a string path convenience form
- or a request map with optional `:path` and `:defaults_mode`

When `:path` is omitted, discovery mode is used.

`new/1` accepts optional `:cache_capacity` and `:strict_validation`. When they
are omitted, the Elixir wrapper aligns with the shared wrapper convention:
`cache_capacity: 256` and `strict_validation: true`.

`CtaraDhruv.Ephemeris`:

- `query/2`
- `body_ecliptic_lon_lat/2`
- `cartesian_to_spherical/1`

`CtaraDhruv.Time`:

- `utc_to_jd_tdb/2`
  Requests may include `time_policy`; results now include `diagnostics`.
- `jd_tdb_to_utc/2`
- `nutation/1`
- `nutation_utc/2`
- `approximate_local_noon/1`
- `ayanamsha_system_count/0`
- `reference_plane_default/1`

`CtaraDhruv.Math`:

- classifiers and canonical name lookups:
  `rashi_from_longitude/1`, `nakshatra_from_longitude/1`,
  `nakshatra28_from_longitude/1`, `rashi_from_tropical/1`,
  `nakshatra_from_tropical/1`, `nakshatra28_from_tropical/1`,
  `graha_name/1`, `yogini_name/1`, `rashi_name/1`, `nakshatra_name/1`,
  `nakshatra28_name/1`, `sphuta_name/1`, `upagraha_name/1`
- relationship, dignity, combustion, and lord helpers:
  `hora_lord/1`, `masa_lord/1`, `samvatsara_lord/1`,
  `exaltation_degree/1`, `debilitation_degree/1`, `moolatrikone_range/1`,
  `combustion_threshold/1`, `combust?/1`, `all_combustion_status/1`,
  `naisargika_maitri/1`, `tatkalika_maitri/1`, `panchadha_maitri/1`,
  `dignity_in_rashi/1`, `dignity_in_rashi_with_positions/1`,
  `node_dignity_in_rashi/1`, `natural_benefic_malefic/1`,
  `moon_benefic_nature/1`, `graha_gender/1`
- drishti, upagraha, sphuta, and ashtakavarga helpers:
  `graha_drishti/1`, `graha_drishti_matrix/1`, `sun_based_upagrahas/1`,
  `time_upagraha_jd/1`, `all_sphutas/1`, `calculate_ashtakavarga/1`,
  `calculate_bav/1`, `calculate_all_bav/1`, `calculate_sav/1`,
  `trikona_sodhana/1`, `ekadhipatya_sodhana/1`

`CtaraDhruv.Vedic`:

- `ayanamsha/2`
- `lunar_node/2`
- `rise_set/2`
- `all_events/2`
- `lagna/2`
- `lagna/3`
- `mc/2`
- `mc/3`
- `ramc/2`
- `bhavas/2`
- `bhavas/3`

`CtaraDhruv.Panchang`:

- `tithi/2`
- `karana/2`
- `yoga/2`
- `nakshatra/2`
- `vaar/2`
- `hora/2`
- `ghatika/2`
- `masa/2`
- `ayana/2`
- `varsha/2`
- `daily/2`
- `elongation_at/2`
- `sidereal_sum_at/2`
- `vedic_day_sunrises/2`
- `body_ecliptic_lon_lat/2`
- `tithi_at/2`
- `karana_at/2`
- `yoga_at/2`
- `nakshatra_at/2`
- `vaar_from_sunrises/2`
- `hora_from_sunrises/2`
- `ghatika_from_sunrises/2`
- `ghatika_from_elapsed/1`
- `ghatikas_since_sunrise/1`

`CtaraDhruv.Jyotish`:

- `graha_longitudes/2`
- `graha_positions/2`
- `special_lagnas/2`
- `arudha/2`
- `upagrahas/2`
- `bindus/2`
- `ashtakavarga/2`
- `drishti/2`
- `charakaraka/2`
- `shadbala/2`
- `bhavabala/2`
- `vimsopaka/2`
- `balas/2`
- `avastha/2`
- `full_kundali/2`
- `full_kundali/3`
- `amsha/2`

`CtaraDhruv.Search`:

- `conjunction/2`
- `grahan/2`
- `lunar_phase/2`
- `sankranti/2`
- `motion/2`

`CtaraDhruv.Dasha`:

- `hierarchy/2`
- `snapshot/2`
- `level0/2`
- `level0_entity/2`
- `children/2`
- `child_period/2`
- `complete_level/2`

Returned dasha entity maps include `:name` with the exact canonical Sanskrit
entity name.

`CtaraDhruv.Tara`:

- `compute/2`
- `catalog_info/1`
- `propagate_position/1`
- `apply_aberration/1`
- `apply_light_deflection/1`
- `galactic_anticenter_icrs/0`

## Request Config Maps

Common request maps can include:

- `utc`
- `location`
- `sankranti_config`
- `riseset_config`
- `bhava_config`
- `time_policy`

Chart-related config maps:

- `graha_positions_config`
- `bindus_config`
- `drishti_config`
- `full_kundali_config`
- `amsha_scope`
- `amsha_selection`
- search request maps with `:op`-specific fields for conjunction, grahan, lunar phase, sankranti, and motion
- dasha request maps for hierarchy, snapshot, `level0`, `level0_entity`, `children`, `child_period`, and `complete_level` queries

Time-based upagraha config map:

- `gulika_point`
- `maandi_point`
- `other_point`
- `gulika_planet`
- `maandi_planet`

Accepted enum-style string values:

- points: `"start"`, `"middle"`, `"end"`
- planets: `"rahu"`, `"saturn"`

## Wrapper Behavior Notes

- `bindus_config` and `full_kundali_config` can both carry `upagraha_config`.
- `full_kundali/3`, `lagna/3`, `mc/3`, and `bhavas/3` are convenience arities that inject `:sankranti_config`.
- Enum-like strings are normalized through the NIF boundary and usually come back as atoms in results.
- `full_kundali_config[:dasha_config]` supports `:systems`, `:max_level`,
  `:max_levels`, and `:snapshot_utc`.

For build/runtime notes, see [`bindings/elixir-open/README.md`](../../../bindings/elixir-open/README.md).
