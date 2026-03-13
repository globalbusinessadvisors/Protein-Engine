use std::collections::HashSet;

use pe_quantum_wasm::{
    GateType, MolecularHamiltonian, QaoaResult, QuboInstance, VqeResult,
};

use crate::error::QuantumRouterError;
use crate::traits::QuantumBackend;

/// Dispatches quantum jobs to the optimal available backend.
///
/// Routing logic per ADR-006:
/// 1. Filter by qubit count >= job requirement
/// 2. Filter by gate set superset of required gates
/// 3. Filter by reachability (health check)
/// 4. Sort by closest qubit count match (minimize waste)
/// 5. Fallback to LocalSimulator if it qualifies
/// 6. Return Err(NoSuitableBackend) if nothing qualifies
pub struct QuantumRouter {
    backends: Vec<Box<dyn QuantumBackend>>,
}

impl QuantumRouter {
    pub fn new(backends: Vec<Box<dyn QuantumBackend>>) -> Self {
        Self { backends }
    }

    /// Submit a VQE job, routing to the best available backend.
    pub async fn submit_vqe(
        &self,
        hamiltonian: &MolecularHamiltonian,
        required_qubits: u32,
        required_gates: &HashSet<GateType>,
    ) -> Result<VqeResult, QuantumRouterError> {
        let backend = self
            .select_backend(required_qubits, required_gates)
            .await?;
        backend.submit_vqe(hamiltonian.clone()).await
    }

    /// Submit a QAOA job, routing to the best available backend.
    pub async fn submit_qaoa(
        &self,
        qubo: &QuboInstance,
        required_qubits: u32,
        required_gates: &HashSet<GateType>,
    ) -> Result<QaoaResult, QuantumRouterError> {
        let backend = self
            .select_backend(required_qubits, required_gates)
            .await?;
        backend.submit_qaoa(qubo.clone()).await
    }

    /// Select the best backend for a job with the given requirements.
    async fn select_backend(
        &self,
        required_qubits: u32,
        required_gates: &HashSet<GateType>,
    ) -> Result<&dyn QuantumBackend, QuantumRouterError> {
        // Collect candidates with their capabilities
        let mut candidates: Vec<(usize, u32)> = Vec::new();

        for (idx, backend) in self.backends.iter().enumerate() {
            let caps = backend.capabilities();

            // Filter by qubit count
            if caps.max_qubits < required_qubits {
                continue;
            }

            // Filter by gate set (must be superset)
            if !required_gates.is_subset(&caps.gate_set) {
                continue;
            }

            // Filter by reachability
            if !backend.is_reachable().await {
                continue;
            }

            candidates.push((idx, caps.max_qubits));
        }

        if candidates.is_empty() {
            return Err(QuantumRouterError::NoSuitableBackend {
                required_qubits,
            });
        }

        // Sort by closest qubit count match (smallest sufficient)
        candidates.sort_by_key(|&(_, qubits)| qubits);

        let best_idx = candidates[0].0;
        Ok(self.backends[best_idx].as_ref())
    }
}
