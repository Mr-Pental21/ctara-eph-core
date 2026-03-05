"""Tests for panchang computation."""

import pytest
from conftest import skip_no_kernels, skip_no_eop


@skip_no_kernels
@skip_no_eop
class TestPanchangCompute:
    def test_panchang_basic(self, engine_handles):
        """Compute panchang at Delhi for 2024-01-15 with core elements."""
        from ctara_dhruv.panchang import panchang, INCLUDE_ALL_CORE
        from ctara_dhruv.types import UtcTime, GeoLocation
        from ctara_dhruv.engine import engine, lsk, eop
        utc = UtcTime(2024, 1, 15, 12, 0, 0.0)
        delhi = GeoLocation(lat_deg=28.6139, lon_deg=77.2090)
        result = panchang(
            engine()._ptr, eop(), lsk(), utc, delhi,
            include_mask=INCLUDE_ALL_CORE,
        )
        assert result.tithi is not None
        assert 0 <= result.tithi.tithi_index <= 29
        assert result.karana is not None
        assert result.yoga is not None
        assert 0 <= result.yoga.yoga_index <= 26
        assert result.vaar is not None
        assert 0 <= result.vaar.vaar_index <= 6
        assert result.nakshatra is not None
        assert 0 <= result.nakshatra.nakshatra_index <= 26

    def test_panchang_with_calendar(self, engine_handles):
        """Compute panchang with calendar elements (masa, ayana, varsha)."""
        from ctara_dhruv.panchang import panchang, INCLUDE_ALL
        from ctara_dhruv.types import UtcTime, GeoLocation
        from ctara_dhruv.engine import engine, lsk, eop
        utc = UtcTime(2024, 6, 15, 12, 0, 0.0)
        delhi = GeoLocation(lat_deg=28.6139, lon_deg=77.2090)
        result = panchang(
            engine()._ptr, eop(), lsk(), utc, delhi,
            include_mask=INCLUDE_ALL,
        )
        assert result.masa is not None
        assert 0 <= result.masa.masa_index <= 11
        assert result.ayana is not None
        assert result.ayana.ayana in (0, 1)

    def test_panchang_from_jd(self, engine_handles):
        """Compute panchang from JD TDB float input."""
        from ctara_dhruv.panchang import panchang, INCLUDE_TITHI
        from ctara_dhruv.types import GeoLocation
        from ctara_dhruv.engine import engine, lsk, eop
        delhi = GeoLocation(lat_deg=28.6139, lon_deg=77.2090)
        result = panchang(
            engine()._ptr, eop(), lsk(), 2460310.5, delhi,
            include_mask=INCLUDE_TITHI,
        )
        assert result.tithi is not None


@skip_no_kernels
@skip_no_eop
class TestIndividualPanchang:
    def test_tithi_for_date(self, engine_handles):
        from ctara_dhruv.panchang import tithi_for_date
        from ctara_dhruv.types import UtcTime
        from ctara_dhruv.engine import engine
        utc = UtcTime(2024, 1, 15, 12, 0, 0.0)
        t = tithi_for_date(engine()._ptr, utc)
        assert 0 <= t.tithi_index <= 29
        assert t.paksha in (0, 1)
        assert 1 <= t.tithi_in_paksha <= 15

    def test_karana_for_date(self, engine_handles):
        from ctara_dhruv.panchang import karana_for_date
        from ctara_dhruv.types import UtcTime
        from ctara_dhruv.engine import engine
        utc = UtcTime(2024, 1, 15, 12, 0, 0.0)
        k = karana_for_date(engine()._ptr, utc)
        assert 0 <= k.karana_index <= 59

    def test_yoga_for_date(self, engine_handles):
        from ctara_dhruv.panchang import yoga_for_date
        from ctara_dhruv.types import UtcTime
        from ctara_dhruv.engine import engine
        utc = UtcTime(2024, 1, 15, 12, 0, 0.0)
        y = yoga_for_date(engine()._ptr, utc)
        assert 0 <= y.yoga_index <= 26


class TestSamvatsara:
    def test_samvatsara_2024(self):
        """2024 CE should map to a valid 60-year cycle position."""
        from ctara_dhruv.panchang import samvatsara_from_year
        s = samvatsara_from_year(2024)
        assert 0 <= s.samvatsara_index <= 59
        assert 1 <= s.cycle_position <= 60

    def test_samvatsara_2000(self):
        from ctara_dhruv.panchang import samvatsara_from_year
        s = samvatsara_from_year(2000)
        assert 0 <= s.samvatsara_index <= 59
