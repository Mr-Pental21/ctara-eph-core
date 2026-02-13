//! UTC calendar date/time with sub-second precision.
//!
//! Provides `UtcTime`, the canonical UTC representation used throughout
//! the engine. Conversion to/from JD TDB requires a [`LeapSecondKernel`].

use crate::LeapSecondKernel;
use crate::julian::{calendar_to_jd, jd_to_calendar, jd_to_tdb_seconds, tdb_seconds_to_jd};

/// UTC calendar date with sub-second precision.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct UtcTime {
    pub year: i32,
    pub month: u32,
    pub day: u32,
    pub hour: u32,
    pub minute: u32,
    pub second: f64,
}

impl UtcTime {
    pub fn new(year: i32, month: u32, day: u32, hour: u32, minute: u32, second: f64) -> Self {
        Self {
            year,
            month,
            day,
            hour,
            minute,
            second,
        }
    }

    /// Convert to Julian Date TDB using leap-second data.
    pub fn to_jd_tdb(&self, lsk: &LeapSecondKernel) -> f64 {
        let day_frac = self.day as f64
            + self.hour as f64 / 24.0
            + self.minute as f64 / 1440.0
            + self.second / 86_400.0;
        let jd = calendar_to_jd(self.year, self.month, day_frac);
        // jd is UTC JD; convert to TDB seconds then back to JD
        let utc_s = jd_to_tdb_seconds(jd);
        let tdb_s = lsk.utc_to_tdb(utc_s);
        tdb_seconds_to_jd(tdb_s)
    }

    /// Convert from Julian Date TDB back to UTC calendar.
    pub fn from_jd_tdb(jd_tdb: f64, lsk: &LeapSecondKernel) -> Self {
        let tdb_s = jd_to_tdb_seconds(jd_tdb);
        let utc_s = lsk.tdb_to_utc(tdb_s);
        let utc_jd = tdb_seconds_to_jd(utc_s);
        let (year, month, day_frac) = jd_to_calendar(utc_jd);
        let day = day_frac.floor() as u32;
        let frac = day_frac.fract();
        let total_seconds = frac * 86_400.0;
        let hour = (total_seconds / 3600.0).floor() as u32;
        let minute = ((total_seconds % 3600.0) / 60.0).floor() as u32;
        let second = total_seconds % 60.0;
        Self {
            year,
            month,
            day,
            hour,
            minute,
            second,
        }
    }
}

impl std::fmt::Display for UtcTime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let whole = self.second as u32;
        let frac = self.second - whole as f64;
        if frac.abs() < 1e-9 {
            write!(
                f,
                "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
                self.year, self.month, self.day, self.hour, self.minute, whole
            )
        } else {
            write!(
                f,
                "{:04}-{:02}-{:02}T{:02}:{:02}:{:09.6}Z",
                self.year, self.month, self.day, self.hour, self.minute, self.second
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_constructor() {
        let t = UtcTime::new(2024, 3, 20, 12, 30, 45.5);
        assert_eq!(t.year, 2024);
        assert_eq!(t.month, 3);
        assert_eq!(t.day, 20);
        assert_eq!(t.hour, 12);
        assert_eq!(t.minute, 30);
        assert!((t.second - 45.5).abs() < 1e-12);
    }

    #[test]
    fn display_whole_seconds() {
        let t = UtcTime::new(2024, 1, 15, 0, 0, 0.0);
        assert_eq!(t.to_string(), "2024-01-15T00:00:00Z");
    }

    #[test]
    fn display_fractional_seconds() {
        let t = UtcTime::new(2024, 1, 15, 12, 30, 45.123);
        let s = t.to_string();
        assert!(s.contains("12:30:"), "got: {s}");
    }
}
