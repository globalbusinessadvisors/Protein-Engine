use serde::{Deserialize, Serialize};

/// Post-quantum ML-DSA signature wrapper.
///
/// In production this holds the full ~2420-byte ML-DSA-65 signature.
/// In tests, a deterministic stub signature can be used via MockCryptoSigner.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MlDsaSignature(pub Vec<u8>);

impl MlDsaSignature {
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}

/// SHA3-256 hash of a serialized journal entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EntryHash(pub [u8; 32]);

impl EntryHash {
    /// The zero hash used as prev_hash for the genesis entry (JC-3).
    pub const GENESIS: EntryHash = EntryHash([0u8; 32]);

    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl std::fmt::Display for EntryHash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", hex::encode(self.0))
    }
}

/// The 9 auditable event types from ADR-010.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EntryType {
    VariantDesigned,
    FitnessScored,
    StructureValidated,
    SafetyScreened,
    ExperimentRecorded,
    ModelUpdated,
    VqeCompleted,
    CycleCompleted,
    AgentRetired,
}
