use pe_core::{AminoAcidSequence, Embedding320, YamanakaFactor};
use uuid::Uuid;

use crate::error::VectorError;
use crate::in_memory::{InMemoryGraphStore, InMemoryVectorStore};
use crate::meta::{DesignMethod, VariantMeta};
use crate::traits::{EmbeddingModel, GraphStore, MockEmbeddingModel, VectorStore};

// ────────────────────────────────────────────────────────────────────
// Helpers
// ────────────────────────────────────────────────────────────────────

fn make_embedding(seed: f32) -> Embedding320 {
    let mut arr = [0.0f32; 320];
    for (i, val) in arr.iter_mut().enumerate() {
        *val = ((i as f32) * seed).sin();
    }
    Embedding320::new(arr)
}

fn make_unit_embedding(dim: usize) -> Embedding320 {
    let mut arr = [0.0f32; 320];
    if dim < 320 {
        arr[dim] = 1.0;
    }
    Embedding320::new(arr)
}

fn make_meta(id: Uuid) -> VariantMeta {
    VariantMeta {
        variant_id: id,
        target_factor: YamanakaFactor::OCT4,
        generation: 0,
        composite_score: Some(0.85),
        design_method: DesignMethod::WildType,
    }
}

// ────────────────────────────────────────────────────────────────────
// EmbeddingModel mock tests
// ────────────────────────────────────────────────────────────────────

#[test]
fn mock_embedding_model_returns_fixed_vector() {
    let expected = make_embedding(1.0);
    let expected_clone = expected.clone();

    let mut mock = MockEmbeddingModel::new();
    mock.expect_embed()
        .returning(move |_| Ok(expected_clone.clone()));

    let seq = AminoAcidSequence::new("ACDEFGHIKL").unwrap();
    let result = mock.embed(&seq).unwrap();
    assert_eq!(result, expected);
}

#[test]
fn mock_embedding_model_can_return_error() {
    let mut mock = MockEmbeddingModel::new();
    mock.expect_embed()
        .returning(|_| Err(VectorError::EmbeddingFailed("test error".into())));

    let seq = AminoAcidSequence::new("ACDEFGHIKL").unwrap();
    let result = mock.embed(&seq);
    assert!(result.is_err());
}

// ────────────────────────────────────────────────────────────────────
// InMemoryVectorStore tests
// ────────────────────────────────────────────────────────────────────

#[test]
fn insert_and_retrieve_by_id() {
    let mut store = InMemoryVectorStore::new();
    let id = Uuid::new_v4();
    let emb = make_embedding(1.0);
    let meta = make_meta(id);

    store.insert(id, emb, meta.clone()).unwrap();

    let retrieved = store.get_meta(id).unwrap();
    assert_eq!(retrieved, Some(meta));
}

#[test]
fn get_meta_returns_none_for_unknown_id() {
    let store = InMemoryVectorStore::new();
    let result = store.get_meta(Uuid::new_v4()).unwrap();
    assert_eq!(result, None);
}

#[test]
fn count_reflects_inserted_embeddings() {
    let mut store = InMemoryVectorStore::new();
    assert_eq!(store.count(), 0);

    for i in 0..5 {
        let id = Uuid::new_v4();
        store
            .insert(id, make_embedding(i as f32), make_meta(id))
            .unwrap();
    }
    assert_eq!(store.count(), 5);
}

#[test]
fn duplicate_insert_rejected() {
    let mut store = InMemoryVectorStore::new();
    let id = Uuid::new_v4();
    let emb = make_embedding(1.0);

    store.insert(id, emb.clone(), make_meta(id)).unwrap();
    let result = store.insert(id, emb, make_meta(id));
    assert!(matches!(result, Err(VectorError::DuplicateEmbedding(_))));
}

#[test]
fn search_nearest_returns_empty_for_empty_store() {
    let store = InMemoryVectorStore::new();
    let query = make_embedding(1.0);
    let results = store.search_nearest(&query, 5).unwrap();
    assert!(results.is_empty());
}

#[test]
fn search_nearest_rejects_zero_k() {
    let store = InMemoryVectorStore::new();
    let query = make_embedding(1.0);
    let result = store.search_nearest(&query, 0);
    assert!(matches!(result, Err(VectorError::InvalidK)));
}

#[test]
fn search_nearest_returns_k_closest_by_cosine_similarity() {
    let mut store = InMemoryVectorStore::new();

    // Insert 100 known vectors using unit vectors in different dimensions.
    // This gives us perfect control over cosine similarity:
    // cosine_similarity(unit(i), unit(j)) = 1.0 if i==j, 0.0 otherwise
    let mut ids = Vec::new();
    for dim in 0..100 {
        let id = Uuid::new_v4();
        ids.push(id);
        store
            .insert(id, make_unit_embedding(dim), make_meta(id))
            .unwrap();
    }

    // Query with the same unit vector as ids[42]
    let query = make_unit_embedding(42);
    let results = store.search_nearest(&query, 5).unwrap();

    assert_eq!(results.len(), 5);

    // The first result must be the exact match (similarity = 1.0)
    assert_eq!(results[0].0, ids[42]);
    assert!((results[0].1 - 1.0).abs() < 1e-6);

    // The remaining results should have similarity 0.0 (orthogonal)
    for &(_, sim) in &results[1..] {
        assert!(sim.abs() < 1e-6);
    }
}

#[test]
fn search_nearest_results_sorted_descending() {
    let mut store = InMemoryVectorStore::new();

    // Create embeddings with known similarity ordering.
    // Use a query that's a scaled version along dim 0.
    let query = make_unit_embedding(0);

    // Insert vectors with decreasing similarity to query:
    // embed_a = [1, 0, 0, ...] → sim = 1.0
    // embed_b = [0.5, 0.866, 0, ...] → sim = 0.5
    // embed_c = [0, 1, 0, ...] → sim = 0.0
    let id_a = Uuid::new_v4();
    let mut arr_a = [0.0f32; 320];
    arr_a[0] = 1.0;
    store
        .insert(id_a, Embedding320::new(arr_a), make_meta(id_a))
        .unwrap();

    let id_b = Uuid::new_v4();
    let mut arr_b = [0.0f32; 320];
    arr_b[0] = 0.5;
    arr_b[1] = 0.866;
    store
        .insert(id_b, Embedding320::new(arr_b), make_meta(id_b))
        .unwrap();

    let id_c = Uuid::new_v4();
    let mut arr_c = [0.0f32; 320];
    arr_c[1] = 1.0;
    store
        .insert(id_c, Embedding320::new(arr_c), make_meta(id_c))
        .unwrap();

    let results = store.search_nearest(&query, 3).unwrap();
    assert_eq!(results.len(), 3);
    assert_eq!(results[0].0, id_a);
    assert_eq!(results[1].0, id_b);
    assert_eq!(results[2].0, id_c);

    // Verify ordering
    assert!(results[0].1 >= results[1].1);
    assert!(results[1].1 >= results[2].1);
}

#[test]
fn search_nearest_returns_fewer_than_k_if_store_smaller() {
    let mut store = InMemoryVectorStore::new();

    let id = Uuid::new_v4();
    store
        .insert(id, make_embedding(1.0), make_meta(id))
        .unwrap();

    let results = store.search_nearest(&make_embedding(1.0), 10).unwrap();
    assert_eq!(results.len(), 1);
}

// ────────────────────────────────────────────────────────────────────
// Segment serialization round-trip
// ────────────────────────────────────────────────────────────────────

#[test]
fn vec_seg_index_seg_round_trip_preserves_search_results() {
    let mut store = InMemoryVectorStore::new();

    // Insert several embeddings
    let mut ids = Vec::new();
    for dim in 0..10 {
        let id = Uuid::new_v4();
        ids.push(id);
        let meta = VariantMeta {
            variant_id: id,
            target_factor: YamanakaFactor::SOX2,
            generation: dim as u32,
            composite_score: Some(dim as f64 / 10.0),
            design_method: DesignMethod::Mutation,
        };
        store
            .insert(id, make_unit_embedding(dim), meta)
            .unwrap();
    }

    // Serialize
    let vec_seg = store.to_vec_seg();
    let index_seg = store.to_index_seg();

    // Deserialize into new store
    let restored = InMemoryVectorStore::from_segments(&vec_seg, &index_seg).unwrap();

    assert_eq!(restored.count(), store.count());

    // Verify metadata round-trip
    for &id in &ids {
        let orig_meta = store.get_meta(id).unwrap();
        let rest_meta = restored.get_meta(id).unwrap();
        assert_eq!(orig_meta, rest_meta);
    }

    // Verify search results match
    let query = make_unit_embedding(5);
    let orig_results = store.search_nearest(&query, 3).unwrap();
    let rest_results = restored.search_nearest(&query, 3).unwrap();
    assert_eq!(orig_results.len(), rest_results.len());
    for (orig, rest) in orig_results.iter().zip(rest_results.iter()) {
        assert_eq!(orig.0, rest.0);
        assert!((orig.1 - rest.1).abs() < 1e-6);
    }
}

#[test]
fn vec_seg_round_trip_empty_store() {
    let store = InMemoryVectorStore::new();
    let vec_seg = store.to_vec_seg();
    let index_seg = store.to_index_seg();

    let restored = InMemoryVectorStore::from_segments(&vec_seg, &index_seg).unwrap();
    assert_eq!(restored.count(), 0);
}

#[test]
fn deserialization_rejects_truncated_vec_seg() {
    let mut store = InMemoryVectorStore::new();
    let id = Uuid::new_v4();
    store
        .insert(id, make_embedding(1.0), make_meta(id))
        .unwrap();

    let vec_seg = store.to_vec_seg();
    // Truncate in the middle
    let truncated = &vec_seg[..vec_seg.len() / 2];
    let result = InMemoryVectorStore::from_vec_seg(truncated);
    assert!(result.is_err());
}

// ────────────────────────────────────────────────────────────────────
// GraphStore tests
// ────────────────────────────────────────────────────────────────────

#[test]
fn graph_store_add_and_query_neighbors() {
    let mut graph = InMemoryGraphStore::new();
    let a = Uuid::new_v4();
    let b = Uuid::new_v4();
    let c = Uuid::new_v4();

    graph.add_edge(a, b, 0.9).unwrap();
    graph.add_edge(a, c, 0.5).unwrap();

    let neighbors = graph.neighbors(a).unwrap();
    assert_eq!(neighbors.len(), 2);

    // b should see a as neighbor (bidirectional)
    let b_neighbors = graph.neighbors(b).unwrap();
    assert_eq!(b_neighbors.len(), 1);
    assert_eq!(b_neighbors[0].0, a);
}

#[test]
fn graph_store_returns_empty_for_unknown_node() {
    let graph = InMemoryGraphStore::new();
    let neighbors = graph.neighbors(Uuid::new_v4()).unwrap();
    assert!(neighbors.is_empty());
}
