#!/usr/bin/env python3
from __future__ import annotations

from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[3]
HEADER_PATH = REPO_ROOT / "crates" / "dhruv_ffi_c" / "include" / "dhruv.h"
CDEF_PATH = REPO_ROOT / "bindings" / "python-open" / "src" / "ctara_dhruv" / "_cdef.py"

START_MARKER = '_RAW_HEADER: str = r"""\n'
END_MARKER = '\n"""\n\n# ---------------------------------------------------------------------------\n# Extract API version from #define in header\n# ---------------------------------------------------------------------------\n'


def main() -> int:
    source = CDEF_PATH.read_text(encoding="utf-8")
    start = source.index(START_MARKER) + len(START_MARKER)
    end = source.index(END_MARKER)
    header = HEADER_PATH.read_text(encoding="utf-8").rstrip("\n")
    updated = source[:start] + header + source[end:]
    CDEF_PATH.write_text(updated, encoding="utf-8")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
