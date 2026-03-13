use pe_rvf::{RvfError, SegmentProducer, SegmentType};

use crate::result::MinimizationResult;

/// Produces JOURNAL_SEG bytes containing serialized solver results.
pub struct SolverSegmentProducer {
    results: Vec<MinimizationResult>,
}

impl SolverSegmentProducer {
    pub fn new(results: Vec<MinimizationResult>) -> Self {
        Self { results }
    }
}

impl SegmentProducer for SolverSegmentProducer {
    fn segment_type(&self) -> SegmentType {
        SegmentType::JournalSeg
    }

    fn produce(&self) -> Result<Vec<u8>, RvfError> {
        serde_json::to_vec(&self.results).map_err(|e| {
            RvfError::DeserializationFailed(format!("solver result serialization: {e}"))
        })
    }
}
