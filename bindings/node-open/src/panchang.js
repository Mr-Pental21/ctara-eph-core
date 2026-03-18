'use strict';

const { addon } = require('./native');
const { checkStatus } = require('./errors');

function bhavaSystemCount() {
  return addon.bhavaSystemCount();
}

function computeRiseSet(engine, eop, location, config, eventCode, jdTdbApprox, lsk) {
  const r = addon.computeRiseSet(engine._handle, eop._handle, location, config, eventCode, jdTdbApprox, lsk._handle);
  checkStatus('compute_rise_set', r.status);
  return r.result;
}

function computeAllEvents(engine, eop, location, config, jdTdbApprox, lsk) {
  const r = addon.computeAllEvents(engine._handle, eop._handle, location, config, jdTdbApprox, lsk._handle);
  checkStatus('compute_all_events', r.status);
  return r.results;
}

function computeRiseSetUtc(engine, eop, lsk, location, eventCode, utc, config) {
  const r = addon.computeRiseSetUtc(engine._handle, eop._handle, lsk._handle, location, eventCode, utc, config);
  checkStatus('compute_rise_set_utc', r.status);
  return r.result;
}

function computeAllEventsUtc(engine, eop, lsk, location, utc, config) {
  const r = addon.computeAllEventsUtc(engine._handle, eop._handle, lsk._handle, location, utc, config);
  checkStatus('compute_all_events_utc', r.status);
  return r.results;
}

function computeBhavas(engine, eop, location, lsk, jdTdb, config) {
  const r = addon.computeBhavas(engine._handle, eop._handle, location, lsk._handle, jdTdb, config);
  checkStatus('compute_bhavas', r.status);
  return r.bhava;
}

function computeBhavasUtc(engine, eop, lsk, location, utc, config) {
  const r = addon.computeBhavasUtc(engine._handle, eop._handle, lsk._handle, location, utc, config);
  checkStatus('compute_bhavas_utc', r.status);
  return r.bhava;
}

function lagnaDeg(lsk, eop, location, jdTdb, config) {
  const r = config == null
    ? addon.lagnaDeg(lsk._handle, eop._handle, location, jdTdb)
    : addon.lagnaDeg(lsk._handle, eop._handle, location, jdTdb, config);
  checkStatus('lagna_deg', r.status);
  return r.degrees;
}

function mcDeg(lsk, eop, location, jdTdb, config) {
  const r = config == null
    ? addon.mcDeg(lsk._handle, eop._handle, location, jdTdb)
    : addon.mcDeg(lsk._handle, eop._handle, location, jdTdb, config);
  checkStatus('mc_deg', r.status);
  return r.degrees;
}

function ramcDeg(lsk, eop, location, jdTdb) {
  const r = addon.ramcDeg(lsk._handle, eop._handle, location, jdTdb);
  checkStatus('ramc_deg', r.status);
  return r.degrees;
}

function lagnaDegUtc(lsk, eop, location, utc, config) {
  const r = config == null
    ? addon.lagnaDegUtc(lsk._handle, eop._handle, location, utc)
    : addon.lagnaDegUtc(lsk._handle, eop._handle, location, utc, config);
  checkStatus('lagna_deg_utc', r.status);
  return r.degrees;
}

function mcDegUtc(lsk, eop, location, utc, config) {
  const r = config == null
    ? addon.mcDegUtc(lsk._handle, eop._handle, location, utc)
    : addon.mcDegUtc(lsk._handle, eop._handle, location, utc, config);
  checkStatus('mc_deg_utc', r.status);
  return r.degrees;
}

function ramcDegUtc(lsk, eop, location, utc) {
  const r = addon.ramcDegUtc(lsk._handle, eop._handle, location, utc);
  checkStatus('ramc_deg_utc', r.status);
  return r.degrees;
}

function riseSetResultToUtc(lsk, riseSetResult) {
  const r = addon.riseSetResultToUtc(lsk._handle, riseSetResult);
  checkStatus('riseset_result_to_utc', r.status);
  return r.utc;
}

function tithiForDate(engine, utc) {
  const r = addon.tithiForDate(engine._handle, utc);
  checkStatus('tithi_for_date', r.status);
  return r.tithi;
}

function karanaForDate(engine, utc) {
  const r = addon.karanaForDate(engine._handle, utc);
  checkStatus('karana_for_date', r.status);
  return r.karana;
}

function yogaForDate(engine, utc, config = addon.sankrantiConfigDefault()) {
  const r = addon.yogaForDate(engine._handle, utc, config);
  checkStatus('yoga_for_date', r.status);
  return r.yoga;
}

function nakshatraForDate(engine, utc, config = addon.sankrantiConfigDefault()) {
  const r = addon.nakshatraForDate(engine._handle, utc, config);
  checkStatus('nakshatra_for_date', r.status);
  return r.nakshatra;
}

function vaarForDate(engine, eop, utc, location, config = addon.riseSetConfigDefault()) {
  const r = addon.vaarForDate(engine._handle, eop._handle, utc, location, config);
  checkStatus('vaar_for_date', r.status);
  return r.vaar;
}

function horaForDate(engine, eop, utc, location, config = addon.riseSetConfigDefault()) {
  const r = addon.horaForDate(engine._handle, eop._handle, utc, location, config);
  checkStatus('hora_for_date', r.status);
  return r.hora;
}

function ghatikaForDate(engine, eop, utc, location, config = addon.riseSetConfigDefault()) {
  const r = addon.ghatikaForDate(engine._handle, eop._handle, utc, location, config);
  checkStatus('ghatika_for_date', r.status);
  return r.ghatika;
}

function masaForDate(engine, utc, config = addon.sankrantiConfigDefault()) {
  const r = addon.masaForDate(engine._handle, utc, config);
  checkStatus('masa_for_date', r.status);
  return r.masa;
}

function ayanaForDate(engine, utc, config = addon.sankrantiConfigDefault()) {
  const r = addon.ayanaForDate(engine._handle, utc, config);
  checkStatus('ayana_for_date', r.status);
  return r.ayana;
}

function varshaForDate(engine, utc, config = addon.sankrantiConfigDefault()) {
  const r = addon.varshaForDate(engine._handle, utc, config);
  checkStatus('varsha_for_date', r.status);
  return r.varsha;
}

function panchangComputeEx(engine, eop, lsk, request) {
  const r = addon.panchangComputeEx(engine._handle, eop._handle, lsk._handle, request);
  checkStatus('panchang_compute_ex', r.status);
  return r.result;
}

module.exports = {
  bhavaSystemCount,
  computeRiseSet,
  computeAllEvents,
  computeRiseSetUtc,
  computeAllEventsUtc,
  computeBhavas,
  computeBhavasUtc,
  lagnaDeg,
  mcDeg,
  ramcDeg,
  lagnaDegUtc,
  mcDegUtc,
  ramcDegUtc,
  riseSetResultToUtc,
  tithiForDate,
  karanaForDate,
  yogaForDate,
  nakshatraForDate,
  vaarForDate,
  horaForDate,
  ghatikaForDate,
  masaForDate,
  ayanaForDate,
  varshaForDate,
  panchangComputeEx,
};
