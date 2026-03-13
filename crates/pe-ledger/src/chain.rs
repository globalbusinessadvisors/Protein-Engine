//! JournalChain — aggregate root (DDD-003, Aggregate 3).
//!
//! Append-only, hash-chained, signed journal enforcing invariants JC-1 through JC-6.

use chrono::Utc;
use tracing::debug;

use crate::entry::JournalEntry;
use crate::error::LedgerError;
use crate::signer::CryptoSigner;
use crate::types::{EntryHash, EntryType, MlDsaSignature};

/// The aggregate root for the cryptographic append-only journal.
///
/// Invariants (from DDD-003):
/// - JC-1: sequence_numbers are strictly sequential starting at 0
/// - JC-2: each entry's prev_hash == hash of the preceding entry
/// - JC-3: genesis entry has prev_hash = [0; 32]
/// - JC-4: every entry carries a valid ML-DSA signature
/// - JC-5: entries are immutable once appended
/// - JC-6: verify_chain() validates the entire chain
pub struct JournalChain {
    entries: Vec<JournalEntry>,
    tip_hash: EntryHash,
}

impl JournalChain {
    /// Create an empty chain with the genesis tip hash (all zeros).
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            tip_hash: EntryHash::GENESIS,
        }
    }

    /// Append a new entry to the chain.
    ///
    /// Enforces JC-1 (sequential numbering), JC-2/JC-3 (hash chaining),
    /// and JC-4 (ML-DSA signature).
    pub fn append_entry(
        &mut self,
        entry_type: EntryType,
        payload: Vec<u8>,
        signer: &dyn CryptoSigner,
    ) -> Result<EntryHash, LedgerError> {
        let seq = self.entries.len() as u64;
        let prev_hash = self.tip_hash;
        let timestamp = Utc::now();

        // Build a temporary entry with an empty signature to compute signable bytes.
        let mut entry = JournalEntry {
            sequence_number: seq,
            timestamp,
            prev_hash,
            entry_type,
            payload,
            signature: MlDsaSignature(Vec::new()),
        };

        // Sign the entry's canonical bytes (JC-4).
        let signable = entry.signable_bytes();
        let signature = signer
            .sign(&signable)
            .map_err(|e| LedgerError::SigningFailed(e.to_string()))?;
        entry.signature = signature;

        // Compute the entry's hash and update the chain tip (JC-2).
        let entry_hash = entry.compute_hash();
        self.tip_hash = entry_hash;
        self.entries.push(entry);

        debug!(seq = seq, hash = %entry_hash, "appended journal entry");

        Ok(entry_hash)
    }

    /// Verify the entire chain from genesis to tip.
    ///
    /// Checks JC-2 (hash chain), JC-4 (signatures). Returns `Err(TamperDetected)`
    /// on the first broken link.
    pub fn verify_chain(&self, verifier: &dyn CryptoSigner) -> Result<bool, LedgerError> {
        let mut expected_prev = EntryHash::GENESIS;

        for (i, entry) in self.entries.iter().enumerate() {
            let seq = i as u64;

            // JC-1: check sequence number
            if entry.sequence_number != seq {
                return Err(LedgerError::SequenceGap {
                    expected: seq,
                    actual: entry.sequence_number,
                });
            }

            // JC-2 / JC-3: check prev_hash
            if entry.prev_hash != expected_prev {
                return Err(LedgerError::TamperDetected {
                    index: seq,
                    expected: expected_prev.to_string(),
                    actual: entry.prev_hash.to_string(),
                });
            }

            // JC-4: verify signature
            let signable = entry.signable_bytes();
            match verifier.verify(&signable, &entry.signature) {
                Ok(true) => {}
                Ok(false) => {
                    return Err(LedgerError::InvalidSignature { index: seq });
                }
                Err(e) => {
                    return Err(LedgerError::VerificationFailed(format!(
                        "entry {}: {}",
                        seq, e
                    )));
                }
            }

            // Advance expected prev_hash
            expected_prev = entry.compute_hash();
        }

        Ok(true)
    }

    /// Number of entries in the chain.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the chain is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// The current chain tip hash.
    pub fn tip_hash(&self) -> EntryHash {
        self.tip_hash
    }

    /// Read-only access to all entries.
    pub fn entries(&self) -> &[JournalEntry] {
        &self.entries
    }

    /// Mutable access to entries (for tamper simulation in tests only).
    #[cfg(test)]
    pub(crate) fn entries_mut(&mut self) -> &mut Vec<JournalEntry> {
        &mut self.entries
    }
}

impl Default for JournalChain {
    fn default() -> Self {
        Self::new()
    }
}
