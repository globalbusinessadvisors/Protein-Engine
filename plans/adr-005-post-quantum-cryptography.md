# ADR-005: Post-Quantum Cryptography with ML-DSA and ML-KEM

**Status:** Accepted
**Date:** 2026-03-13
**Deciders:** Platform Architecture Team
**Relates to:** FR-07, NFR-04, NFR-06

---

## Context

Protein-Engine produces high-value intellectual property: novel protein variant designs, experimental results linking sequences to biological function, and model weights trained on proprietary data. This data must be:

1. **Tamper-evident**: Any modification to the experiment log or design history must be detectable
2. **Authenticity-proven**: Every entry must be attributable to a specific researcher or compute node
3. **Future-proof**: Signatures must remain secure against quantum computing attacks (Shor's algorithm breaks RSA/ECDSA)

NIST finalized post-quantum standards in 2024:
- **ML-DSA** (Module-Lattice Digital Signature Algorithm, formerly CRYSTALS-Dilithium): Post-quantum digital signatures
- **ML-KEM** (Module-Lattice Key Encapsulation Mechanism, formerly CRYSTALS-Kyber): Post-quantum key exchange

## Decision

**All cryptographic signing uses ML-DSA. All key encapsulation uses ML-KEM. Classical algorithms (RSA, ECDSA, X25519) are not used anywhere in the platform.** Hashing uses SHA3 (Keccak).

Implementation via Rust crates:
- `pqcrypto-mldsa` (v0.2) for signatures
- `pqcrypto-mlkem` (v0.2) for key encapsulation
- `sha3` (v0.10) for hashing

## Where Crypto Is Applied

| Location | Algorithm | Purpose |
|----------|-----------|---------|
| JOURNAL_SEG entries | SHA3-256 hash chain | Each entry includes hash of previous entry — tamper-evident append-only log |
| WITNESS_SEG | ML-DSA signature per entry | Every scoring run, experiment result, model update is signed |
| MANIFEST_SEG | ML-DSA signature | Root manifest is signed; `parent_hash` links to parent .rvf |
| CRYPTO_SEG | TEE attestation + ML-DSA | Proves quantum calculations ran inside verified secure enclave |
| QuDAG P2P | ML-KEM key encapsulation | Peer-to-peer data exchange encrypted with post-quantum keys |
| QuDAG transactions | ML-DSA signatures | Every distributed ledger transaction is signed |

## Rationale

- **NIST standardized**: ML-DSA and ML-KEM are the primary NIST PQC standards — maximum interoperability and long-term trust
- **Quantum relevance**: A platform that routes jobs to actual quantum computers should not be vulnerable to quantum attacks on its own cryptography
- **No hybrid complexity**: By going post-quantum only (no classical fallback), we avoid the complexity of hybrid signature schemes
- **Performance acceptable**: ML-DSA-65 signing is ~2ms, verification ~0.8ms; ML-KEM-768 encapsulation ~0.1ms — negligible compared to protein scoring

## Consequences

### Positive
- Experiment log is provably tamper-evident across the lifetime of the platform
- No migration needed when quantum computers become cryptographically relevant
- WITNESS_SEG provides a court-admissible audit trail for IP disputes
- Post-quantum P2P means distributed lab networks are secure against future attacks

### Negative
- ML-DSA signatures are larger than ECDSA (~2.4 KB vs 64 bytes) — increases WITNESS_SEG size
- `pqcrypto-*` crates are C FFI wrappers — adds C compilation requirement to build chain
- No WASM support for `pqcrypto-*` out of the box — WASM builds use verification-only (verify signatures, cannot create new ones) or a pure-Rust fallback
- Key management (generation, storage, rotation) is a new operational concern

### WASM Constraint

The `pqcrypto-mldsa` crate links to C code that does not compile to wasm32. For the WASM target:
- **Signature verification**: Use a pure-Rust ML-DSA verifier (or pre-verify on native before packaging)
- **No signing in browser**: Browser clients verify WITNESS_SEG integrity but cannot create new signed entries
- **Signing requires native**: All signing operations happen on native server/CLI nodes

This is acceptable because the browser is a read/explore/score interface, not an authoritative data source.

## CryptoSigner Trait (from ADR-004)

```rust
#[automock]
pub trait CryptoSigner: Send + Sync {
    fn sign(&self, data: &[u8]) -> Result<MlDsaSignature>;
    fn verify(&self, data: &[u8], sig: &MlDsaSignature) -> Result<bool>;
}
```

Tests use `MockSigner` returning deterministic signatures, avoiding the 2ms real signing cost in unit tests.
