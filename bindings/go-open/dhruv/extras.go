package dhruv

import "ctara-dhruv-core/bindings/go-open/internal/cabi"

func DegToDms(degrees float64) (Dms, error) {
	out, st := cabi.DegToDms(degrees)
	return out, statusErr("deg_to_dms", st)
}

func RashiFromLongitude(siderealLon float64) (RashiInfo, error) {
	out, st := cabi.RashiFromLongitude(siderealLon)
	return out, statusErr("rashi_from_longitude", st)
}

func NakshatraFromLongitude(siderealLon float64) (NakshatraInfo, error) {
	out, st := cabi.NakshatraFromLongitude(siderealLon)
	return out, statusErr("nakshatra_from_longitude", st)
}

func Nakshatra28FromLongitude(siderealLon float64) (Nakshatra28Info, error) {
	out, st := cabi.Nakshatra28FromLongitude(siderealLon)
	return out, statusErr("nakshatra28_from_longitude", st)
}

func RashiFromTropical(tropicalLon float64, ayanamshaSystem uint32, jdTdb float64, useNutation bool) (RashiInfo, error) {
	out, st := cabi.RashiFromTropical(tropicalLon, ayanamshaSystem, jdTdb, useNutation)
	return out, statusErr("rashi_from_tropical", st)
}

func NakshatraFromTropical(tropicalLon float64, ayanamshaSystem uint32, jdTdb float64, useNutation bool) (NakshatraInfo, error) {
	out, st := cabi.NakshatraFromTropical(tropicalLon, ayanamshaSystem, jdTdb, useNutation)
	return out, statusErr("nakshatra_from_tropical", st)
}

func Nakshatra28FromTropical(tropicalLon float64, ayanamshaSystem uint32, jdTdb float64, useNutation bool) (Nakshatra28Info, error) {
	out, st := cabi.Nakshatra28FromTropical(tropicalLon, ayanamshaSystem, jdTdb, useNutation)
	return out, statusErr("nakshatra28_from_tropical", st)
}

func RashiFromTropicalUTC(lsk *LSK, tropicalLon float64, ayanamshaSystem uint32, utc UtcTime, useNutation bool) (RashiInfo, error) {
	out, st := cabi.RashiFromTropicalUTC(lsk.h, tropicalLon, ayanamshaSystem, utc, useNutation)
	return out, statusErr("rashi_from_tropical_utc", st)
}

func NakshatraFromTropicalUTC(lsk *LSK, tropicalLon float64, ayanamshaSystem uint32, utc UtcTime, useNutation bool) (NakshatraInfo, error) {
	out, st := cabi.NakshatraFromTropicalUTC(lsk.h, tropicalLon, ayanamshaSystem, utc, useNutation)
	return out, statusErr("nakshatra_from_tropical_utc", st)
}

func Nakshatra28FromTropicalUTC(lsk *LSK, tropicalLon float64, ayanamshaSystem uint32, utc UtcTime, useNutation bool) (Nakshatra28Info, error) {
	out, st := cabi.Nakshatra28FromTropicalUTC(lsk.h, tropicalLon, ayanamshaSystem, utc, useNutation)
	return out, statusErr("nakshatra28_from_tropical_utc", st)
}

func RashiCount() uint32                      { return cabi.RashiCount() }
func NakshatraCount(schemeCode uint32) uint32 { return cabi.NakshatraCount(schemeCode) }
func RashiName(index uint32) string           { return cabi.RashiName(index) }
func NakshatraName(index uint32) string       { return cabi.NakshatraName(index) }
func Nakshatra28Name(index uint32) string     { return cabi.Nakshatra28Name(index) }
func MasaName(index uint32) string            { return cabi.MasaName(index) }
func AyanaName(index uint32) string           { return cabi.AyanaName(index) }
func SamvatsaraName(index uint32) string      { return cabi.SamvatsaraName(index) }
func TithiName(index uint32) string           { return cabi.TithiName(index) }
func KaranaName(index uint32) string          { return cabi.KaranaName(index) }
func YogaName(index uint32) string            { return cabi.YogaName(index) }
func VaarName(index uint32) string            { return cabi.VaarName(index) }
func HoraName(index uint32) string            { return cabi.HoraName(index) }
func GrahaName(index uint32) string           { return cabi.GrahaName(index) }
func YoginiName(index uint32) string          { return cabi.YoginiName(index) }
func SphutaName(index uint32) string          { return cabi.SphutaName(index) }
func SpecialLagnaName(index uint32) string    { return cabi.SpecialLagnaName(index) }
func ArudhaPadaName(index uint32) string      { return cabi.ArudhaPadaName(index) }
func UpagrahaName(index uint32) string        { return cabi.UpagrahaName(index) }

func TithiFromElongation(elongation float64) (TithiPosition, error) {
	out, st := cabi.TithiFromElongation(elongation)
	return out, statusErr("tithi_from_elongation", st)
}

func KaranaFromElongation(elongation float64) (KaranaPosition, error) {
	out, st := cabi.KaranaFromElongation(elongation)
	return out, statusErr("karana_from_elongation", st)
}

func YogaFromSum(sum float64) (YogaPosition, error) {
	out, st := cabi.YogaFromSum(sum)
	return out, statusErr("yoga_from_sum", st)
}

func VaarFromJD(jd float64) int32                  { return cabi.VaarFromJD(jd) }
func MasaFromRashiIndex(rashi uint32) int32        { return cabi.MasaFromRashiIndex(rashi) }
func AyanaFromSiderealLongitude(lon float64) int32 { return cabi.AyanaFromSiderealLongitude(lon) }
func NthRashiFrom(rashi, offset uint32) int32      { return cabi.NthRashiFrom(rashi, offset) }
func RashiLord(rashi uint32) int32                 { return cabi.RashiLord(rashi) }
func HoraAt(vaarIndex, horaIndex uint32) int32     { return cabi.HoraAt(vaarIndex, horaIndex) }

func SamvatsaraFromYear(year int32) (SamvatsaraResult, error) {
	out, st := cabi.SamvatsaraFromYear(year)
	return out, statusErr("samvatsara_from_year", st)
}

func RiseSetResultToUTC(lsk *LSK, result RiseSetResult) (UtcTime, error) {
	out, st := cabi.RiseSetResultToUTC(lsk.h, result)
	return out, statusErr("riseset_result_to_utc", st)
}

func (e *Engine) ElongationAt(jdTdb float64) (float64, error) {
	out, st := cabi.ElongationAt(e.h, jdTdb)
	return out, statusErr("elongation_at", st)
}

func (e *Engine) SiderealSumAt(jdTdb float64, config SankrantiConfig) (float64, error) {
	out, st := cabi.SiderealSumAt(e.h, jdTdb, config)
	return out, statusErr("sidereal_sum_at", st)
}

func (e *Engine) VedicDaySunrises(ep *EOP, utc UtcTime, loc GeoLocation, config RiseSetConfig) (float64, float64, error) {
	sunrise, next, st := cabi.VedicDaySunrises(e.h, ep.h, utc, loc, config)
	return sunrise, next, statusErr("vedic_day_sunrises", st)
}

func (e *Engine) BodyEclipticLonLat(bodyCode int32, jdTdb float64) (float64, float64, error) {
	lon, lat, st := cabi.BodyEclipticLonLat(e.h, bodyCode, jdTdb)
	return lon, lat, statusErr("body_ecliptic_lon_lat", st)
}

func (e *Engine) TithiAt(jdTdb, sunriseJd float64) (TithiInfo, error) {
	out, st := cabi.TithiAt(e.h, jdTdb, sunriseJd)
	return out, statusErr("tithi_at", st)
}

func (e *Engine) KaranaAt(jdTdb, sunriseJd float64) (KaranaInfo, error) {
	out, st := cabi.KaranaAt(e.h, jdTdb, sunriseJd)
	return out, statusErr("karana_at", st)
}

func (e *Engine) YogaAt(jdTdb, sunriseJd float64, config SankrantiConfig) (YogaInfo, error) {
	out, st := cabi.YogaAt(e.h, jdTdb, sunriseJd, config)
	return out, statusErr("yoga_at", st)
}

func VaarFromSunrises(lsk *LSK, sunriseJd, nextSunriseJd float64) (VaarInfo, error) {
	out, st := cabi.VaarFromSunrises(lsk.h, sunriseJd, nextSunriseJd)
	return out, statusErr("vaar_from_sunrises", st)
}

func HoraFromSunrises(lsk *LSK, queryJd, sunriseJd, nextSunriseJd float64) (HoraInfo, error) {
	out, st := cabi.HoraFromSunrises(lsk.h, queryJd, sunriseJd, nextSunriseJd)
	return out, statusErr("hora_from_sunrises", st)
}

func GhatikaFromSunrises(lsk *LSK, queryJd, sunriseJd, nextSunriseJd float64) (GhatikaInfo, error) {
	out, st := cabi.GhatikaFromSunrises(lsk.h, queryJd, sunriseJd, nextSunriseJd)
	return out, statusErr("ghatika_from_sunrises", st)
}

func (e *Engine) NakshatraAt(jdTdb, moonSiderealDeg float64, config SankrantiConfig) (PanchangNakshatraInfo, error) {
	out, st := cabi.NakshatraAt(e.h, jdTdb, moonSiderealDeg, config)
	return out, statusErr("nakshatra_at", st)
}

func GhatikaFromElapsed(queryJd, sunriseJd, nextSunriseJd float64) (int32, error) {
	out, st := cabi.GhatikaFromElapsed(queryJd, sunriseJd, nextSunriseJd)
	return out, statusErr("ghatika_from_elapsed", st)
}

func GhatikasSinceSunrise(queryJd, sunriseJd float64) (float64, error) {
	out, st := cabi.GhatikasSinceSunrise(queryJd, sunriseJd)
	return out, statusErr("ghatikas_since_sunrise", st)
}

func AllSphutas(inputs SphutalInputs) (SphutalResult, error) {
	out, st := cabi.AllSphutas(inputs)
	return out, statusErr("all_sphutas", st)
}

func BhriguBindu(rahu, moon float64) float64           { return cabi.BhriguBindu(rahu, moon) }
func PranaSphuta(lagna, moon float64) float64          { return cabi.PranaSphuta(lagna, moon) }
func DehaSphuta(moon, lagna float64) float64           { return cabi.DehaSphuta(moon, lagna) }
func MrityuSphuta(eighthLord, lagna float64) float64   { return cabi.MrityuSphuta(eighthLord, lagna) }
func TithiSphuta(moon, sun, lagna float64) float64     { return cabi.TithiSphuta(moon, sun, lagna) }
func YogaSphuta(sun, moon float64) float64             { return cabi.YogaSphuta(sun, moon) }
func YogaSphutaNormalized(sun, moon float64) float64   { return cabi.YogaSphutaNormalized(sun, moon) }
func RahuTithiSphuta(rahu, sun, lagna float64) float64 { return cabi.RahuTithiSphuta(rahu, sun, lagna) }
func KshetraSphuta(moon, mars, jupiter, venus, lagna float64) float64 {
	return cabi.KshetraSphuta(moon, mars, jupiter, venus, lagna)
}
func BeejaSphuta(sun, venus, jupiter float64) float64 { return cabi.BeejaSphuta(sun, venus, jupiter) }
func Trisphuta(lagna, moon, gulika float64) float64   { return cabi.Trisphuta(lagna, moon, gulika) }
func Chatussphuta(trisphutaVal, sun float64) float64  { return cabi.Chatussphuta(trisphutaVal, sun) }
func Panchasphuta(chatussphutaVal, rahu float64) float64 {
	return cabi.Panchasphuta(chatussphutaVal, rahu)
}
func SookshmaTrisphuta(lagna, moon, gulika, sun float64) float64 {
	return cabi.SookshmaTrisphuta(lagna, moon, gulika, sun)
}
func AvayogaSphuta(sun, moon float64) float64     { return cabi.AvayogaSphuta(sun, moon) }
func Kunda(lagna, moon, mars float64) float64     { return cabi.Kunda(lagna, moon, mars) }
func BhavaLagna(sunLon, ghatikas float64) float64 { return cabi.BhavaLagna(sunLon, ghatikas) }
func HoraLagna(sunLon, ghatikas float64) float64  { return cabi.HoraLagna(sunLon, ghatikas) }
func GhatiLagna(sunLon, ghatikas float64) float64 { return cabi.GhatiLagna(sunLon, ghatikas) }
func VighatiLagna(lagnaLon, vighatikas float64) float64 {
	return cabi.VighatiLagna(lagnaLon, vighatikas)
}
func VarnadaLagna(lagnaLon, horaLagnaLon float64) float64 {
	return cabi.VarnadaLagna(lagnaLon, horaLagnaLon)
}
func SreeLagna(moonLon, lagnaLon float64) float64     { return cabi.SreeLagna(moonLon, lagnaLon) }
func PranapadaLagna(sunLon, ghatikas float64) float64 { return cabi.PranapadaLagna(sunLon, ghatikas) }
func InduLagna(moonLon float64, lagnaLord, moon9thLord uint32) float64 {
	return cabi.InduLagna(moonLon, lagnaLord, moon9thLord)
}

func ArudhaPada(bhavaCuspLon, lordLon float64) (ArudhaResult, error) {
	out, st := cabi.ArudhaPada(bhavaCuspLon, lordLon)
	return out, statusErr("arudha_pada", st)
}

func (e *Engine) SunBasedUpagrahas(jdTdb float64, ayanamshaSystem uint32, useNutation bool) (AllUpagrahas, error) {
	lons, err := e.GrahaSiderealLongitudes(jdTdb, ayanamshaSystem, useNutation)
	if err != nil {
		return AllUpagrahas{}, err
	}
	out, st := cabi.SunBasedUpagrahas(lons.Longitudes[0])
	return out, statusErr("sun_based_upagrahas", st)
}

func TimeUpagrahaConfigDefault() TimeUpagrahaConfig {
	return cabi.TimeUpagrahaConfigDefault()
}

func TimeUpagrahaJD(upagrahaIndex uint32, weekday uint32, isDay bool, sunriseJd, sunsetJd, nextSunriseJd float64) (float64, error) {
	jd, st := cabi.TimeUpagrahaJD(upagrahaIndex, weekday, isDay, sunriseJd, sunsetJd, nextSunriseJd)
	return jd, statusErr("time_upagraha_jd", st)
}

func TimeUpagrahaJDWithConfig(upagrahaIndex uint32, weekday uint32, isDay bool, sunriseJd, sunsetJd, nextSunriseJd float64, cfg TimeUpagrahaConfig) (float64, error) {
	jd, st := cabi.TimeUpagrahaJDWithConfig(upagrahaIndex, weekday, isDay, sunriseJd, sunsetJd, nextSunriseJd, cfg)
	return jd, statusErr("time_upagraha_jd_with_config", st)
}

func (e *Engine) TimeUpagrahaJDUTC(ep *EOP, loc GeoLocation, riseCfg RiseSetConfig, utc UtcTime, upagrahaIndex uint32) (float64, error) {
	jd, st := cabi.TimeUpagrahaJDUTC(e.h, ep.h, loc, riseCfg, utc, upagrahaIndex)
	return jd, statusErr("time_upagraha_jd_utc", st)
}

func (e *Engine) TimeUpagrahaJDUTCWithConfig(ep *EOP, loc GeoLocation, riseCfg RiseSetConfig, utc UtcTime, upagrahaIndex uint32, cfg TimeUpagrahaConfig) (float64, error) {
	jd, st := cabi.TimeUpagrahaJDUTCWithConfig(e.h, ep.h, loc, riseCfg, utc, upagrahaIndex, cfg)
	return jd, statusErr("time_upagraha_jd_utc_with_config", st)
}

func CalculateAshtakavarga(grahaRashis [7]uint8, lagnaRashi uint8) (AshtakavargaResult, error) {
	out, st := cabi.CalculateAshtakavarga(grahaRashis, lagnaRashi)
	return out, statusErr("calculate_ashtakavarga", st)
}

func CalculateBAV(grahaIndex uint8, grahaRashis [7]uint8, lagnaRashi uint8) (BhinnaAshtakavarga, error) {
	out, st := cabi.CalculateBAV(grahaIndex, grahaRashis, lagnaRashi)
	return out, statusErr("calculate_bav", st)
}

func CalculateAllBAV(grahaRashis [7]uint8, lagnaRashi uint8) ([7]BhinnaAshtakavarga, error) {
	out, st := cabi.CalculateAllBAV(grahaRashis, lagnaRashi)
	return out, statusErr("calculate_all_bav", st)
}

func CalculateSAV(bavs [7]BhinnaAshtakavarga) (SarvaAshtakavarga, error) {
	out, st := cabi.CalculateSAV(bavs)
	return out, statusErr("calculate_sav", st)
}

func TrikonaSodhana(totals [12]uint8) ([12]uint8, error) {
	out, st := cabi.TrikonaSodhana(totals)
	return out, statusErr("trikona_sodhana", st)
}

func EkadhipatyaSodhana(totals [12]uint8, grahaRashis [7]uint8, lagnaRashi uint8) ([12]uint8, error) {
	out, st := cabi.EkadhipatyaSodhana(totals, grahaRashis, lagnaRashi)
	return out, statusErr("ekadhipatya_sodhana", st)
}

func (e *Engine) AshtakavargaForDate(ep *EOP, utc UtcTime, loc GeoLocation, ayanamshaSystem uint32, useNutation bool) (AshtakavargaResult, error) {
	out, st := cabi.AshtakavargaForDate(e.h, ep.h, utc, loc, ayanamshaSystem, useNutation)
	return out, statusErr("ashtakavarga_for_date", st)
}

func GrahaDrishti(grahaIndex uint32, sourceLon, targetLon float64) (DrishtiEntry, error) {
	out, st := cabi.GrahaDrishti(grahaIndex, sourceLon, targetLon)
	return out, statusErr("graha_drishti", st)
}

func GrahaDrishtiMatrixForLongitudes(siderealLons [GrahaCount]float64) (GrahaDrishtiMatrix, error) {
	out, st := cabi.GrahaDrishtiMatrixForLongitudes(siderealLons)
	return out, statusErr("graha_drishti_matrix", st)
}

func (e *Engine) DrishtiForDate(ep *EOP, utc UtcTime, loc GeoLocation, bhavaCfg BhavaConfig, riseCfg RiseSetConfig, ayanamshaSystem uint32, useNutation bool, config DrishtiConfig) (DrishtiResult, error) {
	out, st := cabi.DrishtiForDate(e.h, ep.h, utc, loc, bhavaCfg, riseCfg, ayanamshaSystem, useNutation, config)
	return out, statusErr("drishti", st)
}

func (e *Engine) GrahaPositionsForDate(ep *EOP, utc UtcTime, loc GeoLocation, bhavaCfg BhavaConfig, ayanamshaSystem uint32, useNutation bool, cfg GrahaPositionsConfig) (GrahaPositions, error) {
	out, st := cabi.GrahaPositionsForDate(e.h, ep.h, utc, loc, bhavaCfg, ayanamshaSystem, useNutation, cfg)
	return out, statusErr("graha_positions", st)
}

func (e *Engine) CoreBindusForDate(ep *EOP, utc UtcTime, loc GeoLocation, bhavaCfg BhavaConfig, riseCfg RiseSetConfig, ayanamshaSystem uint32, useNutation bool, cfg BindusConfig) (BindusResult, error) {
	out, st := cabi.CoreBindusForDate(e.h, ep.h, utc, loc, bhavaCfg, riseCfg, ayanamshaSystem, useNutation, cfg)
	return out, statusErr("core_bindus", st)
}

func AmshaLongitude(siderealLon float64, amshaCode uint16, variationCode uint8) (float64, error) {
	out, st := cabi.AmshaLongitude(siderealLon, amshaCode, variationCode)
	return out, statusErr("amsha_longitude", st)
}

func AmshaRashiInfo(siderealLon float64, amshaCode uint16, variationCode uint8) (RashiInfo, error) {
	out, st := cabi.AmshaRashiInfo(siderealLon, amshaCode, variationCode)
	return out, statusErr("amsha_rashi_info", st)
}

func AmshaLongitudes(siderealLon float64, amshaCodes []uint16, variationCodes []uint8) ([]float64, error) {
	out, st, prior := cabi.AmshaLongitudes(siderealLon, amshaCodes, variationCodes)
	return out, wrapStatus("amsha_longitudes", st, prior)
}

func (e *Engine) AmshaChartForDate(ep *EOP, utc UtcTime, loc GeoLocation, bhavaCfg BhavaConfig, riseCfg RiseSetConfig, ayanamshaSystem uint32, useNutation bool, amshaCode uint16, variationCode uint8, scope AmshaChartScope) (AmshaChart, error) {
	out, st := cabi.AmshaChartForDate(e.h, ep.h, utc, loc, bhavaCfg, riseCfg, ayanamshaSystem, useNutation, amshaCode, variationCode, scope)
	return out, statusErr("amsha_chart_for_date", st)
}
