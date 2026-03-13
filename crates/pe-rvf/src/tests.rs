use chrono::Utc;

use crate::builder::RvfBuilder;
use crate::capability::Capability;
use crate::error::RvfError;
use crate::manifest::Manifest;
use crate::rvf_file::RvfFile;
use crate::segment::SegmentType;
use crate::traits::{MockSegmentProducer, RvfAssembler, SegmentProducer};

// ────────────────────────────────────────────────────────────────────
// Helpers
// ────────────────────────────────────────────────────────────────────

fn test_manifest() -> Manifest {
    Manifest::new(
        "protein-engine".into(),
        "0.1.0".into(),
        None,
        None,
        Utc::now(),
    )
    .unwrap()
}

fn test_manifest_with_parent(parent_hash: [u8; 32]) -> Manifest {
    Manifest::new(
        "protein-engine".into(),
        "0.1.0".into(),
        Some(parent_hash),
        None,
        Utc::now(),
    )
    .unwrap()
}

fn build_minimal_rvf() -> RvfFile {
    let mut builder = RvfBuilder::new();
    builder.set_manifest(test_manifest());
    builder
        .add_segment(SegmentType::ManifestSeg, b"manifest-data".to_vec())
        .unwrap();
    builder.build().unwrap()
}

fn mock_producer(seg_type: SegmentType, data: Vec<u8>) -> MockSegmentProducer {
    let mut mock = MockSegmentProducer::new();
    mock.expect_segment_type()
        .returning(move || seg_type);
    mock.expect_produce()
        .returning(move || Ok(data.clone()));
    mock
}

// ────────────────────────────────────────────────────────────────────
// SegmentType tests
// ────────────────────────────────────────────────────────────────────

#[test]
fn segment_type_discriminants_match_adr001() {
    assert_eq!(SegmentType::ManifestSeg.as_u8(), 0x00);
    assert_eq!(SegmentType::VecSeg.as_u8(), 0x01);
    assert_eq!(SegmentType::IndexSeg.as_u8(), 0x02);
    assert_eq!(SegmentType::OverlaySeg.as_u8(), 0x03);
    assert_eq!(SegmentType::JournalSeg.as_u8(), 0x04);
    assert_eq!(SegmentType::GraphSeg.as_u8(), 0x05);
    assert_eq!(SegmentType::QuantSeg.as_u8(), 0x06);
    assert_eq!(SegmentType::MetaSeg.as_u8(), 0x07);
    assert_eq!(SegmentType::HotSeg.as_u8(), 0x08);
    assert_eq!(SegmentType::SketchSeg.as_u8(), 0x09);
    assert_eq!(SegmentType::WasmSeg.as_u8(), 0x0A);
    assert_eq!(SegmentType::WitnessSeg.as_u8(), 0x0B);
    assert_eq!(SegmentType::CryptoSeg.as_u8(), 0x0C);
    assert_eq!(SegmentType::MetaIdxSeg.as_u8(), 0x0D);
    assert_eq!(SegmentType::KernelSeg.as_u8(), 0x0E);
}

#[test]
fn segment_type_all_has_15_entries() {
    assert_eq!(SegmentType::ALL.len(), 15);
}

#[test]
fn segment_type_round_trip_u8() {
    for seg in SegmentType::ALL {
        assert_eq!(SegmentType::from_u8(seg.as_u8()), Some(seg));
    }
}

#[test]
fn segment_type_from_u8_invalid_returns_none() {
    assert_eq!(SegmentType::from_u8(0x0F), None);
    assert_eq!(SegmentType::from_u8(0xFF), None);
}

#[test]
fn segment_type_ordering_matches_discriminant() {
    let mut sorted = SegmentType::ALL.to_vec();
    sorted.sort();
    assert_eq!(sorted, SegmentType::ALL.to_vec());
}

// ────────────────────────────────────────────────────────────────────
// Manifest tests (RF-7)
// ────────────────────────────────────────────────────────────────────

#[test]
fn manifest_new_valid() {
    let m = Manifest::new(
        "test".into(),
        "1.0.0".into(),
        None,
        None,
        Utc::now(),
    );
    assert!(m.is_ok());
    let m = m.unwrap();
    assert_eq!(m.name, "test");
    assert!(m.capabilities.is_empty());
}

#[test]
fn manifest_rejects_empty_name() {
    let result = Manifest::new("".into(), "1.0.0".into(), None, None, Utc::now());
    assert!(matches!(result, Err(RvfError::EmptyName)));
}

#[test]
fn manifest_rejects_empty_version() {
    let result = Manifest::new("test".into(), "".into(), None, None, Utc::now());
    assert!(matches!(result, Err(RvfError::EmptyVersion)));
}

#[test]
fn manifest_accepts_parent_hash() {
    let hash = [0xABu8; 32];
    let m = Manifest::new("test".into(), "1.0".into(), Some(hash), None, Utc::now()).unwrap();
    assert_eq!(m.parent_hash, Some(hash));
}

// ────────────────────────────────────────────────────────────────────
// SegmentProducer mock tests
// ────────────────────────────────────────────────────────────────────

#[test]
fn mock_segment_producer_returns_fixture_bytes() {
    let producer = mock_producer(SegmentType::VecSeg, vec![1, 2, 3]);
    assert_eq!(producer.segment_type(), SegmentType::VecSeg);
    assert_eq!(producer.produce().unwrap(), vec![1, 2, 3]);
}

#[test]
fn mock_segment_producer_used_in_builder() {
    let producer = mock_producer(SegmentType::ManifestSeg, b"manifest-data".to_vec());
    let seg_type = producer.segment_type();
    let data = producer.produce().unwrap();

    let mut builder = RvfBuilder::new();
    builder.set_manifest(test_manifest());
    builder.add_segment(seg_type, data).unwrap();
    let rvf = builder.build().unwrap();

    assert!(rvf.segments().contains_key(&SegmentType::ManifestSeg));
}

// ────────────────────────────────────────────────────────────────────
// RvfBuilder / RvfFile invariant tests
// ────────────────────────────────────────────────────────────────────

#[test]
fn build_with_all_segments_produces_valid_rvf_file() {
    let mut builder = RvfBuilder::new();
    builder.set_manifest(test_manifest());

    for seg in SegmentType::ALL {
        builder
            .add_segment(seg, format!("data-{:02X}", seg.as_u8()).into_bytes())
            .unwrap();
    }

    let rvf = builder.build().unwrap();
    assert_eq!(rvf.segments().len(), 15);
    assert!(!rvf.manifest().capabilities.is_empty());
}

#[test]
fn build_fails_without_manifest_seg_rf1() {
    let mut builder = RvfBuilder::new();
    builder.set_manifest(test_manifest());
    // Add a non-manifest segment but not ManifestSeg
    builder
        .add_segment(SegmentType::VecSeg, b"vec-data".to_vec())
        .unwrap();

    let result = builder.build();
    assert!(matches!(result, Err(RvfError::MissingManifest)));
}

#[test]
fn build_fails_without_manifest_set() {
    let mut builder = RvfBuilder::new();
    builder
        .add_segment(SegmentType::ManifestSeg, b"data".to_vec())
        .unwrap();

    let result = builder.build();
    assert!(matches!(result, Err(RvfError::ManifestNotSet)));
}

#[test]
fn capabilities_auto_populated_from_present_segments_rf2_rf3_rf4() {
    let mut builder = RvfBuilder::new();
    builder.set_manifest(test_manifest());
    builder
        .add_segment(SegmentType::ManifestSeg, b"manifest".to_vec())
        .unwrap();
    builder
        .add_segment(SegmentType::VecSeg, b"vec-data".to_vec())
        .unwrap();
    builder
        .add_segment(SegmentType::WasmSeg, b"wasm-data".to_vec())
        .unwrap();
    builder
        .add_segment(SegmentType::SketchSeg, b"sketch-data".to_vec())
        .unwrap();
    builder
        .add_segment(SegmentType::CryptoSeg, b"crypto-data".to_vec())
        .unwrap();

    let rvf = builder.build().unwrap();
    let caps = &rvf.manifest().capabilities;

    // RF-4: VEC_SEG → VecSearch
    assert!(caps.contains(&Capability::VecSearch));
    // RF-3: WASM_SEG → WasmRuntime
    assert!(caps.contains(&Capability::WasmRuntime));
    // SKETCH_SEG → QuantumVqe
    assert!(caps.contains(&Capability::QuantumVqe));
    // CRYPTO_SEG → TeeAttestation
    assert!(caps.contains(&Capability::TeeAttestation));
}

#[test]
fn capabilities_not_present_when_segment_absent() {
    let rvf = build_minimal_rvf();
    let caps = &rvf.manifest().capabilities;
    assert!(!caps.contains(&Capability::VecSearch));
    assert!(!caps.contains(&Capability::WasmRuntime));
}

#[test]
fn segments_ordered_by_type_id_in_output_rf5() {
    let mut builder = RvfBuilder::new();
    builder.set_manifest(test_manifest());

    // Add in reverse order to verify ordering
    builder
        .add_segment(SegmentType::KernelSeg, b"kernel".to_vec())
        .unwrap();
    builder
        .add_segment(SegmentType::WasmSeg, b"wasm".to_vec())
        .unwrap();
    builder
        .add_segment(SegmentType::VecSeg, b"vec".to_vec())
        .unwrap();
    builder
        .add_segment(SegmentType::ManifestSeg, b"manifest".to_vec())
        .unwrap();

    let rvf = builder.build().unwrap();
    let keys: Vec<u8> = rvf.segments().keys().map(|s| s.as_u8()).collect();

    // Must be sorted ascending
    let mut sorted = keys.clone();
    sorted.sort();
    assert_eq!(keys, sorted);
}

#[test]
fn parent_hash_links_child_to_parent() {
    let parent = build_minimal_rvf();
    let parent_hash = *parent.file_hash();

    let mut builder = RvfBuilder::new();
    builder.set_manifest(test_manifest_with_parent(parent_hash));
    builder
        .add_segment(SegmentType::ManifestSeg, b"child-manifest".to_vec())
        .unwrap();

    let child = builder.build().unwrap();
    assert_eq!(child.manifest().parent_hash, Some(parent_hash));
    assert_ne!(child.file_hash(), parent.file_hash());
}

#[test]
fn file_hash_is_deterministic_for_same_inputs_rf6() {
    let ts = Utc::now();

    let build_rvf = || {
        let manifest = Manifest::new(
            "test".into(),
            "1.0.0".into(),
            None,
            None,
            ts,
        )
        .unwrap();

        let mut builder = RvfBuilder::new();
        builder.set_manifest(manifest);
        builder
            .add_segment(SegmentType::ManifestSeg, b"manifest-data".to_vec())
            .unwrap();
        builder
            .add_segment(SegmentType::VecSeg, b"vec-data".to_vec())
            .unwrap();
        builder.build().unwrap()
    };

    let rvf1 = build_rvf();
    let rvf2 = build_rvf();

    assert_eq!(rvf1.file_hash(), rvf2.file_hash());
}

#[test]
fn file_hash_changes_with_different_data() {
    let ts = Utc::now();

    let manifest = Manifest::new("test".into(), "1.0.0".into(), None, None, ts).unwrap();
    let mut builder1 = RvfBuilder::new();
    builder1.set_manifest(manifest.clone());
    builder1
        .add_segment(SegmentType::ManifestSeg, b"data-a".to_vec())
        .unwrap();
    let rvf1 = builder1.build().unwrap();

    let mut builder2 = RvfBuilder::new();
    builder2.set_manifest(manifest);
    builder2
        .add_segment(SegmentType::ManifestSeg, b"data-b".to_vec())
        .unwrap();
    let rvf2 = builder2.build().unwrap();

    assert_ne!(rvf1.file_hash(), rvf2.file_hash());
}

#[test]
fn round_trip_serialize_deserialize_preserves_all_segments() {
    let ts = Utc::now();
    let manifest = Manifest::new(
        "protein-engine".into(),
        "0.1.0".into(),
        Some([0xAA; 32]),
        Some([0xBB; 32]),
        ts,
    )
    .unwrap();

    let mut builder = RvfBuilder::new();
    builder.set_manifest(manifest);
    builder
        .add_segment(SegmentType::ManifestSeg, b"manifest-bytes".to_vec())
        .unwrap();
    builder
        .add_segment(SegmentType::VecSeg, b"vec-bytes".to_vec())
        .unwrap();
    builder
        .add_segment(SegmentType::WasmSeg, b"wasm-bytes".to_vec())
        .unwrap();
    builder
        .add_segment(SegmentType::KernelSeg, b"kernel-bytes".to_vec())
        .unwrap();

    let original = builder.build().unwrap();
    let serialized = original.serialize();
    let restored = RvfFile::deserialize(&serialized).unwrap();

    assert_eq!(original.manifest().name, restored.manifest().name);
    assert_eq!(original.manifest().version, restored.manifest().version);
    assert_eq!(
        original.manifest().parent_hash,
        restored.manifest().parent_hash
    );
    assert_eq!(
        original.manifest().signing_key_fingerprint,
        restored.manifest().signing_key_fingerprint
    );
    assert_eq!(
        original.manifest().capabilities,
        restored.manifest().capabilities
    );
    assert_eq!(original.segments().len(), restored.segments().len());
    for (seg_type, data) in original.segments() {
        assert_eq!(restored.segments().get(seg_type).unwrap(), data);
    }
    assert_eq!(original.file_hash(), restored.file_hash());
}

#[test]
fn round_trip_empty_segments() {
    let mut builder = RvfBuilder::new();
    builder.set_manifest(test_manifest());
    builder
        .add_segment(SegmentType::ManifestSeg, Vec::new())
        .unwrap();

    let original = builder.build().unwrap();
    let serialized = original.serialize();
    let restored = RvfFile::deserialize(&serialized).unwrap();
    assert_eq!(
        restored.segments().get(&SegmentType::ManifestSeg).unwrap(),
        &Vec::<u8>::new()
    );
}

#[test]
fn deserialize_rejects_truncated_data() {
    let rvf = build_minimal_rvf();
    let serialized = rvf.serialize();
    // Truncate in the middle
    let truncated = &serialized[..serialized.len() / 2];
    let result = RvfFile::deserialize(truncated);
    assert!(result.is_err());
}

#[test]
fn deserialize_rejects_invalid_segment_type() {
    // Craft data with invalid segment type
    let manifest = test_manifest();
    let manifest_bytes = serde_json::to_vec(&manifest).unwrap();
    let mut data = Vec::new();
    data.extend_from_slice(&(manifest_bytes.len() as u32).to_be_bytes());
    data.extend_from_slice(&manifest_bytes);
    // Invalid segment type 0xFF
    data.push(0xFF);
    data.extend_from_slice(&4u32.to_be_bytes());
    data.extend_from_slice(b"test");

    let result = RvfFile::deserialize(&data);
    assert!(result.is_err());
}

#[test]
fn duplicate_segment_rejected() {
    let mut builder = RvfBuilder::new();
    builder.set_manifest(test_manifest());
    builder
        .add_segment(SegmentType::ManifestSeg, b"data1".to_vec())
        .unwrap();
    let result = builder.add_segment(SegmentType::ManifestSeg, b"data2".to_vec());
    assert!(matches!(result, Err(RvfError::DuplicateSegment(0x00))));
}

// ────────────────────────────────────────────────────────────────────
// Capability inference edge cases
// ────────────────────────────────────────────────────────────────────

#[test]
fn index_seg_also_implies_vec_search() {
    let mut builder = RvfBuilder::new();
    builder.set_manifest(test_manifest());
    builder
        .add_segment(SegmentType::ManifestSeg, b"m".to_vec())
        .unwrap();
    builder
        .add_segment(SegmentType::IndexSeg, b"idx".to_vec())
        .unwrap();

    let rvf = builder.build().unwrap();
    assert!(rvf.manifest().capabilities.contains(&Capability::VecSearch));
}

#[test]
fn overlay_and_graph_imply_protein_scoring() {
    let mut builder = RvfBuilder::new();
    builder.set_manifest(test_manifest());
    builder
        .add_segment(SegmentType::ManifestSeg, b"m".to_vec())
        .unwrap();
    builder
        .add_segment(SegmentType::OverlaySeg, b"overlay".to_vec())
        .unwrap();
    builder
        .add_segment(SegmentType::GraphSeg, b"graph".to_vec())
        .unwrap();

    let rvf = builder.build().unwrap();
    assert!(rvf
        .manifest()
        .capabilities
        .contains(&Capability::ProteinScoring));
}

#[test]
fn witness_seg_implies_p2p_sync() {
    let mut builder = RvfBuilder::new();
    builder.set_manifest(test_manifest());
    builder
        .add_segment(SegmentType::ManifestSeg, b"m".to_vec())
        .unwrap();
    builder
        .add_segment(SegmentType::WitnessSeg, b"witness".to_vec())
        .unwrap();

    let rvf = builder.build().unwrap();
    assert!(rvf.manifest().capabilities.contains(&Capability::P2pSync));
}

#[test]
fn kernel_seg_implies_mcp_agent() {
    let mut builder = RvfBuilder::new();
    builder.set_manifest(test_manifest());
    builder
        .add_segment(SegmentType::ManifestSeg, b"m".to_vec())
        .unwrap();
    builder
        .add_segment(SegmentType::KernelSeg, b"kernel".to_vec())
        .unwrap();

    let rvf = builder.build().unwrap();
    assert!(rvf.manifest().capabilities.contains(&Capability::McpAgent));
}

#[test]
fn journal_seg_implies_evolution() {
    let mut builder = RvfBuilder::new();
    builder.set_manifest(test_manifest());
    builder
        .add_segment(SegmentType::ManifestSeg, b"m".to_vec())
        .unwrap();
    builder
        .add_segment(SegmentType::JournalSeg, b"journal".to_vec())
        .unwrap();

    let rvf = builder.build().unwrap();
    assert!(rvf
        .manifest()
        .capabilities
        .contains(&Capability::Evolution));
}

// ────────────────────────────────────────────────────────────────────
// SegmentProducer integration with builder
// ────────────────────────────────────────────────────────────────────

#[test]
fn multiple_mock_producers_feed_builder() {
    let producers: Vec<MockSegmentProducer> = vec![
        mock_producer(SegmentType::ManifestSeg, b"manifest".to_vec()),
        mock_producer(SegmentType::VecSeg, b"embeddings".to_vec()),
        mock_producer(SegmentType::WasmSeg, b"wasm-binary".to_vec()),
    ];

    let mut builder = RvfBuilder::new();
    builder.set_manifest(test_manifest());

    for p in &producers {
        let data = p.produce().unwrap();
        builder.add_segment(p.segment_type(), data).unwrap();
    }

    let rvf = builder.build().unwrap();
    assert_eq!(rvf.segments().len(), 3);
    assert!(rvf.manifest().capabilities.contains(&Capability::VecSearch));
    assert!(rvf
        .manifest()
        .capabilities
        .contains(&Capability::WasmRuntime));
}

// ────────────────────────────────────────────────────────────────────
// Serialization format correctness
// ────────────────────────────────────────────────────────────────────

#[test]
fn serialized_segments_appear_in_type_order() {
    let mut builder = RvfBuilder::new();
    builder.set_manifest(test_manifest());
    // Add in reverse order
    builder
        .add_segment(SegmentType::KernelSeg, b"kernel".to_vec())
        .unwrap();
    builder
        .add_segment(SegmentType::ManifestSeg, b"manifest".to_vec())
        .unwrap();
    builder
        .add_segment(SegmentType::VecSeg, b"vec".to_vec())
        .unwrap();

    let rvf = builder.build().unwrap();
    let serialized = rvf.serialize();

    // Skip manifest header (4 bytes len + manifest JSON)
    let manifest_bytes = serde_json::to_vec(rvf.manifest()).unwrap();
    let offset = 4 + manifest_bytes.len();

    // Read segment type IDs in order
    let mut cursor = offset;
    let mut type_ids = Vec::new();
    while cursor < serialized.len() {
        type_ids.push(serialized[cursor]);
        cursor += 1;
        let len = u32::from_be_bytes(
            serialized[cursor..cursor + 4].try_into().unwrap(),
        ) as usize;
        cursor += 4 + len;
    }

    assert_eq!(type_ids, vec![0x00, 0x01, 0x0E]);
}
