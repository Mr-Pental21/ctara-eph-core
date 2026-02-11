//! Celestial event search engine: conjunctions, oppositions, aspects, eclipses,
//! stationary points, and max-speed events.
//!
//! This crate provides:
//! - General-purpose conjunction/separation engine for any body pair
//! - Lunar eclipse computation (penumbral, partial, total)
//! - Solar eclipse computation (geocentric and topocentric)
//! - Stationary point search (retrograde/direct stations)
//! - Max-speed search (velocity extrema)

pub mod conjunction;
pub mod conjunction_types;
pub mod eclipse;
pub mod eclipse_types;
pub mod error;
pub mod stationary;
pub mod stationary_types;

pub use conjunction::{next_conjunction, prev_conjunction, search_conjunctions};
pub use conjunction_types::{ConjunctionConfig, ConjunctionEvent, SearchDirection};
pub use eclipse::{
    next_lunar_eclipse, next_solar_eclipse, prev_lunar_eclipse, prev_solar_eclipse,
    search_lunar_eclipses, search_solar_eclipses,
};
pub use eclipse_types::{
    EclipseConfig, GeoLocation, LunarEclipse, LunarEclipseType, SolarEclipse, SolarEclipseType,
};
pub use error::SearchError;
pub use stationary::{
    next_max_speed, next_stationary, prev_max_speed, prev_stationary, search_max_speed,
    search_stationary,
};
pub use stationary_types::{
    MaxSpeedEvent, MaxSpeedType, StationaryConfig, StationaryEvent, StationType,
};
