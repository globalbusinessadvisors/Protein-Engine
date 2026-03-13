//! Integration test: evolution engine + scoring + vector store + mock ledger.
//!
//! Real: SimpleEvolutionEngine, EnsemblePredictor, InMemoryVectorStore.
//! Mock: LedgerWriter (verify append_entry calls).

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use pe_core::{
    AminoAcidSequence, Embedding320, FitnessWeights, ProteinVariant, ScoredVariant, YamanakaFactor,
};
use pe_ledger::{EntryHash, EntryType, LedgerError, LedgerWriter};
use pe_neural::traits::FitnessPredictor;
use pe_neural::{EnsemblePredictor, LstmScorer, NBeatsScorer, TransformerScorer};
use pe_swarm::{EvolutionEngine, SimpleEvolutionEngine};
use pe_vector::traits::EmbeddingModel;
use pe_vector::VectorError;

// ── HashEmbedder ─────────────────────────────────────────────────────

struct HashEmbedder;

impl EmbeddingModel for HashEmbedder {
    fn embed(&self, sequence: &AminoAcidSequence) -> Result<Embedding320, VectorError> {
        let mut data = [0.0f32; 320];
        let residues = sequence.as_slice();
        for (i, &aa) in residues.iter().enumerate() {
            let aa_code = aa.to_char() as u32;
            let seed = aa_code.wrapping_mul(2654435761).wrapping_add(i as u32);
            for j in 0..4 {
                let idx = (i * 4 + j) % 320;
                let hash = seed.wrapping_mul((j as u32 + 1).wrapping_mul(0x9E3779B9));
                let val = (hash as f32 / u32::MAX as f32) * 2.0 - 1.0;
                data[idx] += val;
            }
        }
        let norm: f32 = data.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm > 1e-8 {
            for val in &mut data {
                *val /= norm;
            }
        }
        Ok(Embedding320::new(data))
    }
}

// ── Counting LedgerWriter ────────────────────────────────────────────

struct CountingLedger {
    count: Arc<AtomicUsize>,
}

impl CountingLedger {
    fn new(counter: Arc<AtomicUsize>) -> Self {
        Self { count: counter }
    }
}

impl LedgerWriter for CountingLedger {
    fn append_entry(
        &mut self,
        _entry_type: EntryType,
        _payload: Vec<u8>,
    ) -> Result<EntryHash, LedgerError> {
        self.count.fetch_add(1, Ordering::SeqCst);
        Ok(EntryHash([0u8; 32]))
    }

    fn verify_chain(&self) -> Result<bool, LedgerError> {
        Ok(true)
    }

    fn len(&self) -> usize {
        self.count.load(Ordering::SeqCst)
    }
}

// ── Helpers ──────────────────────────────────────────────────────────

const AMINO_ACIDS: &[u8] = b"ACDEFGHIKLMNPQRSTVWY";

fn make_sequence(seed: usize, len: usize) -> AminoAcidSequence {
    let chars: String = (0..len)
        .map(|i| {
            let idx = (seed.wrapping_mul(31).wrapping_add(i * 7)) % AMINO_ACIDS.len();
            AMINO_ACIDS[idx] as char
        })
        .collect();
    AminoAcidSequence::new(&chars).expect("valid sequence")
}

fn build_predictor() -> EnsemblePredictor<TransformerScorer, LstmScorer, NBeatsScorer> {
    EnsemblePredictor::new(
        TransformerScorer::new(0.0),
        LstmScorer::new(0.75),
        NBeatsScorer::new(0.7),
        FitnessWeights::default_weights(),
    )
}

fn seed_population(n: usize) -> Vec<ProteinVariant> {
    (0..n)
        .map(|i| {
            let seq = make_sequence(i, 30);
            ProteinVariant::wild_type(format!("seed-{i}"), seq, YamanakaFactor::OCT4)
        })
        .collect()
}

// ── Tests ────────────────────────────────────────────────────────────

#[test]
fn three_evolution_cycles_with_population_20() {
    let engine = SimpleEvolutionEngine::new();
    let embedder = HashEmbedder;
    let predictor = build_predictor();
    let append_count = Arc::new(AtomicUsize::new(0));
    let mut ledger = CountingLedger::new(append_count.clone());

    let mut population = seed_population(20);
    let top_k = 10;

    for gen in 0..3u32 {
        // Mutate each variant
        let mut new_variants: Vec<ProteinVariant> = Vec::new();
        for v in &population {
            if let Ok(mutated) = engine.mutate(v) {
                new_variants.push(mutated);
            }
        }
        // Also try some crossovers
        for i in 0..population.len() / 2 {
            if let Ok(child) = engine.crossover(&population[i], &population[population.len() - 1 - i]) {
                new_variants.push(child);
            }
        }
        assert!(!new_variants.is_empty(), "generation {gen} produced no variants");

        // Score
        let mut scored: Vec<ScoredVariant> = new_variants
            .iter()
            .map(|v| {
                let emb = embedder.embed(v.sequence()).expect("embed");
                let score = predictor.predict(v, &emb).expect("predict");
                ScoredVariant {
                    variant: v.clone(),
                    score,
                }
            })
            .collect();

        // Select top-k
        let selected = engine.select(&scored, top_k);
        assert!(
            selected.len() <= top_k,
            "select returned {} > top_k={}",
            selected.len(),
            top_k
        );

        // Verify selected are the best by composite score
        scored.sort_by(|a, b| {
            b.score
                .composite()
                .partial_cmp(&a.score.composite())
                .unwrap()
        });
        for (i, sv) in selected.iter().enumerate() {
            assert!(
                (sv.score.composite() - scored[i].score.composite()).abs() < 1e-10,
                "selected[{i}] composite mismatch"
            );
        }

        // Log to ledger
        let payload = format!("gen={gen},promoted={}", selected.len());
        ledger
            .append_entry(EntryType::CycleCompleted, payload.into_bytes())
            .expect("ledger append");

        // Promoted become next population
        population = selected.into_iter().map(|sv| sv.variant).collect();
    }

    assert_eq!(
        append_count.load(Ordering::SeqCst),
        3,
        "ledger should have 3 entries (one per cycle)"
    );
    assert!(
        !population.is_empty(),
        "population should not be empty after 3 cycles"
    );
}

#[test]
fn select_returns_sorted_by_composite_descending() {
    let engine = SimpleEvolutionEngine::new();
    let embedder = HashEmbedder;
    let predictor = build_predictor();

    let pop = seed_population(30);
    let scored: Vec<ScoredVariant> = pop
        .iter()
        .map(|v| {
            let emb = embedder.embed(v.sequence()).expect("embed");
            let score = predictor.predict(v, &emb).expect("predict");
            ScoredVariant {
                variant: v.clone(),
                score,
            }
        })
        .collect();

    let top = engine.select(&scored, 5);
    assert_eq!(top.len(), 5);

    for window in top.windows(2) {
        assert!(
            window[0].score.composite() >= window[1].score.composite(),
            "selection not sorted: {} < {}",
            window[0].score.composite(),
            window[1].score.composite()
        );
    }
}
