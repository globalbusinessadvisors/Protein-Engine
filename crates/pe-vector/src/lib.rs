//! pe-vector: Vector storage and embedding management for protein sequences.
//!
//! Provides `EmbeddingModel` and `VectorStore` traits with an in-memory
//! implementation and optional RuVector-backed HNSW nearest-neighbor search.

pub mod error;
pub mod in_memory;
pub mod meta;
pub mod traits;

pub use error::VectorError;
pub use in_memory::{InMemoryGraphStore, InMemoryVectorStore};
pub use meta::{DesignMethod, VariantMeta};
pub use traits::{EmbeddingModel, GraphStore, VectorStore};

#[cfg(test)]
mod tests;
