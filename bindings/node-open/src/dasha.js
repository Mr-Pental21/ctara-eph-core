'use strict';

const { addon } = require('./native');
const { checkStatus } = require('./errors');

class DashaHierarchy {
  constructor(handle) {
    this._handle = handle;
    this._closed = false;
  }

  close() {
    if (this._closed) return;
    addon.dashaHierarchyFree(this._handle);
    this._closed = true;
    this._handle = null;
  }

  levelCount() {
    const r = addon.dashaHierarchyLevelCount(this._handle);
    checkStatus('dasha_hierarchy_level_count', r.status);
    return r.count;
  }

  periodCount(level) {
    const r = addon.dashaHierarchyPeriodCount(this._handle, level);
    checkStatus('dasha_hierarchy_period_count', r.status);
    return r.count;
  }

  periodAt(level, idx) {
    const r = addon.dashaHierarchyPeriodAt(this._handle, level, idx);
    checkStatus('dasha_hierarchy_period_at', r.status);
    return r.period;
  }
}

function dashaSelectionConfigDefault() {
  return addon.dashaSelectionConfigDefault();
}

function dashaVariationConfigDefault() {
  return addon.dashaVariationConfigDefault();
}

function assignOptional(target, key, value) {
  if (value !== undefined && value !== null) {
    target[key] = value;
  }
}

function baseRequest(request, defaults = {}) {
  const normalized = {
    ayanamshaSystem: request.ayanamshaSystem ?? 0,
    useNutation: request.useNutation ?? true,
    system: request.system ?? 0,
    ...defaults,
  };
  assignOptional(normalized, 'birthUtc', request.birthUtc);
  assignOptional(normalized, 'birthJd', request.birthJd);
  assignOptional(normalized, 'location', request.location);
  assignOptional(normalized, 'bhavaConfig', request.bhavaConfig);
  assignOptional(normalized, 'risesetConfig', request.risesetConfig);
  assignOptional(normalized, 'variationConfig', request.variationConfig);
  assignOptional(normalized, 'inputs', request.inputs);
  return normalized;
}

function dashaHierarchy(engine, eop, request = {}) {
  const r = addon.dashaHierarchy(engine._handle, eop._handle, baseRequest(request, {
    maxLevel: request.maxLevel ?? 3,
  }));
  checkStatus('dasha_hierarchy', r.status);
  return new DashaHierarchy(r.handle);
}

function dashaSnapshot(engine, eop, request = {}) {
  const normalized = baseRequest(request, {
    maxLevel: request.maxLevel ?? 3,
  });
  assignOptional(normalized, 'queryUtc', request.queryUtc);
  assignOptional(normalized, 'queryJd', request.queryJd);
  const r = addon.dashaSnapshot(engine._handle, eop._handle, normalized);
  checkStatus('dasha_snapshot', r.status);
  return r.snapshot;
}

function dashaLevel0(engine, eop, request = {}) {
  const r = addon.dashaLevel0(engine._handle, eop._handle, baseRequest(request));
  checkStatus('dasha_level0', r.status);
  return r.periods;
}

function dashaLevel0Entity(engine, eop, request = {}) {
  const entity = request.entity ?? null;
  const normalized = baseRequest(request);
  normalized.entityType = request.entityType ?? entity?.entityType;
  normalized.entityIndex = request.entityIndex ?? entity?.entityIndex;
  const r = addon.dashaLevel0Entity(engine._handle, eop._handle, normalized);
  checkStatus('dasha_level0_entity', r.status);
  return r.found ? r.period : null;
}

function dashaChildren(engine, eop, request = {}) {
  const normalized = baseRequest(request);
  assignOptional(normalized, 'parent', request.parent);
  const r = addon.dashaChildren(engine._handle, eop._handle, normalized);
  checkStatus('dasha_children', r.status);
  return r.periods;
}

function dashaChildPeriod(engine, eop, request = {}) {
  const childEntity = request.childEntity ?? null;
  const normalized = baseRequest(request);
  assignOptional(normalized, 'parent', request.parent);
  normalized.childEntityType = request.childEntityType ?? childEntity?.entityType;
  normalized.childEntityIndex = request.childEntityIndex ?? childEntity?.entityIndex;
  const r = addon.dashaChildPeriod(engine._handle, eop._handle, normalized);
  checkStatus('dasha_child_period', r.status);
  return r.found ? r.period : null;
}

function dashaCompleteLevel(engine, eop, request = {}) {
  const normalized = baseRequest(request);
  assignOptional(normalized, 'parentPeriods', request.parentPeriods);
  assignOptional(normalized, 'childLevel', request.childLevel);
  const r = addon.dashaCompleteLevel(engine._handle, eop._handle, normalized);
  checkStatus('dasha_complete_level', r.status);
  return r.periods;
}

module.exports = {
  DashaHierarchy,
  dashaSelectionConfigDefault,
  dashaVariationConfigDefault,
  dashaHierarchy,
  dashaSnapshot,
  dashaLevel0,
  dashaLevel0Entity,
  dashaChildren,
  dashaChildPeriod,
  dashaCompleteLevel,
};
