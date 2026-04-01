#!/usr/bin/env python3
from __future__ import annotations

import argparse
import os
import shutil
import subprocess
import sys
from pathlib import Path


PACKAGE_DIR = Path(__file__).resolve().parents[1] / "src" / "ctara_dhruv"


def shared_library_name() -> str:
    if sys.platform == "darwin":
        return "libdhruv_ffi_c.dylib"
    if sys.platform == "win32":
        return "dhruv_ffi_c.dll"
    return "libdhruv_ffi_c.so"


def resolve_repo_root(explicit: str | None) -> Path:
    candidates: list[Path] = []
    if explicit:
        candidates.append(Path(explicit))
    env_root = os.environ.get("DHRUV_REPO_ROOT")
    if env_root:
        candidates.append(Path(env_root))
    candidates.append(Path(__file__).resolve().parents[3])

    for candidate in candidates:
        cargo_toml = candidate / "Cargo.toml"
        if cargo_toml.is_file():
            return candidate

    checked = ", ".join(str(candidate) for candidate in candidates)
    raise SystemExit(f"unable to locate repo root with Cargo.toml; checked: {checked}")


def main() -> int:
    parser = argparse.ArgumentParser(description="Build and bundle dhruv_ffi_c for Python packaging.")
    parser.add_argument("--repo-root", help="Explicit repository root containing Cargo.toml")
    args = parser.parse_args()

    root = resolve_repo_root(args.repo_root)
    cargo = shutil.which("cargo")
    if cargo is None:
        raise SystemExit(
            "cargo not found in PATH; install Rust toolchain before building Python wheels"
        )

    subprocess.run(
        [
            cargo,
            "build",
            "-p",
            "dhruv_ffi_c",
            "--release",
            "--manifest-path",
            str(root / "Cargo.toml"),
        ],
        check=True,
        cwd=root,
    )

    lib_name = shared_library_name()
    built_lib = root / "target" / "release" / lib_name
    if not built_lib.is_file():
        raise SystemExit(f"expected built library at {built_lib}")

    PACKAGE_DIR.mkdir(parents=True, exist_ok=True)
    bundled_lib = PACKAGE_DIR / lib_name
    shutil.copy2(built_lib, bundled_lib)
    print(bundled_lib)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
