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

function dashaHierarchyUtc(
  engine,
  eop,
  birthUtc,
  location,
  {
    ayanamshaSystem = 0,
    useNutation = true,
    system = 0,
    maxLevel = 3,
  } = {},
) {
  const r = addon.dashaHierarchyUtc(
    engine._handle,
    eop._handle,
    birthUtc,
    location,
    ayanamshaSystem,
    !!useNutation,
    system,
    maxLevel,
  );
  checkStatus('dasha_hierarchy_utc', r.status);
  return new DashaHierarchy(r.handle);
}

function dashaSnapshotUtc(
  engine,
  eop,
  birthUtc,
  queryUtc,
  location,
  {
    ayanamshaSystem = 0,
    useNutation = true,
    system = 0,
    maxLevel = 3,
  } = {},
) {
  const r = addon.dashaSnapshotUtc(
    engine._handle,
    eop._handle,
    birthUtc,
    queryUtc,
    location,
    ayanamshaSystem,
    !!useNutation,
    system,
    maxLevel,
  );
  checkStatus('dasha_snapshot_utc', r.status);
  return r.snapshot;
}

function dashaLevel0Utc(
  engine,
  eop,
  birthUtc,
  location,
  {
    ayanamshaSystem = 0,
    useNutation = true,
    system = 0,
  } = {},
) {
  const r = addon.dashaLevel0Utc(
    engine._handle,
    eop._handle,
    birthUtc,
    location,
    ayanamshaSystem,
    !!useNutation,
    system,
  );
  checkStatus('dasha_level0_utc', r.status);
  return r.periods;
}

function dashaLevel0EntityUtc(
  engine,
  eop,
  birthUtc,
  location,
  entity,
  {
    ayanamshaSystem = 0,
    useNutation = true,
    system = 0,
  } = {},
) {
  const r = addon.dashaLevel0EntityUtc(
    engine._handle,
    eop._handle,
    birthUtc,
    location,
    ayanamshaSystem,
    !!useNutation,
    system,
    entity.entityType,
    entity.entityIndex,
  );
  checkStatus('dasha_level0_entity_utc', r.status);
  return r.found ? r.period : null;
}

function dashaChildrenUtc(
  engine,
  eop,
  birthUtc,
  location,
  parent,
  {
    ayanamshaSystem = 0,
    useNutation = true,
    system = 0,
    variationConfig = null,
  } = {},
) {
  const r = addon.dashaChildrenUtc(
    engine._handle,
    eop._handle,
    birthUtc,
    location,
    ayanamshaSystem,
    !!useNutation,
    system,
    variationConfig,
    parent,
  );
  checkStatus('dasha_children_utc', r.status);
  return r.periods;
}

function dashaChildPeriodUtc(
  engine,
  eop,
  birthUtc,
  location,
  parent,
  childEntity,
  {
    ayanamshaSystem = 0,
    useNutation = true,
    system = 0,
    variationConfig = null,
  } = {},
) {
  const r = addon.dashaChildPeriodUtc(
    engine._handle,
    eop._handle,
    birthUtc,
    location,
    ayanamshaSystem,
    !!useNutation,
    system,
    variationConfig,
    parent,
    childEntity.entityType,
    childEntity.entityIndex,
  );
  checkStatus('dasha_child_period_utc', r.status);
  return r.found ? r.period : null;
}

function dashaCompleteLevelUtc(
  engine,
  eop,
  birthUtc,
  location,
  parentPeriods,
  childLevel,
  {
    ayanamshaSystem = 0,
    useNutation = true,
    system = 0,
    variationConfig = null,
  } = {},
) {
  const r = addon.dashaCompleteLevelUtc(
    engine._handle,
    eop._handle,
    birthUtc,
    location,
    ayanamshaSystem,
    !!useNutation,
    system,
    variationConfig,
    parentPeriods,
    childLevel,
  );
  checkStatus('dasha_complete_level_utc', r.status);
  return r.periods;
}

module.exports = {
  DashaHierarchy,
  dashaSelectionConfigDefault,
  dashaVariationConfigDefault,
  dashaHierarchyUtc,
  dashaSnapshotUtc,
  dashaLevel0Utc,
  dashaLevel0EntityUtc,
  dashaChildrenUtc,
  dashaChildPeriodUtc,
  dashaCompleteLevelUtc,
};
