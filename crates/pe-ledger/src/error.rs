use thiserror::Error;

#[derive(Debug, Error)]
pub enum LedgerError {
    #[error("hash chain broken at entry {index}: expected {expected}, got {actual}")]
    TamperDetected {
        index: u64,
        expected: String,
        actual: String,
    },

    #[error("invalid signature at entry {index}")]
    InvalidSignature { index: u64 },

    #[error("sequence number gap: expected {expected}, got {actual}")]
    SequenceGap { expected: u64, actual: u64 },

    #[error("signing failed: {0}")]
    SigningFailed(String),

    #[error("verification failed: {0}")]
    VerificationFailed(String),

    #[error("serialization failed: {0}")]
    SerializationFailed(String),

    #[error("deserialization failed: {0}")]
    DeserializationFailed(String),

    #[error("chain is empty")]
    EmptyChain,
}
