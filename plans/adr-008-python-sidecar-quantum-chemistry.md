# ADR-008: Python Sidecar for Quantum Chemistry (pyChemiQ)

**Status:** Accepted
**Date:** 2026-03-13
**Deciders:** Platform Architecture Team
**Relates to:** FR-06, ADR-006

---

## Context

Origin Quantum's quantum chemistry stack (pyChemiQ, pyqpanda-algorithm, pyqpanda3) is written in Python with C++ extensions. It provides:
- VQE molecular Hamiltonian solvers
- QAOA combinatorial optimization
- Grover's search implementations
- Access to Origin's Wukong 72-qubit processor

There is no Rust equivalent. We have three integration options:
1. **Embed Python in Rust** via PyO3 — tight coupling, complex build, no WASM support
2. **Rewrite in Rust** — enormous effort, Origin's C++ backend not open
3. **Run Python as a sidecar service** — HTTP API, Docker-native, clean boundary

## Decision

**We run pyChemiQ as a standalone Docker sidecar (`chemiq-sidecar`) exposing an HTTP/JSON API.** The `pe-chemistry` Rust crate communicates with it via `reqwest` HTTP client. The sidecar is optional — when unavailable, `QuantumRouter` (ADR-006) falls back to the local pure-Rust simulator.

### Sidecar Structure

```
services/chemiq-sidecar/
├── main.py              # FastAPI/Flask HTTP server
├── requirements.txt     # pychemiq, pyqpanda3, pyqpanda_alg
└── Dockerfile           # Python 3.11 + Origin Quantum deps
```

### HTTP API Surface

```
POST /vqe
  Body: { molecule: str, basis_set: str, ansatz: str, max_iterations: int }
  Response: { energy: float, parameters: [float], iterations: int, converged: bool }

POST /qaoa
  Body: { qubo_matrix: [[float]], p_layers: int }
  Response: { solution: [int], cost: float, iterations: int }

GET /health
  Response: { status: "ok", backend: "origin_quantum"|"simulator" }

GET /capabilities
  Response: { max_qubits: int, available_ansatze: [str], backend: str }
```

### pe-chemistry Bridge

```rust
pub struct ChemiqBridge {
    base_url: String,      // e.g., "http://chemiq-sidecar:8100"
    client: reqwest::Client,
    timeout: Duration,
}

impl QuantumBackend for ChemiqBridge {
    async fn submit_vqe(&self, hamiltonian: MolecularHamiltonian) -> Result<VqeResult> {
        // HTTP POST to /vqe, deserialize response
    }
}
```

## Rationale

- **Clean boundary**: Rust side has zero Python dependencies; pe-chemistry is a pure HTTP client
- **Docker-native**: Sidecar runs as a container in docker-compose alongside the main engine
- **Origin Quantum access**: Full access to Wukong hardware, pyqpanda-algorithm, and pyChemiQ VQE implementations
- **Optional**: Platform works without the sidecar; quantum falls back to local simulator
- **Language-appropriate**: Quantum chemistry libraries are Python-native; forcing them into Rust adds no value

## Consequences

### Positive
- Main Rust build has no Python/C++ toolchain dependency
- Sidecar can be updated independently (new pyChemiQ version = new Docker image)
- HTTP boundary is trivially mockable in London School tests (MockBackend)
- Sidecar can run on a GPU-equipped node while Rust engine runs on CPU

### Negative
- HTTP serialization overhead per VQE call (~1-5ms)
- Sidecar process must be managed (health checks, restart policy)
- Not available in WASM mode (no HTTP to localhost from browser)
- Cold start of sidecar container adds ~5s to first job

### Docker Compose Integration

```yaml
services:
  protein-engine:
    build: { context: ., dockerfile: docker/Dockerfile.node }
    depends_on: [chemiq-sidecar]
    environment:
      CHEMIQ_URL: http://chemiq-sidecar:8100

  chemiq-sidecar:
    build: { context: services/chemiq-sidecar }
    ports: ["8100:8100"]
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:8100/health"]
      interval: 10s
```
