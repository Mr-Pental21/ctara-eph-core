package cabi

/*
#cgo CFLAGS: -I${SRCDIR}/../../../../crates/dhruv_ffi_c/include
#cgo linux LDFLAGS: -L${SRCDIR}/../../../../target/release -ldhruv_ffi_c -Wl,-rpath,${SRCDIR}/../../../../target/release
#cgo darwin LDFLAGS: -L${SRCDIR}/../../../../target/release -ldhruv_ffi_c -Wl,-rpath,${SRCDIR}/../../../../target/release
#cgo windows LDFLAGS: -L${SRCDIR}/../../../../target/x86_64-pc-windows-gnu/release -ldhruv_ffi_c
#include "dhruv.h"
#include <stdlib.h>
*/
import "C"

import (
	"fmt"
	"math"
	"unsafe"
)

type EngineHandle struct{ ptr *C.DhruvEngineHandle }
type LskHandle struct{ ptr *C.DhruvLskHandle }
type EopHandle struct{ ptr *C.DhruvEopHandle }
type ConfigHandle struct{ ptr *C.DhruvConfigHandle }
type TaraCatalogHandle struct{ ptr *C.DhruvTaraCatalogHandle }
type DashaHierarchyHandle struct{ ptr C.DhruvDashaHierarchyHandle }
type DashaPeriodListHandle struct{ ptr C.DhruvDashaPeriodListHandle }

func (h EngineHandle) Valid() bool      { return h.ptr != nil }
func (h LskHandle) Valid() bool         { return h.ptr != nil }
func (h EopHandle) Valid() bool         { return h.ptr != nil }
func (h ConfigHandle) Valid() bool      { return h.ptr != nil }
func (h TaraCatalogHandle) Valid() bool { return h.ptr != nil }
func (h DashaHierarchyHandle) Valid() bool {
	return h.ptr != nil
}
func (h DashaPeriodListHandle) Valid() bool { return h.ptr != nil }

func APIVersion() uint32 { return uint32(C.dhruv_api_version()) }

func encodeFixedCString(dst []byte, s string) error {
	if len(s)+1 > len(dst) {
		return fmt.Errorf("string exceeds fixed buffer: %d > %d", len(s)+1, len(dst))
	}
	copy(dst, s)
	dst[len(s)] = 0
	return nil
}

func boolU8(v bool) C.uint8_t {
	if v {
		return C.uint8_t(1)
	}
	return C.uint8_t(0)
}

func cUTC(utc UtcTime) C.DhruvUtcTime {
	return C.DhruvUtcTime{
		year:   C.int32_t(utc.Year),
		month:  C.uint32_t(utc.Month),
		day:    C.uint32_t(utc.Day),
		hour:   C.uint32_t(utc.Hour),
		minute: C.uint32_t(utc.Minute),
		second: C.double(utc.Second),
	}
}

func goUTC(utc C.DhruvUtcTime) UtcTime {
	return UtcTime{
		Year:   int32(utc.year),
		Month:  uint32(utc.month),
		Day:    uint32(utc.day),
		Hour:   uint32(utc.hour),
		Minute: uint32(utc.minute),
		Second: float64(utc.second),
	}
}

func goOptionalUTC(valid bool, utc C.DhruvUtcTime) *UtcTime {
	if !valid {
		return nil
	}
	value := goUTC(utc)
	return &value
}

func isZeroUTC(utc UtcTime) bool {
	return utc.Year == 0 &&
		utc.Month == 0 &&
		utc.Day == 0 &&
		utc.Hour == 0 &&
		utc.Minute == 0 &&
		utc.Second == 0
}

func resolveSearchTimeKind(queryMode, timeKind int32, atUTC, startUTC, endUTC UtcTime) int32 {
	if timeKind == SearchTimeUTC {
		return SearchTimeUTC
	}
	if queryMode == 2 {
		if !isZeroUTC(startUTC) || !isZeroUTC(endUTC) {
			return SearchTimeUTC
		}
		return SearchTimeJDTDB
	}
	if !isZeroUTC(atUTC) {
		return SearchTimeUTC
	}
	return SearchTimeJDTDB
}

func jdUTCToUTC(jd float64) UtcTime {
	z := math.Floor(jd + 0.5)
	f := jd + 0.5 - z
	a := z
	if z >= 2299161.0 {
		alpha := math.Floor((z - 1867216.25) / 36524.25)
		a = z + 1.0 + alpha - math.Floor(alpha/4.0)
	}
	b := a + 1524.0
	c := math.Floor((b - 122.1) / 365.25)
	d := math.Floor(365.25 * c)
	e := math.Floor((b - d) / 30.6001)
	dayFloat := b - d - math.Floor(30.6001*e) + f

	month := int32(e - 1.0)
	if e >= 14.0 {
		month = int32(e - 13.0)
	}
	year := int32(c - 4716.0)
	if month <= 2 {
		year = int32(c - 4715.0)
	}

	day := math.Floor(dayFloat)
	frac := dayFloat - day
	totalSeconds := frac * 86400.0
	hour := math.Floor(totalSeconds / 3600.0)
	minute := math.Floor(math.Mod(totalSeconds, 3600.0) / 60.0)
	second := totalSeconds - hour*3600.0 - minute*60.0

	return UtcTime{
		Year:   year,
		Month:  uint32(month),
		Day:    uint32(day),
		Hour:   uint32(hour),
		Minute: uint32(minute),
		Second: second,
	}
}

func cTimeConversionOptions(opts TimeConversionOptions) C.DhruvTimeConversionOptions {
	return C.DhruvTimeConversionOptions{
		warn_on_fallback:          boolU8(opts.WarnOnFallback),
		delta_t_model:             C.int32_t(opts.DeltaTModel),
		freeze_future_dut1:        boolU8(opts.FreezeFutureDut1),
		pre_range_dut1:            C.double(opts.PreRangeDut1),
		future_delta_t_transition: C.int32_t(opts.FutureDeltaTTransition),
		future_transition_years:   C.double(opts.FutureTransitionYears),
		smh_future_family:         C.int32_t(opts.SmhFutureFamily),
	}
}

func cTimePolicy(policy TimePolicy) C.DhruvTimePolicy {
	return C.DhruvTimePolicy{
		mode:    C.int32_t(policy.Mode),
		options: cTimeConversionOptions(policy.Options),
	}
}

func cUtcToTdbRequest(req UtcToTdbRequest) C.DhruvUtcToTdbRequest {
	return C.DhruvUtcToTdbRequest{
		utc:    cUTC(req.UTC),
		policy: cTimePolicy(req.Policy),
	}
}

func goTimeWarning(w C.DhruvTimeWarning) TimeWarning {
	return TimeWarning{
		Kind:                 int32(w.kind),
		UtcSeconds:           float64(w.utc_seconds),
		FirstEntryUtcSeconds: float64(w.first_entry_utc_seconds),
		LastEntryUtcSeconds:  float64(w.last_entry_utc_seconds),
		UsedDeltaAtSeconds:   float64(w.used_delta_at_seconds),
		Mjd:                  float64(w.mjd),
		FirstEntryMjd:        float64(w.first_entry_mjd),
		LastEntryMjd:         float64(w.last_entry_mjd),
		UsedDut1Seconds:      float64(w.used_dut1_seconds),
		DeltaTModel:          int32(w.delta_t_model),
		DeltaTSegment:        int32(w.delta_t_segment),
	}
}

func goTimeDiagnostics(diag C.DhruvTimeDiagnostics) TimeDiagnostics {
	warnings := make([]TimeWarning, 0, int(diag.warning_count))
	for i := 0; i < int(diag.warning_count) && i < MaxTimeWarnings; i++ {
		warnings = append(warnings, goTimeWarning(diag.warnings[i]))
	}
	return TimeDiagnostics{
		Source:      int32(diag.source),
		TtMinusUtcS: float64(diag.tt_minus_utc_s),
		Warnings:    warnings,
	}
}

func goUtcToTdbResult(out C.DhruvUtcToTdbResult) UtcToTdbResult {
	return UtcToTdbResult{
		JdTdb:       float64(out.jd_tdb),
		Diagnostics: goTimeDiagnostics(out.diagnostics),
	}
}

func cGeo(loc GeoLocation) C.DhruvGeoLocation {
	return C.DhruvGeoLocation{
		latitude_deg:  C.double(loc.LatitudeDeg),
		longitude_deg: C.double(loc.LongitudeDeg),
		altitude_m:    C.double(loc.AltitudeM),
	}
}

func cRiseSetConfig(cfg RiseSetConfig) C.DhruvRiseSetConfig {
	return C.DhruvRiseSetConfig{
		use_refraction:      boolU8(cfg.UseRefraction),
		sun_limb:            C.int32_t(cfg.SunLimb),
		altitude_correction: boolU8(cfg.AltitudeCorrection),
	}
}

func goTimeUpagrahaConfig(cfg C.DhruvTimeUpagrahaConfig) TimeUpagrahaConfig {
	return TimeUpagrahaConfig{
		GulikaPoint:  uint8(cfg.gulika_point),
		MaandiPoint:  uint8(cfg.maandi_point),
		OtherPoint:   uint8(cfg.other_point),
		GulikaPlanet: uint8(cfg.gulika_planet),
		MaandiPlanet: uint8(cfg.maandi_planet),
	}
}

func cTimeUpagrahaConfig(cfg TimeUpagrahaConfig) C.DhruvTimeUpagrahaConfig {
	return C.DhruvTimeUpagrahaConfig{
		gulika_point:  C.int32_t(cfg.GulikaPoint),
		maandi_point:  C.int32_t(cfg.MaandiPoint),
		other_point:   C.int32_t(cfg.OtherPoint),
		gulika_planet: C.int32_t(cfg.GulikaPlanet),
		maandi_planet: C.int32_t(cfg.MaandiPlanet),
	}
}

func TimeUpagrahaConfigDefault() TimeUpagrahaConfig {
	return goTimeUpagrahaConfig(C.dhruv_time_upagraha_config_default())
}

func goRiseSetConfig(cfg C.DhruvRiseSetConfig) RiseSetConfig {
	return RiseSetConfig{
		UseRefraction:      cfg.use_refraction != 0,
		SunLimb:            int32(cfg.sun_limb),
		AltitudeCorrection: cfg.altitude_correction != 0,
	}
}

func cBhavaConfig(cfg BhavaConfig) C.DhruvBhavaConfig {
	return C.DhruvBhavaConfig{
		system:                             C.int32_t(cfg.System),
		starting_point:                     C.int32_t(cfg.StartingPoint),
		custom_start_deg:                   C.double(cfg.CustomStartDeg),
		reference_mode:                     C.int32_t(cfg.ReferenceMode),
		output_mode:                        C.int32_t(cfg.OutputMode),
		ayanamsha_system:                   C.int32_t(cfg.AyanamshaSystem),
		use_nutation:                       boolU8(cfg.UseNutation),
		reference_plane:                    C.int32_t(cfg.ReferencePlane),
		use_rashi_bhava_for_bala_avastha:   boolU8(cfg.UseRashiBhavaForBalaAvastha),
		include_node_aspects_for_drik_bala: boolU8(cfg.IncludeNodeAspectsForDrikBala),
		divide_guru_buddh_drishti_by_4_for_drik_bala: boolU8(cfg.DivideGuruBuddhDrishtiBy4ForDrikBala),
		include_rashi_bhava_results:                  boolU8(cfg.IncludeRashiBhavaResults),
	}
}

func goBhavaConfig(cfg C.DhruvBhavaConfig) BhavaConfig {
	return BhavaConfig{
		System:                               int32(cfg.system),
		StartingPoint:                        int32(cfg.starting_point),
		CustomStartDeg:                       float64(cfg.custom_start_deg),
		ReferenceMode:                        int32(cfg.reference_mode),
		OutputMode:                           int32(cfg.output_mode),
		AyanamshaSystem:                      int32(cfg.ayanamsha_system),
		UseNutation:                          cfg.use_nutation != 0,
		ReferencePlane:                       int32(cfg.reference_plane),
		UseRashiBhavaForBalaAvastha:          cfg.use_rashi_bhava_for_bala_avastha != 0,
		IncludeNodeAspectsForDrikBala:        cfg.include_node_aspects_for_drik_bala != 0,
		DivideGuruBuddhDrishtiBy4ForDrikBala: cfg.divide_guru_buddh_drishti_by_4_for_drik_bala != 0,
		IncludeRashiBhavaResults:             cfg.include_rashi_bhava_results != 0,
	}
}

func cSankrantiConfig(cfg SankrantiConfig) C.DhruvSankrantiConfig {
	return C.DhruvSankrantiConfig{
		ayanamsha_system: C.int32_t(cfg.AyanamshaSystem),
		use_nutation:     boolU8(cfg.UseNutation),
		reference_plane:  C.int32_t(cfg.ReferencePlane),
		step_size_days:   C.double(cfg.StepSizeDays),
		max_iterations:   C.uint32_t(cfg.MaxIterations),
		convergence_days: C.double(cfg.ConvergenceDays),
	}
}

func goSankrantiConfig(cfg C.DhruvSankrantiConfig) SankrantiConfig {
	return SankrantiConfig{
		AyanamshaSystem: int32(cfg.ayanamsha_system),
		UseNutation:     cfg.use_nutation != 0,
		ReferencePlane:  int32(cfg.reference_plane),
		StepSizeDays:    float64(cfg.step_size_days),
		MaxIterations:   uint32(cfg.max_iterations),
		ConvergenceDays: float64(cfg.convergence_days),
	}
}

func cGrahaLongitudesConfig(cfg GrahaLongitudesConfig) C.DhruvGrahaLongitudesConfig {
	return C.DhruvGrahaLongitudesConfig{
		kind:             C.int32_t(cfg.Kind),
		ayanamsha_system: C.int32_t(cfg.AyanamshaSystem),
		use_nutation:     boolU8(cfg.UseNutation),
		precession_model: C.int32_t(cfg.PrecessionModel),
		reference_plane:  C.int32_t(cfg.ReferencePlane),
	}
}

func GrahaLongitudesConfigDefault() GrahaLongitudesConfig {
	cfg := C.dhruv_graha_longitudes_config_default()
	return GrahaLongitudesConfig{
		Kind:            int32(cfg.kind),
		AyanamshaSystem: int32(cfg.ayanamsha_system),
		UseNutation:     cfg.use_nutation != 0,
		PrecessionModel: int32(cfg.precession_model),
		ReferencePlane:  int32(cfg.reference_plane),
	}
}

func cEngineConfig(cfg EngineConfig) (C.DhruvEngineConfig, error) {
	if len(cfg.SpkPaths) == 0 {
		return C.DhruvEngineConfig{}, fmt.Errorf("at least one SPK path is required")
	}
	if len(cfg.SpkPaths) > MaxSpkPaths {
		return C.DhruvEngineConfig{}, fmt.Errorf("spk path count exceeds %d", MaxSpkPaths)
	}
	var out C.DhruvEngineConfig
	out.spk_path_count = C.uint32_t(len(cfg.SpkPaths))
	for i, p := range cfg.SpkPaths {
		buf := (*[PathCapacity]byte)(unsafe.Pointer(&out.spk_paths_utf8[i][0]))
		if err := encodeFixedCString(buf[:], p); err != nil {
			return C.DhruvEngineConfig{}, fmt.Errorf("invalid spk path %d: %w", i, err)
		}
	}
	lsk := (*[PathCapacity]byte)(unsafe.Pointer(&out.lsk_path_utf8[0]))
	if err := encodeFixedCString(lsk[:], cfg.LskPath); err != nil {
		return C.DhruvEngineConfig{}, fmt.Errorf("invalid lsk path: %w", err)
	}
	out.cache_capacity = C.uint64_t(cfg.CacheCapacity)
	out.strict_validation = boolU8(cfg.StrictValidation)
	return out, nil
}

func cQuery(q Query) C.DhruvQuery {
	return C.DhruvQuery{
		target:       C.int32_t(q.Target),
		observer:     C.int32_t(q.Observer),
		frame:        C.int32_t(q.Frame),
		epoch_tdb_jd: C.double(q.EpochTdbJD),
	}
}

func cQueryRequest(q QueryRequest) C.DhruvQueryRequest {
	return C.DhruvQueryRequest{
		target:       C.int32_t(q.Target),
		observer:     C.int32_t(q.Observer),
		frame:        C.int32_t(q.Frame),
		time_kind:    C.int32_t(q.TimeKind),
		epoch_tdb_jd: C.double(q.EpochTdbJD),
		utc:          cUTC(q.UTC),
		output_mode:  C.int32_t(q.OutputMode),
	}
}

func goStateVector(v C.DhruvStateVector) StateVector {
	var out StateVector
	for i := 0; i < 3; i++ {
		out.PositionKm[i] = float64(v.position_km[i])
		out.VelocityKm[i] = float64(v.velocity_km_s[i])
	}
	return out
}

func goSphericalState(v C.DhruvSphericalState) SphericalState {
	return SphericalState{
		LonDeg:        float64(v.lon_deg),
		LatDeg:        float64(v.lat_deg),
		DistanceKm:    float64(v.distance_km),
		LonSpeed:      float64(v.lon_speed),
		LatSpeed:      float64(v.lat_speed),
		DistanceSpeed: float64(v.distance_speed),
	}
}

func goQueryResult(v C.DhruvQueryResult, outputMode int32) QueryResult {
	out := QueryResult{OutputMode: outputMode}
	if outputMode != QueryOutputSpherical {
		state := goStateVector(v.state_vector)
		out.State = &state
	}
	if outputMode != QueryOutputCartesian {
		spherical := goSphericalState(v.spherical_state)
		out.SphericalState = &spherical
	}
	return out
}

func goSphericalCoords(v C.DhruvSphericalCoords) SphericalCoords {
	return SphericalCoords{
		LonDeg:     float64(v.lon_deg),
		LatDeg:     float64(v.lat_deg),
		DistanceKm: float64(v.distance_km),
	}
}

func CreateEngine(cfg EngineConfig) (EngineHandle, Status, error) {
	ccfg, err := cEngineConfig(cfg)
	if err != nil {
		return EngineHandle{}, StatusInvalidConfig, err
	}
	var out *C.DhruvEngineHandle
	st := Status(C.dhruv_engine_new(&ccfg, &out))
	return EngineHandle{ptr: out}, st, nil
}

func (h *EngineHandle) Free() Status {
	if h == nil || h.ptr == nil {
		return StatusOK
	}
	st := Status(C.dhruv_engine_free(h.ptr))
	h.ptr = nil
	return st
}

func QueryEngine(h EngineHandle, q Query) (StateVector, Status) {
	cq := cQuery(q)
	var out C.DhruvStateVector
	st := Status(C.dhruv_engine_query(h.ptr, &cq, &out))
	return goStateVector(out), st
}

func QueryEngineRequest(h EngineHandle, q QueryRequest) (QueryResult, Status) {
	cq := cQueryRequest(q)
	var out C.DhruvQueryResult
	st := Status(C.dhruv_engine_query_request(h.ptr, &cq, &out))
	return goQueryResult(out, q.OutputMode), st
}

func QueryOnce(cfg EngineConfig, q Query) (StateVector, Status, error) {
	ccfg, err := cEngineConfig(cfg)
	if err != nil {
		return StateVector{}, StatusInvalidConfig, err
	}
	cq := cQuery(q)
	var out C.DhruvStateVector
	st := Status(C.dhruv_query_once(&ccfg, &cq, &out))
	return goStateVector(out), st, nil
}

func CartesianToSpherical(position [3]float64) (SphericalCoords, Status) {
	cpos := [3]C.double{C.double(position[0]), C.double(position[1]), C.double(position[2])}
	var out C.DhruvSphericalCoords
	st := Status(C.dhruv_cartesian_to_spherical(&cpos[0], &out))
	return goSphericalCoords(out), st
}

func LoadConfig(opts ConfigLoadOptions) (ConfigHandle, Status) {
	var cPath *C.char
	if opts.Path != nil {
		cPath = C.CString(*opts.Path)
		defer C.free(unsafe.Pointer(cPath))
	}
	var out *C.DhruvConfigHandle
	st := Status(C.dhruv_config_load(cPath, C.int32_t(opts.DefaultsMode), &out))
	return ConfigHandle{ptr: out}, st
}

func (h *ConfigHandle) Free() Status {
	if h == nil || h.ptr == nil {
		return StatusOK
	}
	st := Status(C.dhruv_config_free(h.ptr))
	h.ptr = nil
	return st
}

func ConfigClearActive() Status {
	return Status(C.dhruv_config_clear_active())
}

func LoadLSK(path string) (LskHandle, Status) {
	cpath := C.CString(path)
	defer C.free(unsafe.Pointer(cpath))
	var out *C.DhruvLskHandle
	st := Status(C.dhruv_lsk_load(cpath, &out))
	return LskHandle{ptr: out}, st
}

func (h *LskHandle) Free() Status {
	if h == nil || h.ptr == nil {
		return StatusOK
	}
	st := Status(C.dhruv_lsk_free(h.ptr))
	h.ptr = nil
	return st
}

func LoadEOP(path string) (EopHandle, Status) {
	cpath := C.CString(path)
	defer C.free(unsafe.Pointer(cpath))
	var out *C.DhruvEopHandle
	st := Status(C.dhruv_eop_load(cpath, &out))
	return EopHandle{ptr: out}, st
}

func (h *EopHandle) Free() Status {
	if h == nil || h.ptr == nil {
		return StatusOK
	}
	st := Status(C.dhruv_eop_free(h.ptr))
	h.ptr = nil
	return st
}

func UTCToTdbJD(lsk LskHandle, eop EopHandle, req UtcToTdbRequest) (UtcToTdbResult, Status) {
	creq := cUtcToTdbRequest(req)
	var out C.DhruvUtcToTdbResult
	st := Status(C.dhruv_utc_to_tdb_jd(lsk.ptr, eop.ptr, &creq, &out))
	return goUtcToTdbResult(out), st
}

func JdTdbToUTC(lsk LskHandle, jdTdb float64) (UtcTime, Status) {
	var out C.DhruvUtcTime
	st := Status(C.dhruv_jd_tdb_to_utc(lsk.ptr, C.double(jdTdb), &out))
	return goUTC(out), st
}

func NutationIau2000b(jdTdb float64) (float64, float64, Status) {
	var dpsi, deps C.double
	st := Status(C.dhruv_nutation_iau2000b(C.double(jdTdb), &dpsi, &deps))
	return float64(dpsi), float64(deps), st
}

func NutationIau2000bUTC(lsk LskHandle, utc UtcTime) (float64, float64, Status) {
	cutc := cUTC(utc)
	var dpsi, deps C.double
	st := Status(C.dhruv_nutation_iau2000b_utc(lsk.ptr, &cutc, &dpsi, &deps))
	return float64(dpsi), float64(deps), st
}

func ApproximateLocalNoonJD(jdUTMidnight, longitudeDeg float64) float64 {
	return float64(C.dhruv_approximate_local_noon_jd(C.double(jdUTMidnight), C.double(longitudeDeg)))
}

func AyanamshaSystemCount() uint32 { return uint32(C.dhruv_ayanamsha_system_count()) }
func ReferencePlaneDefault(systemCode int32) int32 {
	return int32(C.dhruv_reference_plane_default(C.int32_t(systemCode)))
}

func AyanamshaComputeEx(lsk LskHandle, eop EopHandle, req AyanamshaComputeRequest) (float64, Status) {
	creq := C.DhruvAyanamshaComputeRequest{
		system_code:      C.int32_t(req.SystemCode),
		mode:             C.int32_t(req.Mode),
		time_kind:        C.int32_t(req.TimeKind),
		jd_tdb:           C.double(req.JdTdb),
		utc:              cUTC(req.UTC),
		use_nutation:     boolU8(req.UseNutation),
		delta_psi_arcsec: C.double(req.DeltaPsiArcsec),
	}
	var out C.double
	st := Status(C.dhruv_ayanamsha_compute_ex(lsk.ptr, &creq, eop.ptr, &out))
	return float64(out), st
}

func RiseSetConfigDefault() RiseSetConfig {
	cfg := C.dhruv_riseset_config_default()
	return goRiseSetConfig(cfg)
}

func ComputeRiseSet(engine EngineHandle, eop EopHandle, loc GeoLocation, cfg RiseSetConfig, eventCode int32, jdTdbApprox float64, lsk LskHandle) (RiseSetResult, Status) {
	cloc := cGeo(loc)
	ccfg := cRiseSetConfig(cfg)
	var out C.DhruvRiseSetResult
	st := Status(C.dhruv_compute_rise_set(engine.ptr, lsk.ptr, eop.ptr, &cloc, C.int32_t(eventCode), C.double(jdTdbApprox), &ccfg, &out))
	return RiseSetResult{ResultType: int32(out.result_type), EventCode: int32(out.event_code), JdTdb: float64(out.jd_tdb)}, st
}

func ComputeAllEvents(engine EngineHandle, eop EopHandle, loc GeoLocation, cfg RiseSetConfig, jdTdbApprox float64, lsk LskHandle) ([8]RiseSetResult, Status) {
	cloc := cGeo(loc)
	ccfg := cRiseSetConfig(cfg)
	var out [8]C.DhruvRiseSetResult
	st := Status(C.dhruv_compute_all_events(engine.ptr, lsk.ptr, eop.ptr, &cloc, C.double(jdTdbApprox), &ccfg, (*C.DhruvRiseSetResult)(unsafe.Pointer(&out[0]))))
	var goOut [8]RiseSetResult
	for i := 0; i < 8; i++ {
		goOut[i] = RiseSetResult{ResultType: int32(out[i].result_type), EventCode: int32(out[i].event_code), JdTdb: float64(out[i].jd_tdb)}
	}
	return goOut, st
}

func ComputeRiseSetUTC(engine EngineHandle, eop EopHandle, lsk LskHandle, loc GeoLocation, eventCode int32, utc UtcTime, cfg RiseSetConfig) (RiseSetResultUTC, Status) {
	cloc := cGeo(loc)
	cutc := cUTC(utc)
	ccfg := cRiseSetConfig(cfg)
	var out C.DhruvRiseSetResultUtc
	st := Status(C.dhruv_compute_rise_set_utc(engine.ptr, lsk.ptr, eop.ptr, &cloc, C.int32_t(eventCode), &cutc, &ccfg, &out))
	return RiseSetResultUTC{ResultType: int32(out.result_type), EventCode: int32(out.event_code), UTC: goUTC(out.utc)}, st
}

func ComputeAllEventsUTC(engine EngineHandle, eop EopHandle, lsk LskHandle, loc GeoLocation, utc UtcTime, cfg RiseSetConfig) ([8]RiseSetResultUTC, Status) {
	cloc := cGeo(loc)
	cutc := cUTC(utc)
	ccfg := cRiseSetConfig(cfg)
	var out [8]C.DhruvRiseSetResultUtc
	st := Status(C.dhruv_compute_all_events_utc(engine.ptr, lsk.ptr, eop.ptr, &cloc, &cutc, &ccfg, (*C.DhruvRiseSetResultUtc)(unsafe.Pointer(&out[0]))))
	var goOut [8]RiseSetResultUTC
	for i := 0; i < 8; i++ {
		goOut[i] = RiseSetResultUTC{ResultType: int32(out[i].result_type), EventCode: int32(out[i].event_code), UTC: goUTC(out[i].utc)}
	}
	return goOut, st
}

func BhavaConfigDefault() BhavaConfig {
	cfg := C.dhruv_bhava_config_default()
	return goBhavaConfig(cfg)
}

func BhavaSystemCount() uint32 { return uint32(C.dhruv_bhava_system_count()) }

func goBhavaResult(v C.DhruvBhavaResult) BhavaResult {
	var out BhavaResult
	for i := 0; i < 12; i++ {
		out.Bhavas[i] = Bhava{
			Number:   uint8(v.bhavas[i].number),
			CuspDeg:  float64(v.bhavas[i].cusp_deg),
			StartDeg: float64(v.bhavas[i].start_deg),
			EndDeg:   float64(v.bhavas[i].end_deg),
		}
	}
	out.LagnaDeg = float64(v.lagna_deg)
	out.MCDeg = float64(v.mc_deg)
	if v.rashi_bhava_valid != 0 {
		rashi := BhavaResult{
			LagnaDeg: float64(v.rashi_bhava_lagna_deg),
			MCDeg:    float64(v.rashi_bhava_mc_deg),
		}
		for i := 0; i < 12; i++ {
			rashi.Bhavas[i] = Bhava{
				Number:   uint8(v.rashi_bhava_bhavas[i].number),
				CuspDeg:  float64(v.rashi_bhava_bhavas[i].cusp_deg),
				StartDeg: float64(v.rashi_bhava_bhavas[i].start_deg),
				EndDeg:   float64(v.rashi_bhava_bhavas[i].end_deg),
			}
		}
		out.RashiBhava = &rashi
	}
	return out
}

func ComputeBhavas(engine EngineHandle, eop EopHandle, loc GeoLocation, lsk LskHandle, jdTdb float64, cfg BhavaConfig) (BhavaResult, Status) {
	cloc := cGeo(loc)
	ccfg := cBhavaConfig(cfg)
	var out C.DhruvBhavaResult
	st := Status(C.dhruv_compute_bhavas(engine.ptr, lsk.ptr, eop.ptr, &cloc, C.double(jdTdb), &ccfg, &out))
	return goBhavaResult(out), st
}

func ComputeBhavasUTC(engine EngineHandle, eop EopHandle, lsk LskHandle, loc GeoLocation, utc UtcTime, cfg BhavaConfig) (BhavaResult, Status) {
	cloc := cGeo(loc)
	cutc := cUTC(utc)
	ccfg := cBhavaConfig(cfg)
	var out C.DhruvBhavaResult
	st := Status(C.dhruv_compute_bhavas_utc(engine.ptr, lsk.ptr, eop.ptr, &cloc, &cutc, &ccfg, &out))
	return goBhavaResult(out), st
}

func LagnaDeg(lsk LskHandle, eop EopHandle, loc GeoLocation, jdTdb float64) (float64, Status) {
	cloc := cGeo(loc)
	var out C.double
	st := Status(C.dhruv_lagna_deg(lsk.ptr, eop.ptr, &cloc, C.double(jdTdb), &out))
	return float64(out), st
}

func LagnaDegWithConfig(lsk LskHandle, eop EopHandle, loc GeoLocation, jdTdb float64, cfg BhavaConfig) (float64, Status) {
	cloc := cGeo(loc)
	ccfg := cBhavaConfig(cfg)
	var out C.double
	st := Status(C.dhruv_lagna_deg_with_config(lsk.ptr, eop.ptr, &cloc, C.double(jdTdb), &ccfg, &out))
	return float64(out), st
}

func MCDeg(lsk LskHandle, eop EopHandle, loc GeoLocation, jdTdb float64) (float64, Status) {
	cloc := cGeo(loc)
	var out C.double
	st := Status(C.dhruv_mc_deg(lsk.ptr, eop.ptr, &cloc, C.double(jdTdb), &out))
	return float64(out), st
}

func MCDegWithConfig(lsk LskHandle, eop EopHandle, loc GeoLocation, jdTdb float64, cfg BhavaConfig) (float64, Status) {
	cloc := cGeo(loc)
	ccfg := cBhavaConfig(cfg)
	var out C.double
	st := Status(C.dhruv_mc_deg_with_config(lsk.ptr, eop.ptr, &cloc, C.double(jdTdb), &ccfg, &out))
	return float64(out), st
}

func RAMCDeg(lsk LskHandle, eop EopHandle, loc GeoLocation, jdTdb float64) (float64, Status) {
	cloc := cGeo(loc)
	var out C.double
	st := Status(C.dhruv_ramc_deg(lsk.ptr, eop.ptr, &cloc, C.double(jdTdb), &out))
	return float64(out), st
}

func LagnaDegUTC(lsk LskHandle, eop EopHandle, loc GeoLocation, utc UtcTime) (float64, Status) {
	cloc := cGeo(loc)
	cutc := cUTC(utc)
	var out C.double
	st := Status(C.dhruv_lagna_deg_utc(lsk.ptr, eop.ptr, &cloc, &cutc, &out))
	return float64(out), st
}

func LagnaDegUTCWithConfig(lsk LskHandle, eop EopHandle, loc GeoLocation, utc UtcTime, cfg BhavaConfig) (float64, Status) {
	cloc := cGeo(loc)
	cutc := cUTC(utc)
	ccfg := cBhavaConfig(cfg)
	var out C.double
	st := Status(C.dhruv_lagna_deg_utc_with_config(lsk.ptr, eop.ptr, &cloc, &cutc, &ccfg, &out))
	return float64(out), st
}

func MCDegUTC(lsk LskHandle, eop EopHandle, loc GeoLocation, utc UtcTime) (float64, Status) {
	cloc := cGeo(loc)
	cutc := cUTC(utc)
	var out C.double
	st := Status(C.dhruv_mc_deg_utc(lsk.ptr, eop.ptr, &cloc, &cutc, &out))
	return float64(out), st
}

func MCDegUTCWithConfig(lsk LskHandle, eop EopHandle, loc GeoLocation, utc UtcTime, cfg BhavaConfig) (float64, Status) {
	cloc := cGeo(loc)
	cutc := cUTC(utc)
	ccfg := cBhavaConfig(cfg)
	var out C.double
	st := Status(C.dhruv_mc_deg_utc_with_config(lsk.ptr, eop.ptr, &cloc, &cutc, &ccfg, &out))
	return float64(out), st
}

func RAMCDegUTC(lsk LskHandle, eop EopHandle, loc GeoLocation, utc UtcTime) (float64, Status) {
	cloc := cGeo(loc)
	cutc := cUTC(utc)
	var out C.double
	st := Status(C.dhruv_ramc_deg_utc(lsk.ptr, eop.ptr, &cloc, &cutc, &out))
	return float64(out), st
}

func LunarNodeCount() uint32 { return uint32(C.dhruv_lunar_node_count()) }

func LunarNodeDeg(nodeCode, modeCode int32, jdTdb float64) (float64, Status) {
	var out C.double
	st := Status(C.dhruv_lunar_node_deg(C.int32_t(nodeCode), C.int32_t(modeCode), C.double(jdTdb), &out))
	return float64(out), st
}

func LunarNodeDegWithEngine(engine EngineHandle, nodeCode, modeCode int32, jdTdb float64) (float64, Status) {
	var out C.double
	st := Status(C.dhruv_lunar_node_deg_with_engine(engine.ptr, C.int32_t(nodeCode), C.int32_t(modeCode), C.double(jdTdb), &out))
	return float64(out), st
}

func LunarNodeDegUTC(lsk LskHandle, nodeCode, modeCode int32, utc UtcTime) (float64, Status) {
	cutc := cUTC(utc)
	var out C.double
	st := Status(C.dhruv_lunar_node_deg_utc(lsk.ptr, C.int32_t(nodeCode), C.int32_t(modeCode), &cutc, &out))
	return float64(out), st
}

func LunarNodeDegUTCWithEngine(engine EngineHandle, lsk LskHandle, nodeCode, modeCode int32, utc UtcTime) (float64, Status) {
	cutc := cUTC(utc)
	var out C.double
	st := Status(C.dhruv_lunar_node_deg_utc_with_engine(engine.ptr, lsk.ptr, C.int32_t(nodeCode), C.int32_t(modeCode), &cutc, &out))
	return float64(out), st
}

func LunarNodeComputeEx(lsk LskHandle, eop EopHandle, req LunarNodeRequest) (float64, Status) {
	creq := C.DhruvLunarNodeRequest{
		node_code: C.int32_t(req.NodeCode),
		mode_code: C.int32_t(req.ModeCode),
		backend:   C.int32_t(req.Backend),
		time_kind: C.int32_t(req.TimeKind),
		jd_tdb:    C.double(req.JdTdb),
		utc:       cUTC(req.UTC),
	}
	var out C.double
	st := Status(C.dhruv_lunar_node_compute_ex(lsk.ptr, eop.ptr, &creq, &out))
	return float64(out), st
}

func ConjunctionConfigDefault() ConjunctionConfig {
	cfg := C.dhruv_conjunction_config_default()
	return ConjunctionConfig{
		TargetSeparationDeg: float64(cfg.target_separation_deg),
		StepSizeDays:        float64(cfg.step_size_days),
		MaxIterations:       uint32(cfg.max_iterations),
		ConvergenceDays:     float64(cfg.convergence_days),
	}
}

func SearchConjunction(engine EngineHandle, req ConjunctionSearchRequest, capacity uint32) (ConjunctionEvent, bool, []ConjunctionEvent, Status) {
	timeKind := resolveSearchTimeKind(req.QueryMode, req.TimeKind, req.AtUTC, req.StartUTC, req.EndUTC)
	creq := C.DhruvConjunctionSearchRequest{
		body1_code:   C.int32_t(req.Body1Code),
		body2_code:   C.int32_t(req.Body2Code),
		query_mode:   C.int32_t(req.QueryMode),
		time_kind:    C.int32_t(timeKind),
		at_jd_tdb:    C.double(req.AtJdTdb),
		start_jd_tdb: C.double(req.StartJdTdb),
		end_jd_tdb:   C.double(req.EndJdTdb),
		at_utc:       cUTC(req.AtUTC),
		start_utc:    cUTC(req.StartUTC),
		end_utc:      cUTC(req.EndUTC),
		config: C.DhruvConjunctionConfig{
			target_separation_deg: C.double(req.Config.TargetSeparationDeg),
			step_size_days:        C.double(req.Config.StepSizeDays),
			max_iterations:        C.uint32_t(req.Config.MaxIterations),
			convergence_days:      C.double(req.Config.ConvergenceDays),
		},
	}
	var outEvent C.DhruvConjunctionEvent
	var found C.uint8_t
	var outCount C.uint32_t
	var arr []C.DhruvConjunctionEvent
	var arrPtr *C.DhruvConjunctionEvent
	if capacity > 0 {
		arr = make([]C.DhruvConjunctionEvent, capacity)
		arrPtr = &arr[0]
	}
	st := Status(C.dhruv_conjunction_search_ex(engine.ptr, &creq, &outEvent, &found, arrPtr, C.uint32_t(capacity), &outCount))
	goEvent := ConjunctionEvent{
		UTC:                 goUTC(outEvent.utc),
		JdTdb:               float64(outEvent.jd_tdb),
		ActualSeparationDeg: float64(outEvent.actual_separation_deg),
		Body1LongitudeDeg:   float64(outEvent.body1_longitude_deg),
		Body2LongitudeDeg:   float64(outEvent.body2_longitude_deg),
		Body1LatitudeDeg:    float64(outEvent.body1_latitude_deg),
		Body2LatitudeDeg:    float64(outEvent.body2_latitude_deg),
		Body1Code:           int32(outEvent.body1_code),
		Body2Code:           int32(outEvent.body2_code),
	}
	count := int(outCount)
	if count > len(arr) {
		count = len(arr)
	}
	events := make([]ConjunctionEvent, count)
	for i := 0; i < count; i++ {
		events[i] = ConjunctionEvent{
			UTC:                 goUTC(arr[i].utc),
			JdTdb:               float64(arr[i].jd_tdb),
			ActualSeparationDeg: float64(arr[i].actual_separation_deg),
			Body1LongitudeDeg:   float64(arr[i].body1_longitude_deg),
			Body2LongitudeDeg:   float64(arr[i].body2_longitude_deg),
			Body1LatitudeDeg:    float64(arr[i].body1_latitude_deg),
			Body2LatitudeDeg:    float64(arr[i].body2_latitude_deg),
			Body1Code:           int32(arr[i].body1_code),
			Body2Code:           int32(arr[i].body2_code),
		}
	}
	return goEvent, found != 0, events, st
}

func GrahanConfigDefault() GrahanConfig {
	cfg := C.dhruv_grahan_config_default()
	return GrahanConfig{IncludePenumbral: cfg.include_penumbral != 0, IncludePeakDetails: cfg.include_peak_details != 0}
}

func SearchGrahan(engine EngineHandle, req GrahanSearchRequest, capacity uint32) (ChandraGrahanResult, SuryaGrahanResult, bool, []ChandraGrahanResult, []SuryaGrahanResult, Status) {
	timeKind := resolveSearchTimeKind(req.QueryMode, req.TimeKind, req.AtUTC, req.StartUTC, req.EndUTC)
	creq := C.DhruvGrahanSearchRequest{
		grahan_kind:  C.int32_t(req.GrahanKind),
		query_mode:   C.int32_t(req.QueryMode),
		time_kind:    C.int32_t(timeKind),
		at_jd_tdb:    C.double(req.AtJdTdb),
		start_jd_tdb: C.double(req.StartJdTdb),
		end_jd_tdb:   C.double(req.EndJdTdb),
		at_utc:       cUTC(req.AtUTC),
		start_utc:    cUTC(req.StartUTC),
		end_utc:      cUTC(req.EndUTC),
		config:       C.DhruvGrahanConfig{include_penumbral: boolU8(req.Config.IncludePenumbral), include_peak_details: boolU8(req.Config.IncludePeakDetails)},
	}
	var outC C.DhruvChandraGrahanResult
	var outS C.DhruvSuryaGrahanResult
	var found C.uint8_t
	var outCount C.uint32_t
	var carr []C.DhruvChandraGrahanResult
	var sarr []C.DhruvSuryaGrahanResult
	var cptr *C.DhruvChandraGrahanResult
	var sptr *C.DhruvSuryaGrahanResult
	if capacity > 0 {
		carr = make([]C.DhruvChandraGrahanResult, capacity)
		sarr = make([]C.DhruvSuryaGrahanResult, capacity)
		cptr = &carr[0]
		sptr = &sarr[0]
	}
	st := Status(C.dhruv_grahan_search_ex(engine.ptr, &creq, &outC, &outS, &found, cptr, sptr, C.uint32_t(capacity), &outCount))
	toC := func(v C.DhruvChandraGrahanResult) ChandraGrahanResult {
		return ChandraGrahanResult{
			GrahanType:           int32(v.grahan_type),
			Magnitude:            float64(v.magnitude),
			PenumbralMagnitude:   float64(v.penumbral_magnitude),
			GreatestGrahanUTC:    goUTC(v.greatest_grahan_utc),
			GreatestGrahanJd:     float64(v.greatest_grahan_jd),
			P1UTC:                goUTC(v.p1_utc),
			P1Jd:                 float64(v.p1_jd),
			U1UTC:                goOptionalUTC(float64(v.u1_jd) != -1.0, v.u1_utc),
			U1Jd:                 float64(v.u1_jd),
			U2UTC:                goOptionalUTC(float64(v.u2_jd) != -1.0, v.u2_utc),
			U2Jd:                 float64(v.u2_jd),
			U3UTC:                goOptionalUTC(float64(v.u3_jd) != -1.0, v.u3_utc),
			U3Jd:                 float64(v.u3_jd),
			U4UTC:                goOptionalUTC(float64(v.u4_jd) != -1.0, v.u4_utc),
			U4Jd:                 float64(v.u4_jd),
			P4UTC:                goUTC(v.p4_utc),
			P4Jd:                 float64(v.p4_jd),
			MoonEclipticLatDeg:   float64(v.moon_ecliptic_lat_deg),
			AngularSeparationDeg: float64(v.angular_separation_deg),
		}
	}
	toS := func(v C.DhruvSuryaGrahanResult) SuryaGrahanResult {
		return SuryaGrahanResult{
			GrahanType:           int32(v.grahan_type),
			Magnitude:            float64(v.magnitude),
			GreatestGrahanUTC:    goUTC(v.greatest_grahan_utc),
			GreatestGrahanJd:     float64(v.greatest_grahan_jd),
			C1UTC:                goOptionalUTC(float64(v.c1_jd) != -1.0, v.c1_utc),
			C1Jd:                 float64(v.c1_jd),
			C2UTC:                goOptionalUTC(float64(v.c2_jd) != -1.0, v.c2_utc),
			C2Jd:                 float64(v.c2_jd),
			C3UTC:                goOptionalUTC(float64(v.c3_jd) != -1.0, v.c3_utc),
			C3Jd:                 float64(v.c3_jd),
			C4UTC:                goOptionalUTC(float64(v.c4_jd) != -1.0, v.c4_utc),
			C4Jd:                 float64(v.c4_jd),
			MoonEclipticLatDeg:   float64(v.moon_ecliptic_lat_deg),
			AngularSeparationDeg: float64(v.angular_separation_deg),
		}
	}
	count := int(outCount)
	if count > len(carr) {
		count = len(carr)
	}
	chEvents := make([]ChandraGrahanResult, count)
	suEvents := make([]SuryaGrahanResult, count)
	for i := 0; i < count; i++ {
		chEvents[i] = toC(carr[i])
		suEvents[i] = toS(sarr[i])
	}
	return toC(outC), toS(outS), found != 0, chEvents, suEvents, st
}

func StationaryConfigDefault() StationaryConfig {
	cfg := C.dhruv_stationary_config_default()
	return StationaryConfig{
		StepSizeDays:      float64(cfg.step_size_days),
		MaxIterations:     uint32(cfg.max_iterations),
		ConvergenceDays:   float64(cfg.convergence_days),
		NumericalStepDays: float64(cfg.numerical_step_days),
	}
}

func SearchMotion(engine EngineHandle, req MotionSearchRequest, capacity uint32) (StationaryEvent, MaxSpeedEvent, bool, []StationaryEvent, []MaxSpeedEvent, Status) {
	timeKind := resolveSearchTimeKind(req.QueryMode, req.TimeKind, req.AtUTC, req.StartUTC, req.EndUTC)
	creq := C.DhruvMotionSearchRequest{
		body_code:    C.int32_t(req.BodyCode),
		motion_kind:  C.int32_t(req.MotionKind),
		query_mode:   C.int32_t(req.QueryMode),
		time_kind:    C.int32_t(timeKind),
		at_jd_tdb:    C.double(req.AtJdTdb),
		start_jd_tdb: C.double(req.StartJdTdb),
		end_jd_tdb:   C.double(req.EndJdTdb),
		at_utc:       cUTC(req.AtUTC),
		start_utc:    cUTC(req.StartUTC),
		end_utc:      cUTC(req.EndUTC),
		config: C.DhruvStationaryConfig{
			step_size_days:      C.double(req.Config.StepSizeDays),
			max_iterations:      C.uint32_t(req.Config.MaxIterations),
			convergence_days:    C.double(req.Config.ConvergenceDays),
			numerical_step_days: C.double(req.Config.NumericalStepDays),
		},
	}
	var outSt C.DhruvStationaryEvent
	var outMs C.DhruvMaxSpeedEvent
	var found C.uint8_t
	var outCount C.uint32_t
	var stArr []C.DhruvStationaryEvent
	var msArr []C.DhruvMaxSpeedEvent
	var stPtr *C.DhruvStationaryEvent
	var msPtr *C.DhruvMaxSpeedEvent
	if capacity > 0 {
		stArr = make([]C.DhruvStationaryEvent, capacity)
		msArr = make([]C.DhruvMaxSpeedEvent, capacity)
		stPtr = &stArr[0]
		msPtr = &msArr[0]
	}
	st := Status(C.dhruv_motion_search_ex(engine.ptr, &creq, &outSt, &outMs, &found, stPtr, msPtr, C.uint32_t(capacity), &outCount))
	convSt := func(v C.DhruvStationaryEvent) StationaryEvent {
		return StationaryEvent{UTC: goUTC(v.utc), JdTdb: float64(v.jd_tdb), BodyCode: int32(v.body_code), LongitudeDeg: float64(v.longitude_deg), LatitudeDeg: float64(v.latitude_deg), StationType: int32(v.station_type)}
	}
	convMs := func(v C.DhruvMaxSpeedEvent) MaxSpeedEvent {
		return MaxSpeedEvent{UTC: goUTC(v.utc), JdTdb: float64(v.jd_tdb), BodyCode: int32(v.body_code), LongitudeDeg: float64(v.longitude_deg), LatitudeDeg: float64(v.latitude_deg), SpeedDegPerDay: float64(v.speed_deg_per_day), SpeedType: int32(v.speed_type)}
	}
	count := int(outCount)
	if count > len(stArr) {
		count = len(stArr)
	}
	stEvents := make([]StationaryEvent, count)
	msEvents := make([]MaxSpeedEvent, count)
	for i := 0; i < count; i++ {
		stEvents[i] = convSt(stArr[i])
		msEvents[i] = convMs(msArr[i])
	}
	return convSt(outSt), convMs(outMs), found != 0, stEvents, msEvents, st
}

func SankrantiConfigDefault() SankrantiConfig {
	return goSankrantiConfig(C.dhruv_sankranti_config_default())
}

func SearchLunarPhase(engine EngineHandle, req LunarPhaseSearchRequest, capacity uint32) (LunarPhaseEvent, bool, []LunarPhaseEvent, Status) {
	timeKind := resolveSearchTimeKind(req.QueryMode, req.TimeKind, req.AtUTC, req.StartUTC, req.EndUTC)
	creq := C.DhruvLunarPhaseSearchRequest{
		phase_kind:   C.int32_t(req.PhaseKind),
		query_mode:   C.int32_t(req.QueryMode),
		time_kind:    C.int32_t(timeKind),
		at_jd_tdb:    C.double(req.AtJdTdb),
		start_jd_tdb: C.double(req.StartJdTdb),
		end_jd_tdb:   C.double(req.EndJdTdb),
		at_utc:       cUTC(req.AtUTC),
		start_utc:    cUTC(req.StartUTC),
		end_utc:      cUTC(req.EndUTC),
	}
	var out C.DhruvLunarPhaseEvent
	var found C.uint8_t
	var outCount C.uint32_t
	var arr []C.DhruvLunarPhaseEvent
	var ptr *C.DhruvLunarPhaseEvent
	if capacity > 0 {
		arr = make([]C.DhruvLunarPhaseEvent, capacity)
		ptr = &arr[0]
	}
	st := Status(C.dhruv_lunar_phase_search_ex(engine.ptr, &creq, &out, &found, ptr, C.uint32_t(capacity), &outCount))
	conv := func(v C.DhruvLunarPhaseEvent) LunarPhaseEvent {
		return LunarPhaseEvent{UTC: goUTC(v.utc), Phase: int32(v.phase), MoonLongitudeDeg: float64(v.moon_longitude_deg), SunLongitudeDeg: float64(v.sun_longitude_deg)}
	}
	count := int(outCount)
	if count > len(arr) {
		count = len(arr)
	}
	events := make([]LunarPhaseEvent, count)
	for i := 0; i < count; i++ {
		events[i] = conv(arr[i])
	}
	return conv(out), found != 0, events, st
}

func SearchSankranti(engine EngineHandle, req SankrantiSearchRequest, capacity uint32) (SankrantiEvent, bool, []SankrantiEvent, Status) {
	timeKind := resolveSearchTimeKind(req.QueryMode, req.TimeKind, req.AtUTC, req.StartUTC, req.EndUTC)
	creq := C.DhruvSankrantiSearchRequest{
		target_kind:  C.int32_t(req.TargetKind),
		query_mode:   C.int32_t(req.QueryMode),
		rashi_index:  C.int32_t(req.RashiIndex),
		time_kind:    C.int32_t(timeKind),
		at_jd_tdb:    C.double(req.AtJdTdb),
		start_jd_tdb: C.double(req.StartJdTdb),
		end_jd_tdb:   C.double(req.EndJdTdb),
		at_utc:       cUTC(req.AtUTC),
		start_utc:    cUTC(req.StartUTC),
		end_utc:      cUTC(req.EndUTC),
		config:       cSankrantiConfig(req.Config),
	}
	var out C.DhruvSankrantiEvent
	var found C.uint8_t
	var outCount C.uint32_t
	var arr []C.DhruvSankrantiEvent
	var ptr *C.DhruvSankrantiEvent
	if capacity > 0 {
		arr = make([]C.DhruvSankrantiEvent, capacity)
		ptr = &arr[0]
	}
	st := Status(C.dhruv_sankranti_search_ex(engine.ptr, &creq, &out, &found, ptr, C.uint32_t(capacity), &outCount))
	conv := func(v C.DhruvSankrantiEvent) SankrantiEvent {
		return SankrantiEvent{UTC: goUTC(v.utc), RashiIndex: int32(v.rashi_index), SunSiderealLongitudeDeg: float64(v.sun_sidereal_longitude_deg), SunTropicalLongitudeDeg: float64(v.sun_tropical_longitude_deg)}
	}
	count := int(outCount)
	if count > len(arr) {
		count = len(arr)
	}
	events := make([]SankrantiEvent, count)
	for i := 0; i < count; i++ {
		events[i] = conv(arr[i])
	}
	return conv(out), found != 0, events, st
}

func goTithiInfo(v C.DhruvTithiInfo) TithiInfo {
	return TithiInfo{TithiIndex: int32(v.tithi_index), Paksha: int32(v.paksha), TithiInPaksha: int32(v.tithi_in_paksha), Start: goUTC(v.start), End: goUTC(v.end)}
}

func goKaranaInfo(v C.DhruvKaranaInfo) KaranaInfo {
	return KaranaInfo{KaranaIndex: int32(v.karana_index), KaranaNameIndex: int32(v.karana_name_index), Start: goUTC(v.start), End: goUTC(v.end)}
}

func goYogaInfo(v C.DhruvYogaInfo) YogaInfo {
	return YogaInfo{YogaIndex: int32(v.yoga_index), Start: goUTC(v.start), End: goUTC(v.end)}
}

func goVaarInfo(v C.DhruvVaarInfo) VaarInfo {
	return VaarInfo{VaarIndex: int32(v.vaar_index), Start: goUTC(v.start), End: goUTC(v.end)}
}

func goHoraInfo(v C.DhruvHoraInfo) HoraInfo {
	return HoraInfo{HoraIndex: int32(v.hora_index), HoraPosition: int32(v.hora_position), Start: goUTC(v.start), End: goUTC(v.end)}
}

func goGhatikaInfo(v C.DhruvGhatikaInfo) GhatikaInfo {
	return GhatikaInfo{Value: int32(v.value), Start: goUTC(v.start), End: goUTC(v.end)}
}

func goNakshatraInfo(v C.DhruvPanchangNakshatraInfo) PanchangNakshatraInfo {
	return PanchangNakshatraInfo{NakshatraIndex: int32(v.nakshatra_index), Pada: int32(v.pada), Start: goUTC(v.start), End: goUTC(v.end)}
}

func goMasaInfo(v C.DhruvMasaInfo) MasaInfo {
	return MasaInfo{MasaIndex: int32(v.masa_index), Adhika: v.adhika != 0, Start: goUTC(v.start), End: goUTC(v.end)}
}

func goAyanaInfo(v C.DhruvAyanaInfo) AyanaInfo {
	return AyanaInfo{Ayana: int32(v.ayana), Start: goUTC(v.start), End: goUTC(v.end)}
}

func goVarshaInfo(v C.DhruvVarshaInfo) VarshaInfo {
	return VarshaInfo{SamvatsaraIndex: int32(v.samvatsara_index), Order: int32(v.order), Start: goUTC(v.start), End: goUTC(v.end)}
}

func TithiForDate(engine EngineHandle, utc UtcTime) (TithiInfo, Status) {
	cutc := cUTC(utc)
	var out C.DhruvTithiInfo
	st := Status(C.dhruv_tithi_for_date(engine.ptr, &cutc, &out))
	return goTithiInfo(out), st
}

func KaranaForDate(engine EngineHandle, utc UtcTime) (KaranaInfo, Status) {
	cutc := cUTC(utc)
	var out C.DhruvKaranaInfo
	st := Status(C.dhruv_karana_for_date(engine.ptr, &cutc, &out))
	return goKaranaInfo(out), st
}

func YogaForDate(engine EngineHandle, utc UtcTime, cfg SankrantiConfig) (YogaInfo, Status) {
	cutc := cUTC(utc)
	ccfg := cSankrantiConfig(cfg)
	var out C.DhruvYogaInfo
	st := Status(C.dhruv_yoga_for_date(engine.ptr, &cutc, &ccfg, &out))
	return goYogaInfo(out), st
}

func NakshatraForDate(engine EngineHandle, utc UtcTime, cfg SankrantiConfig) (PanchangNakshatraInfo, Status) {
	cutc := cUTC(utc)
	ccfg := cSankrantiConfig(cfg)
	var out C.DhruvPanchangNakshatraInfo
	st := Status(C.dhruv_nakshatra_for_date(engine.ptr, &cutc, &ccfg, &out))
	return goNakshatraInfo(out), st
}

func VaarForDate(engine EngineHandle, eop EopHandle, utc UtcTime, loc GeoLocation, cfg RiseSetConfig) (VaarInfo, Status) {
	cutc, cloc, ccfg := cUTC(utc), cGeo(loc), cRiseSetConfig(cfg)
	var out C.DhruvVaarInfo
	st := Status(C.dhruv_vaar_for_date(engine.ptr, eop.ptr, &cutc, &cloc, &ccfg, &out))
	return goVaarInfo(out), st
}

func HoraForDate(engine EngineHandle, eop EopHandle, utc UtcTime, loc GeoLocation, cfg RiseSetConfig) (HoraInfo, Status) {
	cutc, cloc, ccfg := cUTC(utc), cGeo(loc), cRiseSetConfig(cfg)
	var out C.DhruvHoraInfo
	st := Status(C.dhruv_hora_for_date(engine.ptr, eop.ptr, &cutc, &cloc, &ccfg, &out))
	return goHoraInfo(out), st
}

func GhatikaForDate(engine EngineHandle, eop EopHandle, utc UtcTime, loc GeoLocation, cfg RiseSetConfig) (GhatikaInfo, Status) {
	cutc, cloc, ccfg := cUTC(utc), cGeo(loc), cRiseSetConfig(cfg)
	var out C.DhruvGhatikaInfo
	st := Status(C.dhruv_ghatika_for_date(engine.ptr, eop.ptr, &cutc, &cloc, &ccfg, &out))
	return goGhatikaInfo(out), st
}

func MasaForDate(engine EngineHandle, utc UtcTime, cfg SankrantiConfig) (MasaInfo, Status) {
	cutc := cUTC(utc)
	ccfg := cSankrantiConfig(cfg)
	var out C.DhruvMasaInfo
	st := Status(C.dhruv_masa_for_date(engine.ptr, &cutc, &ccfg, &out))
	return goMasaInfo(out), st
}

func AyanaForDate(engine EngineHandle, utc UtcTime, cfg SankrantiConfig) (AyanaInfo, Status) {
	cutc := cUTC(utc)
	ccfg := cSankrantiConfig(cfg)
	var out C.DhruvAyanaInfo
	st := Status(C.dhruv_ayana_for_date(engine.ptr, &cutc, &ccfg, &out))
	return goAyanaInfo(out), st
}

func VarshaForDate(engine EngineHandle, utc UtcTime, cfg SankrantiConfig) (VarshaInfo, Status) {
	cutc := cUTC(utc)
	ccfg := cSankrantiConfig(cfg)
	var out C.DhruvVarshaInfo
	st := Status(C.dhruv_varsha_for_date(engine.ptr, &cutc, &ccfg, &out))
	return goVarshaInfo(out), st
}

func PanchangComputeEx(engine EngineHandle, eop EopHandle, lsk LskHandle, req PanchangComputeRequest) (PanchangOperationResult, Status) {
	creq := C.DhruvPanchangComputeRequest{
		time_kind:        C.int32_t(req.TimeKind),
		jd_tdb:           C.double(req.JdTdb),
		utc:              cUTC(req.UTC),
		include_mask:     C.uint32_t(req.IncludeMask),
		location:         cGeo(req.Location),
		riseset_config:   cRiseSetConfig(req.RiseSetConfig),
		sankranti_config: cSankrantiConfig(req.SankrantiConfig),
	}
	var out C.DhruvPanchangOperationResult
	st := Status(C.dhruv_panchang_compute_ex(engine.ptr, eop.ptr, lsk.ptr, &creq, &out))
	res := PanchangOperationResult{
		TithiValid:     out.tithi_valid != 0,
		Tithi:          goTithiInfo(out.tithi),
		KaranaValid:    out.karana_valid != 0,
		Karana:         goKaranaInfo(out.karana),
		YogaValid:      out.yoga_valid != 0,
		Yoga:           goYogaInfo(out.yoga),
		VaarValid:      out.vaar_valid != 0,
		Vaar:           goVaarInfo(out.vaar),
		HoraValid:      out.hora_valid != 0,
		Hora:           goHoraInfo(out.hora),
		GhatikaValid:   out.ghatika_valid != 0,
		Ghatika:        goGhatikaInfo(out.ghatika),
		NakshatraValid: out.nakshatra_valid != 0,
		Nakshatra:      goNakshatraInfo(out.nakshatra),
		MasaValid:      out.masa_valid != 0,
		Masa:           goMasaInfo(out.masa),
		AyanaValid:     out.ayana_valid != 0,
		Ayana:          goAyanaInfo(out.ayana),
		VarshaValid:    out.varsha_valid != 0,
		Varsha:         goVarshaInfo(out.varsha),
	}
	return res, st
}

func ComputeGrahaLongitudes(engine EngineHandle, jdTdb float64, cfg GrahaLongitudesConfig) (GrahaLongitudes, Status) {
	var out C.DhruvGrahaLongitudes
	ccfg := cGrahaLongitudesConfig(cfg)
	st := Status(C.dhruv_graha_longitudes(engine.ptr, C.double(jdTdb), &ccfg, &out))
	var goOut GrahaLongitudes
	for i := 0; i < GrahaCount; i++ {
		goOut.Longitudes[i] = float64(out.longitudes[i])
	}
	return goOut, st
}

func MovingOsculatingApogeesForDate(engine EngineHandle, eop EopHandle, utc UtcTime, grahas []uint8, cfg GrahaLongitudesConfig) (MovingOsculatingApogees, Status) {
	cutc := cUTC(utc)
	ccfg := cGrahaLongitudesConfig(cfg)
	var out C.DhruvMovingOsculatingApogees
	var ptr *C.uint8_t
	if len(grahas) > 0 {
		ptr = (*C.uint8_t)(&grahas[0])
	}
	st := Status(C.dhruv_moving_osculating_apogees_for_date(
		engine.ptr,
		eop.ptr,
		&cutc,
		ptr,
		C.uint8_t(len(grahas)),
		&ccfg,
		&out,
	))
	goOut := MovingOsculatingApogees{Entries: make([]MovingOsculatingApogeeEntry, int(out.count))}
	for i := 0; i < int(out.count); i++ {
		entry := out.entries[i]
		goOut.Entries[i] = MovingOsculatingApogeeEntry{
			GrahaIndex:              uint8(entry.graha_index),
			SiderealLongitude:       float64(entry.sidereal_longitude),
			AyanamshaDeg:            float64(entry.ayanamsha_deg),
			ReferencePlaneLongitude: float64(entry.reference_plane_longitude),
		}
	}
	return goOut, st
}

func SpecialLagnasForDate(engine EngineHandle, eop EopHandle, utc UtcTime, loc GeoLocation, riseset RiseSetConfig, ayanamshaSystem uint32, useNutation bool) (SpecialLagnas, Status) {
	cutc, cloc, ccfg := cUTC(utc), cGeo(loc), cRiseSetConfig(riseset)
	var out C.DhruvSpecialLagnas
	st := Status(C.dhruv_special_lagnas_for_date(engine.ptr, eop.ptr, &cutc, &cloc, &ccfg, C.uint32_t(ayanamshaSystem), boolU8(useNutation), &out))
	return SpecialLagnas{
		BhavaLagna:     float64(out.bhava_lagna),
		HoraLagna:      float64(out.hora_lagna),
		GhatiLagna:     float64(out.ghati_lagna),
		VighatiLagna:   float64(out.vighati_lagna),
		VarnadaLagna:   float64(out.varnada_lagna),
		SreeLagna:      float64(out.sree_lagna),
		PranapadaLagna: float64(out.pranapada_lagna),
		InduLagna:      float64(out.indu_lagna),
	}, st
}

func ArudhaPadasForDate(engine EngineHandle, eop EopHandle, utc UtcTime, loc GeoLocation, ayanamshaSystem uint32, useNutation bool) ([12]ArudhaResult, Status) {
	cutc, cloc := cUTC(utc), cGeo(loc)
	var out [12]C.DhruvArudhaResult
	st := Status(C.dhruv_arudha_padas_for_date(engine.ptr, eop.ptr, &cutc, &cloc, C.uint32_t(ayanamshaSystem), boolU8(useNutation), (*C.DhruvArudhaResult)(unsafe.Pointer(&out[0]))))
	var goOut [12]ArudhaResult
	for i := 0; i < 12; i++ {
		goOut[i] = ArudhaResult{BhavaNumber: uint8(out[i].bhava_number), LongitudeDeg: float64(out[i].longitude_deg), RashiIndex: uint8(out[i].rashi_index)}
	}
	return goOut, st
}

func AllUpagrahasForDate(engine EngineHandle, eop EopHandle, utc UtcTime, loc GeoLocation, ayanamshaSystem uint32, useNutation bool) (AllUpagrahas, Status) {
	cutc, cloc := cUTC(utc), cGeo(loc)
	var out C.DhruvAllUpagrahas
	st := Status(C.dhruv_all_upagrahas_for_date(engine.ptr, eop.ptr, &cutc, &cloc, C.uint32_t(ayanamshaSystem), boolU8(useNutation), &out))
	return AllUpagrahas{
		Gulika: float64(out.gulika), Maandi: float64(out.maandi), Kaala: float64(out.kaala), Mrityu: float64(out.mrityu),
		ArthaPrahara: float64(out.artha_prahara), YamaGhantaka: float64(out.yama_ghantaka), Dhooma: float64(out.dhooma),
		Vyatipata: float64(out.vyatipata), Parivesha: float64(out.parivesha), IndraChapa: float64(out.indra_chapa), Upaketu: float64(out.upaketu),
	}, st
}

func AllUpagrahasForDateWithConfig(engine EngineHandle, eop EopHandle, utc UtcTime, loc GeoLocation, ayanamshaSystem uint32, useNutation bool, cfg TimeUpagrahaConfig) (AllUpagrahas, Status) {
	cutc, cloc, ccfg := cUTC(utc), cGeo(loc), cTimeUpagrahaConfig(cfg)
	var out C.DhruvAllUpagrahas
	st := Status(C.dhruv_all_upagrahas_for_date_with_config(
		engine.ptr,
		eop.ptr,
		&cutc,
		&cloc,
		C.uint32_t(ayanamshaSystem),
		boolU8(useNutation),
		&ccfg,
		&out,
	))
	return AllUpagrahas{
		Gulika: float64(out.gulika), Maandi: float64(out.maandi), Kaala: float64(out.kaala), Mrityu: float64(out.mrityu),
		ArthaPrahara: float64(out.artha_prahara), YamaGhantaka: float64(out.yama_ghantaka), Dhooma: float64(out.dhooma),
		Vyatipata: float64(out.vyatipata), Parivesha: float64(out.parivesha), IndraChapa: float64(out.indra_chapa), Upaketu: float64(out.upaketu),
	}, st
}

func goDashaPeriod(out C.DhruvDashaPeriod) DashaPeriod {
	return DashaPeriod{
		EntityType:  uint8(out.entity_type),
		EntityIndex: uint8(out.entity_index),
		EntityName:  cString((*C.char)(unsafe.Pointer(out.entity_name))),
		StartJD:     float64(out.start_jd),
		EndJD:       float64(out.end_jd),
		StartUTC:    jdUTCToUTC(float64(out.start_jd)),
		EndUTC:      jdUTCToUTC(float64(out.end_jd)),
		Level:       uint8(out.level),
		Order:       uint16(out.order),
		ParentIdx:   uint32(out.parent_idx),
	}
}

func cDashaPeriod(period DashaPeriod) C.DhruvDashaPeriod {
	return C.DhruvDashaPeriod{
		entity_type:  C.uint8_t(period.EntityType),
		entity_index: C.uint8_t(period.EntityIndex),
		entity_name:  nil,
		start_jd:     C.double(period.StartJD),
		end_jd:       C.double(period.EndJD),
		start_utc:    cUTC(period.StartUTC),
		end_utc:      cUTC(period.EndUTC),
		level:        C.uint8_t(period.Level),
		order:        C.uint16_t(period.Order),
		parent_idx:   C.uint32_t(period.ParentIdx),
	}
}

func goDashaSelectionConfig(v C.DhruvDashaSelectionConfig) DashaSelectionConfig {
	var out DashaSelectionConfig
	out.Count = uint8(v.count)
	out.MaxLevel = uint8(v.max_level)
	out.YoginiScheme = uint8(v.yogini_scheme)
	out.UseAbhijit = v.use_abhijit != 0
	for i := 0; i < MaxDashaSystems; i++ {
		out.Systems[i] = uint8(v.systems[i])
		out.MaxLevels[i] = uint8(v.max_levels[i])
	}
	for i := 0; i < len(out.LevelMethods); i++ {
		out.LevelMethods[i] = uint8(v.level_methods[i])
	}
	if v.snapshot_time.time_kind != C.int32_t(DashaTimeNone) {
		out.SnapshotTime = &DashaSnapshotTime{
			TimeKind: int32(v.snapshot_time.time_kind),
			JDUtc:    float64(v.snapshot_time.jd_utc),
			UTC:      goUTC(v.snapshot_time.utc),
		}
	}
	return out
}

func cDashaSelectionConfig(cfg DashaSelectionConfig) C.DhruvDashaSelectionConfig {
	var out C.DhruvDashaSelectionConfig
	out.count = C.uint8_t(cfg.Count)
	out.max_level = C.uint8_t(cfg.MaxLevel)
	out.yogini_scheme = C.uint8_t(cfg.YoginiScheme)
	out.use_abhijit = boolU8(cfg.UseAbhijit)
	for i := 0; i < MaxDashaSystems; i++ {
		out.systems[i] = C.uint8_t(cfg.Systems[i])
		out.max_levels[i] = C.uint8_t(cfg.MaxLevels[i])
	}
	for i := 0; i < len(cfg.LevelMethods); i++ {
		out.level_methods[i] = C.uint8_t(cfg.LevelMethods[i])
	}
	out.snapshot_time.time_kind = C.int32_t(DashaTimeNone)
	if cfg.SnapshotTime != nil {
		out.snapshot_time.time_kind = C.int32_t(cfg.SnapshotTime.TimeKind)
		out.snapshot_time.jd_utc = C.double(cfg.SnapshotTime.JDUtc)
		out.snapshot_time.utc = cUTC(cfg.SnapshotTime.UTC)
	}
	return out
}

func DashaSelectionConfigDefault() DashaSelectionConfig {
	return goDashaSelectionConfig(C.dhruv_dasha_selection_config_default())
}

func goDashaVariationConfig(v C.DhruvDashaVariationConfig) DashaVariationConfig {
	var out DashaVariationConfig
	out.YoginiScheme = uint8(v.yogini_scheme)
	out.UseAbhijit = v.use_abhijit != 0
	for i := 0; i < len(out.LevelMethods); i++ {
		out.LevelMethods[i] = uint8(v.level_methods[i])
	}
	return out
}

func cDashaVariationConfig(cfg DashaVariationConfig) C.DhruvDashaVariationConfig {
	var out C.DhruvDashaVariationConfig
	out.yogini_scheme = C.uint8_t(cfg.YoginiScheme)
	out.use_abhijit = boolU8(cfg.UseAbhijit)
	for i := 0; i < len(cfg.LevelMethods); i++ {
		out.level_methods[i] = C.uint8_t(cfg.LevelMethods[i])
	}
	return out
}

func DashaVariationConfigDefault() DashaVariationConfig {
	return goDashaVariationConfig(C.dhruv_dasha_variation_config_default())
}

func cRashiDashaInputs(inputs RashiDashaInputs) C.DhruvRashiDashaInputs {
	var out C.DhruvRashiDashaInputs
	for i := 0; i < len(inputs.GrahaSiderealLons); i++ {
		out.graha_sidereal_lons[i] = C.double(inputs.GrahaSiderealLons[i])
	}
	out.lagna_sidereal_lon = C.double(inputs.LagnaSiderealLon)
	return out
}

func cDashaInputs(inputs DashaInputs) C.DhruvDashaInputs {
	var out C.DhruvDashaInputs
	out.has_moon_sid_lon = boolU8(inputs.HasMoonSidLon)
	out.moon_sid_lon = C.double(inputs.MoonSidLon)
	out.has_rashi_inputs = boolU8(inputs.HasRashiInputs)
	out.rashi_inputs = cRashiDashaInputs(inputs.RashiInputs)
	out.has_sunrise_sunset = boolU8(inputs.HasSunriseSet)
	out.sunrise_jd = C.double(inputs.SunriseJD)
	out.sunset_jd = C.double(inputs.SunsetJD)
	return out
}

func cDashaBirthContext(ctx DashaBirthContext) C.DhruvDashaBirthContext {
	return C.DhruvDashaBirthContext{
		time_kind:        C.int32_t(ctx.TimeKind),
		birth_jd:         C.double(ctx.BirthJD),
		birth_utc:        cUTC(ctx.BirthUTC),
		has_location:     boolU8(ctx.HasLocation),
		location:         cGeo(ctx.Location),
		bhava_config:     cBhavaConfig(ctx.BhavaConfig),
		riseset_config:   cRiseSetConfig(ctx.RiseSetConfig),
		sankranti_config: cSankrantiConfig(ctx.SankrantiConfig),
		has_inputs:       boolU8(ctx.HasInputs),
		inputs:           cDashaInputs(ctx.Inputs),
	}
}

func cAmshaSelectionConfig(cfg AmshaSelectionConfig) C.DhruvAmshaSelectionConfig {
	var out C.DhruvAmshaSelectionConfig
	out.count = C.uint8_t(cfg.Count)
	for i := 0; i < len(cfg.Codes); i++ {
		out.codes[i] = C.uint16_t(cfg.Codes[i])
		out.variations[i] = C.uint8_t(cfg.Variations[i])
	}
	return out
}

func FullKundaliConfigDefault() FullKundaliConfig {
	cfg := C.dhruv_full_kundali_config_default()
	return FullKundaliConfig{
		IncludeBhavaCusps:     cfg.include_bhava_cusps != 0,
		IncludeGrahaPositions: cfg.include_graha_positions != 0,
		IncludeBindus:         cfg.include_bindus != 0,
		IncludeDrishti:        cfg.include_drishti != 0,
		IncludeAshtakavarga:   cfg.include_ashtakavarga != 0,
		IncludeUpagrahas:      cfg.include_upagrahas != 0,
		IncludeSphutas:        cfg.include_sphutas != 0,
		IncludeSpecialLagnas:  cfg.include_special_lagnas != 0,
		IncludeAmshas:         cfg.include_amshas != 0,
		IncludeShadbala:       cfg.include_shadbala != 0,
		IncludeBhavaBala:      cfg.include_bhavabala != 0,
		IncludeVimsopaka:      cfg.include_vimsopaka != 0,
		IncludeAvastha:        cfg.include_avastha != 0,
		IncludeCharakaraka:    cfg.include_charakaraka != 0,
		CharakarakaScheme:     uint8(cfg.charakaraka_scheme),
		NodeDignityPolicy:     uint32(cfg.node_dignity_policy),
		UpagrahaConfig:        goTimeUpagrahaConfig(cfg.upagraha_config),
		GrahaPositionsConfig: GrahaPositionsConfig{
			IncludeNakshatra:    cfg.graha_positions_config.include_nakshatra != 0,
			IncludeLagna:        cfg.graha_positions_config.include_lagna != 0,
			IncludeOuterPlanets: cfg.graha_positions_config.include_outer_planets != 0,
			IncludeBhava:        cfg.graha_positions_config.include_bhava != 0,
		},
		BindusConfig: BindusConfig{
			IncludeNakshatra: cfg.bindus_config.include_nakshatra != 0,
			IncludeBhava:     cfg.bindus_config.include_bhava != 0,
			UpagrahaConfig:   goTimeUpagrahaConfig(cfg.bindus_config.upagraha_config),
		},
		DrishtiConfig: DrishtiConfig{
			IncludeBhava:  cfg.drishti_config.include_bhava != 0,
			IncludeLagna:  cfg.drishti_config.include_lagna != 0,
			IncludeBindus: cfg.drishti_config.include_bindus != 0,
		},
		AmshaScope: AmshaChartScope{
			IncludeBhavaCusps:    cfg.amsha_scope.include_bhava_cusps != 0,
			IncludeArudhaPadas:   cfg.amsha_scope.include_arudha_padas != 0,
			IncludeUpagrahas:     cfg.amsha_scope.include_upagrahas != 0,
			IncludeSphutas:       cfg.amsha_scope.include_sphutas != 0,
			IncludeSpecialLagnas: cfg.amsha_scope.include_special_lagnas != 0,
		},
		AmshaSelection:  AmshaSelectionConfig{Count: uint8(cfg.amsha_selection.count)},
		IncludePanchang: cfg.include_panchang != 0,
		IncludeCalendar: cfg.include_calendar != 0,
		IncludeDasha:    cfg.include_dasha != 0,
		DashaConfig:     goDashaSelectionConfig(cfg.dasha_config),
	}
}

func cFullKundaliConfig(cfg FullKundaliConfig) C.DhruvFullKundaliConfig {
	out := C.dhruv_full_kundali_config_default()
	out.include_bhava_cusps = boolU8(cfg.IncludeBhavaCusps)
	out.include_graha_positions = boolU8(cfg.IncludeGrahaPositions)
	out.include_bindus = boolU8(cfg.IncludeBindus)
	out.include_drishti = boolU8(cfg.IncludeDrishti)
	out.include_ashtakavarga = boolU8(cfg.IncludeAshtakavarga)
	out.include_upagrahas = boolU8(cfg.IncludeUpagrahas)
	out.include_sphutas = boolU8(cfg.IncludeSphutas)
	out.include_special_lagnas = boolU8(cfg.IncludeSpecialLagnas)
	out.include_amshas = boolU8(cfg.IncludeAmshas)
	out.include_shadbala = boolU8(cfg.IncludeShadbala)
	out.include_bhavabala = boolU8(cfg.IncludeBhavaBala)
	out.include_vimsopaka = boolU8(cfg.IncludeVimsopaka)
	out.include_avastha = boolU8(cfg.IncludeAvastha)
	out.include_charakaraka = boolU8(cfg.IncludeCharakaraka)
	out.charakaraka_scheme = C.uint8_t(cfg.CharakarakaScheme)
	out.node_dignity_policy = C.uint32_t(cfg.NodeDignityPolicy)
	out.upagraha_config = cTimeUpagrahaConfig(cfg.UpagrahaConfig)
	out.graha_positions_config = C.DhruvGrahaPositionsConfig{
		include_nakshatra:     boolU8(cfg.GrahaPositionsConfig.IncludeNakshatra),
		include_lagna:         boolU8(cfg.GrahaPositionsConfig.IncludeLagna),
		include_outer_planets: boolU8(cfg.GrahaPositionsConfig.IncludeOuterPlanets),
		include_bhava:         boolU8(cfg.GrahaPositionsConfig.IncludeBhava),
	}
	out.bindus_config = C.DhruvBindusConfig{
		include_nakshatra: boolU8(cfg.BindusConfig.IncludeNakshatra),
		include_bhava:     boolU8(cfg.BindusConfig.IncludeBhava),
		upagraha_config:   cTimeUpagrahaConfig(cfg.BindusConfig.UpagrahaConfig),
	}
	out.drishti_config = C.DhruvDrishtiConfig{
		include_bhava:  boolU8(cfg.DrishtiConfig.IncludeBhava),
		include_lagna:  boolU8(cfg.DrishtiConfig.IncludeLagna),
		include_bindus: boolU8(cfg.DrishtiConfig.IncludeBindus),
	}
	out.amsha_scope = cAmshaScope(cfg.AmshaScope)
	out.amsha_selection = cAmshaSelectionConfig(cfg.AmshaSelection)
	out.include_panchang = boolU8(cfg.IncludePanchang)
	out.include_calendar = boolU8(cfg.IncludeCalendar)
	out.include_dasha = boolU8(cfg.IncludeDasha)
	out.dasha_config = cDashaSelectionConfig(cfg.DashaConfig)
	return out
}

func DashaHierarchy(engine EngineHandle, eop EopHandle, request DashaHierarchyRequest) (DashaHierarchyHandle, Status) {
	crequest := C.DhruvDashaHierarchyRequest{
		birth:     cDashaBirthContext(request.Birth),
		system:    C.uint8_t(request.System),
		max_level: C.uint8_t(request.MaxLevel),
		variation: cDashaVariationConfig(request.Variation),
	}
	var out C.DhruvDashaHierarchyHandle
	st := Status(C.dhruv_dasha_hierarchy(engine.ptr, eop.ptr, &crequest, &out))
	return DashaHierarchyHandle{ptr: out}, st
}

func (h *DashaHierarchyHandle) Free() {
	if h == nil || h.ptr == nil {
		return
	}
	C.dhruv_dasha_hierarchy_free(h.ptr)
	h.ptr = nil
}

func (h DashaHierarchyHandle) LevelCount() (uint8, Status) {
	var out C.uint8_t
	st := Status(C.dhruv_dasha_hierarchy_level_count(h.ptr, &out))
	return uint8(out), st
}

func (h DashaHierarchyHandle) PeriodCount(level uint8) (uint32, Status) {
	var out C.uint32_t
	st := Status(C.dhruv_dasha_hierarchy_period_count(h.ptr, C.uint8_t(level), &out))
	return uint32(out), st
}

func (h DashaHierarchyHandle) PeriodAt(level uint8, idx uint32) (DashaPeriod, Status) {
	var out C.DhruvDashaPeriod
	st := Status(C.dhruv_dasha_hierarchy_period_at(h.ptr, C.uint8_t(level), C.uint32_t(idx), &out))
	return goDashaPeriod(out), st
}

func RunDashaSnapshot(engine EngineHandle, eop EopHandle, request DashaSnapshotRequest) (DashaSnapshot, Status) {
	crequest := C.DhruvDashaSnapshotRequest{
		birth:           cDashaBirthContext(request.Birth),
		query_time_kind: C.int32_t(request.QueryTimeKind),
		query_jd:        C.double(request.QueryJD),
		query_utc:       cUTC(request.QueryUTC),
		system:          C.uint8_t(request.System),
		max_level:       C.uint8_t(request.MaxLevel),
		variation:       cDashaVariationConfig(request.Variation),
	}
	var out C.DhruvDashaSnapshot
	st := Status(C.dhruv_dasha_snapshot(engine.ptr, eop.ptr, &crequest, &out))
	res := DashaSnapshot{System: uint8(out.system), QueryJD: float64(out.query_jd), QueryUTC: jdUTCToUTC(float64(out.query_jd)), Count: uint8(out.count)}
	for i := 0; i < len(res.Periods); i++ {
		res.Periods[i] = goDashaPeriod(out.periods[i])
	}
	return res, st
}

func (h *DashaPeriodListHandle) Free() {
	if h == nil || h.ptr == nil {
		return
	}
	C.dhruv_dasha_period_list_free(h.ptr)
	h.ptr = nil
}

func (h DashaPeriodListHandle) Count() (uint32, Status) {
	var out C.uint32_t
	st := Status(C.dhruv_dasha_period_list_count(h.ptr, &out))
	return uint32(out), st
}

func (h DashaPeriodListHandle) At(idx uint32) (DashaPeriod, Status) {
	var out C.DhruvDashaPeriod
	st := Status(C.dhruv_dasha_period_list_at(h.ptr, C.uint32_t(idx), &out))
	return goDashaPeriod(out), st
}

func goDashaPeriodList(handle DashaPeriodListHandle) ([]DashaPeriod, Status) {
	defer handle.Free()
	count, st := handle.Count()
	if st != StatusOK {
		return nil, st
	}
	periods := make([]DashaPeriod, int(count))
	for idx := uint32(0); idx < count; idx++ {
		period, st := handle.At(idx)
		if st != StatusOK {
			return nil, st
		}
		periods[idx] = period
	}
	return periods, StatusOK
}

func DashaLevel0(engine EngineHandle, eop EopHandle, request DashaLevel0Request) ([]DashaPeriod, Status) {
	crequest := C.DhruvDashaLevel0Request{
		birth:  cDashaBirthContext(request.Birth),
		system: C.uint8_t(request.System),
	}
	var handle C.DhruvDashaPeriodListHandle
	st := Status(C.dhruv_dasha_level0(engine.ptr, eop.ptr, &crequest, &handle))
	if st != StatusOK {
		return nil, st
	}
	return goDashaPeriodList(DashaPeriodListHandle{ptr: handle})
}

func DashaLevel0Entity(engine EngineHandle, eop EopHandle, request DashaLevel0EntityRequest) (DashaPeriod, bool, Status) {
	crequest := C.DhruvDashaLevel0EntityRequest{
		birth:        cDashaBirthContext(request.Birth),
		system:       C.uint8_t(request.System),
		entity_type:  C.uint8_t(request.EntityType),
		entity_index: C.uint8_t(request.EntityIndex),
	}
	var found C.uint8_t
	var out C.DhruvDashaPeriod
	st := Status(C.dhruv_dasha_level0_entity(engine.ptr, eop.ptr, &crequest, &found, &out))
	return goDashaPeriod(out), found != 0, st
}

func DashaChildren(engine EngineHandle, eop EopHandle, request DashaChildrenRequest) ([]DashaPeriod, Status) {
	crequest := C.DhruvDashaChildrenRequest{
		birth:     cDashaBirthContext(request.Birth),
		system:    C.uint8_t(request.System),
		variation: cDashaVariationConfig(request.Variation),
		parent:    cDashaPeriod(request.Parent),
	}
	var handle C.DhruvDashaPeriodListHandle
	st := Status(C.dhruv_dasha_children(engine.ptr, eop.ptr, &crequest, &handle))
	if st != StatusOK {
		return nil, st
	}
	return goDashaPeriodList(DashaPeriodListHandle{ptr: handle})
}

func DashaChildPeriod(engine EngineHandle, eop EopHandle, request DashaChildPeriodRequest) (DashaPeriod, bool, Status) {
	crequest := C.DhruvDashaChildPeriodRequest{
		birth:              cDashaBirthContext(request.Birth),
		system:             C.uint8_t(request.System),
		variation:          cDashaVariationConfig(request.Variation),
		parent:             cDashaPeriod(request.Parent),
		child_entity_type:  C.uint8_t(request.ChildEntityType),
		child_entity_index: C.uint8_t(request.ChildEntityIndex),
	}
	var found C.uint8_t
	var out C.DhruvDashaPeriod
	st := Status(C.dhruv_dasha_child_period(engine.ptr, eop.ptr, &crequest, &found, &out))
	return goDashaPeriod(out), found != 0, st
}

func DashaCompleteLevel(engine EngineHandle, eop EopHandle, request DashaCompleteLevelRequest, parentPeriods []DashaPeriod) ([]DashaPeriod, Status) {
	crequest := C.DhruvDashaCompleteLevelRequest{
		birth:       cDashaBirthContext(request.Birth),
		system:      C.uint8_t(request.System),
		variation:   cDashaVariationConfig(request.Variation),
		child_level: C.uint8_t(request.ChildLevel),
	}
	cparents := make([]C.DhruvDashaPeriod, len(parentPeriods))
	for i, period := range parentPeriods {
		cparents[i] = cDashaPeriod(period)
	}
	var parentPtr *C.DhruvDashaPeriod
	if len(cparents) > 0 {
		parentPtr = &cparents[0]
	}
	var handle C.DhruvDashaPeriodListHandle
	st := Status(C.dhruv_dasha_complete_level(engine.ptr, eop.ptr, &crequest, parentPtr, C.uint32_t(len(cparents)), &handle))
	if st != StatusOK {
		return nil, st
	}
	return goDashaPeriodList(DashaPeriodListHandle{ptr: handle})
}

func LoadTaraCatalog(path string) (TaraCatalogHandle, Status) {
	bytes := []byte(path)
	ptr := C.CBytes(bytes)
	defer C.free(ptr)
	var out *C.DhruvTaraCatalogHandle
	st := Status(C.dhruv_tara_catalog_load((*C.uint8_t)(ptr), C.uint32_t(len(bytes)), &out))
	return TaraCatalogHandle{ptr: out}, st
}

func (h *TaraCatalogHandle) Free() {
	if h == nil || h.ptr == nil {
		return
	}
	C.dhruv_tara_catalog_free(h.ptr)
	h.ptr = nil
}

func TaraComputeEx(h TaraCatalogHandle, req TaraComputeRequest) (TaraComputeResult, Status) {
	creq := C.DhruvTaraComputeRequest{
		tara_id:           C.int32_t(req.TaraID),
		output_kind:       C.int32_t(req.OutputKind),
		jd_tdb:            C.double(req.JdTdb),
		ayanamsha_deg:     C.double(req.AyanamshaDeg),
		config:            C.DhruvTaraConfig{accuracy: C.int32_t(req.Config.Accuracy), apply_parallax: boolU8(req.Config.ApplyParallax)},
		earth_state_valid: boolU8(req.EarthStateValid),
	}
	for i := 0; i < 3; i++ {
		creq.earth_state.position_au[i] = C.double(req.EarthState.PositionAUDay[i])
		creq.earth_state.velocity_au_day[i] = C.double(req.EarthState.VelocityAUDay[i])
	}
	var out C.DhruvTaraComputeResult
	st := Status(C.dhruv_tara_compute_ex(h.ptr, &creq, &out))
	var res TaraComputeResult
	res.OutputKind = int32(out.output_kind)
	res.Equatorial = EquatorialPosition{RADeg: float64(out.equatorial.ra_deg), DecDeg: float64(out.equatorial.dec_deg), DistanceAU: float64(out.equatorial.distance_au)}
	res.Ecliptic = SphericalCoords{LonDeg: float64(out.ecliptic.lon_deg), LatDeg: float64(out.ecliptic.lat_deg), DistanceKm: float64(out.ecliptic.distance_km)}
	res.SiderealLongitudeDeg = float64(out.sidereal_longitude_deg)
	return res, st
}

func TaraGalacticCenterEcliptic(h TaraCatalogHandle, jdTdb float64) (SphericalCoords, Status) {
	var out C.DhruvSphericalCoords
	st := Status(C.dhruv_tara_galactic_center_ecliptic(h.ptr, C.double(jdTdb), &out))
	return goSphericalCoords(out), st
}

func TaraPropagatePosition(raDeg, decDeg, parallaxMas, pmRaMasYr, pmDecMasYr, rvKmS, dtYears float64) (EquatorialPosition, Status) {
	var out C.DhruvEquatorialPosition
	st := Status(C.dhruv_tara_propagate_position(
		C.double(raDeg),
		C.double(decDeg),
		C.double(parallaxMas),
		C.double(pmRaMasYr),
		C.double(pmDecMasYr),
		C.double(rvKmS),
		C.double(dtYears),
		&out,
	))
	return EquatorialPosition{RADeg: float64(out.ra_deg), DecDeg: float64(out.dec_deg), DistanceAU: float64(out.distance_au)}, st
}

func TaraApplyAberration(direction [3]float64, earthVelAUDay [3]float64) ([3]float64, Status) {
	var cdir [3]C.double
	var cvel [3]C.double
	for i := 0; i < 3; i++ {
		cdir[i] = C.double(direction[i])
		cvel[i] = C.double(earthVelAUDay[i])
	}
	var out [3]C.double
	st := Status(C.dhruv_tara_apply_aberration(&cdir[0], &cvel[0], &out[0]))
	return [3]float64{float64(out[0]), float64(out[1]), float64(out[2])}, st
}

func TaraApplyLightDeflection(direction [3]float64, sunToObserver [3]float64, sunObserverDistanceAU float64) ([3]float64, Status) {
	var cdir [3]C.double
	var csun [3]C.double
	for i := 0; i < 3; i++ {
		cdir[i] = C.double(direction[i])
		csun[i] = C.double(sunToObserver[i])
	}
	var out [3]C.double
	st := Status(C.dhruv_tara_apply_light_deflection(&cdir[0], &csun[0], C.double(sunObserverDistanceAU), &out[0]))
	return [3]float64{float64(out[0]), float64(out[1]), float64(out[2])}, st
}

func TaraGalacticAnticenterICRS() ([3]float64, Status) {
	var out [3]C.double
	st := Status(C.dhruv_tara_galactic_anticenter_icrs(&out[0]))
	return [3]float64{float64(out[0]), float64(out[1]), float64(out[2])}, st
}

func goBhavaBala(out C.DhruvBhavaBalaResult) BhavaBalaResult {
	var res BhavaBalaResult
	for i := 0; i < 12; i++ {
		e := out.entries[i]
		res.Entries[i] = BhavaBalaEntry{
			BhavaNumber:     uint8(e.bhava_number),
			CuspSiderealLon: float64(e.cusp_sidereal_lon),
			RashiIndex:      uint8(e.rashi_index),
			LordGrahaIndex:  uint8(e.lord_graha_index),
			Bhavadhipati:    float64(e.bhavadhipati),
			Dig:             float64(e.dig),
			Drishti:         float64(e.drishti),
			OccupationBonus: float64(e.occupation_bonus),
			RisingBonus:     float64(e.rising_bonus),
			TotalVirupas:    float64(e.total_virupas),
			TotalRupas:      float64(e.total_rupas),
		}
	}
	return res
}

func goShadbala(out C.DhruvShadbalaResult) ShadbalaResult {
	var res ShadbalaResult
	for i := 0; i < SaptaGrahaCount; i++ {
		e := out.entries[i]
		res.Entries[i] = ShadbalaEntry{
			GrahaIndex: uint8(e.graha_index),
			Sthana: SthanaBalaBreakdown{
				Uchcha:       float64(e.sthana.uchcha),
				Saptavargaja: float64(e.sthana.saptavargaja),
				Ojhayugma:    float64(e.sthana.ojhayugma),
				Kendradi:     float64(e.sthana.kendradi),
				Drekkana:     float64(e.sthana.drekkana),
				Total:        float64(e.sthana.total),
			},
			Dig: float64(e.dig),
			Kala: KalaBalaBreakdown{
				Nathonnatha: float64(e.kala.nathonnatha),
				Paksha:      float64(e.kala.paksha),
				Tribhaga:    float64(e.kala.tribhaga),
				Abda:        float64(e.kala.abda),
				Masa:        float64(e.kala.masa),
				Vara:        float64(e.kala.vara),
				Hora:        float64(e.kala.hora),
				Ayana:       float64(e.kala.ayana),
				Yuddha:      float64(e.kala.yuddha),
				Total:       float64(e.kala.total),
			},
			Cheshta:           float64(e.cheshta),
			Naisargika:        float64(e.naisargika),
			Drik:              float64(e.drik),
			TotalShashtiamsas: float64(e.total_shashtiamsas),
			TotalRupas:        float64(e.total_rupas),
			RequiredStrength:  float64(e.required_strength),
			IsStrong:          e.is_strong != 0,
		}
	}
	return res
}

func goVimsopaka(out C.DhruvVimsopakaResult) VimsopakaResult {
	var res VimsopakaResult
	for i := 0; i < GrahaCount; i++ {
		e := out.entries[i]
		res.Entries[i] = VimsopakaEntry{
			GrahaIndex:   uint8(e.graha_index),
			Shadvarga:    float64(e.shadvarga),
			Saptavarga:   float64(e.saptavarga),
			Dashavarga:   float64(e.dashavarga),
			Shodasavarga: float64(e.shodasavarga),
		}
	}
	return res
}

func ShadbalaForDate(engine EngineHandle, eop EopHandle, utc UtcTime, loc GeoLocation, bhavaCfg BhavaConfig, riseCfg RiseSetConfig, ayanamshaSystem uint32, useNutation bool, amshaSelection AmshaSelectionConfig) (ShadbalaResult, Status) {
	cutc, cloc := cUTC(utc), cGeo(loc)
	cbhava, crise := cBhavaConfig(bhavaCfg), cRiseSetConfig(riseCfg)
	camsha := cAmshaSelectionConfig(amshaSelection)
	var out C.DhruvShadbalaResult
	st := Status(C.dhruv_shadbala_for_date(
		engine.ptr,
		eop.ptr,
		&cutc,
		&cloc,
		&cbhava,
		&crise,
		C.uint32_t(ayanamshaSystem),
		boolU8(useNutation),
		&camsha,
		&out,
	))
	return goShadbala(out), st
}

func CalculateBhavaBala(inputs BhavaBalaInputs) (BhavaBalaResult, Status) {
	cin := C.DhruvBhavaBalaInputs{
		ascendant_sidereal_lon: C.double(inputs.AscendantSiderealLon),
		meridian_sidereal_lon:  C.double(inputs.MeridianSiderealLon),
		birth_period:           C.uint32_t(inputs.BirthPeriod),
	}
	for i := 0; i < 12; i++ {
		cin.cusp_sidereal_lons[i] = C.double(inputs.CuspSiderealLons[i])
		cin.house_lord_strengths[i] = C.double(inputs.HouseLordStrengths[i])
		for j := 0; j < GrahaCount; j++ {
			cin.aspect_virupas[j][i] = C.double(inputs.AspectVirupas[j][i])
		}
	}
	for i := 0; i < GrahaCount; i++ {
		cin.graha_bhava_numbers[i] = C.uint8_t(inputs.GrahaBhavaNumbers[i])
	}
	var out C.DhruvBhavaBalaResult
	st := Status(C.dhruv_calculate_bhavabala(&cin, &out))
	return goBhavaBala(out), st
}

func BhavaBalaForDate(engine EngineHandle, eop EopHandle, utc UtcTime, loc GeoLocation, bhavaCfg BhavaConfig, riseCfg RiseSetConfig, ayanamshaSystem uint32, useNutation bool) (BhavaBalaResult, Status) {
	cutc, cloc := cUTC(utc), cGeo(loc)
	cbhava, crise := cBhavaConfig(bhavaCfg), cRiseSetConfig(riseCfg)
	var out C.DhruvBhavaBalaResult
	st := Status(C.dhruv_bhavabala_for_date(
		engine.ptr,
		eop.ptr,
		&cutc,
		&cloc,
		&cbhava,
		&crise,
		C.uint32_t(ayanamshaSystem),
		boolU8(useNutation),
		&out,
	))
	return goBhavaBala(out), st
}

func VimsopakaForDate(engine EngineHandle, eop EopHandle, utc UtcTime, loc GeoLocation, ayanamshaSystem uint32, useNutation bool, nodeDignityPolicy uint32, amshaSelection AmshaSelectionConfig) (VimsopakaResult, Status) {
	cutc, cloc := cUTC(utc), cGeo(loc)
	camsha := cAmshaSelectionConfig(amshaSelection)
	var out C.DhruvVimsopakaResult
	st := Status(C.dhruv_vimsopaka_for_date(
		engine.ptr,
		eop.ptr,
		&cutc,
		&cloc,
		C.uint32_t(ayanamshaSystem),
		boolU8(useNutation),
		C.uint32_t(nodeDignityPolicy),
		&camsha,
		&out,
	))
	return goVimsopaka(out), st
}

func BalasForDate(engine EngineHandle, eop EopHandle, utc UtcTime, loc GeoLocation, bhavaCfg BhavaConfig, riseCfg RiseSetConfig, ayanamshaSystem uint32, useNutation bool, nodeDignityPolicy uint32, amshaSelection AmshaSelectionConfig) (BalaBundleResult, Status) {
	cutc, cloc := cUTC(utc), cGeo(loc)
	cbhava, crise := cBhavaConfig(bhavaCfg), cRiseSetConfig(riseCfg)
	camsha := cAmshaSelectionConfig(amshaSelection)
	var out C.DhruvBalaBundleResult
	st := Status(C.dhruv_balas_for_date(
		engine.ptr,
		eop.ptr,
		&cutc,
		&cloc,
		&cbhava,
		&crise,
		C.uint32_t(ayanamshaSystem),
		boolU8(useNutation),
		C.uint32_t(nodeDignityPolicy),
		&camsha,
		&out,
	))
	return BalaBundleResult{
		Shadbala:     goShadbala(out.shadbala),
		Vimsopaka:    goVimsopaka(out.vimsopaka),
		Ashtakavarga: goAshtakavarga(out.ashtakavarga),
		BhavaBala:    goBhavaBala(out.bhavabala),
	}, st
}

func AvasthaForDate(engine EngineHandle, eop EopHandle, utc UtcTime, loc GeoLocation, bhavaCfg BhavaConfig, riseCfg RiseSetConfig, ayanamshaSystem uint32, useNutation bool, nodeDignityPolicy uint32, amshaSelection AmshaSelectionConfig) (AllGrahaAvasthas, Status) {
	cutc, cloc := cUTC(utc), cGeo(loc)
	cbhava, crise := cBhavaConfig(bhavaCfg), cRiseSetConfig(riseCfg)
	camsha := cAmshaSelectionConfig(amshaSelection)
	var out C.DhruvAllGrahaAvasthas
	st := Status(C.dhruv_avastha_for_date(
		engine.ptr,
		eop.ptr,
		&cutc,
		&cloc,
		&cbhava,
		&crise,
		C.uint32_t(ayanamshaSystem),
		boolU8(useNutation),
		C.uint32_t(nodeDignityPolicy),
		&camsha,
		&out,
	))
	var res AllGrahaAvasthas
	for i := 0; i < GrahaCount; i++ {
		e := out.entries[i]
		var sub [5]uint8
		for j := 0; j < 5; j++ {
			sub[j] = uint8(e.sayanadi.sub_states[j])
		}
		res.Entries[i] = GrahaAvasthas{
			Baladi:    uint8(e.baladi),
			Jagradadi: uint8(e.jagradadi),
			Deeptadi:  uint8(e.deeptadi),
			Lajjitadi: uint8(e.lajjitadi),
			Sayanadi: SayanadiResult{
				Avastha:   uint8(e.sayanadi.avastha),
				SubStates: sub,
			},
		}
	}
	return res, st
}

func CharakarakaForDate(engine EngineHandle, eop EopHandle, utc UtcTime, ayanamshaSystem uint32, useNutation bool, scheme uint8) (CharakarakaResult, Status) {
	cutc := cUTC(utc)
	var out C.DhruvCharakarakaResult
	st := Status(C.dhruv_charakaraka_for_date(
		engine.ptr,
		eop.ptr,
		&cutc,
		C.uint32_t(ayanamshaSystem),
		boolU8(useNutation),
		C.uint8_t(scheme),
		&out,
	))
	var res CharakarakaResult
	res.Scheme = uint8(out.scheme)
	res.UsedEightKarakas = out.used_eight_karakas != 0
	res.Count = uint8(out.count)
	for i := 0; i < MaxCharakarakaEntries; i++ {
		e := out.entries[i]
		res.Entries[i] = CharakarakaEntry{
			RoleCode:                uint8(e.role_code),
			GrahaIndex:              uint8(e.graha_index),
			Rank:                    uint8(e.rank),
			LongitudeDeg:            float64(e.longitude_deg),
			DegreesInRashi:          float64(e.degrees_in_rashi),
			EffectiveDegreesInRashi: float64(e.effective_degrees_in_rashi),
		}
	}
	return res, st
}

func FullKundaliForDateSummary(engine EngineHandle, eop EopHandle, utc UtcTime, loc GeoLocation, bhavaCfg BhavaConfig, riseCfg RiseSetConfig, ayanamshaSystem uint32, useNutation bool) (FullKundaliSummary, Status) {
	cutc, cloc := cUTC(utc), cGeo(loc)
	cbhava, crise := cBhavaConfig(bhavaCfg), cRiseSetConfig(riseCfg)
	cfg := C.dhruv_full_kundali_config_default()
	var out C.DhruvFullKundaliResult
	st := Status(C.dhruv_full_kundali_for_date(
		engine.ptr,
		eop.ptr,
		&cutc,
		&cloc,
		&cbhava,
		&crise,
		C.uint32_t(ayanamshaSystem),
		boolU8(useNutation),
		&cfg,
		&out,
	))
	if st != StatusOK {
		return FullKundaliSummary{}, st
	}
	defer C.dhruv_full_kundali_result_free(&out)
	return FullKundaliSummary{
		AyanamshaDeg:        float64(out.ayanamsha_deg),
		BhavaCuspsValid:     out.bhava_cusps_valid != 0,
		GrahaPositionsValid: out.graha_positions_valid != 0,
		BindusValid:         out.bindus_valid != 0,
		DrishtiValid:        out.drishti_valid != 0,
		AshtakavargaValid:   out.ashtakavarga_valid != 0,
		UpagrahasValid:      out.upagrahas_valid != 0,
		SpecialLagnasValid:  out.special_lagnas_valid != 0,
		AmshasValid:         out.amshas_valid != 0,
		AmshasCount:         uint8(out.amshas_count),
		ShadbalaValid:       out.shadbala_valid != 0,
		BhavaBalaValid:      out.bhavabala_valid != 0,
		VimsopakaValid:      out.vimsopaka_valid != 0,
		AvasthaValid:        out.avastha_valid != 0,
		CharakarakaValid:    out.charakaraka_valid != 0,
		PanchangValid:       out.panchang_valid != 0,
		DashaCount:          uint8(out.dasha_count),
		DashaSnapshotCount:  uint8(out.dasha_snapshot_count),
	}, st
}

func goFullPanchangInfo(v C.DhruvPanchangInfo) FullPanchangInfo {
	out := FullPanchangInfo{
		Tithi: TithiInfo{
			TithiIndex:    int32(v.tithi.tithi_index),
			Paksha:        int32(v.tithi.paksha),
			TithiInPaksha: int32(v.tithi.tithi_in_paksha),
			Start:         goUTC(v.tithi.start),
			End:           goUTC(v.tithi.end),
		},
		Karana: KaranaInfo{
			KaranaIndex:     int32(v.karana.karana_index),
			KaranaNameIndex: int32(v.karana.karana_name_index),
			Start:           goUTC(v.karana.start),
			End:             goUTC(v.karana.end),
		},
		Yoga: YogaInfo{
			YogaIndex: int32(v.yoga.yoga_index),
			Start:     goUTC(v.yoga.start),
			End:       goUTC(v.yoga.end),
		},
		Vaar: VaarInfo{
			VaarIndex: int32(v.vaar.vaar_index),
			Start:     goUTC(v.vaar.start),
			End:       goUTC(v.vaar.end),
		},
		Hora: HoraInfo{
			HoraIndex:    int32(v.hora.hora_index),
			HoraPosition: int32(v.hora.hora_position),
			Start:        goUTC(v.hora.start),
			End:          goUTC(v.hora.end),
		},
		Ghatika: GhatikaInfo{
			Value: int32(v.ghatika.value),
			Start: goUTC(v.ghatika.start),
			End:   goUTC(v.ghatika.end),
		},
		Nakshatra: PanchangNakshatraInfo{
			NakshatraIndex: int32(v.nakshatra.nakshatra_index),
			Pada:           int32(v.nakshatra.pada),
			Start:          goUTC(v.nakshatra.start),
			End:            goUTC(v.nakshatra.end),
		},
		CalendarValid: v.calendar_valid != 0,
	}
	if v.calendar_valid != 0 {
		out.Masa = &MasaInfo{
			MasaIndex: int32(v.masa.masa_index),
			Adhika:    v.masa.adhika != 0,
			Start:     goUTC(v.masa.start),
			End:       goUTC(v.masa.end),
		}
		out.Ayana = &AyanaInfo{
			Ayana: int32(v.ayana.ayana),
			Start: goUTC(v.ayana.start),
			End:   goUTC(v.ayana.end),
		}
		out.Varsha = &VarshaInfo{
			SamvatsaraIndex: int32(v.varsha.samvatsara_index),
			Order:           int32(v.varsha.order),
			Start:           goUTC(v.varsha.start),
			End:             goUTC(v.varsha.end),
		}
	}
	return out
}

func goFullKundaliDashaHierarchy(handle C.DhruvDashaHierarchyHandle, system uint8) (FullKundaliDashaHierarchy, Status) {
	var levelCount C.uint8_t
	st := Status(C.dhruv_dasha_hierarchy_level_count(handle, &levelCount))
	if st != StatusOK {
		return FullKundaliDashaHierarchy{}, st
	}
	out := FullKundaliDashaHierarchy{System: system, Levels: make([]FullKundaliDashaLevel, int(levelCount))}
	for lvl := 0; lvl < int(levelCount); lvl++ {
		var periodCount C.uint32_t
		st = Status(C.dhruv_dasha_hierarchy_period_count(handle, C.uint8_t(lvl), &periodCount))
		if st != StatusOK {
			return FullKundaliDashaHierarchy{}, st
		}
		level := FullKundaliDashaLevel{Level: uint8(lvl), Periods: make([]DashaPeriod, int(periodCount))}
		for idx := 0; idx < int(periodCount); idx++ {
			var period C.DhruvDashaPeriod
			st = Status(C.dhruv_dasha_hierarchy_period_at(handle, C.uint8_t(lvl), C.uint32_t(idx), &period))
			if st != StatusOK {
				return FullKundaliDashaHierarchy{}, st
			}
			level.Periods[idx] = DashaPeriod{
				EntityType:  uint8(period.entity_type),
				EntityIndex: uint8(period.entity_index),
				EntityName:  cString((*C.char)(unsafe.Pointer(period.entity_name))),
				StartJD:     float64(period.start_jd),
				EndJD:       float64(period.end_jd),
				Level:       uint8(period.level),
				Order:       uint16(period.order),
				ParentIdx:   uint32(period.parent_idx),
			}
		}
		out.Levels[lvl] = level
	}
	return out, StatusOK
}

func FullKundaliForDate(engine EngineHandle, eop EopHandle, utc UtcTime, loc GeoLocation, bhavaCfg BhavaConfig, riseCfg RiseSetConfig, ayanamshaSystem uint32, useNutation bool, cfg FullKundaliConfig) (FullKundaliResult, Status) {
	cutc, cloc := cUTC(utc), cGeo(loc)
	cbhava, crise := cBhavaConfig(bhavaCfg), cRiseSetConfig(riseCfg)
	ccfg := cFullKundaliConfig(cfg)
	var out C.DhruvFullKundaliResult
	st := Status(C.dhruv_full_kundali_for_date(
		engine.ptr,
		eop.ptr,
		&cutc,
		&cloc,
		&cbhava,
		&crise,
		C.uint32_t(ayanamshaSystem),
		boolU8(useNutation),
		&ccfg,
		&out,
	))
	if st != StatusOK {
		return FullKundaliResult{}, st
	}
	defer C.dhruv_full_kundali_result_free(&out)

	res := FullKundaliResult{AyanamshaDeg: float64(out.ayanamsha_deg)}
	if out.bhava_cusps_valid != 0 {
		v := goBhavaResult(out.bhava_cusps)
		res.BhavaCusps = &v
	}
	if out.rashi_bhava_cusps_valid != 0 {
		v := goBhavaResult(out.rashi_bhava_cusps)
		res.RashiBhavaCusps = &v
	}
	if out.graha_positions_valid != 0 {
		v := GrahaPositions{Lagna: goGrahaEntry(out.graha_positions.lagna)}
		for i := 0; i < GrahaCount; i++ {
			v.Grahas[i] = goGrahaEntry(out.graha_positions.grahas[i])
		}
		for i := 0; i < len(v.OuterPlanets); i++ {
			v.OuterPlanets[i] = goGrahaEntry(out.graha_positions.outer_planets[i])
		}
		res.GrahaPositions = &v
	}
	if out.bindus_valid != 0 {
		v := BindusResult{
			BhriguBindu:    goGrahaEntry(out.bindus.bhrigu_bindu),
			PranapadaLagna: goGrahaEntry(out.bindus.pranapada_lagna),
			Gulika:         goGrahaEntry(out.bindus.gulika),
			Maandi:         goGrahaEntry(out.bindus.maandi),
			HoraLagna:      goGrahaEntry(out.bindus.hora_lagna),
			GhatiLagna:     goGrahaEntry(out.bindus.ghati_lagna),
			SreeLagna:      goGrahaEntry(out.bindus.sree_lagna),
		}
		for i := 0; i < 12; i++ {
			v.ArudhaPadas[i] = goGrahaEntry(out.bindus.arudha_padas[i])
		}
		if out.bindus.rashi_bhava_arudha_padas_valid != 0 {
			v.RashiBhavaArudhaPadasValid = true
			for i := 0; i < 12; i++ {
				v.RashiBhavaArudhaPadas[i] = goGrahaEntry(out.bindus.rashi_bhava_arudha_padas[i])
			}
		}
		res.Bindus = &v
	}
	if out.drishti_valid != 0 {
		var v DrishtiResult
		for i := 0; i < GrahaCount; i++ {
			v.GrahaToLagna[i] = goDrishtiEntry(out.drishti.graha_to_lagna[i])
			for j := 0; j < GrahaCount; j++ {
				v.GrahaToGraha[i][j] = goDrishtiEntry(out.drishti.graha_to_graha[i][j])
			}
			for j := 0; j < 12; j++ {
				v.GrahaToBhava[i][j] = goDrishtiEntry(out.drishti.graha_to_bhava[i][j])
				v.GrahaToRashiBhava[i][j] = goDrishtiEntry(out.drishti.graha_to_rashi_bhava[i][j])
			}
			for j := 0; j < 19; j++ {
				v.GrahaToBindus[i][j] = goDrishtiEntry(out.drishti.graha_to_bindus[i][j])
			}
		}
		res.Drishti = &v
	}
	if out.ashtakavarga_valid != 0 {
		v := goAshtakavarga(out.ashtakavarga)
		res.Ashtakavarga = &v
	}
	if out.upagrahas_valid != 0 {
		v := AllUpagrahas{
			Gulika: float64(out.upagrahas.gulika), Maandi: float64(out.upagrahas.maandi), Kaala: float64(out.upagrahas.kaala), Mrityu: float64(out.upagrahas.mrityu),
			ArthaPrahara: float64(out.upagrahas.artha_prahara), YamaGhantaka: float64(out.upagrahas.yama_ghantaka), Dhooma: float64(out.upagrahas.dhooma),
			Vyatipata: float64(out.upagrahas.vyatipata), Parivesha: float64(out.upagrahas.parivesha), IndraChapa: float64(out.upagrahas.indra_chapa), Upaketu: float64(out.upagrahas.upaketu),
		}
		res.Upagrahas = &v
	}
	if out.sphutas_valid != 0 {
		var v SphutalResult
		for i := 0; i < SphutaCount; i++ {
			v.Longitudes[i] = float64(out.sphutas.longitudes[i])
		}
		res.Sphutas = &v
	}
	if out.special_lagnas_valid != 0 {
		v := SpecialLagnas{
			BhavaLagna: float64(out.special_lagnas.bhava_lagna), HoraLagna: float64(out.special_lagnas.hora_lagna),
			GhatiLagna: float64(out.special_lagnas.ghati_lagna), VighatiLagna: float64(out.special_lagnas.vighati_lagna),
			VarnadaLagna: float64(out.special_lagnas.varnada_lagna), SreeLagna: float64(out.special_lagnas.sree_lagna),
			PranapadaLagna: float64(out.special_lagnas.pranapada_lagna), InduLagna: float64(out.special_lagnas.indu_lagna),
		}
		res.SpecialLagnas = &v
	}
	if out.amshas_valid != 0 && out.amshas_count > 0 {
		res.Amshas = make([]AmshaChart, int(out.amshas_count))
		for i := 0; i < int(out.amshas_count); i++ {
			chart := AmshaChart{
				AmshaCode:                  uint16(out.amshas[i].amsha_code),
				VariationCode:              uint8(out.amshas[i].variation_code),
				Lagna:                      goAmshaEntry(out.amshas[i].lagna),
				BhavaCuspsValid:            out.amshas[i].bhava_cusps_valid != 0,
				RashiBhavaCuspsValid:       out.amshas[i].rashi_bhava_cusps_valid != 0,
				ArudhaPadasValid:           out.amshas[i].arudha_padas_valid != 0,
				RashiBhavaArudhaPadasValid: out.amshas[i].rashi_bhava_arudha_padas_valid != 0,
				UpagrahasValid:             out.amshas[i].upagrahas_valid != 0,
				SphutasValid:               out.amshas[i].sphutas_valid != 0,
				SpecialLagnasValid:         out.amshas[i].special_lagnas_valid != 0,
			}
			for j := 0; j < GrahaCount; j++ {
				chart.Grahas[j] = goAmshaEntry(out.amshas[i].grahas[j])
			}
			if chart.BhavaCuspsValid {
				chart.BhavaCusps = goAmshaEntries(out.amshas[i].bhava_cusps[:])
			}
			if chart.RashiBhavaCuspsValid {
				chart.RashiBhavaCusps = goAmshaEntries(out.amshas[i].rashi_bhava_cusps[:])
			}
			if chart.ArudhaPadasValid {
				chart.ArudhaPadas = goAmshaEntries(out.amshas[i].arudha_padas[:])
			}
			if chart.RashiBhavaArudhaPadasValid {
				chart.RashiBhavaArudhaPadas = goAmshaEntries(out.amshas[i].rashi_bhava_arudha_padas[:])
			}
			if chart.UpagrahasValid {
				chart.Upagrahas = goAmshaEntries(out.amshas[i].upagrahas[:])
			}
			if chart.SphutasValid {
				chart.Sphutas = goAmshaEntries(out.amshas[i].sphutas[:])
			}
			if chart.SpecialLagnasValid {
				chart.SpecialLagnas = goAmshaEntries(out.amshas[i].special_lagnas[:])
			}
			res.Amshas[i] = chart
		}
	}
	if out.shadbala_valid != 0 {
		v := goShadbala(out.shadbala)
		res.Shadbala = &v
	}
	if out.bhavabala_valid != 0 {
		v := goBhavaBala(out.bhavabala)
		res.BhavaBala = &v
	}
	if out.vimsopaka_valid != 0 {
		v := goVimsopaka(out.vimsopaka)
		res.Vimsopaka = &v
	}
	if out.avastha_valid != 0 {
		var v AllGrahaAvasthas
		for i := 0; i < GrahaCount; i++ {
			e := out.avastha.entries[i]
			v.Entries[i] = GrahaAvasthas{
				Baladi: uint8(e.baladi), Jagradadi: uint8(e.jagradadi), Deeptadi: uint8(e.deeptadi), Lajjitadi: uint8(e.lajjitadi),
				Sayanadi: SayanadiResult{Avastha: uint8(e.sayanadi.avastha), SubStates: [5]uint8{uint8(e.sayanadi.sub_states[0]), uint8(e.sayanadi.sub_states[1]), uint8(e.sayanadi.sub_states[2]), uint8(e.sayanadi.sub_states[3]), uint8(e.sayanadi.sub_states[4])}},
			}
		}
		res.Avastha = &v
	}
	if out.charakaraka_valid != 0 {
		v := CharakarakaResult{Scheme: uint8(out.charakaraka.scheme), UsedEightKarakas: out.charakaraka.used_eight_karakas != 0, Count: uint8(out.charakaraka.count)}
		for i := 0; i < MaxCharakarakaEntries; i++ {
			e := out.charakaraka.entries[i]
			v.Entries[i] = CharakarakaEntry{
				RoleCode: uint8(e.role_code), GrahaIndex: uint8(e.graha_index), Rank: uint8(e.rank),
				LongitudeDeg: float64(e.longitude_deg), DegreesInRashi: float64(e.degrees_in_rashi), EffectiveDegreesInRashi: float64(e.effective_degrees_in_rashi),
			}
		}
		res.Charakaraka = &v
	}
	if out.panchang_valid != 0 {
		v := goFullPanchangInfo(out.panchang)
		res.Panchang = &v
	}
	if out.dasha_count > 0 {
		res.Dasha = make([]FullKundaliDashaHierarchy, int(out.dasha_count))
		for i := 0; i < int(out.dasha_count); i++ {
			hierarchy, dst := goFullKundaliDashaHierarchy(out.dasha_handles[i], uint8(out.dasha_systems[i]))
			if dst != StatusOK {
				return FullKundaliResult{}, dst
			}
			res.Dasha[i] = hierarchy
		}
	}
	if out.dasha_snapshot_count > 0 {
		res.DashaSnapshots = make([]DashaSnapshot, int(out.dasha_snapshot_count))
		for i := 0; i < int(out.dasha_snapshot_count); i++ {
			s := out.dasha_snapshots[i]
			snap := DashaSnapshot{System: uint8(s.system), QueryJD: float64(s.query_jd), QueryUTC: jdUTCToUTC(float64(s.query_jd)), Count: uint8(s.count)}
			for j := 0; j < len(snap.Periods); j++ {
				p := s.periods[j]
				snap.Periods[j] = DashaPeriod{
					EntityType: uint8(p.entity_type), EntityIndex: uint8(p.entity_index), EntityName: cString((*C.char)(unsafe.Pointer(p.entity_name))), StartJD: float64(p.start_jd),
					EndJD: float64(p.end_jd), StartUTC: jdUTCToUTC(float64(p.start_jd)), EndUTC: jdUTCToUTC(float64(p.end_jd)), Level: uint8(p.level), Order: uint16(p.order), ParentIdx: uint32(p.parent_idx),
				}
			}
			res.DashaSnapshots[i] = snap
		}
	}
	return res, StatusOK
}
