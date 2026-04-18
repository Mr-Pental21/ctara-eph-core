'use strict';

const { addon } = require('./native');
const { checkStatus } = require('./errors');

function elongationAt(engine, jdTdb) {
  const r = addon.elongationAt(engine._handle, jdTdb);
  checkStatus('elongation_at', r.status);
  return r.value;
}

function siderealSumAt(engine, jdTdb, config) {
  const r = addon.siderealSumAt(engine._handle, jdTdb, config);
  checkStatus('sidereal_sum_at', r.status);
  return r.value;
}

function vedicDaySunrises(engine, eop, utc, location, config) {
  const r = addon.vedicDaySunrises(engine._handle, eop._handle, utc, location, config);
  checkStatus('vedic_day_sunrises', r.status);
  return { sunriseJd: r.sunriseJd, nextSunriseJd: r.nextSunriseJd };
}

function bodyEclipticLonLat(engine, bodyCode, jdTdb) {
  const r = addon.bodyEclipticLonLat(engine._handle, bodyCode, jdTdb);
  checkStatus('body_ecliptic_lon_lat', r.status);
  return { lonDeg: r.lonDeg, latDeg: r.latDeg };
}

function tithiAt(engine, jdTdb, sunriseJd) {
  const r = addon.tithiAt(engine._handle, jdTdb, sunriseJd);
  checkStatus('tithi_at', r.status);
  return r.tithi;
}

function karanaAt(engine, jdTdb, sunriseJd) {
  const r = addon.karanaAt(engine._handle, jdTdb, sunriseJd);
  checkStatus('karana_at', r.status);
  return r.karana;
}

function yogaAt(engine, jdTdb, sunriseJd, config) {
  const r = addon.yogaAt(engine._handle, jdTdb, sunriseJd, config);
  checkStatus('yoga_at', r.status);
  return r.yoga;
}

function vaarFromSunrises(lsk, sunriseJd, nextSunriseJd) {
  const r = addon.vaarFromSunrises(lsk._handle, sunriseJd, nextSunriseJd);
  checkStatus('vaar_from_sunrises', r.status);
  return r.vaar;
}

function horaFromSunrises(lsk, queryJd, sunriseJd, nextSunriseJd) {
  const r = addon.horaFromSunrises(lsk._handle, queryJd, sunriseJd, nextSunriseJd);
  checkStatus('hora_from_sunrises', r.status);
  return r.hora;
}

function ghatikaFromSunrises(lsk, queryJd, sunriseJd, nextSunriseJd) {
  const r = addon.ghatikaFromSunrises(lsk._handle, queryJd, sunriseJd, nextSunriseJd);
  checkStatus('ghatika_from_sunrises', r.status);
  return r.ghatika;
}

function nakshatraAt(engine, jdTdb, moonSiderealDeg, config) {
  const r = addon.nakshatraAt(engine._handle, jdTdb, moonSiderealDeg, config);
  checkStatus('nakshatra_at', r.status);
  return r.nakshatra;
}

function ghatikaFromElapsed(queryJd, sunriseJd, nextSunriseJd) {
  const r = addon.ghatikaFromElapsed(queryJd, sunriseJd, nextSunriseJd);
  checkStatus('ghatika_from_elapsed', r.status);
  return r.value;
}

function ghatikasSinceSunrise(queryJd, sunriseJd) {
  const r = addon.ghatikasSinceSunrise(queryJd, sunriseJd);
  checkStatus('ghatikas_since_sunrise', r.status);
  return r.value;
}

function allSphutas(inputs) {
  const r = addon.allSphutas(inputs);
  checkStatus('all_sphutas', r.status);
  return r.result;
}

function bhriguBindu(rahu, moon) { return addon.bhriguBindu(rahu, moon); }
function pranaSphuta(lagna, moon) { return addon.pranaSphuta(lagna, moon); }
function dehaSphuta(moon, lagna) { return addon.dehaSphuta(moon, lagna); }
function mrityuSphuta(eighthLord, lagna) { return addon.mrityuSphuta(eighthLord, lagna); }
function tithiSphuta(moon, sun, lagna) { return addon.tithiSphuta(moon, sun, lagna); }
function yogaSphuta(sun, moon) { return addon.yogaSphuta(sun, moon); }
function yogaSphutaNormalized(sun, moon) { return addon.yogaSphutaNormalized(sun, moon); }
function rahuTithiSphuta(rahu, sun, lagna) { return addon.rahuTithiSphuta(rahu, sun, lagna); }
function kshetraSphuta(moon, mars, jupiter, venus, lagna) { return addon.kshetraSphuta(moon, mars, jupiter, venus, lagna); }
function beejaSphuta(sun, venus, jupiter) { return addon.beejaSphuta(sun, venus, jupiter); }
function trisphuta(lagna, moon, gulika) { return addon.trisphuta(lagna, moon, gulika); }
function chatussphuta(trisphutaVal, sun) { return addon.chatussphuta(trisphutaVal, sun); }
function panchasphuta(chatussphutaVal, rahu) { return addon.panchasphuta(chatussphutaVal, rahu); }
function sookshmaTrisphuta(lagna, moon, gulika, sun) { return addon.sookshmaTrisphuta(lagna, moon, gulika, sun); }
function avayogaSphuta(sun, moon) { return addon.avayogaSphuta(sun, moon); }
function kunda(lagna, moon, mars) { return addon.kunda(lagna, moon, mars); }
function bhavaLagna(sunLon, ghatikas) { return addon.bhavaLagna(sunLon, ghatikas); }
function horaLagna(sunLon, ghatikas) { return addon.horaLagna(sunLon, ghatikas); }
function ghatiLagna(sunLon, ghatikas) { return addon.ghatiLagna(sunLon, ghatikas); }
function vighatiLagna(lagnaLon, vighatikas) { return addon.vighatiLagna(lagnaLon, vighatikas); }
function varnadaLagna(lagnaLon, horaLagnaLon) { return addon.varnadaLagna(lagnaLon, horaLagnaLon); }
function sreeLagna(moonLon, lagnaLon) { return addon.sreeLagna(moonLon, lagnaLon); }
function pranapadaLagna(sunLon, ghatikas) { return addon.pranapadaLagna(sunLon, ghatikas); }
function induLagna(moonLon, lagnaLord, moon9thLord) { return addon.induLagna(moonLon, lagnaLord, moon9thLord); }

function arudhaPada(bhavaCuspLon, lordLon, rashiHint = 0) {
  const r = addon.arudhaPada(bhavaCuspLon, lordLon, rashiHint);
  checkStatus('arudha_pada', r.status);
  return r.result;
}

function sunBasedUpagrahas(engineOrSunSiderealLongitude, jdTdb, ayanamshaSystem = 0, useNutation = true) {
  let sunSiderealLongitude = engineOrSunSiderealLongitude;
  if (engineOrSunSiderealLongitude && engineOrSunSiderealLongitude._handle) {
    const sid = addon.grahaLongitudes(engineOrSunSiderealLongitude._handle, jdTdb, {
      kind: 0,
      ayanamshaSystem,
      useNutation: !!useNutation,
    });
    checkStatus('graha_longitudes', sid.status);
    sunSiderealLongitude = sid.longitudes[0];
  }
  const r = addon.sunBasedUpagrahas(sunSiderealLongitude);
  checkStatus('sun_based_upagrahas', r.status);
  return r.result;
}

function timeUpagrahaJd(upagrahaIndex, weekday, isDay, sunriseJd, sunsetJd, nextSunriseJd, upagrahaConfig = undefined) {
  const r = upagrahaConfig === undefined
    ? addon.timeUpagrahaJd(upagrahaIndex, weekday, !!isDay, sunriseJd, sunsetJd, nextSunriseJd)
    : addon.timeUpagrahaJd(upagrahaIndex, weekday, !!isDay, sunriseJd, sunsetJd, nextSunriseJd, upagrahaConfig);
  checkStatus(upagrahaConfig === undefined ? 'time_upagraha_jd' : 'time_upagraha_jd_with_config', r.status);
  return r.jdTdb;
}

function timeUpagrahaJdUtc(engine, eop, utc, location, riseSetConfig, upagrahaIndex, upagrahaConfig = undefined) {
  const r = upagrahaConfig === undefined
    ? addon.timeUpagrahaJdUtc(
      engine._handle,
      eop._handle,
      utc,
      location,
      riseSetConfig,
      upagrahaIndex,
    )
    : addon.timeUpagrahaJdUtc(
      engine._handle,
      eop._handle,
      utc,
      location,
      riseSetConfig,
      upagrahaIndex,
      upagrahaConfig,
    );
  checkStatus(upagrahaConfig === undefined ? 'time_upagraha_jd_utc' : 'time_upagraha_jd_utc_with_config', r.status);
  return r.jdTdb;
}

function calculateAshtakavarga(grahaRashis, lagnaRashi) {
  const r = addon.calculateAshtakavarga(grahaRashis, lagnaRashi);
  checkStatus('calculate_ashtakavarga', r.status);
  return r.result;
}

function calculateBav(grahaIndex, grahaRashis, lagnaRashi) {
  const r = addon.calculateBav(grahaIndex, grahaRashis, lagnaRashi);
  checkStatus('calculate_bav', r.status);
  return r.result;
}

function calculateAllBav(grahaRashis, lagnaRashi) {
  const r = addon.calculateAllBav(grahaRashis, lagnaRashi);
  checkStatus('calculate_all_bav', r.status);
  return r.results;
}

function calculateSav(bavs) {
  const r = addon.calculateSav(bavs);
  checkStatus('calculate_sav', r.status);
  return r.result;
}

function trikonaSodhana(totals) {
  const r = addon.trikonaSodhana(totals);
  checkStatus('trikona_sodhana', r.status);
  return r.result;
}

function ekadhipatyaSodhana(totals, grahaRashis, lagnaRashi) {
  const r = addon.ekadhipatyaSodhana(totals, grahaRashis, lagnaRashi);
  checkStatus('ekadhipatya_sodhana', r.status);
  return r.result;
}

function ashtakavargaForDate(engine, eop, utc, location, ayanamshaSystem = 0, useNutation = true) {
  const r = addon.ashtakavargaForDate(engine._handle, eop._handle, utc, location, ayanamshaSystem, !!useNutation);
  checkStatus('ashtakavarga_for_date', r.status);
  return r.result;
}

function grahaDrishti(grahaIndex, sourceLon, targetLon) {
  const r = addon.grahaDrishti(grahaIndex, sourceLon, targetLon);
  checkStatus('graha_drishti', r.status);
  return r.result;
}

function grahaDrishtiMatrixForLongitudes(siderealLongitudes) {
  const r = addon.grahaDrishtiMatrix(siderealLongitudes);
  checkStatus('graha_drishti_matrix', r.status);
  return r.result;
}

function drishtiForDate(engine, eop, utc, location, bhavaConfig, riseSetConfig, ayanamshaSystem = 0, useNutation = true, config) {
  const r = addon.drishtiForDate(
    engine._handle,
    eop._handle,
    utc,
    location,
    bhavaConfig,
    riseSetConfig,
    ayanamshaSystem,
    !!useNutation,
    config,
  );
  checkStatus('drishti', r.status);
  return r.result;
}

function grahaPositionsForDate(engine, eop, utc, location, bhavaConfig, ayanamshaSystem = 0, useNutation = true, config) {
  const r = addon.grahaPositionsForDate(
    engine._handle,
    eop._handle,
    utc,
    location,
    bhavaConfig,
    ayanamshaSystem,
    !!useNutation,
    config,
  );
  checkStatus('graha_positions', r.status);
  return r.result;
}

function horaLord(vaarIndex, horaIndex) {
  return addon.horaLord(vaarIndex, horaIndex).grahaIndex;
}

function masaLord(masaIndex) {
  return addon.masaLord(masaIndex).grahaIndex;
}

function samvatsaraLord(samvatsaraIndex) {
  return addon.samvatsaraLord(samvatsaraIndex).grahaIndex;
}

function exaltationDegree(grahaIndex) {
  const r = addon.exaltationDegree(grahaIndex);
  checkStatus('exaltation_degree', r.status);
  return r.hasValue ? r.value : null;
}

function debilitationDegree(grahaIndex) {
  const r = addon.debilitationDegree(grahaIndex);
  checkStatus('debilitation_degree', r.status);
  return r.hasValue ? r.value : null;
}

function moolatrikoneRange(grahaIndex) {
  const r = addon.moolatrikoneRange(grahaIndex);
  checkStatus('moolatrikone_range', r.status);
  return r.hasValue ? { rashiIndex: r.rashiIndex, startDeg: r.startDeg, endDeg: r.endDeg } : null;
}

function combustionThreshold(grahaIndex, isRetrograde = false) {
  const r = addon.combustionThreshold(grahaIndex, !!isRetrograde);
  checkStatus('combustion_threshold', r.status);
  return r.hasValue ? r.value : null;
}

function isCombust(grahaIndex, grahaSidLon, sunSidLon, isRetrograde = false) {
  const r = addon.isCombust(grahaIndex, grahaSidLon, sunSidLon, !!isRetrograde);
  checkStatus('is_combust', r.status);
  return r.value;
}

function allCombustionStatus(siderealLons9, retrogradeFlags9) {
  const r = addon.allCombustionStatus(siderealLons9, retrogradeFlags9);
  checkStatus('all_combustion_status', r.status);
  return r.result;
}

function naisargikaMaitri(grahaIndex, otherIndex) {
  const r = addon.naisargikaMaitri(grahaIndex, otherIndex);
  checkStatus('naisargika_maitri', r.status);
  return r.code;
}

function tatkalikaMaitri(grahaRashiIndex, otherRashiIndex) {
  const r = addon.tatkalikaMaitri(grahaRashiIndex, otherRashiIndex);
  checkStatus('tatkalika_maitri', r.status);
  return r.code;
}

function panchadhaMaitri(naisargikaCode, tatkalikaCode) {
  const r = addon.panchadhaMaitri(naisargikaCode, tatkalikaCode);
  checkStatus('panchadha_maitri', r.status);
  return r.code;
}

function dignityInRashi(grahaIndex, siderealLon, rashiIndex) {
  const r = addon.dignityInRashi(grahaIndex, siderealLon, rashiIndex);
  checkStatus('dignity_in_rashi', r.status);
  return r.code;
}

function dignityInRashiWithPositions(grahaIndex, siderealLon, rashiIndex, saptaRashiIndices) {
  const r = addon.dignityInRashiWithPositions(grahaIndex, siderealLon, rashiIndex, saptaRashiIndices);
  checkStatus('dignity_in_rashi_with_positions', r.status);
  return r.code;
}

function nodeDignityInRashi(grahaIndex, rashiIndex, grahaRashiIndices9, policyCode) {
  const r = addon.nodeDignityInRashi(grahaIndex, rashiIndex, grahaRashiIndices9, policyCode);
  checkStatus('node_dignity_in_rashi', r.status);
  return r.code;
}

function naturalBeneficMalefic(grahaIndex) {
  const r = addon.naturalBeneficMalefic(grahaIndex);
  checkStatus('natural_benefic_malefic', r.status);
  return r.code;
}

function moonBeneficNature(moonSunElongation) {
  const r = addon.moonBeneficNature(moonSunElongation);
  checkStatus('moon_benefic_nature', r.status);
  return r.code;
}

function grahaGender(grahaIndex) {
  const r = addon.grahaGender(grahaIndex);
  checkStatus('graha_gender', r.status);
  return r.code;
}

function coreBindusForDate(engine, eop, utc, location, bhavaConfig, riseSetConfig, ayanamshaSystem = 0, useNutation = true, config) {
  const r = addon.coreBindusForDate(
    engine._handle,
    eop._handle,
    utc,
    location,
    bhavaConfig,
    riseSetConfig,
    ayanamshaSystem,
    !!useNutation,
    config,
  );
  checkStatus('core_bindus', r.status);
  return r.result;
}

function amshaLongitude(siderealLon, amshaCode, variationCode) {
  const r = addon.amshaLongitude(siderealLon, amshaCode, variationCode);
  checkStatus('amsha_longitude', r.status);
  return r.longitudeDeg;
}

function amshaRashiInfo(siderealLon, amshaCode, variationCode) {
  const r = addon.amshaRashiInfo(siderealLon, amshaCode, variationCode);
  checkStatus('amsha_rashi_info', r.status);
  return r.rashi;
}

function amshaLongitudes(siderealLon, amshaCodes, variationCodes) {
  const r = addon.amshaLongitudes(siderealLon, amshaCodes, variationCodes);
  checkStatus('amsha_longitudes', r.status);
  return r.longitudes;
}

function amshaChartForDate(
  engine,
  eop,
  utc,
  location,
  bhavaConfig,
  riseSetConfig,
  ayanamshaSystem = 0,
  useNutation = true,
  amshaCode,
  variationCode,
  scope,
) {
  const r = addon.amshaChartForDate(
    engine._handle,
    eop._handle,
    utc,
    location,
    bhavaConfig,
    riseSetConfig,
    ayanamshaSystem,
    !!useNutation,
    amshaCode,
    variationCode,
    scope,
  );
  checkStatus('amsha_chart_for_date', r.status);
  return r.result;
}

function amshaVariations(amshaCode) {
  const r = addon.amshaVariations(amshaCode);
  checkStatus('amsha_variations', r.status);
  return r.catalog;
}

function amshaVariationsMany(amshaCodes) {
  const r = addon.amshaVariationsMany(amshaCodes);
  checkStatus('amsha_variations_many', r.status);
  return r.catalogs;
}

module.exports = {
  elongationAt,
  siderealSumAt,
  vedicDaySunrises,
  bodyEclipticLonLat,
  tithiAt,
  karanaAt,
  yogaAt,
  vaarFromSunrises,
  horaFromSunrises,
  ghatikaFromSunrises,
  nakshatraAt,
  ghatikaFromElapsed,
  ghatikasSinceSunrise,
  allSphutas,
  bhriguBindu,
  pranaSphuta,
  dehaSphuta,
  mrityuSphuta,
  tithiSphuta,
  yogaSphuta,
  yogaSphutaNormalized,
  rahuTithiSphuta,
  kshetraSphuta,
  beejaSphuta,
  trisphuta,
  chatussphuta,
  panchasphuta,
  sookshmaTrisphuta,
  avayogaSphuta,
  kunda,
  bhavaLagna,
  horaLagna,
  ghatiLagna,
  vighatiLagna,
  varnadaLagna,
  sreeLagna,
  pranapadaLagna,
  induLagna,
  arudhaPada,
  sunBasedUpagrahas,
  timeUpagrahaJd,
  timeUpagrahaJdUtc,
  calculateAshtakavarga,
  calculateBav,
  calculateAllBav,
  calculateSav,
  trikonaSodhana,
  ekadhipatyaSodhana,
  ashtakavargaForDate,
  grahaDrishti,
  grahaDrishtiMatrixForLongitudes,
  drishtiForDate,
  grahaPositionsForDate,
  horaLord,
  masaLord,
  samvatsaraLord,
  exaltationDegree,
  debilitationDegree,
  moolatrikoneRange,
  combustionThreshold,
  isCombust,
  allCombustionStatus,
  naisargikaMaitri,
  tatkalikaMaitri,
  panchadhaMaitri,
  dignityInRashi,
  dignityInRashiWithPositions,
  nodeDignityInRashi,
  naturalBeneficMalefic,
  moonBeneficNature,
  grahaGender,
  coreBindusForDate,
  amshaLongitude,
  amshaRashiInfo,
  amshaLongitudes,
  amshaChartForDate,
  amshaVariations,
  amshaVariationsMany,
};
