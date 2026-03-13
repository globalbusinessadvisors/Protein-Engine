//! pe-rvf: RVF cognitive container builder and segment definitions.
//!
//! Assembles all platform data, models, indices, and runtime into
//! a single `.rvf` file — the universal deployment artifact.
//!
//! See ADR-001 for segment allocation and DDD-003 (Aggregate 4) for invariants.

pub mod builder;
pub mod capability;
pub mod error;
pub mod manifest;
pub mod rvf_file;
pub mod segment;
pub mod traits;

pub use builder::RvfBuilder;
pub use capability::Capability;
pub use error::RvfError;
pub use manifest::Manifest;
pub use rvf_file::RvfFile;
pub use segment::SegmentType;
pub use traits::{RvfAssembler, SegmentProducer};

#[cfg(test)]
mod tests;
