"""Tests for shadbala, vimsopaka, and avastha computation."""

import pytest
from conftest import skip_no_kernels, skip_no_eop


@skip_no_kernels
@skip_no_eop
class TestShadbala:
    def test_shadbala_result_structure(self, engine_handles):
        """Shadbala should return 7 entries (sapta grahas only)."""
        from ctara_dhruv.shadbala import shadbala
        from ctara_dhruv.engine import engine, lsk, eop
        result = shadbala(
            engine(), lsk(), eop(),
            jd_utc=(2024, 1, 15, 6, 0, 0.0),
            location=(28.6139, 77.2090),
        )
        assert len(result.entries) == 7
        for i, entry in enumerate(result.entries):
            assert entry.graha_index == i
            assert entry.total_shashtiamsas > 0
            assert entry.total_rupas > 0
            assert entry.required_strength > 0

    def test_shadbala_components_positive(self, engine_handles):
        """Naisargika bala should always be positive for all grahas."""
        from ctara_dhruv.shadbala import shadbala
        from ctara_dhruv.engine import engine, lsk, eop
        result = shadbala(
            engine(), lsk(), eop(),
            jd_utc=(2024, 1, 15, 6, 0, 0.0),
            location=(28.6139, 77.2090),
        )
        for entry in result.entries:
            assert entry.naisargika > 0
            assert entry.sthana.total >= 0
            assert entry.kala.total >= 0


@skip_no_kernels
@skip_no_eop
class TestVimsopaka:
    def test_vimsopaka_result_structure(self, engine_handles):
        """Vimsopaka should return 9 entries (all navagrahas)."""
        from ctara_dhruv.shadbala import vimsopaka
        from ctara_dhruv.engine import engine, lsk, eop
        result = vimsopaka(
            engine(), lsk(), eop(),
            jd_utc=(2024, 1, 15, 6, 0, 0.0),
            location=(28.6139, 77.2090),
        )
        assert len(result.entries) == 9
        for i, entry in enumerate(result.entries):
            assert entry.graha_index == i
            # All scores should be between 0 and 20
            assert 0 <= entry.shadvarga <= 20
            assert 0 <= entry.saptavarga <= 20
            assert 0 <= entry.dashavarga <= 20
            assert 0 <= entry.shodasavarga <= 20


@skip_no_kernels
@skip_no_eop
class TestAvastha:
    def test_avastha_result_structure(self, engine_handles):
        """Avastha should return 9 entries with all 5 categories."""
        from ctara_dhruv.shadbala import avastha
        from ctara_dhruv.engine import engine, lsk, eop
        result = avastha(
            engine(), lsk(), eop(),
            jd_utc=(2024, 1, 15, 6, 0, 0.0),
            location=(28.6139, 77.2090),
        )
        assert len(result.entries) == 9
        for entry in result.entries:
            assert 0 <= entry.baladi <= 4
            assert 0 <= entry.jagradadi <= 2
            assert 0 <= entry.deeptadi <= 8
            assert 0 <= entry.lajjitadi <= 5
            assert 0 <= entry.sayanadi.avastha <= 11
            assert len(entry.sayanadi.sub_states) == 5

    def test_bala_helpers_accept_amsha_selection(self, engine_handles):
        """Standalone bala helpers should accept amsha_selection overrides."""
        from ctara_dhruv.shadbala import avastha, balas, shadbala, vimsopaka
        from ctara_dhruv.engine import engine, lsk, eop

        d2_variation = {"count": 1, "codes": [2], "variations": [1]}
        d9_default = {"count": 1, "codes": [9], "variations": [0]}

        shadbala_result = shadbala(
            engine(), lsk(), eop(),
            jd_utc=(2024, 1, 15, 6, 0, 0.0),
            location=(28.6139, 77.2090),
            amsha_selection=d2_variation,
        )
        assert len(shadbala_result.entries) == 7

        vimsopaka_result = vimsopaka(
            engine(), lsk(), eop(),
            jd_utc=(2024, 1, 15, 6, 0, 0.0),
            location=(28.6139, 77.2090),
            amsha_selection=d2_variation,
        )
        assert len(vimsopaka_result.entries) == 9

        bundle = balas(
            engine(), lsk(), eop(),
            jd_utc=(2024, 1, 15, 6, 0, 0.0),
            location=(28.6139, 77.2090),
            amsha_selection=d2_variation,
        )
        assert len(bundle.shadbala.entries) == 7
        assert len(bundle.vimsopaka.entries) == 9

        avastha_result = avastha(
            engine(), lsk(), eop(),
            jd_utc=(2024, 1, 15, 6, 0, 0.0),
            location=(28.6139, 77.2090),
            amsha_selection=d9_default,
        )
        assert len(avastha_result.entries) == 9
