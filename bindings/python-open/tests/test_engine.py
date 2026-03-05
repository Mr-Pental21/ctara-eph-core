"""Tests for engine lifecycle and initialization."""

import pytest
from conftest import skip_no_kernels


@skip_no_kernels
class TestEngine:
    def test_api_version(self, engine_handles):
        """API version should be at least 42 (unified search ABI)."""
        assert engine_handles.api_version >= 42

    def test_engine_init_close(self, bsp_path, lsk_path):
        from ctara_dhruv.engine import Engine
        eng = Engine([bsp_path], lsk_path)
        assert eng._handle is not None
        eng.close()

    def test_context_manager(self, bsp_path, lsk_path):
        from ctara_dhruv.engine import Engine
        with Engine([bsp_path], lsk_path) as eng:
            assert eng._handle is not None

    def test_double_close_safe(self, bsp_path, lsk_path):
        from ctara_dhruv.engine import Engine
        eng = Engine([bsp_path], lsk_path)
        eng.close()
        eng.close()  # Should not raise

    def test_closed_engine_raises(self, bsp_path, lsk_path):
        from ctara_dhruv.engine import Engine
        eng = Engine([bsp_path], lsk_path)
        eng.close()
        with pytest.raises(RuntimeError, match="Engine is closed"):
            _ = eng._ptr

    def test_too_many_spk_paths_raises(self, bsp_path, lsk_path):
        from ctara_dhruv.engine import Engine
        with pytest.raises(ValueError, match="Too many SPK paths"):
            Engine([bsp_path] * 9, lsk_path)

    def test_singleton_init(self, bsp_path, lsk_path, eop_path):
        from ctara_dhruv.engine import init, engine, lsk
        eng = init([bsp_path], lsk_path, eop_path)
        assert engine() is eng
        assert lsk() is not None

    def test_load_lsk(self, bsp_path, lsk_path):
        from ctara_dhruv.engine import Engine
        with Engine([bsp_path], lsk_path) as eng:
            eng.load_lsk(lsk_path)
            assert eng._lsk is not None
