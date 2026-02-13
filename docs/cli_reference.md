# `dhruv_cli` — Command-Line Reference

Debug and operations CLI for the ctara-dhruv ephemeris engine.

```
dhruv <COMMAND> [OPTIONS]
```

All engine-dependent commands require `--bsp` and `--lsk` kernel paths.
Location-dependent commands additionally require `--lat`, `--lon`, and `--eop`.

---

## Table of Contents

1. [Common Flags](#common-flags)
2. [Ephemeris Queries](#ephemeris-queries)
3. [Rashi / Nakshatra](#rashi--nakshatra)
4. [Ayanamsha / Nutation / Lunar Nodes](#ayanamsha--nutation--lunar-nodes)
5. [Lagna / Bhava / Sunrise](#lagna--bhava--sunrise)
6. [Panchang (combined)](#panchang-combined)
7. [Panchang Elements](#panchang-elements)
8. [Jyotish Composite](#jyotish-composite)
9. [Search: Lunar Phases](#search-lunar-phases)
10. [Search: Sankranti](#search-sankranti)
11. [Search: Conjunctions](#search-conjunctions)
12. [Search: Eclipses](#search-eclipses)
13. [Search: Stationary / Max Speed](#search-stationary--max-speed)
14. [Individual Sphuta Formulas](#individual-sphuta-formulas)
15. [Individual Special Lagna Formulas](#individual-special-lagna-formulas)
16. [Utility Primitives](#utility-primitives)
17. [Panchang Intermediates](#panchang-intermediates)
18. [Low-Level Ashtakavarga / Drishti](#low-level-ashtakavarga--drishti)
19. [C-Only Patterns Not in CLI](#c-only-patterns-not-in-cli)

---

## Common Flags

| Flag | Type | Description |
|---|---|---|
| `--date` | `YYYY-MM-DDThh:mm:ssZ` | UTC datetime |
| `--bsp` | path | SPK kernel (e.g. `de442s.bsp`) |
| `--lsk` | path | Leap-second kernel (e.g. `naif0012.tls`) |
| `--eop` | path | IERS EOP file (e.g. `finals2000A.all`) |
| `--lat` | f64 | Latitude in degrees (north positive) |
| `--lon` | f64 | Longitude in degrees (east positive) |
| `--alt` | f64 | Altitude in meters (default 0) |
| `--ayanamsha` | i32 | Ayanamsha system code (0-19, default 0=Lahiri) |
| `--nutation` | flag | Apply nutation correction |

---

## Ephemeris Queries

### `position` — Spherical coordinates of a body

```
dhruv position --date 2024-03-20T12:00:00Z --target 499 --observer 399 --bsp de442s.bsp --lsk naif0012.tls
```

| Flag | Description |
|---|---|
| `--target` | NAIF body code (10=Sun, 301=Moon, 499=Mars, etc.) |
| `--observer` | NAIF observer code (default 399=Earth) |

### `sidereal-longitude` — Sidereal longitude of a body

```
dhruv sidereal-longitude --date 2024-03-20T12:00:00Z --target 499 --bsp de442s.bsp --lsk naif0012.tls
```

Adds `--ayanamsha` and `--nutation` flags.

### `graha-longitudes` — All 9 graha sidereal longitudes

```
dhruv graha-longitudes --date 2024-03-20T12:00:00Z --bsp de442s.bsp --lsk naif0012.tls
```

---

## Rashi / Nakshatra

### `rashi` — Rashi from sidereal longitude (no engine)

```
dhruv rashi 45.5
```

### `nakshatra` — Nakshatra from sidereal longitude (no engine)

```
dhruv nakshatra 100.0
dhruv nakshatra 100.0 --scheme 28
```

### `rashi-tropical` — Rashi from tropical longitude + ayanamsha (no engine)

```
dhruv rashi-tropical 70.0 --ayanamsha 0 --jd 2460388.0 --nutation
```

### `nakshatra-tropical` — Nakshatra from tropical longitude + ayanamsha (no engine)

```
dhruv nakshatra-tropical 70.0 --ayanamsha 0 --jd 2460388.0 --nutation
```

### `dms` — Convert degrees to DMS (no engine)

```
dhruv dms 123.456
```

---

## Ayanamsha / Nutation / Lunar Nodes

### `ayanamsha-compute`

```
dhruv ayanamsha-compute --date 2024-03-20T12:00:00Z --ayanamsha 0 --nutation --bsp de442s.bsp --lsk naif0012.tls
```

### `nutation-compute`

```
dhruv nutation-compute --date 2024-03-20T12:00:00Z --bsp de442s.bsp --lsk naif0012.tls
```

### `lunar-node`

```
dhruv lunar-node --date 2024-03-20T12:00:00Z --node rahu --mode mean --bsp de442s.bsp --lsk naif0012.tls
```

---

## Lagna / Bhava / Sunrise

### `lagna-compute` — Lagna (Ascendant), MC, and RAMC

```
dhruv lagna-compute --date 2024-03-20T12:00:00Z --lat 28.6 --lon 77.2 --bsp de442s.bsp --lsk naif0012.tls --eop finals2000A.all
```

### `bhavas` — House cusps

```
dhruv bhavas --date 2024-03-20T12:00:00Z --lat 28.6 --lon 77.2 --bsp de442s.bsp --lsk naif0012.tls --eop finals2000A.all
```

### `sunrise` — Sunrise/sunset and twilight events

```
dhruv sunrise --date 2024-03-20T12:00:00Z --lat 28.6 --lon 77.2 --bsp de442s.bsp --lsk naif0012.tls --eop finals2000A.all
```

---

## Panchang (combined)

### `panchang` — All panchang elements for a date

```
dhruv panchang --date 2024-03-20T12:00:00Z --lat 28.6 --lon 77.2 --bsp de442s.bsp --lsk naif0012.tls --eop finals2000A.all --calendar
```

| Flag | Description |
|---|---|
| `--calendar` | Include masa, ayana, varsha |

---

## Panchang Elements

Each element can be queried independently.

### `tithi`

```
dhruv tithi --date 2024-03-20T12:00:00Z --bsp de442s.bsp --lsk naif0012.tls
```

### `karana`

```
dhruv karana --date 2024-03-20T12:00:00Z --bsp de442s.bsp --lsk naif0012.tls
```

### `yoga`

```
dhruv yoga --date 2024-03-20T12:00:00Z --ayanamsha 0 --bsp de442s.bsp --lsk naif0012.tls
```

### `moon-nakshatra`

```
dhruv moon-nakshatra --date 2024-03-20T12:00:00Z --ayanamsha 0 --bsp de442s.bsp --lsk naif0012.tls
```

### `vaar`

```
dhruv vaar --date 2024-03-20T12:00:00Z --lat 28.6 --lon 77.2 --bsp de442s.bsp --lsk naif0012.tls --eop finals2000A.all
```

### `hora`

```
dhruv hora --date 2024-03-20T12:00:00Z --lat 28.6 --lon 77.2 --bsp de442s.bsp --lsk naif0012.tls --eop finals2000A.all
```

### `ghatika`

```
dhruv ghatika --date 2024-03-20T12:00:00Z --lat 28.6 --lon 77.2 --bsp de442s.bsp --lsk naif0012.tls --eop finals2000A.all
```

### `masa`

```
dhruv masa --date 2024-03-20T12:00:00Z --ayanamsha 0 --bsp de442s.bsp --lsk naif0012.tls
```

### `ayana`

```
dhruv ayana --date 2024-03-20T12:00:00Z --ayanamsha 0 --bsp de442s.bsp --lsk naif0012.tls
```

### `varsha`

```
dhruv varsha --date 2024-03-20T12:00:00Z --ayanamsha 0 --bsp de442s.bsp --lsk naif0012.tls
```

---

## Jyotish Composite

### `graha-positions` — Comprehensive graha positions

```
dhruv graha-positions --date 2024-03-20T12:00:00Z --lat 28.6 --lon 77.2 \
  --bsp de442s.bsp --lsk naif0012.tls --eop finals2000A.all \
  --nakshatra --lagna --outer --bhava
```

### `sphutas` — All 16 sphutas

```
dhruv sphutas --date 2024-03-20T12:00:00Z --lat 28.6 --lon 77.2 \
  --bsp de442s.bsp --lsk naif0012.tls --eop finals2000A.all
```

### `special-lagnas` — All 8 special lagnas

```
dhruv special-lagnas --date 2024-03-20T12:00:00Z --lat 28.6 --lon 77.2 \
  --bsp de442s.bsp --lsk naif0012.tls --eop finals2000A.all
```

### `arudha-padas` — All 12 arudha padas

```
dhruv arudha-padas --date 2024-03-20T12:00:00Z --lat 28.6 --lon 77.2 \
  --bsp de442s.bsp --lsk naif0012.tls --eop finals2000A.all
```

### `ashtakavarga` — BAV + SAV

```
dhruv ashtakavarga --date 2024-03-20T12:00:00Z --lat 28.6 --lon 77.2 \
  --bsp de442s.bsp --lsk naif0012.tls --eop finals2000A.all
```

### `upagrahas` — All 11 upagrahas

```
dhruv upagrahas --date 2024-03-20T12:00:00Z --lat 28.6 --lon 77.2 \
  --bsp de442s.bsp --lsk naif0012.tls --eop finals2000A.all
```

### `core-bindus` — 19 curated sensitive points

```
dhruv core-bindus --date 2024-03-20T12:00:00Z --lat 28.6 --lon 77.2 \
  --bsp de442s.bsp --lsk naif0012.tls --eop finals2000A.all --nakshatra --bhava
```

### `drishti` — Graha drishti with virupa strength

```
dhruv drishti --date 2024-03-20T12:00:00Z --lat 28.6 --lon 77.2 \
  --bsp de442s.bsp --lsk naif0012.tls --eop finals2000A.all --bhava --lagna --bindus
```

---

## Search: Lunar Phases

| Command | Description |
|---|---|
| `next-purnima --date ... --bsp ... --lsk ...` | Next full moon |
| `prev-purnima --date ... --bsp ... --lsk ...` | Previous full moon |
| `next-amavasya --date ... --bsp ... --lsk ...` | Next new moon |
| `prev-amavasya --date ... --bsp ... --lsk ...` | Previous new moon |
| `search-purnimas --start ... --end ... --bsp ... --lsk ...` | Full moons in range |
| `search-amavasyas --start ... --end ... --bsp ... --lsk ...` | New moons in range |

---

## Search: Sankranti

| Command | Description |
|---|---|
| `next-sankranti --date ... --bsp ... --lsk ...` | Next solar ingress |
| `prev-sankranti --date ... --bsp ... --lsk ...` | Previous solar ingress |
| `search-sankrantis --start ... --end ... --bsp ... --lsk ...` | Solar ingresses in range |
| `next-specific-sankranti --date ... --rashi 0 --bsp ... --lsk ...` | Next entry into specific rashi |
| `prev-specific-sankranti --date ... --rashi 0 --bsp ... --lsk ...` | Previous entry from specific rashi |

All accept `--ayanamsha` and `--nutation`.

---

## Search: Conjunctions

| Command | Description |
|---|---|
| `next-conjunction --date ... --body1 10 --body2 301 --bsp ... --lsk ...` | Next conjunction |
| `prev-conjunction --date ... --body1 10 --body2 301 --bsp ... --lsk ...` | Previous conjunction |
| `search-conjunctions --start ... --end ... --body1 10 --body2 301 --bsp ... --lsk ...` | Conjunctions in range |

Body codes: 10=Sun, 301=Moon, 199=Mercury, 299=Venus, 499=Mars, 599=Jupiter, 699=Saturn.

---

## Search: Eclipses

| Command | Description |
|---|---|
| `next-chandra-grahan --date ... --bsp ... --lsk ...` | Next lunar eclipse |
| `prev-chandra-grahan --date ... --bsp ... --lsk ...` | Previous lunar eclipse |
| `search-chandra-grahan --start ... --end ... --bsp ... --lsk ...` | Lunar eclipses in range |
| `next-surya-grahan --date ... --bsp ... --lsk ...` | Next solar eclipse |
| `prev-surya-grahan --date ... --bsp ... --lsk ...` | Previous solar eclipse |
| `search-surya-grahan --start ... --end ... --bsp ... --lsk ...` | Solar eclipses in range |

---

## Search: Stationary / Max Speed

| Command | Description |
|---|---|
| `next-stationary --date ... --body 499 --bsp ... --lsk ...` | Next stationary point |
| `prev-stationary --date ... --body 499 --bsp ... --lsk ...` | Previous stationary point |
| `search-stationary --start ... --end ... --body 499 --bsp ... --lsk ...` | Stationary points in range |
| `next-max-speed --date ... --body 499 --bsp ... --lsk ...` | Next max-speed event |
| `prev-max-speed --date ... --body 499 --bsp ... --lsk ...` | Previous max-speed event |
| `search-max-speed --start ... --end ... --body 499 --bsp ... --lsk ...` | Max-speed events in range |

---

## Individual Sphuta Formulas

Pure math — no engine or kernel files required. All longitudes in degrees.

| Command | Flags | Formula |
|---|---|---|
| `bhrigu-bindu` | `--rahu --moon` | Midpoint(Rahu, Moon) |
| `prana-sphuta` | `--lagna --moon` | Lagna + Moon |
| `deha-sphuta` | `--moon --lagna` | Moon + Lagna |
| `mrityu-sphuta` | `--eighth-lord --lagna` | 8th lord + Lagna |
| `tithi-sphuta` | `--moon --sun --lagna` | (Moon - Sun) + Lagna |
| `yoga-sphuta` | `--sun --moon` | Sun + Moon |
| `yoga-sphuta-normalized` | `--sun --moon` | (Sun + Moon) mod 360 |
| `rahu-tithi-sphuta` | `--rahu --sun --lagna` | (Rahu - Sun) + Lagna |
| `kshetra-sphuta` | `--venus --moon --mars --jupiter --lagna` | Venus+Moon+Mars+Jupiter+Lagna |
| `beeja-sphuta` | `--sun --venus --jupiter` | Sun + Venus + Jupiter |
| `tri-sphuta` | `--lagna --moon --gulika` | Lagna + Moon + Gulika |
| `chatus-sphuta` | `--trisphuta --sun` | TriSphuta + Sun |
| `pancha-sphuta` | `--chatussphuta --rahu` | ChatusSphuta + Rahu |
| `sookshma-trisphuta` | `--lagna --moon --gulika --sun` | Lagna+Moon+Gulika+Sun |
| `avayoga-sphuta` | `--sun --moon` | Avayoga formula |
| `kunda` | `--lagna --moon --mars` | Lagna + Moon + Mars |

Example:
```
dhruv bhrigu-bindu --rahu 120.5 --moon 45.3
```

---

## Individual Special Lagna Formulas

Pure math — no engine required.

| Command | Flags |
|---|---|
| `bhava-lagna` | `--sun-lon --ghatikas` |
| `hora-lagna` | `--sun-lon --ghatikas` |
| `ghati-lagna` | `--sun-lon --ghatikas` |
| `vighati-lagna` | `--lagna-lon --vighatikas` |
| `varnada-lagna` | `--lagna-lon --hora-lagna-lon` |
| `sree-lagna` | `--moon-lon --lagna-lon` |
| `pranapada-lagna` | `--sun-lon --ghatikas` |
| `indu-lagna` | `--moon-lon --lagna-lord (0-8) --moon-9th-lord (0-8)` |

Example:
```
dhruv bhava-lagna --sun-lon 340.5 --ghatikas 12.3
```

---

## Utility Primitives

Pure math — no engine required.

| Command | Flags | Output |
|---|---|---|
| `tithi-from-elongation` | `--elongation` | Tithi name, paksha, degrees |
| `karana-from-elongation` | `--elongation` | Karana name, index, degrees |
| `yoga-from-sum` | `--sum` | Yoga name, index, degrees |
| `vaar-from-jd` | `--jd` | Weekday (Sanskrit + English) |
| `masa-from-rashi` | `--rashi (0-11)` | Masa name |
| `ayana-from-lon` | `--lon` | Uttarayana or Dakshinayana |
| `samvatsara-compute` | `--year` | Samvatsara name + cycle index |
| `nth-rashi-from` | `--rashi --offset` | Resulting rashi name + index |
| `rashi-lord` | `--rashi (0-11)` | Lord graha name |
| `normalize360` | `--deg` | Angle normalized to [0, 360) |
| `arudha-pada-compute` | `--cusp-lon --lord-lon` | Pada longitude + rashi |
| `sun-based-upagrahas` | `--sun-lon` | 5 upagraha longitudes |

Example:
```
dhruv tithi-from-elongation --elongation 45.0
dhruv vaar-from-jd --jd 2460388.5
dhruv samvatsara-compute --year 2024
```

---

## Panchang Intermediates

Engine required. Building blocks for custom panchang pipelines.

| Command | Flags | Output |
|---|---|---|
| `elongation-at` | `--date --bsp --lsk` | Moon-Sun elongation (degrees) |
| `sidereal-sum-at` | `--date --ayanamsha --nutation --bsp --lsk` | Sidereal Sun+Moon sum (degrees) |
| `body-lon-lat` | `--date --body (NAIF) --bsp --lsk` | Ecliptic lon + lat (degrees) |
| `vedic-day-sunrises` | `--date --lat --lon --alt --bsp --lsk --eop` | Today's + next sunrise JD |
| `tithi-at` | `--date --elongation --bsp --lsk` | Tithi with start/end from pre-computed elongation |
| `karana-at` | `--date --elongation --bsp --lsk` | Karana with start/end from pre-computed elongation |
| `yoga-at` | `--date --sum --ayanamsha --nutation --bsp --lsk` | Yoga with start/end from pre-computed sum |
| `nakshatra-at` | `--date --moon-sid --ayanamsha --nutation --bsp --lsk` | Nakshatra with start/end from pre-computed Moon longitude |

These allow composing custom workflows — compute intermediates once, reuse across
multiple classifiers:

```bash
# Compute elongation once, use for both tithi and karana
ELONG=$(dhruv elongation-at --date 2024-03-20T12:00:00Z --bsp de442s.bsp --lsk naif0012.tls)
dhruv tithi-at --date 2024-03-20T12:00:00Z --elongation ${ELONG%°} --bsp de442s.bsp --lsk naif0012.tls
dhruv karana-at --date 2024-03-20T12:00:00Z --elongation ${ELONG%°} --bsp de442s.bsp --lsk naif0012.tls
```

---

## Low-Level Ashtakavarga / Drishti

### `calculate-ashtakavarga` — Full BAV + SAV from rashi positions (no engine)

```
dhruv calculate-ashtakavarga --graha-rashis 0,3,7,10,4,2,9 --lagna-rashi 0
```

`--graha-rashis`: comma-separated rashi indices (0-11) for Sun, Moon, Mars, Mercury, Jupiter, Venus, Saturn.

### `graha-drishti-compute` — Single drishti between two points (no engine)

```
dhruv graha-drishti-compute --graha 2 --source 30.0 --target 210.0
```

`--graha`: graha index (0=Sun, 1=Moon, 2=Mars, ..., 8=Ketu).

### `graha-drishti-matrix-compute` — Full 9x9 aspect matrix (no engine)

```
dhruv graha-drishti-matrix-compute --longitudes 10,50,120,200,280,340,170,95,275
```

`--longitudes`: comma-separated sidereal longitudes for all 9 grahas (Sun through Ketu).

---

## C-Only Patterns Not in CLI

The following C ABI (`dhruv_ffi_c`) function categories have no CLI equivalents
because their purpose is handled natively by the CLI binary or Rust:

### Engine Lifecycle (9 FFI functions)

`dhruv_engine_new`, `dhruv_engine_free`, `dhruv_engine_query`, `dhruv_query_once`,
`dhruv_lsk_load`, `dhruv_lsk_free`, `dhruv_eop_load`, `dhruv_eop_free`,
`dhruv_api_version`.

**CLI equivalent:** The `load_engine()` and `load_eop()` helper functions construct
`Engine` and `EopKernel` on the stack per command invocation. Rust's `Drop` handles
cleanup. No explicit lifecycle management is needed or exposed.

### Memory Management (0 dedicated FFI functions)

The C ABI uses static strings and caller-allocated structs. The only heap operations
are the handle `_free` functions above.

**CLI equivalent:** The CLI binary manages all memory through Rust's ownership model.
There is nothing to expose.

### Name/Count Lookups (22 FFI functions)

`dhruv_rashi_name`, `dhruv_rashi_count`, `dhruv_nakshatra_name`, `dhruv_nakshatra28_name`,
`dhruv_nakshatra_count`, `dhruv_tithi_name`, `dhruv_karana_name`, `dhruv_yoga_name`,
`dhruv_vaar_name`, `dhruv_hora_name`, `dhruv_masa_name`, `dhruv_ayana_name`,
`dhruv_samvatsara_name`, `dhruv_graha_name`, `dhruv_graha_english_name`,
`dhruv_sphuta_name`, `dhruv_special_lagna_name`, `dhruv_arudha_pada_name`,
`dhruv_upagraha_name`, `dhruv_ayanamsha_system_count`, `dhruv_bhava_system_count`,
`dhruv_lunar_node_count`.

**CLI equivalent:** Every command that produces Vedic results already prints
human-readable names using the Rust enum `.name()` methods. For example, `dhruv rashi 45.5`
prints `Vrishabha (Taurus)`. Standalone name-lookup commands would be redundant.

### Config Defaults (6 FFI functions)

`dhruv_riseset_config_default`, `dhruv_bhava_config_default`,
`dhruv_conjunction_config_default`, `dhruv_grahan_config_default`,
`dhruv_stationary_config_default`, `dhruv_sankranti_config_default`.

**CLI equivalent:** Each command uses sensible defaults internally.
`RiseSetConfig::default()`, `BhavaConfig::default()`, etc. are applied
automatically when optional flags are omitted.

### UTC Convenience Helpers (4 FFI functions)

`dhruv_utc_to_tdb_jd`, `dhruv_jd_tdb_to_utc`, `dhruv_riseset_result_to_utc`,
`dhruv_query_utc_spherical`.

**CLI equivalent:** The CLI accepts `--date` as an ISO 8601 UTC string and
converts to TDB internally using `UtcDate::to_jd_tdb()`. Results are printed
as human-readable values. These FFI functions exist because C callers cannot
use Rust's `UtcDate` type.
