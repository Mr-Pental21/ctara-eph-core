package dhruv

import "ctara-dhruv-core/bindings/go-open/internal/cabi"

func RiseSetConfigDefault() RiseSetConfig { return cabi.RiseSetConfigDefault() }
func BhavaConfigDefault() BhavaConfig     { return cabi.BhavaConfigDefault() }

func BhavaSystemCount() uint32 { return cabi.BhavaSystemCount() }

func (e *Engine) ComputeRiseSet(ep *EOP, loc GeoLocation, cfg RiseSetConfig, eventCode int32, jdTdbApprox float64, lsk *LSK) (RiseSetResult, error) {
	out, st := cabi.ComputeRiseSet(e.h, ep.h, loc, cfg, eventCode, jdTdbApprox, lsk.h)
	return out, statusErr("compute_rise_set", st)
}

func (e *Engine) ComputeAllEvents(ep *EOP, loc GeoLocation, cfg RiseSetConfig, jdTdbApprox float64, lsk *LSK) ([8]RiseSetResult, error) {
	out, st := cabi.ComputeAllEvents(e.h, ep.h, loc, cfg, jdTdbApprox, lsk.h)
	return out, statusErr("compute_all_events", st)
}

func (e *Engine) ComputeRiseSetUTC(ep *EOP, lsk *LSK, loc GeoLocation, eventCode int32, utc UtcTime, cfg RiseSetConfig) (RiseSetResultUTC, error) {
	out, st := cabi.ComputeRiseSetUTC(e.h, ep.h, lsk.h, loc, eventCode, utc, cfg)
	return out, statusErr("compute_rise_set_utc", st)
}

func (e *Engine) ComputeAllEventsUTC(ep *EOP, lsk *LSK, loc GeoLocation, utc UtcTime, cfg RiseSetConfig) ([8]RiseSetResultUTC, error) {
	out, st := cabi.ComputeAllEventsUTC(e.h, ep.h, lsk.h, loc, utc, cfg)
	return out, statusErr("compute_all_events_utc", st)
}

func (e *Engine) ComputeBhavas(ep *EOP, loc GeoLocation, lsk *LSK, jdTdb float64, cfg BhavaConfig) (BhavaResult, error) {
	out, st := cabi.ComputeBhavas(e.h, ep.h, loc, lsk.h, jdTdb, cfg)
	return out, statusErr("compute_bhavas", st)
}

func (e *Engine) ComputeBhavasUTC(ep *EOP, lsk *LSK, loc GeoLocation, utc UtcTime, cfg BhavaConfig) (BhavaResult, error) {
	out, st := cabi.ComputeBhavasUTC(e.h, ep.h, lsk.h, loc, utc, cfg)
	return out, statusErr("compute_bhavas_utc", st)
}

func (e *Engine) LagnaDeg(lsk *LSK, ep *EOP, loc GeoLocation, jdTdb float64) (float64, error) {
	out, st := cabi.LagnaDeg(lsk.h, ep.h, loc, jdTdb)
	return out, statusErr("lagna_deg", st)
}

func (e *Engine) LagnaDegWithConfig(lsk *LSK, ep *EOP, loc GeoLocation, jdTdb float64, cfg BhavaConfig) (float64, error) {
	out, st := cabi.LagnaDegWithConfig(lsk.h, ep.h, loc, jdTdb, cfg)
	return out, statusErr("lagna_deg_with_config", st)
}

func (e *Engine) MCDeg(lsk *LSK, ep *EOP, loc GeoLocation, jdTdb float64) (float64, error) {
	out, st := cabi.MCDeg(lsk.h, ep.h, loc, jdTdb)
	return out, statusErr("mc_deg", st)
}

func (e *Engine) MCDegWithConfig(lsk *LSK, ep *EOP, loc GeoLocation, jdTdb float64, cfg BhavaConfig) (float64, error) {
	out, st := cabi.MCDegWithConfig(lsk.h, ep.h, loc, jdTdb, cfg)
	return out, statusErr("mc_deg_with_config", st)
}

func (e *Engine) RAMCDeg(lsk *LSK, ep *EOP, loc GeoLocation, jdTdb float64) (float64, error) {
	out, st := cabi.RAMCDeg(lsk.h, ep.h, loc, jdTdb)
	return out, statusErr("ramc_deg", st)
}

func (e *Engine) LagnaDegUTC(lsk *LSK, ep *EOP, loc GeoLocation, utc UtcTime) (float64, error) {
	out, st := cabi.LagnaDegUTC(lsk.h, ep.h, loc, utc)
	return out, statusErr("lagna_deg_utc", st)
}

func (e *Engine) LagnaDegUTCWithConfig(lsk *LSK, ep *EOP, loc GeoLocation, utc UtcTime, cfg BhavaConfig) (float64, error) {
	out, st := cabi.LagnaDegUTCWithConfig(lsk.h, ep.h, loc, utc, cfg)
	return out, statusErr("lagna_deg_utc_with_config", st)
}

func (e *Engine) MCDegUTC(lsk *LSK, ep *EOP, loc GeoLocation, utc UtcTime) (float64, error) {
	out, st := cabi.MCDegUTC(lsk.h, ep.h, loc, utc)
	return out, statusErr("mc_deg_utc", st)
}

func (e *Engine) MCDegUTCWithConfig(lsk *LSK, ep *EOP, loc GeoLocation, utc UtcTime, cfg BhavaConfig) (float64, error) {
	out, st := cabi.MCDegUTCWithConfig(lsk.h, ep.h, loc, utc, cfg)
	return out, statusErr("mc_deg_utc_with_config", st)
}

func (e *Engine) RAMCDegUTC(lsk *LSK, ep *EOP, loc GeoLocation, utc UtcTime) (float64, error) {
	out, st := cabi.RAMCDegUTC(lsk.h, ep.h, loc, utc)
	return out, statusErr("ramc_deg_utc", st)
}

func (e *Engine) TithiForDate(utc UtcTime) (TithiInfo, error) {
	out, st := cabi.TithiForDate(e.h, utc)
	return out, statusErr("tithi_for_date", st)
}

func (e *Engine) KaranaForDate(utc UtcTime) (KaranaInfo, error) {
	out, st := cabi.KaranaForDate(e.h, utc)
	return out, statusErr("karana_for_date", st)
}

func (e *Engine) YogaForDate(utc UtcTime, cfg SankrantiConfig) (YogaInfo, error) {
	out, st := cabi.YogaForDate(e.h, utc, cfg)
	return out, statusErr("yoga_for_date", st)
}

func (e *Engine) NakshatraForDate(utc UtcTime, cfg SankrantiConfig) (PanchangNakshatraInfo, error) {
	out, st := cabi.NakshatraForDate(e.h, utc, cfg)
	return out, statusErr("nakshatra_for_date", st)
}

func (e *Engine) VaarForDate(ep *EOP, utc UtcTime, loc GeoLocation, cfg RiseSetConfig) (VaarInfo, error) {
	out, st := cabi.VaarForDate(e.h, ep.h, utc, loc, cfg)
	return out, statusErr("vaar_for_date", st)
}

func (e *Engine) HoraForDate(ep *EOP, utc UtcTime, loc GeoLocation, cfg RiseSetConfig) (HoraInfo, error) {
	out, st := cabi.HoraForDate(e.h, ep.h, utc, loc, cfg)
	return out, statusErr("hora_for_date", st)
}

func (e *Engine) GhatikaForDate(ep *EOP, utc UtcTime, loc GeoLocation, cfg RiseSetConfig) (GhatikaInfo, error) {
	out, st := cabi.GhatikaForDate(e.h, ep.h, utc, loc, cfg)
	return out, statusErr("ghatika_for_date", st)
}

func (e *Engine) MasaForDate(utc UtcTime, cfg SankrantiConfig) (MasaInfo, error) {
	out, st := cabi.MasaForDate(e.h, utc, cfg)
	return out, statusErr("masa_for_date", st)
}

func (e *Engine) AyanaForDate(utc UtcTime, cfg SankrantiConfig) (AyanaInfo, error) {
	out, st := cabi.AyanaForDate(e.h, utc, cfg)
	return out, statusErr("ayana_for_date", st)
}

func (e *Engine) VarshaForDate(utc UtcTime, cfg SankrantiConfig) (VarshaInfo, error) {
	out, st := cabi.VarshaForDate(e.h, utc, cfg)
	return out, statusErr("varsha_for_date", st)
}

func (e *Engine) PanchangComputeEx(ep *EOP, lsk *LSK, req PanchangComputeRequest) (PanchangOperationResult, error) {
	out, st := cabi.PanchangComputeEx(e.h, ep.h, lsk.h, req)
	return out, statusErr("panchang_compute_ex", st)
}
