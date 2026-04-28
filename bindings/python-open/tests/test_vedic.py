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
        from ctara_dhruv.vedic import graha_name, yogini_name
        assert graha_name(0) == "Surya"
        assert graha_name(1) == "Chandra"
        assert graha_name(2) == "Mangal"
        assert graha_name(3) == "Buddh"
        assert yogini_name(0) == "Mangala"
        assert yogini_name(7) == "Sankata"

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


class TestSphutas:
    """Test individual sphuta pure-math functions."""

    def test_bhrigu_bindu(self):
        from ctara_dhruv.vedic import bhrigu_bindu
        result = bhrigu_bindu(120.0, 60.0)
        assert 0 <= result < 360

    def test_prana_sphuta(self):
        from ctara_dhruv.vedic import prana_sphuta
        result = prana_sphuta(100.0, 200.0)
        assert 0 <= result < 360

    def test_deha_sphuta(self):
        from ctara_dhruv.vedic import deha_sphuta
        result = deha_sphuta(200.0, 100.0)
        assert 0 <= result < 360

    def test_mrityu_sphuta(self):
        from ctara_dhruv.vedic import mrityu_sphuta
        result = mrityu_sphuta(150.0, 100.0)
        assert 0 <= result < 360

    def test_tithi_sphuta(self):
        from ctara_dhruv.vedic import tithi_sphuta
        result = tithi_sphuta(200.0, 100.0, 80.0)
        assert 0 <= result < 360

    def test_yoga_sphuta(self):
        from ctara_dhruv.vedic import yoga_sphuta
        result = yoga_sphuta(100.0, 200.0)
        assert result == pytest.approx(300.0)

    def test_yoga_sphuta_normalized(self):
        from ctara_dhruv.vedic import yoga_sphuta_normalized
        result = yoga_sphuta_normalized(200.0, 250.0)
        assert 0 <= result < 360

    def test_rahu_tithi_sphuta(self):
        from ctara_dhruv.vedic import rahu_tithi_sphuta
        result = rahu_tithi_sphuta(120.0, 100.0, 80.0)
        assert 0 <= result < 360

    def test_kshetra_sphuta(self):
        from ctara_dhruv.vedic import all_sphutas, kshetra_sphuta

        sun = 10.0
        moon = 20.0
        mars = 30.0
        jupiter = 40.0
        venus = 50.0
        rahu = 60.0
        lagna = 70.0
        eighth_lord = 80.0
        gulika = 90.0

        result = kshetra_sphuta(moon, mars, jupiter, venus, lagna)
        all_result = all_sphutas(sun, moon, mars, jupiter, venus, rahu, lagna, eighth_lord, gulika)
        # ALL_SPHUTAS order in dhruv_vedic_base: KshetraSphuta is index 8.
        idx = 8
        assert result == pytest.approx(all_result.longitudes[idx], abs=1e-9)
        assert 0 <= result < 360

    def test_beeja_sphuta(self):
        from ctara_dhruv.vedic import beeja_sphuta
        result = beeja_sphuta(100.0, 300.0, 120.0)
        assert 0 <= result < 360

    def test_trisphuta(self):
        from ctara_dhruv.vedic import trisphuta
        result = trisphuta(100.0, 200.0, 80.0)
        assert 0 <= result < 360

    def test_chatussphuta(self):
        from ctara_dhruv.vedic import chatussphuta
        result = chatussphuta(100.0, 200.0)
        assert 0 <= result < 360

    def test_panchasphuta(self):
        from ctara_dhruv.vedic import panchasphuta
        result = panchasphuta(100.0, 120.0)
        assert 0 <= result < 360

    def test_sookshma_trisphuta(self):
        from ctara_dhruv.vedic import sookshma_trisphuta
        result = sookshma_trisphuta(100.0, 200.0, 80.0, 150.0)
        assert 0 <= result < 360

    def test_avayoga_sphuta(self):
        from ctara_dhruv.vedic import avayoga_sphuta
        result = avayoga_sphuta(100.0, 200.0)
        assert 0 <= result < 360

    def test_kunda(self):
        from ctara_dhruv.vedic import kunda
        result = kunda(100.0, 200.0, 50.0)
        assert 0 <= result < 360


class TestSpecialLagnasMath:
    """Test individual special lagna pure-math functions."""

    def test_bhava_lagna(self):
        from ctara_dhruv.vedic import bhava_lagna
        result = bhava_lagna(100.0, 10.0)
        assert 0 <= result < 360

    def test_hora_lagna(self):
        from ctara_dhruv.vedic import hora_lagna
        result = hora_lagna(100.0, 10.0)
        assert 0 <= result < 360

    def test_ghati_lagna(self):
        from ctara_dhruv.vedic import ghati_lagna
        result = ghati_lagna(100.0, 10.0)
        assert 0 <= result < 360

    def test_vighati_lagna(self):
        from ctara_dhruv.vedic import vighati_lagna
        result = vighati_lagna(100.0, 100.0)
        assert 0 <= result < 360

    def test_varnada_lagna(self):
        from ctara_dhruv.vedic import varnada_lagna
        result = varnada_lagna(100.0, 200.0)
        assert 0 <= result < 360

    def test_sree_lagna(self):
        from ctara_dhruv.vedic import sree_lagna
        result = sree_lagna(200.0, 100.0)
        assert 0 <= result < 360

    def test_pranapada_lagna(self):
        from ctara_dhruv.vedic import pranapada_lagna
        result = pranapada_lagna(100.0, 10.0)
        assert 0 <= result < 360

    def test_indu_lagna(self):
        from ctara_dhruv.vedic import indu_lagna
        result = indu_lagna(200.0, 2, 5)  # Mars as lagna lord, Venus as moon 9th lord
        assert 0 <= result < 360


class TestLunarNodeHelpers:
    """Test lunar node count and UTC helpers."""

    def test_lunar_node_count(self):
        from ctara_dhruv.vedic import lunar_node_count
        assert lunar_node_count() == 2  # Rahu, Ketu


@skip_no_kernels
class TestLunarNodeUtc:
    def test_lunar_node_deg_utc(self, engine_handles):
        from ctara_dhruv.vedic import lunar_node_deg_utc
        from ctara_dhruv.types import UtcTime
        from ctara_dhruv.engine import lsk
        utc = UtcTime(2024, 1, 15, 12, 0, 0.0)
        result = lunar_node_deg_utc(lsk(), 0, 0, utc)  # Rahu mean
        assert 0 <= result < 360

    def test_lunar_node_deg_utc_with_engine(self, engine_handles):
        from ctara_dhruv.vedic import lunar_node_deg_utc_with_engine
        from ctara_dhruv.types import UtcTime
        from ctara_dhruv.engine import engine, lsk
        utc = UtcTime(2024, 1, 15, 12, 0, 0.0)
        result = lunar_node_deg_utc_with_engine(
            engine()._ptr, lsk(), 0, 1, utc
        )  # Rahu true with engine
        assert 0 <= result < 360


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
