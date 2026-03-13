//! SegmentProducer implementations for JOURNAL_SEG and WITNESS_SEG.

use pe_rvf::error::RvfError;
use pe_rvf::segment::SegmentType;
use pe_rvf::traits::SegmentProducer;
use serde::{Deserialize, Serialize};

use crate::entry::JournalEntry;

/// Produces the JOURNAL_SEG segment: serialized journal entries.
pub struct JournalSegProducer {
    entries: Vec<JournalEntry>,
}

impl JournalSegProducer {
    pub fn new(entries: Vec<JournalEntry>) -> Self {
        Self { entries }
    }
}

impl SegmentProducer for JournalSegProducer {
    fn segment_type(&self) -> SegmentType {
        SegmentType::JournalSeg
    }

    fn produce(&self) -> Result<Vec<u8>, RvfError> {
        serde_json::to_vec(&self.entries)
            .map_err(|e| RvfError::SerializationFailed(e.to_string()))
    }
}

/// A single witness record for the QuDAG-compatible WITNESS_SEG format.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WitnessRecord {
    pub sequence_number: u64,
    pub entry_hash: String,
    pub prev_hash: String,
    pub entry_type: String,
    pub signature_hex: String,
    pub timestamp_ms: i64,
}

/// Produces the WITNESS_SEG segment: QuDAG-compatible witness chain.
pub struct WitnessSegProducer {
    entries: Vec<JournalEntry>,
}

impl WitnessSegProducer {
    pub fn new(entries: Vec<JournalEntry>) -> Self {
        Self { entries }
    }
}

impl SegmentProducer for WitnessSegProducer {
    fn segment_type(&self) -> SegmentType {
        SegmentType::WitnessSeg
    }

    fn produce(&self) -> Result<Vec<u8>, RvfError> {
        let records: Vec<WitnessRecord> = self
            .entries
            .iter()
            .map(|e| {
                let entry_hash = e.compute_hash();
                WitnessRecord {
                    sequence_number: e.sequence_number,
                    entry_hash: hex::encode(entry_hash.as_bytes()),
                    prev_hash: hex::encode(e.prev_hash.as_bytes()),
                    entry_type: format!("{:?}", e.entry_type),
                    signature_hex: hex::encode(e.signature.as_bytes()),
                    timestamp_ms: e.timestamp.timestamp_millis(),
                }
            })
            .collect();

        serde_json::to_vec(&records)
            .map_err(|e| RvfError::SerializationFailed(e.to_string()))
    }
}
