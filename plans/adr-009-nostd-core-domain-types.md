# ADR-009: no_std Core Domain Types in pe-core

**Status:** Accepted
**Date:** 2026-03-13
**Deciders:** Platform Architecture Team
**Relates to:** FR-01, NFR-05

---

## Context

Protein-Engine targets platforms ranging from full Linux servers to WASM browser tabs to Raspberry Pi lab nodes. The core domain types (`ProteinVariant`, `Mutation`, `FitnessScore`, `AminoAcidSequence`, `ExperimentResult`) are used by every crate in the workspace. If pe-core depends on `std`, it cannot compile on `no_std` targets, and changes to core types may inadvertently break WASM or embedded builds.

## Decision

**`pe-core` is a `no_std` crate with optional `alloc` support.** It uses `#![no_std]` at the crate root and depends only on:
- `serde` (with `derive`, no `std` feature)
- `uuid` (with `js` feature for WASM compatibility)
- `chrono` (with `wasmbind` feature)

All collection types use `alloc::vec::Vec`, `alloc::string::String`, and `alloc::collections::BTreeMap` (not `HashMap`, which requires `std` for the random state).

## Rationale

- **Universal compilation**: pe-core compiles on every target without conditional compilation or feature flags
- **Zero feature flags in pe-core**: No `#[cfg(feature = "...")]` in domain types — they are the same everywhere
- **Forced simplicity**: `no_std` prevents accidentally pulling in filesystem, networking, or threading dependencies into domain types
- **Embedded future-proofing**: If a future lab instrument runs a bare-metal Rust firmware, it can use pe-core directly

## Type Design Constraints

| Constraint | Reason |
|-----------|--------|
| `BTreeMap` instead of `HashMap` | `HashMap` requires `std::collections::hash_map::RandomState` |
| `alloc::string::String` instead of `std::string::String` | Same type, but import from `alloc` |
| No `std::io` usage | Use `serde` for serialization instead of `Read`/`Write` traits |
| No `std::time` | Use `chrono` with `wasmbind` feature |
| No floating-point formatting via `std::fmt` | Use `serde` for f64 serialization |

## Consequences

### Positive
- pe-core is the bedrock of the workspace — guaranteed to compile everywhere
- Domain type changes are immediately validated across all targets in CI
- Encourages rich domain modeling without platform-specific concerns
- Other crates depend on pe-core freely without feature flag coordination

### Negative
- `BTreeMap` is O(log n) lookup vs `HashMap` O(1) — acceptable for metadata maps (small N)
- Cannot use `std::error::Error` in `no_std` — use `thiserror` with `no_std` support or custom error types
- Some `serde` features require `std`; must be careful with feature selection
- Developers must remember to import from `alloc` not `std` in pe-core

## Crate Header

```rust
// pe-core/src/lib.rs
#![no_std]
extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;
use alloc::collections::BTreeMap;
use serde::{Serialize, Deserialize};
use uuid::Uuid;
```
