use thiserror::Error;

#[derive(Debug, Error)]
pub enum QuantumError {
    #[error("circuit requires {required} qubits but simulator supports at most {max}")]
    TooManyQubits { required: usize, max: usize },

    #[error("qubit index {0} out of range for {1}-qubit register")]
    QubitOutOfRange(usize, usize),

    #[error("control and target qubits must differ (both are {0})")]
    SameControlTarget(usize),

    #[error("circuit has no qubits")]
    EmptyCircuit,

    #[error("VQE did not converge after {0} iterations")]
    VqeDidNotConverge(usize),

    #[error("QAOA did not converge after {0} iterations")]
    QaoaDidNotConverge(usize),

    #[error("Hamiltonian dimension {got} does not match circuit dimension {expected}")]
    HamiltonianDimensionMismatch { expected: usize, got: usize },

    #[error("QUBO matrix dimension {got} does not match qubit count {expected}")]
    QuboDimensionMismatch { expected: usize, got: usize },

    #[error("QUBO matrix must be square, got {rows}x{cols}")]
    QuboNotSquare { rows: usize, cols: usize },

    #[error("serialization failed: {0}")]
    SerializationFailed(String),
}
