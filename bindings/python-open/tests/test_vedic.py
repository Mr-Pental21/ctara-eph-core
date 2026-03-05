"""Tests for vedic base functions: rashi, nakshatra, bhava, rise/set, names."""

import pytest
from conftest import skip_no_kernels, skip_no_eop


class TestRashiPureMath:
    def test_rashi_from_longitude_mesha(self):
        """0-30 deg = Mesha (index 0)."""
        from ctara_dhruv.vedic import rashi_from_longitude
        r = rashi_from_longitude(15.0)
        assert r.rashi_index == 0
        assert abs(r.degrees_in_rashi - 15.0) < 0.01

    def test_rashi_from_longitude_vrishabha(self):
        """30-60 deg = Vrishabha (index 1)."""
        from ctara_dhruv.vedic import rashi_from_longitude
        r = rashi_from_longitude(45.0)
        assert r.rashi_index == 1
        assert abs(r.degrees_in_rashi - 15.0) < 0.01

    def test_rashi_from_longitude_meena(self):
        """330-360 deg = Meena (index 11)."""
        from ctara_dhruv.vedic import rashi_from_longitude
        r = rashi_from_longitude(350.0)
        assert r.rashi_index == 11

    def test_rashi_boundary(self):
        """Exactly 30 deg should be Vrishabha (index 1)."""
        from ctara_dhruv.vedic import rashi_from_longitude
        r = rashi_from_longitude(30.0)
        assert r.rashi_index == 1
        assert abs(r.degrees_in_rashi) < 0.01

    def test_rashi_count(self):
        from ctara_dhruv.vedic import rashi_count
        assert rashi_count() == 12


class TestNakshatraPureMath:
    def test_nakshatra_ashwini(self):
        """0-13.333 deg = Ashwini (index 0)."""
        from ctara_dhruv.vedic import nakshatra_from_longitude
        n = nakshatra_from_longitude(5.0)
        assert n.nakshatra_index == 0
        assert n.pada == 2  # 3.333-6.666 is pada 2

    def test_nakshatra_bharani(self):
        """13.333-26.666 deg = Bharani (index 1)."""
        from ctara_dhruv.vedic import nakshatra_from_longitude
        n = nakshatra_from_longitude(15.0)
        assert n.nakshatra_index == 1

    def test_nakshatra_revati(self):
        """346.666-360 deg = Revati (index 26)."""
        from ctara_dhruv.vedic import nakshatra_from_longitude
        n = nakshatra_from_longitude(355.0)
        assert n.nakshatra_index == 26

    def test_nakshatra_pada_range(self):
        """Pada should always be 1-4."""
        from ctara_dhruv.vedic import nakshatra_from_longitude
        for deg in [0.0, 5.0, 10.0, 90.0, 180.0, 270.0, 359.0]:
            n = nakshatra_from_longitude(deg)
            assert 1 <= n.pada <= 4

    def test_nakshatra_count_27(self):
        from ctara_dhruv.vedic import nakshatra_count
        assert nakshatra_count(27) == 27

    def test_nakshatra_count_28(self):
        from ctara_dhruv.vedic import nakshatra_count
        assert nakshatra_count(28) == 28

    def test_nakshatra28_abhijit(self):
        """Abhijit spans ~276.666 to 280.888 in 28-scheme."""
        from ctara_dhruv.vedic import nakshatra28_from_longitude
        n = nakshatra28_from_longitude(278.0)
        assert n.nakshatra_index == 21  # Abhijit


class TestNameLookups:
    def test_rashi_names(self):
        from ctara_dhruv.vedic import rashi_name
        assert rashi_name(0) == "Mesha"
        assert rashi_name(1) == "Vrishabha"
        assert rashi_name(11) == "Meena"

    def test_rashi_name_invalid(self):
        from ctara_dhruv.vedic import rashi_name
        assert rashi_name(12) is None

    def test_nakshatra_names(self):
        from ctara_dhruv.vedic import nakshatra_name
        assert nakshatra_name(0) == "Ashwini"
        assert nakshatra_name(26) == "Revati"

    def test_nakshatra_name_invalid(self):
        from ctara_dhruv.vedic import nakshatra_name
        assert nakshatra_name(27) is None

    def test_graha_names(self):
        from ctara_dhruv.vedic import graha_name, graha_english_name
        assert graha_name(0) == "Surya"
        assert graha_english_name(0) == "Sun"
        assert graha_name(1) == "Chandra"
        assert graha_english_name(1) == "Moon"

    def test_tithi_name(self):
        from ctara_dhruv.vedic import tithi_name
        name = tithi_name(0)
        assert name is not None
        assert len(name) > 0


class TestRashiLord:
    def test_mesha_lord_is_mars(self):
        """Lord of Mesha (0) is Mangal (graha index 2)."""
        from ctara_dhruv.vedic import rashi_lord
        assert rashi_lord(0) == 2

    def test_vrishabha_lord_is_venus(self):
        """Lord of Vrishabha (1) is Shukra (graha index 5)."""
        from ctara_dhruv.vedic import rashi_lord
        assert rashi_lord(1) == 5

    def test_invalid_rashi_returns_negative(self):
        from ctara_dhruv.vedic import rashi_lord
        assert rashi_lord(12) == -1


class TestDms:
    def test_deg_to_dms(self):
        """Convert 45.5083 degrees to DMS."""
        from ctara_dhruv.vedic import deg_to_dms
        dms = deg_to_dms(45.5083)
        assert dms.degrees == 45
        assert dms.minutes == 30
        assert abs(dms.seconds - 29.88) < 1.0


class TestTithiPureMath:
    def test_tithi_from_elongation(self):
        """Tithi from elongation: 0 deg = Shukla Pratipada, 180 deg = Purnima."""
        from ctara_dhruv.vedic import tithi_from_elongation
        t0 = tithi_from_elongation(6.0)
        assert t0.tithi_index == 0
        assert t0.paksha == 0  # Shukla

    def test_tithi_purnima(self):
        """Elongation ~180 deg = Purnima (tithi_index 14)."""
        from ctara_dhruv.vedic import tithi_from_elongation
        t = tithi_from_elongation(175.0)
        assert t.tithi_index == 14
        assert t.paksha == 0
        assert t.tithi_in_paksha == 15

    def test_karana_from_elongation(self):
        from ctara_dhruv.vedic import karana_from_elongation
        k = karana_from_elongation(3.0)
        assert k.karana_index == 0
        assert 0 <= k.degrees_in_karana < 6

    def test_yoga_from_sum(self):
        from ctara_dhruv.vedic import yoga_from_sum
        y = yoga_from_sum(10.0)
        assert y.yoga_index == 0
        assert 0 <= y.degrees_in_yoga < 13.34


@skip_no_kernels
@skip_no_eop
class TestSunrise:
    def test_sunrise_delhi(self, engine_handles):
        """Sunrise at Delhi (~28.6N, 77.2E) should return a valid event."""
        from ctara_dhruv.vedic import sunrise
        from ctara_dhruv.types import GeoLocation
        from ctara_dhruv.engine import engine, lsk, eop
        delhi = GeoLocation(lat_deg=28.6139, lon_deg=77.2090, alt_m=0.0)
        # Approximate local noon JD for 2024-01-15
        jd_noon = 2460324.0
        result = sunrise(engine()._ptr, lsk(), eop(), delhi, jd_noon)
        assert result.result_type == 0  # Event found
        assert result.jd_tdb > 0


@skip_no_kernels
@skip_no_eop
class TestBhava:
    def test_bhava_system_count(self, engine_handles):
        from ctara_dhruv.vedic import bhava_system_count
        assert bhava_system_count() == 10

    def test_bhava_compute(self, engine_handles):
        """Compute bhava at Delhi for a known date, check 12 houses."""
        from ctara_dhruv.vedic import compute_bhavas_utc
        from ctara_dhruv.types import GeoLocation, UtcTime
        from ctara_dhruv.engine import engine, lsk, eop
        delhi = GeoLocation(lat_deg=28.6139, lon_deg=77.2090)
        utc = UtcTime(2024, 1, 15, 6, 0, 0.0)
        result = compute_bhavas_utc(engine()._ptr, lsk(), eop(), delhi, utc)
        assert len(result.bhavas) == 12
        for b in result.bhavas:
            assert 1 <= b.number <= 12
            assert 0 <= b.cusp_deg < 360
