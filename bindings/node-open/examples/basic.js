'use strict';

const path = require('node:path');
const dhruv = require('..');

function kernelPath(name) {
  return process.env[name] || '';
}

const spk = kernelPath('DHRUV_SPK_PATH');
const lsk = kernelPath('DHRUV_LSK_PATH');
const eopPath = kernelPath('DHRUV_EOP_PATH');

if (!spk || !lsk || !eopPath) {
  console.error('Set DHRUV_SPK_PATH, DHRUV_LSK_PATH, and DHRUV_EOP_PATH before running this example.');
  process.exit(1);
}

dhruv.verifyAbi();

const engine = dhruv.Engine.create({
  spkPaths: [path.resolve(spk)],
  lskPath: path.resolve(lsk),
  cacheCapacity: 64,
  strictValidation: false,
});

const eop = dhruv.EOP.load(path.resolve(eopPath));
const lskHandle = dhruv.LSK.load(path.resolve(lsk));

const state = engine.query({
  target: 301,
  observer: 399,
  frame: 1,
  epochTdbJd: 2451545.0,
});

console.log('state.positionKm[0]=', state.positionKm[0]);

const utc = { year: 2025, month: 1, day: 1, hour: 0, minute: 0, second: 0 };
const converted = dhruv.utcToTdbJd(lskHandle, { utc });
console.log('jd=', converted.jdTdb);

const tithi = dhruv.tithiForDate(engine, { year: 2025, month: 1, day: 15, hour: 12, minute: 0, second: 0 });
console.log('tithi=', tithi.tithiIndex);

lskHandle.close();
eop.close();
engine.close();
