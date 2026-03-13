//! pe-quantum-wasm: Pure-Rust statevector quantum simulator.
//!
//! Provides VQE and QAOA implementations that run on every target
//! including WASM browsers. Always available as the offline fallback
//! when remote quantum backends are unreachable.

pub mod circuit;
pub mod error;
pub mod hamiltonian;
pub mod qaoa;
pub mod statevector;
pub mod types;
pub mod vqe;

pub use circuit::{CircuitBuilder, QuantumCircuit};
pub use error::QuantumError;
pub use hamiltonian::MolecularHamiltonian;
pub use qaoa::{QaoaConfig, QaoaResult, QaoaRunner, QuboInstance};
pub use statevector::StateVector;
pub use types::{BackendCapabilities, GateType, ProviderName, MAX_QUBITS};
pub use vqe::{VqeConfig, VqeResult, VqeRunner};

#[cfg(test)]
mod tests;
