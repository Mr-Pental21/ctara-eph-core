"""Tests for time conversion functions."""

import pytest
from conftest import skip_no_kernels


@skip_no_kernels
class TestUtcToJdTdb:
    def test_j2000_epoch(self, engine_handles):
        """J2000.0 = 2000-01-01T12:00:00 TDB => JD TDB 2451545.0."""
        from ctara_dhruv.time import utc_to_jd_tdb
        from ctara_dhruv.engine import lsk
        # UTC and TDB differ by ~64s of leap seconds at J2000,
        # so UTC 2000-01-01T12:00:00 maps to JD TDB ~2451545.0007
        from ctara_dhruv.types import UtcTime, UtcToTdbRequest
        jd = utc_to_jd_tdb(lsk(), UtcToTdbRequest(utc=UtcTime(2000, 1, 1, 12, 0, 0.0))).jd_tdb
        assert abs(jd - 2451545.0) < 0.01

    def test_2024_jan_1_noon(self, engine_handles):
        """2024-01-01T12:00:00 UTC => JD TDB ~2460311.0 (noon is 0.5 past midnight JD)."""
        from ctara_dhruv.time import utc_to_jd_tdb
        from ctara_dhruv.engine import lsk
        from ctara_dhruv.types import UtcTime, UtcToTdbRequest
        jd = utc_to_jd_tdb(lsk(), UtcToTdbRequest(utc=UtcTime(2024, 1, 1, 12, 0, 0.0))).jd_tdb
        assert abs(jd - 2460311.0) < 0.01


@skip_no_kernels
class TestJdTdbToUtc:
    def test_roundtrip(self, engine_handles):
        """UTC -> JD TDB -> UTC should round-trip within seconds."""
        from ctara_dhruv.time import utc_to_jd_tdb, jd_tdb_to_utc
        from ctara_dhruv.engine import lsk
        from ctara_dhruv.types import UtcTime, UtcToTdbRequest
        jd = utc_to_jd_tdb(lsk(), UtcToTdbRequest(utc=UtcTime(2024, 6, 15, 10, 30, 0.0))).jd_tdb
        utc = jd_tdb_to_utc(lsk(), jd)
        assert utc.year == 2024
        assert utc.month == 6
        assert utc.day == 15
        assert utc.hour == 10
        assert utc.minute == 30
        assert abs(utc.second) < 2.0  # within TDB-UTC offset

    def test_j2000_inverse(self, engine_handles):
        """JD TDB 2451545.0 should map back near 2000-01-01."""
        from ctara_dhruv.time import jd_tdb_to_utc
        from ctara_dhruv.engine import lsk
        utc = jd_tdb_to_utc(lsk(), 2451545.0)
        assert utc.year == 2000
        assert utc.month == 1
        assert utc.day == 1


@skip_no_kernels
class TestNutation:
    def test_nutation_at_j2000(self, engine_handles):
        """Nutation at J2000 should return non-zero dpsi and deps."""
        from ctara_dhruv.time import nutation
        dpsi, deps = nutation(2451545.0)
        assert abs(dpsi) > 0
        assert abs(deps) > 0
        # IAU 2000B nutation at J2000 is ~-14" dpsi, ~-6" deps
        assert abs(dpsi) < 30
        assert abs(deps) < 20

    def test_nutation_utc(self, engine_handles):
        """Nutation via UTC should match JD TDB variant."""
        from ctara_dhruv.time import nutation, nutation_utc, utc_to_jd_tdb
        from ctara_dhruv.types import UtcTime
        from ctara_dhruv.engine import lsk
        utc = UtcTime(2024, 1, 1, 0, 0, 0.0)
        dpsi_utc, deps_utc = nutation_utc(lsk(), utc)
        from ctara_dhruv.types import UtcToTdbRequest
        jd = utc_to_jd_tdb(lsk(), UtcToTdbRequest(utc=utc)).jd_tdb
        dpsi_jd, deps_jd = nutation(jd)
        # Should be very close (sub-arcsecond)
        assert abs(dpsi_utc - dpsi_jd) < 0.1
        assert abs(deps_utc - deps_jd) < 0.1


class TestApproximateLocalNoon:
    def test_greenwich_noon(self):
        """At Greenwich (lon=0), noon is at 0.5 day offset from midnight."""
        from ctara_dhruv.time import approximate_local_noon_jd
        jd_midnight = 2460310.5  # ~2024-01-01 0h UT
        jd_noon = approximate_local_noon_jd(jd_midnight, 0.0)
        assert abs(jd_noon - (jd_midnight + 0.5)) < 0.01

    def test_east_longitude_shifts_earlier(self):
        """East longitude (e.g. 90E) should shift noon earlier in UT."""
        from ctara_dhruv.time import approximate_local_noon_jd
        jd_midnight = 2460310.5
        jd_noon_greenwich = approximate_local_noon_jd(jd_midnight, 0.0)
        jd_noon_east = approximate_local_noon_jd(jd_midnight, 90.0)
        assert jd_noon_east < jd_noon_greenwich
