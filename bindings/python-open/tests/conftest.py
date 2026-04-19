"""Shared fixtures for ctara_dhruv tests."""

import os
import sys
from pathlib import Path

import pytest

# Kernel files location (relative to repo root)
_REPO_ROOT = Path(__file__).parent.parent.parent.parent
_KERNEL_DIR = Path(os.environ.get("CTARA_DHRUV_TEST_KERNEL_DIR", _REPO_ROOT / "kernels" / "data"))
_BSP_PATH = _KERNEL_DIR / "de442s.bsp"
_LSK_PATH = _KERNEL_DIR / "naif0012.tls"
_EOP_PATH = _KERNEL_DIR / "finals2000A.all"
_TARA_PATH = _KERNEL_DIR / "tara_catalog.json"


def _has_kernels():
    return _BSP_PATH.is_file() and _LSK_PATH.is_file()


def _has_eop():
    return _EOP_PATH.is_file()


def _has_tara():
    return _TARA_PATH.is_file()


skip_no_kernels = pytest.mark.skipif(
    not _has_kernels(), reason="Kernel files not found in kernels/data/"
)

skip_no_eop = pytest.mark.skipif(
    not _has_eop(), reason="EOP file not found in kernels/data/"
)

skip_no_tara = pytest.mark.skipif(
    not _has_tara(), reason="Tara catalog not found in kernels/data/"
)


@pytest.fixture(scope="session")
def bsp_path():
    return str(_BSP_PATH)


@pytest.fixture(scope="session")
def lsk_path():
    return str(_LSK_PATH)


@pytest.fixture(scope="session")
def eop_path():
    if _EOP_PATH.is_file():
        return str(_EOP_PATH)
    return None


@pytest.fixture(scope="session")
def tara_path():
    if _TARA_PATH.is_file():
        return str(_TARA_PATH)
    return None


class _EngineAccessor:
    """Proxy that always accesses the current engine singleton.

    Tests that call ``ctara_dhruv.init()`` close the previous engine object.
    This proxy delegates every access to the *current* singleton so that the
    session-scoped fixture never goes stale.
    """

    @property
    def _ptr(self):
        import ctara_dhruv
        return ctara_dhruv.engine()._ptr

    @property
    def api_version(self):
        import ctara_dhruv
        return ctara_dhruv.engine().api_version


@pytest.fixture(scope="session")
def engine_handles(bsp_path, lsk_path, eop_path):
    """Create engine + LSK + EOP handles for session-scoped tests."""
    if not _has_kernels():
        pytest.skip("Kernel files not found in kernels/data/")
    import ctara_dhruv
    ctara_dhruv.init([bsp_path], lsk_path, eop_path)
    yield _EngineAccessor()


@pytest.fixture(autouse=True)
def _ensure_singleton(bsp_path, lsk_path, eop_path):
    """Re-init singleton if a prior test closed it."""
    yield
    if not _has_kernels():
        return
    try:
        import ctara_dhruv
        ctara_dhruv.engine()._ptr
    except RuntimeError:
        import ctara_dhruv
        ctara_dhruv.init([bsp_path], lsk_path, eop_path)
