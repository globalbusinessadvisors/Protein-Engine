# ADR-006: Hardware-Agnostic Quantum Backend Routing with Offline Fallback

**Status:** Accepted
**Date:** 2026-03-13
**Deciders:** Platform Architecture Team
**Relates to:** FR-06, NFR-05

---

## Context

Protein-Engine uses quantum simulation for two key tasks:
1. **VQE (Variational Quantum Eigensolver)**: Computing molecular Hamiltonians for protein binding energy estimation
2. **QAOA (Quantum Approximate Optimization Algorithm)**: Solving sequence-design QUBO formulations

Multiple quantum hardware providers exist with different APIs, qubit counts, gate sets, and availability:
- Origin Quantum (Wukong, 72 qubits) via pyChemiQ/pyqpanda
- IBM Quantum (127+ qubits) via OpenQASM
- IonQ, AWS Braket, Quantinuum via OpenQASM
- Local simulation (always available, no network)

The platform must work offline (a researcher on a plane) and must not hard-code a single vendor.

## Decision

**We implement a `QuantumRouter` that dispatches quantum jobs to the best available backend based on job requirements and backend capabilities.** The router implements the strategy pattern with automatic fallback to a pure-Rust local simulator (`pe-quantum-wasm`) when all remote backends are unreachable.

### Architecture

```
pe-quantum (native only)
├── QuantumRouter          — selects backend, manages fallback
├── OriginQuantumBackend   — pyChemiQ sidecar HTTP bridge
├── OpenQasmBackend        — IBM, IonQ, Braket, Quantinuum
└── trait QuantumBackend   — common interface

pe-quantum-wasm (all targets, pure Rust)
└── LocalSimulator         — statevector simulator, always available
     implements QuantumBackend
```

### Routing Logic

```
fn select_backend(job: &QuantumJob) -> &dyn QuantumBackend:
    1. Filter backends by: qubit_count >= job.required_qubits
    2. Filter by: gate_set superset of job.required_gates
    3. Filter by: is_reachable (health check within last 30s)
    4. Sort by: closest qubit count match (minimize waste)
    5. If no remote backend qualifies AND job fits local simulator:
       return LocalSimulator
    6. If nothing qualifies: return Err(NoSuitableBackend)
```

## Rationale

- **Vendor independence**: Trait boundary means adding a new quantum provider requires only implementing `QuantumBackend`
- **Offline-first**: `pe-quantum-wasm` is a pure-Rust statevector simulator with no dependencies — it compiles to WASM and runs in the browser, ensuring quantum simulation is always available
- **Cost optimization**: Router can prefer cheaper/faster backends for small jobs and reserve expensive hardware for large circuits
- **Testability**: `MockBackend` in London School tests verifies routing logic without any quantum hardware

## Consequences

### Positive
- Researchers without quantum hardware access can still run small simulations locally
- WASM browser mode includes quantum simulation — no server required
- Adding Origin Quantum's next-gen hardware is a new struct implementing the trait
- VQE snapshots cached in SKETCH_SEG avoid redundant re-computation

### Negative
- Local simulator limited to ~20 qubits (exponential memory growth)
- pyChemiQ sidecar introduces a Python process dependency for Origin Quantum
- OpenQASM translation layer adds complexity for non-Origin backends
- Backend health checking adds latency to first job submission

### Python Sidecar (pyChemiQ)

Origin Quantum's VQE implementation is Python-only (pyChemiQ). Rather than embedding Python, we run a separate `chemiq-sidecar` Docker container exposing an HTTP API:

```
pe-chemistry ──HTTP──► chemiq-sidecar (Python)
                       ├── pyChemiQ (VQE)
                       ├── pyqpanda-algorithm
                       └── pyqpanda3
```

This keeps the Rust codebase pure Rust while accessing Origin Quantum's full chemistry stack.

## QuantumBackend Trait

```rust
#[async_trait]
#[automock]
pub trait QuantumBackend: Send + Sync {
    async fn submit_vqe(&self, hamiltonian: MolecularHamiltonian) -> Result<VqeResult>;
    async fn submit_qaoa(&self, qubo: QuboInstance) -> Result<QaoaResult>;
    fn capabilities(&self) -> BackendCapabilities;
}

pub struct BackendCapabilities {
    pub max_qubits: u32,
    pub gate_set: HashSet<GateType>,
    pub is_simulator: bool,
    pub provider: ProviderName,
}
```
