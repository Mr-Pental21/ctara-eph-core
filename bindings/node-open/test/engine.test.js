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

  assert.ok(Number.isFinite(state.state.positionKm[0]));
  assert.equal(state.sphericalState, null);

  const utc = { year: 2025, month: 1, day: 1, hour: 0, minute: 0, second: 0 };
  const jd = dhruv.utcToTdbJd(lsk, utc);
  const back = dhruv.jdTdbToUtc(lsk, jd);
  const spherical = engine.query({
    target: 301,
    observer: 399,
    frame: 1,
    utc,
    outputMode: dhruv.QUERY_OUTPUT.SPHERICAL,
  });

  assert.equal(back.year, utc.year);
  assert.equal(back.month, utc.month);
  assert.equal(back.day, utc.day);
  assert.equal(spherical.state, null);
  assert.ok(Number.isFinite(spherical.sphericalState.lonDeg));

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
  const riseCfg = dhruv.riseSetConfigDefault();
  const sankCfg = dhruv.sankrantiConfigDefault();
  const bhavaCfg = dhruv.bhavaConfigDefault();
  const utc = {
    year: 2025,
    month: 1,
    day: 15,
    hour: 12,
    minute: 0,
    second: 0,
  };

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

  const tithi = dhruv.tithiForDate(engine, utc);
  assert.ok(Number.isInteger(tithi.tithiIndex));

  const karana = dhruv.karanaForDate(engine, utc);
  assert.ok(Number.isInteger(karana.karanaIndex));

  const yoga = dhruv.yogaForDate(engine, utc, sankCfg);
  assert.ok(Number.isInteger(yoga.yogaIndex));

  const nak = dhruv.nakshatraForDate(engine, utc, sankCfg);
  assert.ok(Number.isInteger(nak.nakshatraIndex));

  const loc = { latitudeDeg: 12.9716, longitudeDeg: 77.5946, altitudeM: 920 };
  const vaar = dhruv.vaarForDate(engine, eop, utc, loc, riseCfg);
  assert.ok(Number.isInteger(vaar.vaarIndex));

  const hora = dhruv.horaForDate(engine, eop, utc, loc, riseCfg);
  assert.ok(Number.isInteger(hora.horaIndex));

  const ghatika = dhruv.ghatikaForDate(engine, eop, utc, loc, riseCfg);
  assert.ok(Number.isInteger(ghatika.value));

  const masa = dhruv.masaForDate(engine, utc, sankCfg);
  assert.ok(Number.isInteger(masa.masaIndex));

  const ayana = dhruv.ayanaForDate(engine, utc, sankCfg);
  assert.ok(Number.isInteger(ayana.ayana));

  const varsha = dhruv.varshaForDate(engine, utc, sankCfg);
  assert.ok(Number.isInteger(varsha.samvatsaraIndex));
  const panchang = dhruv.panchangComputeEx(engine, eop, lsk, {
    timeKind: 1,
    jdTdb: -1,
    utc,
    includeMask: 1,
    location: loc,
    riseSetConfig: riseCfg,
    sankrantiConfig: sankCfg,
  });
  assert.equal(typeof panchang, 'object');
  assert.ok(dhruv.bhavaSystemCount() >= 1);

  const riseSet = dhruv.computeRiseSet(engine, eop, loc, riseCfg, 0, 2460000.5, lsk);
  assert.ok(Number.isInteger(riseSet.eventCode));
  assert.ok(Number.isFinite(riseSet.jdTdb));
  const riseUtc = dhruv.riseSetResultToUtc(lsk, riseSet);
  assert.ok(Number.isInteger(riseUtc.year));

  const allEvents = dhruv.computeAllEvents(engine, eop, loc, riseCfg, 2460000.5, lsk);
  assert.equal(allEvents.length, 8);

  const riseSetUtc = dhruv.computeRiseSetUtc(engine, eop, lsk, loc, 0, utc, riseCfg);
  assert.ok(Number.isInteger(riseSetUtc.eventCode));
  assert.equal(typeof riseSetUtc.utc.year, 'number');

  const allEventsUtc = dhruv.computeAllEventsUtc(engine, eop, lsk, loc, utc, riseCfg);
  assert.equal(allEventsUtc.length, 8);

  const bhavas = dhruv.computeBhavas(engine, eop, loc, lsk, 2460000.5, bhavaCfg);
  assert.equal(bhavas.bhavas.length, 12);
  const bhavasUtc = dhruv.computeBhavasUtc(engine, eop, lsk, loc, utc, bhavaCfg);
  assert.equal(bhavasUtc.bhavas.length, 12);

  const lagna = dhruv.lagnaDeg(lsk, eop, loc, 2460000.5);
  const mc = dhruv.mcDeg(lsk, eop, loc, 2460000.5);
  const ramc = dhruv.ramcDeg(lsk, eop, loc, 2460000.5);
  const lagnaUtc = dhruv.lagnaDegUtc(lsk, eop, loc, utc);
  const mcUtc = dhruv.mcDegUtc(lsk, eop, loc, utc);
  const ramcUtc = dhruv.ramcDegUtc(lsk, eop, loc, utc);
  assert.ok(Number.isFinite(lagna));
  assert.ok(Number.isFinite(mc));
  assert.ok(Number.isFinite(ramc));
  assert.ok(Number.isFinite(lagnaUtc));
  assert.ok(Number.isFinite(mcUtc));
  assert.ok(Number.isFinite(ramcUtc));

  const ayanamshaDeg = dhruv.ayanamshaComputeEx(
    lsk,
    {
      systemCode: 0,
      mode: 2,
      timeKind: 1,
      jdTdb: 0,
      utc,
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
  const nodeUtc = dhruv.lunarNodeDegUtc(lsk, 0, 0, utc);
  assert.ok(Number.isFinite(nodeUtc));
  assert.equal(typeof dhruv.lunarNodeDegUtcWithEngine, 'function');

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
  assert.equal(shadbala.entries.length, 7);

  const vimsopaka = dhruv.vimsopakaForDate(engine, eop, utc, loc, 0, true, 0);
  assert.equal(vimsopaka.entries.length, 9);

  const avastha = dhruv.avasthaForDate(engine, eop, utc, loc, bhavaCfg, riseCfg, 0, true, 0);
  assert.equal(avastha.entries.length, 9);
  const charakaraka = dhruv.charakarakaForDate(engine, eop, utc, 0, true, 'parashari');
  assert.ok(charakaraka.count >= 7);
  assert.ok(charakaraka.count <= 8);
  assert.equal(charakaraka.scheme, dhruv.CHARAKARAKA_SCHEME.MIXED_PARASHARA);
  assert.ok(Array.isArray(charakaraka.entries));

  const specialLagnas = dhruv.specialLagnasForDate(engine, eop, utc, loc, riseCfg, 0, true);
  assert.ok(Number.isFinite(specialLagnas.bhavaLagna));
  try {
    const arudhas = dhruv.arudhaPadasForDate(engine, eop, utc, loc, 0, true);
    assert.equal(arudhas.length, 12);
  } catch (err) {
    assert.equal(err.status, dhruv.STATUS.INVALID_QUERY);
  }
  try {
    const upagrahas = dhruv.allUpagrahasForDate(engine, eop, utc, loc, 0, true);
    assert.ok(Number.isFinite(upagrahas.gulika));
  } catch (err) {
    assert.equal(err.status, dhruv.STATUS.INVALID_QUERY);
  }

  const kundali = dhruv.fullKundaliSummaryForDate(engine, eop, {
    year: 2025,
    month: 1,
    day: 15,
    hour: 12,
    minute: 0,
    second: 0,
  }, loc, 0, true);
  assert.ok(Number.isFinite(kundali.ayanamshaDeg));
  assert.equal(typeof kundali.charakarakaValid, 'boolean');

  const fullCfg = dhruv.fullKundaliConfigDefault();
  fullCfg.includeBhavaCusps = true;
  fullCfg.includeBindus = true;
  fullCfg.includeUpagrahas = true;
  fullCfg.includeSphutas = true;
  fullCfg.includeSpecialLagnas = true;
  fullCfg.includeDasha = true;
  fullCfg.includeAmshas = true;
  fullCfg.dashaConfig.count = 2;
  fullCfg.dashaConfig.systems[0] = 0;
  fullCfg.dashaConfig.systems[1] = 1;
  fullCfg.dashaConfig.maxLevels[0] = 0;
  fullCfg.dashaConfig.maxLevels[1] = 1;
  fullCfg.amshaScope = {
    includeBhavaCusps: true,
    includeArudhaPadas: true,
    includeUpagrahas: true,
    includeSphutas: true,
    includeSpecialLagnas: true,
  };
  fullCfg.amshaSelection.count = 1;
  fullCfg.amshaSelection.codes[0] = 9;
  fullCfg.amshaSelection.variations[0] = 0;
  const kundaliFull = dhruv.fullKundaliForDate(engine, eop, utc, loc, bhavaCfg, riseCfg, 0, true, fullCfg);
  assert.equal(kundaliFull.sphutas.longitudes.length, 16);
  assert.equal(kundaliFull.amshas.length, 1);
  assert.equal(kundaliFull.amshas[0].bhavaCusps.length, 12);
  assert.equal(kundaliFull.amshas[0].arudhaPadas.length, 12);
  assert.equal(kundaliFull.amshas[0].upagrahas.length, 11);
  assert.equal(kundaliFull.amshas[0].sphutas.length, 16);
  assert.equal(kundaliFull.amshas[0].specialLagnas.length, 8);
  assert.equal(kundaliFull.dasha.length, 2);
  assert.equal(kundaliFull.dasha[0].system, 0);
  assert.equal(kundaliFull.dasha[1].system, 1);
  assert.equal(kundaliFull.dasha[0].levels.length, 1);
  assert.equal(kundaliFull.dasha[1].levels.length, 2);
  const jdNow = dhruv.utcToTdbJd(lsk, utc);

  const elongation = dhruv.elongationAt(engine, jdNow);
  assert.ok(Number.isFinite(elongation));
  const siderealSum = dhruv.siderealSumAt(engine, jdNow, sankCfg);
  assert.ok(Number.isFinite(siderealSum));

  const sunrises = dhruv.vedicDaySunrises(engine, eop, utc, loc, riseCfg);
  assert.ok(Number.isFinite(sunrises.sunriseJd));
  assert.ok(Number.isFinite(sunrises.nextSunriseJd));

  const lonLat = dhruv.bodyEclipticLonLat(engine, 301, jdNow);
  assert.ok(Number.isFinite(lonLat.lonDeg));
  assert.ok(Number.isFinite(lonLat.latDeg));

  try {
    const tithiAt = dhruv.tithiAt(engine, jdNow, sunrises.sunriseJd);
    assert.ok(Number.isInteger(tithiAt.tithiIndex));
  } catch (err) {
    assert.equal(err.status, dhruv.STATUS.NO_CONVERGENCE);
  }
  try {
    const karanaAt = dhruv.karanaAt(engine, jdNow, sunrises.sunriseJd);
    assert.ok(Number.isInteger(karanaAt.karanaIndex));
  } catch (err) {
    assert.equal(err.status, dhruv.STATUS.NO_CONVERGENCE);
  }
  try {
    const yogaAt = dhruv.yogaAt(engine, jdNow, sunrises.sunriseJd, sankCfg);
    assert.ok(Number.isInteger(yogaAt.yogaIndex));
  } catch (err) {
    assert.equal(err.status, dhruv.STATUS.NO_CONVERGENCE);
  }

  const vaarFromSunrises = dhruv.vaarFromSunrises(lsk, sunrises.sunriseJd, sunrises.nextSunriseJd);
  const horaFromSunrises = dhruv.horaFromSunrises(lsk, jdNow, sunrises.sunriseJd, sunrises.nextSunriseJd);
  const ghatikaFromSunrises = dhruv.ghatikaFromSunrises(lsk, jdNow, sunrises.sunriseJd, sunrises.nextSunriseJd);
  assert.ok(Number.isInteger(vaarFromSunrises.vaarIndex));
  assert.ok(Number.isInteger(horaFromSunrises.horaIndex));
  assert.ok(Number.isInteger(ghatikaFromSunrises.value));

  const moonSidereal = dhruv.grahaSiderealLongitudes(engine, jdNow, 0, true)[1];
  const nakshatraAt = dhruv.nakshatraAt(engine, jdNow, moonSidereal, sankCfg);
  assert.ok(Number.isInteger(nakshatraAt.nakshatraIndex));

  const ghatikaElapsed = dhruv.ghatikaFromElapsed(jdNow, sunrises.sunriseJd, sunrises.nextSunriseJd);
  const ghatikasSinceSunrise = dhruv.ghatikasSinceSunrise(jdNow, sunrises.sunriseJd);
  assert.ok(Number.isInteger(ghatikaElapsed));
  assert.ok(Number.isFinite(ghatikasSinceSunrise));

  const sphutas = dhruv.allSphutas({
    sun: 10,
    moon: 20,
    mars: 30,
    jupiter: 40,
    venus: 50,
    rahu: 60,
    lagna: 70,
    eighthLord: 80,
    gulika: 90,
  });
  assert.equal(sphutas.longitudes.length, 16);
  const kshetraViaScalar = dhruv.kshetraSphuta(20, 30, 40, 50, 70);
  // ALL_SPHUTAS order in dhruv_vedic_base: KshetraSphuta is index 8.
  const kshetraIdx = 8;
  assert.ok(Math.abs(kshetraViaScalar - sphutas.longitudes[kshetraIdx]) < 1e-9);

  const arudhaPada = dhruv.arudhaPada(100, 130, 0);
  assert.ok(Number.isFinite(arudhaPada.longitudeDeg));

  const sunUpagrahas = dhruv.sunBasedUpagrahas(dhruv.grahaSiderealLongitudes(engine, jdNow, 0, true)[0]);
  assert.ok(Number.isFinite(sunUpagrahas.dhooma));
  try {
    const weekday = dhruv.vaarFromJd(sunrises.sunriseJd);
    const isDay = 1;
    const sunsetEstimate = (sunrises.sunriseJd + sunrises.nextSunriseJd) / 2.0;
    const timeUpagraha = dhruv.timeUpagrahaJd(0, weekday, isDay, sunrises.sunriseJd, sunsetEstimate, sunrises.nextSunriseJd);
    assert.ok(Number.isFinite(timeUpagraha));
  } catch (err) {
    assert.ok(err.status === dhruv.STATUS.INVALID_QUERY || err.status === dhruv.STATUS.NO_CONVERGENCE);
  }
  try {
    const timeUpagrahaUtc = dhruv.timeUpagrahaJdUtc(engine, eop, utc, loc, riseCfg, 0);
    assert.ok(Number.isFinite(timeUpagrahaUtc));
  } catch (err) {
    assert.ok(err.status === dhruv.STATUS.INVALID_QUERY || err.status === dhruv.STATUS.NO_CONVERGENCE);
  }

  const ashtakavarga = dhruv.calculateAshtakavarga([0, 1, 2, 3, 4, 5, 6], 0);
  assert.equal(ashtakavarga.bavs.length, 7);
  assert.equal(ashtakavarga.bavs[0].contributors.length, 12);
  const bav = dhruv.calculateBav(0, [0, 1, 2, 3, 4, 5, 6], 0);
  assert.equal(bav.points.length, 12);
  assert.equal(bav.contributors.length, 12);
  for (let i = 0; i < 12; i += 1) {
    assert.equal(bav.contributors[i].length, 8);
    assert.equal(bav.contributors[i].reduce((a, b) => a + b, 0), bav.points[i]);
  }
  const allBav = dhruv.calculateAllBav([0, 1, 2, 3, 4, 5, 6], 0);
  assert.equal(allBav.length, 7);
  const sav = dhruv.calculateSav(allBav);
  assert.equal(sav.totalPoints.length, 12);
  assert.equal(dhruv.trikonaSodhana(Array(12).fill(1)).length, 12);
  assert.equal(dhruv.ekadhipatyaSodhana(Array(12).fill(1), [0, 1, 2, 3, 4, 5, 6], 0).length, 12);
  const ashtakavargaDate = dhruv.ashtakavargaForDate(engine, eop, utc, loc, 0, true);
  assert.equal(ashtakavargaDate.bavs.length, 7);

  const drishtiEntry = dhruv.grahaDrishti(0, 10, 100);
  assert.ok(Number.isFinite(drishtiEntry.totalVirupa));
  const drishtiMatrix = dhruv.grahaDrishtiMatrixForLongitudes(Array(9).fill(0).map((_, i) => i * 10));
  assert.equal(drishtiMatrix.length, 9);
  const drishtiCfg = { includeBhava: true, includeLagna: true, includeBindus: false };
  const drishti = dhruv.drishtiForDate(engine, eop, utc, loc, bhavaCfg, riseCfg, 0, true, drishtiCfg);
  assert.equal(drishti.grahaToGraha.length, 9);

  const grahaPosCfg = { includeNakshatra: true, includeLagna: true, includeOuterPlanets: true, includeBhava: true };
  const grahaPositions = dhruv.grahaPositionsForDate(engine, eop, utc, loc, bhavaCfg, 0, true, grahaPosCfg);
  assert.equal(grahaPositions.grahas.length, 9);

  const bindusCfg = { includeNakshatra: true, includeBhava: true };
  const bindus = dhruv.coreBindusForDate(engine, eop, utc, loc, bhavaCfg, riseCfg, 0, true, bindusCfg);
  assert.equal(bindus.arudhaPadas.length, 12);

  const amshaLon = dhruv.amshaLongitude(100, 9, 0);
  assert.ok(Number.isFinite(amshaLon));
  const amshaRashi = dhruv.amshaRashiInfo(100, 9, 0);
  assert.ok(Number.isInteger(amshaRashi.rashiIndex));
  const amshaLons = dhruv.amshaLongitudes(100, [9, 10], [0, 0]);
  assert.equal(amshaLons.length, 2);
  const amshaScope = {
    includeBhavaCusps: true,
    includeArudhaPadas: true,
    includeUpagrahas: true,
    includeSphutas: true,
    includeSpecialLagnas: true,
  };
  const amshaChart = dhruv.amshaChartForDate(engine, eop, utc, loc, bhavaCfg, riseCfg, 0, true, 9, 0, amshaScope);
  assert.equal(amshaChart.grahas.length, 9);
  assert.equal(amshaChart.bhavaCusps.length, 12);
  assert.equal(amshaChart.arudhaPadas.length, 12);
  assert.equal(amshaChart.upagrahas.length, 11);
  assert.equal(amshaChart.sphutas.length, 16);
  assert.equal(amshaChart.specialLagnas.length, 8);

  const dashaCfg = dhruv.dashaSelectionConfigDefault();
  assert.equal(typeof dashaCfg.count, 'number');
  const dashaHierarchy = dhruv.dashaHierarchyUtc(engine, eop, utc, loc, {
    ayanamshaSystem: 0,
    useNutation: true,
    system: 0,
    maxLevel: 1,
  });
  const levelCount = dashaHierarchy.levelCount();
  assert.ok(levelCount >= 0);
  if (levelCount > 0) {
    const firstLevelCount = dashaHierarchy.periodCount(0);
    assert.ok(firstLevelCount >= 0);
    if (firstLevelCount > 0) {
      const firstPeriod = dashaHierarchy.periodAt(0, 0);
      assert.ok(Number.isFinite(firstPeriod.startJd));
    }
  }
  dashaHierarchy.close();

  lsk.close();
  eop.close();
  engine.close();
});
