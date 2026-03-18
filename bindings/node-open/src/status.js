'use strict';

const EXPECTED_API_VERSION = 47;

const STATUS = {
  OK: 0,
  INVALID_CONFIG: 1,
  INVALID_QUERY: 2,
  KERNEL_LOAD: 3,
  TIME_CONVERSION: 4,
  UNSUPPORTED_QUERY: 5,
  EPOCH_OUT_OF_RANGE: 6,
  NULL_POINTER: 7,
  EOP_LOAD: 8,
  EOP_OUT_OF_RANGE: 9,
  INVALID_LOCATION: 10,
  NO_CONVERGENCE: 11,
  INVALID_SEARCH_CONFIG: 12,
  INVALID_INPUT: 13,
  INTERNAL: 255,
};

const STATUS_NAME = new Map(
  Object.entries(STATUS).map(([k, v]) => [v, k]),
);

function statusName(code) {
  return STATUS_NAME.get(code) || `UNKNOWN_${code}`;
}

module.exports = {
  EXPECTED_API_VERSION,
  STATUS,
  statusName,
};
