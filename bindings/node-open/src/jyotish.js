'use strict';

const { addon } = require('./native');
const { checkStatus } = require('./errors');

const CHARAKARAKA_SCHEME = Object.freeze({
  EIGHT: 0,
  SEVEN_NO_PITRI: 1,
  SEVEN_PK_MERGED_MK: 2,
  MIXED_PARASHARA: 3,
});

const CHARAKARAKA_ROLE = Object.freeze({
  ATMA: 0,
  AMATYA: 1,
  BHRATRI: 2,
  MATRI: 3,
  PITRI: 4,
  PUTRA: 5,
  GNATI: 6,
  DARA: 7,
  MATRI_PUTRA: 8,
});

const GRAHA_LONGITUDE_KIND = Object.freeze({
  SIDEREAL: 0,
  TROPICAL: 1,
});

const PRECESSION_MODEL = Object.freeze({
  NEWCOMB1895: 0,
  LIESKE1977: 1,
  IAU2006: 2,
  VONDRAK2011: 3,
});

function normalizeCharakarakaScheme(scheme) {
  if (typeof scheme === 'number') {
    return scheme >>> 0;
  }
  if (typeof scheme === 'string') {
    const k = scheme.trim().toLowerCase().replaceAll('_', '-');
    if (k === 'eight' || k === '8' || k === '8-charakaraka' || k === 'jaimini' || k === 'jaimini-8') {
      return CHARAKARAKA_SCHEME.EIGHT;
    }
    if (k === 'seven-no-pitri' || k === '7-no-pitri' || k === '7-planet' || k === 'seven-planet') {
      return CHARAKARAKA_SCHEME.SEVEN_NO_PITRI;
    }
    if (k === 'seven-pk-merged-mk' || k === '7-pk-merged-mk' || k === 'pk-merged-mk') {
      return CHARAKARAKA_SCHEME.SEVEN_PK_MERGED_MK;
    }
    if (k === 'mixed-parashara' || k === 'mixed' || k === 'parashari' || k === 'parashara' || k === '7-8-parashara') {
      return CHARAKARAKA_SCHEME.MIXED_PARASHARA;
    }
  }
  throw new Error(`invalid charakaraka scheme: ${scheme}`);
}

function grahaLongitudes(engine, jdTdb, config = undefined) {
  const r = config === undefined
    ? addon.grahaLongitudes(engine._handle, jdTdb)
    : addon.grahaLongitudes(engine._handle, jdTdb, config);
  checkStatus('graha_longitudes', r.status);
  return r.longitudes;
}

function specialLagnasForDate(engine, eop, utc, location, risesetConfig, ayanamshaSystem = 0, useNutation = true) {
  const r = addon.specialLagnasForDate(engine._handle, eop._handle, utc, location, risesetConfig, ayanamshaSystem, !!useNutation);
  checkStatus('special_lagnas_for_date', r.status);
  return r.lagnas;
}

function arudhaPadasForDate(engine, eop, utc, location, ayanamshaSystem = 0, useNutation = true) {
  const r = addon.arudhaPadasForDate(engine._handle, eop._handle, utc, location, ayanamshaSystem, !!useNutation);
  checkStatus('arudha_padas_for_date', r.status);
  return r.results;
}

function allUpagrahasForDate(engine, eop, utc, location, ayanamshaSystem = 0, useNutation = true, upagrahaConfig = undefined) {
  const r = upagrahaConfig === undefined
    ? addon.allUpagrahasForDate(engine._handle, eop._handle, utc, location, ayanamshaSystem, !!useNutation)
    : addon.allUpagrahasForDate(engine._handle, eop._handle, utc, location, ayanamshaSystem, !!useNutation, upagrahaConfig);
  checkStatus(upagrahaConfig === undefined ? 'all_upagrahas_for_date' : 'all_upagrahas_for_date_with_config', r.status);
  return r.upagrahas;
}

function charakarakaForDate(
  engine,
  eop,
  utc,
  ayanamshaSystem = 0,
  useNutation = true,
  scheme = CHARAKARAKA_SCHEME.EIGHT,
) {
  const schemeCode = normalizeCharakarakaScheme(scheme);
  const r = addon.charakarakaForDate(
    engine._handle,
    eop._handle,
    utc,
    ayanamshaSystem,
    !!useNutation,
    schemeCode,
  );
  checkStatus('charakaraka_for_date', r.status);
  return r.result;
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

function rashiFromTropical(tropicalLongitudeDeg, ayanamshaSystem, jdTdb, useNutation = true) {
  const r = addon.rashiFromTropical(tropicalLongitudeDeg, ayanamshaSystem, jdTdb, !!useNutation);
  checkStatus('rashi_from_tropical', r.status);
  return r.rashi;
}

function nakshatraFromTropical(tropicalLongitudeDeg, ayanamshaSystem, jdTdb, useNutation = true) {
  const r = addon.nakshatraFromTropical(tropicalLongitudeDeg, ayanamshaSystem, jdTdb, !!useNutation);
  checkStatus('nakshatra_from_tropical', r.status);
  return r.nakshatra;
}

function nakshatra28FromTropical(tropicalLongitudeDeg, ayanamshaSystem, jdTdb, useNutation = true) {
  const r = addon.nakshatra28FromTropical(tropicalLongitudeDeg, ayanamshaSystem, jdTdb, !!useNutation);
  checkStatus('nakshatra28_from_tropical', r.status);
  return r.nakshatra28;
}

function rashiFromTropicalUtc(lsk, tropicalLongitudeDeg, ayanamshaSystem, utc, useNutation = true) {
  const r = addon.rashiFromTropicalUtc(lsk._handle, tropicalLongitudeDeg, ayanamshaSystem, utc, !!useNutation);
  checkStatus('rashi_from_tropical_utc', r.status);
  return r.rashi;
}

function nakshatraFromTropicalUtc(lsk, tropicalLongitudeDeg, ayanamshaSystem, utc, useNutation = true) {
  const r = addon.nakshatraFromTropicalUtc(lsk._handle, tropicalLongitudeDeg, ayanamshaSystem, utc, !!useNutation);
  checkStatus('nakshatra_from_tropical_utc', r.status);
  return r.nakshatra;
}

function nakshatra28FromTropicalUtc(lsk, tropicalLongitudeDeg, ayanamshaSystem, utc, useNutation = true) {
  const r = addon.nakshatra28FromTropicalUtc(lsk._handle, tropicalLongitudeDeg, ayanamshaSystem, utc, !!useNutation);
  checkStatus('nakshatra28_from_tropical_utc', r.status);
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
function yoginiName(index) { return addon.yoginiName(index); }
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
  GRAHA_LONGITUDE_KIND,
  PRECESSION_MODEL,
  grahaLongitudes,
  specialLagnasForDate,
  arudhaPadasForDate,
  allUpagrahasForDate,
  charakarakaForDate,
  CHARAKARAKA_SCHEME,
  CHARAKARAKA_ROLE,
  rashiCount,
  nakshatraCount,
  rashiFromLongitude,
  nakshatraFromLongitude,
  nakshatra28FromLongitude,
  rashiFromTropical,
  nakshatraFromTropical,
  nakshatra28FromTropical,
  rashiFromTropicalUtc,
  nakshatraFromTropicalUtc,
  nakshatra28FromTropicalUtc,
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
  yoginiName,
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
