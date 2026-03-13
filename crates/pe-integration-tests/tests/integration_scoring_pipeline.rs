//! Integration test: pe-core → pe-vector → pe-neural scoring pipeline.
//!
//! Wires real InMemoryVectorStore + real EnsemblePredictor (stub scorers)
//! with a deterministic HashEmbedder. No mocks.

use pe_core::{
    AminoAcidSequence, Embedding320, FitnessWeights, ProteinVariant, ScoredVariant, YamanakaFactor,
};
use pe_neural::traits::FitnessPredictor;
use pe_neural::{EnsemblePredictor, LstmScorer, NBeatsScorer, TransformerScorer};
use pe_vector::traits::{EmbeddingModel, VectorStore};
use pe_vector::{DesignMethod, InMemoryVectorStore, VariantMeta, VectorError};

// ── Deterministic hash-based embedder (same algorithm as pe-cli) ─────

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

// ── Helpers ──────────────────────────────────────────────────────────

fn build_predictor() -> EnsemblePredictor<TransformerScorer, LstmScorer, NBeatsScorer> {
    EnsemblePredictor::new(
        TransformerScorer::new(0.0),
        LstmScorer::new(0.75),
        NBeatsScorer::new(0.7),
        FitnessWeights::default_weights(),
    )
}

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

fn factors() -> [YamanakaFactor; 4] {
    [
        YamanakaFactor::OCT4,
        YamanakaFactor::SOX2,
        YamanakaFactor::KLF4,
        YamanakaFactor::CMYC,
    ]
}

// ── Tests ────────────────────────────────────────────────────────────

#[test]
fn create_50_variants_embed_store_and_search() {
    let embedder = HashEmbedder;
    let predictor = build_predictor();
    let mut store = InMemoryVectorStore::new();

    let mut variants = Vec::new();
    let mut scored = Vec::new();

    // Create 50 variants with different sequences
    for i in 0..50 {
        let seq = make_sequence(i, 30 + i % 20);
        let factor = factors()[i % 4].clone();
        let v = ProteinVariant::wild_type(format!("variant-{i}"), seq, factor);
        variants.push(v);
    }

    // Embed and store all
    for v in &variants {
        let emb = embedder.embed(v.sequence()).expect("embed");
        let meta = VariantMeta {
            variant_id: v.id(),
            target_factor: v.target_factor().clone(),
            generation: v.generation(),
            composite_score: None,
            design_method: DesignMethod::WildType,
        };
        store.insert(v.id(), emb, meta).expect("insert");
    }

    assert_eq!(store.count(), 50);

    // Score all variants
    for v in &variants {
        let emb = embedder.embed(v.sequence()).expect("embed");
        let score = predictor.predict(v, &emb).expect("predict");

        // All fitness components must be in [0, 1]
        assert!(score.reprogramming_efficiency() >= 0.0 && score.reprogramming_efficiency() <= 1.0);
        assert!(score.expression_stability() >= 0.0 && score.expression_stability() <= 1.0);
        assert!(score.structural_plausibility() >= 0.0 && score.structural_plausibility() <= 1.0);
        assert!(score.safety_score() >= 0.0 && score.safety_score() <= 1.0);
        assert!(score.composite() >= 0.0 && score.composite() <= 1.0);

        scored.push(ScoredVariant {
            variant: v.clone(),
            score,
        });
    }

    assert_eq!(scored.len(), 50);
}

#[test]
fn search_nearest_returns_sorted_by_similarity() {
    let embedder = HashEmbedder;
    let mut store = InMemoryVectorStore::new();

    for i in 0..20 {
        let seq = make_sequence(i, 30);
        let factor = factors()[i % 4].clone();
        let v = ProteinVariant::wild_type(format!("v-{i}"), seq, factor);
        let emb = embedder.embed(v.sequence()).expect("embed");
        let meta = VariantMeta {
            variant_id: v.id(),
            target_factor: v.target_factor().clone(),
            generation: 0,
            composite_score: None,
            design_method: DesignMethod::WildType,
        };
        store.insert(v.id(), emb, meta).expect("insert");
    }

    // Query with a known variant's embedding
    let query_seq = make_sequence(0, 30);
    let query_emb = embedder
        .embed(&query_seq)
        .expect("embed query");

    let results = store.search_nearest(&query_emb, 5).expect("search");

    assert_eq!(results.len(), 5);

    // Must be descending by similarity
    for window in results.windows(2) {
        assert!(
            window[0].1 >= window[1].1,
            "results not sorted: {} < {}",
            window[0].1,
            window[1].1
        );
    }

    // Top result should be exact match (similarity ≈ 1.0)
    assert!(
        results[0].1 > 0.99,
        "top result similarity {:.4} should be near 1.0",
        results[0].1
    );
}

#[test]
fn embedder_is_deterministic() {
    let embedder = HashEmbedder;
    let seq = AminoAcidSequence::new("MAGHLASDFAF").expect("valid");
    let e1 = embedder.embed(&seq).expect("embed 1");
    let e2 = embedder.embed(&seq).expect("embed 2");
    assert_eq!(e1, e2, "same sequence must produce same embedding");
}

#[test]
fn metadata_survives_round_trip() {
    let embedder = HashEmbedder;
    let mut store = InMemoryVectorStore::new();

    let seq = make_sequence(42, 30);
    let v = ProteinVariant::wild_type("meta-test", seq, YamanakaFactor::OCT4);
    let emb = embedder.embed(v.sequence()).expect("embed");
    let meta = VariantMeta {
        variant_id: v.id(),
        target_factor: YamanakaFactor::OCT4,
        generation: 7,
        composite_score: Some(0.85),
        design_method: DesignMethod::Mutation,
    };
    store.insert(v.id(), emb, meta.clone()).expect("insert");

    let retrieved = store.get_meta(v.id()).expect("get_meta").expect("found");
    assert_eq!(retrieved, meta);
}

#[test]
fn vec_seg_round_trip_preserves_search_results() {
    let embedder = HashEmbedder;
    let mut store = InMemoryVectorStore::new();

    for i in 0..10 {
        let seq = make_sequence(i, 30);
        let v = ProteinVariant::wild_type(format!("rt-{i}"), seq, YamanakaFactor::SOX2);
        let emb = embedder.embed(v.sequence()).expect("embed");
        let meta = VariantMeta {
            variant_id: v.id(),
            target_factor: YamanakaFactor::SOX2,
            generation: 0,
            composite_score: None,
            design_method: DesignMethod::WildType,
        };
        store.insert(v.id(), emb, meta).expect("insert");
    }

    // Serialize and reconstruct
    let vec_seg = store.to_vec_seg();
    let idx_seg = store.to_index_seg();
    let restored = InMemoryVectorStore::from_segments(&vec_seg, &idx_seg).expect("restore");

    assert_eq!(restored.count(), 10);

    // Same query should return same results
    let query_emb = embedder
        .embed(&make_sequence(0, 30))
        .expect("embed");
    let orig = store.search_nearest(&query_emb, 3).expect("orig search");
    let rest = restored.search_nearest(&query_emb, 3).expect("restored search");
    assert_eq!(orig.len(), rest.len());
    for (o, r) in orig.iter().zip(rest.iter()) {
        assert_eq!(o.0, r.0, "same IDs");
        assert!((o.1 - r.1).abs() < 1e-6, "same similarities");
    }
}
