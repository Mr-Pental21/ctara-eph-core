//! Engine-aware and date-aware Vedic computations.

pub mod ayanamsha;
mod ayanamsha_anchor;
mod ayanamsha_tara;
pub mod bhava;
pub mod bhava_types;
pub mod error;
pub mod lagna;
pub mod lunar_nodes;
pub mod riseset;
pub mod riseset_types;
pub mod time_policy;

pub use ayanamsha::{
    AyanamshaSystem, ayanamsha_deg, ayanamsha_deg_on_plane, ayanamsha_deg_static,
    ayanamsha_deg_with_catalog, ayanamsha_deg_with_catalog_and_model,
    ayanamsha_deg_with_catalog_on_plane, ayanamsha_deg_with_model, ayanamsha_mean_deg,
    ayanamsha_mean_deg_static, ayanamsha_mean_deg_static_on_plane,
    ayanamsha_mean_deg_static_with_model, ayanamsha_mean_deg_with_catalog,
    ayanamsha_mean_deg_with_catalog_and_model, ayanamsha_mean_deg_with_model, ayanamsha_true_deg,
    ayanamsha_true_deg_with_model, jd_tdb_to_centuries, tdb_seconds_to_centuries,
};
pub use bhava::compute_bhavas;
pub use bhava_types::{
    Bhava, BhavaConfig, BhavaReferenceMode, BhavaResult, BhavaStartingPoint, BhavaSystem,
};
pub use error::VedicError;
pub use lagna::{lagna_and_mc_rad, lagna_longitude_rad, mc_longitude_rad, ramc_rad};
pub use lunar_nodes::{
    LunarNode, NodeMode, lunar_node_deg, lunar_node_deg_for_epoch,
    lunar_node_deg_for_epoch_on_plane, lunar_node_deg_for_epoch_with_model, mean_ketu_deg,
    mean_rahu_deg, true_ketu_deg, true_rahu_deg,
};
pub use riseset::{approximate_local_noon_jd, compute_all_events, compute_rise_set};
pub use riseset_types::{GeoLocation, RiseSetConfig, RiseSetEvent, RiseSetResult, SunLimb};
pub use time_policy::{set_time_conversion_policy, time_conversion_policy};
