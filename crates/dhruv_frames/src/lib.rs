//! Frame conversion helpers for ephemeris computations.
//!
//! Provides coordinate frame rotations (ICRF ↔ Ecliptic J2000) and
//! coordinate system conversions (Cartesian ↔ Spherical).

pub mod invariable;
pub mod nutation;
pub mod obliquity;
pub mod precession;
pub mod rotation;
pub mod spherical;

pub use nutation::{
    equation_of_equinoxes_and_true_obliquity, fundamental_arguments, nutation_iau2000b,
};
pub use obliquity::{
    COS_OBL, OBLIQUITY_J2000_DEG, OBLIQUITY_J2000_RAD, SIN_OBL, mean_obliquity_of_date_arcsec,
    mean_obliquity_of_date_rad,
};
pub use precession::{
    DEFAULT_PRECESSION_MODEL, PrecessionModel, ecliptic_inclination_arcsec,
    ecliptic_inclination_arcsec_with_model, ecliptic_node_longitude_arcsec,
    ecliptic_node_longitude_arcsec_with_model, general_precession_longitude_arcsec,
    general_precession_longitude_arcsec_with_model, general_precession_longitude_deg,
    general_precession_longitude_deg_with_model, general_precession_rate_deg_per_day,
    general_precession_rate_deg_per_day_with_model, precess_ecliptic_date_to_j2000,
    precess_ecliptic_date_to_j2000_with_model, precess_ecliptic_j2000_to_date,
    precess_ecliptic_j2000_to_date_with_model,
};
pub use invariable::{
    INVARIABLE_INCLINATION_DEG, INVARIABLE_NODE_DEG, ReferencePlane,
    ecliptic_lon_to_invariable_lon, ecliptic_to_invariable, icrf_to_invariable,
    icrf_to_reference_plane, invariable_lon_to_ecliptic_lon, invariable_to_ecliptic,
    invariable_to_icrf,
};
pub use rotation::{ecliptic_to_icrf, icrf_to_ecliptic};
pub use spherical::{
    SphericalCoords, SphericalState, cartesian_state_to_spherical_state, cartesian_to_spherical,
    spherical_to_cartesian,
};
