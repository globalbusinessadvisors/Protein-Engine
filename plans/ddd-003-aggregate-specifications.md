# DDD-003: Aggregate Root Specifications and Invariants

**Status:** Accepted
**Date:** 2026-03-13
**Relates to:** DDD-001, DDD-002, ADR-009

---

## Purpose

This document provides detailed specifications for each aggregate root, including all invariants that must hold, factory methods, and the exact validation rules enforced at construction time. These specifications directly inform London School TDD test cases.

---

## Aggregate 1: ProteinVariant

**Crate:** pe-core
**Identity:** `Uuid` (v4, globally unique)
**Lifecycle:** Created by factory, immutable after construction (new mutations produce new variants)

### Structure

```
ProteinVariant
├── id: Uuid                           // identity, assigned at creation
├── name: String                       // human-readable label
├── sequence: AminoAcidSequence        // validated, immutable
├── target_factor: YamanakaFactor      // which Yamanaka factor this variant targets
├── mutations: Vec<Mutation>           // mutations relative to wild-type ancestor
├── generation: u32                    // evolutionary generation number
└── parent_id: Option<Uuid>           // None for wild-type (generation 0)
```

### Invariants

| ID | Invariant | Enforced At |
|----|-----------|-------------|
| PV-1 | `sequence` is a valid AminoAcidSequence (non-empty, all chars in ACDEFGHIKLMNPQRSTVWY) | AminoAcidSequence::new() |
| PV-2 | `id` is a valid Uuid v4 | Construction |
| PV-3 | If `parent_id` is None, then `generation` must be 0 | Factory method |
| PV-4 | If `parent_id` is Some, then `generation` must be > 0 | Factory method |
| PV-5 | Every Mutation in `mutations` has `position` < sequence.len() | Factory method |
| PV-6 | Every Mutation's `from_residue` matches the residue at that position in the **parent** sequence | Mutation::apply() |
| PV-7 | Every Mutation's `to_residue` differs from `from_residue` | Mutation::new() |
| PV-8 | `target_factor` is one of OCT4, SOX2, KLF4, CMYC | Enum — compile-time |

### Factory Methods

```
ProteinVariant::wild_type(name, sequence, target_factor) -> Result<Self>
    // Creates generation-0 variant, no parent, no mutations
    // Validates: PV-1, PV-2

ProteinVariant::from_mutation(parent, mutation) -> Result<Self>
    // Creates child variant with one additional mutation
    // Validates: PV-5, PV-6, PV-7
    // Sets: generation = parent.generation + 1, parent_id = Some(parent.id)

ProteinVariant::from_crossover(parent_a, parent_b, crossover_point) -> Result<Self>
    // Creates child by joining prefix of A with suffix of B
    // Validates: sequences are same length, crossover_point in bounds
    // Sets: generation = max(a.generation, b.generation) + 1
```

### Test Cases (London School — no mocks, pure unit tests)

```
test "wild_type creates generation-0 variant with no parent"
test "wild_type rejects empty sequence"
test "wild_type rejects sequence with invalid residue 'X'"
test "from_mutation increments generation"
test "from_mutation sets parent_id to parent's id"
test "from_mutation rejects position out of bounds"
test "from_mutation rejects from_residue mismatch"
test "from_mutation rejects to_residue == from_residue"
test "from_crossover combines sequences at crossover point"
test "from_crossover rejects mismatched sequence lengths"
```

---

## Aggregate 2: FitnessScore

**Crate:** pe-core
**Identity:** None (value object)
**Lifecycle:** Created by FitnessPredictor, immutable

### Structure

```
FitnessScore
├── reprogramming_efficiency: f64     // 0.0..=1.0
├── expression_stability: f64         // 0.0..=1.0
├── structural_plausibility: f64      // 0.0..=1.0
├── safety_score: f64                 // 0.0..=1.0 (lower = safer)
└── composite: f64                    // weighted average, 0.0..=1.0
```

### Invariants

| ID | Invariant | Enforced At |
|----|-----------|-------------|
| FS-1 | All sub-scores in [0.0, 1.0] | Construction |
| FS-2 | `composite` equals weighted average of sub-scores | Construction |
| FS-3 | Weights sum to 1.0 | FitnessWeights validation |
| FS-4 | Two FitnessScores with identical fields are equal | PartialEq impl |

### Factory Methods

```
FitnessScore::new(reprogramming, stability, plausibility, safety, weights) -> Result<Self>
    // Validates all sub-scores in [0.0, 1.0]
    // Computes composite as: weights.r * reprogramming + weights.s * stability
    //                        + weights.p * plausibility + weights.f * (1.0 - safety)
    // Note: safety is inverted because lower = safer, but higher composite = better

FitnessWeights::new(r, s, p, f) -> Result<Self>
    // Validates r + s + p + f == 1.0 (within f64 epsilon)
```

### Test Cases

```
test "new computes correct composite from weights"
test "new rejects sub-score > 1.0"
test "new rejects sub-score < 0.0"
test "new rejects NaN sub-score"
test "safety_score is inverted in composite (lower safety = higher composite)"
test "two FitnessScores with same fields are equal"
test "FitnessWeights rejects weights not summing to 1.0"
```

---

## Aggregate 3: JournalChain

**Crate:** pe-ledger
**Identity:** Implicit (singleton per platform instance)
**Lifecycle:** Created empty, grows monotonically via append

### Structure

```
JournalChain
├── entries: Vec<JournalEntry>        // ordered, append-only
└── tip_hash: EntryHash               // SHA3-256 of last entry

JournalEntry
├── sequence_number: u64
├── timestamp: DateTime<Utc>
├── prev_hash: [u8; 32]              // SHA3-256 of previous entry
├── entry_type: EntryType
├── payload: Vec<u8>                  // serialized domain event
└── signature: MlDsaSignature
```

### Invariants

| ID | Invariant | Enforced At |
|----|-----------|-------------|
| JC-1 | Entries are strictly ordered by sequence_number (no gaps, no duplicates) | append_entry() |
| JC-2 | entry[i].prev_hash == SHA3-256(serialize(entry[i-1])) for all i > 0 | append_entry() |
| JC-3 | entry[0].prev_hash == [0u8; 32] (genesis entry) | first append |
| JC-4 | Every entry has a valid ML-DSA signature over (sequence_number \|\| prev_hash \|\| payload) | append_entry() |
| JC-5 | No entry is ever modified or removed after append | Type system (no &mut access to past entries) |
| JC-6 | verify_chain() returns Ok(true) if and only if all entries satisfy JC-2 and JC-4 | verify_chain() |

### Methods

```
JournalChain::new() -> Self
    // Empty chain, tip_hash = [0u8; 32]

JournalChain::append_entry(entry_type, payload, signer: &dyn CryptoSigner) -> Result<EntryHash>
    // 1. sequence_number = entries.len() as u64
    // 2. prev_hash = self.tip_hash
    // 3. signing_data = sequence_number || prev_hash || payload
    // 4. signature = signer.sign(signing_data)
    // 5. entry = JournalEntry { sequence_number, timestamp: now(), prev_hash, entry_type, payload, signature }
    // 6. new_hash = SHA3-256(serialize(entry))
    // 7. self.entries.push(entry)
    // 8. self.tip_hash = new_hash
    // 9. return Ok(new_hash)

JournalChain::verify_chain() -> Result<bool>
    // Iterates all entries, checks hash chain + signatures

JournalChain::to_journal_seg() -> Vec<u8>
    // Serializes to JOURNAL_SEG binary format

JournalChain::from_journal_seg(data: &[u8]) -> Result<Self>
    // Deserializes + verifies chain integrity
```

### Test Cases (London School — mock CryptoSigner)

```
test "append to empty chain sets prev_hash to zeros"
test "append chains hashes correctly across 3 entries"
test "verify_chain succeeds on valid chain"
test "verify_chain detects tampered payload"
test "verify_chain detects tampered prev_hash"
test "verify_chain detects invalid signature"
test "sequence_numbers are strictly sequential"
test "round-trip through journal_seg preserves all entries"
test "from_journal_seg rejects corrupt data"
```

---

## Aggregate 4: RvfFile

**Crate:** pe-rvf
**Identity:** SHA3-256 hash of the complete file
**Lifecycle:** Built once by RvfBuilder, immutable after construction

### Structure

```
RvfFile
├── manifest: Manifest
├── segments: BTreeMap<SegmentType, Vec<u8>>
└── file_hash: [u8; 32]              // SHA3-256 of complete serialized file

Manifest
├── name: String
├── version: String
├── capabilities: Vec<Capability>
├── parent_hash: Option<[u8; 32]>    // lineage link to parent .rvf
├── signing_key_fingerprint: Option<[u8; 32]>
└── created_at: DateTime<Utc>
```

### Invariants

| ID | Invariant | Enforced At |
|----|-----------|-------------|
| RF-1 | MANIFEST_SEG (0x00) is always present | build() |
| RF-2 | capabilities list accurately reflects present segments | build() |
| RF-3 | If WASM_SEG present, Capability::WasmRuntime in capabilities | build() |
| RF-4 | If VEC_SEG present, Capability::VecSearch in capabilities | build() |
| RF-5 | Segments are ordered by SegmentType ID | build() |
| RF-6 | file_hash is SHA3-256 of the complete serialized output | build() |
| RF-7 | parent_hash, if set, is a valid SHA3-256 hash (32 bytes) | Manifest validation |

### Test Cases (London School — mock SegmentProducers)

```
test "build with all segments produces valid RvfFile"
test "build fails without MANIFEST_SEG"
test "capabilities auto-populated from present segments"
test "segments ordered by type ID in output"
test "parent_hash links child to parent"
test "file_hash is deterministic for same inputs"
test "round-trip serialize/deserialize preserves all segments"
```

---

## Aggregate 5: QuantumJob

**Crate:** pe-quantum
**Identity:** `Uuid` (job tracking ID)
**Lifecycle:** Created -> Submitted -> Running -> Completed | Failed

### Structure

```
QuantumJob
├── id: Uuid
├── job_type: QuantumJobType          // VQE | QAOA
├── status: JobStatus                 // Created | Submitted | Running | Completed | Failed
├── backend: Option<ProviderName>     // assigned after routing
├── submitted_at: Option<DateTime>
├── completed_at: Option<DateTime>
├── input: QuantumJobInput            // Hamiltonian or QUBO
└── result: Option<QuantumJobResult>  // VqeResult or QaoaResult
```

### Invariants

| ID | Invariant | Enforced At |
|----|-----------|-------------|
| QJ-1 | Status transitions: Created -> Submitted -> Running -> Completed\|Failed | State machine methods |
| QJ-2 | `backend` is set when status moves to Submitted | submit() |
| QJ-3 | `result` is set when status moves to Completed | complete() |
| QJ-4 | `result` is None when status is Failed | fail() |
| QJ-5 | Cannot transition backward (e.g., Completed -> Running) | State machine methods |

### Test Cases (London School — mock QuantumBackend)

```
test "job transitions through full lifecycle"
test "cannot complete a job that was not submitted"
test "cannot submit a job twice"
test "failed job has no result"
test "completed job has result matching backend response"
```

---

## Aggregate 6: ExperimentResult

**Crate:** pe-core
**Identity:** Composite (variant_id + timestamp + instrument_id)
**Lifecycle:** Created once from instrument data, immutable

### Structure

```
ExperimentResult
├── variant_id: Uuid                  // references a ProteinVariant
├── assay_type: AssayType
├── measured_values: BTreeMap<String, f64>
├── timestamp: DateTime<Utc>
├── instrument_id: String
└── notes: Option<String>
```

### Invariants

| ID | Invariant | Enforced At |
|----|-----------|-------------|
| ER-1 | `variant_id` is a valid Uuid | Construction |
| ER-2 | `measured_values` is non-empty | Construction |
| ER-3 | All measured values are finite (no NaN, no Inf) | Construction |
| ER-4 | `instrument_id` is non-empty | Construction |
| ER-5 | `timestamp` is not in the future | Construction (best-effort) |

### Test Cases

```
test "construction validates non-empty measured_values"
test "construction rejects NaN measured value"
test "construction rejects empty instrument_id"
test "assay_type correctly categorizes experiment"
```
