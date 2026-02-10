//! Time-scale conversions (UTC/TAI/TT/TDB) and leap-second support.
//!
//! This crate provides:
//! - Julian Date ↔ calendar conversions
//! - LSK (Leapseconds Kernel) file parsing
//! - UTC → TAI → TT → TDB conversion chain (and inverse)
//! - An `Epoch` type for type-safe TDB epoch handling

pub mod eop;
pub mod error;
pub mod julian;
pub mod lsk;
pub mod scales;
pub mod sidereal;

use std::path::Path;

pub use eop::{EopData, EopKernel};
pub use error::TimeError;
pub use julian::{
    calendar_to_jd, jd_to_calendar, jd_to_tdb_seconds, tdb_seconds_to_jd, J2000_JD,
    SECONDS_PER_DAY,
};
pub use lsk::LskData;
pub use sidereal::{earth_rotation_angle_rad, gmst_rad, local_sidereal_time_rad};

/// A loaded leap-second kernel, ready for time conversions.
#[derive(Debug, Clone)]
pub struct LeapSecondKernel {
    data: LskData,
}

impl LeapSecondKernel {
    /// Load an LSK file from a path.
    pub fn load(path: &Path) -> Result<Self, TimeError> {
        let content = std::fs::read_to_string(path)?;
        Self::parse(&content)
    }

    /// Parse an LSK from its text content.
    pub fn parse(content: &str) -> Result<Self, TimeError> {
        let data = lsk::parse_lsk(content)?;
        Ok(Self { data })
    }

    /// Access the parsed LSK data.
    pub fn data(&self) -> &LskData {
        &self.data
    }

    /// Convert UTC seconds past J2000 to TDB seconds past J2000.
    pub fn utc_to_tdb(&self, utc_s: f64) -> f64 {
        scales::utc_to_tdb(utc_s, &self.data)
    }

    /// Convert TDB seconds past J2000 to UTC seconds past J2000.
    pub fn tdb_to_utc(&self, tdb_s: f64) -> f64 {
        scales::tdb_to_utc(tdb_s, &self.data)
    }
}

/// A TDB epoch represented as seconds past J2000.0.
///
/// This is the primary time type used throughout the engine.
/// It wraps an `f64` providing type safety and convenient conversions.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Epoch {
    tdb_seconds: f64,
}

impl Epoch {
    /// Create an epoch from TDB seconds past J2000.0.
    pub fn from_tdb_seconds(s: f64) -> Self {
        Self { tdb_seconds: s }
    }

    /// Create an epoch from a Julian Date in TDB.
    pub fn from_jd_tdb(jd: f64) -> Self {
        Self {
            tdb_seconds: jd_to_tdb_seconds(jd),
        }
    }

    /// Create an epoch from a UTC calendar date using an LSK for leap seconds.
    pub fn from_utc(
        year: i32,
        month: u32,
        day: u32,
        hour: u32,
        min: u32,
        sec: f64,
        lsk: &LeapSecondKernel,
    ) -> Self {
        let day_frac = day as f64 + hour as f64 / 24.0 + min as f64 / 1440.0 + sec / 86_400.0;
        let jd = calendar_to_jd(year, month, day_frac);
        let utc_s = jd_to_tdb_seconds(jd); // Note: this is UTC seconds past J2000, not TDB
        let tdb_s = lsk.utc_to_tdb(utc_s);
        Self {
            tdb_seconds: tdb_s,
        }
    }

    /// TDB seconds past J2000.0.
    pub fn as_tdb_seconds(self) -> f64 {
        self.tdb_seconds
    }

    /// Julian Date in TDB.
    pub fn as_jd_tdb(self) -> f64 {
        tdb_seconds_to_jd(self.tdb_seconds)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn epoch_from_jd_roundtrip() {
        let jd = 2_460_000.5;
        let epoch = Epoch::from_jd_tdb(jd);
        assert!((epoch.as_jd_tdb() - jd).abs() < 1e-12);
    }

    #[test]
    fn epoch_j2000_is_zero() {
        let epoch = Epoch::from_jd_tdb(J2000_JD);
        assert_eq!(epoch.as_tdb_seconds(), 0.0);
    }
}
