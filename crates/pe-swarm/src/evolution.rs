//! SimpleEvolutionEngine — concrete EvolutionEngine implementation.

use rand::Rng;

use pe_core::{AminoAcid, Mutation, ProteinVariant, ScoredVariant};

use crate::error::SwarmError;
use crate::traits::EvolutionEngine;

const AMINO_ACIDS: [AminoAcid; 20] = [
    AminoAcid::Ala,
    AminoAcid::Cys,
    AminoAcid::Asp,
    AminoAcid::Glu,
    AminoAcid::Phe,
    AminoAcid::Gly,
    AminoAcid::His,
    AminoAcid::Ile,
    AminoAcid::Lys,
    AminoAcid::Leu,
    AminoAcid::Met,
    AminoAcid::Asn,
    AminoAcid::Pro,
    AminoAcid::Gln,
    AminoAcid::Arg,
    AminoAcid::Ser,
    AminoAcid::Thr,
    AminoAcid::Val,
    AminoAcid::Trp,
    AminoAcid::Tyr,
];

/// Simple evolutionary engine using random single-point mutation,
/// single-point crossover, and fitness-proportional selection.
pub struct SimpleEvolutionEngine;

impl SimpleEvolutionEngine {
    pub fn new() -> Self {
        Self
    }
}

impl Default for SimpleEvolutionEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl EvolutionEngine for SimpleEvolutionEngine {
    fn mutate(&self, variant: &ProteinVariant) -> Result<ProteinVariant, SwarmError> {
        let mut rng = rand::thread_rng();
        let seq = variant.sequence();
        let len = seq.len();
        let pos = rng.gen_range(0..len);
        let from = seq.as_slice()[pos];

        // Pick a different amino acid
        let mut to = from;
        while to == from {
            to = AMINO_ACIDS[rng.gen_range(0..20)];
        }

        let mutation =
            Mutation::new(pos, from, to).map_err(|e| SwarmError::EvolutionFailed(e.to_string()))?;
        ProteinVariant::from_mutation(variant, mutation)
            .map_err(|e| SwarmError::EvolutionFailed(e.to_string()))
    }

    fn crossover(
        &self,
        a: &ProteinVariant,
        b: &ProteinVariant,
    ) -> Result<ProteinVariant, SwarmError> {
        let mut rng = rand::thread_rng();
        let len = a.sequence().len();

        if len < 2 {
            return Err(SwarmError::EvolutionFailed(
                "sequence too short for crossover".into(),
            ));
        }

        let point = rng.gen_range(1..len);
        ProteinVariant::from_crossover(a, b, point)
            .map_err(|e| SwarmError::EvolutionFailed(e.to_string()))
    }

    fn select(&self, population: &[ScoredVariant], top_k: usize) -> Vec<ScoredVariant> {
        let mut sorted = population.to_vec();
        sorted.sort_by(|a, b| {
            b.score
                .composite()
                .partial_cmp(&a.score.composite())
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        sorted.truncate(top_k);
        sorted
    }
}
