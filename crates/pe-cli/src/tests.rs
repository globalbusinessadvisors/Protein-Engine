#[cfg(test)]
mod tests {
    use std::fs;
    use tempfile::TempDir;

    use crate::commands;
    use crate::wiring::{HashEmbedder, SignedLedger};
    use pe_vector::InMemoryVectorStore;
    use pe_vector::traits::{EmbeddingModel, VectorStore};
    use pe_vector::{DesignMethod, VariantMeta};
    use pe_core::{AminoAcidSequence, YamanakaFactor};

    fn test_sequence() -> &'static str {
        "ACDEFGHIKLMNPQRSTVWY"
    }

    // ── init creates a valid .rvf file ────────────────────────────────

    #[test]
    fn init_creates_valid_rvf() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.rvf");
        let path_str = path.to_str().unwrap();

        let result = commands::cmd_init(path_str);
        assert!(result.is_ok(), "init failed: {:?}", result.err());

        let output = result.unwrap();
        assert_eq!(output.path, path_str);
        assert!(!output.file_hash.is_empty());

        // Verify the file exists and is a valid RVF
        let data = fs::read(path_str).unwrap();
        assert!(!data.is_empty());

        let rvf = pe_rvf::RvfFile::deserialize(&data);
        assert!(rvf.is_ok(), "RVF deserialization failed: {:?}", rvf.err());
    }

    // ── score with valid sequence prints a composite score ────────────

    #[test]
    fn score_valid_sequence_returns_composite() {
        let result = commands::cmd_score(test_sequence());
        assert!(result.is_ok(), "score failed: {:?}", result.err());

        let output = result.unwrap();
        assert!(
            output.composite >= 0.0 && output.composite <= 1.0,
            "composite {} not in [0,1]",
            output.composite
        );
        assert!(output.reprogramming_efficiency >= 0.0);
        assert!(output.expression_stability >= 0.0);
        assert!(output.structural_plausibility >= 0.0);
    }

    // ── score with invalid sequence returns error ─────────────────────

    #[test]
    fn score_invalid_sequence_errors() {
        let result = commands::cmd_score("XXXINVALID123");
        assert!(result.is_err());
    }

    // ── evolve --generations 1 completes ──────────────────────────────

    #[test]
    fn evolve_one_generation_completes() {
        let result = commands::cmd_evolve(
            1,    // generations
            10,   // population_size
            test_sequence(),
            0.3,  // mutation_rate
            0.2,  // crossover_rate
            5,    // top_k
        );
        assert!(result.is_ok(), "evolve failed: {:?}", result.err());

        let summaries = result.unwrap();
        assert_eq!(summaries.len(), 1);

        let gen = &summaries[0];
        assert_eq!(gen.generation, 0);
        assert!(gen.variants_scored > 0);
        assert!(!gen.promoted.is_empty());
        assert!(gen.promoted.len() <= 5);
        assert!(gen.top_composite > 0.0);
    }

    // ── evolve multiple generations accumulates ───────────────────────

    #[test]
    fn evolve_three_generations() {
        let result = commands::cmd_evolve(3, 8, test_sequence(), 0.5, 0.3, 4);
        assert!(result.is_ok());

        let summaries = result.unwrap();
        assert_eq!(summaries.len(), 3);
        for (i, s) in summaries.iter().enumerate() {
            assert_eq!(s.generation, i as u32);
        }
    }

    // ── search on empty store returns empty ───────────────────────────

    #[test]
    fn search_empty_store_returns_empty() {
        let store = InMemoryVectorStore::new();
        let embedder = HashEmbedder;
        let result = commands::cmd_search(test_sequence(), 5, &store, &embedder);
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    // ── search with populated store returns results ───────────────────

    #[test]
    fn search_populated_store_returns_k_results() {
        let embedder = HashEmbedder;
        let mut store = InMemoryVectorStore::new();

        // Insert 3 variants
        let sequences = ["ACDEFGHIKLMNPQRSTVWY", "WYRSTVPQNMLKIHGFEDCA", "GHIKLMNPQRSTVWYACDEF"];
        for seq_str in &sequences {
            let seq = AminoAcidSequence::new(seq_str).unwrap();
            let emb = embedder.embed(&seq).unwrap();
            let id = uuid::Uuid::new_v4();
            let meta = VariantMeta {
                variant_id: id,
                target_factor: YamanakaFactor::OCT4,
                generation: 0,
                composite_score: None,
                design_method: DesignMethod::WildType,
            };
            store.insert(id, emb, meta).unwrap();
        }

        let result = commands::cmd_search(test_sequence(), 2, &store, &embedder);
        assert!(result.is_ok());
        let hits = result.unwrap();
        assert_eq!(hits.len(), 2);
        assert!(hits[0].similarity >= hits[1].similarity);
    }

    // ── quantum vqe on H2 molecule ───────────────────────────────────

    #[test]
    fn quantum_vqe_h2_returns_negative_energy() {
        let result = commands::cmd_quantum_vqe("h2");
        assert!(result.is_ok(), "VQE failed: {:?}", result.err());

        let output = result.unwrap();
        assert!(output.ground_state_energy < 0.0);
    }

    // ── ledger verify on fresh chain ─────────────────────────────────

    #[test]
    fn ledger_verify_fresh_returns_valid() {
        let ledger = SignedLedger::new();
        let result = commands::cmd_ledger_verify(&ledger);
        assert!(result.is_ok());

        let output = result.unwrap();
        assert!(output.valid);
        assert_eq!(output.entry_count, 0);
    }

    // ── ledger verify after appending entries ─────────────────────────

    #[test]
    fn ledger_verify_after_entries_valid() {
        use pe_ledger::{EntryType, LedgerWriter};

        let mut ledger = SignedLedger::new();
        ledger
            .append_entry(EntryType::VariantDesigned, b"test".to_vec())
            .unwrap();

        let result = commands::cmd_ledger_verify(&ledger);
        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output.valid);
        assert_eq!(output.entry_count, 1);
    }

    // ── ledger show on empty chain ────────────────────────────────────

    #[test]
    fn ledger_show_empty_returns_empty() {
        let ledger = SignedLedger::new();
        let entries = commands::cmd_ledger_show(&ledger, 10).unwrap();
        assert!(entries.is_empty());
    }

    // ── rvf build creates file ────────────────────────────────────────

    #[test]
    fn rvf_build_creates_file() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("output.rvf");
        let path_str = path.to_str().unwrap();

        let result = commands::cmd_rvf_build(path_str);
        assert!(result.is_ok(), "rvf build failed: {:?}", result.err());
        assert!(path.exists());
    }

    // ── rvf inspect shows correct segment count ───────────────────────

    #[test]
    fn rvf_inspect_shows_segments() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("inspect.rvf");
        let path_str = path.to_str().unwrap();

        // Build first
        commands::cmd_rvf_build(path_str).unwrap();

        // Then inspect
        let result = commands::cmd_rvf_inspect(path_str);
        assert!(result.is_ok(), "inspect failed: {:?}", result.err());

        let output = result.unwrap();
        assert_eq!(output.name, "protein-engine");
        // ManifestSeg + VecSeg + IndexSeg = 3
        assert_eq!(output.segment_count, 3);
        assert!(!output.file_hash.is_empty());
    }

    // ── init then inspect round-trip ──────────────────────────────────

    #[test]
    fn init_then_inspect_roundtrip() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("roundtrip.rvf");
        let path_str = path.to_str().unwrap();

        let init_result = commands::cmd_init(path_str).unwrap();
        let inspect_result = commands::cmd_rvf_inspect(path_str).unwrap();

        assert_eq!(init_result.file_hash, inspect_result.file_hash);
        assert_eq!(inspect_result.name, "protein-engine");
        assert_eq!(inspect_result.segment_count, 1); // init only has ManifestSeg
    }

    // ── score is deterministic ────────────────────────────────────────

    #[test]
    fn score_deterministic() {
        let s1 = commands::cmd_score(test_sequence()).unwrap();
        let s2 = commands::cmd_score(test_sequence()).unwrap();
        assert!(
            (s1.composite - s2.composite).abs() < 1e-10,
            "scores differ: {} vs {}",
            s1.composite,
            s2.composite
        );
    }

    // ── format output helpers ─────────────────────────────────────────

    #[test]
    fn json_output_is_parseable() {
        let score = commands::cmd_score(test_sequence()).unwrap();
        let json = crate::format::as_json(&score);
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert!(parsed["composite"].as_f64().is_some());
    }
}
