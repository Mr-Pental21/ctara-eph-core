package dhruv

import "ctara-dhruv-core/bindings/go-open/internal/cabi"

func (e *Engine) GrahaLongitudes(jdTdb float64, cfg GrahaLongitudesConfig) (GrahaLongitudes, error) {
	out, st := cabi.ComputeGrahaLongitudes(e.h, jdTdb, cfg)
	return out, statusErr("graha_longitudes", st)
}

func (e *Engine) SpecialLagnasForDate(ep *EOP, utc UtcTime, loc GeoLocation, riseset RiseSetConfig, ayanamshaSystem uint32, useNutation bool) (SpecialLagnas, error) {
	out, st := cabi.SpecialLagnasForDate(e.h, ep.h, utc, loc, riseset, ayanamshaSystem, useNutation)
	return out, statusErr("special_lagnas_for_date", st)
}

func (e *Engine) ArudhaPadasForDate(ep *EOP, utc UtcTime, loc GeoLocation, ayanamshaSystem uint32, useNutation bool) ([12]ArudhaResult, error) {
	out, st := cabi.ArudhaPadasForDate(e.h, ep.h, utc, loc, ayanamshaSystem, useNutation)
	return out, statusErr("arudha_padas_for_date", st)
}

func (e *Engine) AllUpagrahasForDate(ep *EOP, utc UtcTime, loc GeoLocation, ayanamshaSystem uint32, useNutation bool) (AllUpagrahas, error) {
	out, st := cabi.AllUpagrahasForDate(e.h, ep.h, utc, loc, ayanamshaSystem, useNutation)
	return out, statusErr("all_upagrahas_for_date", st)
}

func (e *Engine) AllUpagrahasForDateWithConfig(ep *EOP, utc UtcTime, loc GeoLocation, ayanamshaSystem uint32, useNutation bool, cfg TimeUpagrahaConfig) (AllUpagrahas, error) {
	out, st := cabi.AllUpagrahasForDateWithConfig(e.h, ep.h, utc, loc, ayanamshaSystem, useNutation, cfg)
	return out, statusErr("all_upagrahas_for_date_with_config", st)
}

func (e *Engine) CharakarakaForDate(ep *EOP, utc UtcTime, ayanamshaSystem uint32, useNutation bool, scheme uint8) (CharakarakaResult, error) {
	out, st := cabi.CharakarakaForDate(e.h, ep.h, utc, ayanamshaSystem, useNutation, scheme)
	return out, statusErr("charakaraka_for_date", st)
}
