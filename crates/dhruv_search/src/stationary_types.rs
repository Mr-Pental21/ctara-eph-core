//! Types for stationary point and max-speed search.

use dhruv_core::Body;

/// Station type: retrograde or direct.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StationType {
    /// Planet's longitude speed crosses from positive to negative (starts retrograde).
    StationRetrograde,
    /// Planet's longitude speed crosses from negative to positive (ends retrograde).
    StationDirect,
}

/// A stationary point event (planet's ecliptic longitude velocity crosses zero).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StationaryEvent {
    /// Event time as Julian Date (TDB).
    pub jd_tdb: f64,
    /// Which body.
    pub body: Body,
    /// Ecliptic longitude at station in degrees [0, 360).
    pub longitude_deg: f64,
    /// Ecliptic latitude at station in degrees.
    pub latitude_deg: f64,
    /// Whether retrograde or direct station.
    pub station_type: StationType,
}

/// Max speed type classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MaxSpeedType {
    /// Peak forward (direct) speed.
    MaxDirect,
    /// Peak retrograde speed (most negative velocity).
    MaxRetrograde,
}

/// A max-speed event (planet's ecliptic longitude acceleration crosses zero).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MaxSpeedEvent {
    /// Event time as Julian Date (TDB).
    pub jd_tdb: f64,
    /// Which body.
    pub body: Body,
    /// Ecliptic longitude at peak speed in degrees [0, 360).
    pub longitude_deg: f64,
    /// Ecliptic latitude at peak speed in degrees.
    pub latitude_deg: f64,
    /// Longitude speed at peak in degrees per day.
    pub speed_deg_per_day: f64,
    /// Whether the peak is in the direct or retrograde direction.
    pub speed_type: MaxSpeedType,
}

/// Configuration for stationary and max-speed searches.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StationaryConfig {
    /// Coarse scan step size in days.
    pub step_size_days: f64,
    /// Maximum bisection iterations (default 50).
    pub max_iterations: u32,
    /// Convergence threshold in days (default 1e-8, ~0.86 ms).
    pub convergence_days: f64,
    /// Step for numerical central difference in days (default 0.01).
    /// Only used by max-speed search for computing acceleration.
    pub numerical_step_days: f64,
}

impl StationaryConfig {
    /// Default config for inner planets (Mercury, Venus, Mars): 1-day step.
    pub fn inner_planet() -> Self {
        Self {
            step_size_days: 1.0,
            max_iterations: 50,
            convergence_days: 1e-8,
            numerical_step_days: 0.01,
        }
    }

    /// Default config for outer planets (Jupiter, Saturn, Uranus, Neptune, Pluto): 2-day step.
    pub fn outer_planet() -> Self {
        Self {
            step_size_days: 2.0,
            max_iterations: 50,
            convergence_days: 1e-8,
            numerical_step_days: 0.01,
        }
    }

    /// Validate the configuration.
    pub(crate) fn validate(&self) -> Result<(), &'static str> {
        if !self.step_size_days.is_finite() || self.step_size_days <= 0.0 {
            return Err("step_size_days must be positive");
        }
        if self.max_iterations == 0 {
            return Err("max_iterations must be > 0");
        }
        if !self.convergence_days.is_finite() || self.convergence_days <= 0.0 {
            return Err("convergence_days must be positive");
        }
        if !self.numerical_step_days.is_finite() || self.numerical_step_days <= 0.0 {
            return Err("numerical_step_days must be positive");
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inner_planet_defaults() {
        let c = StationaryConfig::inner_planet();
        assert!((c.step_size_days - 1.0).abs() < 1e-10);
        assert_eq!(c.max_iterations, 50);
        assert!(c.validate().is_ok());
    }

    #[test]
    fn outer_planet_defaults() {
        let c = StationaryConfig::outer_planet();
        assert!((c.step_size_days - 2.0).abs() < 1e-10);
        assert!(c.validate().is_ok());
    }

    #[test]
    fn rejects_zero_step() {
        let mut c = StationaryConfig::inner_planet();
        c.step_size_days = 0.0;
        assert!(c.validate().is_err());
    }

    #[test]
    fn rejects_negative_step() {
        let mut c = StationaryConfig::inner_planet();
        c.step_size_days = -1.0;
        assert!(c.validate().is_err());
    }

    #[test]
    fn rejects_zero_iterations() {
        let mut c = StationaryConfig::inner_planet();
        c.max_iterations = 0;
        assert!(c.validate().is_err());
    }

    #[test]
    fn rejects_zero_convergence() {
        let mut c = StationaryConfig::inner_planet();
        c.convergence_days = 0.0;
        assert!(c.validate().is_err());
    }

    #[test]
    fn rejects_zero_numerical_step() {
        let mut c = StationaryConfig::inner_planet();
        c.numerical_step_days = 0.0;
        assert!(c.validate().is_err());
    }

    #[test]
    fn station_type_eq() {
        assert_eq!(StationType::StationRetrograde, StationType::StationRetrograde);
        assert_ne!(StationType::StationRetrograde, StationType::StationDirect);
    }

    #[test]
    fn max_speed_type_eq() {
        assert_eq!(MaxSpeedType::MaxDirect, MaxSpeedType::MaxDirect);
        assert_ne!(MaxSpeedType::MaxDirect, MaxSpeedType::MaxRetrograde);
    }
}
