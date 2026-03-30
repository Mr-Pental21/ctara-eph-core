# Node.js Reference

This page summarizes the public Node wrapper exported from
`bindings/node-open/src/index.js`.

## Exported Module Families

- `engine`
- `time`
- `search`
- `panchang`
- `jyotish`
- `extras`
- `shadbala`
- `dasha`
- `tara`

## Exact Export Inventory

`engine.js` exports:

- `Config`
- `Engine`
- `EOP`
- `LSK`
- `apiVersion`
- `verifyAbi`
- `clearActiveConfig`
- `queryOnce`
- `cartesianToSpherical`
- `QUERY_OUTPUT`
- `QUERY_TIME`

`time.js` exports:

- `utcToTdbJd`
  `utcToTdbJd` now accepts a request object with `utc` plus optional `timePolicy`, and returns `{ jdTdb, diagnostics }`.
- `jdTdbToUtc`
- `nutationIau2000b`
- `nutationIau2000bUtc`
- `approximateLocalNoonJd`
- `ayanamshaSystemCount`
- `referencePlaneDefault`
- `ayanamshaComputeEx`
- `lunarNodeCount`
- `lunarNodeDeg`
- `lunarNodeDegWithEngine`
- `lunarNodeDegUtc`
- `lunarNodeDegUtcWithEngine`
- `lunarNodeComputeEx`
- `riseSetConfigDefault`
- `bhavaConfigDefault`
- `sankrantiConfigDefault`

`search.js` exports:

- `conjunctionConfigDefault`
- `grahanConfigDefault`
- `stationaryConfigDefault`
- `conjunctionSearch`
- `grahanSearch`
- `motionSearch`
- `lunarPhaseSearch`
- `sankrantiSearch`

`panchang.js` exports:

- `bhavaSystemCount`
- `computeRiseSet`
- `computeAllEvents`
- `computeRiseSetUtc`
- `computeAllEventsUtc`
- `computeBhavas`
- `computeBhavasUtc`
- `lagnaDeg`
- `mcDeg`
- `ramcDeg`
- `lagnaDegUtc`
- `mcDegUtc`
- `ramcDegUtc`
- `riseSetResultToUtc`
- `tithiForDate`
- `karanaForDate`
- `yogaForDate`
- `nakshatraForDate`
- `vaarForDate`
- `horaForDate`
- `ghatikaForDate`
- `masaForDate`
- `ayanaForDate`
- `varshaForDate`
- `panchangComputeEx`

`jyotish.js` exports:

- `grahaLongitudes`
  Accepts an optional config object with `kind`, `ayanamshaSystem`, `useNutation`, `precessionModel`, and `referencePlane`.
- `specialLagnasForDate`
- `arudhaPadasForDate`
- `allUpagrahasForDate`
- `charakarakaForDate`
- `CHARAKARAKA_SCHEME`
- `CHARAKARAKA_ROLE`
- `rashiCount`
- `nakshatraCount`
- `rashiFromLongitude`
- `nakshatraFromLongitude`
- `nakshatra28FromLongitude`
- `rashiFromTropical`
- `nakshatraFromTropical`
- `nakshatra28FromTropical`
- `rashiFromTropicalUtc`
- `nakshatraFromTropicalUtc`
- `nakshatra28FromTropicalUtc`
- `degToDms`
- `tithiFromElongation`
- `karanaFromElongation`
- `yogaFromSum`
- `samvatsaraFromYear`
- `rashiName`
- `nakshatraName`
- `nakshatra28Name`
- `masaName`
- `ayanaName`
- `samvatsaraName`
- `tithiName`
- `karanaName`
- `yogaName`
- `vaarName`
- `horaName`
- `grahaName`
- `yoginiName`
- `sphutaName`
- `specialLagnaName`
- `arudhaPadaName`
- `upagrahaName`
- `vaarFromJd`
- `masaFromRashiIndex`
- `ayanaFromSiderealLongitude`
- `nthRashiFrom`
- `rashiLord`
- `horaAt`

`extras.js` exports:

- panchang intermediates:
  - `elongationAt`
  - `siderealSumAt`
  - `vedicDaySunrises`
  - `bodyEclipticLonLat`
  - `tithiAt`
  - `karanaAt`
  - `yogaAt`
  - `vaarFromSunrises`
  - `horaFromSunrises`
  - `ghatikaFromSunrises`
  - `nakshatraAt`
  - `ghatikaFromElapsed`
  - `ghatikasSinceSunrise`
- sphuta and special-lagna helpers:
  - `allSphutas`
  - `bhriguBindu`
  - `pranaSphuta`
  - `dehaSphuta`
  - `mrityuSphuta`
  - `tithiSphuta`
  - `yogaSphuta`
  - `yogaSphutaNormalized`
  - `rahuTithiSphuta`
  - `kshetraSphuta`
  - `beejaSphuta`
  - `trisphuta`
  - `chatussphuta`
  - `panchasphuta`
  - `sookshmaTrisphuta`
  - `avayogaSphuta`
  - `kunda`
  - `bhavaLagna`
  - `horaLagna`
  - `ghatiLagna`
  - `vighatiLagna`
  - `varnadaLagna`
  - `sreeLagna`
  - `pranapadaLagna`
  - `induLagna`
  - `arudhaPada`
  - `sunBasedUpagrahas`
- time-based upagraha helpers:
  - `timeUpagrahaJd`
  - `timeUpagrahaJdUtc`
- ashtakavarga, drishti, and charts:
  - `calculateAshtakavarga`
  - `calculateBav`
  - `calculateAllBav`
  - `calculateSav`
  - `trikonaSodhana`
  - `ekadhipatyaSodhana`
  - `ashtakavargaForDate`
  - `grahaDrishti`
  - `grahaDrishtiMatrixForLongitudes`
  - `drishtiForDate`
  - `grahaPositionsForDate`
  - `coreBindusForDate`
  - `amshaLongitude`
  - `amshaRashiInfo`
  - `amshaLongitudes`
  - `amshaChartForDate`

`shadbala.js` exports:

- `calculateBhavaBala`
- `shadbalaForDate`
- `bhavaBalaForDate`
- `vimsopakaForDate`
- `balasForDate`
- `avasthaForDate`
- `fullKundaliConfigDefault`
- `fullKundaliForDate`
- `fullKundaliSummaryForDate`

`dasha.js` exports:

- `DashaHierarchy`
- `dashaSelectionConfigDefault`
- `dashaVariationConfigDefault`
- `dashaHierarchy`
- `dashaSnapshot`
- `dashaLevel0`
- `dashaLevel0Entity`
- `dashaChildren`
- `dashaChildPeriod`
- `dashaCompleteLevel`

Node dasha calls use one request-driven surface per feature. The same functions
accept either:

- `birthUtc` plus `location` for engine-derived inputs
- `birthJd` plus `inputs` for precomputed raw dasha inputs

`dashaSnapshot` similarly accepts either `queryUtc` or `queryJd`.

Returned dasha period objects include `entityName`, the exact canonical
Sanskrit entity name.

`tara.js` exports:

- `TaraCatalog`

## Public Config Objects

Common config objects:

- rise-set config
- bhava config
- sankranti config
- search configs
- drishti config
- graha positions config
- bindus config
- full-kundali config
- dasha selection and variation configs

Time-based upagraha config object:

- `gulikaPoint`
- `maandiPoint`
- `otherPoint`
- `gulikaPlanet`
- `maandiPlanet`

Value mapping:

- points: `0=start`, `1=middle`, `2=end`
- planets: `0=rahu`, `1=saturn`

Other public enum objects:

- `CHARAKARAKA_SCHEME`
- `CHARAKARAKA_ROLE`

`bindusConfig` and `fullKundaliConfig` both accept nested upagraha config.

For build/runtime notes, see [`bindings/node-open/README.md`](../../../bindings/node-open/README.md).
