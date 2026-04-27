'use strict';

const { addon } = require('./native');
const { EXPECTED_API_VERSION } = require('./status');
const { checkStatus } = require('./errors');

class Config {
  constructor(handle) {
    this._handle = handle;
    this._closed = false;
  }

  static load(path = null, defaultsMode = 0) {
    const r = addon.configLoad(path, defaultsMode);
    checkStatus('config_load', r.status);
    return new Config(r.handle);
  }

  close() {
    if (this._closed) return;
    checkStatus('config_free', addon.configFree(this._handle));
    this._closed = true;
    this._handle = null;
  }
}

class LSK {
  constructor(handle) {
    this._handle = handle;
    this._closed = false;
  }

  static load(path) {
    const r = addon.lskLoad(path);
    checkStatus('lsk_load', r.status);
    return new LSK(r.handle);
  }

  close() {
    if (this._closed) return;
    checkStatus('lsk_free', addon.lskFree(this._handle));
    this._closed = true;
    this._handle = null;
  }
}

class EOP {
  constructor(handle) {
    this._handle = handle;
    this._closed = false;
  }

  static load(path) {
    const r = addon.eopLoad(path);
    checkStatus('eop_load', r.status);
    return new EOP(r.handle);
  }

  close() {
    if (this._closed) return;
    checkStatus('eop_free', addon.eopFree(this._handle));
    this._closed = true;
    this._handle = null;
  }
}

class Engine {
  constructor(handle) {
    this._handle = handle;
    this._closed = false;
  }

  static create(config) {
    const r = addon.engineNew(config);
    checkStatus('engine_new', r.status);
    return new Engine(r.handle);
  }

  close() {
    if (this._closed) return;
    checkStatus('engine_free', addon.engineFree(this._handle));
    this._closed = true;
    this._handle = null;
  }

  query(query) {
    const r = addon.engineQuery(this._handle, query);
    checkStatus('engine_query', r.status);
    return r.result;
  }

  replaceSpks(spkPaths) {
    const r = addon.engineReplaceSpks(this._handle, { spkPaths });
    checkStatus('engine_replace_spks', r.status);
    return r.report;
  }

  listSpks() {
    const r = addon.engineListSpks(this._handle);
    checkStatus('engine_list_spks', r.status);
    return r.spks;
  }
}

function apiVersion() {
  return addon.apiVersion();
}

function verifyAbi() {
  const got = apiVersion();
  if (got !== EXPECTED_API_VERSION) {
    throw new Error(`ABI mismatch: expected ${EXPECTED_API_VERSION}, got ${got}`);
  }
}

function clearActiveConfig() {
  checkStatus('config_clear_active', addon.configClearActive());
}

function queryOnce(config, query) {
  const r = addon.queryOnce(config, query);
  checkStatus('query_once', r.status);
  return r.state;
}

function cartesianToSpherical(positionKm) {
  const r = addon.cartesianToSpherical(positionKm);
  checkStatus('cartesian_to_spherical', r.status);
  return r.coords;
}

module.exports = {
  Config,
  Engine,
  EOP,
  LSK,
  apiVersion,
  verifyAbi,
  clearActiveConfig,
  queryOnce,
  cartesianToSpherical,
};
