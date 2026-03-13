//! Integration test: RVF assembly with real segment producers from all crates.
//!
//! Builds a complete .rvf file, verifies manifest, segments, and deterministic hash.

use chrono::Utc;

use pe_core::{
    AminoAcidSequence, FitnessWeights, ProteinVariant, ScoredVariant, YamanakaFactor,
};
use pe_ledger::{
    EntryType, JournalChain, JournalSegProducer, MlDsaSigner, WitnessSegProducer,
};
use pe_neural::{
    EnsemblePredictor, LstmScorer, ModelWeights, NBeatsScorer, QuantSegProducer,
    TransformerScorer,
};
use pe_quantum::{SketchSegProducer, VqeSnapshotCache};
use pe_quantum_wasm::{MolecularHamiltonian, VqeConfig, VqeRunner};
use pe_rvf::segment::SegmentType;
use pe_rvf::traits::{RvfAssembler, SegmentProducer};
use pe_rvf::{Manifest, RvfBuilder, RvfFile};
use pe_swarm::HotSegProducer;
use pe_vector::traits::{EmbeddingModel, VectorStore};
use pe_vector::{DesignMethod, InMemoryVectorStore, VariantMeta, VectorError};

use pe_core::Embedding320;

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

// ── Helpers ──────────────────────────────────────────────────────────

/// Create a builder with manifest and ManifestSeg pre-added.
fn builder_with_manifest(manifest: Manifest) -> RvfBuilder {
    let manifest_bytes = serde_json::to_vec(&manifest).expect("serialize manifest");
    let mut builder = RvfBuilder::new();
    builder.set_manifest(manifest);
    builder
        .add_segment(SegmentType::ManifestSeg, manifest_bytes)
        .expect("add manifest seg");
    builder
}

fn make_scored_variants(n: usize) -> Vec<ScoredVariant> {
    let embedder = HashEmbedder;
    let predictor = EnsemblePredictor::new(
        TransformerScorer::new(0.0),
        LstmScorer::new(0.75),
        NBeatsScorer::new(0.7),
        FitnessWeights::default_weights(),
    );

    (0..n)
        .map(|i| {
            let chars: String = (0..30)
                .map(|j| {
                    let idx = (i.wrapping_mul(31).wrapping_add(j * 7)) % 20;
                    b"ACDEFGHIKLMNPQRSTVWY"[idx] as char
                })
                .collect();
            let seq = AminoAcidSequence::new(&chars).expect("valid");
            let v = ProteinVariant::wild_type(format!("v-{i}"), seq, YamanakaFactor::OCT4);
            let emb = embedder.embed(v.sequence()).expect("embed");
            let score = pe_neural::traits::FitnessPredictor::predict(&predictor, &v, &emb)
                .expect("predict");
            ScoredVariant {
                variant: v,
                score,
            }
        })
        .collect()
}

// ── Tests ────────────────────────────────────────────────────────────

#[test]
fn build_complete_rvf_with_all_segments() {
    let signer = MlDsaSigner::generate();
    let embedder = HashEmbedder;

    // VEC_SEG + INDEX_SEG: vector store with 10 variants
    let mut store = InMemoryVectorStore::new();
    for i in 0..10 {
        let chars: String = (0..30)
            .map(|j| {
                let idx = (i * 31 + j * 7) % 20;
                b"ACDEFGHIKLMNPQRSTVWY"[idx] as char
            })
            .collect();
        let seq = AminoAcidSequence::new(&chars).expect("valid");
        let v = ProteinVariant::wild_type(format!("v-{i}"), seq, YamanakaFactor::OCT4);
        let emb = embedder.embed(v.sequence()).expect("embed");
        let meta = VariantMeta {
            variant_id: v.id(),
            target_factor: YamanakaFactor::OCT4,
            generation: 0,
            composite_score: None,
            design_method: DesignMethod::WildType,
        };
        store.insert(v.id(), emb, meta).expect("insert");
    }
    let vec_seg = store.to_vec_seg();
    let idx_seg = store.to_index_seg();

    // JOURNAL_SEG + WITNESS_SEG: ledger with 5 entries
    let mut chain = JournalChain::new();
    for i in 0..5 {
        chain
            .append_entry(EntryType::CycleCompleted, format!("c-{i}").into_bytes(), &signer)
            .expect("append");
    }
    let journal_producer = JournalSegProducer::new(chain.entries().to_vec());
    let journal_bytes = journal_producer.produce().expect("journal seg");
    let witness_producer = WitnessSegProducer::new(chain.entries().to_vec());
    let witness_bytes = witness_producer.produce().expect("witness seg");

    // QUANT_SEG: model weights
    let weights = ModelWeights {
        transformer: TransformerScorer::new(0.0),
        lstm: LstmScorer::new(0.75),
        nbeats: NBeatsScorer::new(0.7),
    };
    let quant_producer = QuantSegProducer::new(weights);
    let quant_bytes = quant_producer.produce().expect("quant seg");

    // HOT_SEG: top candidates
    let scored = make_scored_variants(10);
    let hot_producer = HotSegProducer::new(scored);
    let hot_bytes = hot_producer.produce().expect("hot seg");

    // SKETCH_SEG: VQE snapshot
    let vqe_runner = VqeRunner::new(VqeConfig::default());
    let vqe_result = vqe_runner
        .run(&MolecularHamiltonian::h2_molecule())
        .expect("vqe");
    let mut cache = VqeSnapshotCache::new();
    cache.add("H2".into(), vqe_result);
    let sketch_producer = SketchSegProducer::new(cache);
    let sketch_bytes = sketch_producer.produce().expect("sketch seg");

    // Build RVF
    let now = Utc::now();
    let manifest = Manifest::new(
        "protein-engine".into(),
        "0.1.0".into(),
        None,
        None,
        now,
    )
    .expect("manifest");

    let mut builder = builder_with_manifest(manifest);
    builder
        .add_segment(SegmentType::VecSeg, vec_seg)
        .expect("add vec");
    builder
        .add_segment(SegmentType::IndexSeg, idx_seg)
        .expect("add idx");
    builder
        .add_segment(SegmentType::JournalSeg, journal_bytes)
        .expect("add journal");
    builder
        .add_segment(SegmentType::WitnessSeg, witness_bytes)
        .expect("add witness");
    builder
        .add_segment(SegmentType::QuantSeg, quant_bytes)
        .expect("add quant");
    builder
        .add_segment(SegmentType::HotSeg, hot_bytes)
        .expect("add hot");
    builder
        .add_segment(SegmentType::SketchSeg, sketch_bytes)
        .expect("add sketch");

    let rvf = builder.build().expect("build RVF");

    // Verify manifest
    assert_eq!(rvf.manifest().name, "protein-engine");
    assert_eq!(rvf.manifest().version, "0.1.0");
    assert!(rvf.manifest().parent_hash.is_none());

    // Verify all segments present
    let segments = rvf.segments();
    assert!(segments.contains_key(&SegmentType::VecSeg));
    assert!(segments.contains_key(&SegmentType::IndexSeg));
    assert!(segments.contains_key(&SegmentType::JournalSeg));
    assert!(segments.contains_key(&SegmentType::WitnessSeg));
    assert!(segments.contains_key(&SegmentType::QuantSeg));
    assert!(segments.contains_key(&SegmentType::HotSeg));
    assert!(segments.contains_key(&SegmentType::SketchSeg));
}

#[test]
fn rvf_serialize_deserialize_round_trip() {
    let now = Utc::now();
    let manifest = Manifest::new(
        "test-roundtrip".into(),
        "1.0.0".into(),
        None,
        None,
        now,
    )
    .expect("manifest");

    let mut builder = builder_with_manifest(manifest);
    builder
        .add_segment(SegmentType::HotSeg, b"test-data".to_vec())
        .expect("add");

    let rvf = builder.build().expect("build");
    let bytes = rvf.serialize();

    let restored = RvfFile::deserialize(&bytes).expect("deserialize");
    assert_eq!(rvf, restored);
    assert_eq!(rvf.file_hash(), restored.file_hash());
}

#[test]
fn rvf_file_hash_is_deterministic() {
    let now = Utc::now();

    let build = || {
        let manifest = Manifest::new(
            "deterministic".into(),
            "0.1.0".into(),
            None,
            None,
            now,
        )
        .expect("manifest");
        let mut b = builder_with_manifest(manifest);
        b.add_segment(SegmentType::HotSeg, b"same-data".to_vec())
            .expect("add");
        b.build().expect("build")
    };

    let rvf1 = build();
    let rvf2 = build();
    assert_eq!(
        rvf1.file_hash(),
        rvf2.file_hash(),
        "same inputs must produce same hash"
    );
}

#[test]
fn rvf_parent_hash_lineage() {
    let now = Utc::now();

    // Build first RVF (no parent)
    let manifest1 = Manifest::new("lineage".into(), "0.1.0".into(), None, None, now)
        .expect("manifest");
    let mut b1 = builder_with_manifest(manifest1);
    b1.add_segment(SegmentType::HotSeg, b"gen-0".to_vec())
        .expect("add");
    let rvf1 = b1.build().expect("build");

    // Build second RVF with parent hash pointing to first
    let manifest2 = Manifest::new(
        "lineage".into(),
        "0.2.0".into(),
        Some(*rvf1.file_hash()),
        None,
        now,
    )
    .expect("manifest2");
    let mut b2 = builder_with_manifest(manifest2);
    b2.add_segment(SegmentType::HotSeg, b"gen-1".to_vec())
        .expect("add");
    let rvf2 = b2.build().expect("build");

    assert_eq!(
        rvf2.manifest().parent_hash.as_ref().unwrap(),
        rvf1.file_hash(),
        "child must reference parent hash"
    );
    assert_ne!(rvf1.file_hash(), rvf2.file_hash());
}

#[test]
fn segments_data_integrity() {
    let scored = make_scored_variants(5);
    let hot_producer = HotSegProducer::new(scored.clone());
    let hot_bytes = hot_producer.produce().expect("produce");

    // Deserialize HOT_SEG back
    let restored: Vec<ScoredVariant> =
        serde_json::from_slice(&hot_bytes).expect("deserialize hot seg");
    assert_eq!(restored.len(), 5);

    for (orig, rest) in scored.iter().zip(restored.iter()) {
        assert_eq!(orig.variant.id(), rest.variant.id());
        assert!(
            (orig.score.composite() - rest.score.composite()).abs() < 1e-10
        );
    }
}
