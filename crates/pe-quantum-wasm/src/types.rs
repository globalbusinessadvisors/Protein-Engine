use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// Maximum qubits the statevector simulator supports.
/// 2^20 = 1,048,576 amplitudes × 16 bytes = ~16 MB — practical limit.
pub const MAX_QUBITS: usize = 20;

/// Available quantum gate types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum GateType {
    H,
    X,
    Y,
    Z,
    Rx,
    Ry,
    Rz,
    Cnot,
    Cz,
}

impl GateType {
    /// All gates supported by the local simulator.
    pub fn all() -> HashSet<GateType> {
        [
            GateType::H,
            GateType::X,
            GateType::Y,
            GateType::Z,
            GateType::Rx,
            GateType::Ry,
            GateType::Rz,
            GateType::Cnot,
            GateType::Cz,
        ]
        .into_iter()
        .collect()
    }
}

/// Identifies a quantum backend provider.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProviderName {
    LocalSimulator,
    OriginQuantum,
    Ibm,
    IonQ,
    AwsBraket,
    Quantinuum,
}

/// Describes what a quantum backend can do.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackendCapabilities {
    pub max_qubits: u32,
    pub gate_set: HashSet<GateType>,
    pub is_simulator: bool,
    pub provider: ProviderName,
}

impl BackendCapabilities {
    /// Capabilities of the local statevector simulator.
    pub fn local_simulator() -> Self {
        Self {
            max_qubits: MAX_QUBITS as u32,
            gate_set: GateType::all(),
            is_simulator: true,
            provider: ProviderName::LocalSimulator,
        }
    }
}
