'use strict';

const test = require('node:test');
const assert = require('node:assert/strict');

const dhruv = require('..');
const { hasKernels, hasEop, kernelPaths } = require('./helpers');

test('api version matches expected ABI', () => {
  assert.equal(dhruv.apiVersion(), dhruv.EXPECTED_API_VERSION);
  assert.doesNotThrow(() => dhruv.verifyAbi());
});

test('engine query and UTC roundtrip', { skip: !hasKernels() }, () => {
  const paths = kernelPaths();

  const engine = dhruv.Engine.create({
    spkPaths: [paths.spk],
    lskPath: paths.lsk,
    cacheCapacity: 64,
    strictValidation: false,
  });

  const lsk = dhruv.LSK.load(paths.lsk);

  const state = engine.query({
    target: 301,
    observer: 399,
    frame: 1,
    epochTdbJd: 2451545.0,
  });

  assert.ok(Number.isFinite(state.positionKm[0]));

  const utc = { year: 2025, month: 1, day: 1, hour: 0, minute: 0, second: 0 };
  const jd = dhruv.utcToTdbJd(lsk, utc);
  const back = dhruv.jdTdbToUtc(lsk, jd);

  assert.equal(back.year, utc.year);
  assert.equal(back.month, utc.month);
  assert.equal(back.day, utc.day);

  lsk.close();
  engine.close();
});

test('search and panchang smoke', { skip: !(hasKernels() && hasEop()) }, () => {
  const paths = kernelPaths();

  const engine = dhruv.Engine.create({
    spkPaths: [paths.spk],
    lskPath: paths.lsk,
    cacheCapacity: 64,
    strictValidation: false,
  });

  const eop = dhruv.EOP.load(paths.eop);
  const lsk = dhruv.LSK.load(paths.lsk);

  const search = dhruv.lunarPhaseSearch(
    engine,
    {
      phaseKind: 1,
      queryMode: 0,
      atJdTdb: 2460000.5,
      startJdTdb: 0,
      endJdTdb: 0,
    },
    8,
  );

  assert.equal(search.found, true);

  const conj = dhruv.conjunctionSearch(
    engine,
    {
      body1Code: 10,
      body2Code: 301,
      queryMode: 0,
      atJdTdb: 2460000.5,
      startJdTdb: 0,
      endJdTdb: 0,
    },
    4,
  );
  assert.equal(conj.found, true);

  const sank = dhruv.sankrantiSearch(
    engine,
    {
      targetKind: 0,
      queryMode: 0,
      rashiIndex: 0,
      atJdTdb: 2460000.5,
      startJdTdb: 0,
      endJdTdb: 0,
    },
    4,
  );
  assert.equal(sank.found, true);

  const grahan = dhruv.grahanSearch(
    engine,
    {
      grahanKind: 0,
      queryMode: 0,
      atJdTdb: 2460000.5,
      startJdTdb: 0,
      endJdTdb: 0,
    },
    2,
  );
  assert.equal(typeof grahan.found, 'boolean');

  const motion = dhruv.motionSearch(
    engine,
    {
      bodyCode: 199,
      motionKind: 0,
      queryMode: 0,
      atJdTdb: 2460000.5,
      startJdTdb: 0,
      endJdTdb: 0,
    },
    2,
  );
  assert.equal(typeof motion.found, 'boolean');

  const tithi = dhruv.tithiForDate(engine, {
    year: 2025,
    month: 1,
    day: 15,
    hour: 12,
    minute: 0,
    second: 0,
  });
  assert.ok(Number.isInteger(tithi.tithiIndex));

  const karana = dhruv.karanaForDate(engine, {
    year: 2025,
    month: 1,
    day: 15,
    hour: 12,
    minute: 0,
    second: 0,
  });
  assert.ok(Number.isInteger(karana.karanaIndex));

  const yoga = dhruv.yogaForDate(engine, {
    year: 2025,
    month: 1,
    day: 15,
    hour: 12,
    minute: 0,
    second: 0,
  });
  assert.ok(Number.isInteger(yoga.yogaIndex));

  const nak = dhruv.nakshatraForDate(engine, {
    year: 2025,
    month: 1,
    day: 15,
    hour: 12,
    minute: 0,
    second: 0,
  });
  assert.ok(Number.isInteger(nak.nakshatraIndex));

  const loc = { latitudeDeg: 12.9716, longitudeDeg: 77.5946, altitudeM: 920 };
  const vaar = dhruv.vaarForDate(engine, eop, {
    year: 2025,
    month: 1,
    day: 15,
    hour: 12,
    minute: 0,
    second: 0,
  }, loc);
  assert.ok(Number.isInteger(vaar.vaarIndex));

  const hora = dhruv.horaForDate(engine, eop, {
    year: 2025,
    month: 1,
    day: 15,
    hour: 12,
    minute: 0,
    second: 0,
  }, loc);
  assert.ok(Number.isInteger(hora.horaIndex));

  const ghatika = dhruv.ghatikaForDate(engine, eop, {
    year: 2025,
    month: 1,
    day: 15,
    hour: 12,
    minute: 0,
    second: 0,
  }, loc);
  assert.ok(Number.isInteger(ghatika.value));

  const masa = dhruv.masaForDate(engine, {
    year: 2025,
    month: 1,
    day: 15,
    hour: 12,
    minute: 0,
    second: 0,
  });
  assert.ok(Number.isInteger(masa.masaIndex));

  const ayana = dhruv.ayanaForDate(engine, {
    year: 2025,
    month: 1,
    day: 15,
    hour: 12,
    minute: 0,
    second: 0,
  });
  assert.ok(Number.isInteger(ayana.ayana));

  const varsha = dhruv.varshaForDate(engine, {
    year: 2025,
    month: 1,
    day: 15,
    hour: 12,
    minute: 0,
    second: 0,
  });
  assert.ok(Number.isInteger(varsha.samvatsaraIndex));

  const ayanamshaDeg = dhruv.ayanamshaComputeEx(
    lsk,
    {
      systemCode: 0,
      mode: 2,
      timeKind: 1,
      jdTdb: 0,
      utc: { year: 2025, month: 1, day: 15, hour: 12, minute: 0, second: 0 },
      useNutation: true,
      deltaPsiArcsec: 0,
    },
    eop,
  );
  assert.ok(Number.isFinite(ayanamshaDeg));

  const node = dhruv.lunarNodeDeg(0, 0, 2451545.0);
  assert.ok(Number.isFinite(node));
  const node2 = dhruv.lunarNodeDegWithEngine(engine, 0, 1, 2451545.0);
  assert.ok(Number.isFinite(node2));

  const rashi = dhruv.rashiFromLongitude(10.0);
  assert.ok(Number.isInteger(rashi.rashiIndex));
  const nak27 = dhruv.nakshatraFromLongitude(10.0);
  assert.ok(Number.isInteger(nak27.nakshatraIndex));
  const nak28 = dhruv.nakshatra28FromLongitude(10.0);
  assert.ok(Number.isInteger(nak28.nakshatraIndex));
  assert.equal(dhruv.rashiCount(), 12);
  assert.ok(dhruv.nakshatraCount(27) >= 27);

  const shadbala = dhruv.shadbalaForDate(engine, eop, {
    year: 2025,
    month: 1,
    day: 15,
    hour: 12,
    minute: 0,
    second: 0,
  }, loc, 0, true);
  assert.equal(shadbala.totalRupas.length, 7);

  const kundali = dhruv.fullKundaliSummaryForDate(engine, eop, {
    year: 2025,
    month: 1,
    day: 15,
    hour: 12,
    minute: 0,
    second: 0,
  }, loc, 0, true);
  assert.ok(Number.isFinite(kundali.ayanamshaDeg));

  lsk.close();
  eop.close();
  engine.close();
});
