//! pe-quantum: Hardware-agnostic quantum backend router.
//!
//! Dispatches VQE and QAOA jobs to the optimal backend based on
//! qubit requirements and availability, with automatic fallback
//! to the local pure-Rust simulator (pe-quantum-wasm).

pub mod error;
pub mod job;
pub mod local_backend;
pub mod router;
pub mod segment;
pub mod traits;

pub use error::QuantumRouterError;
pub use job::{JobStatus, QuantumJob, QuantumJobInput, QuantumJobResult, QuantumJobType};
pub use local_backend::LocalSimulatorBackend;
pub use router::QuantumRouter;
pub use segment::{SketchSegProducer, VqeSnapshot, VqeSnapshotCache};
pub use traits::QuantumBackend;

// Re-export key types from pe-quantum-wasm for convenience
pub use pe_quantum_wasm::{
    BackendCapabilities, GateType, MolecularHamiltonian, ProviderName, QaoaResult, QuboInstance,
    VqeResult,
};

#[cfg(test)]
mod tests;
