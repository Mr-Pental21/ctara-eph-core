//! Types for Sankranti search results.

use dhruv_frames::{DEFAULT_PRECESSION_MODEL, PrecessionModel, ReferencePlane};
use dhruv_time::UtcTime;
use dhruv_vedic_base::{AyanamshaSystem, Rashi, ayanamsha_deg_on_plane, ayanamsha_deg_with_model};

/// Configuration for Sankranti search.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SankrantiConfig {
    /// Ayanamsha system for sidereal longitude.
    pub ayanamsha_system: AyanamshaSystem,
    /// Whether to apply nutation correction to the ayanamsha.
    pub use_nutation: bool,
    /// Precession model used by ayanamsha propagation.
    pub precession_model: PrecessionModel,
    /// Reference plane for longitude measurements.
    ///
    /// Derived from `ayanamsha_system.default_reference_plane()` by default.
    /// Most systems use Ecliptic; Jagganatha uses Invariable.
    pub reference_plane: ReferencePlane,
    /// Coarse scan step size in days (default: 1.0).
    pub step_size_days: f64,
    /// Maximum bisection iterations (default: 50).
    pub max_iterations: u32,
    /// Convergence threshold in days (default: 1e-8).
    pub convergence_days: f64,
}

impl SankrantiConfig {
    /// Create with specified ayanamsha system and nutation flag, default search parameters.
    pub fn new(ayanamsha_system: AyanamshaSystem, use_nutation: bool) -> Self {
        Self::new_with_model(ayanamsha_system, use_nutation, DEFAULT_PRECESSION_MODEL)
    }

    /// Create with specified ayanamsha system, nutation flag, and precession model.
    pub fn new_with_model(
        ayanamsha_system: AyanamshaSystem,
        use_nutation: bool,
        precession_model: PrecessionModel,
    ) -> Self {
        Self {
            ayanamsha_system,
            use_nutation,
            precession_model,
            reference_plane: ayanamsha_system.default_reference_plane(),
            step_size_days: 1.0,
            max_iterations: 50,
            convergence_days: 1e-8,
        }
    }

    /// Default configuration with Lahiri ayanamsha.
    pub fn default_lahiri() -> Self {
        Self {
            ayanamsha_system: AyanamshaSystem::Lahiri,
            use_nutation: false,
            precession_model: DEFAULT_PRECESSION_MODEL,
            reference_plane: ReferencePlane::Ecliptic,
            step_size_days: 1.0,
            max_iterations: 50,
            convergence_days: 1e-8,
        }
    }

    /// Ayanamsha at `t_centuries`, using this configuration's model and plane settings.
    ///
    /// For `Ecliptic` plane, this returns the standard ecliptic ayanamsha.
    /// For `Invariable` plane, returns the ayanamsha computed on the invariable plane
    /// (nutation is not applied — it's an ecliptic concept).
    pub fn ayanamsha_deg_at_centuries(&self, t_centuries: f64) -> f64 {
        ayanamsha_deg_on_plane(
            self.ayanamsha_system,
            t_centuries,
            self.use_nutation,
            self.precession_model,
            self.reference_plane,
        )
    }

    /// Ayanamsha at `t_centuries` using the ECLIPTIC plane regardless of config.
    ///
    /// Used by code paths that always need ecliptic ayanamsha (e.g. tithi, elongation).
    pub fn ayanamsha_deg_ecliptic(&self, t_centuries: f64) -> f64 {
        ayanamsha_deg_with_model(
            self.ayanamsha_system,
            t_centuries,
            self.use_nutation,
            self.precession_model,
        )
    }

    pub fn validate(&self) -> Result<(), &'static str> {
        if self.step_size_days <= 0.0 {
            return Err("step_size_days must be positive");
        }
        if self.max_iterations == 0 {
            return Err("max_iterations must be > 0");
        }
        if self.convergence_days <= 0.0 {
            return Err("convergence_days must be positive");
        }
        Ok(())
    }
}

/// A Sankranti event: the Sun entering a new rashi.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SankrantiEvent {
    /// UTC time of the event.
    pub utc: UtcTime,
    /// The rashi the Sun is entering.
    pub rashi: Rashi,
    /// 0-based rashi index (0=Mesha .. 11=Meena).
    pub rashi_index: u8,
    /// Sun's sidereal longitude at the boundary (degrees, ~N*30).
    pub sun_sidereal_longitude_deg: f64,
    /// Sun's tropical longitude at the event (degrees).
    pub sun_tropical_longitude_deg: f64,
}
