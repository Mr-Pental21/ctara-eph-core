#!/usr/bin/env python3
"""Verify split-kernel manifest checksums and local file presence."""

from __future__ import annotations

import argparse
import hashlib
from pathlib import Path
import sys


def md5(path: Path) -> str:
    h = hashlib.md5()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            h.update(chunk)
    return h.hexdigest()


def iter_rows(manifest: Path):
    with manifest.open("r", encoding="utf-8") as handle:
        for line_no, line in enumerate(handle, 1):
            line = line.rstrip("\n")
            if not line or line.startswith("#"):
                continue
            fields = line.split("|")
            if len(fields) != 9:
                raise ValueError(f"{manifest}:{line_no}: expected 9 fields, found {len(fields)}")
            yield line_no, fields


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--manifest",
        type=Path,
        default=Path("kernels/manifest/de441_de442_splits.tsv"),
    )
    parser.add_argument("--data-dir", type=Path, default=Path("kernels/data"))
    args = parser.parse_args()

    errors = 0
    for line_no, fields in iter_rows(args.manifest):
        name, _parent, _begin, _end, _source, expected_bytes, expected_md5, _precedence, _notes = fields
        path = args.data_dir / name
        if not path.exists():
            print(f"{args.manifest}:{line_no}: missing {path}", file=sys.stderr)
            errors += 1
            continue
        actual_bytes = str(path.stat().st_size)
        if expected_bytes and actual_bytes != expected_bytes:
            print(
                f"{args.manifest}:{line_no}: byte mismatch for {name}: "
                f"expected {expected_bytes}, actual {actual_bytes}",
                file=sys.stderr,
            )
            errors += 1
        actual_md5 = md5(path)
        if expected_md5 and actual_md5 != expected_md5.lower():
            print(
                f"{args.manifest}:{line_no}: md5 mismatch for {name}: "
                f"expected {expected_md5}, actual {actual_md5}",
                file=sys.stderr,
            )
            errors += 1
        print(f"verified {name} {actual_bytes} {actual_md5}")

    return 1 if errors else 0


if __name__ == "__main__":
    raise SystemExit(main())
