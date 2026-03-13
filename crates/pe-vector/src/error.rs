use thiserror::Error;

#[derive(Debug, Error)]
pub enum VectorError {
    #[error("duplicate embedding for variant {0}")]
    DuplicateEmbedding(uuid::Uuid),

    #[error("variant {0} not found in vector store")]
    VariantNotFound(uuid::Uuid),

    #[error("embedding model failed: {0}")]
    EmbeddingFailed(String),

    #[error("serialization failed: {0}")]
    SerializationFailed(String),

    #[error("deserialization failed: {0}")]
    DeserializationFailed(String),

    #[error("k must be greater than zero")]
    InvalidK,
}
