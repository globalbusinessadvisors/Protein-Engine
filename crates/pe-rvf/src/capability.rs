use serde::{Deserialize, Serialize};

use crate::segment::SegmentType;

/// Capabilities that an RVF file can expose, auto-inferred from present segments.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Capability {
    VecSearch,
    ProteinScoring,
    Evolution,
    WasmRuntime,
    QuantumVqe,
    P2pSync,
    McpAgent,
    TeeAttestation,
}

/// Returns the capabilities implied by the presence of a given segment type.
pub fn capabilities_for_segment(seg: SegmentType) -> &'static [Capability] {
    match seg {
        SegmentType::VecSeg => &[Capability::VecSearch],
        SegmentType::IndexSeg => &[Capability::VecSearch],
        SegmentType::OverlaySeg => &[Capability::ProteinScoring],
        SegmentType::GraphSeg => &[Capability::ProteinScoring],
        SegmentType::QuantSeg => &[Capability::ProteinScoring],
        SegmentType::WasmSeg => &[Capability::WasmRuntime],
        SegmentType::SketchSeg => &[Capability::QuantumVqe],
        SegmentType::WitnessSeg => &[Capability::P2pSync],
        SegmentType::CryptoSeg => &[Capability::TeeAttestation],
        SegmentType::KernelSeg => &[Capability::McpAgent],
        SegmentType::JournalSeg => &[Capability::Evolution],
        SegmentType::HotSeg => &[Capability::Evolution],
        _ => &[],
    }
}
