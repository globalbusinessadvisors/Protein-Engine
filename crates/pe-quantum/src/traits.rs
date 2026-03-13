use async_trait::async_trait;
use pe_quantum_wasm::{
    BackendCapabilities, MolecularHamiltonian, QaoaResult, QuboInstance, VqeResult,
};

use crate::error::QuantumRouterError;

/// Common interface for all quantum backends (remote hardware + local simulator).
///
/// Implementations: `LocalSimulatorBackend` (pe-quantum-wasm), `OriginQuantumBackend`,
/// `OpenQasmBackend`. Mockable for London School TDD.
#[async_trait]
#[cfg_attr(test, mockall::automock)]
pub trait QuantumBackend: Send + Sync {
    async fn submit_vqe(
        &self,
        hamiltonian: MolecularHamiltonian,
    ) -> Result<VqeResult, QuantumRouterError>;

    async fn submit_qaoa(
        &self,
        qubo: QuboInstance,
    ) -> Result<QaoaResult, QuantumRouterError>;

    fn capabilities(&self) -> BackendCapabilities;

    /// Health check — returns true if the backend is reachable.
    /// Local simulator always returns true.
    async fn is_reachable(&self) -> bool;
}
