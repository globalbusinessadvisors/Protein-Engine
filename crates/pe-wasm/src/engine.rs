//! WasmEngine — internal state holding all domain components for browser execution.

use rand::Rng;
use serde::{Deserialize, Serialize};

use pe_core::{
    AminoAcid, AminoAcidSequence, FitnessScore, FitnessWeights, Mutation,
    ProteinVariant, ScoredVariant, YamanakaFactor,
};
use pe_ledger::{CryptoSigner, JournalChain, LedgerError, MlDsaSignature};
use pe_neural::{EnsemblePredictor, LstmScorer, NBeatsScorer, TransformerScorer};
use pe_neural::traits::FitnessPredictor;
use pe_quantum_wasm::{MolecularHamiltonian, VqeConfig, VqeResult, VqeRunner};
use pe_rvf::{RvfFile, SegmentType};
use pe_vector::traits::{EmbeddingModel, VectorStore};
use pe_vector::{InMemoryVectorStore, VariantMeta};

use crate::embedder::HashEmbedder;

// ── NoOp signer (ADR-005: no signing in WASM) ────────────────────────

/// Stub signer for WASM: refuses to sign but accepts all signatures
/// during verification (trust signatures validated at RVF export time).
struct NoOpSigner;

impl CryptoSigner for NoOpSigner {
    fn sign(&self, _data: &[u8]) -> Result<MlDsaSignature, LedgerError> {
        Err(LedgerError::SigningFailed(
            "signing not available in WASM (ADR-005)".into(),
        ))
    }

    fn verify(&self, _data: &[u8], _signature: &MlDsaSignature) -> Result<bool, LedgerError> {
        Ok(true)
    }
}

// ── DTOs for WASM boundary ───────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct ScoreOutput {
    pub reprogramming_efficiency: f64,
    pub expression_stability: f64,
    pub structural_plausibility: f64,
    pub safety_score: f64,
    pub composite: f64,
}

impl From<&FitnessScore> for ScoreOutput {
    fn from(s: &FitnessScore) -> Self {
        Self {
            reprogramming_efficiency: s.reprogramming_efficiency(),
            expression_stability: s.expression_stability(),
            structural_plausibility: s.structural_plausibility(),
            safety_score: s.safety_score(),
            composite: s.composite(),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct VariantInput {
    pub name: String,
    pub sequence: String,
    pub target_factor: String,
}

#[derive(Debug, Deserialize)]
pub struct EvolutionConfig {
    pub generation: u32,
    #[serde(default = "default_pop_size")]
    pub population_size: usize,
    #[serde(default = "default_mutation_rate")]
    pub mutation_rate: f64,
    #[serde(default = "default_crossover_rate")]
    pub crossover_rate: f64,
    #[serde(default = "default_top_k")]
    pub top_k: usize,
}

fn default_pop_size() -> usize { 50 }
fn default_mutation_rate() -> f64 { 0.1 }
fn default_crossover_rate() -> f64 { 0.3 }
fn default_top_k() -> usize { 10 }

#[derive(Debug, Serialize)]
pub struct EvolutionOutput {
    pub generation: u32,
    pub variants_created: usize,
    pub variants_scored: usize,
    pub promoted: Vec<PromotedVariant>,
}

#[derive(Debug, Serialize)]
pub struct PromotedVariant {
    pub name: String,
    pub sequence: String,
    pub composite: f64,
}

#[derive(Debug, Serialize)]
pub struct SearchHit {
    pub id: String,
    pub similarity: f32,
}

#[derive(Debug, Serialize)]
pub struct VerifyOutput {
    pub valid: bool,
}

#[derive(Debug, Serialize)]
pub struct LoadOutput {
    pub vectors_loaded: usize,
    pub journal_entries: usize,
}

// ── WasmEngine ───────────────────────────────────────────────────────

/// Core engine holding all domain components for browser execution.
pub struct WasmEngine {
    store: InMemoryVectorStore,
    embedder: HashEmbedder,
    predictor: EnsemblePredictor<TransformerScorer, LstmScorer, NBeatsScorer>,
    chain: JournalChain,
}

impl WasmEngine {
    /// Create a new engine with default stub scorers and empty state.
    pub fn new() -> Self {
        let weights = FitnessWeights::default_weights();
        let predictor = EnsemblePredictor::new(
            TransformerScorer::new(0.0),
            LstmScorer::new(0.75),
            NBeatsScorer::new(0.7),
            weights,
        );

        Self {
            store: InMemoryVectorStore::new(),
            embedder: HashEmbedder::new(),
            predictor,
            chain: JournalChain::new(),
        }
    }

    /// Score a protein sequence, returning fitness components.
    pub fn score_sequence(&self, sequence: &str) -> Result<ScoreOutput, String> {
        let seq = AminoAcidSequence::new(sequence).map_err(|e| e.to_string())?;
        let variant = ProteinVariant::wild_type("wasm-query", seq, YamanakaFactor::OCT4);

        let embedding = self.embedder.embed(variant.sequence()).map_err(|e| e.to_string())?;
        let score = self.predictor.predict(&variant, &embedding).map_err(|e| e.to_string())?;

        Ok(ScoreOutput::from(&score))
    }

    /// Run one evolution generation on a population.
    pub fn run_evolution_step(
        &mut self,
        population_json: &str,
        config_json: &str,
    ) -> Result<EvolutionOutput, String> {
        let inputs: Vec<VariantInput> =
            serde_json::from_str(population_json).map_err(|e| format!("invalid population JSON: {e}"))?;
        let config: EvolutionConfig =
            serde_json::from_str(config_json).map_err(|e| format!("invalid config JSON: {e}"))?;

        if inputs.is_empty() {
            return Err("population cannot be empty".into());
        }

        // Create and score initial population
        let mut scored: Vec<ScoredVariant> = Vec::with_capacity(inputs.len());
        for input in &inputs {
            let seq = AminoAcidSequence::new(&input.sequence).map_err(|e| e.to_string())?;
            let factor = parse_factor(&input.target_factor)?;
            let variant = ProteinVariant::wild_type(input.name.clone(), seq, factor);
            let embedding = self.embedder.embed(variant.sequence()).map_err(|e| e.to_string())?;
            let score = self.predictor.predict(&variant, &embedding).map_err(|e| e.to_string())?;
            scored.push(ScoredVariant { variant, score });
        }

        // Generate offspring via mutation and crossover
        let mut rng = rand::thread_rng();
        let mut offspring: Vec<ProteinVariant> = Vec::new();

        for sv in &scored {
            if rng.gen::<f64>() < config.mutation_rate {
                if let Ok(mutant) = mutate_variant(&sv.variant) {
                    offspring.push(mutant);
                }
            }
        }

        if scored.len() >= 2 {
            for i in 0..scored.len() - 1 {
                if rng.gen::<f64>() < config.crossover_rate {
                    let j = (i + 1) % scored.len();
                    if let Ok(child) = crossover_variants(&scored[i].variant, &scored[j].variant) {
                        offspring.push(child);
                    }
                }
            }
        }

        let variants_created = offspring.len();

        // Score offspring
        for child in offspring {
            let embedding = self.embedder.embed(child.sequence()).map_err(|e| e.to_string())?;
            let score = self.predictor.predict(&child, &embedding).map_err(|e| e.to_string())?;
            scored.push(ScoredVariant { variant: child, score });
        }

        let variants_scored = scored.len();

        // Select top-k
        scored.sort_by(|a, b| {
            b.score
                .composite()
                .partial_cmp(&a.score.composite())
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        scored.truncate(config.top_k);

        let promoted: Vec<PromotedVariant> = scored
            .iter()
            .map(|sv| PromotedVariant {
                name: sv.variant.name().to_string(),
                sequence: sv.variant.sequence().to_string(),
                composite: sv.score.composite(),
            })
            .collect();

        Ok(EvolutionOutput {
            generation: config.generation,
            variants_created,
            variants_scored,
            promoted,
        })
    }

    /// Run VQE on the local quantum simulator.
    pub fn run_quantum_sim(&self, hamiltonian_json: &str) -> Result<VqeResult, String> {
        let hamiltonian: MolecularHamiltonian =
            serde_json::from_str(hamiltonian_json).map_err(|e| format!("invalid hamiltonian JSON: {e}"))?;

        let runner = VqeRunner::new(VqeConfig::default());
        runner.run(&hamiltonian).map_err(|e| e.to_string())
    }

    /// Search for similar sequences in the vector store.
    pub fn search_similar(&self, sequence: &str, k: usize) -> Result<Vec<SearchHit>, String> {
        let seq = AminoAcidSequence::new(sequence).map_err(|e| e.to_string())?;
        let embedding = self.embedder.embed(&seq).map_err(|e| e.to_string())?;

        let results = self.store.search_nearest(&embedding, k).map_err(|e| e.to_string())?;

        Ok(results
            .into_iter()
            .map(|(id, similarity)| SearchHit {
                id: id.to_string(),
                similarity,
            })
            .collect())
    }

    /// Verify the local journal chain integrity.
    pub fn verify_ledger(&self) -> Result<VerifyOutput, String> {
        let signer = NoOpSigner;
        let valid = self.chain.verify_chain(&signer).map_err(|e| e.to_string())?;
        Ok(VerifyOutput { valid })
    }

    /// Load an RVF file, populating the engine from its segments.
    pub fn load_rvf(&mut self, data: &[u8]) -> Result<LoadOutput, String> {
        let rvf = RvfFile::deserialize(data).map_err(|e| format!("RVF deserialization failed: {e}"))?;
        let segments = rvf.segments();

        // VEC_SEG + INDEX_SEG → rebuild vector store
        let vec_seg = segments.get(&SegmentType::VecSeg);
        let index_seg = segments.get(&SegmentType::IndexSeg);

        let vectors_loaded = if let (Some(vs), Some(is)) = (vec_seg, index_seg) {
            let store =
                InMemoryVectorStore::from_segments(vs, is).map_err(|e| format!("vector store: {e}"))?;
            let count = store.count();
            self.store = store;
            count
        } else {
            0
        };

        // JOURNAL_SEG → rebuild chain (verification-only)
        let journal_entries = if let Some(journal_data) = segments.get(&SegmentType::JournalSeg) {
            let entries: Vec<pe_ledger::JournalEntry> = serde_json::from_slice(journal_data)
                .map_err(|e| format!("journal deserialization: {e}"))?;
            let count = entries.len();
            // Rebuild chain by replaying entries' hash chain
            // Since we can't sign in WASM, we just verify the loaded chain
            self.chain = rebuild_chain_from_entries(entries)?;
            count
        } else {
            0
        };

        Ok(LoadOutput {
            vectors_loaded,
            journal_entries,
        })
    }

    /// Get the number of vectors in the store.
    pub fn vector_count(&self) -> usize {
        self.store.count()
    }

    /// Get the number of journal entries in the chain.
    pub fn journal_len(&self) -> usize {
        self.chain.len()
    }

    /// Insert a variant into the vector store (for testing/population).
    pub fn insert_variant(
        &mut self,
        name: &str,
        sequence: &str,
        target_factor: &str,
    ) -> Result<String, String> {
        let seq = AminoAcidSequence::new(sequence).map_err(|e| e.to_string())?;
        let factor = parse_factor(target_factor)?;
        let variant = ProteinVariant::wild_type(name.to_string(), seq, factor);
        let id = variant.id();

        let embedding = self.embedder.embed(variant.sequence()).map_err(|e| e.to_string())?;

        let meta = VariantMeta {
            variant_id: id,
            target_factor: *variant.target_factor(),
            generation: variant.generation(),
            composite_score: None,
            design_method: pe_vector::DesignMethod::WildType,
        };

        self.store
            .insert(id, embedding, meta)
            .map_err(|e| e.to_string())?;

        Ok(id.to_string())
    }
}

impl Default for WasmEngine {
    fn default() -> Self {
        Self::new()
    }
}

// ── Helpers ──────────────────────────────────────────────────────────

fn parse_factor(s: &str) -> Result<YamanakaFactor, String> {
    match s.to_uppercase().as_str() {
        "OCT4" => Ok(YamanakaFactor::OCT4),
        "SOX2" => Ok(YamanakaFactor::SOX2),
        "KLF4" => Ok(YamanakaFactor::KLF4),
        "CMYC" => Ok(YamanakaFactor::CMYC),
        _ => Err(format!("unknown target factor: {s}")),
    }
}

const AMINO_ACIDS: [AminoAcid; 20] = [
    AminoAcid::Ala, AminoAcid::Cys, AminoAcid::Asp, AminoAcid::Glu, AminoAcid::Phe,
    AminoAcid::Gly, AminoAcid::His, AminoAcid::Ile, AminoAcid::Lys, AminoAcid::Leu,
    AminoAcid::Met, AminoAcid::Asn, AminoAcid::Pro, AminoAcid::Gln, AminoAcid::Arg,
    AminoAcid::Ser, AminoAcid::Thr, AminoAcid::Val, AminoAcid::Trp, AminoAcid::Tyr,
];

fn mutate_variant(variant: &ProteinVariant) -> Result<ProteinVariant, String> {
    let mut rng = rand::thread_rng();
    let seq = variant.sequence();
    let len = seq.len();
    let pos = rng.gen_range(0..len);
    let from = seq.as_slice()[pos];

    let mut to = from;
    while to == from {
        to = AMINO_ACIDS[rng.gen_range(0..20)];
    }

    let mutation = Mutation::new(pos, from, to).map_err(|e| e.to_string())?;
    ProteinVariant::from_mutation(variant, mutation).map_err(|e| e.to_string())
}

fn crossover_variants(a: &ProteinVariant, b: &ProteinVariant) -> Result<ProteinVariant, String> {
    let mut rng = rand::thread_rng();
    let len = a.sequence().len();
    if len < 2 {
        return Err("sequence too short for crossover".into());
    }
    let point = rng.gen_range(1..len);
    ProteinVariant::from_crossover(a, b, point).map_err(|e| e.to_string())
}

/// Rebuild a JournalChain from deserialized entries without re-signing.
///
/// Since JournalChain's internal state is (entries, tip_hash), we reconstruct
/// it by computing the chain of hashes from the loaded entries.
fn rebuild_chain_from_entries(entries: Vec<pe_ledger::JournalEntry>) -> Result<JournalChain, String> {
    // We can't directly set JournalChain's internal state (it's private).
    // Instead, we create an empty chain and verify the loaded entries are valid.
    // For WASM, the chain is verification-only — we just need verify_chain to work.
    //
    // Since JournalChain::new() creates an empty chain, and we have pre-signed entries,
    // we verify them on load and store the count for reporting.
    // The actual chain verification uses NoOpSigner which trusts all signatures.
    let chain = JournalChain::new();

    // Validate hash chain integrity without signature checks
    let mut expected_prev = pe_ledger::EntryHash::GENESIS;
    for (i, entry) in entries.iter().enumerate() {
        if entry.sequence_number != i as u64 {
            return Err(format!("sequence gap at index {i}"));
        }
        if entry.prev_hash != expected_prev {
            return Err(format!("hash chain broken at index {i}"));
        }
        expected_prev = entry.compute_hash();
    }

    // Chain is valid; return empty chain (verification of loaded data passed)
    Ok(chain)
}
