'use strict';

const test = require('node:test');
const assert = require('node:assert/strict');

const dhruv = require('..');
const { hasKernels, hasEop, hasTara, kernelPaths } = require('./helpers');

test('dasha snapshot smoke', { skip: !(hasKernels() && hasEop()) }, () => {
  const paths = kernelPaths();

  const engine = dhruv.Engine.create({
    spkPaths: [paths.spk],
    lskPath: paths.lsk,
    cacheCapacity: 64,
    strictValidation: false,
  });
  const eop = dhruv.EOP.load(paths.eop);

  const loc = { latitudeDeg: 12.9716, longitudeDeg: 77.5946, altitudeM: 920 };
  const birthUtc = { year: 1990, month: 1, day: 1, hour: 12, minute: 0, second: 0 };
  const queryUtc = { year: 2025, month: 1, day: 1, hour: 12, minute: 0, second: 0 };

  const snapshot = dhruv.dashaSnapshotUtc(engine, eop, birthUtc, queryUtc, loc, {
    ayanamshaSystem: 0,
    useNutation: true,
    system: 0,
    maxLevel: 3,
  });

  assert.ok(snapshot.count >= 0);

  eop.close();
  engine.close();
});

test('low-tier dasha wrappers smoke', { skip: !(hasKernels() && hasEop()) }, () => {
  const paths = kernelPaths();

  const engine = dhruv.Engine.create({
    spkPaths: [paths.spk],
    lskPath: paths.lsk,
    cacheCapacity: 64,
    strictValidation: false,
  });
  const eop = dhruv.EOP.load(paths.eop);

  const loc = { latitudeDeg: 12.9716, longitudeDeg: 77.5946, altitudeM: 920 };
  const birthUtc = { year: 1990, month: 1, day: 1, hour: 12, minute: 0, second: 0 };

  const level0 = dhruv.dashaLevel0Utc(engine, eop, birthUtc, loc, {
    ayanamshaSystem: 0,
    useNutation: true,
    system: 0,
  });
  assert.ok(level0.length > 0);

  const first = level0[0];
  const same = dhruv.dashaLevel0EntityUtc(engine, eop, birthUtc, loc, {
    entityType: first.entityType,
    entityIndex: first.entityIndex,
  }, {
    ayanamshaSystem: 0,
    useNutation: true,
    system: 0,
  });
  assert.equal(same.entityIndex, first.entityIndex);

  const variation = dhruv.dashaVariationConfigDefault();
  const children = dhruv.dashaChildrenUtc(engine, eop, birthUtc, loc, first, {
    ayanamshaSystem: 0,
    useNutation: true,
    system: 0,
    variationConfig: variation,
  });
  assert.ok(children.length > 0);

  const child = dhruv.dashaChildPeriodUtc(engine, eop, birthUtc, loc, first, {
    entityType: children[0].entityType,
    entityIndex: children[0].entityIndex,
  }, {
    ayanamshaSystem: 0,
    useNutation: true,
    system: 0,
    variationConfig: variation,
  });
  assert.equal(child.entityIndex, children[0].entityIndex);

  const complete = dhruv.dashaCompleteLevelUtc(engine, eop, birthUtc, loc, level0, 1, {
    ayanamshaSystem: 0,
    useNutation: true,
    system: 0,
    variationConfig: variation,
  });
  assert.ok(complete.length >= children.length);

  eop.close();
  engine.close();
});

test('tara smoke', { skip: !hasTara() }, () => {
  const paths = kernelPaths();
  const cat = dhruv.TaraCatalog.load(paths.tara);

  const gc = cat.galacticCenterEcliptic(2451545.0);
  assert.ok(Number.isFinite(gc.lonDeg));

  const result = cat.compute({
    taraId: 0,
    outputKind: 1,
    jdTdb: 2451545.0,
    ayanamshaDeg: 24.0,
    config: { accuracy: 0, applyParallax: true },
  });
  assert.ok(Number.isFinite(result.siderealLongitudeDeg));

  cat.close();
});
