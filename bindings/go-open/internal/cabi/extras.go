package cabi

/*
#cgo CFLAGS: -I${SRCDIR}/../../../../crates/dhruv_ffi_c/include
#include "dhruv.h"
*/
import "C"

import (
	"fmt"
	"unsafe"
)

func goDms(v C.DhruvDms) Dms {
	return Dms{Degrees: uint16(v.degrees), Minutes: uint8(v.minutes), Seconds: float64(v.seconds)}
}

func goRashiInfo(v C.DhruvRashiInfo) RashiInfo {
	return RashiInfo{RashiIndex: uint8(v.rashi_index), Dms: goDms(v.dms), DegreesInRashi: float64(v.degrees_in_rashi)}
}

func goNakshatraInfo27(v C.DhruvNakshatraInfo) NakshatraInfo {
	return NakshatraInfo{
		NakshatraIndex:     uint8(v.nakshatra_index),
		Pada:               uint8(v.pada),
		DegreesInNakshatra: float64(v.degrees_in_nakshatra),
		DegreesInPada:      float64(v.degrees_in_pada),
	}
}

func goNakshatra28Info(v C.DhruvNakshatra28Info) Nakshatra28Info {
	return Nakshatra28Info{
		NakshatraIndex:     uint8(v.nakshatra_index),
		Pada:               uint8(v.pada),
		DegreesInNakshatra: float64(v.degrees_in_nakshatra),
	}
}

func goDrishtiEntry(v C.DhruvDrishtiEntry) DrishtiEntry {
	return DrishtiEntry{
		AngularDistance: float64(v.angular_distance),
		BaseVirupa:      float64(v.base_virupa),
		SpecialVirupa:   float64(v.special_virupa),
		TotalVirupa:     float64(v.total_virupa),
	}
}

func goGrahaEntry(v C.DhruvGrahaEntry) GrahaEntry {
	return GrahaEntry{
		SiderealLongitude: float64(v.sidereal_longitude),
		RashiIndex:        uint8(v.rashi_index),
		NakshatraIndex:    uint8(v.nakshatra_index),
		Pada:              uint8(v.pada),
		BhavaNumber:       uint8(v.bhava_number),
		RashiBhavaNumber:  uint8(v.rashi_bhava_number),
	}
}

func cString(ptr *C.char) string {
	if ptr == nil {
		return ""
	}
	return C.GoString(ptr)
}

func DegToDms(degrees float64) (Dms, Status) {
	var out C.DhruvDms
	st := Status(C.dhruv_deg_to_dms(C.double(degrees), &out))
	return goDms(out), st
}

func RashiFromLongitude(siderealLon float64) (RashiInfo, Status) {
	var out C.DhruvRashiInfo
	st := Status(C.dhruv_rashi_from_longitude(C.double(siderealLon), &out))
	return goRashiInfo(out), st
}

func NakshatraFromLongitude(siderealLon float64) (NakshatraInfo, Status) {
	var out C.DhruvNakshatraInfo
	st := Status(C.dhruv_nakshatra_from_longitude(C.double(siderealLon), &out))
	return goNakshatraInfo27(out), st
}

func Nakshatra28FromLongitude(siderealLon float64) (Nakshatra28Info, Status) {
	var out C.DhruvNakshatra28Info
	st := Status(C.dhruv_nakshatra28_from_longitude(C.double(siderealLon), &out))
	return goNakshatra28Info(out), st
}

func RashiFromTropical(tropicalLon float64, ayanamshaSystem uint32, jdTdb float64, useNutation bool) (RashiInfo, Status) {
	var out C.DhruvRashiInfo
	st := Status(C.dhruv_rashi_from_tropical(C.double(tropicalLon), C.uint32_t(ayanamshaSystem), C.double(jdTdb), boolU8(useNutation), &out))
	return goRashiInfo(out), st
}

func NakshatraFromTropical(tropicalLon float64, ayanamshaSystem uint32, jdTdb float64, useNutation bool) (NakshatraInfo, Status) {
	var out C.DhruvNakshatraInfo
	st := Status(C.dhruv_nakshatra_from_tropical(C.double(tropicalLon), C.uint32_t(ayanamshaSystem), C.double(jdTdb), boolU8(useNutation), &out))
	return goNakshatraInfo27(out), st
}

func Nakshatra28FromTropical(tropicalLon float64, ayanamshaSystem uint32, jdTdb float64, useNutation bool) (Nakshatra28Info, Status) {
	var out C.DhruvNakshatra28Info
	st := Status(C.dhruv_nakshatra28_from_tropical(C.double(tropicalLon), C.uint32_t(ayanamshaSystem), C.double(jdTdb), boolU8(useNutation), &out))
	return goNakshatra28Info(out), st
}

func RashiFromTropicalUTC(lsk LskHandle, tropicalLon float64, ayanamshaSystem uint32, utc UtcTime, useNutation bool) (RashiInfo, Status) {
	cutc := cUTC(utc)
	var out C.DhruvRashiInfo
	st := Status(C.dhruv_rashi_from_tropical_utc(lsk.ptr, C.double(tropicalLon), C.uint32_t(ayanamshaSystem), &cutc, boolU8(useNutation), &out))
	return goRashiInfo(out), st
}

func NakshatraFromTropicalUTC(lsk LskHandle, tropicalLon float64, ayanamshaSystem uint32, utc UtcTime, useNutation bool) (NakshatraInfo, Status) {
	cutc := cUTC(utc)
	var out C.DhruvNakshatraInfo
	st := Status(C.dhruv_nakshatra_from_tropical_utc(lsk.ptr, C.double(tropicalLon), C.uint32_t(ayanamshaSystem), &cutc, boolU8(useNutation), &out))
	return goNakshatraInfo27(out), st
}

func Nakshatra28FromTropicalUTC(lsk LskHandle, tropicalLon float64, ayanamshaSystem uint32, utc UtcTime, useNutation bool) (Nakshatra28Info, Status) {
	cutc := cUTC(utc)
	var out C.DhruvNakshatra28Info
	st := Status(C.dhruv_nakshatra28_from_tropical_utc(lsk.ptr, C.double(tropicalLon), C.uint32_t(ayanamshaSystem), &cutc, boolU8(useNutation), &out))
	return goNakshatra28Info(out), st
}

func RashiCount() uint32 { return uint32(C.dhruv_rashi_count()) }
func NakshatraCount(schemeCode uint32) uint32 {
	return uint32(C.dhruv_nakshatra_count(C.uint32_t(schemeCode)))
}
func RashiName(index uint32) string {
	return cString((*C.char)(unsafe.Pointer(C.dhruv_rashi_name(C.uint32_t(index)))))
}
func NakshatraName(index uint32) string {
	return cString((*C.char)(unsafe.Pointer(C.dhruv_nakshatra_name(C.uint32_t(index)))))
}
func Nakshatra28Name(index uint32) string {
	return cString((*C.char)(unsafe.Pointer(C.dhruv_nakshatra28_name(C.uint32_t(index)))))
}
func MasaName(index uint32) string {
	return cString((*C.char)(unsafe.Pointer(C.dhruv_masa_name(C.uint32_t(index)))))
}
func AyanaName(index uint32) string {
	return cString((*C.char)(unsafe.Pointer(C.dhruv_ayana_name(C.uint32_t(index)))))
}
func SamvatsaraName(index uint32) string {
	return cString((*C.char)(unsafe.Pointer(C.dhruv_samvatsara_name(C.uint32_t(index)))))
}
func TithiName(index uint32) string {
	return cString((*C.char)(unsafe.Pointer(C.dhruv_tithi_name(C.uint32_t(index)))))
}
func KaranaName(index uint32) string {
	return cString((*C.char)(unsafe.Pointer(C.dhruv_karana_name(C.uint32_t(index)))))
}
func YogaName(index uint32) string {
	return cString((*C.char)(unsafe.Pointer(C.dhruv_yoga_name(C.uint32_t(index)))))
}
func VaarName(index uint32) string {
	return cString((*C.char)(unsafe.Pointer(C.dhruv_vaar_name(C.uint32_t(index)))))
}
func HoraName(index uint32) string {
	return cString((*C.char)(unsafe.Pointer(C.dhruv_hora_name(C.uint32_t(index)))))
}
func GrahaName(index uint32) string {
	return cString((*C.char)(unsafe.Pointer(C.dhruv_graha_name(C.uint32_t(index)))))
}
func YoginiName(index uint32) string {
	return cString((*C.char)(unsafe.Pointer(C.dhruv_yogini_name(C.uint32_t(index)))))
}
func SphutaName(index uint32) string {
	return cString((*C.char)(unsafe.Pointer(C.dhruv_sphuta_name(C.uint32_t(index)))))
}
func SpecialLagnaName(index uint32) string {
	return cString((*C.char)(unsafe.Pointer(C.dhruv_special_lagna_name(C.uint32_t(index)))))
}
func ArudhaPadaName(index uint32) string {
	return cString((*C.char)(unsafe.Pointer(C.dhruv_arudha_pada_name(C.uint32_t(index)))))
}
func UpagrahaName(index uint32) string {
	return cString((*C.char)(unsafe.Pointer(C.dhruv_upagraha_name(C.uint32_t(index)))))
}

func TithiFromElongation(elongation float64) (TithiPosition, Status) {
	var out C.DhruvTithiPosition
	st := Status(C.dhruv_tithi_from_elongation(C.double(elongation), &out))
	return TithiPosition{TithiIndex: int32(out.tithi_index), Paksha: int32(out.paksha), TithiInPaksha: int32(out.tithi_in_paksha), DegreesInTithi: float64(out.degrees_in_tithi)}, st
}

func KaranaFromElongation(elongation float64) (KaranaPosition, Status) {
	var out C.DhruvKaranaPosition
	st := Status(C.dhruv_karana_from_elongation(C.double(elongation), &out))
	return KaranaPosition{KaranaIndex: int32(out.karana_index), DegreesInKarana: float64(out.degrees_in_karana)}, st
}

func YogaFromSum(sum float64) (YogaPosition, Status) {
	var out C.DhruvYogaPosition
	st := Status(C.dhruv_yoga_from_sum(C.double(sum), &out))
	return YogaPosition{YogaIndex: int32(out.yoga_index), DegreesInYoga: float64(out.degrees_in_yoga)}, st
}

func VaarFromJD(jd float64) int32 { return int32(C.dhruv_vaar_from_jd(C.double(jd))) }
func MasaFromRashiIndex(rashi uint32) int32 {
	return int32(C.dhruv_masa_from_rashi_index(C.uint32_t(rashi)))
}
func AyanaFromSiderealLongitude(lon float64) int32 {
	return int32(C.dhruv_ayana_from_sidereal_longitude(C.double(lon)))
}
func NthRashiFrom(rashi, offset uint32) int32 {
	return int32(C.dhruv_nth_rashi_from(C.uint32_t(rashi), C.uint32_t(offset)))
}
func RashiLord(rashi uint32) int32 { return int32(C.dhruv_rashi_lord(C.uint32_t(rashi))) }
func HoraAt(vaarIndex, horaIndex uint32) int32 {
	return int32(C.dhruv_hora_at(C.uint32_t(vaarIndex), C.uint32_t(horaIndex)))
}
func HoraLord(vaarIndex, horaIndex uint32) int32 {
	return int32(C.dhruv_hora_lord(C.uint32_t(vaarIndex), C.uint32_t(horaIndex)))
}
func MasaLord(masaIndex uint32) int32 {
	return int32(C.dhruv_masa_lord(C.uint32_t(masaIndex)))
}
func SamvatsaraLord(samvatsaraIndex uint32) int32 {
	return int32(C.dhruv_samvatsara_lord(C.uint32_t(samvatsaraIndex)))
}

func ExaltationDegree(grahaIndex uint32) (bool, float64, Status) {
	var has C.uint8_t
	var out C.double
	st := Status(C.dhruv_exaltation_degree(C.uint32_t(grahaIndex), &has, &out))
	return has != 0, float64(out), st
}

func DebilitationDegree(grahaIndex uint32) (bool, float64, Status) {
	var has C.uint8_t
	var out C.double
	st := Status(C.dhruv_debilitation_degree(C.uint32_t(grahaIndex), &has, &out))
	return has != 0, float64(out), st
}

func MoolatrikoneRange(grahaIndex uint32) (bool, uint8, float64, float64, Status) {
	var has C.uint8_t
	var rashi C.uint8_t
	var start C.double
	var end C.double
	st := Status(C.dhruv_moolatrikone_range(C.uint32_t(grahaIndex), &has, &rashi, &start, &end))
	return has != 0, uint8(rashi), float64(start), float64(end), st
}

func CombustionThreshold(grahaIndex uint32, isRetrograde bool) (bool, float64, Status) {
	var has C.uint8_t
	var out C.double
	st := Status(C.dhruv_combustion_threshold(C.uint32_t(grahaIndex), boolU8(isRetrograde), &has, &out))
	return has != 0, float64(out), st
}

func IsCombust(grahaIndex uint32, grahaSidLon, sunSidLon float64, isRetrograde bool) (bool, Status) {
	var out C.uint8_t
	st := Status(C.dhruv_is_combust(C.uint32_t(grahaIndex), C.double(grahaSidLon), C.double(sunSidLon), boolU8(isRetrograde), &out))
	return out != 0, st
}

func AllCombustionStatus(siderealLons [9]float64, retrogradeFlags [9]bool) ([9]bool, Status) {
	var csidereal [9]C.double
	var cretro [9]C.uint8_t
	for i := 0; i < 9; i++ {
		csidereal[i] = C.double(siderealLons[i])
		cretro[i] = boolU8(retrogradeFlags[i])
	}
	var out [9]C.uint8_t
	st := Status(C.dhruv_all_combustion_status(&csidereal[0], &cretro[0], &out[0]))
	var result [9]bool
	for i := 0; i < 9; i++ {
		result[i] = out[i] != 0
	}
	return result, st
}

func NaisargikaMaitri(grahaIndex, otherIndex uint32) (int32, Status) {
	var out C.int32_t
	st := Status(C.dhruv_naisargika_maitri(C.uint32_t(grahaIndex), C.uint32_t(otherIndex), &out))
	return int32(out), st
}

func TatkalikaMaitri(grahaRashiIndex, otherRashiIndex uint32) (int32, Status) {
	var out C.int32_t
	st := Status(C.dhruv_tatkalika_maitri(C.uint32_t(grahaRashiIndex), C.uint32_t(otherRashiIndex), &out))
	return int32(out), st
}

func PanchadhaMaitri(naisargikaCode, tatkalikaCode int32) (int32, Status) {
	var out C.int32_t
	st := Status(C.dhruv_panchadha_maitri(C.int32_t(naisargikaCode), C.int32_t(tatkalikaCode), &out))
	return int32(out), st
}

func DignityInRashi(grahaIndex uint32, siderealLon float64, rashiIndex uint32) (int32, Status) {
	var out C.int32_t
	st := Status(C.dhruv_dignity_in_rashi(C.uint32_t(grahaIndex), C.double(siderealLon), C.uint32_t(rashiIndex), &out))
	return int32(out), st
}

func DignityInRashiWithPositions(grahaIndex uint32, siderealLon float64, rashiIndex uint32, saptaRashiIndices [7]uint8) (int32, Status) {
	var csapta [7]C.uint8_t
	for i := 0; i < 7; i++ {
		csapta[i] = C.uint8_t(saptaRashiIndices[i])
	}
	var out C.int32_t
	st := Status(C.dhruv_dignity_in_rashi_with_positions(C.uint32_t(grahaIndex), C.double(siderealLon), C.uint32_t(rashiIndex), &csapta[0], &out))
	return int32(out), st
}

func NodeDignityInRashi(grahaIndex uint32, rashiIndex uint32, grahaRashiIndices [9]uint8, policyCode int32) (int32, Status) {
	var call [9]C.uint8_t
	for i := 0; i < 9; i++ {
		call[i] = C.uint8_t(grahaRashiIndices[i])
	}
	var out C.int32_t
	st := Status(C.dhruv_node_dignity_in_rashi(C.uint32_t(grahaIndex), C.uint32_t(rashiIndex), &call[0], C.int32_t(policyCode), &out))
	return int32(out), st
}

func NaturalBeneficMalefic(grahaIndex uint32) (int32, Status) {
	var out C.int32_t
	st := Status(C.dhruv_natural_benefic_malefic(C.uint32_t(grahaIndex), &out))
	return int32(out), st
}

func MoonBeneficNature(moonSunElongation float64) (int32, Status) {
	var out C.int32_t
	st := Status(C.dhruv_moon_benefic_nature(C.double(moonSunElongation), &out))
	return int32(out), st
}

func GrahaGender(grahaIndex uint32) (int32, Status) {
	var out C.int32_t
	st := Status(C.dhruv_graha_gender(C.uint32_t(grahaIndex), &out))
	return int32(out), st
}

func SamvatsaraFromYear(year int32) (SamvatsaraResult, Status) {
	var out C.DhruvSamvatsaraResult
	st := Status(C.dhruv_samvatsara_from_year(C.int32_t(year), &out))
	return SamvatsaraResult{SamvatsaraIndex: int32(out.samvatsara_index), CyclePosition: int32(out.cycle_position)}, st
}

func RiseSetResultToUTC(lsk LskHandle, result RiseSetResult) (UtcTime, Status) {
	cres := C.DhruvRiseSetResult{result_type: C.int32_t(result.ResultType), event_code: C.int32_t(result.EventCode), jd_tdb: C.double(result.JdTdb)}
	var out C.DhruvUtcTime
	st := Status(C.dhruv_riseset_result_to_utc(lsk.ptr, &cres, &out))
	return goUTC(out), st
}

func ElongationAt(engine EngineHandle, jdTdb float64) (float64, Status) {
	var out C.double
	st := Status(C.dhruv_elongation_at(engine.ptr, C.double(jdTdb), &out))
	return float64(out), st
}

func SiderealSumAt(engine EngineHandle, jdTdb float64, config SankrantiConfig) (float64, Status) {
	ccfg := cSankrantiConfig(config)
	var out C.double
	st := Status(C.dhruv_sidereal_sum_at(engine.ptr, C.double(jdTdb), &ccfg, &out))
	return float64(out), st
}

func VedicDaySunrises(engine EngineHandle, eop EopHandle, utc UtcTime, loc GeoLocation, config RiseSetConfig) (float64, float64, Status) {
	cutc, cloc, ccfg := cUTC(utc), cGeo(loc), cRiseSetConfig(config)
	var sunrise, next C.double
	st := Status(C.dhruv_vedic_day_sunrises(engine.ptr, eop.ptr, &cutc, &cloc, &ccfg, &sunrise, &next))
	return float64(sunrise), float64(next), st
}

func BodyEclipticLonLat(engine EngineHandle, bodyCode int32, jdTdb float64) (float64, float64, Status) {
	var lon, lat C.double
	st := Status(C.dhruv_body_ecliptic_lon_lat(engine.ptr, C.int32_t(bodyCode), C.double(jdTdb), &lon, &lat))
	return float64(lon), float64(lat), st
}

func TithiAt(engine EngineHandle, jdTdb, sunriseJd float64) (TithiInfo, Status) {
	var out C.DhruvTithiInfo
	st := Status(C.dhruv_tithi_at(engine.ptr, C.double(jdTdb), C.double(sunriseJd), &out))
	return goTithiInfo(out), st
}

func KaranaAt(engine EngineHandle, jdTdb, sunriseJd float64) (KaranaInfo, Status) {
	var out C.DhruvKaranaInfo
	st := Status(C.dhruv_karana_at(engine.ptr, C.double(jdTdb), C.double(sunriseJd), &out))
	return goKaranaInfo(out), st
}

func YogaAt(engine EngineHandle, jdTdb, sunriseJd float64, config SankrantiConfig) (YogaInfo, Status) {
	ccfg := cSankrantiConfig(config)
	var out C.DhruvYogaInfo
	st := Status(C.dhruv_yoga_at(engine.ptr, C.double(jdTdb), C.double(sunriseJd), &ccfg, &out))
	return goYogaInfo(out), st
}

func VaarFromSunrises(lsk LskHandle, sunriseJd, nextSunriseJd float64) (VaarInfo, Status) {
	var out C.DhruvVaarInfo
	st := Status(C.dhruv_vaar_from_sunrises(lsk.ptr, C.double(sunriseJd), C.double(nextSunriseJd), &out))
	return goVaarInfo(out), st
}

func HoraFromSunrises(lsk LskHandle, queryJd, sunriseJd, nextSunriseJd float64) (HoraInfo, Status) {
	var out C.DhruvHoraInfo
	st := Status(C.dhruv_hora_from_sunrises(lsk.ptr, C.double(queryJd), C.double(sunriseJd), C.double(nextSunriseJd), &out))
	return goHoraInfo(out), st
}

func GhatikaFromSunrises(lsk LskHandle, queryJd, sunriseJd, nextSunriseJd float64) (GhatikaInfo, Status) {
	var out C.DhruvGhatikaInfo
	st := Status(C.dhruv_ghatika_from_sunrises(lsk.ptr, C.double(queryJd), C.double(sunriseJd), C.double(nextSunriseJd), &out))
	return goGhatikaInfo(out), st
}

func NakshatraAt(engine EngineHandle, jdTdb, moonSiderealDeg float64, config SankrantiConfig) (PanchangNakshatraInfo, Status) {
	ccfg := cSankrantiConfig(config)
	var out C.DhruvPanchangNakshatraInfo
	st := Status(C.dhruv_nakshatra_at(engine.ptr, C.double(jdTdb), C.double(moonSiderealDeg), &ccfg, &out))
	return goNakshatraInfo(out), st
}

func GhatikaFromElapsed(queryJd, sunriseJd, nextSunriseJd float64) (int32, Status) {
	var out C.int32_t
	st := Status(C.dhruv_ghatika_from_elapsed(C.double(queryJd), C.double(sunriseJd), C.double(nextSunriseJd), &out))
	return int32(out), st
}

func GhatikasSinceSunrise(queryJd, sunriseJd float64) (float64, Status) {
	var out C.double
	st := Status(C.dhruv_ghatikas_since_sunrise(C.double(queryJd), C.double(sunriseJd), &out))
	return float64(out), st
}

func AllSphutas(inputs SphutalInputs) (SphutalResult, Status) {
	cin := C.DhruvSphutalInputs{
		sun:         C.double(inputs.Sun),
		moon:        C.double(inputs.Moon),
		mars:        C.double(inputs.Mars),
		jupiter:     C.double(inputs.Jupiter),
		venus:       C.double(inputs.Venus),
		rahu:        C.double(inputs.Rahu),
		lagna:       C.double(inputs.Lagna),
		eighth_lord: C.double(inputs.EighthLord),
		gulika:      C.double(inputs.Gulika),
	}
	var out C.DhruvSphutalResult
	st := Status(C.dhruv_all_sphutas(&cin, &out))
	var res SphutalResult
	for i := 0; i < SphutaCount; i++ {
		res.Longitudes[i] = float64(out.longitudes[i])
	}
	return res, st
}

func BhriguBindu(rahu, moon float64) float64 {
	return float64(C.dhruv_bhrigu_bindu(C.double(rahu), C.double(moon)))
}
func PranaSphuta(lagna, moon float64) float64 {
	return float64(C.dhruv_prana_sphuta(C.double(lagna), C.double(moon)))
}
func DehaSphuta(moon, lagna float64) float64 {
	return float64(C.dhruv_deha_sphuta(C.double(moon), C.double(lagna)))
}
func MrityuSphuta(eighthLord, lagna float64) float64 {
	return float64(C.dhruv_mrityu_sphuta(C.double(eighthLord), C.double(lagna)))
}
func TithiSphuta(moon, sun, lagna float64) float64 {
	return float64(C.dhruv_tithi_sphuta(C.double(moon), C.double(sun), C.double(lagna)))
}
func YogaSphuta(sun, moon float64) float64 {
	return float64(C.dhruv_yoga_sphuta(C.double(sun), C.double(moon)))
}
func YogaSphutaNormalized(sun, moon float64) float64 {
	return float64(C.dhruv_yoga_sphuta_normalized(C.double(sun), C.double(moon)))
}
func RahuTithiSphuta(rahu, sun, lagna float64) float64 {
	return float64(C.dhruv_rahu_tithi_sphuta(C.double(rahu), C.double(sun), C.double(lagna)))
}
func KshetraSphuta(moon, mars, jupiter, venus, lagna float64) float64 {
	// Keep Go API argument order stable while mapping to C ABI order.
	return float64(C.dhruv_kshetra_sphuta(C.double(venus), C.double(moon), C.double(mars), C.double(jupiter), C.double(lagna)))
}
func BeejaSphuta(sun, venus, jupiter float64) float64 {
	return float64(C.dhruv_beeja_sphuta(C.double(sun), C.double(venus), C.double(jupiter)))
}
func Trisphuta(lagna, moon, gulika float64) float64 {
	return float64(C.dhruv_trisphuta(C.double(lagna), C.double(moon), C.double(gulika)))
}
func Chatussphuta(trisphutaVal, sun float64) float64 {
	return float64(C.dhruv_chatussphuta(C.double(trisphutaVal), C.double(sun)))
}
func Panchasphuta(chatussphutaVal, rahu float64) float64 {
	return float64(C.dhruv_panchasphuta(C.double(chatussphutaVal), C.double(rahu)))
}
func SookshmaTrisphuta(lagna, moon, gulika, sun float64) float64 {
	return float64(C.dhruv_sookshma_trisphuta(C.double(lagna), C.double(moon), C.double(gulika), C.double(sun)))
}
func AvayogaSphuta(sun, moon float64) float64 {
	return float64(C.dhruv_avayoga_sphuta(C.double(sun), C.double(moon)))
}
func Kunda(lagna, moon, mars float64) float64 {
	return float64(C.dhruv_kunda(C.double(lagna), C.double(moon), C.double(mars)))
}
func BhavaLagna(sunLon, ghatikas float64) float64 {
	return float64(C.dhruv_bhava_lagna(C.double(sunLon), C.double(ghatikas)))
}
func HoraLagna(sunLon, ghatikas float64) float64 {
	return float64(C.dhruv_hora_lagna(C.double(sunLon), C.double(ghatikas)))
}
func GhatiLagna(sunLon, ghatikas float64) float64 {
	return float64(C.dhruv_ghati_lagna(C.double(sunLon), C.double(ghatikas)))
}
func VighatiLagna(lagnaLon, vighatikas float64) float64 {
	return float64(C.dhruv_vighati_lagna(C.double(lagnaLon), C.double(vighatikas)))
}
func VarnadaLagna(lagnaLon, horaLagnaLon float64) float64 {
	return float64(C.dhruv_varnada_lagna(C.double(lagnaLon), C.double(horaLagnaLon)))
}
func SreeLagna(moonLon, lagnaLon float64) float64 {
	return float64(C.dhruv_sree_lagna(C.double(moonLon), C.double(lagnaLon)))
}
func PranapadaLagna(sunLon, ghatikas float64) float64 {
	return float64(C.dhruv_pranapada_lagna(C.double(sunLon), C.double(ghatikas)))
}
func InduLagna(moonLon float64, lagnaLord, moon9thLord uint32) float64 {
	return float64(C.dhruv_indu_lagna(C.double(moonLon), C.uint32_t(lagnaLord), C.uint32_t(moon9thLord)))
}

func ArudhaPada(bhavaCuspLon, lordLon float64) (ArudhaResult, Status) {
	var outRashi C.uint8_t
	lon := C.dhruv_arudha_pada(C.double(bhavaCuspLon), C.double(lordLon), &outRashi)
	return ArudhaResult{BhavaNumber: 0, LongitudeDeg: float64(lon), RashiIndex: uint8(outRashi)}, StatusOK
}

func SunBasedUpagrahas(sunSidLon float64) (AllUpagrahas, Status) {
	var out C.DhruvAllUpagrahas
	st := Status(C.dhruv_sun_based_upagrahas(C.double(sunSidLon), &out))
	return AllUpagrahas{
		Gulika: float64(out.gulika), Maandi: float64(out.maandi), Kaala: float64(out.kaala), Mrityu: float64(out.mrityu),
		ArthaPrahara: float64(out.artha_prahara), YamaGhantaka: float64(out.yama_ghantaka), Dhooma: float64(out.dhooma),
		Vyatipata: float64(out.vyatipata), Parivesha: float64(out.parivesha), IndraChapa: float64(out.indra_chapa), Upaketu: float64(out.upaketu),
	}, st
}

func TimeUpagrahaJD(upagrahaIndex uint32, weekday uint32, isDay bool, sunriseJd, sunsetJd, nextSunriseJd float64) (float64, Status) {
	var outJd C.double
	st := Status(C.dhruv_time_upagraha_jd(
		C.uint32_t(upagrahaIndex),
		C.uint32_t(weekday),
		boolU8(isDay),
		C.double(sunriseJd),
		C.double(sunsetJd),
		C.double(nextSunriseJd),
		&outJd,
	))
	return float64(outJd), st
}

func TimeUpagrahaJDWithConfig(upagrahaIndex uint32, weekday uint32, isDay bool, sunriseJd, sunsetJd, nextSunriseJd float64, cfg TimeUpagrahaConfig) (float64, Status) {
	var outJd C.double
	ccfg := cTimeUpagrahaConfig(cfg)
	st := Status(C.dhruv_time_upagraha_jd_with_config(
		C.uint32_t(upagrahaIndex),
		C.uint32_t(weekday),
		boolU8(isDay),
		C.double(sunriseJd),
		C.double(sunsetJd),
		C.double(nextSunriseJd),
		&ccfg,
		&outJd,
	))
	return float64(outJd), st
}

func TimeUpagrahaJDUTC(engine EngineHandle, eop EopHandle, loc GeoLocation, risesetConfig RiseSetConfig, utc UtcTime, upagrahaIndex uint32) (float64, Status) {
	cloc, ccfg, cutc := cGeo(loc), cRiseSetConfig(risesetConfig), cUTC(utc)
	var outJD C.double
	st := Status(C.dhruv_time_upagraha_jd_utc(engine.ptr, eop.ptr, &cutc, &cloc, &ccfg, C.uint32_t(upagrahaIndex), &outJD))
	return float64(outJD), st
}

func TimeUpagrahaJDUTCWithConfig(engine EngineHandle, eop EopHandle, loc GeoLocation, risesetConfig RiseSetConfig, utc UtcTime, upagrahaIndex uint32, upagrahaConfig TimeUpagrahaConfig) (float64, Status) {
	cloc, ccfg, cutc, cupa := cGeo(loc), cRiseSetConfig(risesetConfig), cUTC(utc), cTimeUpagrahaConfig(upagrahaConfig)
	var outJD C.double
	st := Status(C.dhruv_time_upagraha_jd_utc_with_config(
		engine.ptr,
		eop.ptr,
		&cutc,
		&cloc,
		&ccfg,
		&cupa,
		C.uint32_t(upagrahaIndex),
		&outJD,
	))
	return float64(outJD), st
}

func goBhinna(v C.DhruvBhinnaAshtakavarga) BhinnaAshtakavarga {
	out := BhinnaAshtakavarga{GrahaIndex: uint8(v.graha_index)}
	for i := 0; i < 12; i++ {
		out.Points[i] = uint8(v.points[i])
		for j := 0; j < 8; j++ {
			out.Contributors[i][j] = uint8(v.contributors[i][j])
		}
	}
	return out
}

func goSarva(v C.DhruvSarvaAshtakavarga) SarvaAshtakavarga {
	var out SarvaAshtakavarga
	for i := 0; i < 12; i++ {
		out.TotalPoints[i] = uint8(v.total_points[i])
		out.AfterTrikona[i] = uint8(v.after_trikona[i])
		out.AfterEkadhipatya[i] = uint8(v.after_ekadhipatya[i])
	}
	return out
}

func goAshtakavarga(v C.DhruvAshtakavargaResult) AshtakavargaResult {
	var out AshtakavargaResult
	for i := 0; i < SaptaGrahaCount; i++ {
		out.BAVs[i] = goBhinna(v.bavs[i])
	}
	out.SAV = goSarva(v.sav)
	return out
}

func CalculateAshtakavarga(grahaRashis [SaptaGrahaCount]uint8, lagnaRashi uint8) (AshtakavargaResult, Status) {
	var out C.DhruvAshtakavargaResult
	st := Status(C.dhruv_calculate_ashtakavarga((*C.uint8_t)(unsafe.Pointer(&grahaRashis[0])), C.uint8_t(lagnaRashi), &out))
	return goAshtakavarga(out), st
}

func CalculateBAV(grahaIndex uint8, grahaRashis [SaptaGrahaCount]uint8, lagnaRashi uint8) (BhinnaAshtakavarga, Status) {
	var out C.DhruvBhinnaAshtakavarga
	st := Status(C.dhruv_calculate_bav(C.uint8_t(grahaIndex), (*C.uint8_t)(unsafe.Pointer(&grahaRashis[0])), C.uint8_t(lagnaRashi), &out))
	return goBhinna(out), st
}

func CalculateAllBAV(grahaRashis [SaptaGrahaCount]uint8, lagnaRashi uint8) ([SaptaGrahaCount]BhinnaAshtakavarga, Status) {
	var out [SaptaGrahaCount]C.DhruvBhinnaAshtakavarga
	st := Status(C.dhruv_calculate_all_bav((*C.uint8_t)(unsafe.Pointer(&grahaRashis[0])), C.uint8_t(lagnaRashi), (*C.DhruvBhinnaAshtakavarga)(unsafe.Pointer(&out[0]))))
	var res [SaptaGrahaCount]BhinnaAshtakavarga
	for i := 0; i < SaptaGrahaCount; i++ {
		res[i] = goBhinna(out[i])
	}
	return res, st
}

func CalculateSAV(bavs [SaptaGrahaCount]BhinnaAshtakavarga) (SarvaAshtakavarga, Status) {
	var cbavs [SaptaGrahaCount]C.DhruvBhinnaAshtakavarga
	for i := 0; i < SaptaGrahaCount; i++ {
		cbavs[i].graha_index = C.uint8_t(bavs[i].GrahaIndex)
		for j := 0; j < 12; j++ {
			cbavs[i].points[j] = C.uint8_t(bavs[i].Points[j])
			for k := 0; k < 8; k++ {
				cbavs[i].contributors[j][k] = C.uint8_t(bavs[i].Contributors[j][k])
			}
		}
	}
	var out C.DhruvSarvaAshtakavarga
	st := Status(C.dhruv_calculate_sav((*C.DhruvBhinnaAshtakavarga)(unsafe.Pointer(&cbavs[0])), &out))
	return goSarva(out), st
}

func TrikonaSodhana(totals [12]uint8) ([12]uint8, Status) {
	var out [12]C.uint8_t
	st := Status(C.dhruv_trikona_sodhana((*C.uint8_t)(unsafe.Pointer(&totals[0])), (*C.uint8_t)(unsafe.Pointer(&out[0]))))
	var res [12]uint8
	for i := 0; i < 12; i++ {
		res[i] = uint8(out[i])
	}
	return res, st
}

func EkadhipatyaSodhana(totals [12]uint8, grahaRashis [SaptaGrahaCount]uint8, lagnaRashi uint8) ([12]uint8, Status) {
	var out [12]C.uint8_t
	st := Status(C.dhruv_ekadhipatya_sodhana((*C.uint8_t)(unsafe.Pointer(&totals[0])), (*C.uint8_t)(unsafe.Pointer(&grahaRashis[0])), C.uint8_t(lagnaRashi), (*C.uint8_t)(unsafe.Pointer(&out[0]))))
	var res [12]uint8
	for i := 0; i < 12; i++ {
		res[i] = uint8(out[i])
	}
	return res, st
}

func AshtakavargaForDate(engine EngineHandle, eop EopHandle, utc UtcTime, loc GeoLocation, ayanamshaSystem uint32, useNutation bool) (AshtakavargaResult, Status) {
	cutc, cloc := cUTC(utc), cGeo(loc)
	var out C.DhruvAshtakavargaResult
	st := Status(C.dhruv_ashtakavarga_for_date(engine.ptr, eop.ptr, &cutc, &cloc, C.uint32_t(ayanamshaSystem), boolU8(useNutation), &out))
	return goAshtakavarga(out), st
}

func GrahaDrishti(grahaIndex uint32, sourceLon, targetLon float64) (DrishtiEntry, Status) {
	var out C.DhruvDrishtiEntry
	st := Status(C.dhruv_graha_drishti(C.uint32_t(grahaIndex), C.double(sourceLon), C.double(targetLon), &out))
	return goDrishtiEntry(out), st
}

func GrahaDrishtiMatrixForLongitudes(siderealLons [GrahaCount]float64) (GrahaDrishtiMatrix, Status) {
	var csid [GrahaCount]C.double
	for i := 0; i < GrahaCount; i++ {
		csid[i] = C.double(siderealLons[i])
	}
	var out C.DhruvGrahaDrishtiMatrix
	st := Status(C.dhruv_graha_drishti_matrix(&csid[0], &out))
	var res GrahaDrishtiMatrix
	for i := 0; i < GrahaCount; i++ {
		for j := 0; j < GrahaCount; j++ {
			res.Entries[i][j] = goDrishtiEntry(out.entries[i][j])
		}
	}
	return res, st
}

func cDrishtiConfig(cfg DrishtiConfig) C.DhruvDrishtiConfig {
	return C.DhruvDrishtiConfig{include_bhava: boolU8(cfg.IncludeBhava), include_lagna: boolU8(cfg.IncludeLagna), include_bindus: boolU8(cfg.IncludeBindus)}
}

func DrishtiForDate(engine EngineHandle, eop EopHandle, utc UtcTime, loc GeoLocation, bhavaCfg BhavaConfig, riseCfg RiseSetConfig, ayanamshaSystem uint32, useNutation bool, config DrishtiConfig) (DrishtiResult, Status) {
	cutc, cloc := cUTC(utc), cGeo(loc)
	cbhava, crise, cdr := cBhavaConfig(bhavaCfg), cRiseSetConfig(riseCfg), cDrishtiConfig(config)
	var out C.DhruvDrishtiResult
	st := Status(C.dhruv_drishti(engine.ptr, eop.ptr, &cutc, &cloc, &cbhava, &crise, C.uint32_t(ayanamshaSystem), boolU8(useNutation), &cdr, &out))
	var res DrishtiResult
	for i := 0; i < GrahaCount; i++ {
		for j := 0; j < GrahaCount; j++ {
			res.GrahaToGraha[i][j] = goDrishtiEntry(out.graha_to_graha[i][j])
		}
		for j := 0; j < 12; j++ {
			res.GrahaToBhava[i][j] = goDrishtiEntry(out.graha_to_bhava[i][j])
			res.GrahaToRashiBhava[i][j] = goDrishtiEntry(out.graha_to_rashi_bhava[i][j])
		}
		res.GrahaToLagna[i] = goDrishtiEntry(out.graha_to_lagna[i])
		for j := 0; j < 19; j++ {
			res.GrahaToBindus[i][j] = goDrishtiEntry(out.graha_to_bindus[i][j])
		}
	}
	return res, st
}

func cGrahaPositionsConfig(cfg GrahaPositionsConfig) C.DhruvGrahaPositionsConfig {
	return C.DhruvGrahaPositionsConfig{
		include_nakshatra:     boolU8(cfg.IncludeNakshatra),
		include_lagna:         boolU8(cfg.IncludeLagna),
		include_outer_planets: boolU8(cfg.IncludeOuterPlanets),
		include_bhava:         boolU8(cfg.IncludeBhava),
	}
}

func GrahaPositionsForDate(engine EngineHandle, eop EopHandle, utc UtcTime, loc GeoLocation, bhavaCfg BhavaConfig, ayanamshaSystem uint32, useNutation bool, cfg GrahaPositionsConfig) (GrahaPositions, Status) {
	cutc, cloc := cUTC(utc), cGeo(loc)
	cbhava, ccfg := cBhavaConfig(bhavaCfg), cGrahaPositionsConfig(cfg)
	var out C.DhruvGrahaPositions
	st := Status(C.dhruv_graha_positions(engine.ptr, eop.ptr, &cutc, &cloc, &cbhava, C.uint32_t(ayanamshaSystem), boolU8(useNutation), &ccfg, &out))
	var res GrahaPositions
	for i := 0; i < GrahaCount; i++ {
		res.Grahas[i] = goGrahaEntry(out.grahas[i])
	}
	res.Lagna = goGrahaEntry(out.lagna)
	for i := 0; i < 3; i++ {
		res.OuterPlanets[i] = goGrahaEntry(out.outer_planets[i])
	}
	return res, st
}

func cBindusConfig(cfg BindusConfig) C.DhruvBindusConfig {
	return C.DhruvBindusConfig{
		include_nakshatra: boolU8(cfg.IncludeNakshatra),
		include_bhava:     boolU8(cfg.IncludeBhava),
		upagraha_config:   cTimeUpagrahaConfig(cfg.UpagrahaConfig),
	}
}

func CoreBindusForDate(engine EngineHandle, eop EopHandle, utc UtcTime, loc GeoLocation, bhavaCfg BhavaConfig, riseCfg RiseSetConfig, ayanamshaSystem uint32, useNutation bool, cfg BindusConfig) (BindusResult, Status) {
	cutc, cloc := cUTC(utc), cGeo(loc)
	cbhava, crise, ccfg := cBhavaConfig(bhavaCfg), cRiseSetConfig(riseCfg), cBindusConfig(cfg)
	var out C.DhruvBindusResult
	st := Status(C.dhruv_core_bindus(engine.ptr, eop.ptr, &cutc, &cloc, &cbhava, &crise, C.uint32_t(ayanamshaSystem), boolU8(useNutation), &ccfg, &out))
	var res BindusResult
	for i := 0; i < 12; i++ {
		res.ArudhaPadas[i] = goGrahaEntry(out.arudha_padas[i])
	}
	res.BhriguBindu = goGrahaEntry(out.bhrigu_bindu)
	res.PranapadaLagna = goGrahaEntry(out.pranapada_lagna)
	res.Gulika = goGrahaEntry(out.gulika)
	res.Maandi = goGrahaEntry(out.maandi)
	res.HoraLagna = goGrahaEntry(out.hora_lagna)
	res.GhatiLagna = goGrahaEntry(out.ghati_lagna)
	res.SreeLagna = goGrahaEntry(out.sree_lagna)
	return res, st
}

func AmshaLongitude(siderealLon float64, amshaCode uint16, variationCode uint8) (float64, Status) {
	var out C.double
	st := Status(C.dhruv_amsha_longitude(C.double(siderealLon), C.uint16_t(amshaCode), C.uint8_t(variationCode), &out))
	return float64(out), st
}

func AmshaRashiInfo(siderealLon float64, amshaCode uint16, variationCode uint8) (RashiInfo, Status) {
	var out C.DhruvRashiInfo
	st := Status(C.dhruv_amsha_rashi_info(C.double(siderealLon), C.uint16_t(amshaCode), C.uint8_t(variationCode), &out))
	return goRashiInfo(out), st
}

func AmshaLongitudes(siderealLon float64, amshaCodes []uint16, variationCodes []uint8) ([]float64, Status, error) {
	if len(amshaCodes) != len(variationCodes) {
		return nil, StatusInvalidInput, fmt.Errorf("amsha code/variation length mismatch")
	}
	if len(amshaCodes) == 0 {
		return []float64{}, StatusOK, nil
	}
	count := len(amshaCodes)
	codes := make([]C.uint16_t, count)
	vars := make([]C.uint8_t, count)
	out := make([]C.double, count)
	for i := 0; i < count; i++ {
		codes[i] = C.uint16_t(amshaCodes[i])
		vars[i] = C.uint8_t(variationCodes[i])
	}
	st := Status(C.dhruv_amsha_longitudes(C.double(siderealLon), &codes[0], &vars[0], C.uint32_t(count), &out[0]))
	res := make([]float64, count)
	for i := 0; i < count; i++ {
		res[i] = float64(out[i])
	}
	return res, st, nil
}

func cAmshaScope(scope AmshaChartScope) C.DhruvAmshaChartScope {
	return C.DhruvAmshaChartScope{
		include_bhava_cusps:    boolU8(scope.IncludeBhavaCusps),
		include_arudha_padas:   boolU8(scope.IncludeArudhaPadas),
		include_upagrahas:      boolU8(scope.IncludeUpagrahas),
		include_sphutas:        boolU8(scope.IncludeSphutas),
		include_special_lagnas: boolU8(scope.IncludeSpecialLagnas),
		include_outer_planets:  boolU8(scope.IncludeOuterPlanets),
	}
}

func goAmshaEntry(v C.DhruvAmshaEntry) AmshaEntry {
	return AmshaEntry{
		SiderealLongitude: float64(v.sidereal_longitude),
		RashiIndex:        uint8(v.rashi_index),
		DmsDegrees:        uint16(v.dms_degrees),
		DmsMinutes:        uint8(v.dms_minutes),
		DmsSeconds:        float64(v.dms_seconds),
		DegreesInRashi:    float64(v.degrees_in_rashi),
	}
}

func goAmshaEntries(src []C.DhruvAmshaEntry) []AmshaEntry {
	out := make([]AmshaEntry, len(src))
	for i := range src {
		out[i] = goAmshaEntry(src[i])
	}
	return out
}

func goAmshaVariationInfo(v C.DhruvAmshaVariationInfo) AmshaVariationInfo {
	return AmshaVariationInfo{
		AmshaCode:     uint16(v.amsha_code),
		VariationCode: uint8(v.variation_code),
		Name:          cString((*C.char)(unsafe.Pointer(&v.name[0]))),
		Label:         cString((*C.char)(unsafe.Pointer(&v.label[0]))),
		IsDefault:     v.is_default != 0,
		Description:   cString((*C.char)(unsafe.Pointer(&v.description[0]))),
	}
}

func goAmshaVariationCatalog(v C.DhruvAmshaVariationList) AmshaVariationCatalog {
	out := AmshaVariationCatalog{
		AmshaCode:            uint16(v.amsha_code),
		DefaultVariationCode: uint8(v.default_variation_code),
		Variations:           make([]AmshaVariationInfo, int(v.count)),
	}
	for i := 0; i < int(v.count); i++ {
		out.Variations[i] = goAmshaVariationInfo(v.variations[i])
	}
	return out
}

func AmshaChartForDate(engine EngineHandle, eop EopHandle, utc UtcTime, loc GeoLocation, bhavaCfg BhavaConfig, riseCfg RiseSetConfig, ayanamshaSystem uint32, useNutation bool, amshaCode uint16, variationCode uint8, scope AmshaChartScope) (AmshaChart, Status) {
	cutc, cloc := cUTC(utc), cGeo(loc)
	cbhava, crise, cscope := cBhavaConfig(bhavaCfg), cRiseSetConfig(riseCfg), cAmshaScope(scope)
	var out C.DhruvAmshaChart
	st := Status(C.dhruv_amsha_chart_for_date(engine.ptr, eop.ptr, &cutc, &cloc, &cbhava, &crise, C.uint32_t(ayanamshaSystem), boolU8(useNutation), C.uint16_t(amshaCode), C.uint8_t(variationCode), &cscope, &out))
	res := AmshaChart{
		AmshaCode:                  uint16(out.amsha_code),
		VariationCode:              uint8(out.variation_code),
		Lagna:                      goAmshaEntry(out.lagna),
		BhavaCuspsValid:            out.bhava_cusps_valid != 0,
		RashiBhavaCuspsValid:       out.rashi_bhava_cusps_valid != 0,
		ArudhaPadasValid:           out.arudha_padas_valid != 0,
		RashiBhavaArudhaPadasValid: out.rashi_bhava_arudha_padas_valid != 0,
		UpagrahasValid:             out.upagrahas_valid != 0,
		SphutasValid:               out.sphutas_valid != 0,
		SpecialLagnasValid:         out.special_lagnas_valid != 0,
		OuterPlanetsValid:          out.outer_planets_valid != 0,
	}
	for i := 0; i < GrahaCount; i++ {
		res.Grahas[i] = goAmshaEntry(out.grahas[i])
	}
	if res.OuterPlanetsValid {
		res.OuterPlanets = goAmshaEntries(out.outer_planets[:])
	}
	if res.BhavaCuspsValid {
		res.BhavaCusps = goAmshaEntries(out.bhava_cusps[:])
	}
	if res.RashiBhavaCuspsValid {
		res.RashiBhavaCusps = goAmshaEntries(out.rashi_bhava_cusps[:])
	}
	if res.ArudhaPadasValid {
		res.ArudhaPadas = goAmshaEntries(out.arudha_padas[:])
	}
	if res.RashiBhavaArudhaPadasValid {
		res.RashiBhavaArudhaPadas = goAmshaEntries(out.rashi_bhava_arudha_padas[:])
	}
	if res.UpagrahasValid {
		res.Upagrahas = goAmshaEntries(out.upagrahas[:])
	}
	if res.SphutasValid {
		res.Sphutas = goAmshaEntries(out.sphutas[:])
	}
	if res.SpecialLagnasValid {
		res.SpecialLagnas = goAmshaEntries(out.special_lagnas[:])
	}
	return res, st
}

func AmshaVariations(amshaCode uint16) (AmshaVariationCatalog, Status) {
	var out C.DhruvAmshaVariationList
	st := Status(C.dhruv_amsha_variations(C.uint16_t(amshaCode), &out))
	return goAmshaVariationCatalog(out), st
}

func AmshaVariationsMany(amshaCodes []uint16) ([]AmshaVariationCatalog, Status, error) {
	var out C.DhruvAmshaVariationCatalogs
	if len(amshaCodes) == 0 {
		st := Status(C.dhruv_amsha_variations_many(nil, 0, &out))
		return []AmshaVariationCatalog{}, st, nil
	}
	codes := make([]C.uint16_t, len(amshaCodes))
	for i, code := range amshaCodes {
		codes[i] = C.uint16_t(code)
	}
	st := Status(C.dhruv_amsha_variations_many(&codes[0], C.uint32_t(len(codes)), &out))
	res := make([]AmshaVariationCatalog, int(out.count))
	for i := 0; i < int(out.count); i++ {
		res[i] = goAmshaVariationCatalog(out.lists[i])
	}
	return res, st, nil
}
