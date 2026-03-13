#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use chrono::Utc;
    use uuid::Uuid;

    use pe_core::AssayType;

    use crate::error::StreamError;
    use crate::midstream::MidstreamSource;
    use crate::normalizer::ReadingNormalizer;
    use crate::processor::StreamProcessor;
    use crate::traits::{InstrumentSource, MockInstrumentSource};
    use crate::types::{InstrumentReading, InstrumentType};

    // ── helpers ──────────────────────────────────────────────────────────

    fn sample_reading(
        instrument_type: InstrumentType,
        data: BTreeMap<String, f64>,
    ) -> InstrumentReading {
        InstrumentReading {
            instrument_type,
            instrument_id: "INST-001".to_string(),
            timestamp: Utc::now(),
            raw_data: data,
            channel: Some("A1".to_string()),
        }
    }

    fn default_normalizer(variant_id: Uuid) -> ReadingNormalizer {
        let mut n = ReadingNormalizer::new();
        n.map_variant("INST-001", "A1", variant_id);
        n
    }

    // ── process_next returns ExperimentResult from mocked reading ────────

    #[tokio::test]
    async fn process_next_returns_experiment_result_from_mock() {
        let variant_id = Uuid::new_v4();
        let mut data = BTreeMap::new();
        data.insert("fluorescence".to_string(), 1234.5);
        let reading = sample_reading(InstrumentType::FlowCytometer, data);

        let mut mock = MockInstrumentSource::new();
        let reading_clone = reading.clone();
        mock.expect_read_next()
            .times(1)
            .returning(move || {
                let r = reading_clone.clone();
                Box::pin(async move { Ok(Some(r)) })
            });
        mock.expect_instrument_type()
            .return_const(InstrumentType::FlowCytometer);

        let normalizer = default_normalizer(variant_id);
        let mut processor = StreamProcessor::new(mock, normalizer);

        let result = processor.process_next().await.unwrap().unwrap();
        assert_eq!(result.variant_id(), variant_id);
        assert_eq!(*result.assay_type(), AssayType::FlowCytometry);
        assert!(result.measured_values().contains_key("fluorescence"));
        assert_eq!(result.instrument_id(), "INST-001");
    }

    // ── process_next returns None when source exhausted ─────────────────

    #[tokio::test]
    async fn process_next_returns_none_when_exhausted() {
        let mut mock = MockInstrumentSource::new();
        mock.expect_read_next()
            .times(1)
            .returning(|| Box::pin(async { Ok(None) }));
        mock.expect_instrument_type()
            .return_const(InstrumentType::PlateReader);

        let normalizer = ReadingNormalizer::new();
        let mut processor = StreamProcessor::new(mock, normalizer);

        assert!(processor.process_next().await.unwrap().is_none());
    }

    // ── normalizer rejects NaN values ───────────────────────────────────

    #[test]
    fn normalizer_rejects_nan_values() {
        let variant_id = Uuid::new_v4();
        let normalizer = default_normalizer(variant_id);

        let mut data = BTreeMap::new();
        data.insert("value".to_string(), f64::NAN);
        let reading = sample_reading(InstrumentType::PlateReader, data);

        let result = normalizer.normalize(&reading);
        assert!(result.is_err());
        match result.unwrap_err() {
            StreamError::NonFiniteValue { field } => assert_eq!(field, "value"),
            other => panic!("expected NonFiniteValue, got: {:?}", other),
        }
    }

    #[test]
    fn normalizer_rejects_infinity() {
        let variant_id = Uuid::new_v4();
        let normalizer = default_normalizer(variant_id);

        let mut data = BTreeMap::new();
        data.insert("signal".to_string(), f64::INFINITY);
        let reading = sample_reading(InstrumentType::PlateReader, data);

        let result = normalizer.normalize(&reading);
        assert!(result.is_err());
        match result.unwrap_err() {
            StreamError::NonFiniteValue { field } => assert_eq!(field, "signal"),
            other => panic!("expected NonFiniteValue, got: {:?}", other),
        }
    }

    // ── normalizer maps field names ─────────────────────────────────────

    #[test]
    fn normalizer_maps_instrument_specific_fields_to_canonical() {
        let variant_id = Uuid::new_v4();
        let mut normalizer = default_normalizer(variant_id);
        normalizer.map_field("FL1-A", "fluorescence_channel_1");
        normalizer.map_field("FSC-A", "forward_scatter");

        let mut data = BTreeMap::new();
        data.insert("FL1-A".to_string(), 500.0);
        data.insert("FSC-A".to_string(), 120.0);
        let reading = sample_reading(InstrumentType::FlowCytometer, data);

        let result = normalizer.normalize(&reading).unwrap();
        assert!(result.measured_values().contains_key("fluorescence_channel_1"));
        assert!(result.measured_values().contains_key("forward_scatter"));
        assert!(!result.measured_values().contains_key("FL1-A"));
    }

    #[test]
    fn normalizer_passes_through_unmapped_fields() {
        let variant_id = Uuid::new_v4();
        let normalizer = default_normalizer(variant_id);

        let mut data = BTreeMap::new();
        data.insert("some_raw_field".to_string(), 42.0);
        let reading = sample_reading(InstrumentType::Opentrons, data);

        let result = normalizer.normalize(&reading).unwrap();
        assert!(result.measured_values().contains_key("some_raw_field"));
    }

    // ── process_batch collects up to max ────────────────────────────────

    #[tokio::test]
    async fn process_batch_collects_up_to_max() {
        let variant_id = Uuid::new_v4();
        let mut call_count = 0u32;

        let mut mock = MockInstrumentSource::new();
        mock.expect_read_next()
            .times(3)
            .returning(move || {
                call_count += 1;
                let mut data = BTreeMap::new();
                data.insert("val".to_string(), call_count as f64);
                let reading = InstrumentReading {
                    instrument_type: InstrumentType::PlateReader,
                    instrument_id: "INST-001".to_string(),
                    timestamp: Utc::now(),
                    raw_data: data,
                    channel: Some("A1".to_string()),
                };
                Box::pin(async move { Ok(Some(reading)) })
            });
        mock.expect_instrument_type()
            .return_const(InstrumentType::PlateReader);

        let normalizer = default_normalizer(variant_id);
        let mut processor = StreamProcessor::new(mock, normalizer);

        let results = processor.process_batch(3).await.unwrap();
        assert_eq!(results.len(), 3);
    }

    #[tokio::test]
    async fn process_batch_stops_at_none() {
        let variant_id = Uuid::new_v4();
        let mut call_count = 0u32;

        let mut mock = MockInstrumentSource::new();
        mock.expect_read_next()
            .times(3)
            .returning(move || {
                call_count += 1;
                if call_count <= 2 {
                    let mut data = BTreeMap::new();
                    data.insert("val".to_string(), call_count as f64);
                    let reading = InstrumentReading {
                        instrument_type: InstrumentType::Hamilton,
                        instrument_id: "INST-001".to_string(),
                        timestamp: Utc::now(),
                        raw_data: data,
                        channel: Some("A1".to_string()),
                    };
                    Box::pin(async move { Ok(Some(reading)) })
                } else {
                    Box::pin(async { Ok(None) })
                }
            });
        mock.expect_instrument_type()
            .return_const(InstrumentType::Hamilton);

        let normalizer = default_normalizer(variant_id);
        let mut processor = StreamProcessor::new(mock, normalizer);

        let results = processor.process_batch(10).await.unwrap();
        assert_eq!(results.len(), 2);
    }

    // ── instrument_type propagated ──────────────────────────────────────

    #[tokio::test]
    async fn instrument_type_is_correctly_propagated() {
        let mut mock = MockInstrumentSource::new();
        mock.expect_instrument_type()
            .return_const(InstrumentType::Hamilton);
        mock.expect_read_next()
            .returning(|| Box::pin(async { Ok(None) }));

        let normalizer = ReadingNormalizer::new();
        let processor = StreamProcessor::new(mock, normalizer);

        assert_eq!(
            processor.source().instrument_type(),
            InstrumentType::Hamilton
        );
    }

    // ── instrument type to assay type mapping ───────────────────────────

    #[test]
    fn flow_cytometer_maps_to_flow_cytometry_assay() {
        let variant_id = Uuid::new_v4();
        let normalizer = default_normalizer(variant_id);

        let mut data = BTreeMap::new();
        data.insert("signal".to_string(), 1.0);
        let reading = sample_reading(InstrumentType::FlowCytometer, data);

        let result = normalizer.normalize(&reading).unwrap();
        assert_eq!(*result.assay_type(), AssayType::FlowCytometry);
    }

    #[test]
    fn plate_reader_maps_to_plate_reader_assay() {
        let variant_id = Uuid::new_v4();
        let normalizer = default_normalizer(variant_id);

        let mut data = BTreeMap::new();
        data.insert("od".to_string(), 0.5);
        let reading = sample_reading(InstrumentType::PlateReader, data);

        let result = normalizer.normalize(&reading).unwrap();
        assert_eq!(*result.assay_type(), AssayType::PlateReader);
    }

    #[test]
    fn opentrons_maps_to_custom_assay() {
        let variant_id = Uuid::new_v4();
        let normalizer = default_normalizer(variant_id);

        let mut data = BTreeMap::new();
        data.insert("volume".to_string(), 200.0);
        let reading = sample_reading(InstrumentType::Opentrons, data);

        let result = normalizer.normalize(&reading).unwrap();
        assert_eq!(
            *result.assay_type(),
            AssayType::Custom("Opentrons".into())
        );
    }

    // ── normalizer rejects empty reading ────────────────────────────────

    #[test]
    fn normalizer_rejects_empty_raw_data() {
        let variant_id = Uuid::new_v4();
        let normalizer = default_normalizer(variant_id);

        let data = BTreeMap::new();
        let reading = sample_reading(InstrumentType::PlateReader, data);

        let result = normalizer.normalize(&reading);
        assert!(result.is_err());
        match result.unwrap_err() {
            StreamError::EmptyReading => {}
            other => panic!("expected EmptyReading, got: {:?}", other),
        }
    }

    // ── variant mapping ─────────────────────────────────────────────────

    #[test]
    fn normalizer_uses_default_variant_when_no_mapping() {
        let default_id = Uuid::new_v4();
        let mut normalizer = ReadingNormalizer::new();
        normalizer.set_default_variant(default_id);

        let mut data = BTreeMap::new();
        data.insert("val".to_string(), 1.0);
        let reading = InstrumentReading {
            instrument_type: InstrumentType::PlateReader,
            instrument_id: "UNKNOWN".to_string(),
            timestamp: Utc::now(),
            raw_data: data,
            channel: None,
        };

        let result = normalizer.normalize(&reading).unwrap();
        assert_eq!(result.variant_id(), default_id);
    }

    #[test]
    fn normalizer_errors_when_no_mapping_and_no_default() {
        let normalizer = ReadingNormalizer::new();

        let mut data = BTreeMap::new();
        data.insert("val".to_string(), 1.0);
        let reading = InstrumentReading {
            instrument_type: InstrumentType::PlateReader,
            instrument_id: "UNKNOWN".to_string(),
            timestamp: Utc::now(),
            raw_data: data,
            channel: None,
        };

        let result = normalizer.normalize(&reading);
        assert!(result.is_err());
    }

    // ── MidstreamSource stub ────────────────────────────────────────────

    #[tokio::test]
    async fn midstream_source_returns_none() {
        let mut source = MidstreamSource::new(
            InstrumentType::FlowCytometer,
            "MID-001".to_string(),
        );

        assert_eq!(source.instrument_type(), InstrumentType::FlowCytometer);
        assert_eq!(source.instrument_id(), "MID-001");
        assert!(source.read_next().await.unwrap().is_none());
    }
}
