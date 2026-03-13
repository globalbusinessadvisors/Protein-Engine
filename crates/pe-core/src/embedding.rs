//! Fixed-dimension protein embedding vector.

use alloc::vec::Vec;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// 320-dimensional embedding vector (ESM-2 per-residue dimension).
#[derive(Clone, Debug, PartialEq)]
pub struct Embedding320([f32; 320]);

impl Serialize for Embedding320 {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.0.as_slice().serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Embedding320 {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let v: Vec<f32> = Vec::deserialize(deserializer)?;
        let arr: [f32; 320] = v
            .try_into()
            .map_err(|_| serde::de::Error::custom("expected 320 f32 values"))?;
        Ok(Embedding320(arr))
    }
}

impl Embedding320 {
    /// Wrap a raw array into an embedding.
    pub fn new(data: [f32; 320]) -> Self {
        Self(data)
    }

    /// All-zeros embedding.
    pub fn zeros() -> Self {
        Self([0.0_f32; 320])
    }

    /// View the embedding as a slice.
    pub fn as_slice(&self) -> &[f32] {
        &self.0
    }

    /// Dimensionality (compile-time constant).
    pub fn dim() -> usize {
        320
    }

    /// Dot product with another embedding.
    pub fn dot(&self, other: &Self) -> f32 {
        self.0
            .iter()
            .zip(other.0.iter())
            .map(|(a, b)| a * b)
            .sum()
    }

    /// L2 (Euclidean) norm.
    pub fn norm(&self) -> f32 {
        self.dot(self).sqrt()
    }

    /// Cosine similarity in [-1.0, 1.0].
    ///
    /// Returns 0.0 when either vector has zero norm, avoiding division by zero.
    pub fn cosine_similarity(&self, other: &Self) -> f32 {
        let norm_a = self.norm();
        let norm_b = other.norm();
        if norm_a == 0.0 || norm_b == 0.0 {
            return 0.0;
        }
        self.dot(other) / (norm_a * norm_b)
    }
}
