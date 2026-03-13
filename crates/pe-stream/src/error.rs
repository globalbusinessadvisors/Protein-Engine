use thiserror::Error;

#[derive(Debug, Error)]
pub enum StreamError {
    #[error("instrument read failed: {0}")]
    ReadFailed(String),

    #[error("normalization failed: {0}")]
    NormalizationFailed(String),

    #[error("invalid reading: {field} has non-finite value")]
    NonFiniteValue { field: String },

    #[error("no variant mapping for instrument {instrument_id}, channel {channel}")]
    NoVariantMapping {
        instrument_id: String,
        channel: String,
    },

    #[error("empty reading: no data fields")]
    EmptyReading,

    #[error("domain error: {0}")]
    DomainError(String),
}

impl From<pe_core::CoreError> for StreamError {
    fn from(e: pe_core::CoreError) -> Self {
        StreamError::DomainError(e.to_string())
    }
}
