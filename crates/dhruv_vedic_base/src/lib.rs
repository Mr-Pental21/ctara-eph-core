//! Open derived Vedic calculations built on core ephemeris outputs.
//!
//! This crate provides:
//! - Ayanamsha computation for 20 sidereal reference systems
//! - Sunrise/sunset and twilight calculations
//! - Ascendant (Lagna) and MC computation
//! - Bhava (house) systems: 10 division methods
//! - Rashi (zodiac sign) and DMS conversion
//! - Nakshatra (lunar mansion) with pada, 27 and 28 schemes
//!
//! All implementations are clean-room, derived from IAU standards
//! and public astronomical formulas.

pub mod ascendant;
pub mod ayanamsha;
pub mod bhava;
pub mod bhava_types;
pub mod error;
pub mod lunar_nodes;
pub mod nakshatra;
pub mod rashi;
pub mod riseset;
pub mod riseset_types;

pub use ascendant::{ascendant_and_mc_rad, ascendant_longitude_rad, mc_longitude_rad, ramc_rad};
pub use ayanamsha::{
    AyanamshaSystem, ayanamsha_deg, ayanamsha_mean_deg, ayanamsha_true_deg,
    jd_tdb_to_centuries, tdb_seconds_to_centuries,
};
pub use bhava::compute_bhavas;
pub use bhava_types::{
    Bhava, BhavaConfig, BhavaReferenceMode, BhavaResult, BhavaStartingPoint, BhavaSystem,
};
pub use error::VedicError;
pub use lunar_nodes::{
    LunarNode, NodeMode, lunar_node_deg, mean_ketu_deg, mean_rahu_deg, true_ketu_deg,
    true_rahu_deg,
};
pub use nakshatra::{
    ALL_NAKSHATRAS_27, ALL_NAKSHATRAS_28, Nakshatra, Nakshatra28, Nakshatra28Info, NakshatraInfo,
    nakshatra28_from_longitude, nakshatra28_from_tropical, nakshatra_from_longitude,
    nakshatra_from_tropical,
};
pub use rashi::{
    ALL_RASHIS, Dms, Rashi, RashiInfo, deg_to_dms, rashi_from_longitude, rashi_from_tropical,
};
pub use riseset::{approximate_local_noon_jd, compute_all_events, compute_rise_set};
pub use riseset_types::{GeoLocation, RiseSetConfig, RiseSetEvent, RiseSetResult, SunLimb};
