'use strict';

const { addon } = require('./native');
const { checkStatus } = require('./errors');

function grahaSiderealLongitudes(engine, jdTdb, ayanamshaSystem = 0, useNutation = true) {
  const r = addon.grahaSiderealLongitudes(engine._handle, jdTdb, ayanamshaSystem, !!useNutation);
  checkStatus('graha_sidereal_longitudes', r.status);
  return r.longitudes;
}

function grahaTropicalLongitudes(engine, jdTdb) {
  const r = addon.grahaTropicalLongitudes(engine._handle, jdTdb);
  checkStatus('graha_tropical_longitudes', r.status);
  return r.longitudes;
}

function rashiCount() {
  return addon.rashiCount();
}

function nakshatraCount(schemeCode = 27) {
  return addon.nakshatraCount(schemeCode);
}

function rashiFromLongitude(siderealLongitudeDeg) {
  const r = addon.rashiFromLongitude(siderealLongitudeDeg);
  checkStatus('rashi_from_longitude', r.status);
  return r.rashi;
}

function nakshatraFromLongitude(siderealLongitudeDeg) {
  const r = addon.nakshatraFromLongitude(siderealLongitudeDeg);
  checkStatus('nakshatra_from_longitude', r.status);
  return r.nakshatra;
}

function nakshatra28FromLongitude(siderealLongitudeDeg) {
  const r = addon.nakshatra28FromLongitude(siderealLongitudeDeg);
  checkStatus('nakshatra28_from_longitude', r.status);
  return r.nakshatra28;
}

function degToDms(degrees) {
  const r = addon.degToDms(degrees);
  checkStatus('deg_to_dms', r.status);
  return r.dms;
}

function tithiFromElongation(elongationDeg) {
  const r = addon.tithiFromElongation(elongationDeg);
  checkStatus('tithi_from_elongation', r.status);
  return r.tithiPosition;
}

function karanaFromElongation(elongationDeg) {
  const r = addon.karanaFromElongation(elongationDeg);
  checkStatus('karana_from_elongation', r.status);
  return r.karanaPosition;
}

function yogaFromSum(sumDeg) {
  const r = addon.yogaFromSum(sumDeg);
  checkStatus('yoga_from_sum', r.status);
  return r.yogaPosition;
}

function samvatsaraFromYear(year) {
  const r = addon.samvatsaraFromYear(year);
  checkStatus('samvatsara_from_year', r.status);
  return r.samvatsara;
}

function rashiName(index) { return addon.rashiName(index); }
function nakshatraName(index) { return addon.nakshatraName(index); }
function nakshatra28Name(index) { return addon.nakshatra28Name(index); }
function masaName(index) { return addon.masaName(index); }
function ayanaName(index) { return addon.ayanaName(index); }
function samvatsaraName(index) { return addon.samvatsaraName(index); }
function tithiName(index) { return addon.tithiName(index); }
function karanaName(index) { return addon.karanaName(index); }
function yogaName(index) { return addon.yogaName(index); }
function vaarName(index) { return addon.vaarName(index); }
function horaName(index) { return addon.horaName(index); }
function grahaName(index) { return addon.grahaName(index); }
function grahaEnglishName(index) { return addon.grahaEnglishName(index); }
function sphutaName(index) { return addon.sphutaName(index); }
function specialLagnaName(index) { return addon.specialLagnaName(index); }
function arudhaPadaName(index) { return addon.arudhaPadaName(index); }
function upagrahaName(index) { return addon.upagrahaName(index); }

function vaarFromJd(jd) { return addon.vaarFromJd(jd); }
function masaFromRashiIndex(rashiIndex) { return addon.masaFromRashiIndex(rashiIndex); }
function ayanaFromSiderealLongitude(lonDeg) { return addon.ayanaFromSiderealLongitude(lonDeg); }
function nthRashiFrom(rashiIndex, offset) { return addon.nthRashiFrom(rashiIndex, offset); }
function rashiLord(rashiIndex) { return addon.rashiLord(rashiIndex); }
function horaAt(vaarIndex, horaIndex) { return addon.horaAt(vaarIndex, horaIndex); }

module.exports = {
  grahaSiderealLongitudes,
  grahaTropicalLongitudes,
  rashiCount,
  nakshatraCount,
  rashiFromLongitude,
  nakshatraFromLongitude,
  nakshatra28FromLongitude,
  degToDms,
  tithiFromElongation,
  karanaFromElongation,
  yogaFromSum,
  samvatsaraFromYear,
  rashiName,
  nakshatraName,
  nakshatra28Name,
  masaName,
  ayanaName,
  samvatsaraName,
  tithiName,
  karanaName,
  yogaName,
  vaarName,
  horaName,
  grahaName,
  grahaEnglishName,
  sphutaName,
  specialLagnaName,
  arudhaPadaName,
  upagrahaName,
  vaarFromJd,
  masaFromRashiIndex,
  ayanaFromSiderealLongitude,
  nthRashiFrom,
  rashiLord,
  horaAt,
};
