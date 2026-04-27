#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
MANIFEST="${ROOT_DIR}/kernels/manifest/de441_de442_splits.tsv"
DATA_DIR="${ROOT_DIR}/kernels/data"
LSK="${DATA_DIR}/naif0012.tls"
SPKMERGE_BIN="${SPKMERGE:-spkmerge}"
UPDATE_MANIFEST=0

usage() {
  cat <<'EOF'
Usage: generate_split_kernels.sh [--manifest <path>] [--data-dir <path>] [--spkmerge <path>] [--update-manifest]

Generates DE441/DE442 split SPK files from kernels/manifest/de441_de442_splits.tsv.
The SPKMERGE executable must be on PATH or provided with --spkmerge.
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --manifest)
      MANIFEST="$2"
      shift 2
      ;;
    --data-dir)
      DATA_DIR="$2"
      LSK="${DATA_DIR}/naif0012.tls"
      shift 2
      ;;
    --spkmerge)
      SPKMERGE_BIN="$2"
      shift 2
      ;;
    --update-manifest)
      UPDATE_MANIFEST=1
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "Unknown argument: $1" >&2
      usage >&2
      exit 1
      ;;
  esac
done

if [[ ! -f "$MANIFEST" ]]; then
  echo "Manifest not found: $MANIFEST" >&2
  exit 1
fi

if [[ ! -f "$LSK" ]]; then
  echo "Leap-seconds kernel not found: $LSK" >&2
  exit 1
fi

if ! command -v "$SPKMERGE_BIN" >/dev/null 2>&1; then
  echo "SPKMERGE executable not found: $SPKMERGE_BIN" >&2
  echo "Download NAIF's utility from https://naif.jpl.nasa.gov/pub/naif/utilities/PC_Linux_64bit/spkmerge or set --spkmerge." >&2
  exit 1
fi

md5_for_file() {
  local file_path="$1"
  if command -v md5sum >/dev/null 2>&1; then
    md5sum "$file_path" | awk '{print tolower($1)}'
    return
  fi
  if command -v md5 >/dev/null 2>&1; then
    md5 -q "$file_path" | tr '[:upper:]' '[:lower:]'
    return
  fi
  echo "No MD5 tool found (md5sum or md5)." >&2
  return 1
}

size_for_file() {
  local file_path="$1"
  if stat -c '%s' "$file_path" >/dev/null 2>&1; then
    stat -c '%s' "$file_path"
  else
    stat -f '%z' "$file_path"
  fi
}

tmp_dir="$(mktemp -d)"
trap 'rm -rf "$tmp_dir"' EXIT

updated_manifest="${tmp_dir}/$(basename "$MANIFEST")"
: > "$updated_manifest"

while IFS= read -r line; do
  if [[ -z "${line}" || "${line:0:1}" == "#" ]]; then
    echo "$line" >> "$updated_manifest"
    continue
  fi

  IFS='|' read -r name parent begin_tdb end_tdb source_url bytes md5 precedence notes <<< "$line"

  source_spk="${DATA_DIR}/${parent}"
  output_spk="${DATA_DIR}/${name}"
  config_file="${tmp_dir}/${name}.mrg"
  log_file="${tmp_dir}/${name}.log"

  if [[ ! -f "$source_spk" ]]; then
    echo "Source SPK not found for ${name}: ${source_spk}" >&2
    exit 1
  fi

  cat > "$config_file" <<EOF
LEAPSECONDS_KERNEL = ${LSK}
SPK_KERNEL         = ${output_spk}
  LOG_FILE         = ${log_file}
  BEGIN_TIME       = ${begin_tdb}
  END_TIME         = ${end_tdb}
  SOURCE_SPK_KERNEL = ${source_spk}
    INCLUDE_COMMENTS = yes
EOF

  echo "Generating ${name}"
  rm -f "$output_spk"
  "$SPKMERGE_BIN" "$config_file"

  actual_bytes="$(size_for_file "$output_spk")"
  actual_md5="$(md5_for_file "$output_spk")"
  echo "Generated ${name} (${actual_bytes} bytes, ${actual_md5})"
  echo "${name}|${parent}|${begin_tdb}|${end_tdb}|${source_url}|${actual_bytes}|${actual_md5}|${precedence}|${notes}" >> "$updated_manifest"
done < "$MANIFEST"

if [[ "$UPDATE_MANIFEST" -eq 1 ]]; then
  mv "$updated_manifest" "$MANIFEST"
  echo "Updated manifest: $MANIFEST"
fi
