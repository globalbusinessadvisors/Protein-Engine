use async_trait::async_trait;

use crate::error::StreamError;
use crate::types::{InstrumentReading, InstrumentType};

/// Source of raw instrument readings.
///
/// Implementations wrap real instrument connections (serial, network, file).
/// Mockable for London School TDD.
#[async_trait]
#[cfg_attr(test, mockall::automock)]
pub trait InstrumentSource: Send + Sync {
    /// Read the next available instrument data point, or None if exhausted.
    async fn read_next(&mut self) -> Result<Option<InstrumentReading>, StreamError>;

    /// The type of instrument this source reads from.
    fn instrument_type(&self) -> InstrumentType;
}
