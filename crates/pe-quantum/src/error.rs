use thiserror::Error;

#[derive(Debug, Error)]
pub enum QuantumRouterError {
    #[error("no suitable backend for job requiring {required_qubits} qubits")]
    NoSuitableBackend { required_qubits: u32 },

    #[error("backend execution failed: {0}")]
    BackendFailed(String),

    #[error("invalid job state transition from {from} to {to}")]
    InvalidTransition { from: String, to: String },

    #[error("job has not been submitted yet")]
    NotSubmitted,

    #[error("job has already been submitted")]
    AlreadySubmitted,

    #[error("job is not in Running state")]
    NotRunning,

    #[error("serialization failed: {0}")]
    SerializationFailed(String),

    #[error("deserialization failed: {0}")]
    DeserializationFailed(String),
}
