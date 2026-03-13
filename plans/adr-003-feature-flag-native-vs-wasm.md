# ADR-003: Feature Flag Strategy — native vs wasm

**Status:** Accepted
**Date:** 2026-03-13
**Deciders:** Platform Architecture Team
**Relates to:** FR-09, FR-11, NFR-01, NFR-05

---

## Context

Protein-Engine must compile to two fundamentally different targets from the same source tree:
1. **Native** (x86_64, aarch64): Full server with tokio async runtime, axum HTTP, libp2p networking, ONNX Runtime GPU inference, polars dataframes, rayon parallelism
2. **WASM** (wasm32-unknown-unknown): Browser runtime with wasm-bindgen, web-sys, gloo-net, IndexedDB storage, single-threaded execution

Many Rust crates in the dependency tree are incompatible with WASM (tokio with `full` features, libp2p, ort, rayon). Including them in a WASM build causes compilation failures or bloated bundles.

## Decision

**We use two mutually exclusive Cargo feature flags — `native` and `wasm` — to gate target-specific dependencies.** The `native` feature is the default. The `wasm` feature is activated explicitly during wasm-pack builds. The `pe-core` crate has no feature flags and compiles everywhere (`no_std` compatible).

```toml
[features]
default = ["native"]
native = ["dep:tokio", "dep:axum", "dep:libp2p", "dep:ort", "dep:faer",
          "dep:polars", "dep:rayon", "dep:tracing-subscriber",
          "dep:rvf-server", "dep:rvf-runtime", "dep:tower"]
wasm   = ["dep:wasm-bindgen", "dep:wasm-bindgen-futures", "dep:js-sys",
          "dep:web-sys", "dep:gloo-net", "dep:gloo-storage",
          "dep:serde-wasm-bindgen", "dep:console_error_panic_hook",
          "dep:tracing-wasm", "dep:rvf-wasm"]
```

## Rationale

- **Mutual exclusivity prevents accidental cross-contamination**: A crate cannot accidentally pull tokio into a WASM build
- **`pe-core` as the universal foundation**: Domain types (ProteinVariant, Mutation, FitnessScore) compile on every target without conditional compilation
- **Build command clarity**: `cargo build --features native` vs `wasm-pack build --no-default-features --features wasm` — no ambiguity
- **Crate-level target specialization**: pe-api (native only), pe-wasm (wasm only), pe-cli (native only) each activate only their feature

## Crate-to-Feature Mapping

| Crate | native | wasm | no flags (both) |
|-------|--------|------|-----------------|
| pe-core | - | - | Always compiles |
| pe-vector | Uses ort, rayon | Uses candle-core only | Core traits always available |
| pe-neural | Full ensemble + ort | Lightweight candle | FitnessPredictor trait always available |
| pe-quantum | Hardware backend routing | - | Not compiled for WASM |
| pe-quantum-wasm | - | - | Pure Rust, always available |
| pe-swarm | tokio + libp2p mesh | Single-threaded coordinator | Agent traits always available |
| pe-ledger | QuDAG P2P | Local-only chain | LedgerWriter trait always available |
| pe-rvf | rvf-server, rvf-runtime | rvf-wasm | rvf-types, rvf-wire always available |
| pe-api | axum HTTP/WS | - | Not compiled for WASM |
| pe-wasm | - | wasm-bindgen entry | Not compiled for native |
| pe-cli | Binary entry | - | Not compiled for WASM |
| pe-stream | tokio streams | - | Not compiled for WASM |
| pe-solver | faer, rayon | ndarray only | Core solver traits always available |
| pe-chemistry | reqwest HTTP bridge | - | Not compiled for WASM |
| pe-governance | daa full | daa local | Governance traits always available |

## Consequences

### Positive
- WASM bundle stays small (< 10 MB) by excluding server-only dependencies
- CI can test both profiles in parallel without conflicts
- `no_std` pe-core guarantees domain logic works on Raspberry Pi, embedded, and WASM
- Feature flags documented in workspace Cargo.toml — single source of truth

### Negative
- Conditional compilation (`#[cfg(feature = "native")]`) adds visual noise to crates that span both targets
- Some logic duplication: native uses rayon for parallelism, WASM is single-threaded — parallel algorithms need two implementations
- Testing both profiles doubles CI time

### Build Commands

```bash
# Native (default)
cargo build --release --features native

# WASM
wasm-pack build crates/pe-wasm --target bundler \
    --out-dir ../../web/pkg \
    -- --no-default-features --features wasm

# Raspberry Pi
cargo build --release --target aarch64-unknown-linux-gnu --features native

# CI validation: ensure wasm doesn't pull native deps
cargo check --target wasm32-unknown-unknown --no-default-features --features wasm
```

## Compiler Flags

```toml
# .cargo/config.toml
[target.wasm32-unknown-unknown]
rustflags = ["-C", "opt-level=z", "-C", "lto=thin"]

[target.x86_64-unknown-linux-musl]
rustflags = ["-C", "opt-level=3", "-C", "target-cpu=native", "-C", "lto=fat"]

[target.aarch64-unknown-linux-gnu]
linker = "aarch64-linux-gnu-gcc"
rustflags = ["-C", "opt-level=3"]
```
