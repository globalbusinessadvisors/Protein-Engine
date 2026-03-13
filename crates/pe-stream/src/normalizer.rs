//! Anti-corruption layer converting raw InstrumentReadings to ExperimentResult domain types.

use std::collections::BTreeMap;

use uuid::Uuid;

use pe_core::{AssayType, ExperimentResult};

use crate::error::StreamError;
use crate::types::{InstrumentReading, InstrumentType};

/// Maps instrument-specific field names to canonical metric names,
/// validates readings, and associates them with variant IDs.
pub struct ReadingNormalizer {
    /// Maps (instrument_id, channel) → variant_id for plate/well lookups.
    variant_map: BTreeMap<(String, String), Uuid>,
    /// Maps instrument-specific raw field names → canonical metric names.
    field_map: BTreeMap<String, String>,
    /// Default variant ID when no mapping is found (for single-variant experiments).
    default_variant_id: Option<Uuid>,
}

impl ReadingNormalizer {
    pub fn new() -> Self {
        Self {
            variant_map: BTreeMap::new(),
            field_map: BTreeMap::new(),
            default_variant_id: None,
        }
    }

    /// Register a mapping from (instrument_id, channel) to a variant UUID.
    pub fn map_variant(&mut self, instrument_id: &str, channel: &str, variant_id: Uuid) {
        self.variant_map
            .insert((instrument_id.to_string(), channel.to_string()), variant_id);
    }

    /// Register a field name mapping: raw instrument field → canonical name.
    pub fn map_field(&mut self, raw_name: &str, canonical_name: &str) {
        self.field_map
            .insert(raw_name.to_string(), canonical_name.to_string());
    }

    /// Set a default variant ID for readings without explicit mapping.
    pub fn set_default_variant(&mut self, variant_id: Uuid) {
        self.default_variant_id = Some(variant_id);
    }

    /// Convert a raw instrument reading into a domain ExperimentResult.
    pub fn normalize(&self, reading: &InstrumentReading) -> Result<ExperimentResult, StreamError> {
        // Validate: reject NaN and Inf values
        for (key, value) in &reading.raw_data {
            if !value.is_finite() {
                return Err(StreamError::NonFiniteValue {
                    field: key.clone(),
                });
            }
        }

        if reading.raw_data.is_empty() {
            return Err(StreamError::EmptyReading);
        }

        // Resolve variant ID
        let channel = reading.channel.clone().unwrap_or_default();
        let variant_id = self
            .variant_map
            .get(&(reading.instrument_id.clone(), channel.clone()))
            .copied()
            .or(self.default_variant_id)
            .ok_or_else(|| StreamError::NoVariantMapping {
                instrument_id: reading.instrument_id.clone(),
                channel,
            })?;

        // Map field names: apply field_map, pass through unmapped fields
        let measured_values: BTreeMap<String, f64> = reading
            .raw_data
            .iter()
            .map(|(k, v)| {
                let canonical = self.field_map.get(k).cloned().unwrap_or_else(|| k.clone());
                (canonical, *v)
            })
            .collect();

        // Map instrument type to assay type
        let assay_type = match reading.instrument_type {
            InstrumentType::FlowCytometer => AssayType::FlowCytometry,
            InstrumentType::PlateReader => AssayType::PlateReader,
            InstrumentType::Opentrons => AssayType::Custom("Opentrons".into()),
            InstrumentType::Hamilton => AssayType::Custom("Hamilton".into()),
        };

        ExperimentResult::new(
            variant_id,
            assay_type,
            measured_values,
            reading.timestamp,
            reading.instrument_id.clone(),
            None,
        )
        .map_err(StreamError::from)
    }
}

impl Default for ReadingNormalizer {
    fn default() -> Self {
        Self::new()
    }
}
