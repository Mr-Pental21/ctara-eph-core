# Go Reference

This page summarizes the public Go wrapper from the `dhruv` package, using
`bindings/go-open/dhruv/` as the source of truth.

## Primary Public Types

Lifecycle and handles:

- `Engine`
- `LSK`
- `EOP`
- `Config`
- `TaraCatalog`

Core inputs and configs:

- `EngineConfig`
- `ConfigLoadOptions`
- `Query`
- `QueryRequest`
- `QueryResult`
- `UtcTime`
- `GeoLocation`
- `RiseSetConfig`
- `BhavaConfig`
- `SankrantiConfig`
- `ConjunctionConfig`
- `GrahanConfig`
- `StationaryConfig`
- `GrahaPositionsConfig`
- `BindusConfig`
- `DrishtiConfig`
- `TimeUpagrahaConfig`
- `AmshaChartScope`
- `AmshaSelectionConfig`
- `FullKundaliConfig`
- `DashaSelectionConfig`
- `DashaVariationConfig`
- `TaraConfig`

Query request constants:

- `QueryTimeJDTDB`
- `QueryTimeUTC`
- `QueryOutputCartesian`
- `QueryOutputSpherical`
- `QueryOutputBoth`

Upagraha config constants:

- `UpagrahaPointStart`
- `UpagrahaPointMiddle`
- `UpagrahaPointEnd`
- `GulikaMaandiPlanetRahu`
- `GulikaMaandiPlanetSaturn`

## Package-Level Function Inventory

Lifecycle and runtime:

- `APIVersion`
- `VerifyABI`
- `LoadConfig`
- `ConfigLoadOptionsDefault`
- `ClearActiveConfig`
- `NewEngine`
- `LoadLSK`
- `LoadEOP`
- `LoadTaraCatalog`
- `QueryOnce`
- `CartesianToSpherical`

Default-config helpers:

- `RiseSetConfigDefault`
- `BhavaConfigDefault`
- `SankrantiConfigDefault`
- `ConjunctionConfigDefault`
- `GrahanConfigDefault`
- `StationaryConfigDefault`
- `DashaSelectionConfigDefault`
- `DashaVariationConfigDefault`
- `FullKundaliConfigDefault`
- `TimeUpagrahaConfigDefault`

Go config loading uses the same main request shape as the C ABI:

- `LoadConfig(ConfigLoadOptions)`
- `ConfigLoadOptions.Path` is nullable for discovery mode
- `ConfigLoadOptions.DefaultsMode` selects recommended defaults vs none
- `ConfigLoadOptionsDefault()` returns discovery mode with recommended defaults

Time and ayanamsha:

- `UTCToTdbJD`
  `UTCToTdbJD` now takes `(*LSK, *EOP, UtcToTdbRequest)` and returns `UtcToTdbResult`, including typed diagnostics.
- `JdTdbToUTC`
- `NutationIau2000b`
- `NutationIau2000bUTC`
- `ApproximateLocalNoonJD`
- `AyanamshaSystemCount`
- `ReferencePlaneDefault`
- `AyanamshaComputeEx`
- `LunarNodeCount`
- `LunarNodeDeg`
- `LunarNodeDegUTC`
- `LunarNodeComputeEx`

Classifiers, names, and pure helpers:

- `DegToDms`
- `RashiFromLongitude`
- `NakshatraFromLongitude`
- `Nakshatra28FromLongitude`
- `RashiFromTropical`
- `NakshatraFromTropical`
- `Nakshatra28FromTropical`
- `RashiFromTropicalUTC`
- `NakshatraFromTropicalUTC`
- `Nakshatra28FromTropicalUTC`
- `RashiCount`
- `NakshatraCount`
- `RashiName`
- `NakshatraName`
- `Nakshatra28Name`
- `MasaName`
- `AyanaName`
- `SamvatsaraName`
- `TithiName`
- `KaranaName`
- `YogaName`
- `VaarName`
- `HoraName`
- `GrahaName`
- `YoginiName`
- `SphutaName`
- `SpecialLagnaName`
- `ArudhaPadaName`
- `UpagrahaName`
- `TithiFromElongation`
- `KaranaFromElongation`
- `YogaFromSum`
- `VaarFromJD`
- `MasaFromRashiIndex`
- `AyanaFromSiderealLongitude`
- `NthRashiFrom`
- `RashiLord`
- `HoraAt`
- `SamvatsaraFromYear`
- `RiseSetResultToUTC`
- `VaarFromSunrises`
- `HoraFromSunrises`
- `GhatikaFromSunrises`
- `GhatikaFromElapsed`
- `GhatikasSinceSunrise`
- `HoraLord`
- `MasaLord`
- `SamvatsaraLord`
- `ExaltationDegree`
- `DebilitationDegree`
- `MoolatrikoneRange`
- `CombustionThreshold`
- `IsCombust`
- `AllCombustionStatus`
- `NaisargikaMaitri`
- `TatkalikaMaitri`
- `PanchadhaMaitri`
- `DignityInRashi`
- `DignityInRashiWithPositions`
- `NodeDignityInRashi`
- `NaturalBeneficMalefic`
- `MoonBeneficNature`
- `GrahaGender`

Pure sphuta, special-lagna, and upagraha helpers:

- `AllSphutas`
- `BhriguBindu`
- `PranaSphuta`
- `DehaSphuta`
- `MrityuSphuta`
- `TithiSphuta`
- `YogaSphuta`
- `YogaSphutaNormalized`
- `RahuTithiSphuta`
- `KshetraSphuta`
- `BeejaSphuta`
- `Trisphuta`
- `Chatussphuta`
- `Panchasphuta`
- `SookshmaTrisphuta`
- `AvayogaSphuta`
- `Kunda`
- `BhavaLagna`
- `HoraLagna`
- `GhatiLagna`
- `VighatiLagna`
- `VarnadaLagna`
- `SreeLagna`
- `PranapadaLagna`
- `InduLagna`
- `ArudhaPada`
- `TimeUpagrahaJD`
- `TimeUpagrahaJDWithConfig`

Pure ashtakavarga and drishti helpers:

- `CalculateAshtakavarga`
- `CalculateBAV`
- `CalculateAllBAV`
- `CalculateSAV`
- `TrikonaSodhana`
- `EkadhipatyaSodhana`
- `GrahaDrishti`
- `GrahaDrishtiMatrixForLongitudes`

Amsha helpers:

- `AmshaLongitude`
- `AmshaRashiInfo`
- `AmshaLongitudes`

## Engine Method Inventory

Ephemeris and node helpers:

- `(*Engine).Query`
- `(*Engine).LunarNodeDegWithEngine`
- `(*Engine).LunarNodeDegUTCWithEngine`

Go uses one main query request surface. `QueryRequest` carries JD-vs-UTC input
and cartesian-vs-spherical output selection instead of separate `QueryUTC` or
`QueryUTCSpherical` entrypoints.

Go dasha period results expose structured `StartUTC` / `EndUTC` alongside
`StartJD` / `EndJD`. Dasha snapshots expose `QueryUTC` alongside `QueryJD`.

Go high-level search/event results follow the same rule: conjunction, grahan,
stationary, and max-speed results expose structured UTC alongside their
existing JD/TDB fields, while sankranti and lunar-phase results remain UTC-first.

The corresponding Go search request structs carry `AtUTC` / `StartUTC` /
`EndUTC` alongside `AtJdTdb` / `StartJdTdb` / `EndJdTdb`, with one shared
request surface per feature instead of separate UTC-specific methods.

Go range-search methods auto-expand their internal buffers until the full
result set is returned. The optional final argument is only the initial
internal chunk size, not a public truncation cap.

Panchang and vedic basics:

- `(*Engine).ComputeRiseSet`
- `(*Engine).ComputeAllEvents`
- `(*Engine).ComputeRiseSetUTC`
- `(*Engine).ComputeAllEventsUTC`
- `(*Engine).ComputeBhavas`
- `(*Engine).ComputeBhavasUTC`
- `(*Engine).LagnaDeg`
- `(*Engine).LagnaDegWithConfig`
- `(*Engine).MCDeg`
- `(*Engine).MCDegWithConfig`
- `(*Engine).RAMCDeg`
- `(*Engine).LagnaDegUTC`
- `(*Engine).LagnaDegUTCWithConfig`
- `(*Engine).MCDegUTC`
- `(*Engine).MCDegUTCWithConfig`
- `(*Engine).RAMCDegUTC`
- `(*Engine).TithiForDate`
- `(*Engine).KaranaForDate`
- `(*Engine).YogaForDate`
- `(*Engine).NakshatraForDate`
- `(*Engine).VaarForDate`
- `(*Engine).HoraForDate`
- `(*Engine).GhatikaForDate`
- `(*Engine).MasaForDate`
- `(*Engine).AyanaForDate`
- `(*Engine).VarshaForDate`
- `(*Engine).PanchangComputeEx`
- `(*Engine).ElongationAt`
- `(*Engine).SiderealSumAt`
- `(*Engine).VedicDaySunrises`
- `(*Engine).BodyEclipticLonLat`
- `(*Engine).TithiAt`
- `(*Engine).KaranaAt`
- `(*Engine).YogaAt`
- `(*Engine).NakshatraAt`

Jyotish and charts:

- `(*Engine).GrahaLongitudes`
  Uses `GrahaLongitudesConfig` with `GrahaLongitudeKindSidereal` or `GrahaLongitudeKindTropical`, plus optional `PrecessionModel*` and `ReferencePlane*` choices.
- `(*Engine).MovingOsculatingApogeesForDate`
  Returns moving heliocentric osculating apogees for graha indices 2..6
  (`Mangal,Buddh,Guru,Shukra,Shani`) with sidereal longitude, ayanamsha, and
  reference-plane longitude.
- `(*Engine).SpecialLagnasForDate`
- `(*Engine).ArudhaPadasForDate`
- `(*Engine).AllUpagrahasForDate`
- `(*Engine).AllUpagrahasForDateWithConfig`
- `(*Engine).CharakarakaForDate`
- `(*Engine).GrahaPositionsForDate`
- `(*Engine).CoreBindusForDate`
- `(*Engine).DrishtiForDate`
- `(*Engine).AshtakavargaForDate`
- `(*Engine).FullKundaliForDateSummary`
- `(*Engine).FullKundaliForDate`
- `(*Engine).TimeUpagrahaJDUTC`
- `(*Engine).TimeUpagrahaJDUTCWithConfig`

Strength, dasha, amsha, and tara:

- `(*Engine).ShadbalaForDate`
- `(*Engine).BhavaBalaForDate`
- `(*Engine).VimsopakaForDate`
- `(*Engine).BalasForDate`
- `(*Engine).AvasthaForDate`
- `(*Engine).DashaHierarchy`
- `(*Engine).DashaSnapshot`
- `(*Engine).DashaLevel0`
- `(*Engine).DashaLevel0Entity`
- `(*Engine).DashaChildren`
- `(*Engine).DashaChildPeriod`
- `(*Engine).DashaCompleteLevel`
- `(*Engine).AmshaChartForDate`
- `(*TaraCatalog).Compute`
- `(*TaraCatalog).GalacticCenterEcliptic`
- `TaraPropagatePosition`
- `TaraApplyAberration`
- `TaraApplyLightDeflection`
- `TaraGalacticAnticenterICRS`

Go dasha period structs expose `EntityName` with the exact canonical Sanskrit
entity name.
Go dasha requests now accept either UTC/location birth context or precomputed
raw dasha inputs through the shared request structs.

Search:

- `(*Engine).ConjunctionSearch`
- `(*Engine).GrahanSearch`
- `(*Engine).MotionSearch`
- `(*Engine).LunarPhaseSearch`
- `(*Engine).SankrantiSearch`

## Config Notes

`TimeUpagrahaConfig` fields:

- `GulikaPoint`
- `MaandiPoint`
- `OtherPoint`
- `GulikaPlanet`
- `MaandiPlanet`

`BindusConfig` and `FullKundaliConfig` both carry `UpagrahaConfig`.

`FullKundaliConfig` also includes:

- root include flags
- `GrahaPositionsConfig`
- `BindusConfig`
- `DrishtiConfig`
- `AmshaScope`
- `AmshaSelection`
- `DashaConfig`

`ShadbalaForDate`, `VimsopakaForDate`, `BalasForDate`, and `AvasthaForDate`
accept `AmshaSelection`. Embedded `FullKundaliResult.Amshas` returns the
resolved amsha union used by the call.

`DashaSelectionConfig` supports per-system hierarchy depth through `MaxLevels`
and optional full-kundali snapshots through `SnapshotTime`, typically with
`TimeKind = DashaTimeUTC` plus `UTC`.

Defaults preserved by `TimeUpagrahaConfigDefault()`:

- Gulika = Rahu period start
- Maandi = Rahu period end
- other time-based upagrahas = period start

For build/runtime notes, see [`bindings/go-open/README.md`](../../../bindings/go-open/README.md).

## Rashi-Bhava Bhava Config

`BhavaConfig` includes `UseRashiBhavaForBalaAvastha` and `IncludeRashiBhavaResults`, both defaulting to `true`. It also includes `IncludeNodeAspectsForDrikBala`, defaulting to `false`, which controls whether Rahu/Ketu incoming aspects contribute to Shadbala Drik Bala. Existing bhava fields remain configured-system outputs; rashi-bhava sibling fields such as `RashiBhavaCusps`, `RashiBhavaNumber`, and `GrahaToRashiBhava` expose the equal-house/whole-sign companion basis.
