'use strict';

const { addon } = require('./native');
const { checkStatus } = require('./errors');

function normalizeAmshaSelection(amshaSelection) {
  if (!amshaSelection) {
    return addon.fullKundaliConfigDefault().amshaSelection;
  }

  const normalized = addon.fullKundaliConfigDefault().amshaSelection;
  normalized.count = amshaSelection.count ?? 0;

  if (Array.isArray(amshaSelection.codes)) {
    for (let i = 0; i < amshaSelection.codes.length && i < normalized.codes.length; i += 1) {
      normalized.codes[i] = amshaSelection.codes[i];
    }
  }

  if (Array.isArray(amshaSelection.variations)) {
    for (let i = 0; i < amshaSelection.variations.length && i < normalized.variations.length; i += 1) {
      normalized.variations[i] = amshaSelection.variations[i];
    }
  }

  return normalized;
}

function normalizeFullKundaliConfig(config) {
  const normalized = {
    ...addon.fullKundaliConfigDefault(),
    ...config,
  };
  normalized.amshaSelection = normalizeAmshaSelection(normalized.amshaSelection);
  return normalized;
}

function shadbalaForDate(
  engine,
  eop,
  utc,
  location,
  ayanamshaSystem = 0,
  useNutation = true,
  bhavaConfig = addon.bhavaConfigDefault(),
  riseSetConfig = addon.riseSetConfigDefault(),
  amshaSelection = addon.fullKundaliConfigDefault().amshaSelection,
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
    normalizeAmshaSelection(amshaSelection),
  );
  checkStatus('shadbala_for_date', r.status);
  return r.result;
}

function calculateBhavaBala(inputs) {
  const r = addon.calculateBhavaBala(inputs);
  checkStatus('calculate_bhavabala', r.status);
  return r.result;
}

function bhavaBalaForDate(
  engine,
  eop,
  utc,
  location,
  ayanamshaSystem = 0,
  useNutation = true,
  bhavaConfig = addon.bhavaConfigDefault(),
  riseSetConfig = addon.riseSetConfigDefault(),
) {
  const r = addon.bhavaBalaForDate(
    engine._handle,
    eop._handle,
    utc,
    location,
    ayanamshaSystem,
    !!useNutation,
    bhavaConfig,
    riseSetConfig,
  );
  checkStatus('bhavabala_for_date', r.status);
  return r.result;
}

function vimsopakaForDate(engine, eop, utc, location, ayanamshaSystem = 0, useNutation = true, nodeDignityPolicy = 0, amshaSelection = addon.fullKundaliConfigDefault().amshaSelection) {
  const r = addon.vimsopakaForDate(
    engine._handle,
    eop._handle,
    utc,
    location,
    ayanamshaSystem,
    !!useNutation,
    nodeDignityPolicy,
    normalizeAmshaSelection(amshaSelection),
  );
  checkStatus('vimsopaka_for_date', r.status);
  return r.result;
}

function balasForDate(
  engine,
  eop,
  utc,
  location,
  bhavaConfig = addon.bhavaConfigDefault(),
  riseSetConfig = addon.riseSetConfigDefault(),
  ayanamshaSystem = 0,
  useNutation = true,
  nodeDignityPolicy = 0,
  amshaSelection = addon.fullKundaliConfigDefault().amshaSelection,
) {
  const r = addon.balasForDate(
    engine._handle,
    eop._handle,
    utc,
    location,
    bhavaConfig,
    riseSetConfig,
    ayanamshaSystem,
    !!useNutation,
    nodeDignityPolicy,
    normalizeAmshaSelection(amshaSelection),
  );
  checkStatus('balas_for_date', r.status);
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
  amshaSelection = addon.fullKundaliConfigDefault().amshaSelection,
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
    normalizeAmshaSelection(amshaSelection),
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

function fullKundaliConfigDefault() {
  return addon.fullKundaliConfigDefault();
}

function fullKundaliForDate(
  engine,
  eop,
  utc,
  location,
  bhavaConfig = addon.bhavaConfigDefault(),
  riseSetConfig = addon.riseSetConfigDefault(),
  ayanamshaSystem = 0,
  useNutation = true,
  config = addon.fullKundaliConfigDefault(),
) {
  const r = addon.fullKundaliForDate(
    engine._handle,
    eop._handle,
    utc,
    location,
    bhavaConfig,
    riseSetConfig,
    ayanamshaSystem,
    !!useNutation,
    normalizeFullKundaliConfig(config),
  );
  checkStatus('full_kundali_for_date', r.status);
  return r.result;
}

module.exports = {
  calculateBhavaBala,
  shadbalaForDate,
  bhavaBalaForDate,
  vimsopakaForDate,
  balasForDate,
  avasthaForDate,
  fullKundaliConfigDefault,
  fullKundaliForDate,
  fullKundaliSummaryForDate,
};
