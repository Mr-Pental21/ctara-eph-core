"""Engine lifecycle and initialization."""

from __future__ import annotations

import threading

from ._ffi import ffi, lib
from ._check import check

_DHRUV_PATH_CAPACITY = 512
_DHRUV_MAX_SPK_PATHS = 8


class Engine:
    """Ephemeris engine wrapping a DhruvEngineHandle."""

    def __init__(
        self,
        spk_paths: list[str],
        lsk_path: str | None = None,
        cache_capacity: int = 256,
        strict_validation: bool = True,
    ):
        if len(spk_paths) > _DHRUV_MAX_SPK_PATHS:
            raise ValueError(f"Too many SPK paths (max {_DHRUV_MAX_SPK_PATHS})")

        cfg = ffi.new("DhruvEngineConfig *")
        cfg.spk_path_count = len(spk_paths)
        for i, p in enumerate(spk_paths):
            p_bytes = p.encode("utf-8")
            ffi.memmove(cfg.spk_paths_utf8[i], p_bytes, len(p_bytes))
        if lsk_path:
            lsk_bytes = lsk_path.encode("utf-8")
            ffi.memmove(cfg.lsk_path_utf8, lsk_bytes, len(lsk_bytes))
        cfg.cache_capacity = cache_capacity
        cfg.strict_validation = 1 if strict_validation else 0

        handle = ffi.new("DhruvEngineHandle **")
        check(lib.dhruv_engine_new(cfg, handle), "engine_new")
        self._handle = handle[0]
        self._lsk = ffi.NULL
        self._eop = ffi.NULL

    @property
    def _ptr(self):
        if self._handle == ffi.NULL:
            raise RuntimeError("Engine is closed")
        return self._handle

    def load_lsk(self, path: str) -> None:
        """Load a standalone LSK handle via dhruv_lsk_load."""
        lsk_handle = ffi.new("DhruvLskHandle **")
        check(lib.dhruv_lsk_load(path.encode("utf-8"), lsk_handle), "lsk_load")
        self._lsk = lsk_handle[0]

    def load_eop(self, path: str) -> None:
        """Load an EOP handle via dhruv_eop_load."""
        if self._eop != ffi.NULL:
            lib.dhruv_eop_free(self._eop)
        eop_handle = ffi.new("DhruvEopHandle **")
        check(lib.dhruv_eop_load(path.encode("utf-8"), eop_handle), "eop_load")
        self._eop = eop_handle[0]

    def load_config(
        self, path: str | None = None, defaults_mode: int = 0
    ) -> None:
        """Load layered config via dhruv_config_load."""
        path_ptr = ffi.NULL if path is None else path.encode("utf-8")
        cfg_handle = ffi.new("DhruvConfigHandle **")
        check(
            lib.dhruv_config_load(path_ptr, defaults_mode, cfg_handle),
            "config_load",
        )
        # Handle is kept active by the library's internal resolver;
        # we free it here since activation already happened.
        lib.dhruv_config_free(cfg_handle[0])

    def clear_config(self) -> None:
        """Clear the active layered config via dhruv_config_clear_active."""
        check(lib.dhruv_config_clear_active(), "config_clear_active")

    @property
    def api_version(self) -> int:
        """Return the ABI version number."""
        return lib.dhruv_api_version()

    def close(self) -> None:
        if self._handle != ffi.NULL:
            if self._eop != ffi.NULL:
                lib.dhruv_eop_free(self._eop)
                self._eop = ffi.NULL
            if self._lsk != ffi.NULL:
                lib.dhruv_lsk_free(self._lsk)
                self._lsk = ffi.NULL
            lib.dhruv_engine_free(self._handle)
            self._handle = ffi.NULL

    def __enter__(self):
        return self

    def __exit__(self, *args):
        self.close()

    def __del__(self):
        pass  # Don't rely on __del__ per design decision


# ---------------------------------------------------------------------------
# Module-level singleton
# ---------------------------------------------------------------------------

_lock = threading.Lock()
_engine: Engine | None = None
_lsk = None
_eop = None


def init(
    spk_paths: list[str],
    lsk_path: str,
    eop_path: str | None = None,
    **kw,
) -> Engine:
    """Initialize the global engine singleton."""
    global _engine, _lsk, _eop
    with _lock:
        if _engine is not None:
            _engine.close()
        _engine = Engine(spk_paths, lsk_path, **kw)
        # Load standalone LSK for functions that need it
        lsk_handle = ffi.new("DhruvLskHandle **")
        check(lib.dhruv_lsk_load(lsk_path.encode("utf-8"), lsk_handle), "lsk_load")
        _lsk = lsk_handle[0]
        if eop_path:
            eop_handle = ffi.new("DhruvEopHandle **")
            check(
                lib.dhruv_eop_load(eop_path.encode("utf-8"), eop_handle), "eop_load"
            )
            _eop = eop_handle[0]
        return _engine


def engine() -> Engine:
    """Return the global engine singleton, or raise if not initialized."""
    if _engine is None:
        raise RuntimeError("Call ctara_dhruv.init() first")
    return _engine


def lsk():
    """Return the global LSK handle, or raise if not initialized."""
    if _lsk is None:
        raise RuntimeError("Call ctara_dhruv.init() first")
    return _lsk


def eop():
    """Return the global EOP handle, or None if not loaded."""
    return _eop
