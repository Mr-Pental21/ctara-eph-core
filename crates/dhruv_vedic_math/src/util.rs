//! Shared utility functions for vedic calculations.

/// Normalize an angle to [0, 360) degrees.
pub fn normalize_360(deg: f64) -> f64 {
    let r = deg % 360.0;
    if r < 0.0 { r + 360.0 } else { r }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_zero() {
        assert!((normalize_360(0.0) - 0.0).abs() < 1e-15);
    }

    #[test]
    fn normalize_positive() {
        assert!((normalize_360(45.0) - 45.0).abs() < 1e-15);
    }

    #[test]
    fn normalize_360_wraps() {
        assert!((normalize_360(360.0) - 0.0).abs() < 1e-15);
    }

    #[test]
    fn normalize_negative() {
        assert!((normalize_360(-10.0) - 350.0).abs() < 1e-15);
    }

    #[test]
    fn normalize_large() {
        assert!((normalize_360(730.0) - 10.0).abs() < 1e-10);
    }

    #[test]
    fn normalize_large_negative() {
        assert!((normalize_360(-370.0) - 350.0).abs() < 1e-10);
    }
}
