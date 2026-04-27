# Elixir Wrapper (`elixir-open`)

Open-source Elixir bindings for `ctara-dhruv-core`, implemented as a Rustler
NIF that calls the in-repo Rust crates directly.

## Status

- OTP app: `:ctara_dhruv`
- Binding strategy: source-built Rustler NIF (`native/dhruv_elixir_nif`)
- Package root: `bindings/elixir-open`
- Build mode: Hex package with source-built NIFs, no precompiled NIFs yet

## End-User Docs

Usage-first documentation for this wrapper lives in
[`../../docs/end_user/elixir/README.md`](../../docs/end_user/elixir/README.md).

## Install

Published installs use Hex:

```elixir
{:ctara_dhruv, "~> 0.1.0"}
```

The package is published from unified `vX.Y.Z` tags, but the Rustler NIF is
still compiled from source during `mix deps.compile`.

## Prerequisites

- Elixir 1.19+
- Erlang/OTP 28+
- Rust toolchain (`cargo`)

## Build

From `bindings/elixir-open`:

```bash
mix deps.get
mix test
```

Rustler compiles the NIF automatically during Mix compilation.

## Test

```bash
mix test
```

The ExUnit suite runs wrapper smoke coverage across the native families. Tests
that require SPK/LSK/EOP/tara data skip gracefully when those files are absent.

## Benchmark

```bash
mix run bench/all_functions.exs
```

Optional environment knobs:

- `DHRUV_BENCH_ITERATIONS=3`
- `DHRUV_BENCH_WARMUP=1`
- `DHRUV_BENCH_FILTER='Jyotish|Dasha'`

## Quickstart

```elixir
alias CtaraDhruv.{Engine, Ephemeris, Time}

{:ok, engine} =
  Engine.new(%{
    spk_paths: ["/abs/path/to/de442s.bsp"],
    lsk_path: "/abs/path/to/naif0012.tls",
    cache_capacity: 64,
    strict_validation: false
  })

{:ok, state} =
  Ephemeris.query(engine, %{
    target: :mars,
    observer: :solar_system_barycenter,
    frame: :eclip_j2000,
    epoch_tdb_jd: 2_451_545.0
  })

{:ok, time_result} =
  Time.utc_to_jd_tdb(engine, %{
    utc: %{year: 2024, month: 1, day: 1, hour: 12, minute: 0, second: 0.0},
    time_policy: %{mode: :hybrid_delta_t}
  })

IO.inspect(state)
IO.inspect(time_result.jd_tdb)
IO.inspect(time_result.diagnostics)

:ok = Engine.close(engine)
```

## Sidereal Chart Output

## Runtime SPK Replacement

Long-lived `CtaraDhruv.Engine` handles expose:

- `CtaraDhruv.Engine.replace_spks(engine, spk_paths)`
- `CtaraDhruv.Engine.list_spks(engine)`

Replacement swaps the full active SPK set atomically. Shared kernels are reused
when canonical path, file size, and modified time match.

The direct Vedic bhava surface is tropical unless you provide a
`sankranti_config`. The Elixir wrapper now exposes convenience arities for that
explicitly:

```elixir
alias CtaraDhruv.{Jyotish, Vedic}

location = %{latitude_deg: 28.6139, longitude_deg: 77.2090, altitude_m: 0.0}
utc = %{year: 2015, month: 1, day: 15, hour: 6, minute: 0, second: 0.0}
sidereal = %{ayanamsha_system: :lahiri, use_nutation: false}

{:ok, lagna} = Vedic.lagna(engine, %{utc: utc, location: location}, sidereal)
{:ok, bhavas} = Vedic.bhavas(engine, %{utc: utc, location: location}, sidereal)

{:ok, chart} =
  Jyotish.full_kundali(
    engine,
    %{utc: utc, location: location},
    sidereal
  )
```

Notes:

- `Vedic.lagna/2`, `Vedic.mc/2`, and `Vedic.bhavas/2` stay tropical when
  `:sankranti_config` is omitted.
- `Jyotish.full_kundali/3` applies the supplied ayanamsha to the full chart,
  including returned `bhava_cusps`.
- `full_kundali` now includes `graha_positions.lagna` by default.

## Amsha Notes

The Elixir wrapper exposes amsha-related behavior through
`CtaraDhruv.Jyotish`.

Dedicated amsha requests:

```elixir
{:ok, result} =
  Jyotish.amsha(engine, %{
    utc: utc,
    location: location,
    amsha_requests: [%{code: 9}, %{code: 2, variation: 1}],
    amsha_scope: %{
      include_bhava_cusps: true,
      include_arudha_padas: true,
      include_upagrahas: true,
      include_sphutas: true,
      include_special_lagnas: true,
      include_outer_planets: true
    }
  })
```

Variation discovery helpers live on `CtaraDhruv.Math`:

```elixir
{:ok, d2_catalog} = CtaraDhruv.Math.amsha_variations(%{amsha_code: 2})
{:ok, catalogs} = CtaraDhruv.Math.amsha_variations_many(%{amsha_codes: [2, 9]})
```

Embedded amsha configuration in `full_kundali`:

```elixir
{:ok, chart} =
  Jyotish.full_kundali(engine, %{
    utc: utc,
    location: location,
    full_kundali_config: %{
      include_amshas: true,
      amsha_selection: [%{code: 9}],
      amsha_scope: %{include_sphutas: true, include_special_lagnas: true}
    }
  })
```

Result maps may now include these optional amsha chart keys when requested and
available:

- `:bhava_cusps`
- `:arudha_padas`
- `:upagrahas`
- `:sphutas`
- `:special_lagnas`
- `:outer_planets`

Graha-position and longitude maps keep navagraha lists at length 9 and expose
Uranus, Neptune, and Pluto as sibling `:outer_planets` sections. Outer planets
are positional display entities only and do not participate in bala, avastha,
dasha, drishti, or lordship calculations.

Standalone bala request maps also accept `:amsha_selection` with the same
`[%{code: ..., variation: ...}]` shape used by `full_kundali`. Embedded
`full_kundali` `:amshas` results now include the full resolved amsha union used
internally by the call. Variation codes remain numeric on input, but each code
is now interpreted in the namespace of that request's amsha code.

## Coverage

Public modules included in this wrapper:

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

Each module returns `{:ok, value}` or
`{:error, %CtaraDhruv.Error{kind, message, details}}`. The only long-lived
wrapper-owned struct is `%CtaraDhruv.Engine{}`.

`CtaraDhruv.Time` now includes the intended helper subset
(`nutation_utc/2`, `approximate_local_noon/1`, `ayanamsha_system_count/0`,
`reference_plane_default/1`), `CtaraDhruv.Panchang` includes the composable
intermediate helpers, `CtaraDhruv.Math` covers the pure helper surface, and
`CtaraDhruv.Tara` exposes the low-level propagation/correction primitives in
addition to the main request/config compute API.

## Time-Based Upagraha Config

The Elixir wrapper accepts `:upagraha_config` maps for:

- `CtaraDhruv.Jyotish.upagrahas/2`
- `CtaraDhruv.Jyotish.bindus/2`
- `CtaraDhruv.Jyotish.full_kundali/2`

Supported keys are:

- `:gulika_point`, `:maandi_point`, `:other_point`
- `:gulika_planet`, `:maandi_planet`

Accepted values are strings or atoms matching:

- points: `start`, `middle`, `end`
- planets: `rahu`, `saturn`

## Notes

- The wrapper keeps the NIF boundary private in `CtaraDhruv.Native`.
- Most results are returned as atom-keyed maps and lists rather than large
  Elixir struct graphs.
- The default tara catalog is the embedded Rust catalog; loading a JSON catalog
  from disk is optional.
