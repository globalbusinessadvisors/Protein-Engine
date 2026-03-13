use pe_core::{Embedding320, ProteinVariant};
use serde::{Deserialize, Serialize};

use crate::error::NeuralError;
use crate::traits::{ModelLoader, SubModelScorer};

// ────────────────────────────────────────────────────────────────────
// TransformerScorer
// ────────────────────────────────────────────────────────────────────

/// Stub Transformer scorer for reprogramming efficiency.
///
/// Placeholder heuristic: scores based on hydrophobicity distribution
/// of the sequence. Real implementation will load weights from QUANT_SEG.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransformerScorer {
    /// Bias added to the heuristic score (loaded from model weights).
    bias: f64,
}

impl TransformerScorer {
    pub fn new(bias: f64) -> Self {
        Self { bias }
    }
}

impl SubModelScorer for TransformerScorer {
    fn score(
        &self,
        variant: &ProteinVariant,
        embedding: &Embedding320,
    ) -> Result<f64, NeuralError> {
        // Heuristic: use embedding magnitude as a proxy for sequence quality,
        // normalized to [0, 1] via sigmoid-like transform.
        let norm = embedding.norm() as f64;
        let raw = 1.0 / (1.0 + (-norm + 10.0).exp());

        // Factor in sequence composition: penalise very short sequences
        let len_factor = (variant.sequence().len() as f64).min(500.0) / 500.0;

        let score = (raw * 0.7 + len_factor * 0.3 + self.bias).clamp(0.0, 1.0);
        Ok(score)
    }

    fn model_name(&self) -> &str {
        "transformer"
    }
}

impl ModelLoader for TransformerScorer {
    fn load_from_bytes(data: &[u8]) -> Result<Self, NeuralError> {
        serde_json::from_slice(data)
            .map_err(|e| NeuralError::InvalidWeights(format!("transformer: {e}")))
    }
}

// ────────────────────────────────────────────────────────────────────
// LstmScorer
// ────────────────────────────────────────────────────────────────────

/// Stub LSTM scorer for expression stability.
///
/// Placeholder heuristic: scores based on sequence length and mutation count.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LstmScorer {
    baseline: f64,
}

impl LstmScorer {
    pub fn new(baseline: f64) -> Self {
        Self { baseline }
    }
}

impl SubModelScorer for LstmScorer {
    fn score(
        &self,
        variant: &ProteinVariant,
        _embedding: &Embedding320,
    ) -> Result<f64, NeuralError> {
        // Heuristic: stability decreases with mutation count
        let mutation_penalty = variant.mutations().len() as f64 * 0.05;
        // Longer sequences tend to be more stable (up to a point)
        let len_bonus = (variant.sequence().len() as f64).min(300.0) / 300.0 * 0.2;

        let score = (self.baseline + len_bonus - mutation_penalty).clamp(0.0, 1.0);
        Ok(score)
    }

    fn model_name(&self) -> &str {
        "lstm"
    }
}

impl ModelLoader for LstmScorer {
    fn load_from_bytes(data: &[u8]) -> Result<Self, NeuralError> {
        serde_json::from_slice(data)
            .map_err(|e| NeuralError::InvalidWeights(format!("lstm: {e}")))
    }
}

// ────────────────────────────────────────────────────────────────────
// NBeatsScorer
// ────────────────────────────────────────────────────────────────────

/// Stub N-BEATS scorer for structural plausibility / outcome forecasting.
///
/// Placeholder: returns a configurable baseline score.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NBeatsScorer {
    baseline: f64,
}

impl NBeatsScorer {
    pub fn new(baseline: f64) -> Self {
        Self { baseline }
    }
}

impl SubModelScorer for NBeatsScorer {
    fn score(
        &self,
        _variant: &ProteinVariant,
        _embedding: &Embedding320,
    ) -> Result<f64, NeuralError> {
        Ok(self.baseline.clamp(0.0, 1.0))
    }

    fn model_name(&self) -> &str {
        "nbeats"
    }
}

impl ModelLoader for NBeatsScorer {
    fn load_from_bytes(data: &[u8]) -> Result<Self, NeuralError> {
        serde_json::from_slice(data)
            .map_err(|e| NeuralError::InvalidWeights(format!("nbeats: {e}")))
    }
}
