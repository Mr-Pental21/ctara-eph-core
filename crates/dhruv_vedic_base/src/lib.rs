//! Open derived Vedic calculations built on core ephemeris outputs.
//!
//! This crate provides:
//! - Ayanamsha computation for 20 sidereal reference systems
//! - Sunrise/sunset and twilight calculations
//!
//! All implementations are clean-room, derived from IAU standards
//! and public astronomical formulas.

pub mod ayanamsha;
pub mod error;
pub mod riseset;
pub mod riseset_types;

pub use ayanamsha::{
    AyanamshaSystem, ayanamsha_deg, ayanamsha_mean_deg, ayanamsha_true_deg,
    jd_tdb_to_centuries, tdb_seconds_to_centuries,
};
pub use error::VedicError;
pub use riseset::{approximate_local_noon_jd, compute_all_events, compute_rise_set};
pub use riseset_types::{GeoLocation, RiseSetConfig, RiseSetEvent, RiseSetResult, SunLimb};
