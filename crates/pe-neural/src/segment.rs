use pe_rvf::{RvfError, SegmentProducer, SegmentType};
use serde::{Deserialize, Serialize};

use crate::error::NeuralError;
use crate::scorers::{LstmScorer, NBeatsScorer, TransformerScorer};

/// Serializable model weight bundle for all three scorers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelWeights {
    pub transformer: TransformerScorer,
    pub lstm: LstmScorer,
    pub nbeats: NBeatsScorer,
}

impl ModelWeights {
    pub fn to_bytes(&self) -> Result<Vec<u8>, NeuralError> {
        serde_json::to_vec(self)
            .map_err(|e| NeuralError::SerializationFailed(e.to_string()))
    }

    pub fn from_bytes(data: &[u8]) -> Result<Self, NeuralError> {
        serde_json::from_slice(data)
            .map_err(|e| NeuralError::DeserializationFailed(e.to_string()))
    }
}

/// Produces QUANT_SEG bytes containing serialized model weights.
pub struct QuantSegProducer {
    weights: ModelWeights,
}

impl QuantSegProducer {
    pub fn new(weights: ModelWeights) -> Self {
        Self { weights }
    }
}

impl SegmentProducer for QuantSegProducer {
    fn segment_type(&self) -> SegmentType {
        SegmentType::QuantSeg
    }

    fn produce(&self) -> Result<Vec<u8>, RvfError> {
        self.weights
            .to_bytes()
            .map_err(|e| RvfError::SegmentTooLarge(e.to_string().len()))
    }
}
