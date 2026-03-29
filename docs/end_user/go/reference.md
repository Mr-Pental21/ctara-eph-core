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

Time and ayanamsha:

- `UTCToTdbJD`
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
- `(*Engine).QueryUTC`
- `(*Engine).QueryUTCSpherical`
- `(*Engine).LunarNodeDegWithEngine`
- `(*Engine).LunarNodeDegUTCWithEngine`

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

- `(*Engine).GrahaSiderealLongitudes`
- `(*Engine).GrahaTropicalLongitudes`
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
- `(*Engine).DashaHierarchyUTC`
- `(*Engine).DashaSnapshotUTC`
- `(*Engine).DashaLevel0UTC`
- `(*Engine).DashaLevel0EntityUTC`
- `(*Engine).DashaChildrenUTC`
- `(*Engine).DashaChildPeriodUTC`
- `(*Engine).DashaCompleteLevelUTC`
- `(*Engine).AmshaChartForDate`
- `(*TaraCatalog).Compute`
- `(*TaraCatalog).GalacticCenterEcliptic`

Go dasha period structs expose `EntityName` with the exact canonical Sanskrit
entity name.

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

Defaults preserved by `TimeUpagrahaConfigDefault()`:

- Gulika = Rahu period start
- Maandi = Rahu period end
- other time-based upagrahas = period start

For build/runtime notes, see [`bindings/go-open/README.md`](../../../bindings/go-open/README.md).
