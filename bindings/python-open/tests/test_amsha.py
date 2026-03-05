"""Tests for amsha (divisional chart) computation."""

import pytest
from conftest import skip_no_kernels, skip_no_eop


class TestAmshaLongitudePureMath:
    def test_d1_identity(self):
        """D1 (rashi chart) should return the same longitude."""
        from ctara_dhruv.amsha import amsha_longitude
        lon = amsha_longitude(45.0, 1)
        assert abs(lon - 45.0) < 0.01

    def test_d9_navamsha(self):
        """D9 Navamsha: 45 deg should map to a valid amsha longitude."""
        from ctara_dhruv.amsha import amsha_longitude
        result = amsha_longitude(45.0, 9)
        assert 0 <= result < 360

    def test_d9_navamsha_boundary(self):
        """D9: 0 deg should map to 0 deg (Mesha, first navamsha)."""
        from ctara_dhruv.amsha import amsha_longitude
        result = amsha_longitude(0.0, 9)
        assert abs(result) < 0.01 or abs(result - 360.0) < 0.01

    def test_d12_dwadashamsha(self):
        """D12: should produce valid longitude."""
        from ctara_dhruv.amsha import amsha_longitude
        result = amsha_longitude(100.0, 12)
        assert 0 <= result < 360

    def test_d60_shastiamsha(self):
        """D60: should produce valid longitude."""
        from ctara_dhruv.amsha import amsha_longitude
        result = amsha_longitude(200.0, 60)
        assert 0 <= result < 360


class TestAmshaRashiInfo:
    def test_amsha_rashi_info_d9(self):
        """Rashi info for D9 should be valid."""
        from ctara_dhruv.amsha import amsha_rashi_info
        ri = amsha_rashi_info(45.0, 9)
        assert 0 <= ri.rashi_index <= 11
        assert 0 <= ri.degrees_in_rashi < 30


class TestAmshaLongitudesBatch:
    def test_batch_multiple_codes(self):
        """Batch computation for multiple D-codes."""
        from ctara_dhruv.amsha import amsha_longitudes
        results = amsha_longitudes(45.0, [1, 9, 12])
        assert len(results) == 3
        for lon in results:
            assert 0 <= lon < 360


@skip_no_kernels
@skip_no_eop
class TestAmshaChartForDate:
    def test_d9_chart_for_date(self, engine_handles):
        """Compute D9 chart for a birth date."""
        from ctara_dhruv.amsha import amsha_chart_for_date
        from ctara_dhruv.engine import engine, lsk, eop
        chart = amsha_chart_for_date(
            engine(), lsk(), eop(),
            jd_utc=(2024, 1, 15, 6, 0, 0.0),
            location=(28.6139, 77.2090),
            amsha_code=9,
        )
        assert chart.amsha_code == 9
        assert len(chart.grahas) == 9
        for g in chart.grahas:
            assert 0 <= g.sidereal_longitude < 360
            assert 0 <= g.rashi_index <= 11
        assert 0 <= chart.lagna.sidereal_longitude < 360
