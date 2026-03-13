use pe_core::{AminoAcidSequence, Embedding320};
use uuid::Uuid;

use crate::error::VectorError;
use crate::meta::VariantMeta;

/// Transforms an amino acid sequence into a 320-dim embedding vector.
///
/// In production, wraps ESM-2 (via candle). In tests, mocked to return
/// deterministic fixture vectors.
#[cfg_attr(test, mockall::automock)]
pub trait EmbeddingModel: Send + Sync {
    fn embed(&self, sequence: &AminoAcidSequence) -> Result<Embedding320, VectorError>;
}

/// Stores embeddings with metadata and provides nearest-neighbor search.
///
/// Implementations: `InMemoryVectorStore` (brute-force, WASM-safe),
/// `RuVectorStore` (HNSW, native-only).
#[cfg_attr(test, mockall::automock)]
pub trait VectorStore: Send + Sync {
    fn insert(
        &mut self,
        id: Uuid,
        embedding: Embedding320,
        meta: VariantMeta,
    ) -> Result<(), VectorError>;

    /// Returns the `k` nearest neighbors as (id, cosine_similarity) pairs,
    /// sorted descending by similarity.
    fn search_nearest(
        &self,
        query: &Embedding320,
        k: usize,
    ) -> Result<Vec<(Uuid, f32)>, VectorError>;

    fn get_meta(&self, id: Uuid) -> Result<Option<VariantMeta>, VectorError>;

    fn count(&self) -> usize;
}

/// Graph-based storage for protein interaction networks (GRAPH_SEG).
#[cfg_attr(test, mockall::automock)]
pub trait GraphStore: Send + Sync {
    fn add_edge(&mut self, from: Uuid, to: Uuid, weight: f32) -> Result<(), VectorError>;
    fn neighbors(&self, id: Uuid) -> Result<Vec<(Uuid, f32)>, VectorError>;
}
