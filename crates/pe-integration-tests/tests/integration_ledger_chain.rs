//! Integration test: real JournalChain + real MlDsaSigner (post-quantum crypto).
//!
//! Tests append, verify, serialize/deserialize, and tamper detection.

use pe_ledger::{
    CryptoSigner, EntryType, JournalChain, JournalEntry, JournalSegProducer, MlDsaSigner,
    WitnessSegProducer,
};
use pe_rvf::traits::SegmentProducer;

// ── Helpers ──────────────────────────────────────────────────────────

fn all_entry_types() -> Vec<EntryType> {
    vec![
        EntryType::VariantDesigned,
        EntryType::FitnessScored,
        EntryType::StructureValidated,
        EntryType::SafetyScreened,
        EntryType::ExperimentRecorded,
        EntryType::ModelUpdated,
        EntryType::VqeCompleted,
        EntryType::CycleCompleted,
        EntryType::AgentRetired,
    ]
}

// ── Tests ────────────────────────────────────────────────────────────

#[test]
fn append_100_entries_and_verify_chain() {
    let signer = MlDsaSigner::generate();
    let mut chain = JournalChain::new();
    let types = all_entry_types();

    for i in 0..100 {
        let entry_type = types[i % types.len()];
        let payload = format!("entry-{i}").into_bytes();
        chain
            .append_entry(entry_type, payload, &signer)
            .unwrap_or_else(|e| panic!("append entry {i} failed: {e}"));
    }

    assert_eq!(chain.len(), 100);
    assert!(
        chain.verify_chain(&signer).expect("verify"),
        "chain must be valid"
    );
}

#[test]
fn chain_enforces_sequential_hashing() {
    let signer = MlDsaSigner::generate();
    let mut chain = JournalChain::new();

    let h1 = chain
        .append_entry(EntryType::VariantDesigned, b"first".to_vec(), &signer)
        .expect("first");
    let h2 = chain
        .append_entry(EntryType::FitnessScored, b"second".to_vec(), &signer)
        .expect("second");

    assert_ne!(h1, h2, "consecutive hashes must differ");
    assert_eq!(chain.entries()[1].prev_hash, h1, "entry 2 must reference entry 1 hash");
}

#[test]
fn serialize_deserialize_journal_seg_then_verify() {
    let signer = MlDsaSigner::generate();
    let mut chain = JournalChain::new();

    for i in 0..25 {
        chain
            .append_entry(
                EntryType::CycleCompleted,
                format!("cycle-{i}").into_bytes(),
                &signer,
            )
            .expect("append");
    }

    // Produce JOURNAL_SEG
    let producer = JournalSegProducer::new(chain.entries().to_vec());
    let seg_bytes = producer.produce().expect("produce segment");
    assert!(!seg_bytes.is_empty());

    // Deserialize entries
    let restored: Vec<JournalEntry> =
        serde_json::from_slice(&seg_bytes).expect("deserialize entries");
    assert_eq!(restored.len(), 25);

    // Verify each restored entry's signature individually
    for entry in &restored {
        let signable = entry.signable_bytes();
        assert!(
            signer.verify(&signable, &entry.signature).expect("verify sig"),
            "entry {} signature invalid",
            entry.sequence_number
        );
    }
}

#[test]
fn witness_seg_produces_valid_output() {
    let signer = MlDsaSigner::generate();
    let mut chain = JournalChain::new();

    for i in 0..10 {
        chain
            .append_entry(EntryType::FitnessScored, format!("w-{i}").into_bytes(), &signer)
            .expect("append");
    }

    let producer = WitnessSegProducer::new(chain.entries().to_vec());
    let seg_bytes = producer.produce().expect("produce witness seg");
    let records: Vec<serde_json::Value> =
        serde_json::from_slice(&seg_bytes).expect("parse witness");
    assert_eq!(records.len(), 10);

    for (i, rec) in records.iter().enumerate() {
        assert_eq!(rec["sequence_number"], i as u64);
        assert!(rec["entry_hash"].is_string());
        assert!(rec["signature_hex"].is_string());
    }
}

#[test]
fn tamper_detection_on_payload_modification() {
    let signer = MlDsaSigner::generate();
    let mut chain = JournalChain::new();

    for i in 0..10 {
        chain
            .append_entry(EntryType::VariantDesigned, format!("data-{i}").into_bytes(), &signer)
            .expect("append");
    }

    assert!(chain.verify_chain(&signer).expect("verify before tamper"));

    // Serialize → tamper → deserialize → rebuild chain
    let producer = JournalSegProducer::new(chain.entries().to_vec());
    let seg_bytes = producer.produce().expect("produce");
    let mut entries: Vec<JournalEntry> =
        serde_json::from_slice(&seg_bytes).expect("deserialize");

    // Tamper with entry 5's payload
    entries[5].payload = b"TAMPERED".to_vec();

    // Verify the tampered entry's signature fails
    let signable = entries[5].signable_bytes();
    let sig_valid = signer
        .verify(&signable, &entries[5].signature)
        .expect("verify tampered");
    assert!(
        !sig_valid,
        "tampered entry signature must fail verification"
    );
}

#[test]
fn different_signers_cannot_verify_each_other() {
    let signer_a = MlDsaSigner::generate();
    let signer_b = MlDsaSigner::generate();
    let mut chain = JournalChain::new();

    chain
        .append_entry(EntryType::VariantDesigned, b"secret".to_vec(), &signer_a)
        .expect("append");

    // Signer B should not verify signer A's chain — either returns Ok(false)
    // or Err(InvalidSignature), both indicate verification failure.
    let result = chain.verify_chain(&signer_b);
    match result {
        Ok(valid) => assert!(!valid, "different signer must fail chain verification"),
        Err(_) => {} // Signature error is also correct behavior
    }
}

#[test]
fn empty_chain_is_valid() {
    let signer = MlDsaSigner::generate();
    let chain = JournalChain::new();
    assert_eq!(chain.len(), 0);
    assert!(chain.verify_chain(&signer).expect("verify empty"));
}
