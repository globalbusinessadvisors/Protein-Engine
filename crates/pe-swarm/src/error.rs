use thiserror::Error;

#[derive(Debug, Error)]
pub enum SwarmError {
    #[error("agent execution failed: {0}")]
    AgentFailed(String),

    #[error("evolution engine error: {0}")]
    EvolutionFailed(String),

    #[error("scoring failed: {0}")]
    ScoringFailed(String),

    #[error("quantum dispatch failed: {0}")]
    QuantumFailed(String),

    #[error("ledger write failed: {0}")]
    LedgerFailed(String),

    #[error("serialization failed: {0}")]
    SerializationFailed(String),

    #[error("empty population")]
    EmptyPopulation,

    #[error("domain error: {0}")]
    DomainError(String),
}

impl From<pe_core::CoreError> for SwarmError {
    fn from(e: pe_core::CoreError) -> Self {
        SwarmError::DomainError(e.to_string())
    }
}
