package dhruv

import "ctara-dhruv-core/bindings/go-open/internal/cabi"

func DashaSelectionConfigDefault() DashaSelectionConfig { return cabi.DashaSelectionConfigDefault() }
func FullKundaliConfigDefault() FullKundaliConfig       { return cabi.FullKundaliConfigDefault() }

func (e *Engine) ShadbalaForDate(ep *EOP, utc UtcTime, loc GeoLocation, bhavaCfg BhavaConfig, riseCfg RiseSetConfig, ayanamshaSystem uint32, useNutation bool, amshaSelection AmshaSelectionConfig) (ShadbalaResult, error) {
	out, st := cabi.ShadbalaForDate(e.h, ep.h, utc, loc, bhavaCfg, riseCfg, ayanamshaSystem, useNutation, amshaSelection)
	return out, statusErr("shadbala_for_date", st)
}

func CalculateBhavaBala(inputs BhavaBalaInputs) (BhavaBalaResult, error) {
	out, st := cabi.CalculateBhavaBala(inputs)
	return out, statusErr("calculate_bhavabala", st)
}

func (e *Engine) BhavaBalaForDate(ep *EOP, utc UtcTime, loc GeoLocation, bhavaCfg BhavaConfig, riseCfg RiseSetConfig, ayanamshaSystem uint32, useNutation bool) (BhavaBalaResult, error) {
	out, st := cabi.BhavaBalaForDate(e.h, ep.h, utc, loc, bhavaCfg, riseCfg, ayanamshaSystem, useNutation)
	return out, statusErr("bhavabala_for_date", st)
}

func (e *Engine) VimsopakaForDate(ep *EOP, utc UtcTime, loc GeoLocation, ayanamshaSystem uint32, useNutation bool, nodeDignityPolicy uint32, amshaSelection AmshaSelectionConfig) (VimsopakaResult, error) {
	out, st := cabi.VimsopakaForDate(e.h, ep.h, utc, loc, ayanamshaSystem, useNutation, nodeDignityPolicy, amshaSelection)
	return out, statusErr("vimsopaka_for_date", st)
}

func (e *Engine) BalasForDate(ep *EOP, utc UtcTime, loc GeoLocation, bhavaCfg BhavaConfig, riseCfg RiseSetConfig, ayanamshaSystem uint32, useNutation bool, nodeDignityPolicy uint32, amshaSelection AmshaSelectionConfig) (BalaBundleResult, error) {
	out, st := cabi.BalasForDate(e.h, ep.h, utc, loc, bhavaCfg, riseCfg, ayanamshaSystem, useNutation, nodeDignityPolicy, amshaSelection)
	return out, statusErr("balas_for_date", st)
}

func (e *Engine) AvasthaForDate(ep *EOP, utc UtcTime, loc GeoLocation, bhavaCfg BhavaConfig, riseCfg RiseSetConfig, ayanamshaSystem uint32, useNutation bool, nodeDignityPolicy uint32, amshaSelection AmshaSelectionConfig) (AllGrahaAvasthas, error) {
	out, st := cabi.AvasthaForDate(e.h, ep.h, utc, loc, bhavaCfg, riseCfg, ayanamshaSystem, useNutation, nodeDignityPolicy, amshaSelection)
	return out, statusErr("avastha_for_date", st)
}

func (e *Engine) FullKundaliForDateSummary(ep *EOP, utc UtcTime, loc GeoLocation, bhavaCfg BhavaConfig, riseCfg RiseSetConfig, ayanamshaSystem uint32, useNutation bool) (FullKundaliSummary, error) {
	out, st := cabi.FullKundaliForDateSummary(e.h, ep.h, utc, loc, bhavaCfg, riseCfg, ayanamshaSystem, useNutation)
	return out, statusErr("full_kundali_for_date", st)
}

func (e *Engine) FullKundaliForDate(ep *EOP, utc UtcTime, loc GeoLocation, bhavaCfg BhavaConfig, riseCfg RiseSetConfig, ayanamshaSystem uint32, useNutation bool, cfg FullKundaliConfig) (FullKundaliResult, error) {
	out, st := cabi.FullKundaliForDate(e.h, ep.h, utc, loc, bhavaCfg, riseCfg, ayanamshaSystem, useNutation, cfg)
	return out, statusErr("full_kundali_for_date", st)
}
