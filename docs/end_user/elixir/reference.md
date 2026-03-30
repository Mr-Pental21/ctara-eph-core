# Elixir Reference

This page summarizes the public Elixir wrapper from the `CtaraDhruv` modules,
using `bindings/elixir-open/lib/` as the source of truth.

## Public Modules

- `CtaraDhruv`
- `CtaraDhruv.Engine`
- `CtaraDhruv.Ephemeris`
- `CtaraDhruv.Time`
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

Returned dasha entity maps include `:name` with the exact canonical Sanskrit
entity name.

`CtaraDhruv.Tara`:

- `compute/2`
- `catalog_info/1`

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
- dasha request maps for hierarchy and snapshot queries

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

For build/runtime notes, see [`bindings/elixir-open/README.md`](../../../bindings/elixir-open/README.md).
