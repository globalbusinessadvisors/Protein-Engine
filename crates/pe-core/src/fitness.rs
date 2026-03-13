//! Fitness scoring types for protein variant evaluation.

use crate::sequence::CoreError;
use crate::variant::ProteinVariant;
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// FitnessWeights
// ---------------------------------------------------------------------------

/// Normalised weight vector that sums to 1.0.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct FitnessWeights {
    pub reprogramming: f64,
    pub stability: f64,
    pub plausibility: f64,
    pub safety: f64,
}

impl FitnessWeights {
    /// Create a new weight vector.
    ///
    /// All weights must be >= 0.0 and must sum to 1.0 (within 1e-9).
    pub fn new(
        reprogramming: f64,
        stability: f64,
        plausibility: f64,
        safety: f64,
    ) -> Result<Self, CoreError> {
        if reprogramming < 0.0 || stability < 0.0 || plausibility < 0.0 || safety < 0.0 {
            return Err(CoreError::NegativeWeight);
        }
        let sum = reprogramming + stability + plausibility + safety;
        if (sum - 1.0).abs() >= 1e-9 {
            return Err(CoreError::WeightsSumInvalid { sum });
        }
        Ok(Self {
            reprogramming,
            stability,
            plausibility,
            safety,
        })
    }

    /// Default weights: (0.35, 0.25, 0.25, 0.15).
    pub fn default_weights() -> Self {
        Self {
            reprogramming: 0.35,
            stability: 0.25,
            plausibility: 0.25,
            safety: 0.15,
        }
    }
}

// ---------------------------------------------------------------------------
// FitnessScore
// ---------------------------------------------------------------------------

/// Composite fitness score for a protein variant.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct FitnessScore {
    reprogramming_efficiency: f64,
    expression_stability: f64,
    structural_plausibility: f64,
    safety_score: f64,
    composite: f64,
}

/// Validate that a sub-score is a finite number in [0.0, 1.0].
fn validate_subscore(value: f64, field: &'static str) -> Result<(), CoreError> {
    if value.is_nan() {
        return Err(CoreError::FitnessScoreNaN { field });
    }
    if !(0.0..=1.0).contains(&value) {
        return Err(CoreError::FitnessScoreOutOfRange { field, value });
    }
    Ok(())
}

impl FitnessScore {
    /// Compute a composite fitness score from individual sub-scores and weights.
    ///
    /// Each sub-score must be in [0.0, 1.0].
    /// The safety contribution is **inverted**: a lower `safety_score` (safer)
    /// yields a higher composite contribution.
    pub fn new(
        reprogramming_efficiency: f64,
        expression_stability: f64,
        structural_plausibility: f64,
        safety_score: f64,
        weights: &FitnessWeights,
    ) -> Result<Self, CoreError> {
        validate_subscore(reprogramming_efficiency, "reprogramming_efficiency")?;
        validate_subscore(expression_stability, "expression_stability")?;
        validate_subscore(structural_plausibility, "structural_plausibility")?;
        validate_subscore(safety_score, "safety_score")?;

        let composite = weights.reprogramming * reprogramming_efficiency
            + weights.stability * expression_stability
            + weights.plausibility * structural_plausibility
            + weights.safety * (1.0 - safety_score);

        Ok(Self {
            reprogramming_efficiency,
            expression_stability,
            structural_plausibility,
            safety_score,
            composite,
        })
    }

    /// Reprogramming efficiency sub-score.
    pub fn reprogramming_efficiency(&self) -> f64 {
        self.reprogramming_efficiency
    }

    /// Expression stability sub-score.
    pub fn expression_stability(&self) -> f64 {
        self.expression_stability
    }

    /// Structural plausibility sub-score.
    pub fn structural_plausibility(&self) -> f64 {
        self.structural_plausibility
    }

    /// Raw safety sub-score (lower is safer).
    pub fn safety_score(&self) -> f64 {
        self.safety_score
    }

    /// Weighted composite score.
    pub fn composite(&self) -> f64 {
        self.composite
    }
}

// ---------------------------------------------------------------------------
// ScoredVariant
// ---------------------------------------------------------------------------

/// A protein variant paired with its fitness score.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct ScoredVariant {
    pub variant: ProteinVariant,
    pub score: FitnessScore,
}
