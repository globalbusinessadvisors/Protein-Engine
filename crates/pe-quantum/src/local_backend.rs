use async_trait::async_trait;
use pe_quantum_wasm::{
    BackendCapabilities, MolecularHamiltonian, QaoaConfig, QaoaResult, QaoaRunner, QuboInstance,
    VqeConfig, VqeResult, VqeRunner,
};

use crate::error::QuantumRouterError;
use crate::traits::QuantumBackend;

/// Wraps pe-quantum-wasm's pure-Rust simulator as a `QuantumBackend`.
///
/// Always available — no network, no hardware dependency.
pub struct LocalSimulatorBackend {
    vqe_config: VqeConfig,
    qaoa_config: QaoaConfig,
}

impl Default for LocalSimulatorBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl LocalSimulatorBackend {
    pub fn new() -> Self {
        Self {
            vqe_config: VqeConfig::default(),
            qaoa_config: QaoaConfig::default(),
        }
    }

    pub fn with_configs(vqe_config: VqeConfig, qaoa_config: QaoaConfig) -> Self {
        Self {
            vqe_config,
            qaoa_config,
        }
    }
}

#[async_trait]
impl QuantumBackend for LocalSimulatorBackend {
    async fn submit_vqe(
        &self,
        hamiltonian: MolecularHamiltonian,
    ) -> Result<VqeResult, QuantumRouterError> {
        let runner = VqeRunner::new(self.vqe_config.clone());
        runner
            .run(&hamiltonian)
            .map_err(|e| QuantumRouterError::BackendFailed(e.to_string()))
    }

    async fn submit_qaoa(
        &self,
        qubo: QuboInstance,
    ) -> Result<QaoaResult, QuantumRouterError> {
        let runner = QaoaRunner::new(self.qaoa_config.clone());
        runner
            .run(&qubo)
            .map_err(|e| QuantumRouterError::BackendFailed(e.to_string()))
    }

    fn capabilities(&self) -> BackendCapabilities {
        BackendCapabilities::local_simulator()
    }

    async fn is_reachable(&self) -> bool {
        true
    }
}
