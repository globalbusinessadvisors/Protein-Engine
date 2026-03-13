# DDD-001: Protein Engineering Domain Model

**Status:** Accepted
**Date:** 2026-03-13
**Relates to:** FR-01, FR-04, FR-05, ADR-009

---

## Ubiquitous Language

These terms are used consistently across all crates, documentation, APIs, and tests. No synonyms permitted.

| Term | Definition |
|------|-----------|
| **Protein Variant** | A specific amino acid sequence being evaluated as a candidate reprogramming factor. The central entity. |
| **Yamanaka Factor** | One of four transcription factors (OCT4, SOX2, KLF4, cMYC) used for cellular reprogramming. Every variant targets exactly one factor. |
| **Mutation** | A single amino acid substitution at a specific position in a sequence. Described as position + from_residue + to_residue. |
| **Amino Acid Sequence** | A validated, immutable string of single-letter amino acid codes (ACDEFGHIKLMNPQRSTVWY). The fundamental biological data type. |
| **Fitness Score** | A composite prediction of a variant's quality: reprogramming efficiency, expression stability, structural plausibility, and safety score. |
| **Experiment Result** | A measurement from a physical laboratory instrument linked to a specific variant and assay type. |
| **Generation** | One iteration of the evolutionary optimization cycle. Each generation produces a new population of variants. |
| **Lineage** | The parent-child chain linking a variant back through its mutation history to the wild-type ancestor. |
| **Embedding** | A 320-dimensional float vector produced by ESM-2 that represents a sequence's learned biological features. |
| **Scored Variant** | A variant paired with its fitness score. The unit of selection in evolutionary optimization. |
| **Promoted Candidate** | A variant that passed all pipeline stages (scoring, validation, safety screening) and was promoted to HOT_SEG. |
| **Design Cycle** | One complete SAFLA loop: design -> score -> validate -> screen -> (measure) -> learn -> promote. |
| **Journal Entry** | An immutable, signed record of any event in the platform's history. |

---

## Bounded Contexts

### 1. Variant Design Context (pe-core, pe-swarm)

**Responsibility:** Creating, mutating, crossing over, and selecting protein variants.

**Aggregates:**
- `ProteinVariant` (aggregate root)
  - `AminoAcidSequence` (value object)
  - `Mutation` (value object)
  - `YamanakaFactor` (enum value object)

**Entities:**
- `ProteinVariant` — has identity (Uuid), mutable across generations via mutation

**Value Objects:**
- `AminoAcidSequence` — immutable, validated on construction, equality by content
- `Mutation` — immutable triple (position, from, to)
- `YamanakaFactor` — enum: OCT4, SOX2, KLF4, CMYC
- `AminoAcid` — enum of 20 standard residues

**Domain Rules:**
- A sequence must contain only valid amino acid codes (20 standard)
- A mutation's `from_residue` must match the residue at `position` in the parent sequence
- A mutation's `to_residue` must differ from `from_residue`
- A variant's `generation` is always `parent.generation + 1`
- A variant with no parent is generation 0 (wild-type)

**Invariants:**
- `AminoAcidSequence` is never empty
- `Mutation.position` is within bounds of the sequence
- `ProteinVariant.id` is globally unique (Uuid v4)

---

### 2. Fitness Evaluation Context (pe-neural, pe-vector, pe-solver)

**Responsibility:** Predicting biological fitness from sequence data using neural models and vector similarity.

**Aggregates:**
- `FitnessScore` (value object — no identity, pure computation result)

**Value Objects:**
- `FitnessScore` — composite of four sub-scores plus a weighted aggregate
  - `reprogramming_efficiency: f64` (0.0..1.0)
  - `expression_stability: f64` (0.0..1.0)
  - `structural_plausibility: f64` (0.0..1.0)
  - `safety_score: f64` (0.0..1.0, lower = safer)
  - `composite: f64` (weighted aggregate)
- `Embedding320` — 320-dim f32 vector, produced by ESM-2
- `VariantMeta` — filterable metadata attached to a stored embedding
- `ScoredVariant` — (ProteinVariant, FitnessScore) pair

**Domain Rules:**
- All sub-scores are clamped to [0.0, 1.0]
- Composite score is a weighted average; weights are configurable but must sum to 1.0
- Embedding dimensionality is always 320 (ESM-2 t6 8M output)
- Similarity search uses cosine distance

**Domain Services:**
- `EmbeddingModel` — transforms AminoAcidSequence -> Embedding320
- `FitnessPredictor` — transforms (ProteinVariant, Embedding320) -> FitnessScore
- `VectorStore` — stores and retrieves embeddings with nearest-neighbor search

---

### 3. Quantum Simulation Context (pe-quantum, pe-quantum-wasm, pe-chemistry)

**Responsibility:** Computing molecular energy landscapes and solving combinatorial optimization problems via quantum algorithms.

**Aggregates:**
- `QuantumJob` (entity — has identity, lifecycle)

**Entities:**
- `QuantumJob` — tracks submission, execution, and completion of a quantum computation

**Value Objects:**
- `MolecularHamiltonian` — operator describing a molecule's energy (matrix representation)
- `QuboInstance` — quadratic unconstrained binary optimization matrix
- `VqeResult` — ground state energy + variational parameters + convergence flag
- `QaoaResult` — optimal binary solution + cost + iteration count
- `BackendCapabilities` — qubit count, gate set, provider identity

**Domain Rules:**
- A VQE job requires a Hamiltonian with qubit count <= backend's max_qubits
- A QAOA job requires a QUBO matrix of dimension <= backend's max_qubits
- The router selects the backend with the smallest sufficient qubit count
- If no remote backend is reachable, fall back to local simulator
- VQE results are cached in SKETCH_SEG to avoid redundant recomputation

**Domain Services:**
- `QuantumBackend` — submits VQE/QAOA jobs and returns results
- `QuantumRouter` — selects the optimal backend for a given job

---

### 4. Experiment & Lab Context (pe-stream, services/instrument-bridge)

**Responsibility:** Ingesting real-world laboratory measurements and generating instrument protocols.

**Aggregates:**
- `ExperimentResult` (entity — has identity via variant_id + timestamp)

**Entities:**
- `ExperimentResult` — a specific measurement linked to a variant

**Value Objects:**
- `AssayType` — enum: FlowCytometry, WesternBlot, qPCR, PlateReader, CellViability, Custom(String)
- `InstrumentReading` — raw data point from a lab instrument
- `InstrumentType` — enum: Opentrons, Hamilton, FlowCytometer, PlateReader
- `LabProtocol` — generated protocol for Opentrons or Hamilton liquid handlers

**Domain Rules:**
- An ExperimentResult must reference an existing ProteinVariant (by Uuid)
- Measured values are stored as (metric_name: String, value: f64) pairs
- Instrument readings are timestamped and attributed to a specific instrument_id
- Lab protocols are generated only for variants that passed safety screening

**Domain Services:**
- `InstrumentSource` — reads data from lab instruments (flow cytometer, plate reader)
- `ProtocolGenerator` — creates Opentrons/Hamilton protocols from scored variants

---

### 5. Trust & Audit Context (pe-ledger, pe-governance)

**Responsibility:** Maintaining an immutable, cryptographically signed record of all platform events and governing agent lifecycle.

**Aggregates:**
- `JournalChain` (aggregate root — the complete append-only log)
  - `JournalEntry` (entity within the chain)

**Entities:**
- `JournalEntry` — sequence-numbered, hash-chained, signed event record

**Value Objects:**
- `EntryHash` — SHA3-256 hash of a serialized entry
- `MlDsaSignature` — post-quantum digital signature
- `EntryType` — enum of all auditable events (see ADR-010)
- `AgentMetrics` — performance stats for a swarm agent
- `BudgetAllocation` — compute budget assigned to agents per cycle

**Domain Rules:**
- Entries are append-only; no update or delete
- Each entry's `prev_hash` must equal SHA3-256 of the previous entry's serialized form
- Each entry must carry a valid ML-DSA signature
- Chain verification traverses from genesis to tip; any break is a tamper detection
- Corrections are recorded as new entries referencing the corrected entry's sequence_number

**Domain Services:**
- `LedgerWriter` — appends entries and verifies the chain
- `CryptoSigner` — signs and verifies data with ML-DSA
- `LifecycleManager` — decides agent retirement and budget allocation

---

### 6. Packaging & Distribution Context (pe-rvf)

**Responsibility:** Assembling all data, models, indices, and runtime into a single `.rvf` cognitive container.

**Aggregates:**
- `RvfFile` (aggregate root — the complete packaged artifact)
  - `Manifest` (entity — root metadata)
  - `Segment` (value object — typed binary blob)

**Value Objects:**
- `Manifest` — name, version, capabilities list, parent_hash, signing key fingerprint
- `Segment` — (SegmentType, Vec<u8>) pair
- `SegmentType` — enum of 15 segment identifiers (0x00..0x0E)
- `Capability` — enum: VecSearch, ProteinScoring, Evolution, WasmRuntime, QuantumVqe, P2pSync, McpAgent, TeeAttestation

**Domain Rules:**
- Every `.rvf` file must have a MANIFEST_SEG
- MANIFEST_SEG capabilities must accurately reflect which segments are present
- `parent_hash` in MANIFEST_SEG links to the parent `.rvf` file's hash (lineage)
- WASM_SEG is required for browser deployment; KERNEL_SEG for Docker/bare-metal
- Segment ordering within the file follows the SegmentType ID (0x00 first)

**Domain Services:**
- `RvfBuilder` — assembles segments into a valid `.rvf` file
- `SegmentProducer` — produces the binary content for one segment type

---

## Context Map

```
┌─────────────────────────────────────────────────────────────────┐
│                        CONTEXT MAP                               │
│                                                                  │
│  ┌──────────────────┐        ┌──────────────────────┐           │
│  │ Variant Design   │◄──────►│ Fitness Evaluation    │           │
│  │ (pe-core,        │ shared │ (pe-neural, pe-vector,│           │
│  │  pe-swarm)       │ kernel │  pe-solver)           │           │
│  │                  │        │                       │           │
│  │ ProteinVariant   │───────►│ Embedding320          │           │
│  │ Mutation         │        │ FitnessScore          │           │
│  │ AminoAcidSequence│        │ ScoredVariant         │           │
│  └────────┬─────────┘        └───────────┬───────────┘           │
│           │                              │                       │
│           │ conformist                   │ conformist             │
│           ▼                              ▼                       │
│  ┌──────────────────┐        ┌──────────────────────┐           │
│  │ Experiment & Lab  │        │ Quantum Simulation    │           │
│  │ (pe-stream,       │        │ (pe-quantum,          │           │
│  │  instrument-      │        │  pe-quantum-wasm,     │           │
│  │  bridge)          │        │  pe-chemistry)        │           │
│  │                   │        │                       │           │
│  │ ExperimentResult  │        │ VqeResult             │           │
│  │ LabProtocol       │        │ MolecularHamiltonian  │           │
│  └────────┬──────────┘        └───────────┬───────────┘           │
│           │                               │                      │
│           │ published language             │ published language   │
│           ▼                               ▼                      │
│  ┌────────────────────────────────────────────────────┐          │
│  │              Trust & Audit                          │          │
│  │              (pe-ledger, pe-governance)              │          │
│  │                                                     │          │
│  │  JournalEntry ◄── all contexts publish events here  │          │
│  │  MlDsaSignature                                     │          │
│  │  AgentMetrics                                       │          │
│  └────────────────────────┬───────────────────────────┘          │
│                           │                                      │
│                           │ customer/supplier                    │
│                           ▼                                      │
│  ┌────────────────────────────────────────────────────┐          │
│  │           Packaging & Distribution                  │          │
│  │           (pe-rvf)                                  │          │
│  │                                                     │          │
│  │  RvfFile ◄── consumes segments from all contexts    │          │
│  │  Manifest                                           │          │
│  │  Segment                                            │          │
│  └────────────────────────────────────────────────────┘          │
└─────────────────────────────────────────────────────────────────┘
```

### Relationship Types

| Upstream | Downstream | Relationship | Pattern |
|----------|-----------|--------------|---------|
| Variant Design | Fitness Evaluation | Shared Kernel | pe-core types used by both |
| Variant Design | Experiment & Lab | Conformist | Lab conforms to variant IDs and types |
| Fitness Evaluation | Quantum Simulation | Conformist | Quantum conforms to scoring pipeline needs |
| All contexts | Trust & Audit | Published Language | All contexts publish JournalEntry events |
| All contexts | Packaging & Distribution | Customer/Supplier | pe-rvf consumes segments from all producers |
| Quantum Simulation | pyChemiQ Sidecar | Anti-Corruption Layer | pe-chemistry translates HTTP/JSON to domain types |

---

## Anti-Corruption Layers

### pe-chemistry (pyChemiQ ACL)

The pyChemiQ sidecar speaks HTTP/JSON with Python-centric data structures. `pe-chemistry` translates between the sidecar's API and the domain's `MolecularHamiltonian` / `VqeResult` types.

```
Domain types ──► pe-chemistry::ChemiqBridge ──HTTP──► chemiq-sidecar (Python)
                  (translates Rust structs          (pyChemiQ, pyqpanda)
                   to/from JSON)
```

### pe-stream (Instrument ACL)

Lab instruments produce vendor-specific data formats. `pe-stream` normalizes these into `InstrumentReading` and `ExperimentResult` domain types.

```
Raw instrument data ──► pe-stream ──► ExperimentResult (domain type)
(Opentrons JSON,         (ACL)
 Hamilton CSV,
 flow cytometry FCS)
```

### pe-rvf (RVF Format ACL)

The RVF wire format is defined by the external RuVector project. `pe-rvf` translates between domain types and RVF segment binary representations.

```
Domain aggregates ──► pe-rvf::SegmentProducer ──► RVF binary segments
                       (ACL)
```
