'use strict';

const engine = require('./engine');
const time = require('./time');
const search = require('./search');
const panchang = require('./panchang');
const jyotish = require('./jyotish');
const extras = require('./extras');
const shadbala = require('./shadbala');
const dasha = require('./dasha');
const tara = require('./tara');
const { STATUS, EXPECTED_API_VERSION, QUERY_OUTPUT, QUERY_TIME } = require('./status');

module.exports = {
  ...engine,
  ...time,
  ...search,
  ...panchang,
  ...jyotish,
  ...extras,
  ...shadbala,
  ...dasha,
  ...tara,
  STATUS,
  EXPECTED_API_VERSION,
  QUERY_OUTPUT,
  QUERY_TIME,
};
