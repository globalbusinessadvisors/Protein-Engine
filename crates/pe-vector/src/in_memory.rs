use std::collections::{BTreeMap, HashMap};

use pe_core::Embedding320;
use uuid::Uuid;

use crate::error::VectorError;
use crate::meta::VariantMeta;
use crate::traits::{GraphStore, VectorStore};

/// Pure-Rust in-memory vector store using brute-force cosine similarity.
///
/// Serves as:
/// - WASM-target implementation (no native dependencies)
/// - Reference implementation for testing
/// - Serializable to VEC_SEG + INDEX_SEG byte formats
#[derive(Debug, Clone)]
pub struct InMemoryVectorStore {
    embeddings: BTreeMap<Uuid, Embedding320>,
    metadata: BTreeMap<Uuid, VariantMeta>,
}

impl Default for InMemoryVectorStore {
    fn default() -> Self {
        Self::new()
    }
}

impl InMemoryVectorStore {
    pub fn new() -> Self {
        Self {
            embeddings: BTreeMap::new(),
            metadata: BTreeMap::new(),
        }
    }

    /// Serialize the store to VEC_SEG bytes.
    ///
    /// Format: count (u32 BE) + for each entry: uuid (16 bytes) + embedding (320 × f32 LE)
    pub fn to_vec_seg(&self) -> Vec<u8> {
        let count = self.embeddings.len() as u32;
        let entry_size = 16 + 320 * 4; // uuid + f32 * 320
        let mut buf = Vec::with_capacity(4 + self.embeddings.len() * entry_size);

        buf.extend_from_slice(&count.to_be_bytes());
        for (id, emb) in &self.embeddings {
            buf.extend_from_slice(id.as_bytes());
            for &val in emb.as_slice() {
                buf.extend_from_slice(&val.to_le_bytes());
            }
        }
        buf
    }

    /// Serialize metadata to INDEX_SEG bytes.
    ///
    /// Format: count (u32 BE) + for each entry: uuid (16 bytes) + JSON-length (u32 BE) + JSON bytes
    pub fn to_index_seg(&self) -> Vec<u8> {
        let count = self.metadata.len() as u32;
        let mut buf = Vec::with_capacity(4 + self.metadata.len() * 256);

        buf.extend_from_slice(&count.to_be_bytes());
        for (id, meta) in &self.metadata {
            buf.extend_from_slice(id.as_bytes());
            let json = serde_json::to_vec(meta).expect("VariantMeta serialization cannot fail");
            buf.extend_from_slice(&(json.len() as u32).to_be_bytes());
            buf.extend_from_slice(&json);
        }
        buf
    }

    /// Deserialize from VEC_SEG bytes.
    pub fn from_vec_seg(data: &[u8]) -> Result<BTreeMap<Uuid, Embedding320>, VectorError> {
        let mut cursor = 0usize;
        let count = read_u32(data, &mut cursor)? as usize;
        let mut embeddings = BTreeMap::new();

        for _ in 0..count {
            let id = read_uuid(data, &mut cursor)?;
            let mut arr = [0.0f32; 320];
            for val in &mut arr {
                if cursor + 4 > data.len() {
                    return Err(VectorError::DeserializationFailed(
                        "unexpected end of VEC_SEG data".into(),
                    ));
                }
                *val = f32::from_le_bytes(
                    data[cursor..cursor + 4]
                        .try_into()
                        .expect("slice is 4 bytes"),
                );
                cursor += 4;
            }
            embeddings.insert(id, Embedding320::new(arr));
        }
        Ok(embeddings)
    }

    /// Deserialize from INDEX_SEG bytes.
    pub fn from_index_seg(data: &[u8]) -> Result<BTreeMap<Uuid, VariantMeta>, VectorError> {
        let mut cursor = 0usize;
        let count = read_u32(data, &mut cursor)? as usize;
        let mut metadata = BTreeMap::new();

        for _ in 0..count {
            let id = read_uuid(data, &mut cursor)?;
            let json_len = read_u32(data, &mut cursor)? as usize;
            if cursor + json_len > data.len() {
                return Err(VectorError::DeserializationFailed(
                    "unexpected end of INDEX_SEG data".into(),
                ));
            }
            let meta: VariantMeta =
                serde_json::from_slice(&data[cursor..cursor + json_len]).map_err(|e| {
                    VectorError::DeserializationFailed(format!("invalid metadata JSON: {e}"))
                })?;
            cursor += json_len;
            metadata.insert(id, meta);
        }
        Ok(metadata)
    }

    /// Reconstruct an InMemoryVectorStore from VEC_SEG + INDEX_SEG bytes.
    pub fn from_segments(vec_seg: &[u8], index_seg: &[u8]) -> Result<Self, VectorError> {
        let embeddings = Self::from_vec_seg(vec_seg)?;
        let metadata = Self::from_index_seg(index_seg)?;
        Ok(Self {
            embeddings,
            metadata,
        })
    }
}

impl VectorStore for InMemoryVectorStore {
    fn insert(
        &mut self,
        id: Uuid,
        embedding: Embedding320,
        meta: VariantMeta,
    ) -> Result<(), VectorError> {
        if self.embeddings.contains_key(&id) {
            return Err(VectorError::DuplicateEmbedding(id));
        }
        self.embeddings.insert(id, embedding);
        self.metadata.insert(id, meta);
        Ok(())
    }

    fn search_nearest(
        &self,
        query: &Embedding320,
        k: usize,
    ) -> Result<Vec<(Uuid, f32)>, VectorError> {
        if k == 0 {
            return Err(VectorError::InvalidK);
        }

        if self.embeddings.is_empty() {
            return Ok(Vec::new());
        }

        let mut scores: Vec<(Uuid, f32)> = self
            .embeddings
            .iter()
            .map(|(&id, emb)| (id, query.cosine_similarity(emb)))
            .collect();

        // Sort descending by similarity (highest first)
        scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scores.truncate(k);
        Ok(scores)
    }

    fn get_meta(&self, id: Uuid) -> Result<Option<VariantMeta>, VectorError> {
        Ok(self.metadata.get(&id).cloned())
    }

    fn count(&self) -> usize {
        self.embeddings.len()
    }
}

/// Pure-Rust in-memory graph store for protein interaction networks.
#[derive(Debug, Clone, Default)]
pub struct InMemoryGraphStore {
    adjacency: HashMap<Uuid, Vec<(Uuid, f32)>>,
}

impl InMemoryGraphStore {
    pub fn new() -> Self {
        Self::default()
    }
}

impl GraphStore for InMemoryGraphStore {
    fn add_edge(&mut self, from: Uuid, to: Uuid, weight: f32) -> Result<(), VectorError> {
        self.adjacency
            .entry(from)
            .or_default()
            .push((to, weight));
        self.adjacency
            .entry(to)
            .or_default()
            .push((from, weight));
        Ok(())
    }

    fn neighbors(&self, id: Uuid) -> Result<Vec<(Uuid, f32)>, VectorError> {
        Ok(self.adjacency.get(&id).cloned().unwrap_or_default())
    }
}

fn read_u32(data: &[u8], cursor: &mut usize) -> Result<u32, VectorError> {
    if *cursor + 4 > data.len() {
        return Err(VectorError::DeserializationFailed(
            "unexpected end of input reading u32".into(),
        ));
    }
    let bytes: [u8; 4] = data[*cursor..*cursor + 4]
        .try_into()
        .expect("slice is 4 bytes");
    *cursor += 4;
    Ok(u32::from_be_bytes(bytes))
}

fn read_uuid(data: &[u8], cursor: &mut usize) -> Result<Uuid, VectorError> {
    if *cursor + 16 > data.len() {
        return Err(VectorError::DeserializationFailed(
            "unexpected end of input reading UUID".into(),
        ));
    }
    let bytes: [u8; 16] = data[*cursor..*cursor + 16]
        .try_into()
        .expect("slice is 16 bytes");
    *cursor += 16;
    Ok(Uuid::from_bytes(bytes))
}
