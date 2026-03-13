use serde::{Deserialize, Serialize};

/// All 15 segment types defined in ADR-001's segment allocation table.
///
/// Discriminants are fixed to match the RVF wire format.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum SegmentType {
    /// Root metadata, capabilities, lineage hash
    ManifestSeg = 0x00,
    /// 320-dim ESM-2 protein embeddings
    VecSeg = 0x01,
    /// HNSW nearest-neighbor index
    IndexSeg = 0x02,
    /// LoRA adapter deltas
    OverlaySeg = 0x03,
    /// Append-only experiment log
    JournalSeg = 0x04,
    /// GNN protein interaction network
    GraphSeg = 0x05,
    /// INT8 quantized neural weights
    QuantSeg = 0x06,
    /// Per-variant filterable metadata
    MetaSeg = 0x07,
    /// Top-100 promoted candidates
    HotSeg = 0x08,
    /// MinHash sketches + VQE snapshots
    SketchSeg = 0x09,
    /// 5.5KB WASM microkernel
    WasmSeg = 0x0A,
    /// QuDAG cryptographic witness chain
    WitnessSeg = 0x0B,
    /// TEE attestation
    CryptoSeg = 0x0C,
    /// Filterable metadata index
    MetaIdxSeg = 0x0D,
    /// Optional unikernel
    KernelSeg = 0x0E,
}

impl SegmentType {
    /// All segment types in discriminant order.
    pub const ALL: [SegmentType; 15] = [
        SegmentType::ManifestSeg,
        SegmentType::VecSeg,
        SegmentType::IndexSeg,
        SegmentType::OverlaySeg,
        SegmentType::JournalSeg,
        SegmentType::GraphSeg,
        SegmentType::QuantSeg,
        SegmentType::MetaSeg,
        SegmentType::HotSeg,
        SegmentType::SketchSeg,
        SegmentType::WasmSeg,
        SegmentType::WitnessSeg,
        SegmentType::CryptoSeg,
        SegmentType::MetaIdxSeg,
        SegmentType::KernelSeg,
    ];

    /// Convert a raw `u8` discriminant to a `SegmentType`.
    pub fn from_u8(val: u8) -> Option<SegmentType> {
        match val {
            0x00 => Some(SegmentType::ManifestSeg),
            0x01 => Some(SegmentType::VecSeg),
            0x02 => Some(SegmentType::IndexSeg),
            0x03 => Some(SegmentType::OverlaySeg),
            0x04 => Some(SegmentType::JournalSeg),
            0x05 => Some(SegmentType::GraphSeg),
            0x06 => Some(SegmentType::QuantSeg),
            0x07 => Some(SegmentType::MetaSeg),
            0x08 => Some(SegmentType::HotSeg),
            0x09 => Some(SegmentType::SketchSeg),
            0x0A => Some(SegmentType::WasmSeg),
            0x0B => Some(SegmentType::WitnessSeg),
            0x0C => Some(SegmentType::CryptoSeg),
            0x0D => Some(SegmentType::MetaIdxSeg),
            0x0E => Some(SegmentType::KernelSeg),
            _ => None,
        }
    }

    /// Returns the `u8` discriminant.
    pub fn as_u8(self) -> u8 {
        self as u8
    }
}
