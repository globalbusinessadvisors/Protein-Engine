use thiserror::Error;

#[derive(Debug, Error)]
pub enum SolverError {
    #[error("energy landscape has zero dimensions")]
    ZeroDimensions,

    #[error("coordinate index {index} exceeds landscape dimensions ({dimensions})")]
    CoordinateOutOfBounds { index: usize, dimensions: usize },

    #[error("solver failed to converge after {iterations} iterations")]
    DidNotConverge { iterations: usize },

    #[error("invalid solver configuration: {0}")]
    InvalidConfig(String),

    #[error("serialization failed: {0}")]
    SerializationFailed(String),

    #[error("deserialization failed: {0}")]
    DeserializationFailed(String),
}
