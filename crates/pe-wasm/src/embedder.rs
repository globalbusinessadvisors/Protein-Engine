//! Hash-based embedding model for WASM (no ESM-2 available in browser).
//!
//! Produces deterministic 320-dim embeddings from amino acid sequences
//! using a simple hash-based scheme. Same sequence always yields the
//! same embedding, enabling meaningful cosine-similarity comparisons.

use pe_core::{AminoAcidSequence, Embedding320};
use pe_vector::VectorError;
use pe_vector::traits::EmbeddingModel;

/// Deterministic hash-based embedder for browser use.
///
/// Maps each amino acid position to a region of the 320-dim vector,
/// producing consistent embeddings without neural network inference.
pub struct HashEmbedder;

impl HashEmbedder {
    pub fn new() -> Self {
        Self
    }
}

impl Default for HashEmbedder {
    fn default() -> Self {
        Self::new()
    }
}

impl EmbeddingModel for HashEmbedder {
    fn embed(&self, sequence: &AminoAcidSequence) -> Result<Embedding320, VectorError> {
        let mut data = [0.0f32; 320];
        let residues = sequence.as_slice();

        for (i, &aa) in residues.iter().enumerate() {
            // Map each residue to a hash-derived contribution across the vector.
            // Use the amino acid's char code and position for determinism.
            let aa_code = aa.to_char() as u32;
            let seed = aa_code.wrapping_mul(2654435761).wrapping_add(i as u32);

            // Distribute across 4 dimensions per residue position
            for j in 0..4 {
                let idx = (i * 4 + j) % 320;
                let hash = seed.wrapping_mul((j as u32 + 1).wrapping_mul(0x9E3779B9));
                // Map to [-1, 1] range
                let val = (hash as f32 / u32::MAX as f32) * 2.0 - 1.0;
                data[idx] += val;
            }
        }

        // Normalize to unit length for meaningful cosine similarity
        let norm: f32 = data.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm > 1e-8 {
            for val in &mut data {
                *val /= norm;
            }
        }

        Ok(Embedding320::new(data))
    }
}
