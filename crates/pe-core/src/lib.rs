//! pe-core: Universal domain types for the Protein-Engine platform.
//!
//! This crate is `no_std` compatible and compiles on all targets:
//! native, WASM, aarch64, and embedded.

#![no_std]
extern crate alloc;

pub mod sequence;
pub mod variant;
pub mod fitness;
pub mod experiment;
pub mod embedding;

#[cfg(test)]
mod tests;

// Re-export all public types at crate root for convenience
pub use sequence::{AminoAcid, AminoAcidSequence, Mutation, YamanakaFactor, CoreError};
pub use variant::ProteinVariant;
pub use fitness::{FitnessScore, FitnessWeights, ScoredVariant};
pub use experiment::{AssayType, ExperimentResult};
pub use embedding::Embedding320;
