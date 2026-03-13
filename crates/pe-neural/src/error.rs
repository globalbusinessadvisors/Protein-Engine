use thiserror::Error;

#[derive(Debug, Error)]
pub enum NeuralError {
    #[error("sub-model '{model}' failed: {reason}")]
    SubModelFailed { model: String, reason: String },

    #[error("model not loaded: {0}")]
    ModelNotLoaded(String),

    #[error("invalid model weights: {0}")]
    InvalidWeights(String),

    #[error("fitness score construction failed: {0}")]
    FitnessScoreError(String),

    #[error("serialization failed: {0}")]
    SerializationFailed(String),

    #[error("deserialization failed: {0}")]
    DeserializationFailed(String),
}
