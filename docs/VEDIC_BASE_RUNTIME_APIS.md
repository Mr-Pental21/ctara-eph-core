# dhruv_vedic_base Runtime APIs

This document lists crate-root runtime free functions re-exported by
`dhruv_vedic_base`, grouped by domain.

## Ayanamsha, Nodes, and Calendar Classifiers

| Function | Output | Purpose |
|---|---|---|
| `ayana_from_sidereal_longitude` | `Ayana` | Determine ayana from sidereal Sun longitude. |
| `ayanamsha_deg` | `f64` | Compute ayanamsha (optional nutation correction). |
| `ayanamsha_mean_deg` | `f64` | Mean ayanamsha at epoch. |
| `ayanamsha_true_deg` | `f64` | True (nutation-corrected) ayanamsha at epoch. |
| `jd_tdb_to_centuries` | `f64` | Julian Date TDB to Julian centuries since J2000.0. |
| `tdb_seconds_to_centuries` | `f64` | TDB seconds past J2000.0 to Julian centuries. |
| `lunar_node_deg` | `f64` | Unified node longitude API (Rahu/Ketu, mean/true). |
| `mean_rahu_deg` | `f64` | Mean Rahu longitude. |
| `mean_ketu_deg` | `f64` | Mean Ketu longitude. |
| `true_rahu_deg` | `f64` | True Rahu longitude. |
| `true_ketu_deg` | `f64` | True Ketu longitude. |
| `masa_from_rashi_index` | `Masa` | Rashi index to masa mapping. |
| `samvatsara_from_year` | `(Samvatsara, u8)` | CE year to samvatsara (+ index). |

## Rashi / Nakshatra / Tithi / Karana / Yoga / Vaar

| Function | Output | Purpose |
|---|---|---|
| `rashi_from_longitude` | `RashiInfo` | Rashi from sidereal longitude. |
| `rashi_from_tropical` | `RashiInfo` | Rashi from tropical longitude + ayanamsha. |
| `rashi_lord` | `Graha` | Planetary lord of rashi enum. |
| `rashi_lord_by_index` | `Option<Graha>` | Planetary lord of rashi index. |
| `nth_rashi_from` | `u8` | N-th rashi (modulo 12) from starting rashi. |
| `nakshatra_from_longitude` | `NakshatraInfo` | Nakshatra+pada (27-scheme) from sidereal longitude. |
| `nakshatra_from_tropical` | `NakshatraInfo` | Nakshatra+pada from tropical longitude + ayanamsha. |
| `nakshatra28_from_longitude` | `Nakshatra28Info` | Nakshatra (28-scheme) from sidereal longitude. |
| `nakshatra28_from_tropical` | `Nakshatra28Info` | Nakshatra (28-scheme) from tropical longitude + ayanamsha. |
| `tithi_from_elongation` | `TithiPosition` | Tithi from Moon-Sun elongation. |
| `karana_from_elongation` | `KaranaPosition` | Karana from Moon-Sun elongation. |
| `yoga_from_sum` | `YogaPosition` | Yoga from sidereal Sun+Moon sum. |
| `vaar_from_jd` | `Vaar` | Weekday from Julian Date. |
| `vaar_day_lord` | `Hora` | Day lord (hora lord) for vaar. |
| `hora_at` | `Hora` | Hora lord by vaar and hora index. |
| `deg_to_dms` | `Dms` | Decimal degrees to DMS. |

## Rise/Set and Positional Astronomical Helpers

| Function | Output | Purpose |
|---|---|---|
| `approximate_local_noon_jd` | `f64` | Approximate local solar noon JD. |
| `compute_rise_set` | `Result<RiseSetResult, VedicError>` | Compute one rise/set event. |
| `compute_all_events` | `Result<Vec<RiseSetResult>, VedicError>` | Compute all configured rise/set events. |
| `compute_bhavas` | `Result<BhavaResult, VedicError>` | Compute bhava cusps and metadata for configured bhava system. |
| `lagna_longitude_rad` | `Result<f64, VedicError>` | Lagna longitude in radians. |
| `mc_longitude_rad` | `Result<f64, VedicError>` | MC longitude in radians. |
| `lagna_and_mc_rad` | `Result<(f64, f64), VedicError>` | Lagna + MC in one call. |
| `ramc_rad` | `Result<f64, VedicError>` | Right ascension of midheaven (RAMC). |
| `ghatika_from_elapsed` | `GhatikaPosition` | Ghatika position from elapsed daylight fraction. |
| `ghatikas_since_sunrise` | `f64` | Ghatikas elapsed between sunrise and moment. |

## Arudha and Special Lagnas

| Function | Output | Purpose |
|---|---|---|
| `arudha_pada` | `(f64, u8)` | Arudha for one bhava. |
| `all_arudha_padas` | `[ArudhaResult; 12]` | Arudhas for all 12 bhavas. |
| `bhava_lagna` | `f64` | Bhava lagna longitude. |
| `hora_lagna` | `f64` | Hora lagna longitude. |
| `ghati_lagna` | `f64` | Ghati lagna longitude. |
| `vighati_lagna` | `f64` | Vighati lagna longitude. |
| `varnada_lagna` | `f64` | Varnada lagna longitude. |
| `sree_lagna` | `f64` | Sree lagna longitude. |
| `pranapada_lagna` | `f64` | Pranapada lagna longitude. |
| `indu_lagna` | `f64` | Indu lagna (wealth indicator). |
| `all_special_lagnas` | `AllSpecialLagnas` | Compute all special lagnas in one call. |

## Sphuta Family

| Function | Output | Purpose |
|---|---|---|
| `prana_sphuta` | `f64` | Prana sphuta. |
| `deha_sphuta` | `f64` | Deha sphuta. |
| `mrityu_sphuta` | `f64` | Mrityu sphuta. |
| `tithi_sphuta` | `f64` | Tithi sphuta. |
| `yoga_sphuta` | `f64` | Yoga sphuta. |
| `yoga_sphuta_normalized` | `f64` | Yoga sphuta normalized within yoga cycle. |
| `rahu_tithi_sphuta` | `f64` | Rahu tithi sphuta. |
| `bhrigu_bindu` | `f64` | Bhrigu bindu. |
| `beeja_sphuta` | `f64` | Beeja sphuta. |
| `kshetra_sphuta` | `f64` | Kshetra sphuta. |
| `trisphuta` | `f64` | Tri-sphuta. |
| `sookshma_trisphuta` | `f64` | Sookshma tri-sphuta. |
| `chatussphuta` | `f64` | Chatus-sphuta. |
| `panchasphuta` | `f64` | Pancha-sphuta. |
| `avayoga_sphuta` | `f64` | Avayoga sphuta. |
| `kunda` | `f64` | Kunda point. |
| `all_sphutas` | `[(Sphuta, f64); 16]` | Compute all defined sphutas. |

## Drishti and Ashtakavarga

| Function | Output | Purpose |
|---|---|---|
| `base_virupa` | `f64` | Base virupa by angular distance. |
| `special_virupa` | `f64` | Graha-specific virupa bonuses. |
| `graha_drishti` | `DrishtiEntry` | Drishti from one graha to one target. |
| `graha_drishti_matrix` | `GrahaDrishtiMatrix` | Full 9x9 drishti matrix. |
| `calculate_bav` | `BhinnaAshtakavarga` | Compute one BAV chart. |
| `calculate_all_bav` | `[BhinnaAshtakavarga; 7]` | Compute BAV charts for all sapta grahas. |
| `calculate_sav` | `SarvaAshtakavarga` | Compute SAV from BAV set. |
| `trikona_sodhana` | `[u8; 12]` | Trikona sodhana transform. |
| `ekadhipatya_sodhana` | `[u8; 12]` | Ekadhipatya sodhana transform. |
| `calculate_ashtakavarga` | `AshtakavargaResult` | Full ashtakavarga pipeline. |

## Upagraha Helpers

| Function | Output | Purpose |
|---|---|---|
| `sun_based_upagrahas` | `SunBasedUpagrahas` | Compute 5 Sun-based upagrahas. |
| `day_portion_index` | `u8` | Planet portion index for daytime. |
| `night_portion_index` | `u8` | Planet portion index for nighttime. |
| `portion_jd_range` | `(f64, f64)` | JD range for one day/night portion. |
| `time_upagraha_planet` | `(u8, bool)` | Planet and day/night mapping for time upagraha. |
| `time_upagraha_jd` | `f64` | JD at which to evaluate lagna for time upagraha. |

## Utility

| Function | Output | Purpose |
|---|---|---|
| `normalize_360` | `f64` | Normalize angle to `[0, 360)` degrees. |
