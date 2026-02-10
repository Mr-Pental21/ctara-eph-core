//! Types for sunrise/sunset and twilight calculations.
//!
//! Provides geographic location, event types, configuration, and result types
//! used by the rise/set computation module.

use std::f64::consts::PI;

/// Mean Earth radius in meters (IAU nominal, for geometric dip).
const EARTH_RADIUS_M: f64 = 6_371_000.0;

/// Geographic location on Earth's surface.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GeoLocation {
    /// Geodetic latitude in degrees, north positive. Range: [-90, 90].
    pub latitude_deg: f64,
    /// Geodetic longitude in degrees, east positive. Range: [-180, 180].
    pub longitude_deg: f64,
    /// Altitude above mean sea level in meters.
    pub altitude_m: f64,
}

impl GeoLocation {
    /// Create a new geographic location.
    pub fn new(latitude_deg: f64, longitude_deg: f64, altitude_m: f64) -> Self {
        Self {
            latitude_deg,
            longitude_deg,
            altitude_m,
        }
    }

    /// Latitude in radians.
    pub fn latitude_rad(&self) -> f64 {
        self.latitude_deg.to_radians()
    }

    /// Longitude in radians (east positive).
    pub fn longitude_rad(&self) -> f64 {
        self.longitude_deg.to_radians()
    }
}

/// Rise/set event types, including twilight variants.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RiseSetEvent {
    /// Sunrise: upper limb of the Sun at the geometric horizon,
    /// accounting for atmospheric refraction and solar semidiameter.
    /// Depression angle: ~0.8333 deg (50 arcmin).
    Sunrise,
    /// Sunset: upper limb disappears below the horizon.
    Sunset,
    /// Civil dawn: Sun center at -6 deg below horizon.
    CivilDawn,
    /// Civil dusk: Sun center at -6 deg below horizon.
    CivilDusk,
    /// Nautical dawn: Sun center at -12 deg below horizon.
    NauticalDawn,
    /// Nautical dusk: Sun center at -12 deg below horizon.
    NauticalDusk,
    /// Astronomical dawn: Sun center at -18 deg below horizon.
    AstronomicalDawn,
    /// Astronomical dusk: Sun center at -18 deg below horizon.
    AstronomicalDusk,
}

impl RiseSetEvent {
    /// Depression angle in degrees below the geometric horizon.
    ///
    /// For sunrise/sunset this is 0.8333 deg (34' refraction + 16' semidiameter).
    /// For twilight events the standard IAU depression angles apply.
    pub fn depression_deg(self) -> f64 {
        match self {
            Self::Sunrise | Self::Sunset => 50.0 / 60.0, // 0.8333 deg
            Self::CivilDawn | Self::CivilDusk => 6.0,
            Self::NauticalDawn | Self::NauticalDusk => 12.0,
            Self::AstronomicalDawn | Self::AstronomicalDusk => 18.0,
        }
    }

    /// Whether this is a rising (morning) event.
    pub fn is_rising(self) -> bool {
        matches!(
            self,
            Self::Sunrise
                | Self::CivilDawn
                | Self::NauticalDawn
                | Self::AstronomicalDawn
        )
    }
}

/// Configurable parameters for rise/set computation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RiseSetConfig {
    /// Atmospheric refraction at the horizon in arcminutes. Default: 34.0.
    pub refraction_arcmin: f64,
    /// Solar angular semi-diameter in arcminutes. Default: 16.0.
    pub semidiameter_arcmin: f64,
    /// Whether to apply geometric dip correction for observer altitude.
    /// Dip angle = arccos(R / (R + h)) where R = Earth radius, h = altitude.
    /// Approximation: dip = sqrt(2h/R) radians. Default: true.
    pub altitude_correction: bool,
}

impl Default for RiseSetConfig {
    fn default() -> Self {
        Self {
            refraction_arcmin: 34.0,
            semidiameter_arcmin: 16.0,
            altitude_correction: true,
        }
    }
}

impl RiseSetConfig {
    /// Total horizon depression for sunrise/sunset in degrees.
    ///
    /// Combines refraction, solar semidiameter, and (optionally) geometric
    /// dip from observer altitude.
    ///
    /// `h0 = (refraction + semidiameter) / 60 + dip_deg`
    pub fn horizon_depression_deg(&self, altitude_m: f64) -> f64 {
        let base = (self.refraction_arcmin + self.semidiameter_arcmin) / 60.0;
        if self.altitude_correction && altitude_m > 0.0 {
            // Geometric dip: sqrt(2h/R) radians, converted to degrees
            let dip_rad = (2.0 * altitude_m / EARTH_RADIUS_M).sqrt();
            base + dip_rad * (180.0 / PI)
        } else {
            base
        }
    }
}

/// Result of a rise/set computation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RiseSetResult {
    /// Event occurs at the given Julian Date (TDB).
    Event {
        jd_tdb: f64,
        event: RiseSetEvent,
    },
    /// Sun never rises during this solar day (polar night).
    NeverRises,
    /// Sun never sets during this solar day (midnight sun).
    NeverSets,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn depression_sunrise() {
        let d = RiseSetEvent::Sunrise.depression_deg();
        assert!(
            (d - 0.8333).abs() < 0.001,
            "sunrise depression = {d}"
        );
    }

    #[test]
    fn depression_civil() {
        assert_eq!(RiseSetEvent::CivilDawn.depression_deg(), 6.0);
    }

    #[test]
    fn depression_nautical() {
        assert_eq!(RiseSetEvent::NauticalDawn.depression_deg(), 12.0);
    }

    #[test]
    fn depression_astronomical() {
        assert_eq!(RiseSetEvent::AstronomicalDawn.depression_deg(), 18.0);
    }

    #[test]
    fn is_rising_correct() {
        assert!(RiseSetEvent::Sunrise.is_rising());
        assert!(RiseSetEvent::CivilDawn.is_rising());
        assert!(RiseSetEvent::NauticalDawn.is_rising());
        assert!(RiseSetEvent::AstronomicalDawn.is_rising());
        assert!(!RiseSetEvent::Sunset.is_rising());
        assert!(!RiseSetEvent::CivilDusk.is_rising());
        assert!(!RiseSetEvent::NauticalDusk.is_rising());
        assert!(!RiseSetEvent::AstronomicalDusk.is_rising());
    }

    #[test]
    fn default_config() {
        let c = RiseSetConfig::default();
        assert_eq!(c.refraction_arcmin, 34.0);
        assert_eq!(c.semidiameter_arcmin, 16.0);
        assert!(c.altitude_correction);
    }

    #[test]
    fn depression_sea_level() {
        let c = RiseSetConfig::default();
        let d = c.horizon_depression_deg(0.0);
        let expected = (34.0 + 16.0) / 60.0;
        assert!(
            (d - expected).abs() < 1e-10,
            "sea level: {d}, expected {expected}"
        );
    }

    #[test]
    fn depression_1000m() {
        let c = RiseSetConfig::default();
        let d = c.horizon_depression_deg(1000.0);
        let base = (34.0 + 16.0) / 60.0;
        // Dip at 1000m: sqrt(2*1000/6371000) = sqrt(3.14e-4) ≈ 0.01772 rad ≈ 1.015 deg
        assert!(
            d > base + 0.9,
            "1000m depression {d} should exceed base {base} by ~1 deg"
        );
        assert!(
            d < base + 1.2,
            "1000m depression {d} too large"
        );
    }

    #[test]
    fn depression_no_altitude_correction() {
        let c = RiseSetConfig {
            altitude_correction: false,
            ..Default::default()
        };
        let d = c.horizon_depression_deg(10000.0);
        let expected = (34.0 + 16.0) / 60.0;
        assert!(
            (d - expected).abs() < 1e-10,
            "no altitude correction: {d}, expected {expected}"
        );
    }

    #[test]
    fn geolocation_radians() {
        let loc = GeoLocation::new(28.6139, 77.209, 0.0);
        assert!((loc.latitude_rad() - 28.6139_f64.to_radians()).abs() < 1e-15);
        assert!((loc.longitude_rad() - 77.209_f64.to_radians()).abs() < 1e-15);
    }

    #[test]
    fn cos_h_equator_equinox() {
        // At equator (phi=0), equinox (dec=0), h0=-0.8333 deg:
        // cos(H) = [sin(h0) - sin(phi)*sin(dec)] / [cos(phi)*cos(dec)]
        //         = sin(-0.8333 deg) / 1 = -0.01454
        let h0_rad = (-0.8333_f64).to_radians();
        let cos_h = h0_rad.sin(); // phi=0, dec=0 simplification
        assert!(
            (cos_h - (-0.01454)).abs() < 0.001,
            "cos_h = {cos_h}"
        );
    }

    #[test]
    fn cos_h_polar_never_rises() {
        // Tromso (lat=70N), winter solstice (dec=-23.44):
        // cos(H) = [sin(-0.8333 deg) - sin(70 deg)*sin(-23.44 deg)]
        //        / [cos(70 deg)*cos(-23.44 deg)]
        let h0 = (-0.8333_f64).to_radians();
        let phi = 70.0_f64.to_radians();
        let dec = (-23.44_f64).to_radians();
        let cos_h = (h0.sin() - phi.sin() * dec.sin()) / (phi.cos() * dec.cos());
        assert!(
            cos_h > 1.0,
            "cos_h = {cos_h}, should be > 1 (never rises)"
        );
    }

    #[test]
    fn cos_h_polar_never_sets() {
        // Tromso (lat=70N), summer solstice (dec=+23.44):
        let h0 = (-0.8333_f64).to_radians();
        let phi = 70.0_f64.to_radians();
        let dec = 23.44_f64.to_radians();
        let cos_h = (h0.sin() - phi.sin() * dec.sin()) / (phi.cos() * dec.cos());
        assert!(
            cos_h < -1.0,
            "cos_h = {cos_h}, should be < -1 (never sets)"
        );
    }
}
