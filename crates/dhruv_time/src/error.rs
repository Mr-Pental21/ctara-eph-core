//! Error types for time-scale conversions.

use std::error::Error;
use std::fmt::{Display, Formatter};

/// Errors from time conversion, LSK parsing, or EOP lookup.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum TimeError {
    /// LSK file parsing failed.
    LskParse(String),
    /// IERS EOP file parsing failed.
    EopParse(String),
    /// Requested epoch is outside the EOP table range.
    EopOutOfRange,
    /// I/O error.
    Io(String),
    /// UTC epoch is before 1972-Jan-01 (pre-modern leap seconds).
    Pre1972Utc,
}

impl Display for TimeError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::LskParse(msg) => write!(f, "LSK parse error: {msg}"),
            Self::EopParse(msg) => write!(f, "EOP parse error: {msg}"),
            Self::EopOutOfRange => write!(f, "epoch outside EOP table range"),
            Self::Io(msg) => write!(f, "I/O error: {msg}"),
            Self::Pre1972Utc => write!(f, "UTC before 1972-Jan-01 is not supported"),
        }
    }
}

impl Error for TimeError {}

impl From<std::io::Error> for TimeError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e.to_string())
    }
}
