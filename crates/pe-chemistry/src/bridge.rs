//! Anti-corruption layer bridging pe-quantum domain types to the pyChemiQ sidecar HTTP API.

use async_trait::async_trait;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::time::Instant;
use tracing::{debug, warn};

use pe_quantum::traits::QuantumBackend;
use pe_quantum::{
    BackendCapabilities, GateType, MolecularHamiltonian, ProviderName, QaoaResult,
    QuantumRouterError, QuboInstance, VqeResult,
};
use std::collections::HashSet;

use crate::dto::{
    ChemiqCapabilitiesResponse, ChemiqHealthResponse, ChemiqQaoaRequest, ChemiqQaoaResponse,
    ChemiqVqeRequest, ChemiqVqeResponse,
};
use crate::error::ChemistryError;
use crate::http_client::{HttpClient, HttpResponse};

/// Duration for which a health-check result is cached before re-probing.
const HEALTH_CACHE_TTL: Duration = Duration::from_secs(30);

/// Anti-corruption layer that translates Protein-Engine domain types into
/// pyChemiQ sidecar HTTP/JSON calls and back.
pub struct ChemiqBridge<H: HttpClient> {
    http: Arc<H>,
    base_url: String,
    /// Cached (timestamp, reachable) — guarded by an async RwLock.
    health_cache: RwLock<Option<(Instant, bool)>>,
}

impl<H: HttpClient> ChemiqBridge<H> {
    pub fn new(http: Arc<H>, base_url: String) -> Self {
        Self {
            http,
            base_url,
            health_cache: RwLock::new(None),
        }
    }

    /// Translate a `MolecularHamiltonian` into a sidecar VQE request, call the
    /// endpoint, and translate the response back into a domain `VqeResult`.
    async fn call_vqe(
        &self,
        hamiltonian: MolecularHamiltonian,
    ) -> Result<VqeResult, ChemistryError> {
        // Build a description from the Hamiltonian's structure.
        let molecule_desc = format!("{}q-{}t", hamiltonian.num_qubits, hamiltonian.terms.len());
        let req = ChemiqVqeRequest {
            molecule: molecule_desc,
            basis_set: "sto-3g".to_string(),
            ansatz: "UCCSD".to_string(),
            max_iterations: 200,
        };

        let json = serde_json::to_string(&req)
            .map_err(|e| ChemistryError::SerializationFailed(e.to_string()))?;

        let url = format!("{}/vqe", self.base_url);
        debug!(url = %url, "POST /vqe");

        let resp = self.http.post(&url, &json).await?;
        self.check_status(&resp)?;

        let dto: ChemiqVqeResponse = serde_json::from_str(&resp.body)
            .map_err(|e| ChemistryError::ParseError(e.to_string()))?;

        Ok(VqeResult {
            ground_state_energy: dto.energy,
            optimal_parameters: dto.parameters,
            iterations: dto.iterations,
            converged: dto.converged,
        })
    }

    /// Translate a `QuboInstance` into a sidecar QAOA request, call the
    /// endpoint, and translate the response back into a domain `QaoaResult`.
    async fn call_qaoa(&self, qubo: QuboInstance) -> Result<QaoaResult, ChemistryError> {
        let req = ChemiqQaoaRequest {
            qubo_matrix: qubo.matrix.clone(),
            p_layers: qubo.num_variables, // default layers = num_variables
        };

        let json = serde_json::to_string(&req)
            .map_err(|e| ChemistryError::SerializationFailed(e.to_string()))?;

        let url = format!("{}/qaoa", self.base_url);
        debug!(url = %url, "POST /qaoa");

        let resp = self.http.post(&url, &json).await?;
        self.check_status(&resp)?;

        let dto: ChemiqQaoaResponse = serde_json::from_str(&resp.body)
            .map_err(|e| ChemistryError::ParseError(e.to_string()))?;

        Ok(QaoaResult {
            best_bitstring: if dto.solution.is_empty() {
                0
            } else {
                // Convert solution vector to bitstring representation
                dto.solution.iter().enumerate().fold(0usize, |acc, (i, &v)| {
                    if v != 0 { acc | (1 << i) } else { acc }
                })
            },
            best_cost: dto.cost,
            converged: true,
            iterations: dto.iterations,
        })
    }

    /// Check for non-2xx status codes and convert them to `ChemistryError`.
    fn check_status(&self, resp: &HttpResponse) -> Result<(), ChemistryError> {
        if resp.status >= 200 && resp.status < 300 {
            Ok(())
        } else {
            Err(ChemistryError::SidecarError {
                status: resp.status,
                body: resp.body.clone(),
            })
        }
    }

    /// Probe the sidecar's `/health` endpoint with a 30-second cache.
    async fn check_health(&self) -> bool {
        // Fast path: return cached value if still fresh.
        {
            let cache = self.health_cache.read().await;
            if let Some((ts, healthy)) = *cache {
                if ts.elapsed() < HEALTH_CACHE_TTL {
                    return healthy;
                }
            }
        }

        // Slow path: actually probe.
        let url = format!("{}/health", self.base_url);
        let healthy = match self.http.get(&url).await {
            Ok(resp) => {
                if resp.status == 200 {
                    match serde_json::from_str::<ChemiqHealthResponse>(&resp.body) {
                        Ok(h) => h.status == "ok",
                        Err(_) => false,
                    }
                } else {
                    false
                }
            }
            Err(e) => {
                warn!(error = %e, "sidecar health check failed");
                false
            }
        };

        // Update cache.
        {
            let mut cache = self.health_cache.write().await;
            *cache = Some((Instant::now(), healthy));
        }

        healthy
    }

    /// Fetch capabilities from the sidecar.
    pub async fn fetch_capabilities(&self) -> Result<BackendCapabilities, ChemistryError> {
        let url = format!("{}/capabilities", self.base_url);
        let resp = self.http.get(&url).await?;
        self.check_status(&resp)?;

        let dto: ChemiqCapabilitiesResponse = serde_json::from_str(&resp.body)
            .map_err(|e| ChemistryError::ParseError(e.to_string()))?;

        Ok(BackendCapabilities {
            max_qubits: dto.max_qubits,
            gate_set: HashSet::from([
                GateType::H, GateType::X, GateType::Y, GateType::Z,
                GateType::Rx, GateType::Ry, GateType::Rz,
                GateType::Cnot, GateType::Cz,
            ]),
            is_simulator: false,
            provider: ProviderName::OriginQuantum,
        })
    }
}

#[async_trait]
impl<H: HttpClient + 'static> QuantumBackend for ChemiqBridge<H> {
    async fn submit_vqe(
        &self,
        hamiltonian: MolecularHamiltonian,
    ) -> Result<VqeResult, QuantumRouterError> {
        let result = self.call_vqe(hamiltonian).await;
        result.map_err(|e| -> QuantumRouterError { e.into() })
    }

    async fn submit_qaoa(
        &self,
        qubo: QuboInstance,
    ) -> Result<QaoaResult, QuantumRouterError> {
        let result = self.call_qaoa(qubo).await;
        result.map_err(|e| -> QuantumRouterError { e.into() })
    }

    fn capabilities(&self) -> BackendCapabilities {
        // Return a sensible default; a production version would cache
        // the result from fetch_capabilities().
        BackendCapabilities {
            max_qubits: 72,
            gate_set: HashSet::from([
                GateType::H, GateType::X, GateType::Y, GateType::Z,
                GateType::Rx, GateType::Ry, GateType::Rz,
                GateType::Cnot, GateType::Cz,
            ]),
            is_simulator: false,
            provider: ProviderName::OriginQuantum,
        }
    }

    async fn is_reachable(&self) -> bool {
        self.check_health().await
    }
}
