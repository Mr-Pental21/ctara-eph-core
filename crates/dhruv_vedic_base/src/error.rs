//! Error types for Vedic calculations.

use std::error::Error;
use std::fmt::{Display, Formatter};

use dhruv_core::EngineError;
use dhruv_time::TimeError;

/// Errors from Vedic base calculations.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum VedicError {
    /// Error from the ephemeris engine.
    Engine(EngineError),
    /// Error from time conversion / EOP lookup.
    Time(TimeError),
    /// Invalid geographic location parameter.
    InvalidLocation(&'static str),
    /// Iterative algorithm did not converge.
    NoConvergence(&'static str),
}

impl Display for VedicError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Engine(e) => write!(f, "engine error: {e}"),
            Self::Time(e) => write!(f, "time error: {e}"),
            Self::InvalidLocation(msg) => write!(f, "invalid location: {msg}"),
            Self::NoConvergence(msg) => write!(f, "no convergence: {msg}"),
        }
    }
}

impl Error for VedicError {}

impl From<EngineError> for VedicError {
    fn from(e: EngineError) -> Self {
        Self::Engine(e)
    }
}

impl From<TimeError> for VedicError {
    fn from(e: TimeError) -> Self {
        Self::Time(e)
    }
}
