"""Tests for search APIs: conjunction, eclipse, motion, lunar phase, sankranti."""

import pytest
from conftest import skip_no_kernels


J2000 = 2451545.0


@skip_no_kernels
class TestConjunction:
    def test_next_sun_moon_conjunction(self, engine_handles):
        """Find next Sun-Moon conjunction after J2000."""
        from ctara_dhruv.search import next_conjunction
        evt = next_conjunction(
            engine_handles._ptr,
            body1_code=10, body2_code=301,
            after_jd_tdb=J2000,
        )
        assert evt is not None
        assert evt.jd_tdb > J2000
        # Separation should be near zero for a conjunction
        assert evt.actual_separation_deg < 5.0

    def test_prev_conjunction(self, engine_handles):
        """Find previous Sun-Moon conjunction before J2000."""
        from ctara_dhruv.search import prev_conjunction
        evt = prev_conjunction(
            engine_handles._ptr,
            body1_code=10, body2_code=301,
            before_jd_tdb=J2000,
        )
        assert evt is not None
        assert evt.jd_tdb < J2000

    def test_range_conjunctions(self, engine_handles):
        """Search for Sun-Moon conjunctions in a 365-day window."""
        from ctara_dhruv.search import search_conjunctions
        events = search_conjunctions(
            engine_handles._ptr,
            body1_code=10, body2_code=301,
            start_jd=J2000, end_jd=J2000 + 365.0,
            max_results=1,
        )
        # Should find ~12-13 new moons in a year
        assert 11 <= len(events) <= 14
        # All events should be in range
        for e in events:
            assert J2000 <= e.jd_tdb <= J2000 + 365.0


@skip_no_kernels
class TestEclipse:
    def test_next_lunar_eclipse(self, engine_handles):
        """Find next lunar eclipse after J2000."""
        from ctara_dhruv.search import next_lunar_eclipse
        result = next_lunar_eclipse(engine_handles._ptr, after_jd=J2000)
        assert result is not None
        assert result.greatest_grahan_jd > J2000
        assert result.grahan_type in (0, 1, 2)

    def test_next_solar_eclipse(self, engine_handles):
        """Find next solar eclipse after J2000."""
        from ctara_dhruv.search import next_solar_eclipse
        result = next_solar_eclipse(engine_handles._ptr, after_jd=J2000)
        assert result is not None
        assert result.greatest_grahan_jd > J2000
        assert result.grahan_type in (0, 1, 2, 3)


@skip_no_kernels
class TestMotion:
    def test_next_mars_stationary(self, engine_handles):
        """Find next Mars stationary point after J2000."""
        from ctara_dhruv.search import next_stationary
        evt = next_stationary(engine_handles._ptr, body_code=499, after_jd=J2000)
        assert evt is not None
        assert evt.jd_tdb > J2000
        assert evt.station_type in (0, 1)  # retrograde or direct

    def test_next_jupiter_max_speed(self, engine_handles):
        """Find next Jupiter max-speed event after J2000."""
        from ctara_dhruv.search import next_max_speed
        evt = next_max_speed(engine_handles._ptr, body_code=599, after_jd=J2000)
        assert evt is not None
        assert evt.jd_tdb > J2000
        assert abs(evt.speed_deg_per_day) > 0


@skip_no_kernels
class TestLunarPhase:
    def test_next_purnima(self, engine_handles):
        """Find next full moon after J2000."""
        from ctara_dhruv.search import next_purnima
        evt = next_purnima(engine_handles._ptr, after_jd=J2000)
        assert evt is not None
        assert evt.phase == 1  # PURNIMA

    def test_next_amavasya(self, engine_handles):
        """Find next new moon after J2000."""
        from ctara_dhruv.search import next_amavasya
        evt = next_amavasya(engine_handles._ptr, after_jd=J2000)
        assert evt is not None
        assert evt.phase == 0  # AMAVASYA

    def test_range_purnima(self, engine_handles):
        """Search for full moons in a year-long window."""
        from ctara_dhruv.search import search_lunar_phases
        events = search_lunar_phases(
            engine_handles._ptr,
            phase_kind=1,  # Purnima
            start_jd=J2000, end_jd=J2000 + 365.0,
            max_results=1,
        )
        assert 12 <= len(events) <= 14


@skip_no_kernels
class TestSankranti:
    def test_next_sankranti(self, engine_handles):
        """Find next sankranti (any rashi) after J2000."""
        from ctara_dhruv.search import next_sankranti
        evt = next_sankranti(engine_handles._ptr, after_jd=J2000)
        assert evt is not None
        assert 0 <= evt.rashi_index <= 11

    def test_specific_mesha_sankranti(self, engine_handles):
        """Find Mesha sankranti in both directions through one helper."""
        from ctara_dhruv.search import specific_sankranti
        evt = specific_sankranti(
            engine_handles._ptr, at_jd=J2000, rashi_index=0, direction="next",
        )
        assert evt is not None
        assert evt.rashi_index == 0
        prev_evt = specific_sankranti(
            engine_handles._ptr, at_jd=J2000, rashi_index=0, direction="prev",
        )
        assert prev_evt is not None
        assert prev_evt.rashi_index == 0

    def test_range_sankrantis(self, engine_handles):
        """Search for sankrantis in a year should find ~12."""
        from ctara_dhruv.search import search_sankrantis
        events = search_sankrantis(
            engine_handles._ptr,
            start_jd=J2000, end_jd=J2000 + 365.25,
            max_results=1,
        )
        assert len(events) == 12
