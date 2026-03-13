use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha3::{Digest, Sha3_256};

use crate::error::LedgerError;
use crate::types::{EntryHash, EntryType, MlDsaSignature};

/// A single entry in the append-only journal (ADR-010).
///
/// Each entry carries:
/// - A strictly sequential sequence number (JC-1)
/// - The SHA3-256 hash of the previous entry (JC-2), zeros for genesis (JC-3)
/// - An ML-DSA signature over (seq || timestamp || prev_hash || type || payload) (JC-4)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JournalEntry {
    pub sequence_number: u64,
    pub timestamp: DateTime<Utc>,
    pub prev_hash: EntryHash,
    pub entry_type: EntryType,
    pub payload: Vec<u8>,
    pub signature: MlDsaSignature,
}

impl JournalEntry {
    /// Compute the bytes that are signed / hashed for this entry.
    ///
    /// This is the canonical serialization: sequence_number (8 LE) || timestamp (8 LE millis)
    /// || prev_hash (32) || entry_type (JSON) || payload.
    pub fn signable_bytes(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        buf.extend_from_slice(&self.sequence_number.to_le_bytes());
        buf.extend_from_slice(&self.timestamp.timestamp_millis().to_le_bytes());
        buf.extend_from_slice(self.prev_hash.as_bytes());
        // entry_type as a fixed tag
        let type_tag = match self.entry_type {
            EntryType::VariantDesigned => 0u8,
            EntryType::FitnessScored => 1,
            EntryType::StructureValidated => 2,
            EntryType::SafetyScreened => 3,
            EntryType::ExperimentRecorded => 4,
            EntryType::ModelUpdated => 5,
            EntryType::VqeCompleted => 6,
            EntryType::CycleCompleted => 7,
            EntryType::AgentRetired => 8,
        };
        buf.push(type_tag);
        buf.extend_from_slice(&self.payload);
        buf
    }

    /// Compute SHA3-256 hash of this entry's signable bytes + signature.
    pub fn compute_hash(&self) -> EntryHash {
        let mut hasher = Sha3_256::new();
        hasher.update(&self.signable_bytes());
        hasher.update(self.signature.as_bytes());
        let result = hasher.finalize();
        let mut hash = [0u8; 32];
        hash.copy_from_slice(&result);
        EntryHash(hash)
    }

    /// Serialize this entry to JSON bytes.
    pub fn to_bytes(&self) -> Result<Vec<u8>, LedgerError> {
        serde_json::to_vec(self).map_err(|e| LedgerError::SerializationFailed(e.to_string()))
    }

    /// Deserialize an entry from JSON bytes.
    pub fn from_bytes(data: &[u8]) -> Result<Self, LedgerError> {
        serde_json::from_slice(data).map_err(|e| LedgerError::DeserializationFailed(e.to_string()))
    }
}
