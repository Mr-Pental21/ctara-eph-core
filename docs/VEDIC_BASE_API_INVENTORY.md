# dhruv_vedic_base API Inventory

This document tracks the public crate-root API categories for `dhruv_vedic_base`.

## Runtime Function Surface

Full callable runtime functions are documented in:

- `docs/VEDIC_BASE_RUNTIME_APIS.md`

## Core Runtime Input/Output Types

- Location and rise/set:
  - `GeoLocation`
  - `RiseSetConfig`, `RiseSetEvent`, `RiseSetResult`, `SunLimb`
- Bhava:
  - `Bhava`, `BhavaConfig`, `BhavaResult`
  - `BhavaSystem`, `BhavaReferenceMode`, `BhavaStartingPoint`
- Ayanamsha and nodes:
  - `AyanamshaSystem`
  - `LunarNode`, `NodeMode`
- Rashi/nakshatra/tithi/yoga/karana/vaar/masa/samvatsara:
  - `Rashi`, `RashiInfo`, `Dms`
  - `Nakshatra`, `NakshatraInfo`, `Nakshatra28`, `Nakshatra28Info`
  - `Tithi`, `TithiPosition`, `Paksha`
  - `Yoga`, `YogaPosition`
  - `Karana`, `KaranaPosition`
  - `Vaar`
  - `Masa`
  - `Samvatsara`
- Graha / upagraha / drishti / ashtakavarga:
  - `Graha`
  - `Upagraha`, `AllUpagrahas`, `SunBasedUpagrahas`
  - `DrishtiEntry`, `GrahaDrishtiMatrix`
  - `BhinnaAshtakavarga`, `SarvaAshtakavarga`, `AshtakavargaResult`
- Special lagna / arudha / sphuta:
  - `SpecialLagna`, `AllSpecialLagnas`
  - `ArudhaPada`, `ArudhaResult`
  - `Sphuta`, `SphutalInputs`
- Errors:
  - `VedicError`

## Public Constants and Enumerated Sets

The crate root exports constant sets used for deterministic iteration:

- `ALL_RASHIS`, `ALL_NAKSHATRAS_27`, `ALL_NAKSHATRAS_28`
- `ALL_TITHIS`, `ALL_YOGAS`, `ALL_KARANAS`, `ALL_VAARS`, `ALL_MASAS`
- `ALL_GRAHAS`, `SAPTA_GRAHAS`
- `ALL_SPECIAL_LAGNAS`, `ALL_ARUDHA_PADAS`, `ALL_SPHUTAS`, `ALL_UPAGRAHAS`
- `ALL_SAMVATSARAS`, `ALL_AYANAS`

It also exports constants used by algorithms:

- `NAKSHATRA_SPAN_27`, `TITHI_SEGMENT_DEG`, `YOGA_SEGMENT_DEG`, `KARANA_SEGMENT_DEG`
- `GHATIKA_COUNT`, `GHATIKA_MINUTES`, `HORA_COUNT`
- `SAMVATSARA_EPOCH_YEAR`, `SAV_TOTAL`, `BAV_TOTALS`
- `CHALDEAN_SEQUENCE`, `GRAHA_KAKSHA_VALUES`, `TIME_BASED_UPAGRAHAS`

## Public Method Families (Value Helpers)

Many enum/value types expose `name()`, `index()`, and related helper methods,
for example:

- `Rashi::name`, `Rashi::western_name`, `Rashi::index`, `Rashi::all`
- `Nakshatra::name`, `Nakshatra::index`, `Nakshatra::all`
- `Tithi::name`, `Tithi::paksha`, `Tithi::tithi_in_paksha`, `Tithi::index`
- `Graha::name`, `Graha::english_name`, `Graha::index`, `Graha::naif_code`
- `Upagraha::name`, `Upagraha::index`, `Upagraha::is_time_based`

These helpers are part of the public API contract and are intentionally
kept pure and deterministic.
