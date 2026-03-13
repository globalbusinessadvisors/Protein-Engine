//! pe-ledger: Cryptographic append-only journal with hash chaining.
//!
//! Every platform event is recorded as a SHA3-chained, signed journal
//! entry. Provides tamper-evident audit trails serializable to
//! JOURNAL_SEG and WITNESS_SEG in the `.rvf` container.

pub mod chain;
pub mod entry;
pub mod error;
pub mod segment;
pub mod signer;
pub mod traits;
pub mod types;

pub use chain::JournalChain;
pub use entry::JournalEntry;
pub use error::LedgerError;
pub use segment::{JournalSegProducer, WitnessSegProducer};
pub use signer::CryptoSigner;
#[cfg(feature = "native")]
pub use signer::MlDsaSigner;
pub use traits::LedgerWriter;
pub use types::{EntryHash, EntryType, MlDsaSignature};

#[cfg(test)]
mod tests;
