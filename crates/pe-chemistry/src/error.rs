use thiserror::Error;

#[derive(Debug, Error)]
pub enum ChemistryError {
    #[error("HTTP request failed: {0}")]
    HttpError(String),

    #[error("request timed out after {0:?}")]
    Timeout(std::time::Duration),

    #[error("sidecar returned HTTP {status}: {body}")]
    SidecarError { status: u16, body: String },

    #[error("failed to parse sidecar response: {0}")]
    ParseError(String),

    #[error("sidecar is not reachable at {0}")]
    Unreachable(String),

    #[error("serialization failed: {0}")]
    SerializationFailed(String),
}

impl From<ChemistryError> for pe_quantum::QuantumRouterError {
    fn from(e: ChemistryError) -> Self {
        pe_quantum::QuantumRouterError::BackendFailed(e.to_string())
    }
}
