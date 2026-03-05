'use strict';

const { addon } = require('./native');
const { checkStatus } = require('./errors');

function shadbalaForDate(
  engine,
  eop,
  utc,
  location,
  ayanamshaSystem = 0,
  useNutation = true,
  bhavaConfig = addon.bhavaConfigDefault(),
  riseSetConfig = addon.riseSetConfigDefault(),
) {
  const r = addon.shadbalaForDate(
    engine._handle,
    eop._handle,
    utc,
    location,
    ayanamshaSystem,
    !!useNutation,
    bhavaConfig,
    riseSetConfig,
  );
  checkStatus('shadbala_for_date', r.status);
  return r.result;
}

function vimsopakaForDate(engine, eop, utc, location, ayanamshaSystem = 0, useNutation = true, nodeDignityPolicy = 0) {
  const r = addon.vimsopakaForDate(
    engine._handle,
    eop._handle,
    utc,
    location,
    ayanamshaSystem,
    !!useNutation,
    nodeDignityPolicy,
  );
  checkStatus('vimsopaka_for_date', r.status);
  return r.result;
}

function avasthaForDate(
  engine,
  eop,
  utc,
  location,
  bhavaConfig = addon.bhavaConfigDefault(),
  riseSetConfig = addon.riseSetConfigDefault(),
  ayanamshaSystem = 0,
  useNutation = true,
  nodeDignityPolicy = 0,
) {
  const r = addon.avasthaForDate(
    engine._handle,
    eop._handle,
    utc,
    location,
    bhavaConfig,
    riseSetConfig,
    ayanamshaSystem,
    !!useNutation,
    nodeDignityPolicy,
  );
  checkStatus('avastha_for_date', r.status);
  return r.result;
}

function fullKundaliSummaryForDate(engine, eop, utc, location, ayanamshaSystem = 0, useNutation = true) {
  const r = addon.fullKundaliSummaryForDate(
    engine._handle,
    eop._handle,
    utc,
    location,
    ayanamshaSystem,
    !!useNutation,
  );
  checkStatus('full_kundali_for_date', r.status);
  return {
    ayanamshaDeg: r.ayanamshaDeg,
    grahaPositionsValid: !!r.grahaPositionsValid,
    charakarakaValid: !!r.charakarakaValid,
    panchangValid: !!r.panchangValid,
    dashaSnapshotCount: r.dashaSnapshotCount,
  };
}

module.exports = {
  shadbalaForDate,
  vimsopakaForDate,
  avasthaForDate,
  fullKundaliSummaryForDate,
};
