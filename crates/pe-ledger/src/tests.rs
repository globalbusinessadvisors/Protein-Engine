#[cfg(test)]
mod tests {
    use crate::chain::JournalChain;
    use crate::entry::JournalEntry;
    use crate::error::LedgerError;
    use crate::segment::{JournalSegProducer, WitnessRecord, WitnessSegProducer};
    use crate::signer::MockCryptoSigner;
    use crate::types::{EntryHash, EntryType, MlDsaSignature};
    use pe_rvf::traits::SegmentProducer;

    // ── helpers ──────────────────────────────────────────────────────────

    /// Build a mock signer that returns a deterministic 64-byte signature
    /// and always verifies successfully.
    fn mock_signer() -> MockCryptoSigner {
        let mut signer = MockCryptoSigner::new();
        signer.expect_sign().returning(|_data| {
            Ok(MlDsaSignature(vec![0xAB; 64]))
        });
        signer.expect_verify().returning(|_data, _sig| Ok(true));
        signer
    }

    /// Build a mock signer whose verify always returns false.
    fn mock_signer_bad_verify() -> MockCryptoSigner {
        let mut signer = MockCryptoSigner::new();
        signer.expect_sign().returning(|_data| {
            Ok(MlDsaSignature(vec![0xAB; 64]))
        });
        signer.expect_verify().returning(|_data, _sig| Ok(false));
        signer
    }

    // ── JC-3: genesis entry has prev_hash = zeros ───────────────────────

    #[test]
    fn append_to_empty_chain_sets_prev_hash_to_zeros() {
        let signer = mock_signer();
        let mut chain = JournalChain::new();

        chain
            .append_entry(EntryType::VariantDesigned, b"genesis".to_vec(), &signer)
            .unwrap();

        let first = &chain.entries()[0];
        assert_eq!(first.prev_hash, EntryHash::GENESIS);
        assert_eq!(first.sequence_number, 0);
    }

    // ── JC-2: hash chaining across 3 entries ────────────────────────────

    #[test]
    fn append_chains_hashes_correctly_across_three_entries() {
        let signer = mock_signer();
        let mut chain = JournalChain::new();

        let h0 = chain
            .append_entry(EntryType::VariantDesigned, b"e0".to_vec(), &signer)
            .unwrap();
        let h1 = chain
            .append_entry(EntryType::FitnessScored, b"e1".to_vec(), &signer)
            .unwrap();
        let h2 = chain
            .append_entry(EntryType::VqeCompleted, b"e2".to_vec(), &signer)
            .unwrap();

        // Each entry's prev_hash should be the hash of the prior entry
        assert_eq!(chain.entries()[0].prev_hash, EntryHash::GENESIS);
        assert_eq!(chain.entries()[1].prev_hash, h0);
        assert_eq!(chain.entries()[2].prev_hash, h1);
        // tip_hash is the hash of the last entry
        assert_eq!(chain.tip_hash(), h2);
    }

    // ── JC-1: strictly sequential sequence numbers ──────────────────────

    #[test]
    fn sequence_numbers_are_strictly_sequential() {
        let signer = mock_signer();
        let mut chain = JournalChain::new();

        for i in 0..5 {
            chain
                .append_entry(
                    EntryType::ExperimentRecorded,
                    format!("entry-{}", i).into_bytes(),
                    &signer,
                )
                .unwrap();
        }

        for (i, entry) in chain.entries().iter().enumerate() {
            assert_eq!(entry.sequence_number, i as u64);
        }
    }

    // ── JC-6: verify_chain succeeds on valid chain ──────────────────────

    #[test]
    fn verify_chain_succeeds_on_valid_chain() {
        let signer = mock_signer();
        let mut chain = JournalChain::new();

        chain
            .append_entry(EntryType::VariantDesigned, b"a".to_vec(), &signer)
            .unwrap();
        chain
            .append_entry(EntryType::FitnessScored, b"b".to_vec(), &signer)
            .unwrap();
        chain
            .append_entry(EntryType::CycleCompleted, b"c".to_vec(), &signer)
            .unwrap();

        assert!(chain.verify_chain(&signer).unwrap());
    }

    // ── verify_chain detects tampered payload ────────────────────────────

    #[test]
    fn verify_chain_detects_tampered_payload() {
        let signer = mock_signer();
        let mut chain = JournalChain::new();

        chain
            .append_entry(EntryType::VariantDesigned, b"original".to_vec(), &signer)
            .unwrap();
        chain
            .append_entry(EntryType::FitnessScored, b"data".to_vec(), &signer)
            .unwrap();

        // Tamper with the first entry's payload
        chain.entries_mut()[0].payload = b"TAMPERED".to_vec();

        let result = chain.verify_chain(&signer);
        assert!(result.is_err());
        match result.unwrap_err() {
            LedgerError::TamperDetected { index, .. } => assert_eq!(index, 1),
            other => panic!("expected TamperDetected, got: {:?}", other),
        }
    }

    // ── verify_chain detects tampered prev_hash ─────────────────────────

    #[test]
    fn verify_chain_detects_tampered_prev_hash() {
        let signer = mock_signer();
        let mut chain = JournalChain::new();

        chain
            .append_entry(EntryType::VariantDesigned, b"a".to_vec(), &signer)
            .unwrap();
        chain
            .append_entry(EntryType::FitnessScored, b"b".to_vec(), &signer)
            .unwrap();

        // Tamper with entry 1's prev_hash
        chain.entries_mut()[1].prev_hash = EntryHash([0xFF; 32]);

        let result = chain.verify_chain(&signer);
        assert!(result.is_err());
        match result.unwrap_err() {
            LedgerError::TamperDetected { index, .. } => assert_eq!(index, 1),
            other => panic!("expected TamperDetected, got: {:?}", other),
        }
    }

    // ── verify_chain detects invalid signature ──────────────────────────

    #[test]
    fn verify_chain_detects_invalid_signature() {
        let signer = mock_signer();
        let mut chain = JournalChain::new();

        chain
            .append_entry(EntryType::VariantDesigned, b"a".to_vec(), &signer)
            .unwrap();
        chain
            .append_entry(EntryType::FitnessScored, b"b".to_vec(), &signer)
            .unwrap();

        // Verify with a signer that always returns false
        let bad_verifier = mock_signer_bad_verify();
        let result = chain.verify_chain(&bad_verifier);

        assert!(result.is_err());
        match result.unwrap_err() {
            LedgerError::InvalidSignature { index } => assert_eq!(index, 0),
            other => panic!("expected InvalidSignature, got: {:?}", other),
        }
    }

    // ── chain length ────────────────────────────────────────────────────

    #[test]
    fn chain_len_tracks_entries() {
        let signer = mock_signer();
        let mut chain = JournalChain::new();

        assert_eq!(chain.len(), 0);
        assert!(chain.is_empty());

        chain
            .append_entry(EntryType::ModelUpdated, b"x".to_vec(), &signer)
            .unwrap();
        assert_eq!(chain.len(), 1);
        assert!(!chain.is_empty());
    }

    // ── JOURNAL_SEG round-trip ──────────────────────────────────────────

    #[test]
    fn journal_seg_round_trip_preserves_all_entries() {
        let signer = mock_signer();
        let mut chain = JournalChain::new();

        chain
            .append_entry(EntryType::VariantDesigned, b"one".to_vec(), &signer)
            .unwrap();
        chain
            .append_entry(EntryType::FitnessScored, b"two".to_vec(), &signer)
            .unwrap();
        chain
            .append_entry(EntryType::SafetyScreened, b"three".to_vec(), &signer)
            .unwrap();

        let producer = JournalSegProducer::new(chain.entries().to_vec());
        assert_eq!(producer.segment_type(), pe_rvf::SegmentType::JournalSeg);

        let bytes = producer.produce().unwrap();
        let recovered: Vec<JournalEntry> = serde_json::from_slice(&bytes).unwrap();

        assert_eq!(recovered.len(), 3);
        assert_eq!(recovered[0].sequence_number, 0);
        assert_eq!(recovered[1].sequence_number, 1);
        assert_eq!(recovered[2].sequence_number, 2);
        assert_eq!(recovered[0].payload, b"one");
        assert_eq!(recovered[1].payload, b"two");
        assert_eq!(recovered[2].payload, b"three");
        assert_eq!(recovered[0].entry_type, EntryType::VariantDesigned);
        assert_eq!(recovered[1].entry_type, EntryType::FitnessScored);
        assert_eq!(recovered[2].entry_type, EntryType::SafetyScreened);
    }

    // ── WITNESS_SEG serialization ───────────────────────────────────────

    #[test]
    fn witness_seg_produces_parseable_output() {
        let signer = mock_signer();
        let mut chain = JournalChain::new();

        chain
            .append_entry(EntryType::VqeCompleted, b"vqe-data".to_vec(), &signer)
            .unwrap();
        chain
            .append_entry(EntryType::AgentRetired, b"agent-data".to_vec(), &signer)
            .unwrap();

        let producer = WitnessSegProducer::new(chain.entries().to_vec());
        assert_eq!(producer.segment_type(), pe_rvf::SegmentType::WitnessSeg);

        let bytes = producer.produce().unwrap();
        let records: Vec<WitnessRecord> = serde_json::from_slice(&bytes).unwrap();

        assert_eq!(records.len(), 2);
        assert_eq!(records[0].sequence_number, 0);
        assert_eq!(records[1].sequence_number, 1);
        assert_eq!(records[0].entry_type, "VqeCompleted");
        assert_eq!(records[1].entry_type, "AgentRetired");

        // Hashes should be 64 hex chars (32 bytes)
        assert_eq!(records[0].entry_hash.len(), 64);
        assert_eq!(records[0].prev_hash.len(), 64);
        // Genesis prev_hash should be all zeros
        assert_eq!(records[0].prev_hash, "0".repeat(64));
    }

    // ── empty chain tip hash ────────────────────────────────────────────

    #[test]
    fn empty_chain_tip_is_genesis() {
        let chain = JournalChain::new();
        assert_eq!(chain.tip_hash(), EntryHash::GENESIS);
    }

    // ── verify empty chain ──────────────────────────────────────────────

    #[test]
    fn verify_empty_chain_succeeds() {
        let signer = mock_signer();
        let chain = JournalChain::new();
        assert!(chain.verify_chain(&signer).unwrap());
    }

    // ── all 9 entry types ───────────────────────────────────────────────

    #[test]
    fn all_nine_entry_types_can_be_appended() {
        let signer = mock_signer();
        let mut chain = JournalChain::new();

        let types = [
            EntryType::VariantDesigned,
            EntryType::FitnessScored,
            EntryType::StructureValidated,
            EntryType::SafetyScreened,
            EntryType::ExperimentRecorded,
            EntryType::ModelUpdated,
            EntryType::VqeCompleted,
            EntryType::CycleCompleted,
            EntryType::AgentRetired,
        ];

        for (i, et) in types.iter().enumerate() {
            chain
                .append_entry(*et, format!("type-{}", i).into_bytes(), &signer)
                .unwrap();
        }

        assert_eq!(chain.len(), 9);
        assert!(chain.verify_chain(&signer).unwrap());
    }

    // ── Integration test with real ML-DSA signer ────────────────────────

    #[test]
    fn integration_real_mldsa_signer_append_and_verify() {
        use crate::signer::MlDsaSigner;

        let signer = MlDsaSigner::generate();

        let mut chain = JournalChain::new();
        chain
            .append_entry(EntryType::VariantDesigned, b"real-1".to_vec(), &signer)
            .unwrap();
        chain
            .append_entry(EntryType::FitnessScored, b"real-2".to_vec(), &signer)
            .unwrap();
        chain
            .append_entry(EntryType::CycleCompleted, b"real-3".to_vec(), &signer)
            .unwrap();

        assert_eq!(chain.len(), 3);

        // Verify with the same signer (holds the verification key)
        assert!(chain.verify_chain(&signer).unwrap());

        // Tamper and verify detection
        chain.entries_mut()[1].payload = b"TAMPERED".to_vec();
        let result = chain.verify_chain(&signer);
        // Either signature fails or hash chain breaks
        assert!(result.is_err());
    }
}
