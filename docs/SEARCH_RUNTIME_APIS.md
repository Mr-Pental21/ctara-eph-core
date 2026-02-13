# dhruv_search Runtime API (Query Functions Only)

This is the runtime/query surface of `dhruv_search` re-exported from `crates/dhruv_search/src/lib.rs`.

Total runtime functions documented here: **57**.

## Conjunction / Aspect (4)

| Function | Inputs | Output | What it does |
|---|---|---|---|
| `body_ecliptic_lon_lat` | `engine`, `body`, `jd_tdb` | `Result<(f64, f64), SearchError>` | Geocentric ecliptic lon/lat (degrees) for a body. |
| `next_conjunction` | `engine`, `body1`, `body2`, `jd_tdb`, `config` | `Result<Option<ConjunctionEvent>, SearchError>` | Next event where separation reaches target aspect angle. |
| `prev_conjunction` | `engine`, `body1`, `body2`, `jd_tdb`, `config` | `Result<Option<ConjunctionEvent>, SearchError>` | Previous event where separation reaches target angle. |
| `search_conjunctions` | `engine`, `body1`, `body2`, `jd_start`, `jd_end`, `config` | `Result<Vec<ConjunctionEvent>, SearchError>` | All target-separation events in a range. |

## Lunar Phase (6)

| Function | Inputs | Output | What it does |
|---|---|---|---|
| `next_purnima` | `engine`, `utc` | `Result<Option<LunarPhaseEvent>, SearchError>` | Next full moon after UTC instant. |
| `prev_purnima` | `engine`, `utc` | `Result<Option<LunarPhaseEvent>, SearchError>` | Previous full moon before UTC instant. |
| `next_amavasya` | `engine`, `utc` | `Result<Option<LunarPhaseEvent>, SearchError>` | Next new moon after UTC instant. |
| `prev_amavasya` | `engine`, `utc` | `Result<Option<LunarPhaseEvent>, SearchError>` | Previous new moon before UTC instant. |
| `search_purnimas` | `engine`, `start`, `end` | `Result<Vec<LunarPhaseEvent>, SearchError>` | All full moons in UTC range. |
| `search_amavasyas` | `engine`, `start`, `end` | `Result<Vec<LunarPhaseEvent>, SearchError>` | All new moons in UTC range. |

## Grahan (6)

| Function | Inputs | Output | What it does |
|---|---|---|---|
| `next_chandra_grahan` | `engine`, `jd_tdb`, `config` | `Result<Option<ChandraGrahan>, SearchError>` | Next lunar eclipse after `jd_tdb`. |
| `prev_chandra_grahan` | `engine`, `jd_tdb`, `config` | `Result<Option<ChandraGrahan>, SearchError>` | Previous lunar eclipse before `jd_tdb`. |
| `search_chandra_grahan` | `engine`, `jd_start`, `jd_end`, `config` | `Result<Vec<ChandraGrahan>, SearchError>` | All lunar eclipses in range. |
| `next_surya_grahan` | `engine`, `jd_tdb`, `config` | `Result<Option<SuryaGrahan>, SearchError>` | Next geocentric solar eclipse after `jd_tdb`. |
| `prev_surya_grahan` | `engine`, `jd_tdb`, `config` | `Result<Option<SuryaGrahan>, SearchError>` | Previous geocentric solar eclipse before `jd_tdb`. |
| `search_surya_grahan` | `engine`, `jd_start`, `jd_end`, `config` | `Result<Vec<SuryaGrahan>, SearchError>` | All geocentric solar eclipses in range. |

## Sankranti (5)

| Function | Inputs | Output | What it does |
|---|---|---|---|
| `next_sankranti` | `engine`, `utc`, `config` | `Result<Option<SankrantiEvent>, SearchError>` | Next Sun entry into any rashi. |
| `prev_sankranti` | `engine`, `utc`, `config` | `Result<Option<SankrantiEvent>, SearchError>` | Previous Sun entry into any rashi. |
| `search_sankrantis` | `engine`, `start`, `end`, `config` | `Result<Vec<SankrantiEvent>, SearchError>` | All sankrantis in UTC range. |
| `next_specific_sankranti` | `engine`, `utc`, `rashi`, `config` | `Result<Option<SankrantiEvent>, SearchError>` | Next Sun entry into a chosen rashi. |
| `prev_specific_sankranti` | `engine`, `utc`, `rashi`, `config` | `Result<Option<SankrantiEvent>, SearchError>` | Previous Sun entry into a chosen rashi. |

## Stationary / Max-Speed (6)

| Function | Inputs | Output | What it does |
|---|---|---|---|
| `next_stationary` | `engine`, `body`, `jd_tdb`, `config` | `Result<Option<StationaryEvent>, SearchError>` | Next stationary point after `jd_tdb`. |
| `prev_stationary` | `engine`, `body`, `jd_tdb`, `config` | `Result<Option<StationaryEvent>, SearchError>` | Previous stationary point before `jd_tdb`. |
| `search_stationary` | `engine`, `body`, `jd_start`, `jd_end`, `config` | `Result<Vec<StationaryEvent>, SearchError>` | All stationary points in range. |
| `next_max_speed` | `engine`, `body`, `jd_tdb`, `config` | `Result<Option<MaxSpeedEvent>, SearchError>` | Next speed extremum after `jd_tdb`. |
| `prev_max_speed` | `engine`, `body`, `jd_tdb`, `config` | `Result<Option<MaxSpeedEvent>, SearchError>` | Previous speed extremum before `jd_tdb`. |
| `search_max_speed` | `engine`, `body`, `jd_start`, `jd_end`, `config` | `Result<Vec<MaxSpeedEvent>, SearchError>` | All speed extrema in range. |

## Panchang (22)

| Function | Inputs | Output | What it does |
|---|---|---|---|
| `masa_for_date` | `engine`, `utc`, `sankranti_config` | `Result<MasaInfo, SearchError>` | Amanta month + adhika flag + boundaries. |
| `ayana_for_date` | `engine`, `utc`, `sankranti_config` | `Result<AyanaInfo, SearchError>` | Ayana + start/end transitions. |
| `varsha_for_date` | `engine`, `utc`, `sankranti_config` | `Result<VarshaInfo, SearchError>` | Samvatsara + Vedic year boundaries. |
| `elongation_at` | `engine`, `jd_tdb` | `Result<f64, SearchError>` | `(Moon_lon - Sun_lon) mod 360`. |
| `sidereal_sum_at` | `engine`, `jd_tdb`, `sankranti_config` | `Result<f64, SearchError>` | `(Moon_sid + Sun_sid) mod 360`. |
| `moon_sidereal_longitude_at` | `engine`, `jd_tdb`, `sankranti_config` | `Result<f64, SearchError>` | Moon sidereal longitude. |
| `nakshatra_for_date` | `engine`, `utc`, `sankranti_config` | `Result<PanchangNakshatraInfo, SearchError>` | Moon nakshatra/pada + boundaries. |
| `nakshatra_at` | `engine`, `jd_tdb`, `moon_sidereal_deg`, `sankranti_config` | `Result<PanchangNakshatraInfo, SearchError>` | Same using precomputed Moon sidereal longitude. |
| `tithi_for_date` | `engine`, `utc` | `Result<TithiInfo, SearchError>` | Tithi with paksha and boundaries. |
| `tithi_at` | `engine`, `jd_tdb`, `elongation_deg` | `Result<TithiInfo, SearchError>` | Same using precomputed elongation. |
| `karana_for_date` | `engine`, `utc` | `Result<KaranaInfo, SearchError>` | Karana with boundaries. |
| `karana_at` | `engine`, `jd_tdb`, `elongation_deg` | `Result<KaranaInfo, SearchError>` | Same using precomputed elongation. |
| `yoga_for_date` | `engine`, `utc`, `sankranti_config` | `Result<YogaInfo, SearchError>` | Yoga with boundaries. |
| `yoga_at` | `engine`, `jd_tdb`, `sidereal_sum_deg`, `sankranti_config` | `Result<YogaInfo, SearchError>` | Same using precomputed sidereal sum. |
| `vedic_day_sunrises` | `engine`, `eop`, `utc`, `location`, `riseset_config` | `Result<(f64, f64), SearchError>` | Sunrise and next-sunrise JD bounds for Vedic day. |
| `vaar_for_date` | `engine`, `eop`, `utc`, `location`, `riseset_config` | `Result<VaarInfo, SearchError>` | Vedic weekday from sunrise boundaries. |
| `vaar_from_sunrises` | `sunrise_jd`, `next_sunrise_jd`, `lsk` | `VaarInfo` | Weekday from sunrise pair (pure arithmetic). |
| `hora_for_date` | `engine`, `eop`, `utc`, `location`, `riseset_config` | `Result<HoraInfo, SearchError>` | Planetary hour with boundaries. |
| `hora_from_sunrises` | `jd_tdb`, `sunrise_jd`, `next_sunrise_jd`, `lsk` | `HoraInfo` | Hora from sunrise pair (pure arithmetic). |
| `ghatika_for_date` | `engine`, `eop`, `utc`, `location`, `riseset_config` | `Result<GhatikaInfo, SearchError>` | Ghatika with boundaries. |
| `ghatika_from_sunrises` | `jd_tdb`, `sunrise_jd`, `next_sunrise_jd`, `lsk` | `GhatikaInfo` | Ghatika from sunrise pair (pure arithmetic). |
| `panchang_for_date` | `engine`, `eop`, `utc`, `location`, `riseset_config`, `sankranti_config`, `include_calendar` | `Result<PanchangInfo, SearchError>` | One-shot panchang (7 limbs + optional calendar trio). |

## Jyotish Orchestration (8)

| Function | Inputs | Output | What it does |
|---|---|---|---|
| `graha_sidereal_longitudes` | `engine`, `jd_tdb`, `ayanamsha_system`, `use_nutation` | `Result<GrahaLongitudes, SearchError>` | 9 graha sidereal longitudes (incl. node handling). |
| `special_lagnas_for_date` | `engine`, `eop`, `utc`, `location`, `riseset_config`, `aya_config` | `Result<AllSpecialLagnas, SearchError>` | Computes all special lagnas. |
| `arudha_padas_for_date` | `engine`, `eop`, `utc`, `location`, `bhava_config`, `aya_config` | `Result<[ArudhaResult; 12], SearchError>` | Computes arudha padas for 12 houses. |
| `all_upagrahas_for_date` | `engine`, `eop`, `utc`, `location`, `riseset_config`, `aya_config` | `Result<AllUpagrahas, SearchError>` | Computes all 11 upagrahas. |
| `graha_positions` | `engine`, `eop`, `utc`, `location`, `bhava_config`, `aya_config`, `config` | `Result<GrahaPositions, SearchError>` | Extended graha-position API. |
| `ashtakavarga_for_date` | `engine`, `eop`, `utc`, `location`, `aya_config` | `Result<AshtakavargaResult, SearchError>` | Full ashtakavarga result. |
| `core_bindus` | `engine`, `eop`, `utc`, `location`, `bhava_config`, `riseset_config`, `aya_config`, `config` | `Result<BindusResult, SearchError>` | Curated bindu/sensitive points set. |
| `drishti_for_date` | `engine`, `eop`, `utc`, `location`, `bhava_config`, `riseset_config`, `aya_config`, `config` | `Result<DrishtiResult, SearchError>` | Graha drishti matrix (+ optional projections). |

## Related Detailed Docs

- Full inventory (includes helper methods): `docs/SEARCH_API_INVENTORY.md`
- Clean-room provenance: `docs/clean_room_conjunction.md`, `docs/clean_room_grahan.md`, `docs/clean_room_stationary.md`, `docs/clean_room_panchang.md`, `docs/clean_room_tithi_karana_yoga.md`, `docs/clean_room_ashtakavarga.md`, `docs/clean_room_drishti.md`, `docs/clean_room_upagraha.md`
