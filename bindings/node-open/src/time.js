'use strict';

const { addon } = require('./native');
const { checkStatus } = require('./errors');

function utcToTdbJd(lsk, utc) {
  const r = addon.utcToTdbJd(lsk._handle, utc);
  checkStatus('utc_to_tdb_jd', r.status);
  return r.jdTdb;
}

function jdTdbToUtc(lsk, jdTdb) {
  const r = addon.jdTdbToUtc(lsk._handle, jdTdb);
  checkStatus('jd_tdb_to_utc', r.status);
  return r.utc;
}

function nutationIau2000b(jdTdb) {
  const r = addon.nutationIau2000b(jdTdb);
  checkStatus('nutation_iau2000b', r.status);
  return { dpsi: r.dpsi, deps: r.deps };
}

function ayanamshaSystemCount() {
  return addon.ayanamshaSystemCount();
}

function referencePlaneDefault(systemCode) {
  return addon.referencePlaneDefault(systemCode);
}

function ayanamshaComputeEx(lsk, request, eop) {
  const r = addon.ayanamshaComputeEx(lsk._handle, request, eop._handle, 0);
  checkStatus('ayanamsha_compute_ex', r.status);
  return r.ayanamshaDeg;
}

function lunarNodeCount() {
  return addon.lunarNodeCount();
}

function lunarNodeDeg(nodeCode, modeCode, jdTdb) {
  const r = addon.lunarNodeDeg(nodeCode, modeCode, jdTdb);
  checkStatus('lunar_node_deg', r.status);
  return r.longitudeDeg;
}

function lunarNodeDegWithEngine(engine, nodeCode, modeCode, jdTdb) {
  const r = addon.lunarNodeDegWithEngine(engine._handle, nodeCode, modeCode, jdTdb);
  checkStatus('lunar_node_deg_with_engine', r.status);
  return r.longitudeDeg;
}

function riseSetConfigDefault() {
  return addon.riseSetConfigDefault();
}

function bhavaConfigDefault() {
  return addon.bhavaConfigDefault();
}

function sankrantiConfigDefault() {
  return addon.sankrantiConfigDefault();
}

module.exports = {
  utcToTdbJd,
  jdTdbToUtc,
  nutationIau2000b,
  ayanamshaSystemCount,
  referencePlaneDefault,
  ayanamshaComputeEx,
  lunarNodeCount,
  lunarNodeDeg,
  lunarNodeDegWithEngine,
  riseSetConfigDefault,
  bhavaConfigDefault,
  sankrantiConfigDefault,
};
