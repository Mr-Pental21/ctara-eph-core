"""Tests for ayanamsha computation."""

import pytest
from conftest import skip_no_kernels


@skip_no_kernels
class TestAyanamshaCompute:
    def test_system_count(self, engine_handles):
        """There should be 20 supported ayanamsha systems."""
        from ctara_dhruv.ayanamsha import system_count
        assert system_count() == 20

    def test_lahiri_at_2024(self, engine_handles):
        """Lahiri (system 0) at ~Jan 1 2024 should be ~24.17 degrees."""
        from ctara_dhruv.ayanamsha import ayanamsha
        from ctara_dhruv.engine import lsk
        aya = ayanamsha(lsk(), system=0, jd_tdb=2460310.5)
        assert 24.0 < aya < 25.0

    def test_lahiri_at_j2000(self, engine_handles):
        """Lahiri at J2000.0 should be ~23.85 degrees."""
        from ctara_dhruv.ayanamsha import ayanamsha
        from ctara_dhruv.engine import lsk
        aya = ayanamsha(lsk(), system=0, jd_tdb=2451545.0)
        assert 23.5 < aya < 24.2

    def test_raman_system(self, engine_handles):
        """Raman (system 3) should be reasonably close to Lahiri."""
        from ctara_dhruv.ayanamsha import ayanamsha
        from ctara_dhruv.engine import lsk
        raman = ayanamsha(lsk(), system=3, jd_tdb=2460310.5)
        lahiri = ayanamsha(lsk(), system=0, jd_tdb=2460310.5)
        assert abs(raman - lahiri) < 5.0  # Different systems, but same order

    def test_ayanamsha_from_utc(self, engine_handles):
        """Ayanamsha via UtcTime input should match JD TDB variant."""
        from ctara_dhruv.ayanamsha import ayanamsha
        from ctara_dhruv.time import utc_to_jd_tdb
        from ctara_dhruv.types import UtcTime
        from ctara_dhruv.engine import lsk
        utc = UtcTime(2024, 1, 1, 12, 0, 0.0)
        from ctara_dhruv.types import UtcTime, UtcToTdbRequest
        jd = utc_to_jd_tdb(lsk(), UtcToTdbRequest(utc=UtcTime(2024, 1, 1, 12, 0, 0.0))).jd_tdb
        aya_utc = ayanamsha(lsk(), system=0, utc=utc)
        aya_jd = ayanamsha(lsk(), system=0, jd_tdb=jd)
        assert abs(aya_utc - aya_jd) < 0.001

    def test_all_systems_produce_values(self, engine_handles):
        """All 20 systems should produce non-negative ayanamsha at J2000."""
        from ctara_dhruv.ayanamsha import ayanamsha, system_count
        from ctara_dhruv.engine import lsk
        for sys_code in range(system_count()):
            aya = ayanamsha(lsk(), system=sys_code, jd_tdb=2451545.0)
            # All major systems have positive ayanamsha near J2000
            assert aya > 0, f"System {sys_code} returned non-positive: {aya}"


@skip_no_kernels
class TestReferencePlane:
    def test_lahiri_ecliptic_default(self, engine_handles):
        """Lahiri default plane should be ecliptic (0)."""
        from ctara_dhruv.ayanamsha import reference_plane_default
        assert reference_plane_default(0) == 0

    def test_jagganatha_invariable(self, engine_handles):
        """Jagganatha (system 16) default plane should be invariable (1)."""
        from ctara_dhruv.ayanamsha import reference_plane_default
        assert reference_plane_default(16) == 1
