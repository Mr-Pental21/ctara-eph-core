"""Tests for dasha (planetary period) computation."""

import pytest
from conftest import skip_no_kernels, skip_no_eop


# Birth data: Delhi, 2024-01-15 06:00 UTC
BIRTH_UTC = (2024, 1, 15, 6, 0, 0.0)
BIRTH_LOC = (28.6139, 77.2090)


@skip_no_kernels
@skip_no_eop
class TestDashaHierarchy:
    def test_vimshottari_hierarchy(self, engine_handles):
        """Vimshottari dasha hierarchy should have at least 2 levels."""
        from ctara_dhruv.dasha import dasha_hierarchy
        from ctara_dhruv.engine import engine, lsk, eop
        result = dasha_hierarchy(
            engine(), lsk(), eop(),
            jd_utc_birth=BIRTH_UTC,
            location=BIRTH_LOC,
            system=0,  # Vimshottari
            max_level=2,
        )
        assert len(result.levels) >= 2

    def test_vimshottari_level0_has_9_periods(self, engine_handles):
        """Vimshottari Maha Dasha should have 9 periods."""
        from ctara_dhruv.dasha import dasha_hierarchy
        from ctara_dhruv.engine import engine, lsk, eop
        result = dasha_hierarchy(
            engine(), lsk(), eop(),
            jd_utc_birth=BIRTH_UTC,
            location=BIRTH_LOC,
            system=0,
            max_level=0,
        )
        level0 = result.levels[0]
        assert level0.level == 0
        assert len(level0.periods) == 9
        # Periods should be contiguous
        for i in range(len(level0.periods) - 1):
            p = level0.periods[i]
            n = level0.periods[i + 1]
            assert abs(p.end_jd - n.start_jd) < 0.01

    def test_hierarchy_period_fields(self, engine_handles):
        """Each period should have valid entity and JD fields."""
        from ctara_dhruv.dasha import dasha_hierarchy
        from ctara_dhruv.engine import engine, lsk, eop
        result = dasha_hierarchy(
            engine(), lsk(), eop(),
            jd_utc_birth=BIRTH_UTC,
            location=BIRTH_LOC,
            system=0,
            max_level=1,
        )
        for level in result.levels:
            for p in level.periods:
                assert p.entity_type == 0  # Graha for Vimshottari
                assert 0 <= p.entity_index <= 8
                assert p.start_jd < p.end_jd
                assert p.level == level.level


@skip_no_kernels
@skip_no_eop
class TestDashaSnapshot:
    def test_vimshottari_snapshot(self, engine_handles):
        """Snapshot at query time should return active periods."""
        from ctara_dhruv.dasha import dasha_snapshot
        from ctara_dhruv.engine import engine, lsk, eop
        snap = dasha_snapshot(
            engine(), lsk(), eop(),
            jd_utc_birth=BIRTH_UTC,
            jd_utc_query=(2025, 1, 1, 0, 0, 0.0),
            location=BIRTH_LOC,
            system=0,
            max_level=2,
        )
        assert snap.system == 0
        assert len(snap.periods) >= 1
        # Each period should contain the query JD
        for p in snap.periods:
            assert p.start_jd <= snap.query_jd <= p.end_jd


@skip_no_kernels
@skip_no_eop
class TestRashiDasha:
    def test_chara_dasha(self, engine_handles):
        """Chara dasha (system 11) should have 12 Maha periods (rashi-based)."""
        from ctara_dhruv.dasha import dasha_hierarchy
        from ctara_dhruv.engine import engine, lsk, eop
        result = dasha_hierarchy(
            engine(), lsk(), eop(),
            jd_utc_birth=BIRTH_UTC,
            location=BIRTH_LOC,
            system=11,  # Chara
            max_level=0,
        )
        level0 = result.levels[0]
        assert len(level0.periods) == 12
        for p in level0.periods:
            assert p.entity_type == 1  # Rashi
            assert 0 <= p.entity_index <= 11
