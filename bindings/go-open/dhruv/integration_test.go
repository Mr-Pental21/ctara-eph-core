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

	q := Query{Target: 301, Observer: 399, Frame: 1, EpochTdbJD: 2451545.0}
	sv, err := eng.Query(q)
	if err != nil {
		t.Fatalf("Query: %v", err)
	}
	if math.IsNaN(sv.PositionKm[0]) || math.IsInf(sv.PositionKm[0], 0) {
		t.Fatalf("invalid state vector output: %+v", sv)
	}

	lsk, err := LoadLSK(lskPath)
	if err != nil {
		t.Fatalf("LoadLSK: %v", err)
	}
	defer lsk.Close()

	utc := UtcTime{Year: 2025, Month: 1, Day: 1, Hour: 0, Minute: 0, Second: 0}
	jd, err := UTCToTdbJD(lsk, utc)
	if err != nil {
		t.Fatalf("UTCToTdbJD: %v", err)
	}
	back, err := JdTdbToUTC(lsk, jd)
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
		QueryMode: 0,
		AtJdTdb:   2460000.5,
	}
	_, found, _, err := eng.LunarPhaseSearch(req, 8)
	if err != nil {
		t.Fatalf("LunarPhaseSearch: %v", err)
	}
	if !found {
		t.Fatalf("LunarPhaseSearch returned no event")
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
	if _, err := eng.ShadbalaForDate(eop, utc, loc, bhava, riseset, 0, true); err != nil {
		t.Fatalf("ShadbalaForDate: %v", err)
	}
	if _, err := eng.CharakarakaForDate(eop, utc, 0, true, CharakarakaSchemeMixedParashara); err != nil {
		t.Fatalf("CharakarakaForDate: %v", err)
	}

	if _, err := eng.FullKundaliForDateSummary(eop, utc, loc, bhava, riseset, 0, true); err != nil {
		t.Fatalf("FullKundaliForDateSummary: %v", err)
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
