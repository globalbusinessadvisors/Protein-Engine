use crate::error::RvfError;
use crate::manifest::Manifest;
use crate::rvf_file::RvfFile;
use crate::segment::SegmentType;

/// Produces the raw bytes for a single RVF segment.
///
/// Implementations are provided by each domain crate (pe-vector, pe-neural, etc.)
/// and injected into the builder. Mockable for London School TDD.
#[cfg_attr(test, mockall::automock)]
pub trait SegmentProducer: Send + Sync {
    /// The segment type this producer is responsible for.
    fn segment_type(&self) -> SegmentType;

    /// Produce the raw bytes for this segment.
    fn produce(&self) -> Result<Vec<u8>, RvfError>;
}

/// Assembles an immutable `RvfFile` from a manifest and segments.
pub trait RvfAssembler: Send + Sync {
    /// Set the manifest for the file being built.
    fn set_manifest(&mut self, manifest: Manifest);

    /// Add raw segment data. Fails on duplicate segment types.
    fn add_segment(&mut self, seg_type: SegmentType, data: Vec<u8>) -> Result<(), RvfError>;

    /// Consume the builder and produce the final `RvfFile`.
    ///
    /// Enforces all invariants: manifest presence (RF-1), capability inference
    /// (RF-2–RF-4), segment ordering (RF-5), and hash computation (RF-6).
    fn build(self) -> Result<RvfFile, RvfError>;
}
