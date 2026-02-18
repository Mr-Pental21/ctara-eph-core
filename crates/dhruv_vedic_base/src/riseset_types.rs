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
    /// **Not validated** — out-of-range values are accepted but produce
    /// undefined astronomical results from downstream functions.
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
    /// Depression angle in degrees below the geometric horizon for twilight events.
    ///
    /// For sunrise/sunset this returns 0.0 — the actual target altitude is
    /// computed by [`RiseSetConfig::target_altitude_deg`] which accounts for
    /// refraction, semidiameter, and sun limb choice.
    pub fn depression_deg(self) -> f64 {
        match self {
            Self::Sunrise | Self::Sunset => 0.0,
            Self::CivilDawn | Self::CivilDusk => 6.0,
            Self::NauticalDawn | Self::NauticalDusk => 12.0,
            Self::AstronomicalDawn | Self::AstronomicalDusk => 18.0,
        }
    }

    /// Whether this is a rising (morning) event.
    pub fn is_rising(self) -> bool {
        matches!(
            self,
            Self::Sunrise | Self::CivilDawn | Self::NauticalDawn | Self::AstronomicalDawn
        )
    }

    /// Whether this is a sunrise or sunset event (not twilight).
    pub fn is_sun_event(self) -> bool {
        matches!(self, Self::Sunrise | Self::Sunset)
    }
}

/// Which part of the solar disk defines the sunrise/sunset event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum SunLimb {
    /// Sunrise = upper limb appears; Sunset = upper limb disappears.
    /// This is the conventional astronomical definition.
    #[default]
    UpperLimb,
    /// Sunrise/sunset defined by center of disk.
    Center,
    /// Sunrise = lower limb appears; Sunset = lower limb disappears.
    LowerLimb,
}

/// Configurable parameters for rise/set computation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RiseSetConfig {
    /// Apply standard atmospheric refraction (34 arcmin). Default: true.
    pub use_refraction: bool,
    /// Which solar limb defines sunrise/sunset. Default: UpperLimb.
    pub sun_limb: SunLimb,
    /// Whether to apply geometric dip correction for observer altitude.
    /// Dip angle = arccos(R / (R + h)) where R = Earth radius, h = altitude.
    /// Default: true.
    pub altitude_correction: bool,
}

impl Default for RiseSetConfig {
    fn default() -> Self {
        Self {
            use_refraction: true,
            sun_limb: SunLimb::UpperLimb,
            altitude_correction: true,
        }
    }
}

/// Standard atmospheric refraction at the horizon in arcminutes.
const STANDARD_REFRACTION_ARCMIN: f64 = 34.0;

impl RiseSetConfig {
    /// Target altitude of the Sun's center at the event, in degrees.
    ///
    /// For sunrise/sunset events, combines refraction, solar semidiameter
    /// (passed in from the ephemeris), and geometric dip from altitude.
    ///
    /// For twilight events, returns the standard IAU depression angle
    /// (negative altitude), ignoring refraction/semidiameter/limb.
    ///
    /// # Arguments
    /// * `event` — the rise/set event type
    /// * `semidiameter_arcmin` — solar angular semidiameter in arcminutes
    ///   (computed dynamically from Earth-Sun distance)
    /// * `altitude_m` — observer altitude in meters
    pub fn target_altitude_deg(
        &self,
        event: RiseSetEvent,
        semidiameter_arcmin: f64,
        altitude_m: f64,
    ) -> f64 {
        if !event.is_sun_event() {
            // Twilight: fixed depression angle, negative
            return -(event.depression_deg());
        }

        let refraction = if self.use_refraction {
            STANDARD_REFRACTION_ARCMIN
        } else {
            0.0
        };

        // Semidiameter contribution depends on which limb defines the event.
        // UpperLimb: sun center is one semidiameter below the limb at horizon.
        // LowerLimb: sun center is one semidiameter above the limb at horizon.
        // Center: sun center is at the horizon, no semidiameter offset.
        let sd_contrib = match self.sun_limb {
            SunLimb::UpperLimb => semidiameter_arcmin,
            SunLimb::Center => 0.0,
            SunLimb::LowerLimb => -semidiameter_arcmin,
        };

        // Base depression in degrees (negative altitude)
        let base = -(refraction + sd_contrib) / 60.0;

        // Geometric dip correction
        if self.altitude_correction && altitude_m > 0.0 {
            let dip_rad = (2.0 * altitude_m / EARTH_RADIUS_M).sqrt();
            base - dip_rad * (180.0 / PI)
        } else {
            base
        }
    }
}

/// Result of a rise/set computation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RiseSetResult {
    /// Event occurs at the given Julian Date (TDB).
    Event { jd_tdb: f64, event: RiseSetEvent },
    /// Sun never rises during this solar day (polar night).
    NeverRises,
    /// Sun never sets during this solar day (midnight sun).
    NeverSets,
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn depression_sunrise_is_zero() {
        // Sunrise depression is now 0 — actual altitude comes from config
        assert_eq!(RiseSetEvent::Sunrise.depression_deg(), 0.0);
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
    fn is_sun_event() {
        assert!(RiseSetEvent::Sunrise.is_sun_event());
        assert!(RiseSetEvent::Sunset.is_sun_event());
        assert!(!RiseSetEvent::CivilDawn.is_sun_event());
        assert!(!RiseSetEvent::AstronomicalDusk.is_sun_event());
    }

    #[test]
    fn default_config() {
        let c = RiseSetConfig::default();
        assert!(c.use_refraction);
        assert_eq!(c.sun_limb, SunLimb::UpperLimb);
        assert!(c.altitude_correction);
    }

    #[test]
    fn target_altitude_upper_limb_rising() {
        let c = RiseSetConfig::default();
        // Typical semidiameter ~16 arcmin
        let h = c.target_altitude_deg(RiseSetEvent::Sunrise, 16.0, 0.0);
        // -(34 + 16) / 60 = -0.8333 deg
        let expected = -(34.0 + 16.0) / 60.0;
        assert!(
            (h - expected).abs() < 1e-10,
            "upper limb rising: {h}, expected {expected}"
        );
    }

    #[test]
    fn target_altitude_upper_limb_setting() {
        let c = RiseSetConfig::default();
        let h = c.target_altitude_deg(RiseSetEvent::Sunset, 16.0, 0.0);
        // Same as rising for UpperLimb: -(34 + 16) / 60
        let expected = -(34.0 + 16.0) / 60.0;
        assert!(
            (h - expected).abs() < 1e-10,
            "upper limb setting: {h}, expected {expected}"
        );
    }

    #[test]
    fn target_altitude_center() {
        let c = RiseSetConfig {
            sun_limb: SunLimb::Center,
            ..Default::default()
        };
        let h = c.target_altitude_deg(RiseSetEvent::Sunrise, 16.0, 0.0);
        // Center: -(34 + 0) / 60 = -0.5667 deg
        let expected = -34.0 / 60.0;
        assert!(
            (h - expected).abs() < 1e-10,
            "center: {h}, expected {expected}"
        );
    }

    #[test]
    fn target_altitude_lower_limb() {
        let c = RiseSetConfig {
            sun_limb: SunLimb::LowerLimb,
            ..Default::default()
        };
        let h = c.target_altitude_deg(RiseSetEvent::Sunrise, 16.0, 0.0);
        // LowerLimb rising: -(34 + (-16)) / 60 = -(18)/60 = -0.3 deg
        let expected = -(34.0 - 16.0) / 60.0;
        assert!(
            (h - expected).abs() < 1e-10,
            "lower limb: {h}, expected {expected}"
        );
    }

    #[test]
    fn target_altitude_no_refraction() {
        let c = RiseSetConfig {
            use_refraction: false,
            ..Default::default()
        };
        let h = c.target_altitude_deg(RiseSetEvent::Sunrise, 16.0, 0.0);
        // No refraction, UpperLimb: -(0 + 16) / 60 = -0.2667 deg
        let expected = -16.0 / 60.0;
        assert!(
            (h - expected).abs() < 1e-10,
            "no refraction: {h}, expected {expected}"
        );
    }

    #[test]
    fn target_altitude_twilight_ignores_config() {
        let c = RiseSetConfig {
            use_refraction: false,
            sun_limb: SunLimb::LowerLimb,
            ..Default::default()
        };
        let h = c.target_altitude_deg(RiseSetEvent::CivilDawn, 16.0, 0.0);
        assert!((h - (-6.0)).abs() < 1e-10, "civil dawn: {h}, expected -6.0");
    }

    #[test]
    fn target_altitude_with_dip_1000m() {
        let c = RiseSetConfig::default();
        let h = c.target_altitude_deg(RiseSetEvent::Sunrise, 16.0, 1000.0);
        let base = -(34.0 + 16.0) / 60.0;
        // Dip at 1000m ≈ 1.015 deg
        assert!(
            h < base - 0.9,
            "1000m altitude: {h} should be < {}",
            base - 0.9
        );
        assert!(h > base - 1.2, "1000m altitude: {h} too negative");
    }

    #[test]
    fn target_altitude_no_altitude_correction() {
        let c = RiseSetConfig {
            altitude_correction: false,
            ..Default::default()
        };
        let h = c.target_altitude_deg(RiseSetEvent::Sunrise, 16.0, 10000.0);
        let expected = -(34.0 + 16.0) / 60.0;
        assert!(
            (h - expected).abs() < 1e-10,
            "no altitude correction: {h}, expected {expected}"
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
        assert!((cos_h - (-0.01454)).abs() < 0.001, "cos_h = {cos_h}");
    }

    #[test]
    fn cos_h_polar_never_rises() {
        // Tromso (lat=70N), winter solstice (dec=-23.44):
        let h0 = (-0.8333_f64).to_radians();
        let phi = 70.0_f64.to_radians();
        let dec = (-23.44_f64).to_radians();
        let cos_h = (h0.sin() - phi.sin() * dec.sin()) / (phi.cos() * dec.cos());
        assert!(cos_h > 1.0, "cos_h = {cos_h}, should be > 1 (never rises)");
    }

    #[test]
    fn cos_h_polar_never_sets() {
        // Tromso (lat=70N), summer solstice (dec=+23.44):
        let h0 = (-0.8333_f64).to_radians();
        let phi = 70.0_f64.to_radians();
        let dec = 23.44_f64.to_radians();
        let cos_h = (h0.sin() - phi.sin() * dec.sin()) / (phi.cos() * dec.cos());
        assert!(cos_h < -1.0, "cos_h = {cos_h}, should be < -1 (never sets)");
    }

    #[test]
    fn sun_limb_default() {
        assert_eq!(SunLimb::default(), SunLimb::UpperLimb);
    }
}
