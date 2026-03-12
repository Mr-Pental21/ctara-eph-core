//! Error types for pure Vedic calculations.

use std::error::Error;
use std::fmt::{Display, Formatter};

/// Errors from pure Vedic calculations.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum VedicError {
    /// Invalid geographic/location-style parameter.
    InvalidLocation(&'static str),
    /// Iterative pure algorithm did not converge.
    NoConvergence(&'static str),
    /// Invalid input parameter.
    InvalidInput(&'static str),
}

impl Display for VedicError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidLocation(msg) => write!(f, "invalid location: {msg}"),
            Self::NoConvergence(msg) => write!(f, "no convergence: {msg}"),
            Self::InvalidInput(msg) => write!(f, "invalid input: {msg}"),
        }
    }
}

impl Error for VedicError {}
