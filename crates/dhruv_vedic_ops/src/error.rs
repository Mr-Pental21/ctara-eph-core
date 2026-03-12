//! Error types for celestial event search.

use std::error::Error;
use std::fmt::{Display, Formatter};

use dhruv_core::EngineError;

/// Errors from search and grahan computations.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum SearchError {
    /// Error from the ephemeris engine.
    Engine(EngineError),
    /// Invalid search configuration parameter.
    InvalidConfig(&'static str),
    /// Iterative refinement did not converge.
    NoConvergence(&'static str),
}

impl Display for SearchError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Engine(e) => write!(f, "engine error: {e}"),
            Self::InvalidConfig(msg) => write!(f, "invalid config: {msg}"),
            Self::NoConvergence(msg) => write!(f, "no convergence: {msg}"),
        }
    }
}

impl Error for SearchError {}

impl From<EngineError> for SearchError {
    fn from(e: EngineError) -> Self {
        Self::Engine(e)
    }
}

impl From<dhruv_vedic_engine::VedicError> for SearchError {
    fn from(e: dhruv_vedic_engine::VedicError) -> Self {
        match e {
            dhruv_vedic_engine::VedicError::Engine(eng) => Self::Engine(eng),
            dhruv_vedic_engine::VedicError::InvalidInput(msg) => Self::InvalidConfig(msg),
            _ => Self::NoConvergence("vedic calculation failed"),
        }
    }
}

impl From<dhruv_vedic_math::VedicError> for SearchError {
    fn from(e: dhruv_vedic_math::VedicError) -> Self {
        match e {
            dhruv_vedic_math::VedicError::InvalidInput(msg) => Self::InvalidConfig(msg),
            _ => Self::NoConvergence("vedic math calculation failed"),
        }
    }
}

impl From<dhruv_search::SearchError> for SearchError {
    fn from(e: dhruv_search::SearchError) -> Self {
        match e {
            dhruv_search::SearchError::Engine(eng) => Self::Engine(eng),
            dhruv_search::SearchError::InvalidConfig(msg) => Self::InvalidConfig(msg),
            dhruv_search::SearchError::NoConvergence(msg) => Self::NoConvergence(msg),
            _ => Self::NoConvergence("search calculation failed"),
        }
    }
}
