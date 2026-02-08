#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT_DIR"

ALLOWED_NODE_LICENSES="MIT;Apache-2.0;BSD-2-Clause;BSD-3-Clause;ISC;Zlib"

scan_rust() {
  if [[ -f "Cargo.toml" ]]; then
    echo "==> Running cargo-deny license check"
    cargo deny check licenses
  else
    echo "==> Skipping cargo-deny (no Cargo.toml in repository root)"
  fi
}

scan_node_wrapper() {
  local wrapper_dir="$1"
  echo "==> Scanning Node wrapper licenses in ${wrapper_dir}"

  if [[ ! -f "${wrapper_dir}/package.json" ]]; then
    return 0
  fi

  if [[ ! -f "${wrapper_dir}/package-lock.json" && ! -f "${wrapper_dir}/npm-shrinkwrap.json" ]]; then
    echo "ERROR: ${wrapper_dir} has package.json but no npm lockfile."
    echo "Use npm lockfiles so CI can run deterministic license scans."
    return 1
  fi

  (
    cd "${wrapper_dir}"
    npm ci --ignore-scripts
    npx --yes license-checker-rseidelsohn --production --onlyAllow "${ALLOWED_NODE_LICENSES}"
  )
}

scan_python_wrapper() {
  local wrapper_dir="$1"
  local requirements_file=""

  if [[ -f "${wrapper_dir}/requirements.lock.txt" ]]; then
    requirements_file="${wrapper_dir}/requirements.lock.txt"
  elif [[ -f "${wrapper_dir}/requirements.txt" ]]; then
    requirements_file="${wrapper_dir}/requirements.txt"
  elif [[ -f "${wrapper_dir}/pyproject.toml" ]]; then
    echo "ERROR: ${wrapper_dir} has pyproject.toml but no requirements lock file."
    echo "Add requirements.lock.txt (or requirements.txt) for deterministic license scanning."
    return 1
  else
    return 0
  fi

  echo "==> Scanning Python wrapper licenses in ${wrapper_dir}"
  local venv_dir
  venv_dir="$(mktemp -d)"
  trap 'rm -rf "$venv_dir"' RETURN

  python3 -m venv "${venv_dir}"
  # shellcheck source=/dev/null
  source "${venv_dir}/bin/activate"
  python -m pip install --upgrade pip >/dev/null
  python -m pip install pip-licenses >/dev/null
  python -m pip install -r "${requirements_file}" >/dev/null

  local license_json
  license_json="$(mktemp)"
  pip-licenses --format=json --with-license-file --with-system=false > "${license_json}"

  python3 - "${license_json}" <<'PY'
import json
import re
import sys

path = sys.argv[1]
deny_re = re.compile(r"\b(agpl|gpl|lgpl|sspl|busl|bsl)\b", re.IGNORECASE)
ambiguous_re = re.compile(r"\b(unknown|proprietary|custom|other)\b", re.IGNORECASE)
allowed_re = re.compile(r"(mit|apache|bsd|isc|zlib)", re.IGNORECASE)

with open(path, "r", encoding="utf-8") as f:
    data = json.load(f)

violations = []
for pkg in data:
    name = pkg.get("Name", "<unknown>")
    license_text = (pkg.get("License") or "").strip()

    if not license_text:
        violations.append(f"{name}: missing license")
        continue
    if deny_re.search(license_text):
        violations.append(f"{name}: denylisted license ({license_text})")
        continue
    if ambiguous_re.search(license_text):
        violations.append(f"{name}: ambiguous or non-approved license ({license_text})")
        continue
    if not allowed_re.search(license_text):
        violations.append(f"{name}: license not in allowlist ({license_text})")

if violations:
    print("Python license policy violations detected:")
    for v in violations:
        print(f"- {v}")
    sys.exit(1)
PY

  deactivate
  rm -f "${license_json}"
  rm -rf "${venv_dir}"
  trap - RETURN
}

scan_wrappers() {
  if [[ ! -d "bindings" ]]; then
    echo "==> Skipping wrapper scans (no bindings directory)"
    return 0
  fi

  local wrapper
  while IFS= read -r wrapper; do
    scan_node_wrapper "${wrapper}"
    scan_python_wrapper "${wrapper}"
  done < <(find bindings -mindepth 1 -maxdepth 2 -type d | sort)
}

scan_rust
scan_wrappers

echo "==> License gate checks passed"
