#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use pe_quantum::traits::QuantumBackend;
    use pe_quantum::{MolecularHamiltonian, ProviderName, QuboInstance};

    use crate::bridge::ChemiqBridge;
    use crate::error::ChemistryError;
    use crate::http_client::{HttpResponse, MockHttpClient};

    // ── helpers ──────────────────────────────────────────────────────────

    fn ok_response(body: &str) -> HttpResponse {
        HttpResponse {
            status: 200,
            body: body.to_string(),
        }
    }

    fn err_response(status: u16, body: &str) -> HttpResponse {
        HttpResponse {
            status,
            body: body.to_string(),
        }
    }

    fn make_bridge(mock: MockHttpClient) -> ChemiqBridge<MockHttpClient> {
        ChemiqBridge::new(Arc::new(mock), "http://localhost:8100".to_string())
    }

    // ── VQE tests ───────────────────────────────────────────────────────

    #[tokio::test]
    async fn submit_vqe_translates_request_and_response() {
        let mut mock = MockHttpClient::new();
        mock.expect_post()
            .withf(|url, body| {
                url == "http://localhost:8100/vqe"
                    && body.contains("\"basis_set\":\"sto-3g\"")
                    && body.contains("\"ansatz\":\"UCCSD\"")
            })
            .returning(|_, _| {
                let resp = ok_response(
                    r#"{"energy": -1.137, "parameters": [0.1, 0.2], "iterations": 42, "converged": true}"#,
                );
                Box::pin(async move { Ok(resp) })
            });

        let bridge = make_bridge(mock);
        let ham = MolecularHamiltonian::h2_molecule();
        let result = bridge.submit_vqe(ham).await.unwrap();

        assert!((result.ground_state_energy - (-1.137)).abs() < 1e-10);
        assert_eq!(result.optimal_parameters, vec![0.1, 0.2]);
        assert_eq!(result.iterations, 42);
        assert!(result.converged);
    }

    #[tokio::test]
    async fn submit_vqe_propagates_sidecar_error() {
        let mut mock = MockHttpClient::new();
        mock.expect_post().returning(|_, _| {
            let resp = err_response(500, "internal error");
            Box::pin(async move { Ok(resp) })
        });

        let bridge = make_bridge(mock);
        let ham = MolecularHamiltonian::h2_molecule();
        let result = bridge.submit_vqe(ham).await;

        assert!(result.is_err());
        let err_str = result.unwrap_err().to_string();
        assert!(err_str.contains("500") || err_str.contains("internal error"));
    }

    #[tokio::test]
    async fn submit_vqe_propagates_http_error() {
        let mut mock = MockHttpClient::new();
        mock.expect_post().returning(|_, _| {
            Box::pin(async move {
                Err(ChemistryError::HttpError("connection refused".into()))
            })
        });

        let bridge = make_bridge(mock);
        let ham = MolecularHamiltonian::h2_molecule();
        let result = bridge.submit_vqe(ham).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn submit_vqe_propagates_parse_error() {
        let mut mock = MockHttpClient::new();
        mock.expect_post().returning(|_, _| {
            let resp = ok_response("not json");
            Box::pin(async move { Ok(resp) })
        });

        let bridge = make_bridge(mock);
        let ham = MolecularHamiltonian::h2_molecule();
        let result = bridge.submit_vqe(ham).await;

        assert!(result.is_err());
    }

    // ── QAOA tests ──────────────────────────────────────────────────────

    #[tokio::test]
    async fn submit_qaoa_translates_request_and_response() {
        let mut mock = MockHttpClient::new();
        mock.expect_post()
            .withf(|url, _| url == "http://localhost:8100/qaoa")
            .returning(|_, _| {
                let resp = ok_response(
                    r#"{"solution": [1, 0, 1], "cost": -3.5, "iterations": 10}"#,
                );
                Box::pin(async move { Ok(resp) })
            });

        let bridge = make_bridge(mock);
        let qubo = QuboInstance::new(vec![
            vec![-1.0, 0.5, 0.0],
            vec![0.5, -2.0, 0.5],
            vec![0.0, 0.5, -1.0],
        ])
        .unwrap();

        let result = bridge.submit_qaoa(qubo).await.unwrap();
        // solution [1, 0, 1] → bitstring 0b101 = 5
        assert_eq!(result.best_bitstring, 5);
        assert!((result.best_cost - (-3.5)).abs() < 1e-10);
        assert_eq!(result.iterations, 10);
    }

    #[tokio::test]
    async fn submit_qaoa_propagates_sidecar_error() {
        let mut mock = MockHttpClient::new();
        mock.expect_post().returning(|_, _| {
            let resp = err_response(422, "bad qubo");
            Box::pin(async move { Ok(resp) })
        });

        let bridge = make_bridge(mock);
        let qubo = QuboInstance::new(vec![vec![-1.0, 0.0], vec![0.0, -1.0]]).unwrap();
        let result = bridge.submit_qaoa(qubo).await;

        assert!(result.is_err());
    }

    // ── Health check tests ──────────────────────────────────────────────

    #[tokio::test]
    async fn is_reachable_returns_true_on_healthy_sidecar() {
        let mut mock = MockHttpClient::new();
        mock.expect_get()
            .withf(|url| url == "http://localhost:8100/health")
            .returning(|_| {
                let resp = ok_response(r#"{"status": "ok", "backend": "origin_quantum"}"#);
                Box::pin(async move { Ok(resp) })
            });

        let bridge = make_bridge(mock);
        assert!(bridge.is_reachable().await);
    }

    #[tokio::test]
    async fn is_reachable_returns_false_on_http_error() {
        let mut mock = MockHttpClient::new();
        mock.expect_get().returning(|_| {
            Box::pin(async move { Err(ChemistryError::HttpError("refused".into())) })
        });

        let bridge = make_bridge(mock);
        assert!(!bridge.is_reachable().await);
    }

    #[tokio::test]
    async fn is_reachable_returns_false_on_bad_status() {
        let mut mock = MockHttpClient::new();
        mock.expect_get().returning(|_| {
            let resp = err_response(503, "unavailable");
            Box::pin(async move { Ok(resp) })
        });

        let bridge = make_bridge(mock);
        assert!(!bridge.is_reachable().await);
    }

    #[tokio::test]
    async fn is_reachable_returns_false_on_not_ok_status_field() {
        let mut mock = MockHttpClient::new();
        mock.expect_get().returning(|_| {
            let resp =
                ok_response(r#"{"status": "degraded", "backend": "origin_quantum"}"#);
            Box::pin(async move { Ok(resp) })
        });

        let bridge = make_bridge(mock);
        assert!(!bridge.is_reachable().await);
    }

    #[tokio::test]
    async fn health_check_caches_result() {
        let mut mock = MockHttpClient::new();
        // Only one call expected — the second is_reachable should use the cache
        mock.expect_get()
            .times(1)
            .returning(|_| {
                let resp = ok_response(r#"{"status": "ok", "backend": "origin_quantum"}"#);
                Box::pin(async move { Ok(resp) })
            });

        let bridge = make_bridge(mock);
        assert!(bridge.is_reachable().await);
        assert!(bridge.is_reachable().await); // should use cache
    }

    // ── Capabilities test ───────────────────────────────────────────────

    #[tokio::test]
    async fn capabilities_returns_origin_quantum_defaults() {
        let mock = MockHttpClient::new();
        let bridge = make_bridge(mock);
        let caps = bridge.capabilities();

        assert_eq!(caps.max_qubits, 72);
        assert_eq!(caps.provider, ProviderName::OriginQuantum);
        assert!(!caps.is_simulator);
    }

    // ── ACL translation tests ───────────────────────────────────────────

    #[tokio::test]
    async fn vqe_request_contains_molecule_description() {
        let mut mock = MockHttpClient::new();
        mock.expect_post()
            .withf(|_, body| {
                // H2 molecule: 2 qubits, 6 terms → "2q-6t"
                body.contains("\"molecule\":\"2q-6t\"")
            })
            .returning(|_, _| {
                let resp = ok_response(
                    r#"{"energy": -1.0, "parameters": [], "iterations": 1, "converged": true}"#,
                );
                Box::pin(async move { Ok(resp) })
            });

        let bridge = make_bridge(mock);
        let ham = MolecularHamiltonian::h2_molecule();
        let _ = bridge.submit_vqe(ham).await;
    }

    #[tokio::test]
    async fn qaoa_request_contains_qubo_matrix() {
        let mut mock = MockHttpClient::new();
        mock.expect_post()
            .withf(|_, body| body.contains("qubo_matrix") && body.contains("p_layers"))
            .returning(|_, _| {
                let resp =
                    ok_response(r#"{"solution": [0, 1], "cost": -2.0, "iterations": 5}"#);
                Box::pin(async move { Ok(resp) })
            });

        let bridge = make_bridge(mock);
        let qubo = QuboInstance::new(vec![vec![-1.0, 0.0], vec![0.0, -1.0]]).unwrap();
        let _ = bridge.submit_qaoa(qubo).await;
    }

    // ── Fetch capabilities test ─────────────────────────────────────────

    #[tokio::test]
    async fn fetch_capabilities_parses_response() {
        let mut mock = MockHttpClient::new();
        mock.expect_get()
            .withf(|url| url.contains("/capabilities"))
            .returning(|_| {
                let resp = ok_response(
                    r#"{"max_qubits": 72, "available_ansatze": ["UCCSD", "HEA"], "backend": "origin_quantum"}"#,
                );
                Box::pin(async move { Ok(resp) })
            });

        let bridge = make_bridge(mock);
        let caps = bridge.fetch_capabilities().await.unwrap();
        assert_eq!(caps.max_qubits, 72);
        assert_eq!(caps.provider, ProviderName::OriginQuantum);
    }

    #[tokio::test]
    async fn fetch_capabilities_propagates_error() {
        let mut mock = MockHttpClient::new();
        mock.expect_get().returning(|_| {
            let resp = err_response(500, "error");
            Box::pin(async move { Ok(resp) })
        });

        let bridge = make_bridge(mock);
        let result = bridge.fetch_capabilities().await;
        assert!(result.is_err());
    }
}
