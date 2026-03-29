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
const {
  STATUS,
  EXPECTED_API_VERSION,
  DELTA_T_MODEL,
  FUTURE_DELTA_T_TRANSITION,
  QUERY_OUTPUT,
  QUERY_TIME,
  SMH_FUTURE_FAMILY,
  TIME_POLICY,
  TIME_WARNING,
  TT_UTC_SOURCE,
} = require('./status');

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
  DELTA_T_MODEL,
  STATUS,
  EXPECTED_API_VERSION,
  FUTURE_DELTA_T_TRANSITION,
  QUERY_OUTPUT,
  QUERY_TIME,
  SMH_FUTURE_FAMILY,
  TIME_POLICY,
  TIME_WARNING,
  TT_UTC_SOURCE,
};
