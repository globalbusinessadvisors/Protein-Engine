//! MidstreamSource — stub for real-time AI stream analysis via the midstream crate.
//!
//! The midstream integration is native-only. When the midstream crate
//! is available, this source wraps its streaming API to produce
//! InstrumentReadings in real time.

use async_trait::async_trait;

use crate::error::StreamError;
use crate::traits::InstrumentSource;
use crate::types::{InstrumentReading, InstrumentType};

/// Wraps the midstream crate for real-time AI stream analysis.
///
/// In the current release this is a stub that returns no readings.
/// A production deployment would connect to the midstream runtime
/// and translate its events into `InstrumentReading` values.
pub struct MidstreamSource {
    instrument_type: InstrumentType,
    instrument_id: String,
}

impl MidstreamSource {
    pub fn new(instrument_type: InstrumentType, instrument_id: String) -> Self {
        Self {
            instrument_type,
            instrument_id,
        }
    }

    /// The instrument ID this source is configured for.
    pub fn instrument_id(&self) -> &str {
        &self.instrument_id
    }
}

#[async_trait]
impl InstrumentSource for MidstreamSource {
    async fn read_next(&mut self) -> Result<Option<InstrumentReading>, StreamError> {
        // Stub: no readings available until midstream integration is wired up.
        Ok(None)
    }

    fn instrument_type(&self) -> InstrumentType {
        self.instrument_type
    }
}
