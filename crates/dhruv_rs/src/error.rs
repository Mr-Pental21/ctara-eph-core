use std::error::Error;
use std::fmt::{Display, Formatter};

use dhruv_core::EngineError;
use dhruv_tara::TaraError;
use dhruv_time::TimeError;
use dhruv_vedic_base::VedicError;
use dhruv_vedic_ops::SearchError;

/// Unified error type for the high-level Rust facade.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum DhruvError {
    /// Failed to parse a date string.
    DateParse(String),
    /// Error from the underlying engine.
    Engine(EngineError),
    /// Error from time conversion.
    Time(TimeError),
    /// Error from search/panchang computation.
    Search(SearchError),
    /// Error from vedic base computation (rise/set, bhava, etc.).
    Vedic(VedicError),
    /// Error from fixed star computation.
    Tara(TaraError),
    /// Error resolving layered configuration.
    Config(String),
}

impl Display for DhruvError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DateParse(msg) => write!(f, "date parse error: {msg}"),
            Self::Engine(e) => write!(f, "engine error: {e}"),
            Self::Time(e) => write!(f, "time error: {e}"),
            Self::Search(e) => write!(f, "search error: {e}"),
            Self::Vedic(e) => write!(f, "vedic error: {e}"),
            Self::Tara(e) => write!(f, "tara error: {e}"),
            Self::Config(msg) => write!(f, "config error: {msg}"),
        }
    }
}

impl Error for DhruvError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Engine(e) => Some(e),
            Self::Time(e) => Some(e),
            Self::Search(e) => Some(e),
            Self::Vedic(e) => Some(e),
            Self::Tara(e) => Some(e),
            _ => None,
        }
    }
}

impl From<EngineError> for DhruvError {
    fn from(e: EngineError) -> Self {
        Self::Engine(e)
    }
}

impl From<TimeError> for DhruvError {
    fn from(e: TimeError) -> Self {
        Self::Time(e)
    }
}

impl From<SearchError> for DhruvError {
    fn from(e: SearchError) -> Self {
        Self::Search(e)
    }
}

impl From<dhruv_search::SearchError> for DhruvError {
    fn from(e: dhruv_search::SearchError) -> Self {
        Self::Search(e.into())
    }
}

impl From<VedicError> for DhruvError {
    fn from(e: VedicError) -> Self {
        Self::Vedic(e)
    }
}

impl From<TaraError> for DhruvError {
    fn from(e: TaraError) -> Self {
        Self::Tara(e)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_date_parse() {
        let e = DhruvError::DateParse("bad date".into());
        assert!(e.to_string().contains("bad date"));
    }

    #[test]
    fn display_config() {
        let e = DhruvError::Config("bad config".into());
        assert!(e.to_string().contains("bad config"));
    }

    #[test]
    fn from_engine_error() {
        let e: DhruvError = EngineError::InvalidConfig("test").into();
        assert!(matches!(e, DhruvError::Engine(_)));
    }

    #[test]
    fn from_time_error() {
        let e: DhruvError = TimeError::Pre1972Utc.into();
        assert!(matches!(e, DhruvError::Time(_)));
    }
}
