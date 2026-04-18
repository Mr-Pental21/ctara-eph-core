package dhruv

import (
	"math"
	"os"
	"path/filepath"
	"runtime"
	"testing"
)

func repoRootFromTest(t *testing.T) string {
	t.Helper()
	_, file, _, ok := runtime.Caller(0)
	if !ok {
		t.Fatalf("runtime.Caller failed")
	}
	return filepath.Clean(filepath.Join(filepath.Dir(file), "../../.."))
}

func kernelPaths(t *testing.T) (spk, lsk, eop string, ok bool) {
	t.Helper()
	root := repoRootFromTest(t)
	spk = filepath.Join(root, "kernels", "data", "de442s.bsp")
	lsk = filepath.Join(root, "kernels", "data", "naif0012.tls")
	eop = filepath.Join(root, "kernels", "data", "finals2000A.all")
	if !fileExists(spk) || !fileExists(lsk) || !fileExists(eop) {
		return "", "", "", false
	}
	return spk, lsk, eop, true
}

func fileExists(p string) bool {
	_, err := os.Stat(p)
	return err == nil
}

func TestABIVersion(t *testing.T) {
	if APIVersion() != ExpectedAPIVersion {
		t.Fatalf("ABI mismatch: got=%d want=%d", APIVersion(), ExpectedAPIVersion)
	}
}

func TestKshetraSphutaMatchesAllSphutas(t *testing.T) {
	inputs := SphutalInputs{
		Sun: 10, Moon: 20, Mars: 30, Jupiter: 40, Venus: 50,
		Rahu: 60, Lagna: 70, EighthLord: 80, Gulika: 90,
	}
	all, err := AllSphutas(inputs)
	if err != nil {
		t.Fatalf("AllSphutas: %v", err)
	}
	scalar := KshetraSphuta(inputs.Moon, inputs.Mars, inputs.Jupiter, inputs.Venus, inputs.Lagna)

	// ALL_SPHUTAS order in dhruv_vedic_base: KshetraSphuta is index 8.
	kshetraIdx := 8
	if math.Abs(scalar-all.Longitudes[kshetraIdx]) > 1e-9 {
		t.Fatalf("kshetra mismatch: scalar=%v all[%d]=%v", scalar, kshetraIdx, all.Longitudes[kshetraIdx])
	}
}

func TestHelperAndTaraPrimitives(t *testing.T) {
	if HoraLord(0, 0) != 0 {
		t.Fatalf("HoraLord(sunday,0) = %d, want 0", HoraLord(0, 0))
	}

	has, exalt, err := ExaltationDegree(0)
	if err != nil {
		t.Fatalf("ExaltationDegree: %v", err)
	}
	if !has || math.Abs(exalt-10.0) > 1e-9 {
		t.Fatalf("unexpected exaltation degree: has=%v value=%v", has, exalt)
	}

	relationship, err := NaisargikaMaitri(0, 1)
	if err != nil {
		t.Fatalf("NaisargikaMaitri: %v", err)
	}
	if relationship != NaisargikaFriend {
		t.Fatalf("unexpected naisargika relationship: got=%d", relationship)
	}

	position, err := TaraPropagatePosition(10.0, 20.0, 10.0, 0.0, 0.0, 0.0, 0.0)
	if err != nil {
		t.Fatalf("TaraPropagatePosition: %v", err)
	}
	if math.Abs(position.RADeg-10.0) > 1e-9 || math.Abs(position.DecDeg-20.0) > 1e-9 {
		t.Fatalf("unexpected propagated position: %+v", position)
	}

	dir, err := TaraGalacticAnticenterICRS()
	if err != nil {
		t.Fatalf("TaraGalacticAnticenterICRS: %v", err)
	}
	norm := math.Sqrt(dir[0]*dir[0] + dir[1]*dir[1] + dir[2]*dir[2])
	if math.Abs(norm-1.0) > 1e-9 {
		t.Fatalf("anticenter vector not normalized: %v", dir)
	}
}

func TestLoadConfigSupportsDiscoveryOptions(t *testing.T) {
	dir := t.TempDir()
	configPath := filepath.Join(dir, "config.toml")
	if err := os.WriteFile(configPath, []byte("version = 1\n"), 0o644); err != nil {
		t.Fatalf("WriteFile: %v", err)
	}
	t.Setenv("DHRUV_CONFIG_FILE", configPath)
	cfg, err := LoadConfig(ConfigLoadOptionsDefault())
	if err != nil {
		t.Fatalf("LoadConfig: %v", err)
	}
	defer cfg.Close()
	if err := ClearActiveConfig(); err != nil {
		t.Fatalf("ClearActiveConfig: %v", err)
	}
}

func TestEngineQueryAndTimeRoundTrip(t *testing.T) {
	spk, lskPath, _, ok := kernelPaths(t)
	if !ok {
		t.Skip("kernel files missing; skipping integration test")
	}

	eng, err := NewEngine(EngineConfig{
		SpkPaths:         []string{spk},
		LskPath:          lskPath,
		CacheCapacity:    64,
		StrictValidation: false,
	})
	if err != nil {
		t.Fatalf("NewEngine: %v", err)
	}
	defer eng.Close()

	q := QueryRequest{
		Target:     301,
		Observer:   399,
		Frame:      1,
		TimeKind:   QueryTimeJDTDB,
		EpochTdbJD: 2451545.0,
		OutputMode: QueryOutputCartesian,
	}
	result, err := eng.Query(q)
	if err != nil {
		t.Fatalf("Query: %v", err)
	}
	if result.State == nil {
		t.Fatalf("expected cartesian state in query result")
	}
	sv := result.State
	if math.IsNaN(sv.PositionKm[0]) || math.IsInf(sv.PositionKm[0], 0) {
		t.Fatalf("invalid state vector output: %+v", sv)
	}

	lsk, err := LoadLSK(lskPath)
	if err != nil {
		t.Fatalf("LoadLSK: %v", err)
	}
	defer lsk.Close()

	utc := UtcTime{Year: 2025, Month: 1, Day: 1, Hour: 0, Minute: 0, Second: 0}
	resultTime, err := UTCToTdbJD(lsk, nil, UtcToTdbRequest{
		UTC: utc,
		Policy: TimePolicy{
			Mode: TimePolicyHybridDeltaT,
			Options: TimeConversionOptions{
				WarnOnFallback:         true,
				DeltaTModel:            DeltaTModelSmh2016WithPre720Quadratic,
				FreezeFutureDut1:       true,
				PreRangeDut1:           0.0,
				FutureDeltaTTransition: FutureDeltaTTransitionLegacyTtUtcBlend,
				FutureTransitionYears:  100.0,
				SmhFutureFamily:        SmhFutureFamilyAddendum2020Piecewise,
			},
		},
	})
	if err != nil {
		t.Fatalf("UTCToTdbJD: %v", err)
	}
	back, err := JdTdbToUTC(lsk, resultTime.JdTdb)
	if err != nil {
		t.Fatalf("JdTdbToUTC: %v", err)
	}
	if back.Year != utc.Year || back.Month != utc.Month || back.Day != utc.Day {
		t.Fatalf("roundtrip mismatch: got=%+v want=%+v", back, utc)
	}
}

func TestSearchAndPanchangSmoke(t *testing.T) {
	spk, lskPath, eopPath, ok := kernelPaths(t)
	if !ok {
		t.Skip("kernel files missing; skipping integration test")
	}

	eng, err := NewEngine(EngineConfig{
		SpkPaths:         []string{spk},
		LskPath:          lskPath,
		CacheCapacity:    64,
		StrictValidation: false,
	})
	if err != nil {
		t.Fatalf("NewEngine: %v", err)
	}
	defer eng.Close()

	eop, err := LoadEOP(eopPath)
	if err != nil {
		t.Fatalf("LoadEOP: %v", err)
	}
	defer eop.Close()

	req := LunarPhaseSearchRequest{
		PhaseKind: 1,
		QueryMode: 2,
		StartUTC:  UtcTime{Year: 2000, Month: 1, Day: 1, Hour: 12, Minute: 0, Second: 0},
		EndUTC:    UtcTime{Year: 2000, Month: 12, Day: 31, Hour: 12, Minute: 0, Second: 0},
	}
	_, found, events, err := eng.LunarPhaseSearch(req, 1)
	if err != nil {
		t.Fatalf("LunarPhaseSearch: %v", err)
	}
	if found {
		t.Fatalf("range LunarPhaseSearch should not report single-event found=true")
	}
	if len(events) < 12 {
		t.Fatalf("expected auto-expanded lunar phase range results, got %d", len(events))
	}

	utc := UtcTime{Year: 2025, Month: 1, Day: 15, Hour: 12, Minute: 0, Second: 0}
	if _, err := eng.TithiForDate(utc); err != nil {
		t.Fatalf("TithiForDate: %v", err)
	}

	loc := GeoLocation{LatitudeDeg: 12.9716, LongitudeDeg: 77.5946, AltitudeM: 920}
	if _, err := eng.VaarForDate(eop, utc, loc, RiseSetConfigDefault()); err != nil {
		t.Fatalf("VaarForDate: %v", err)
	}

	bhava := BhavaConfigDefault()
	riseset := RiseSetConfigDefault()
    if _, err := eng.ShadbalaForDate(
        eop,
        utc,
        loc,
        bhava,
        riseset,
        0,
        true,
        AmshaSelectionConfig{},
    ); err != nil {
        t.Fatalf("ShadbalaForDate: %v", err)
    }
	if _, err := eng.CharakarakaForDate(eop, utc, 0, true, CharakarakaSchemeMixedParashara); err != nil {
		t.Fatalf("CharakarakaForDate: %v", err)
	}

	if _, err := eng.FullKundaliForDateSummary(eop, utc, loc, bhava, riseset, 0, true); err != nil {
		t.Fatalf("FullKundaliForDateSummary: %v", err)
	}

	cfg := FullKundaliConfigDefault()
	cfg.IncludeDasha = true
	cfg.DashaConfig.Count = 2
	cfg.DashaConfig.Systems[0] = 0
	cfg.DashaConfig.Systems[1] = 1
	cfg.DashaConfig.MaxLevels[0] = 0
	cfg.DashaConfig.MaxLevels[1] = 1
	kundali, err := eng.FullKundaliForDate(eop, utc, loc, bhava, riseset, 0, true, cfg)
	if err != nil {
		t.Fatalf("FullKundaliForDate: %v", err)
	}
	if kundali.Sphutas == nil || len(kundali.Sphutas.Longitudes) != SphutaCount {
		t.Fatalf("expected root sphutas in full kundali")
	}
	if len(kundali.Dasha) != 2 {
		t.Fatalf("expected 2 dasha hierarchies, got %d", len(kundali.Dasha))
	}
	if len(kundali.Dasha[0].Levels) != 1 || len(kundali.Dasha[1].Levels) != 2 {
		t.Fatalf("unexpected per-system dasha depths: %d %d", len(kundali.Dasha[0].Levels), len(kundali.Dasha[1].Levels))
	}

	amshaScope := AmshaChartScope{
		IncludeBhavaCusps:    true,
		IncludeArudhaPadas:   true,
		IncludeUpagrahas:     true,
		IncludeSphutas:       true,
		IncludeSpecialLagnas: true,
	}
	amshaChart, err := eng.AmshaChartForDate(eop, utc, loc, bhava, riseset, 0, true, 9, 0, amshaScope)
	if err != nil {
		t.Fatalf("AmshaChartForDate: %v", err)
	}
	if len(amshaChart.BhavaCusps) != 12 || len(amshaChart.ArudhaPadas) != 12 {
		t.Fatalf("expected bhava/arudha amsha sections, got %d/%d", len(amshaChart.BhavaCusps), len(amshaChart.ArudhaPadas))
	}
	if len(amshaChart.Upagrahas) != 11 || len(amshaChart.Sphutas) != SphutaCount || len(amshaChart.SpecialLagnas) != 8 {
		t.Fatalf(
			"expected upagraha/sphuta/special-lagna amsha sections, got %d/%d/%d",
			len(amshaChart.Upagrahas), len(amshaChart.Sphutas), len(amshaChart.SpecialLagnas),
		)
	}

	amshaCfg := FullKundaliConfigDefault()
	amshaCfg.IncludeBhavaCusps = true
	amshaCfg.IncludeBindus = true
	amshaCfg.IncludeUpagrahas = true
	amshaCfg.IncludeSphutas = true
	amshaCfg.IncludeSpecialLagnas = true
	amshaCfg.IncludeAmshas = true
	amshaCfg.AmshaScope = amshaScope
	amshaCfg.AmshaSelection.Count = 1
	amshaCfg.AmshaSelection.Codes[0] = 9
	amshaCfg.AmshaSelection.Variations[0] = 0
	kundaliWithAmshas, err := eng.FullKundaliForDate(eop, utc, loc, bhava, riseset, 0, true, amshaCfg)
	if err != nil {
		t.Fatalf("FullKundaliForDate (amshas): %v", err)
	}
	if len(kundaliWithAmshas.Amshas) != 1 {
		t.Fatalf("expected 1 amsha chart, got %d", len(kundaliWithAmshas.Amshas))
	}
	if len(kundaliWithAmshas.Amshas[0].Sphutas) != SphutaCount {
		t.Fatalf("expected scoped amsha sphutas in full kundali, got %d", len(kundaliWithAmshas.Amshas[0].Sphutas))
	}
}

func TestAshtakavargaContributors(t *testing.T) {
	bav, err := CalculateBAV(0, [7]uint8{0, 1, 2, 3, 4, 5, 6}, 0)
	if err != nil {
		t.Fatalf("CalculateBAV: %v", err)
	}
	for i := 0; i < 12; i++ {
		row := 0
		for j := 0; j < 8; j++ {
			if bav.Contributors[i][j] > 1 {
				t.Fatalf("invalid contributor value %d at rashi=%d contributor=%d", bav.Contributors[i][j], i, j)
			}
			row += int(bav.Contributors[i][j])
		}
		if row != int(bav.Points[i]) {
			t.Fatalf("contributor row sum mismatch at rashi=%d: got=%d want=%d", i, row, bav.Points[i])
		}
	}
}

func TestLowTierDashaWrappers(t *testing.T) {
	spk, lskPath, eopPath, ok := kernelPaths(t)
	if !ok {
		t.Skip("kernel files missing; skipping integration test")
	}

	eng, err := NewEngine(EngineConfig{
		SpkPaths:         []string{spk},
		LskPath:          lskPath,
		CacheCapacity:    64,
		StrictValidation: false,
	})
	if err != nil {
		t.Fatalf("NewEngine: %v", err)
	}
	defer eng.Close()

	eop, err := LoadEOP(eopPath)
	if err != nil {
		t.Fatalf("LoadEOP: %v", err)
	}
	defer eop.Close()

	birthUTC := UtcTime{Year: 1990, Month: 1, Day: 1, Hour: 12, Minute: 0, Second: 0}
	loc := GeoLocation{LatitudeDeg: 12.9716, LongitudeDeg: 77.5946, AltitudeM: 920}
	bhava := BhavaConfigDefault()
	riseset := RiseSetConfigDefault()
	birth := DashaBirthContext{
		TimeKind:        DashaTimeUTC,
		BirthUTC:        birthUTC,
		HasLocation:     true,
		Location:        loc,
		BhavaConfig:     bhava,
		RiseSetConfig:   riseset,
		SankrantiConfig: SankrantiConfigDefault(),
	}

	level0, err := eng.DashaLevel0(eop, DashaLevel0Request{Birth: birth, System: 0})
	if err != nil {
		t.Fatalf("DashaLevel0: %v", err)
	}
	if len(level0) == 0 {
		t.Fatalf("expected level0 periods")
	}

	first := level0[0]
	same, found, err := eng.DashaLevel0Entity(eop, DashaLevel0EntityRequest{
		Birth:       birth,
		System:      0,
		EntityType:  first.EntityType,
		EntityIndex: first.EntityIndex,
	})
	if err != nil {
		t.Fatalf("DashaLevel0Entity: %v", err)
	}
	if !found || same.EntityIndex != first.EntityIndex {
		t.Fatalf("unexpected level0 entity lookup: found=%v same=%+v first=%+v", found, same, first)
	}

	variation := DashaVariationConfigDefault()
	children, err := eng.DashaChildren(eop, DashaChildrenRequest{
		Birth:     birth,
		System:    0,
		Variation: variation,
		Parent:    first,
	})
	if err != nil {
		t.Fatalf("DashaChildren: %v", err)
	}
	if len(children) == 0 {
		t.Fatalf("expected child periods")
	}

	child, found, err := eng.DashaChildPeriod(eop, DashaChildPeriodRequest{
		Birth:            birth,
		System:           0,
		Variation:        variation,
		Parent:           first,
		ChildEntityType:  children[0].EntityType,
		ChildEntityIndex: children[0].EntityIndex,
	})
	if err != nil {
		t.Fatalf("DashaChildPeriod: %v", err)
	}
	if !found || child.EntityIndex != children[0].EntityIndex {
		t.Fatalf("unexpected child lookup: found=%v child=%+v firstChild=%+v", found, child, children[0])
	}

	complete, err := eng.DashaCompleteLevel(eop, DashaCompleteLevelRequest{
		Birth:      birth,
		System:     0,
		Variation:  variation,
		ChildLevel: 1,
	}, level0)
	if err != nil {
		t.Fatalf("DashaCompleteLevel: %v", err)
	}
	if len(complete) < len(children) {
		t.Fatalf("expected complete child level, got=%d children=%d", len(complete), len(children))
	}
}
