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

func goRiseSetConfig(cfg C.DhruvRiseSetConfig) RiseSetConfig {
	return RiseSetConfig{
		UseRefraction:      cfg.use_refraction != 0,
		SunLimb:            int32(cfg.sun_limb),
		AltitudeCorrection: cfg.altitude_correction != 0,
	}
}

func cBhavaConfig(cfg BhavaConfig) C.DhruvBhavaConfig {
	return C.DhruvBhavaConfig{
		system:           C.int32_t(cfg.System),
		starting_point:   C.int32_t(cfg.StartingPoint),
		custom_start_deg: C.double(cfg.CustomStartDeg),
		reference_mode:   C.int32_t(cfg.ReferenceMode),
		output_mode:      C.int32_t(cfg.OutputMode),
		ayanamsha_system: C.int32_t(cfg.AyanamshaSystem),
		use_nutation:     boolU8(cfg.UseNutation),
		reference_plane:  C.int32_t(cfg.ReferencePlane),
	}
}

func goBhavaConfig(cfg C.DhruvBhavaConfig) BhavaConfig {
	return BhavaConfig{
		System:          int32(cfg.system),
		StartingPoint:   int32(cfg.starting_point),
		CustomStartDeg:  float64(cfg.custom_start_deg),
		ReferenceMode:   int32(cfg.reference_mode),
		OutputMode:      int32(cfg.output_mode),
		AyanamshaSystem: int32(cfg.ayanamsha_system),
		UseNutation:     cfg.use_nutation != 0,
		ReferencePlane:  int32(cfg.reference_plane),
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

func QueryUTC(h EngineHandle, target, observer, frame int32, utc UtcTime) (SphericalState, Status) {
	cutc := cUTC(utc)
	var out C.DhruvSphericalState
	st := Status(C.dhruv_query_utc(h.ptr, C.int32_t(target), C.int32_t(observer), C.int32_t(frame), &cutc, &out))
	return goSphericalState(out), st
}

func QueryUTCSpherical(h EngineHandle, target, observer, frame int32, utc UtcTime) (SphericalState, Status) {
	var out C.DhruvSphericalState
	st := Status(C.dhruv_query_utc_spherical(
		h.ptr,
		C.int32_t(target),
		C.int32_t(observer),
		C.int32_t(frame),
		C.int32_t(utc.Year),
		C.uint32_t(utc.Month),
		C.uint32_t(utc.Day),
		C.uint32_t(utc.Hour),
		C.uint32_t(utc.Minute),
		C.double(utc.Second),
		&out,
	))
	return goSphericalState(out), st
}

func CartesianToSpherical(position [3]float64) (SphericalCoords, Status) {
	cpos := [3]C.double{C.double(position[0]), C.double(position[1]), C.double(position[2])}
	var out C.DhruvSphericalCoords
	st := Status(C.dhruv_cartesian_to_spherical(&cpos[0], &out))
	return goSphericalCoords(out), st
}

func LoadConfig(path string) (ConfigHandle, Status) {
	cPath := C.CBytes([]byte(path))
	defer C.free(cPath)
	var out *C.DhruvConfigHandle
	st := Status(C.dhruv_config_load((*C.uint8_t)(cPath), C.uint32_t(len(path)), &out))
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

func UTCToTdbJD(lsk LskHandle, utc UtcTime) (float64, Status) {
	var out C.double
	st := Status(C.dhruv_utc_to_tdb_jd(
		lsk.ptr,
		C.int32_t(utc.Year),
		C.uint32_t(utc.Month),
		C.uint32_t(utc.Day),
		C.uint32_t(utc.Hour),
		C.uint32_t(utc.Minute),
		C.double(utc.Second),
		&out,
	))
	return float64(out), st
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
	creq := C.DhruvConjunctionSearchRequest{
		body1_code:   C.int32_t(req.Body1Code),
		body2_code:   C.int32_t(req.Body2Code),
		query_mode:   C.int32_t(req.QueryMode),
		at_jd_tdb:    C.double(req.AtJdTdb),
		start_jd_tdb: C.double(req.StartJdTdb),
		end_jd_tdb:   C.double(req.EndJdTdb),
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
	creq := C.DhruvGrahanSearchRequest{
		grahan_kind:  C.int32_t(req.GrahanKind),
		query_mode:   C.int32_t(req.QueryMode),
		at_jd_tdb:    C.double(req.AtJdTdb),
		start_jd_tdb: C.double(req.StartJdTdb),
		end_jd_tdb:   C.double(req.EndJdTdb),
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
			GreatestGrahanJd:     float64(v.greatest_grahan_jd),
			P1Jd:                 float64(v.p1_jd),
			U1Jd:                 float64(v.u1_jd),
			U2Jd:                 float64(v.u2_jd),
			U3Jd:                 float64(v.u3_jd),
			U4Jd:                 float64(v.u4_jd),
			P4Jd:                 float64(v.p4_jd),
			MoonEclipticLatDeg:   float64(v.moon_ecliptic_lat_deg),
			AngularSeparationDeg: float64(v.angular_separation_deg),
		}
	}
	toS := func(v C.DhruvSuryaGrahanResult) SuryaGrahanResult {
		return SuryaGrahanResult{
			GrahanType:           int32(v.grahan_type),
			Magnitude:            float64(v.magnitude),
			GreatestGrahanJd:     float64(v.greatest_grahan_jd),
			C1Jd:                 float64(v.c1_jd),
			C2Jd:                 float64(v.c2_jd),
			C3Jd:                 float64(v.c3_jd),
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
	creq := C.DhruvMotionSearchRequest{
		body_code:    C.int32_t(req.BodyCode),
		motion_kind:  C.int32_t(req.MotionKind),
		query_mode:   C.int32_t(req.QueryMode),
		at_jd_tdb:    C.double(req.AtJdTdb),
		start_jd_tdb: C.double(req.StartJdTdb),
		end_jd_tdb:   C.double(req.EndJdTdb),
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
		return StationaryEvent{JdTdb: float64(v.jd_tdb), BodyCode: int32(v.body_code), LongitudeDeg: float64(v.longitude_deg), LatitudeDeg: float64(v.latitude_deg), StationType: int32(v.station_type)}
	}
	convMs := func(v C.DhruvMaxSpeedEvent) MaxSpeedEvent {
		return MaxSpeedEvent{JdTdb: float64(v.jd_tdb), BodyCode: int32(v.body_code), LongitudeDeg: float64(v.longitude_deg), LatitudeDeg: float64(v.latitude_deg), SpeedDegPerDay: float64(v.speed_deg_per_day), SpeedType: int32(v.speed_type)}
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
	creq := C.DhruvLunarPhaseSearchRequest{
		phase_kind:   C.int32_t(req.PhaseKind),
		query_mode:   C.int32_t(req.QueryMode),
		at_jd_tdb:    C.double(req.AtJdTdb),
		start_jd_tdb: C.double(req.StartJdTdb),
		end_jd_tdb:   C.double(req.EndJdTdb),
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
	creq := C.DhruvSankrantiSearchRequest{
		target_kind:  C.int32_t(req.TargetKind),
		query_mode:   C.int32_t(req.QueryMode),
		rashi_index:  C.int32_t(req.RashiIndex),
		at_jd_tdb:    C.double(req.AtJdTdb),
		start_jd_tdb: C.double(req.StartJdTdb),
		end_jd_tdb:   C.double(req.EndJdTdb),
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

func GrahaSiderealLongitudes(engine EngineHandle, jdTdb float64, ayanamshaSystem uint32, useNutation bool) (GrahaLongitudes, Status) {
	var out C.DhruvGrahaLongitudes
	st := Status(C.dhruv_graha_sidereal_longitudes(engine.ptr, C.double(jdTdb), C.uint32_t(ayanamshaSystem), boolU8(useNutation), &out))
	var goOut GrahaLongitudes
	for i := 0; i < GrahaCount; i++ {
		goOut.Longitudes[i] = float64(out.longitudes[i])
	}
	return goOut, st
}

func GrahaTropicalLongitudes(engine EngineHandle, jdTdb float64) (GrahaLongitudes, Status) {
	var out C.DhruvGrahaLongitudes
	st := Status(C.dhruv_graha_tropical_longitudes(engine.ptr, C.double(jdTdb), &out))
	var goOut GrahaLongitudes
	for i := 0; i < GrahaCount; i++ {
		goOut.Longitudes[i] = float64(out.longitudes[i])
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

func goDashaPeriod(out C.DhruvDashaPeriod) DashaPeriod {
	return DashaPeriod{
		EntityType:  uint8(out.entity_type),
		EntityIndex: uint8(out.entity_index),
		StartJD:     float64(out.start_jd),
		EndJD:       float64(out.end_jd),
		Level:       uint8(out.level),
		Order:       uint16(out.order),
		ParentIdx:   uint32(out.parent_idx),
	}
}

func cDashaPeriod(period DashaPeriod) C.DhruvDashaPeriod {
	return C.DhruvDashaPeriod{
		entity_type:  C.uint8_t(period.EntityType),
		entity_index: C.uint8_t(period.EntityIndex),
		start_jd:     C.double(period.StartJD),
		end_jd:       C.double(period.EndJD),
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
	out.HasSnapshotJd = v.has_snapshot_jd != 0
	out.SnapshotJd = float64(v.snapshot_jd)
	for i := 0; i < MaxDashaSystems; i++ {
		out.Systems[i] = uint8(v.systems[i])
		out.MaxLevels[i] = uint8(v.max_levels[i])
	}
	for i := 0; i < len(out.LevelMethods); i++ {
		out.LevelMethods[i] = uint8(v.level_methods[i])
	}
	return out
}

func cDashaSelectionConfig(cfg DashaSelectionConfig) C.DhruvDashaSelectionConfig {
	var out C.DhruvDashaSelectionConfig
	out.count = C.uint8_t(cfg.Count)
	out.max_level = C.uint8_t(cfg.MaxLevel)
	out.yogini_scheme = C.uint8_t(cfg.YoginiScheme)
	out.use_abhijit = boolU8(cfg.UseAbhijit)
	out.has_snapshot_jd = boolU8(cfg.HasSnapshotJd)
	out.snapshot_jd = C.double(cfg.SnapshotJd)
	for i := 0; i < MaxDashaSystems; i++ {
		out.systems[i] = C.uint8_t(cfg.Systems[i])
		out.max_levels[i] = C.uint8_t(cfg.MaxLevels[i])
	}
	for i := 0; i < len(cfg.LevelMethods); i++ {
		out.level_methods[i] = C.uint8_t(cfg.LevelMethods[i])
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
		GrahaPositionsConfig: GrahaPositionsConfig{
			IncludeNakshatra:    cfg.graha_positions_config.include_nakshatra != 0,
			IncludeLagna:        cfg.graha_positions_config.include_lagna != 0,
			IncludeOuterPlanets: cfg.graha_positions_config.include_outer_planets != 0,
			IncludeBhava:        cfg.graha_positions_config.include_bhava != 0,
		},
		BindusConfig: BindusConfig{
			IncludeNakshatra: cfg.bindus_config.include_nakshatra != 0,
			IncludeBhava:     cfg.bindus_config.include_bhava != 0,
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
	out.graha_positions_config = C.DhruvGrahaPositionsConfig{
		include_nakshatra:     boolU8(cfg.GrahaPositionsConfig.IncludeNakshatra),
		include_lagna:         boolU8(cfg.GrahaPositionsConfig.IncludeLagna),
		include_outer_planets: boolU8(cfg.GrahaPositionsConfig.IncludeOuterPlanets),
		include_bhava:         boolU8(cfg.GrahaPositionsConfig.IncludeBhava),
	}
	out.bindus_config = C.DhruvBindusConfig{
		include_nakshatra: boolU8(cfg.BindusConfig.IncludeNakshatra),
		include_bhava:     boolU8(cfg.BindusConfig.IncludeBhava),
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

func DashaHierarchyUTC(engine EngineHandle, eop EopHandle, birthUTC UtcTime, loc GeoLocation, bhavaCfg BhavaConfig, riseCfg RiseSetConfig, ayanamshaSystem uint32, useNutation bool, system uint8, maxLevel uint8) (DashaHierarchyHandle, Status) {
	cbirth, cloc, cbhava, crise := cUTC(birthUTC), cGeo(loc), cBhavaConfig(bhavaCfg), cRiseSetConfig(riseCfg)
	var out C.DhruvDashaHierarchyHandle
	st := Status(C.dhruv_dasha_hierarchy_utc(engine.ptr, eop.ptr, &cbirth, &cloc, &cbhava, &crise, C.uint32_t(ayanamshaSystem), boolU8(useNutation), C.uint8_t(system), C.uint8_t(maxLevel), &out))
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

func DashaSnapshotUTC(engine EngineHandle, eop EopHandle, birthUTC, queryUTC UtcTime, loc GeoLocation, bhavaCfg BhavaConfig, riseCfg RiseSetConfig, ayanamshaSystem uint32, useNutation bool, system uint8, maxLevel uint8) (DashaSnapshot, Status) {
	cbirth, cquery, cloc := cUTC(birthUTC), cUTC(queryUTC), cGeo(loc)
	cbhava, crise := cBhavaConfig(bhavaCfg), cRiseSetConfig(riseCfg)
	var out C.DhruvDashaSnapshot
	st := Status(C.dhruv_dasha_snapshot_utc(engine.ptr, eop.ptr, &cbirth, &cquery, &cloc, &cbhava, &crise, C.uint32_t(ayanamshaSystem), boolU8(useNutation), C.uint8_t(system), C.uint8_t(maxLevel), &out))
	res := DashaSnapshot{System: uint8(out.system), QueryJD: float64(out.query_jd), Count: uint8(out.count)}
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

func DashaLevel0UTC(engine EngineHandle, eop EopHandle, birthUTC UtcTime, loc GeoLocation, bhavaCfg BhavaConfig, riseCfg RiseSetConfig, ayanamshaSystem uint32, useNutation bool, system uint8) ([]DashaPeriod, Status) {
	cbirth, cloc, cbhava, crise := cUTC(birthUTC), cGeo(loc), cBhavaConfig(bhavaCfg), cRiseSetConfig(riseCfg)
	var handle C.DhruvDashaPeriodListHandle
	st := Status(C.dhruv_dasha_level0_utc(engine.ptr, eop.ptr, &cbirth, &cloc, &cbhava, &crise, C.uint32_t(ayanamshaSystem), boolU8(useNutation), C.uint8_t(system), &handle))
	if st != StatusOK {
		return nil, st
	}
	return goDashaPeriodList(DashaPeriodListHandle{ptr: handle})
}

func DashaLevel0EntityUTC(engine EngineHandle, eop EopHandle, birthUTC UtcTime, loc GeoLocation, bhavaCfg BhavaConfig, riseCfg RiseSetConfig, ayanamshaSystem uint32, useNutation bool, system, entityType, entityIndex uint8) (DashaPeriod, bool, Status) {
	cbirth, cloc, cbhava, crise := cUTC(birthUTC), cGeo(loc), cBhavaConfig(bhavaCfg), cRiseSetConfig(riseCfg)
	var found C.uint8_t
	var out C.DhruvDashaPeriod
	st := Status(C.dhruv_dasha_level0_entity_utc(engine.ptr, eop.ptr, &cbirth, &cloc, &cbhava, &crise, C.uint32_t(ayanamshaSystem), boolU8(useNutation), C.uint8_t(system), C.uint8_t(entityType), C.uint8_t(entityIndex), &found, &out))
	return goDashaPeriod(out), found != 0, st
}

func DashaChildrenUTC(engine EngineHandle, eop EopHandle, birthUTC UtcTime, loc GeoLocation, bhavaCfg BhavaConfig, riseCfg RiseSetConfig, ayanamshaSystem uint32, useNutation bool, system uint8, variationCfg DashaVariationConfig, parent DashaPeriod) ([]DashaPeriod, Status) {
	cbirth, cloc, cbhava, crise := cUTC(birthUTC), cGeo(loc), cBhavaConfig(bhavaCfg), cRiseSetConfig(riseCfg)
	cvariation, cparent := cDashaVariationConfig(variationCfg), cDashaPeriod(parent)
	var handle C.DhruvDashaPeriodListHandle
	st := Status(C.dhruv_dasha_children_utc(engine.ptr, eop.ptr, &cbirth, &cloc, &cbhava, &crise, C.uint32_t(ayanamshaSystem), boolU8(useNutation), C.uint8_t(system), &cvariation, &cparent, &handle))
	if st != StatusOK {
		return nil, st
	}
	return goDashaPeriodList(DashaPeriodListHandle{ptr: handle})
}

func DashaChildPeriodUTC(engine EngineHandle, eop EopHandle, birthUTC UtcTime, loc GeoLocation, bhavaCfg BhavaConfig, riseCfg RiseSetConfig, ayanamshaSystem uint32, useNutation bool, system uint8, variationCfg DashaVariationConfig, parent DashaPeriod, childEntityType, childEntityIndex uint8) (DashaPeriod, bool, Status) {
	cbirth, cloc, cbhava, crise := cUTC(birthUTC), cGeo(loc), cBhavaConfig(bhavaCfg), cRiseSetConfig(riseCfg)
	cvariation, cparent := cDashaVariationConfig(variationCfg), cDashaPeriod(parent)
	var found C.uint8_t
	var out C.DhruvDashaPeriod
	st := Status(C.dhruv_dasha_child_period_utc(engine.ptr, eop.ptr, &cbirth, &cloc, &cbhava, &crise, C.uint32_t(ayanamshaSystem), boolU8(useNutation), C.uint8_t(system), &cvariation, &cparent, C.uint8_t(childEntityType), C.uint8_t(childEntityIndex), &found, &out))
	return goDashaPeriod(out), found != 0, st
}

func DashaCompleteLevelUTC(engine EngineHandle, eop EopHandle, birthUTC UtcTime, loc GeoLocation, bhavaCfg BhavaConfig, riseCfg RiseSetConfig, ayanamshaSystem uint32, useNutation bool, system uint8, variationCfg DashaVariationConfig, parentPeriods []DashaPeriod, childLevel uint8) ([]DashaPeriod, Status) {
	cbirth, cloc, cbhava, crise := cUTC(birthUTC), cGeo(loc), cBhavaConfig(bhavaCfg), cRiseSetConfig(riseCfg)
	cvariation := cDashaVariationConfig(variationCfg)
	cparents := make([]C.DhruvDashaPeriod, len(parentPeriods))
	for i, period := range parentPeriods {
		cparents[i] = cDashaPeriod(period)
	}
	var parentPtr *C.DhruvDashaPeriod
	if len(cparents) > 0 {
		parentPtr = &cparents[0]
	}
	var handle C.DhruvDashaPeriodListHandle
	st := Status(C.dhruv_dasha_complete_level_utc(engine.ptr, eop.ptr, &cbirth, &cloc, &cbhava, &crise, C.uint32_t(ayanamshaSystem), boolU8(useNutation), C.uint8_t(system), &cvariation, parentPtr, C.uint32_t(len(cparents)), C.uint8_t(childLevel), &handle))
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

func ShadbalaForDate(engine EngineHandle, eop EopHandle, utc UtcTime, loc GeoLocation, bhavaCfg BhavaConfig, riseCfg RiseSetConfig, ayanamshaSystem uint32, useNutation bool) (ShadbalaResult, Status) {
	cutc, cloc := cUTC(utc), cGeo(loc)
	cbhava, crise := cBhavaConfig(bhavaCfg), cRiseSetConfig(riseCfg)
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

func VimsopakaForDate(engine EngineHandle, eop EopHandle, utc UtcTime, loc GeoLocation, ayanamshaSystem uint32, useNutation bool, nodeDignityPolicy uint32) (VimsopakaResult, Status) {
	cutc, cloc := cUTC(utc), cGeo(loc)
	var out C.DhruvVimsopakaResult
	st := Status(C.dhruv_vimsopaka_for_date(
		engine.ptr,
		eop.ptr,
		&cutc,
		&cloc,
		C.uint32_t(ayanamshaSystem),
		boolU8(useNutation),
		C.uint32_t(nodeDignityPolicy),
		&out,
	))
	return goVimsopaka(out), st
}

func BalasForDate(engine EngineHandle, eop EopHandle, utc UtcTime, loc GeoLocation, bhavaCfg BhavaConfig, riseCfg RiseSetConfig, ayanamshaSystem uint32, useNutation bool, nodeDignityPolicy uint32) (BalaBundleResult, Status) {
	cutc, cloc := cUTC(utc), cGeo(loc)
	cbhava, crise := cBhavaConfig(bhavaCfg), cRiseSetConfig(riseCfg)
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
		&out,
	))
	return BalaBundleResult{
		Shadbala:     goShadbala(out.shadbala),
		Vimsopaka:    goVimsopaka(out.vimsopaka),
		Ashtakavarga: goAshtakavarga(out.ashtakavarga),
		BhavaBala:    goBhavaBala(out.bhavabala),
	}, st
}

func AvasthaForDate(engine EngineHandle, eop EopHandle, utc UtcTime, loc GeoLocation, bhavaCfg BhavaConfig, riseCfg RiseSetConfig, ayanamshaSystem uint32, useNutation bool, nodeDignityPolicy uint32) (AllGrahaAvasthas, Status) {
	cutc, cloc := cUTC(utc), cGeo(loc)
	cbhava, crise := cBhavaConfig(bhavaCfg), cRiseSetConfig(riseCfg)
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
				AmshaCode:          uint16(out.amshas[i].amsha_code),
				VariationCode:      uint8(out.amshas[i].variation_code),
				Lagna:              goAmshaEntry(out.amshas[i].lagna),
				BhavaCuspsValid:    out.amshas[i].bhava_cusps_valid != 0,
				ArudhaPadasValid:   out.amshas[i].arudha_padas_valid != 0,
				UpagrahasValid:     out.amshas[i].upagrahas_valid != 0,
				SphutasValid:       out.amshas[i].sphutas_valid != 0,
				SpecialLagnasValid: out.amshas[i].special_lagnas_valid != 0,
			}
			for j := 0; j < GrahaCount; j++ {
				chart.Grahas[j] = goAmshaEntry(out.amshas[i].grahas[j])
			}
			if chart.BhavaCuspsValid {
				chart.BhavaCusps = goAmshaEntries(out.amshas[i].bhava_cusps[:])
			}
			if chart.ArudhaPadasValid {
				chart.ArudhaPadas = goAmshaEntries(out.amshas[i].arudha_padas[:])
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
			snap := DashaSnapshot{System: uint8(s.system), QueryJD: float64(s.query_jd), Count: uint8(s.count)}
			for j := 0; j < len(snap.Periods); j++ {
				p := s.periods[j]
				snap.Periods[j] = DashaPeriod{
					EntityType: uint8(p.entity_type), EntityIndex: uint8(p.entity_index), StartJD: float64(p.start_jd),
					EndJD: float64(p.end_jd), Level: uint8(p.level), Order: uint16(p.order), ParentIdx: uint32(p.parent_idx),
				}
			}
			res.DashaSnapshots[i] = snap
		}
	}
	return res, StatusOK
}
