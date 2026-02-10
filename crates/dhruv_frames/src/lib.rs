//! Frame conversion helpers for ephemeris computations.
//!
//! Provides coordinate frame rotations (ICRF ↔ Ecliptic J2000) and
//! coordinate system conversions (Cartesian ↔ Spherical).

pub mod obliquity;
pub mod rotation;
pub mod spherical;

pub use obliquity::{COS_OBL, OBLIQUITY_J2000_DEG, OBLIQUITY_J2000_RAD, SIN_OBL};
pub use rotation::{ecliptic_to_icrf, icrf_to_ecliptic};
pub use spherical::{
    cartesian_state_to_spherical_state, cartesian_to_spherical, spherical_to_cartesian,
    SphericalCoords, SphericalState,
};
