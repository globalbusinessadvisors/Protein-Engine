use thiserror::Error;

#[derive(Debug, Error)]
pub enum RvfError {
    #[error("manifest segment (0x00) is required but was not provided")]
    MissingManifest,

    #[error("manifest has not been set on the builder")]
    ManifestNotSet,

    #[error("duplicate segment type: 0x{0:02X}")]
    DuplicateSegment(u8),

    #[error("manifest name must not be empty")]
    EmptyName,

    #[error("manifest version must not be empty")]
    EmptyVersion,

    #[error("parent_hash must be exactly 32 bytes, got {0}")]
    InvalidParentHash(usize),

    #[error("signing_key_fingerprint must be exactly 32 bytes, got {0}")]
    InvalidSigningKeyFingerprint(usize),

    #[error("deserialization failed: {0}")]
    DeserializationFailed(String),

    #[error("file hash mismatch: expected {expected}, got {actual}")]
    HashMismatch { expected: String, actual: String },

    #[error("segment data too large: {0} bytes exceeds u32::MAX")]
    SegmentTooLarge(usize),

    #[error("serialization failed: {0}")]
    SerializationFailed(String),
}
