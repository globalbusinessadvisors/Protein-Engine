//! Experiment and assay result types.

use alloc::collections::BTreeMap;
use alloc::string::String;
use core::fmt;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::sequence::CoreError;

// ---------------------------------------------------------------------------
// AssayType
// ---------------------------------------------------------------------------

/// Laboratory assay modality.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
pub enum AssayType {
    FlowCytometry,
    WesternBlot,
    QPCR,
    PlateReader,
    CellViability,
    Custom(String),
}

impl fmt::Display for AssayType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::FlowCytometry => f.write_str("FlowCytometry"),
            Self::WesternBlot => f.write_str("WesternBlot"),
            Self::QPCR => f.write_str("qPCR"),
            Self::PlateReader => f.write_str("PlateReader"),
            Self::CellViability => f.write_str("CellViability"),
            Self::Custom(name) => write!(f, "Custom({})", name),
        }
    }
}

// ---------------------------------------------------------------------------
// ExperimentResult
// ---------------------------------------------------------------------------

/// A single experiment result for a protein variant.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct ExperimentResult {
    variant_id: Uuid,
    assay_type: AssayType,
    measured_values: BTreeMap<String, f64>,
    timestamp: DateTime<Utc>,
    instrument_id: String,
    notes: Option<String>,
}

impl ExperimentResult {
    /// Create a validated experiment result.
    ///
    /// # Errors
    ///
    /// - `CoreError::EmptyMeasuredValues` if `measured_values` is empty.
    /// - `CoreError::NonFiniteMeasuredValue` if any value is NaN or infinite.
    /// - `CoreError::EmptyInstrumentId` if `instrument_id` is empty.
    pub fn new(
        variant_id: Uuid,
        assay_type: AssayType,
        measured_values: BTreeMap<String, f64>,
        timestamp: DateTime<Utc>,
        instrument_id: String,
        notes: Option<String>,
    ) -> Result<Self, CoreError> {
        if measured_values.is_empty() {
            return Err(CoreError::EmptyMeasuredValues);
        }
        for (key, value) in &measured_values {
            if !value.is_finite() {
                return Err(CoreError::NonFiniteMeasuredValue { key: key.clone() });
            }
        }
        if instrument_id.is_empty() {
            return Err(CoreError::EmptyInstrumentId);
        }
        Ok(Self {
            variant_id,
            assay_type,
            measured_values,
            timestamp,
            instrument_id,
            notes,
        })
    }

    /// Variant identifier.
    pub fn variant_id(&self) -> Uuid {
        self.variant_id
    }

    /// Assay type used in this experiment.
    pub fn assay_type(&self) -> &AssayType {
        &self.assay_type
    }

    /// Measured numeric values keyed by metric name.
    pub fn measured_values(&self) -> &BTreeMap<String, f64> {
        &self.measured_values
    }

    /// Timestamp when the result was recorded.
    pub fn timestamp(&self) -> &DateTime<Utc> {
        &self.timestamp
    }

    /// Identifier of the instrument that produced this result.
    pub fn instrument_id(&self) -> &str {
        &self.instrument_id
    }

    /// Optional free-text notes.
    pub fn notes(&self) -> Option<&str> {
        self.notes.as_deref()
    }
}
