use crate::error::LedgerError;
use crate::types::{EntryHash, EntryType};

/// Trait for appending entries and verifying the journal chain.
///
/// Mockable for downstream crates that depend on ledger functionality.
#[cfg_attr(test, mockall::automock)]
pub trait LedgerWriter: Send + Sync {
    fn append_entry(
        &mut self,
        entry_type: EntryType,
        payload: Vec<u8>,
    ) -> Result<EntryHash, LedgerError>;

    fn verify_chain(&self) -> Result<bool, LedgerError>;

    fn len(&self) -> usize;
}
