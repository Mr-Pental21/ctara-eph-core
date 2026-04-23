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
  BENEFIC_NATURE,
  CHANDRA_BENEFIC_RULE,
  DIGNITY,
  STATUS,
  EXPECTED_API_VERSION,
  DELTA_T_MODEL,
  FUTURE_DELTA_T_TRANSITION,
  GRAHA_GENDER,
  NAISARGIKA,
  NODE_DIGNITY_POLICY,
  PANCHADHA,
  QUERY_OUTPUT,
  QUERY_TIME,
  SMH_FUTURE_FAMILY,
  TATKALIKA,
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
  BENEFIC_NATURE,
  CHANDRA_BENEFIC_RULE,
  DELTA_T_MODEL,
  DIGNITY,
  STATUS,
  EXPECTED_API_VERSION,
  FUTURE_DELTA_T_TRANSITION,
  GRAHA_GENDER,
  NAISARGIKA,
  NODE_DIGNITY_POLICY,
  PANCHADHA,
  QUERY_OUTPUT,
  QUERY_TIME,
  SMH_FUTURE_FAMILY,
  TATKALIKA,
  TIME_POLICY,
  TIME_WARNING,
  TT_UTC_SOURCE,
};
