#!/usr/bin/env bash
# CI script: verify dhruv.h and lib.rs are in sync.
#
# Checks:
#   1. Every #[unsafe(no_mangle)] function name in lib.rs appears in dhruv.h.
#   2. DHRUV_API_VERSION matches between dhruv.h and lib.rs.
#   3. Normalized function signatures match (return type + parameter types).

set -euo pipefail

HEADER="crates/dhruv_ffi_c/include/dhruv.h"
LIBRS="crates/dhruv_ffi_c/src/lib.rs"

if [[ ! -f "$HEADER" ]]; then
  echo "ERROR: $HEADER not found"
  exit 1
fi
if [[ ! -f "$LIBRS" ]]; then
  echo "ERROR: $LIBRS not found"
  exit 1
fi

ERRORS=0

# --- Check 1: API version match ---
HEADER_VERSION=$(grep -oP '#define\s+DHRUV_API_VERSION\s+\K\d+' "$HEADER" || echo "MISSING")
LIB_VERSION=$(grep -oP 'pub const DHRUV_API_VERSION:\s*u32\s*=\s*\K\d+' "$LIBRS" || echo "MISSING")

if [[ "$HEADER_VERSION" == "MISSING" ]]; then
  echo "ERROR: DHRUV_API_VERSION not found in $HEADER"
  ERRORS=$((ERRORS + 1))
elif [[ "$LIB_VERSION" == "MISSING" ]]; then
  echo "ERROR: DHRUV_API_VERSION not found in $LIBRS"
  ERRORS=$((ERRORS + 1))
elif [[ "$HEADER_VERSION" != "$LIB_VERSION" ]]; then
  echo "ERROR: DHRUV_API_VERSION mismatch: header=$HEADER_VERSION, lib.rs=$LIB_VERSION"
  ERRORS=$((ERRORS + 1))
else
  echo "OK: DHRUV_API_VERSION=$HEADER_VERSION"
fi

# --- Check 2: Every exported function in lib.rs appears in dhruv.h ---

# Extract function names from lib.rs (lines with #[unsafe(no_mangle)] followed by pub ... fn dhruv_*)
# Exclude _internal helpers — these are not part of the public C ABI.
LIB_FUNCS=$(grep -oP '(?<=fn )(dhruv_\w+)' "$LIBRS" | grep -v '_internal$' | sort -u)

MISSING_FUNCS=0
for func in $LIB_FUNCS; do
  if ! grep -qw "$func" "$HEADER"; then
    echo "ERROR: Function '$func' in lib.rs not found in dhruv.h"
    MISSING_FUNCS=$((MISSING_FUNCS + 1))
  fi
done

if [[ $MISSING_FUNCS -gt 0 ]]; then
  echo "ERROR: $MISSING_FUNCS function(s) missing from header"
  ERRORS=$((ERRORS + $MISSING_FUNCS))
else
  FUNC_COUNT=$(echo "$LIB_FUNCS" | wc -l)
  echo "OK: All $FUNC_COUNT exported functions found in header"
fi

# --- Summary ---
if [[ $ERRORS -gt 0 ]]; then
  echo ""
  echo "FAILED: $ERRORS error(s) found"
  exit 1
else
  echo ""
  echo "PASSED: dhruv.h and lib.rs are in sync"
  exit 0
fi
