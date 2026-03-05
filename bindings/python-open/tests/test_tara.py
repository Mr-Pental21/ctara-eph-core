"""Tests for tara (fixed star) catalog and computation."""

import pytest
from conftest import skip_no_kernels, skip_no_tara


J2000 = 2451545.0


@skip_no_tara
class TestTaraCatalog:
    def test_catalog_load_close(self, tara_path):
        """Load and close a tara catalog."""
        from ctara_dhruv.tara import TaraCatalog
        cat = TaraCatalog(tara_path)
        assert cat._handle is not None
        cat.close()

    def test_catalog_context_manager(self, tara_path):
        """TaraCatalog as context manager."""
        from ctara_dhruv.tara import TaraCatalog
        with TaraCatalog(tara_path) as cat:
            assert cat._handle is not None

    def test_double_close_safe(self, tara_path):
        from ctara_dhruv.tara import TaraCatalog
        cat = TaraCatalog(tara_path)
        cat.close()
        cat.close()  # Should not raise


@skip_no_tara
class TestTaraCompute:
    def test_ecliptic_position(self, tara_path):
        """Compute ecliptic position of star id=1 at J2000."""
        from ctara_dhruv.tara import TaraCatalog, tara_compute, TARA_OUTPUT_ECLIPTIC
        with TaraCatalog(tara_path) as cat:
            result = tara_compute(cat, tara_id=1, jd_tdb=J2000, output_kind=TARA_OUTPUT_ECLIPTIC)
            assert result.output_kind == TARA_OUTPUT_ECLIPTIC
            assert result.ecliptic is not None
            assert 0 <= result.ecliptic.lon_deg < 360

    def test_equatorial_position(self, tara_path):
        """Compute equatorial position of star id=1 at J2000."""
        from ctara_dhruv.tara import TaraCatalog, tara_compute, TARA_OUTPUT_EQUATORIAL
        with TaraCatalog(tara_path) as cat:
            result = tara_compute(cat, tara_id=1, jd_tdb=J2000, output_kind=TARA_OUTPUT_EQUATORIAL)
            assert result.output_kind == TARA_OUTPUT_EQUATORIAL
            assert result.equatorial is not None
            assert 0 <= result.equatorial.ra_deg < 360
            assert -90 <= result.equatorial.dec_deg <= 90

    def test_sidereal_position(self, tara_path):
        """Compute sidereal longitude of star id=1 at J2000."""
        from ctara_dhruv.tara import TaraCatalog, tara_compute, TARA_OUTPUT_SIDEREAL
        with TaraCatalog(tara_path) as cat:
            result = tara_compute(
                cat, tara_id=1, jd_tdb=J2000,
                output_kind=TARA_OUTPUT_SIDEREAL,
                ayanamsha_deg=23.85,
            )
            assert result.output_kind == TARA_OUTPUT_SIDEREAL
            assert result.sidereal_longitude_deg is not None
            assert 0 <= result.sidereal_longitude_deg < 360


@skip_no_tara
class TestGalacticCenter:
    def test_galactic_center_ecliptic(self, tara_path):
        """Galactic center should be near ~266 deg ecliptic longitude."""
        from ctara_dhruv.tara import TaraCatalog, galactic_center_ecliptic
        with TaraCatalog(tara_path) as cat:
            sc = galactic_center_ecliptic(cat, J2000)
            assert 260 < sc.lon_deg < 270
            assert abs(sc.lat_deg) < 10  # Near ecliptic
