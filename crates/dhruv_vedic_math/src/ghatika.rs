//! Ghatika computation.
//!
//! A Vedic day (sunrise to next sunrise) is divided into 60 ghatikas,
//! each lasting 24 minutes. Ghatikas are numbered 1-60 from sunrise.
//!
//! Clean-room implementation: standard Vedic timekeeping convention.

/// Minutes per ghatika.
pub const GHATIKA_MINUTES: f64 = 24.0;

/// Number of ghatikas per Vedic day.
pub const GHATIKA_COUNT: u8 = 60;

/// Result of ghatika computation (pure arithmetic, no times).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GhatikaPosition {
    /// Ghatika value (1-60).
    pub value: u8,
    /// 0-based index (0-59).
    pub index: u8,
}

/// Determine the ghatika from the elapsed fraction of the Vedic day.
///
/// `seconds_since_sunrise` is the number of seconds since the Vedic day's sunrise.
/// `vedic_day_duration_seconds` is the total duration from sunrise to next sunrise.
///
/// Ghatikas are 1-indexed (1-60). If the input is exactly at the end,
/// it clamps to ghatika 60.
pub fn ghatika_from_elapsed(
    seconds_since_sunrise: f64,
    vedic_day_duration_seconds: f64,
) -> GhatikaPosition {
    let ghatika_duration = vedic_day_duration_seconds / GHATIKA_COUNT as f64;
    let mut idx = (seconds_since_sunrise / ghatika_duration).floor() as u8;
    if idx >= GHATIKA_COUNT {
        idx = GHATIKA_COUNT - 1;
    }
    GhatikaPosition {
        value: idx + 1,
        index: idx,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ghatika_at_sunrise() {
        let pos = ghatika_from_elapsed(0.0, 86400.0);
        assert_eq!(pos.value, 1);
        assert_eq!(pos.index, 0);
    }

    #[test]
    fn ghatika_at_half_day() {
        // 86400/2 = 43200 seconds → ghatika 31
        let pos = ghatika_from_elapsed(43200.0, 86400.0);
        assert_eq!(pos.value, 31);
        assert_eq!(pos.index, 30);
    }

    #[test]
    fn ghatika_last() {
        // Just before the end of the day
        let pos = ghatika_from_elapsed(86399.0, 86400.0);
        assert_eq!(pos.value, 60);
        assert_eq!(pos.index, 59);
    }

    #[test]
    fn ghatika_clamp_at_end() {
        let pos = ghatika_from_elapsed(86400.0, 86400.0);
        assert_eq!(pos.value, 60);
    }

    #[test]
    fn ghatika_each_is_24_minutes() {
        // With standard 86400s day, each ghatika = 1440 seconds = 24 minutes
        let ghatika_secs: f64 = 86400.0 / 60.0;
        assert!((ghatika_secs - 1440.0).abs() < 1e-10);
        assert!((ghatika_secs / 60.0 - 24.0).abs() < 1e-10);
    }

    #[test]
    fn ghatika_variable_day_length() {
        // Vedic day of 87000 seconds (slightly longer)
        let pos = ghatika_from_elapsed(0.0, 87000.0);
        assert_eq!(pos.value, 1);
        // Halfway through
        let pos = ghatika_from_elapsed(43500.0, 87000.0);
        assert_eq!(pos.value, 31);
    }
}
