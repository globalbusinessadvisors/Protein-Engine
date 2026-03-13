# ADR-001: RVF as Universal Deployment Artifact

**Status:** Accepted
**Date:** 2026-03-13
**Deciders:** Platform Architecture Team
**Relates to:** FR-09, FR-11, NFR-01, NFR-05

---

## Context

Protein-Engine must deploy identically across seven fundamentally different targets: Docker server, WASM browser, bare-metal lab server, Raspberry Pi, Claude Code (MCP), peer-to-peer transfer, and offline researcher workstation. Traditional approaches require separate artifacts per target (Docker images, npm packages, binaries, database dumps, model archives), creating version drift, configuration divergence, and reproducibility failures.

The RuVector project provides RVF (RuVector Format), a binary cognitive container format with 24 segment types that merges database, model, graph engine, kernel, and attestation into a single file.

## Decision

**We adopt RVF as the sole deployment unit for the entire platform.** Every component — protein embeddings, HNSW indices, neural model weights, GNN state, quantum VQE snapshots, cryptographic witness chains, WASM runtime, and optional unikernel — is packaged into a single `protein-engine.rvf` file.

There is no separate Docker image vs. WASM bundle vs. database vs. model archive. The file is the platform.

## Rationale

- **Single-artifact reproducibility**: A researcher who downloads `protein-engine.rvf` gets an exact, verifiable copy of the entire platform state — data, models, runtime, and audit trail
- **Lineage tracking**: MANIFEST_SEG includes `parent_hash`, allowing child `.rvf` files to be provably derived from a parent, creating DNA-style lineage
- **Tamper evidence**: WITNESS_SEG + CRYPTO_SEG provide cryptographic proof that no segment has been altered
- **Zero-conversion transfer**: rvf-wire protocol streams the file as-is between peers; no serialization/deserialization translation layer
- **Feature-flag activation**: Each runtime activates only the segments it needs — WASM_SEG for browsers, KERNEL_SEG for Docker/bare-metal, all segments for MCP

## Consequences

### Positive
- Eliminates "works on my machine" — the artifact is byte-identical everywhere
- Offline-first by design; no network dependency required to operate
- Simplifies CI/CD to a single `build-rvf.sh` pipeline output
- MCP server can expose any segment to AI agents without translation

### Negative
- File size grows with data volume; large variant populations will produce multi-GB `.rvf` files
- All segment producers must agree on the RVF wire format; breaking changes in `rvf-types` affect every crate
- Debugging requires RVF-aware tooling rather than standard database/file inspectors
- Cannot incrementally update a single segment without rebuilding the file (mitigated by rvf-wire streaming append)

### Risks
- RuVector RVF spec is under active development; breaking changes in segment layout could require migration tooling
- Mitigation: pin git revisions; wrap all RVF access behind `pe-rvf` crate traits so internal changes are isolated

## Segment Allocation

| Segment | ID | Contents |
|---------|----|----------|
| MANIFEST_SEG | 0x00 | Root metadata, capabilities, lineage hash |
| VEC_SEG | 0x01 | 320-dim ESM-2 protein embeddings |
| INDEX_SEG | 0x02 | HNSW nearest-neighbor index |
| OVERLAY_SEG | 0x03 | LoRA adapter deltas |
| JOURNAL_SEG | 0x04 | Append-only experiment log |
| GRAPH_SEG | 0x05 | GNN protein interaction network |
| QUANT_SEG | 0x06 | INT8 quantized neural weights |
| META_SEG | 0x07 | Per-variant filterable metadata |
| HOT_SEG | 0x08 | Top-100 promoted candidates |
| SKETCH_SEG | 0x09 | MinHash sketches + VQE snapshots |
| WASM_SEG | 0x0A | 5.5KB WASM microkernel |
| WITNESS_SEG | 0x0B | QuDAG cryptographic witness chain |
| CRYPTO_SEG | 0x0C | TEE attestation |
| META_IDX_SEG | 0x0D | Filterable metadata index |
| KERNEL_SEG | 0x0E | Optional unikernel |
