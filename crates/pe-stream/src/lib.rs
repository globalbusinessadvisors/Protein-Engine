//! pe-stream: Live laboratory instrument data ingestion.
//!
//! Provides `InstrumentSource` trait and `StreamProcessor` for normalizing
//! raw instrument readings from Opentrons, Hamilton, flow cytometers,
//! and plate readers into domain `ExperimentResult` types.

pub mod error;
pub mod midstream;
pub mod normalizer;
pub mod processor;
pub mod traits;
pub mod types;

pub use error::StreamError;
pub use midstream::MidstreamSource;
pub use normalizer::ReadingNormalizer;
pub use processor::StreamProcessor;
pub use traits::InstrumentSource;
pub use types::{InstrumentReading, InstrumentType};

#[cfg(test)]
mod tests;
