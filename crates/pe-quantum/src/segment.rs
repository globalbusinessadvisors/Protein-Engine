use pe_quantum_wasm::VqeResult;
use pe_rvf::{RvfError, SegmentProducer, SegmentType};

use crate::error::QuantumRouterError;

/// A cache of VQE results, stored in SKETCH_SEG to avoid redundant recomputation.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct VqeSnapshotCache {
    pub snapshots: Vec<VqeSnapshot>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct VqeSnapshot {
    pub label: String,
    pub result: VqeResult,
}

impl VqeSnapshotCache {
    pub fn new() -> Self {
        Self {
            snapshots: Vec::new(),
        }
    }

    pub fn add(&mut self, label: String, result: VqeResult) {
        self.snapshots.push(VqeSnapshot { label, result });
    }

    pub fn to_bytes(&self) -> Result<Vec<u8>, QuantumRouterError> {
        serde_json::to_vec(self)
            .map_err(|e| QuantumRouterError::SerializationFailed(e.to_string()))
    }

    pub fn from_bytes(data: &[u8]) -> Result<Self, QuantumRouterError> {
        serde_json::from_slice(data)
            .map_err(|e| QuantumRouterError::DeserializationFailed(e.to_string()))
    }
}

impl Default for VqeSnapshotCache {
    fn default() -> Self {
        Self::new()
    }
}

/// Produces SKETCH_SEG bytes containing cached VQE snapshots.
pub struct SketchSegProducer {
    cache: VqeSnapshotCache,
}

impl SketchSegProducer {
    pub fn new(cache: VqeSnapshotCache) -> Self {
        Self { cache }
    }
}

impl SegmentProducer for SketchSegProducer {
    fn segment_type(&self) -> SegmentType {
        SegmentType::SketchSeg
    }

    fn produce(&self) -> Result<Vec<u8>, RvfError> {
        serde_json::to_vec(&self.cache).map_err(|e| {
            RvfError::DeserializationFailed(format!("VQE snapshot serialization: {e}"))
        })
    }
}
