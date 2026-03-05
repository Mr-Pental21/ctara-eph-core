'use strict';

const { addon } = require('./native');
const { checkStatus } = require('./errors');

function conjunctionConfigDefault() {
  return addon.conjunctionConfigDefault();
}

function grahanConfigDefault() {
  return addon.grahanConfigDefault();
}

function stationaryConfigDefault() {
  return addon.stationaryConfigDefault();
}

function lunarPhaseSearch(engine, request, capacity = 8) {
  const r = addon.lunarPhaseSearch(engine._handle, request, capacity);
  checkStatus('lunar_phase_search_ex', r.status);
  return {
    found: !!r.found,
    count: r.count || 0,
    event: r.event || null,
    events: r.events || [],
  };
}

function conjunctionSearch(engine, request, capacity = 8) {
  const r = addon.conjunctionSearch(engine._handle, request, capacity);
  checkStatus('conjunction_search_ex', r.status);
  return {
    found: !!r.found,
    count: r.count || 0,
    event: r.event || null,
    events: r.events || [],
  };
}

function grahanSearch(engine, request, capacity = 8) {
  const r = addon.grahanSearch(engine._handle, request, capacity);
  checkStatus('grahan_search_ex', r.status);
  return {
    found: !!r.found,
    count: r.count || 0,
    chandra: r.chandra || null,
    surya: r.surya || null,
    chandraEvents: r.chandraEvents || [],
    suryaEvents: r.suryaEvents || [],
  };
}

function motionSearch(engine, request, capacity = 8) {
  const r = addon.motionSearch(engine._handle, request, capacity);
  checkStatus('motion_search_ex', r.status);
  return {
    found: !!r.found,
    count: r.count || 0,
    stationary: r.stationary || null,
    maxSpeed: r.maxSpeed || null,
    stationaryEvents: r.stationaryEvents || [],
    maxSpeedEvents: r.maxSpeedEvents || [],
  };
}

function sankrantiSearch(engine, request, capacity = 8) {
  const r = addon.sankrantiSearch(engine._handle, request, capacity);
  checkStatus('sankranti_search_ex', r.status);
  return {
    found: !!r.found,
    count: r.count || 0,
    event: r.event || null,
    events: r.events || [],
  };
}

module.exports = {
  conjunctionConfigDefault,
  grahanConfigDefault,
  stationaryConfigDefault,
  conjunctionSearch,
  grahanSearch,
  motionSearch,
  lunarPhaseSearch,
  sankrantiSearch,
};
