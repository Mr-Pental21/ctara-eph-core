'use strict';

const { addon } = require('./native');
const { checkStatus } = require('./errors');

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

function yogaForDate(engine, utc) {
  const r = addon.yogaForDate(engine._handle, utc);
  checkStatus('yoga_for_date', r.status);
  return r.yoga;
}

function nakshatraForDate(engine, utc) {
  const r = addon.nakshatraForDate(engine._handle, utc);
  checkStatus('nakshatra_for_date', r.status);
  return r.nakshatra;
}

function vaarForDate(engine, eop, utc, location) {
  const r = addon.vaarForDate(engine._handle, eop._handle, utc, location);
  checkStatus('vaar_for_date', r.status);
  return r.vaar;
}

function horaForDate(engine, eop, utc, location) {
  const r = addon.horaForDate(engine._handle, eop._handle, utc, location);
  checkStatus('hora_for_date', r.status);
  return r.hora;
}

function ghatikaForDate(engine, eop, utc, location) {
  const r = addon.ghatikaForDate(engine._handle, eop._handle, utc, location);
  checkStatus('ghatika_for_date', r.status);
  return r.ghatika;
}

function masaForDate(engine, utc) {
  const r = addon.masaForDate(engine._handle, utc);
  checkStatus('masa_for_date', r.status);
  return r.masa;
}

function ayanaForDate(engine, utc) {
  const r = addon.ayanaForDate(engine._handle, utc);
  checkStatus('ayana_for_date', r.status);
  return r.ayana;
}

function varshaForDate(engine, utc) {
  const r = addon.varshaForDate(engine._handle, utc);
  checkStatus('varsha_for_date', r.status);
  return r.varsha;
}

module.exports = {
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
};
