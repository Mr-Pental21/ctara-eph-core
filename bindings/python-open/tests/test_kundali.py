"""Tests for graha positions, core bindus, and full kundali."""

import pytest
from conftest import skip_no_kernels, skip_no_eop


J2000 = 2451545.0


@skip_no_kernels
class TestGrahaLongitudes:
    def test_graha_sidereal_longitudes(self, engine_handles):
        """Sidereal longitudes for all 9 grahas should be in [0, 360)."""
        from ctara_dhruv.kundali import graha_longitudes
        from ctara_dhruv.engine import engine
        result = graha_longitudes(engine(), jd_tdb=J2000, ayanamsha_system=0)
        assert len(result.longitudes) == 9
        for lon in result.longitudes:
            assert 0 <= lon < 360

    def test_graha_tropical_longitudes(self, engine_handles):
        """Tropical longitudes should differ from sidereal by ~ayanamsha."""
        from ctara_dhruv.kundali import graha_longitudes, graha_tropical_longitudes
        from ctara_dhruv.engine import engine
        sid = graha_longitudes(engine(), jd_tdb=J2000, ayanamsha_system=0)
        trop = graha_tropical_longitudes(engine(), jd_tdb=J2000)
        # Difference should be approximately the ayanamsha (~23.85 at J2000)
        diff = (trop.longitudes[0] - sid.longitudes[0]) % 360
        assert 23.0 < diff < 25.0


@skip_no_kernels
@skip_no_eop
class TestGrahaPositions:
    def test_graha_positions_basic(self, engine_handles):
        """Compute graha positions with default config."""
        from ctara_dhruv.kundali import graha_positions
        from ctara_dhruv.engine import engine, lsk, eop
        result = graha_positions(
            engine(), lsk(), eop(),
            jd_utc=(2024, 1, 15, 6, 0, 0.0),
            location=(28.6139, 77.2090),
        )
        assert len(result.grahas) == 9
        for g in result.grahas:
            assert 0 <= g.sidereal_longitude < 360
            assert 0 <= g.rashi_index <= 11

    def test_graha_positions_with_lagna(self, engine_handles):
        """Compute with include_lagna flag."""
        from ctara_dhruv.kundali import graha_positions
        from ctara_dhruv.engine import engine, lsk, eop
        result = graha_positions(
            engine(), lsk(), eop(),
            jd_utc=(2024, 1, 15, 6, 0, 0.0),
            location=(28.6139, 77.2090),
            config={"include_lagna": 1},
        )
        # Lagna should have a valid longitude
        assert 0 <= result.lagna.sidereal_longitude < 360


@skip_no_kernels
@skip_no_eop
class TestFullKundali:
    def test_full_kundali_default(self, engine_handles):
        """Full kundali with default config should have core sections."""
        from ctara_dhruv.kundali import full_kundali
        from ctara_dhruv.engine import engine, lsk, eop
        result = full_kundali(
            engine(), lsk(), eop(),
            jd_utc=(2024, 1, 15, 6, 0, 0.0),
            location=(28.6139, 77.2090),
        )
        assert result.ayanamsha_deg > 0
        # Default config should include bhava, graha, etc.
        if result.graha_positions is not None:
            assert len(result.graha_positions.grahas) == 9
        if result.bhava_cusps is not None:
            assert len(result.bhava_cusps.bhavas) == 12
        assert result.sphutas is not None
        assert len(result.sphutas.longitudes) == 16

    def test_full_kundali_dasha_hierarchies(self, engine_handles):
        """Full kundali should expose decoded dasha hierarchies with per-system depth."""
        from ctara_dhruv.kundali import full_kundali, full_kundali_config_default
        from ctara_dhruv.engine import engine, lsk, eop

        cfg = full_kundali_config_default()
        cfg.include_dasha = 1
        cfg.dasha_config.count = 2
        cfg.dasha_config.systems[0] = 0
        cfg.dasha_config.systems[1] = 1
        cfg.dasha_config.max_levels[0] = 0
        cfg.dasha_config.max_levels[1] = 1

        result = full_kundali(
            engine(), lsk(), eop(),
            jd_utc=(2024, 1, 15, 6, 0, 0.0),
            location=(28.6139, 77.2090),
            config=cfg,
        )

        assert result.dasha is not None
        assert len(result.dasha) == 2
        assert result.dasha[0].system == 0
        assert result.dasha[1].system == 1
        assert len(result.dasha[0].levels) == 1
        assert len(result.dasha[1].levels) == 2

    def test_full_kundali_ashtakavarga(self, engine_handles):
        """Ashtakavarga in full kundali should have 7 BAVs."""
        from ctara_dhruv.kundali import full_kundali
        from ctara_dhruv.engine import engine, lsk, eop
        result = full_kundali(
            engine(), lsk(), eop(),
            jd_utc=(2024, 1, 15, 6, 0, 0.0),
            location=(28.6139, 77.2090),
        )
        if result.ashtakavarga is not None:
            assert len(result.ashtakavarga.bavs) == 7
            for bav in result.ashtakavarga.bavs:
                assert len(bav.points) == 12
                assert len(bav.contributors) == 12
                for p in bav.points:
                    assert 0 <= p <= 8
                for i in range(12):
                    assert len(bav.contributors[i]) == 8
                    assert sum(bav.contributors[i]) == bav.points[i]
            sav = result.ashtakavarga.sav
            assert len(sav.total_points) == 12
            assert sum(sav.total_points) == 337  # SAV constant

    def test_full_kundali_amsha_scope_and_selection(self, engine_handles):
        """Full kundali should expose scoped amsha chart sections when selected."""
        from ctara_dhruv.kundali import full_kundali, full_kundali_config_default
        from ctara_dhruv.engine import engine, lsk, eop

        cfg = full_kundali_config_default()
        cfg.include_bhava_cusps = 1
        cfg.include_bindus = 1
        cfg.include_upagrahas = 1
        cfg.include_sphutas = 1
        cfg.include_special_lagnas = 1
        cfg.include_amshas = 1
        cfg.amsha_selection.count = 1
        cfg.amsha_selection.codes[0] = 9
        cfg.amsha_selection.variations[0] = 0
        cfg.amsha_scope.include_bhava_cusps = 1
        cfg.amsha_scope.include_arudha_padas = 1
        cfg.amsha_scope.include_upagrahas = 1
        cfg.amsha_scope.include_sphutas = 1
        cfg.amsha_scope.include_special_lagnas = 1

        result = full_kundali(
            engine(), lsk(), eop(),
            jd_utc=(2024, 1, 15, 6, 0, 0.0),
            location=(28.6139, 77.2090),
            config=cfg,
        )

        assert result.amshas is not None
        assert len(result.amshas) == 1
        chart = result.amshas[0]
        assert chart.amsha_code == 9
        assert chart.bhava_cusps is not None
        assert len(chart.bhava_cusps) == 12
        assert chart.arudha_padas is not None
        assert len(chart.arudha_padas) == 12
        assert chart.upagrahas is not None
        assert len(chart.upagrahas) == 11
        assert chart.sphutas is not None
        assert len(chart.sphutas) == 16
        assert chart.special_lagnas is not None
        assert len(chart.special_lagnas) == 8

    def test_charakaraka_for_date(self, engine_handles):
        """Direct charakaraka call should return non-empty assignments."""
        from ctara_dhruv.kundali import charakaraka_for_date
        from ctara_dhruv.engine import engine, lsk, eop
        result = charakaraka_for_date(
            engine(), lsk(), eop(),
            jd_utc=(2024, 1, 15, 6, 0, 0.0),
            ayanamsha_system=0,
            use_nutation=1,
            scheme=3,  # mixed parashara
        )
        assert result.scheme == 3
        assert len(result.entries) >= 7
        assert len(result.entries) <= 8
        assert result.entries[0].rank == 1

    def test_full_kundali_charakaraka_section(self, engine_handles):
        """Full kundali should include charakaraka when enabled in config."""
        from ctara_dhruv.kundali import full_kundali, full_kundali_config_default
        from ctara_dhruv.engine import engine, lsk, eop

        cfg = full_kundali_config_default()
        cfg.include_charakaraka = 1
        cfg.charakaraka_scheme = 2  # seven-pk-merged-mk

        result = full_kundali(
            engine(), lsk(), eop(),
            jd_utc=(2024, 1, 15, 6, 0, 0.0),
            location=(28.6139, 77.2090),
            config=cfg,
        )
        assert result.charakaraka is not None
        assert result.charakaraka.scheme == 2
        assert len(result.charakaraka.entries) == 7
