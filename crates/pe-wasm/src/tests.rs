#[cfg(test)]
mod tests {
    use crate::engine::WasmEngine;

    fn test_sequence() -> &'static str {
        "ACDEFGHIKLMNPQRSTVWY"
    }

    // ── score_sequence with valid sequence returns parseable JSON with composite in [0, 1] ──

    #[test]
    fn score_sequence_valid_returns_composite_in_range() {
        let engine = WasmEngine::new();
        let result = engine.score_sequence(test_sequence());
        assert!(result.is_ok(), "expected Ok, got: {:?}", result.err());

        let output = result.unwrap();
        assert!(
            output.composite >= 0.0 && output.composite <= 1.0,
            "composite {} not in [0, 1]",
            output.composite
        );
        assert!(output.reprogramming_efficiency >= 0.0);
        assert!(output.expression_stability >= 0.0);
        assert!(output.structural_plausibility >= 0.0);
        assert!(output.safety_score >= 0.0);
    }

    // ── score_sequence serializes to valid JSON ──

    #[test]
    fn score_sequence_produces_valid_json() {
        let engine = WasmEngine::new();
        let output = engine.score_sequence(test_sequence()).unwrap();

        let json = serde_json::to_string(&output).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert!(parsed["composite"].as_f64().is_some());
    }

    // ── score_sequence with invalid sequence returns error ──

    #[test]
    fn score_sequence_invalid_returns_error() {
        let engine = WasmEngine::new();
        let result = engine.score_sequence("XXXINVALID123");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(!err.is_empty());
    }

    // ── score_sequence with empty sequence returns error ──

    #[test]
    fn score_sequence_empty_returns_error() {
        let engine = WasmEngine::new();
        let result = engine.score_sequence("");
        assert!(result.is_err());
    }

    // ── run_evolution_step with small population returns next generation ──

    #[test]
    fn evolution_step_returns_promoted_variants() {
        let mut engine = WasmEngine::new();

        let population = serde_json::json!([
            {"name": "v1", "sequence": "ACDEFGHIKLMNPQRSTVWY", "target_factor": "OCT4"},
            {"name": "v2", "sequence": "WYRSTVPQNMLKIHGFEDCA", "target_factor": "OCT4"},
            {"name": "v3", "sequence": "GHIKLMNPQRSTVWYACDEF", "target_factor": "OCT4"}
        ]);

        let config = serde_json::json!({
            "generation": 1,
            "population_size": 10,
            "mutation_rate": 0.5,
            "crossover_rate": 0.5,
            "top_k": 3
        });

        let result = engine.run_evolution_step(
            &population.to_string(),
            &config.to_string(),
        );

        assert!(result.is_ok(), "evolution step failed: {:?}", result.err());
        let output = result.unwrap();
        assert_eq!(output.generation, 1);
        assert!(!output.promoted.is_empty());
        assert!(output.promoted.len() <= 3);
        assert!(output.variants_scored > 0);

        // All promoted variants should have valid composite scores
        for pv in &output.promoted {
            assert!(pv.composite >= 0.0 && pv.composite <= 1.0);
        }
    }

    // ── run_evolution_step with empty population returns error ──

    #[test]
    fn evolution_step_empty_population_errors() {
        let mut engine = WasmEngine::new();
        let result = engine.run_evolution_step("[]", r#"{"generation":1}"#);
        assert!(result.is_err());
    }

    // ── run_evolution_step with invalid JSON returns error ──

    #[test]
    fn evolution_step_invalid_json_errors() {
        let mut engine = WasmEngine::new();
        let result = engine.run_evolution_step("not json", r#"{"generation":1}"#);
        assert!(result.is_err());
    }

    // ── run_quantum_sim returns VQE result ──

    #[test]
    fn quantum_sim_h2_molecule() {
        let engine = WasmEngine::new();

        let h2 = pe_quantum_wasm::MolecularHamiltonian::h2_molecule();
        let h2_json = serde_json::to_string(&h2).unwrap();

        let result = engine.run_quantum_sim(&h2_json);
        assert!(result.is_ok(), "VQE failed: {:?}", result.err());

        let vqe = result.unwrap();
        // H2 ground state energy ≈ −1.137 Hartree
        assert!(
            vqe.ground_state_energy < 0.0,
            "expected negative energy, got {}",
            vqe.ground_state_energy
        );
    }

    // ── search_similar returns k results ──

    #[test]
    fn search_similar_returns_k_results() {
        let mut engine = WasmEngine::new();

        // Populate the store with some variants
        engine.insert_variant("v1", "ACDEFGHIKLMNPQRSTVWY", "OCT4").unwrap();
        engine.insert_variant("v2", "WYRSTVPQNMLKIHGFEDCA", "SOX2").unwrap();
        engine.insert_variant("v3", "GHIKLMNPQRSTVWYACDEF", "KLF4").unwrap();

        let result = engine.search_similar("ACDEFGHIKLMNPQRSTVWY", 2);
        assert!(result.is_ok(), "search failed: {:?}", result.err());

        let hits = result.unwrap();
        assert_eq!(hits.len(), 2);
        // First result should be most similar
        assert!(hits[0].similarity >= hits[1].similarity);
    }

    // ── search_similar with empty store returns empty ──

    #[test]
    fn search_similar_empty_store_returns_empty() {
        let engine = WasmEngine::new();
        let result = engine.search_similar("ACDEFGHIKLMNPQRSTVWY", 5);
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    // ── verify_ledger returns {valid: true} on empty chain ──

    #[test]
    fn verify_ledger_empty_chain_valid() {
        let engine = WasmEngine::new();
        let result = engine.verify_ledger();
        assert!(result.is_ok());
        assert!(result.unwrap().valid);
    }

    // ── WasmEngine::new() initializes with empty state ──

    #[test]
    fn engine_initializes_empty() {
        let engine = WasmEngine::new();
        assert_eq!(engine.vector_count(), 0);
        assert_eq!(engine.journal_len(), 0);
    }

    // ── insert_variant adds to store ──

    #[test]
    fn insert_variant_increases_count() {
        let mut engine = WasmEngine::new();
        assert_eq!(engine.vector_count(), 0);

        engine.insert_variant("v1", "ACDEFGHIKLMNPQRSTVWY", "OCT4").unwrap();
        assert_eq!(engine.vector_count(), 1);

        engine.insert_variant("v2", "WYRSTVPQNMLKIHGFEDCA", "SOX2").unwrap();
        assert_eq!(engine.vector_count(), 2);
    }

    // ── quantum_sim with invalid JSON returns error ──

    #[test]
    fn quantum_sim_invalid_json_errors() {
        let engine = WasmEngine::new();
        let result = engine.run_quantum_sim("not json");
        assert!(result.is_err());
    }

    // ── score determinism: same sequence → same score ──

    #[test]
    fn score_deterministic() {
        let engine = WasmEngine::new();
        let s1 = engine.score_sequence(test_sequence()).unwrap();
        let s2 = engine.score_sequence(test_sequence()).unwrap();
        assert!(
            (s1.composite - s2.composite).abs() < 1e-10,
            "scores differ: {} vs {}",
            s1.composite,
            s2.composite
        );
    }
}
