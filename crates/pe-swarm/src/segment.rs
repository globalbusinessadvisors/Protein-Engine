//! HOT_SEG SegmentProducer — serializes top-100 promoted candidates.

use pe_core::ScoredVariant;
use pe_rvf::error::RvfError;
use pe_rvf::segment::SegmentType;
use pe_rvf::traits::SegmentProducer;

/// Produces HOT_SEG: the top promoted candidates from the latest cycle.
pub struct HotSegProducer {
    candidates: Vec<ScoredVariant>,
}

impl HotSegProducer {
    /// Create a producer with up to 100 candidates.
    pub fn new(mut candidates: Vec<ScoredVariant>) -> Self {
        candidates.truncate(100);
        Self { candidates }
    }
}

impl SegmentProducer for HotSegProducer {
    fn segment_type(&self) -> SegmentType {
        SegmentType::HotSeg
    }

    fn produce(&self) -> Result<Vec<u8>, RvfError> {
        serde_json::to_vec(&self.candidates)
            .map_err(|e| RvfError::SerializationFailed(e.to_string()))
    }
}
