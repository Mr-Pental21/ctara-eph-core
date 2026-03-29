package dhruv

import "ctara-dhruv-core/bindings/go-open/internal/cabi"

func UTCToTdbJD(lsk *LSK, eop *EOP, req UtcToTdbRequest) (UtcToTdbResult, error) {
	var eopHandle cabi.EopHandle
	if eop != nil {
		eopHandle = eop.h
	}
	out, st := cabi.UTCToTdbJD(lsk.h, eopHandle, req)
	return out, statusErr("utc_to_tdb_jd", st)
}

func JdTdbToUTC(lsk *LSK, jdTdb float64) (UtcTime, error) {
	out, st := cabi.JdTdbToUTC(lsk.h, jdTdb)
	return out, statusErr("jd_tdb_to_utc", st)
}

func NutationIau2000b(jdTdb float64) (float64, float64, error) {
	dpsi, deps, st := cabi.NutationIau2000b(jdTdb)
	return dpsi, deps, statusErr("nutation_iau2000b", st)
}

func NutationIau2000bUTC(lsk *LSK, utc UtcTime) (float64, float64, error) {
	dpsi, deps, st := cabi.NutationIau2000bUTC(lsk.h, utc)
	return dpsi, deps, statusErr("nutation_iau2000b_utc", st)
}

func ApproximateLocalNoonJD(jdUTMidnight, longitudeDeg float64) float64 {
	return cabi.ApproximateLocalNoonJD(jdUTMidnight, longitudeDeg)
}

func AyanamshaSystemCount() uint32 { return cabi.AyanamshaSystemCount() }

func ReferencePlaneDefault(systemCode int32) int32 {
	return cabi.ReferencePlaneDefault(systemCode)
}

func AyanamshaComputeEx(lsk *LSK, eop *EOP, req AyanamshaComputeRequest) (float64, error) {
	out, st := cabi.AyanamshaComputeEx(lsk.h, eop.h, req)
	return out, statusErr("ayanamsha_compute_ex", st)
}

func LunarNodeCount() uint32 { return cabi.LunarNodeCount() }

func LunarNodeDeg(nodeCode, modeCode int32, jdTdb float64) (float64, error) {
	out, st := cabi.LunarNodeDeg(nodeCode, modeCode, jdTdb)
	return out, statusErr("lunar_node_deg", st)
}

func (e *Engine) LunarNodeDegWithEngine(nodeCode, modeCode int32, jdTdb float64) (float64, error) {
	out, st := cabi.LunarNodeDegWithEngine(e.h, nodeCode, modeCode, jdTdb)
	return out, statusErr("lunar_node_deg_with_engine", st)
}

func LunarNodeDegUTC(lsk *LSK, nodeCode, modeCode int32, utc UtcTime) (float64, error) {
	out, st := cabi.LunarNodeDegUTC(lsk.h, nodeCode, modeCode, utc)
	return out, statusErr("lunar_node_deg_utc", st)
}

func (e *Engine) LunarNodeDegUTCWithEngine(lsk *LSK, nodeCode, modeCode int32, utc UtcTime) (float64, error) {
	out, st := cabi.LunarNodeDegUTCWithEngine(e.h, lsk.h, nodeCode, modeCode, utc)
	return out, statusErr("lunar_node_deg_utc_with_engine", st)
}

func LunarNodeComputeEx(lsk *LSK, eop *EOP, req LunarNodeRequest) (float64, error) {
	out, st := cabi.LunarNodeComputeEx(lsk.h, eop.h, req)
	return out, statusErr("lunar_node_compute_ex", st)
}
