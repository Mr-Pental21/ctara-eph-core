"""Library discovery for the dhruv_ffi_c shared library.

Search order:
  1. DHRUV_LIB_PATH environment variable (explicit file path)
  2. Same directory as this file (bundled in wheel)
  3. ctypes.util.find_library("dhruv_ffi_c") (system-installed)
  4. <repo_root>/target/release/<platform_lib_name> (dev builds)
"""

from __future__ import annotations

import ctypes.util
import os
import sys
from pathlib import Path


def _platform_lib_name() -> str:
    """Return the platform-specific shared library filename."""
    if sys.platform == "darwin":
        return "libdhruv_ffi_c.dylib"
    if sys.platform == "win32":
        return "dhruv_ffi_c.dll"
    return "libdhruv_ffi_c.so"


def find_library() -> str:
    """Locate the dhruv_ffi_c shared library.

    Returns the absolute path to the library.

    Raises:
        FileNotFoundError: If the library cannot be found.
    """
    lib_name = _platform_lib_name()

    # Step 1: Explicit environment variable.
    env_path = os.environ.get("DHRUV_LIB_PATH")
    if env_path:
        p = Path(env_path)
        if p.is_file():
            return str(p.resolve())
        raise FileNotFoundError(
            f"DHRUV_LIB_PATH is set to '{env_path}' but the file does not exist"
        )

    # Step 2: Bundled alongside this file (wheel layout).
    here = Path(__file__).resolve().parent
    bundled = here / lib_name
    if bundled.is_file():
        return str(bundled)

    # Step 3: System library search via ctypes.
    system_path = ctypes.util.find_library("dhruv_ffi_c")
    if system_path:
        return system_path

    # Step 4: Dev build in repo target/release.
    # _loader.py is at bindings/python-open/src/ctara_dhruv/_loader.py
    # repo root is 4 parents up from the package directory.
    repo_root = here.parent.parent.parent.parent
    dev_lib = repo_root / "target" / "release" / lib_name
    if dev_lib.is_file():
        return str(dev_lib)

    raise FileNotFoundError(
        f"Cannot find {lib_name}. Set DHRUV_LIB_PATH, install the library, "
        f"or run 'cargo build --release -p dhruv_ffi_c'."
    )
