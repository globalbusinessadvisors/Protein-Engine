use pe_core::{Embedding320, FitnessScore, FitnessWeights, ProteinVariant};

use crate::error::NeuralError;
use crate::traits::{FitnessPredictor, SubModelScorer};

/// Ensemble fitness predictor composed of three sub-model scorers.
///
/// - `T` (Transformer): scores reprogramming efficiency
/// - `L` (LSTM): scores expression stability
/// - `N` (N-BEATS): scores structural plausibility
///
/// The safety score is derived as `1.0 - min(transformer, lstm, nbeats)`,
/// representing a conservative risk estimate.
pub struct EnsemblePredictor<T, L, N>
where
    T: SubModelScorer,
    L: SubModelScorer,
    N: SubModelScorer,
{
    transformer: T,
    lstm: L,
    nbeats: N,
    weights: FitnessWeights,
}

impl<T, L, N> EnsemblePredictor<T, L, N>
where
    T: SubModelScorer,
    L: SubModelScorer,
    N: SubModelScorer,
{
    pub fn new(transformer: T, lstm: L, nbeats: N, weights: FitnessWeights) -> Self {
        Self {
            transformer,
            lstm,
            nbeats,
            weights,
        }
    }
}

impl<T, L, N> FitnessPredictor for EnsemblePredictor<T, L, N>
where
    T: SubModelScorer,
    L: SubModelScorer,
    N: SubModelScorer,
{
    fn predict(
        &self,
        variant: &ProteinVariant,
        embedding: &Embedding320,
    ) -> Result<FitnessScore, NeuralError> {
        let reprogramming = self.transformer.score(variant, embedding).map_err(|e| {
            NeuralError::SubModelFailed {
                model: self.transformer.model_name().to_string(),
                reason: e.to_string(),
            }
        })?;

        let stability = self.lstm.score(variant, embedding).map_err(|e| {
            NeuralError::SubModelFailed {
                model: self.lstm.model_name().to_string(),
                reason: e.to_string(),
            }
        })?;

        let plausibility = self.nbeats.score(variant, embedding).map_err(|e| {
            NeuralError::SubModelFailed {
                model: self.nbeats.model_name().to_string(),
                reason: e.to_string(),
            }
        })?;

        // Safety score: conservative risk estimate — lower is safer.
        // Uses the minimum sub-score inverted: high confidence across all
        // models → low risk.
        let min_score = reprogramming.min(stability).min(plausibility);
        let safety_score = 1.0 - min_score;

        FitnessScore::new(
            reprogramming,
            stability,
            plausibility,
            safety_score,
            &self.weights,
        )
        .map_err(|e| NeuralError::FitnessScoreError(e.to_string()))
    }
}
