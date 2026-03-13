#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use tokio::sync::RwLock;

    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    use pe_core::{AminoAcidSequence, Embedding320, FitnessScore, FitnessWeights, ProteinVariant};

    use crate::router::build_router;
    use crate::state::AppState;

    // ── Local mocks ─────────────────────────────────────────────────

    // FitnessPredictor
    mockall::mock! {
        pub Scorer {}
        impl pe_neural::traits::FitnessPredictor for Scorer {
            fn predict(
                &self,
                variant: &ProteinVariant,
                embedding: &Embedding320,
            ) -> Result<FitnessScore, pe_neural::error::NeuralError>;
        }
    }

    // EmbeddingModel
    mockall::mock! {
        pub Embedder {}
        impl pe_vector::traits::EmbeddingModel for Embedder {
            fn embed(
                &self,
                sequence: &AminoAcidSequence,
            ) -> Result<Embedding320, pe_vector::error::VectorError>;
        }
    }

    // VectorStore
    mockall::mock! {
        pub Store {}
        impl pe_vector::traits::VectorStore for Store {
            fn insert(
                &mut self,
                id: uuid::Uuid,
                embedding: Embedding320,
                meta: pe_vector::meta::VariantMeta,
            ) -> Result<(), pe_vector::error::VectorError>;

            fn search_nearest(
                &self,
                query: &Embedding320,
                k: usize,
            ) -> Result<Vec<(uuid::Uuid, f32)>, pe_vector::error::VectorError>;

            fn get_meta(
                &self,
                id: uuid::Uuid,
            ) -> Result<Option<pe_vector::meta::VariantMeta>, pe_vector::error::VectorError>;

            fn count(&self) -> usize;
        }
    }

    // LedgerWriter
    mockall::mock! {
        pub Ledger {}
        impl pe_ledger::LedgerWriter for Ledger {
            fn append_entry(
                &mut self,
                entry_type: pe_ledger::EntryType,
                payload: Vec<u8>,
            ) -> Result<pe_ledger::EntryHash, pe_ledger::LedgerError>;
            fn verify_chain(&self) -> Result<bool, pe_ledger::LedgerError>;
            fn len(&self) -> usize;
        }
    }

    // SwarmCoordinator
    mockall::mock! {
        pub Coordinator {}
        #[async_trait::async_trait]
        impl pe_swarm::SwarmCoordinator for Coordinator {
            async fn run_design_cycle(
                &mut self,
                config: pe_swarm::CycleConfig,
            ) -> Result<pe_swarm::CycleResult, pe_swarm::SwarmError>;
        }
    }

    // ── helpers ──────────────────────────────────────────────────────

    fn fixture_embedding() -> Embedding320 {
        Embedding320::new([0.1f32; 320])
    }

    fn fixture_score() -> FitnessScore {
        let w = FitnessWeights::new(0.25, 0.25, 0.25, 0.25).unwrap();
        FitnessScore::new(0.8, 0.7, 0.9, 0.1, &w).unwrap()
    }

    fn build_test_state(
        scorer: MockScorer,
        embedder: MockEmbedder,
        store: MockStore,
        ledger: MockLedger,
        coordinator: MockCoordinator,
    ) -> AppState {
        AppState {
            scorer: Arc::new(scorer),
            embedder: Arc::new(embedder),
            store: Arc::new(RwLock::new(store)),
            ledger: Arc::new(RwLock::new(ledger)),
            coordinator: Arc::new(RwLock::new(coordinator)),
        }
    }

    async fn response_body(resp: axum::response::Response) -> String {
        let bytes = resp.into_body().collect().await.unwrap().to_bytes();
        String::from_utf8(bytes.to_vec()).unwrap()
    }

    // ── POST /api/variants/score returns 200 ────────────────────────

    #[tokio::test]
    async fn score_variant_returns_200_with_fitness_score() {
        let mut scorer = MockScorer::new();
        scorer.expect_predict().returning(|_, _| Ok(fixture_score()));

        let mut embedder = MockEmbedder::new();
        embedder
            .expect_embed()
            .returning(|_| Ok(fixture_embedding()));

        let store = MockStore::new();
        let ledger = MockLedger::new();
        let coordinator = MockCoordinator::new();

        let state = build_test_state(scorer, embedder, store, ledger, coordinator);
        let app = build_router(state);

        let body = serde_json::json!({
            "name": "test",
            "sequence": "ACDEFGHIKLMNPQRSTVWY",
            "target_factor": "OCT4"
        });

        let req = Request::builder()
            .method("POST")
            .uri("/api/variants/score")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_string(&body).unwrap()))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body_str = response_body(resp).await;
        let json: serde_json::Value = serde_json::from_str(&body_str).unwrap();
        assert!(json["composite"].as_f64().unwrap() > 0.0);
        assert!(json["reprogramming_efficiency"].as_f64().unwrap() > 0.0);
    }

    // ── POST /api/variants/score with invalid sequence returns 400 ──

    #[tokio::test]
    async fn score_variant_invalid_sequence_returns_400() {
        let scorer = MockScorer::new();
        let embedder = MockEmbedder::new();
        let store = MockStore::new();
        let ledger = MockLedger::new();
        let coordinator = MockCoordinator::new();

        let state = build_test_state(scorer, embedder, store, ledger, coordinator);
        let app = build_router(state);

        let body = serde_json::json!({
            "name": "bad",
            "sequence": "XXXINVALID123",
            "target_factor": "OCT4"
        });

        let req = Request::builder()
            .method("POST")
            .uri("/api/variants/score")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_string(&body).unwrap()))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    // ── GET /api/variants/:id returns 404 for unknown ───────────────

    #[tokio::test]
    async fn get_variant_unknown_id_returns_404() {
        let scorer = MockScorer::new();
        let embedder = MockEmbedder::new();
        let mut store = MockStore::new();
        store.expect_get_meta().returning(|_| Ok(None));

        let ledger = MockLedger::new();
        let coordinator = MockCoordinator::new();

        let state = build_test_state(scorer, embedder, store, ledger, coordinator);
        let app = build_router(state);

        let id = uuid::Uuid::new_v4();
        let req = Request::builder()
            .method("GET")
            .uri(&format!("/api/variants/{}", id))
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    // ── GET /api/variants/search returns k results ──────────────────

    #[tokio::test]
    async fn search_variants_returns_results() {
        let scorer = MockScorer::new();

        let mut embedder = MockEmbedder::new();
        embedder
            .expect_embed()
            .returning(|_| Ok(fixture_embedding()));

        let id1 = uuid::Uuid::new_v4();
        let id2 = uuid::Uuid::new_v4();
        let mut store = MockStore::new();
        store
            .expect_search_nearest()
            .returning(move |_, _| Ok(vec![(id1, 0.95), (id2, 0.87)]));

        let ledger = MockLedger::new();
        let coordinator = MockCoordinator::new();

        let state = build_test_state(scorer, embedder, store, ledger, coordinator);
        let app = build_router(state);

        let req = Request::builder()
            .method("GET")
            .uri("/api/variants/search?sequence=ACDEFGHIKLMNPQRSTVWY&k=2")
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body_str = response_body(resp).await;
        let json: Vec<serde_json::Value> = serde_json::from_str(&body_str).unwrap();
        assert_eq!(json.len(), 2);
    }

    // ── POST /api/evolution/cycle returns CycleResult ────────────────

    #[tokio::test]
    async fn run_cycle_returns_cycle_result() {
        let scorer = MockScorer::new();
        let embedder = MockEmbedder::new();
        let store = MockStore::new();
        let ledger = MockLedger::new();

        let mut coordinator = MockCoordinator::new();
        coordinator
            .expect_run_design_cycle()
            .returning(|config| {
                Ok(pe_swarm::CycleResult {
                    promoted: Vec::new(),
                    generation: config.generation,
                    variants_created: 10,
                    variants_scored: 10,
                    variants_validated: 8,
                    variants_screened: 7,
                })
            });

        let state = build_test_state(scorer, embedder, store, ledger, coordinator);
        let app = build_router(state);

        let body = serde_json::json!({
            "generation": 1,
            "population_size": 50,
            "mutation_rate": 0.1,
            "crossover_rate": 0.3,
            "quantum_enabled": false,
            "top_k": 10
        });

        let req = Request::builder()
            .method("POST")
            .uri("/api/evolution/cycle")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_string(&body).unwrap()))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body_str = response_body(resp).await;
        let json: serde_json::Value = serde_json::from_str(&body_str).unwrap();
        assert_eq!(json["generation"], 1);
        assert_eq!(json["variants_created"], 10);
    }

    // ── GET /api/ledger/verify returns valid ─────────────────────────

    #[tokio::test]
    async fn verify_ledger_returns_valid() {
        let scorer = MockScorer::new();
        let embedder = MockEmbedder::new();
        let store = MockStore::new();

        let mut ledger = MockLedger::new();
        ledger.expect_verify_chain().returning(|| Ok(true));

        let coordinator = MockCoordinator::new();

        let state = build_test_state(scorer, embedder, store, ledger, coordinator);
        let app = build_router(state);

        let req = Request::builder()
            .method("GET")
            .uri("/api/ledger/verify")
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body_str = response_body(resp).await;
        let json: serde_json::Value = serde_json::from_str(&body_str).unwrap();
        assert_eq!(json["valid"], true);
    }

    // ── GET /api/health returns 200 ─────────────────────────────────

    #[tokio::test]
    async fn health_returns_200() {
        let scorer = MockScorer::new();
        let embedder = MockEmbedder::new();
        let store = MockStore::new();
        let ledger = MockLedger::new();
        let coordinator = MockCoordinator::new();

        let state = build_test_state(scorer, embedder, store, ledger, coordinator);
        let app = build_router(state);

        let req = Request::builder()
            .method("GET")
            .uri("/api/health")
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body_str = response_body(resp).await;
        let json: serde_json::Value = serde_json::from_str(&body_str).unwrap();
        assert_eq!(json["status"], "ok");
    }

    // ── GET /api/ledger/entries returns count ────────────────────────

    #[tokio::test]
    async fn ledger_entries_returns_count() {
        let scorer = MockScorer::new();
        let embedder = MockEmbedder::new();
        let store = MockStore::new();

        let mut ledger = MockLedger::new();
        ledger.expect_len().returning(|| 42);

        let coordinator = MockCoordinator::new();

        let state = build_test_state(scorer, embedder, store, ledger, coordinator);
        let app = build_router(state);

        let req = Request::builder()
            .method("GET")
            .uri("/api/ledger/entries?limit=10&offset=0")
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body_str = response_body(resp).await;
        let json: serde_json::Value = serde_json::from_str(&body_str).unwrap();
        assert_eq!(json["entry_count"], 42);
    }
}
