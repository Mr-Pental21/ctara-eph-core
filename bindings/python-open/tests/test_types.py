"""Tests for types and enums modules."""

from datetime import datetime


class TestStateVector:
    def test_immutable(self):
        from ctara_dhruv.types import StateVector
        sv = StateVector(1.0, 2.0, 3.0, 0.1, 0.2, 0.3)
        assert sv.x == 1.0
        assert sv.vz == 0.3
        import pytest
        with pytest.raises(AttributeError):
            sv.x = 99.0


class TestUtcTime:
    def test_to_datetime(self):
        from ctara_dhruv.types import UtcTime
        utc = UtcTime(2024, 6, 15, 10, 30, 45.5)
        dt = utc.to_datetime()
        assert dt.year == 2024
        assert dt.month == 6
        assert dt.day == 15
        assert dt.hour == 10
        assert dt.minute == 30
        assert dt.second == 45
        assert dt.microsecond == 500000

    def test_from_datetime(self):
        from ctara_dhruv.types import UtcTime
        dt = datetime(2024, 1, 1, 12, 0, 0, 500000)
        utc = UtcTime.from_datetime(dt)
        assert utc.year == 2024
        assert utc.month == 1
        assert utc.day == 1
        assert utc.hour == 12
        assert abs(utc.second - 0.5) < 0.001

    def test_roundtrip(self):
        from ctara_dhruv.types import UtcTime
        original = UtcTime(2024, 3, 15, 8, 45, 30.123)
        dt = original.to_datetime()
        back = UtcTime.from_datetime(dt)
        assert back.year == original.year
        assert back.month == original.month
        assert back.day == original.day
        assert back.hour == original.hour
        assert back.minute == original.minute
        assert abs(back.second - original.second) < 0.001


class TestSphericalCoords:
    def test_fields(self):
        from ctara_dhruv.types import SphericalCoords
        sc = SphericalCoords(lon_deg=45.0, lat_deg=10.0, distance_km=1e8)
        assert sc.lon_deg == 45.0
        assert sc.lat_deg == 10.0
        assert sc.distance_km == 1e8


class TestGeoLocation:
    def test_default_altitude(self):
        from ctara_dhruv.types import GeoLocation
        loc = GeoLocation(lat_deg=28.6, lon_deg=77.2)
        assert loc.alt_m == 0.0


class TestAshtakavargaTypes:
    def test_bhinna_has_contributors(self):
        from ctara_dhruv.types import BhinnaAshtakavarga
        b = BhinnaAshtakavarga(
            graha_index=0,
            points=[0] * 12,
            contributors=[[0] * 8 for _ in range(12)],
        )
        assert len(b.points) == 12
        assert len(b.contributors) == 12
        assert len(b.contributors[0]) == 8


class TestEnums:
    def test_body_codes(self):
        from ctara_dhruv.enums import Body
        assert Body.SSB == 0
        assert Body.SUN == 10
        assert Body.MOON == 301
        assert Body.MARS == 4
        assert Body.MARS_BARYCENTER == 4

    def test_status_codes(self):
        from ctara_dhruv.enums import DhruvStatus
        assert DhruvStatus.OK == 0
        assert DhruvStatus.NULL_POINTER == 7
        assert DhruvStatus.INTERNAL == 255

    def test_ayanamsha_systems(self):
        from ctara_dhruv.enums import AyanamshaSystem
        assert AyanamshaSystem.LAHIRI == 0
        assert AyanamshaSystem.TRUE_LAHIRI == 1
        assert AyanamshaSystem.JAGGANATHA == 16
        assert len(AyanamshaSystem) == 20

    def test_graha_enum(self):
        from ctara_dhruv.enums import Graha
        assert Graha.SURYA == 0
        assert Graha.SUN == 0
        assert Graha.KETU == 8

    def test_bhava_systems(self):
        from ctara_dhruv.enums import BhavaSystem
        assert BhavaSystem.EQUAL == 0
        assert len(BhavaSystem) == 10

    def test_dasha_systems(self):
        from ctara_dhruv.enums import DashaSystem
        assert DashaSystem.VIMSHOTTARI == 0
        assert DashaSystem.CHARA == 11
        assert DashaSystem.KARAKA_KENDRADI_GRAHA == 22
        assert len(DashaSystem) == 23

    def test_reference_plane(self):
        from ctara_dhruv.enums import ReferencePlane
        assert ReferencePlane.ECLIPTIC == 0
        assert ReferencePlane.INVARIABLE == 1

    def test_charakaraka_enums(self):
        from ctara_dhruv.enums import CharakarakaScheme, CharakarakaRole
        assert CharakarakaScheme.EIGHT == 0
        assert CharakarakaScheme.SEVEN_NO_PITRI == 1
        assert CharakarakaScheme.SEVEN_PK_MERGED_MK == 2
        assert CharakarakaScheme.MIXED_PARASHARA == 3
        assert CharakarakaRole.ATMA == 0
        assert CharakarakaRole.MATRI_PUTRA == 8
