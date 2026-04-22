# CLI Reference

This page summarizes the public `dhruv` command surface from code in
`crates/dhruv_cli/src/main.rs`.

## Shared Flags And Config Groups

Common runtime inputs:

- `--date`
- `--bsp`
- `--lsk`
- `--eop`
- `--lat`
- `--lon`
- `--alt`
- `--ayanamsha`
- `--nutation`

Layered config and time-conversion controls:

- `--config`
- `--no-config`
- `--defaults-mode`
- `--time-policy`
- `--delta-t-model`
- `--future-delta-t-transition`
- `--future-transition-years`
- `--no-freeze-future-dut1`
- `--smh-future-family`
- `--stale-lsk-threshold-days`
- `--stale-eop-threshold-days`

Reusable public option groups:

- Upagraha configuration:
  - `--gulika-point`
  - `--maandi-point`
  - `--other-upagraha-point`
  - `--gulika-planet`
  - `--maandi-planet`
- Kundali amsha scope:
  - `--include-amshas`
  - `--amsha`
  - `--amsha-include-bhava-cusps`
  - `--amsha-include-arudha-padas`
  - `--amsha-include-upagrahas`
  - `--amsha-include-sphutas`
  - `--amsha-include-special-lagnas`

Shared value mappings worth knowing:

- upagraha points: `start`, `middle`, `end`
- Gulika/Maandi planets: `rahu`, `saturn`
- charakaraka schemes: `eight`, `seven-no-pitri`, `seven-pk-merged-mk`, `mixed-parashara`
- `--defaults-mode`: `recommended`, `none`
- `--time-policy`: `strict-lsk`, `hybrid-deltat`

## Command Families

Configuration and classifiers:

- `config-show-effective`
- `rashi`
- `nakshatra`
- `rashi-tropical`
- `nakshatra-tropical`
- `dms`
- `tithi-from-elongation`
- `karana-from-elongation`
- `yoga-from-sum`
- `vaar-from-jd`
- `masa-from-rashi`
- `ayana-from-lon`
- `samvatsara-compute`
- `nth-rashi-from`
- `rashi-lord`
- `normalize360`

Ephemeris and core astronomy:

- `position`
- `sidereal-longitude`
- `graha-longitudes`
- `ayanamsha-compute`
- `nutation-compute`
- `lunar-node`
- `body-lon-lat`

Rise/set, lagna, and bhava:

- `sunrise`
- `bhavas`
- `lagna-compute`
- `vedic-day-sunrises`

Panchang:

- `panchang`
- `tithi`
- `karana`
- `yoga`
- `moon-nakshatra`
- `vaar`
- `hora`
- `ghatika`
- `masa`
- `ayana`
- `varsha`
- `tithi-at`
- `karana-at`
- `yoga-at`
- `nakshatra-at`
- `elongation-at`
- `sidereal-sum-at`

Jyotish and chart building:

- `sphutas`
- `special-lagnas`
- `arudha-padas`
- `upagrahas`
- `graha-positions`
- `core-bindus`
- `drishti`
- `ashtakavarga`
- `charakaraka`
- `osculating-apogee`
- `shadbala`
- `bhavabala`
- `balas`
- `vimsopaka`
- `avastha`
- `kundali`

For `shadbala`, `vimsopaka`, `balas`, `avastha`, and `kundali`, use
`--amsha D<n>[:variation]` to override per-amsha variation selection. When
`kundali --include-amshas` is enabled, returned amsha charts include the full
resolved union of explicit selections and internally required bala/avastha
amshas.

`osculating-apogee` returns moving geocentric osculating apogee longitudes for
`Mangal,Buddh,Guru,Shukra,Shani`:

```bash
dhruv osculating-apogee --date 2026-04-17T13:25:39Z \
  --graha Mangal,Buddh,Guru --ayanamsha 0 --nutation
```

The output includes sidereal apogee longitude, ayanamsha, and reference-plane
longitude. Surya, Chandra, Rahu, and Ketu are invalid for this endpoint.

Amsha:

- `amsha`
- `amsha-chart`

Pure scalar jyotish formulas:

- `bhrigu-bindu`
- `prana-sphuta`
- `deha-sphuta`
- `mrityu-sphuta`
- `tithi-sphuta`
- `yoga-sphuta`
- `yoga-sphuta-normalized`
- `rahu-tithi-sphuta`
- `kshetra-sphuta`
- `beeja-sphuta`
- `tri-sphuta`
- `chatus-sphuta`
- `pancha-sphuta`
- `sookshma-trisphuta`
- `avayoga-sphuta`
- `kunda`
- `bhava-lagna`
- `hora-lagna`
- `ghati-lagna`
- `vighati-lagna`
- `varnada-lagna`
- `sree-lagna`
- `pranapada-lagna`
- `indu-lagna`
- `arudha-pada-compute`
- `sun-based-upagrahas`
- `calculate-ashtakavarga`
- `graha-drishti-compute`
- `graha-drishti-matrix-compute`

Search:

- `conjunction`
- `next-conjunction`
- `prev-conjunction`
- `search-conjunctions`
- `grahan`
- `next-chandra-grahan`
- `prev-chandra-grahan`
- `search-chandra-grahan`
- `next-surya-grahan`
- `prev-surya-grahan`
- `search-surya-grahan`
- `lunar-phase`
- `next-purnima`
- `prev-purnima`
- `search-purnimas`
- `next-amavasya`
- `prev-amavasya`
- `search-amavasyas`
- `sankranti`
- `next-sankranti`
- `prev-sankranti`
- `search-sankrantis`
- `next-specific-sankranti`
- `prev-specific-sankranti`
- `motion`
- `next-stationary`
- `prev-stationary`
- `search-stationary`
- `next-max-speed`
- `prev-max-speed`
- `search-max-speed`

Dasha and tara:

- `dasha`
- `tara-list`
- `tara-position`

The `dasha` command now uses one surface for both invocation styles:

- derived birth context via `--birth-date` plus `--lat` / `--lon`
- raw dasha context via `--birth-jd` plus input attributes such as
  `--moon-sid-lon`, `--graha-sidereal-lons`, `--lagna-sidereal-lon`,
  `--sunrise-jd`, and `--sunset-jd`

## Important Config Behavior

Time-based upagraha options affect:

- `upagrahas`
- `core-bindus`
- `kundali`

Amsha scope on `kundali` can promote dependent root sections automatically when
those sub-sections are requested.

`node_policy` and charakaraka scheme are public kundali/chart behavior knobs.

Layered config behavior is also public CLI behavior:

- explicit CLI flags override config files
- operation-specific config overrides common config
- recommended defaults apply unless `--defaults-mode none` is selected

For the full option-level reference, use [`docs/cli_reference.md`](../../cli_reference.md).
