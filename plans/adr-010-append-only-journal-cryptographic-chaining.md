# ADR-010: Append-Only Journal with Cryptographic Hash Chaining

**Status:** Accepted
**Date:** 2026-03-13
**Deciders:** Platform Architecture Team
**Relates to:** FR-07, NFR-06, ADR-005

---

## Context

Protein-Engine produces a continuous stream of events: variant designs, fitness scores, experiment results, model weight updates, quantum simulation snapshots, and governance decisions. For regulatory compliance, IP protection, and scientific reproducibility, the full history must be:

1. **Immutable** — past entries cannot be altered without detection
2. **Ordered** — the sequence of events is cryptographically enforced
3. **Signed** — each entry is attributable to a specific node/researcher
4. **Distributed** — the journal syncs across nodes via QuDAG P2P

## Decision

**JOURNAL_SEG is an append-only log where each entry contains the SHA3-256 hash of the previous entry, forming a hash chain. Each entry is also ML-DSA signed and recorded in WITNESS_SEG via QuDAG.**

### Entry Structure

```
JournalEntry {
    sequence_number: u64,
    timestamp: DateTime<Utc>,
    prev_hash: [u8; 32],          // SHA3-256 of previous entry
    entry_type: EntryType,
    payload: Vec<u8>,             // serialized event data
    signature: MlDsaSignature,   // ML-DSA signature over (sequence_number || prev_hash || payload)
}
```

### Entry Types

| EntryType | Payload | Trigger |
|-----------|---------|---------|
| `VariantDesigned` | ProteinVariant | SequenceExplorer creates a new variant |
| `FitnessScored` | (Uuid, FitnessScore) | FitnessScorerAgent scores a variant |
| `StructureValidated` | (Uuid, ValidationResult) | StructuralValidator checks plausibility |
| `SafetyScreened` | (Uuid, SafetyResult) | ToxicityScreener classifies risk |
| `ExperimentRecorded` | ExperimentResult | Lab instrument data ingested |
| `ModelUpdated` | ModelCheckpointRef | Neural model weights updated |
| `VqeCompleted` | VqeResult | Quantum VQE job finished |
| `CycleCompleted` | CycleResult | Full SAFLA cycle finished |
| `AgentRetired` | AgentMetrics | Governance retired an underperforming agent |

### Verification

```
fn verify_chain(entries: &[JournalEntry]) -> Result<bool>:
    for i in 1..entries.len():
        expected_prev = sha3_256(serialize(entries[i-1]))
        if entries[i].prev_hash != expected_prev:
            return Err(TamperDetected { index: i })
        if !verify_ml_dsa(entries[i].signature, entries[i].signing_data()):
            return Err(InvalidSignature { index: i })
    return Ok(true)
```

## Dual Storage

- **JOURNAL_SEG** (inside `.rvf`): Local append-only log for offline use
- **WITNESS_SEG** (inside `.rvf`): QuDAG witness records — distributed and replicated across peers
- **QuDAG DAG** (P2P network): Distributed version of the same entries, synced when online

JOURNAL_SEG is the authoritative local copy. WITNESS_SEG is the QuDAG-compatible distributed copy. They contain the same data in different formats.

## Rationale

- **Hash chaining** makes insertion/deletion/reordering detectable without a trusted third party
- **ML-DSA signatures** (ADR-005) provide post-quantum attribution
- **Append-only semantics** prevent "rewriting history" — corrections are new entries referencing old ones
- **QuDAG distribution** ensures no single node can unilaterally alter the record
- **RVF packaging** means the journal travels with the `.rvf` file — full provenance in one artifact

## Consequences

### Positive
- Complete, verifiable history of every design decision and experimental result
- Supports regulatory audits (FDA, EMA) for therapeutic protein candidates
- IP disputes resolvable by examining the cryptographically signed timeline
- Offline nodes accumulate local entries and sync when reconnected

### Negative
- JOURNAL_SEG grows monotonically — cannot compact or prune without breaking the hash chain
- Every event incurs SHA3 hashing + ML-DSA signing overhead (~2-3ms)
- Signing requires access to the private key — key management is an operational concern
- Corrupted entries at any point break verification for all subsequent entries

### Mitigation for Growth
- Periodic checkpointing: create a new `.rvf` generation with `parent_hash` linking to the old one
- Archive old JOURNAL_SEGs in cold storage; active `.rvf` starts fresh with checkpoint reference
- HOT_SEG contains only the top-100 active candidates, independent of journal size
