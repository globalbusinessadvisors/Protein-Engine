use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Supported laboratory instrument types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum InstrumentType {
    Opentrons,
    Hamilton,
    FlowCytometer,
    PlateReader,
}

/// A raw data point from a laboratory instrument.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstrumentReading {
    pub instrument_type: InstrumentType,
    pub instrument_id: String,
    pub timestamp: DateTime<Utc>,
    pub raw_data: BTreeMap<String, f64>,
    pub channel: Option<String>,
}
