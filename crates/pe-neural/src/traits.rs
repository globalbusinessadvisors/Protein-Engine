use pe_core::{Embedding320, FitnessScore, ProteinVariant};

use crate::error::NeuralError;

/// Scores a single aspect of protein fitness (e.g. reprogramming, stability, outcome).
///
/// Three concrete scorers (Transformer, LSTM, N-BEATS) are composed by
/// `EnsemblePredictor` to produce a complete `FitnessScore`.
#[cfg_attr(test, mockall::automock)]
pub trait SubModelScorer: Send + Sync {
    fn score(&self, variant: &ProteinVariant, embedding: &Embedding320) -> Result<f64, NeuralError>;
    fn model_name(&self) -> &str;
}

/// Predicts the full composite fitness of a protein variant.
#[cfg_attr(test, mockall::automock)]
pub trait FitnessPredictor: Send + Sync {
    fn predict(
        &self,
        variant: &ProteinVariant,
        embedding: &Embedding320,
    ) -> Result<FitnessScore, NeuralError>;
}

/// Loads model weights from raw bytes (e.g. QUANT_SEG data).
pub trait ModelLoader: Send + Sync {
    fn load_from_bytes(data: &[u8]) -> Result<Self, NeuralError>
    where
        Self: Sized;
}
