# DDD-002: Bounded Context Integration Patterns

**Status:** Accepted
**Date:** 2026-03-13
**Relates to:** DDD-001, ADR-004, ADR-002

---

## Purpose

This document specifies how bounded contexts communicate, how domain events flow between them, and how the London School TDD mock boundaries align with DDD context boundaries.

---

## Integration Pattern: Domain Events

Bounded contexts communicate through **domain events** — immutable records of something that happened. Events flow in one direction: from the context where they originate to downstream consumers. In-process, events are passed as function arguments or return values through trait boundaries. Across the platform, events are persisted as `JournalEntry` records in the Trust & Audit context.

### Event Catalog

| Event | Origin Context | Consumed By | Payload |
|-------|---------------|-------------|---------|
| `VariantCreated` | Variant Design | Fitness Evaluation, Trust & Audit | ProteinVariant |
| `PopulationEvolved` | Variant Design | Trust & Audit | generation, population_size, mutation_count |
| `VariantScored` | Fitness Evaluation | Variant Design (selection), Trust & Audit | (Uuid, FitnessScore) |
| `EmbeddingStored` | Fitness Evaluation | Packaging (VEC_SEG) | (Uuid, Embedding320) |
| `StructureValidated` | Fitness Evaluation | Variant Design (filter), Trust & Audit | (Uuid, ValidationResult) |
| `SafetyScreened` | Fitness Evaluation | Variant Design (filter), Trust & Audit | (Uuid, SafetyResult) |
| `VqeCompleted` | Quantum Simulation | Fitness Evaluation, Trust & Audit, Packaging (SKETCH_SEG) | VqeResult |
| `QaoaCompleted` | Quantum Simulation | Variant Design, Trust & Audit | QaoaResult |
| `ExperimentRecorded` | Experiment & Lab | Fitness Evaluation (learn), Trust & Audit | ExperimentResult |
| `ProtocolGenerated` | Experiment & Lab | Trust & Audit | LabProtocol |
| `CycleCompleted` | Variant Design | Trust & Audit, Packaging (HOT_SEG) | CycleResult |
| `ModelUpdated` | Fitness Evaluation | Trust & Audit, Packaging (QUANT_SEG) | ModelCheckpointRef |
| `AgentRetired` | Trust & Audit (governance) | Variant Design (swarm) | AgentMetrics |
| `RvfAssembled` | Packaging | Trust & Audit | RvfManifestSummary |

---

## Context Integration Topology

### 1. Variant Design <-> Fitness Evaluation (Shared Kernel)

These two contexts share pe-core types directly. No translation needed.

```
pe-swarm::SequenceExplorer
    │
    │  produces Vec<ProteinVariant>
    ▼
pe-neural::EnsemblePredictor (via FitnessPredictor trait)
    │
    │  returns Vec<ScoredVariant>
    ▼
pe-swarm::SequenceExplorer (selection for next generation)
```

**London School Mock Point:**
- When testing pe-swarm, `FitnessPredictor` is mocked
- When testing pe-neural, `SubModelScorer` (Transformer, LSTM, N-BEATS) are mocked
- pe-core types are **never mocked** — they are the shared kernel

### 2. Variant Design -> Quantum Simulation (Async Request/Response)

Quantum jobs are dispatched asynchronously. The swarm's `QuantumDispatcher` agent submits jobs and polls for results.

```
pe-swarm::QuantumDispatcher
    │
    │  submits MolecularHamiltonian
    ▼
pe-quantum::QuantumRouter (via QuantumBackend trait)
    │
    │  routes to best backend
    ▼
pe-quantum-wasm::LocalSimulator  OR  pe-chemistry::ChemiqBridge
    │
    │  returns VqeResult
    ▼
pe-swarm::QuantumDispatcher (incorporates into cycle)
```

**London School Mock Point:**
- When testing pe-swarm, `QuantumBackend` is mocked (returns canned VqeResult)
- When testing pe-quantum, individual backends are mocked
- When testing pe-chemistry, HTTP responses are mocked

### 3. Experiment & Lab -> Fitness Evaluation (Event-Driven Learning)

Lab results feed back into model training. This is the "LEARN" step of the SAFLA loop.

```
pe-stream::InstrumentSource
    │
    │  produces ExperimentResult
    ▼
pe-swarm::SwarmCoordinator (LEARN step)
    │
    │  passes results to model updater
    ▼
pe-neural (weight update, produces ModelUpdated event)
```

**London School Mock Point:**
- When testing pe-swarm, `InstrumentSource` is mocked (returns canned readings)
- When testing pe-stream, the instrument connection is mocked

### 4. All Contexts -> Trust & Audit (Event Sink)

Every context publishes events to the ledger. The ledger is a passive consumer.

```
Any context
    │
    │  produces domain event
    ▼
pe-ledger::LedgerWriter (via LedgerWriter trait)
    │
    │  wraps in JournalEntry, signs, chains
    ▼
JOURNAL_SEG + WITNESS_SEG (in .rvf)
```

**London School Mock Point:**
- When testing any context, `LedgerWriter` is mocked (verifies append was called with correct event)
- When testing pe-ledger, `CryptoSigner` is mocked

### 5. All Contexts -> Packaging (Segment Production)

Each context produces one or more RVF segments. pe-rvf consumes them all.

```
pe-vector  ──► VEC_SEG, INDEX_SEG, HOT_SEG, SKETCH_SEG
pe-neural  ──► QUANT_SEG, OVERLAY_SEG
pe-vector  ──► GRAPH_SEG, META_SEG, META_IDX_SEG
pe-quantum ──► SKETCH_SEG (VQE snapshots)
pe-ledger  ──► JOURNAL_SEG, WITNESS_SEG, CRYPTO_SEG
pe-wasm    ──► WASM_SEG
pe-rvf     ──► MANIFEST_SEG, KERNEL_SEG
```

**London School Mock Point:**
- When testing pe-rvf, all `SegmentProducer` implementations are mocked (return fixture bytes)

---

## Aggregate Transaction Boundaries

### Rule: One Aggregate Per Transaction

Each operation modifies at most one aggregate. Cross-aggregate consistency is achieved through domain events, not distributed transactions.

| Operation | Aggregate Modified | Events Emitted |
|-----------|-------------------|----------------|
| Create variant | ProteinVariant | VariantCreated |
| Score variant | FitnessScore (created, not modified) | VariantScored |
| Run evolution step | ProteinVariant (new generation) | PopulationEvolved |
| Submit VQE | QuantumJob | VqeCompleted |
| Record experiment | ExperimentResult | ExperimentRecorded |
| Append to journal | JournalChain | (is the event store itself) |
| Build .rvf | RvfFile | RvfAssembled |

### Eventual Consistency Points

- **Scoring -> Selection**: Variants are scored asynchronously; selection waits until all scores are available for the current generation
- **Lab Results -> Model Update**: Model weights are updated in batch after new experiment results arrive, not per-result
- **Quantum -> Scoring**: VQE energy values are incorporated in the next design cycle, not retroactively applied
- **Journal -> WITNESS_SEG**: Local journal entries are synced to QuDAG peers when connectivity is available; not real-time

---

## Repository Pattern (per Aggregate Root)

Each aggregate root has a repository trait for persistence. In production, repositories serialize to RVF segments. In tests, repositories are in-memory mocks.

```rust
// Variant Design context
#[automock]
pub trait VariantRepository: Send + Sync {
    fn save(&mut self, variant: &ProteinVariant) -> Result<()>;
    fn find_by_id(&self, id: Uuid) -> Result<Option<ProteinVariant>>;
    fn find_by_generation(&self, gen: u32) -> Result<Vec<ProteinVariant>>;
    fn find_by_factor(&self, factor: YamanakaFactor) -> Result<Vec<ProteinVariant>>;
}

// Fitness Evaluation context
#[automock]
pub trait ScoreRepository: Send + Sync {
    fn save_score(&mut self, variant_id: Uuid, score: FitnessScore) -> Result<()>;
    fn get_score(&self, variant_id: Uuid) -> Result<Option<FitnessScore>>;
    fn get_top_scored(&self, k: usize) -> Result<Vec<ScoredVariant>>;
}

// Trust & Audit context
// LedgerWriter trait (from ADR-010) serves as the repository for JournalChain
```

---

## Mock Boundary Alignment Summary

The London School mock boundaries (ADR-002) align exactly with DDD bounded context boundaries:

| DDD Context Boundary | Trait Boundary (Mock Point) | Test Isolation |
|---------------------|-----------------------------|----------------|
| Variant Design <-> Fitness Evaluation | `FitnessPredictor`, `VectorStore` | pe-swarm tests mock scoring |
| Variant Design <-> Quantum Simulation | `QuantumBackend` | pe-swarm tests mock quantum |
| Variant Design <-> Experiment & Lab | `InstrumentSource` | pe-swarm tests mock instruments |
| All <-> Trust & Audit | `LedgerWriter`, `CryptoSigner` | All tests mock ledger |
| All <-> Packaging | `SegmentProducer`, `RvfBuilder` | pe-rvf tests mock producers |
| Quantum Simulation <-> pyChemiQ | `QuantumBackend` (ChemiqBridge impl) | pe-quantum tests mock HTTP |

This alignment is not accidental — it is the core design principle. **Every DDD context boundary IS a trait boundary IS a mock boundary.**
