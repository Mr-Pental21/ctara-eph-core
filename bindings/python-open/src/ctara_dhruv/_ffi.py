"""FFI singleton: loads the dhruv_ffi_c shared library and verifies ABI version.

Usage::

    from ctara_dhruv._ffi import ffi, lib
"""

from __future__ import annotations

import cffi

from ._cdef import CDEF, EXPECTED_API_VERSION
from ._loader import find_library

ffi = cffi.FFI()
ffi.cdef(CDEF)
lib = ffi.dlopen(find_library())

_actual = lib.dhruv_api_version()
if _actual != EXPECTED_API_VERSION:
    raise RuntimeError(
        f"ABI mismatch: Python bindings expect v{EXPECTED_API_VERSION}, "
        f"library is v{_actual}"
    )
