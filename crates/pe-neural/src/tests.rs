use pe_core::{
    AminoAcidSequence, Embedding320, FitnessWeights, ProteinVariant, YamanakaFactor,
};

use crate::ensemble::EnsemblePredictor;
use crate::error::NeuralError;
use crate::scorers::{LstmScorer, NBeatsScorer, TransformerScorer};
use crate::segment::{ModelWeights, QuantSegProducer};
use crate::traits::{FitnessPredictor, MockSubModelScorer, SubModelScorer};
use pe_rvf::SegmentProducer;

// ────────────────────────────────────────────────────────────────────
// Helpers
// ────────────────────────────────────────────────────────────────────

fn test_variant() -> ProteinVariant {
    let seq = AminoAcidSequence::new("ACDEFGHIKLMNPQRSTVWY").unwrap();
    ProteinVariant::wild_type("test-variant", seq, YamanakaFactor::OCT4)
}

fn test_embedding() -> Embedding320 {
    let mut arr = [0.0f32; 320];
    for (i, val) in arr.iter_mut().enumerate() {
        *val = (i as f32 * 0.01).sin();
    }
    Embedding320::new(arr)
}

fn default_weights() -> FitnessWeights {
    FitnessWeights::default_weights()
}

fn mock_scorer(name: &'static str, value: f64) -> MockSubModelScorer {
    let mut mock = MockSubModelScorer::new();
    mock.expect_score().returning(move |_, _| Ok(value));
    mock.expect_model_name().return_const(name.to_string());
    mock
}

fn mock_scorer_err(name: &'static str) -> MockSubModelScorer {
    let name_owned = name.to_string();
    let mut mock = MockSubModelScorer::new();
    mock.expect_score().returning(move |_, _| {
        Err(NeuralError::ModelNotLoaded(name.to_string()))
    });
    mock.expect_model_name().return_const(name_owned);
    mock
}

// ────────────────────────────────────────────────────────────────────
// EnsemblePredictor with mocked sub-models
// ────────────────────────────────────────────────────────────────────

#[test]
fn ensemble_aggregates_three_sub_model_scores_with_correct_weights() {
    // Mock: transformer=0.8, lstm=0.7, nbeats=0.9
    let transformer = mock_scorer("transformer", 0.8);
    let lstm = mock_scorer("lstm", 0.7);
    let nbeats = mock_scorer("nbeats", 0.9);

    let weights = default_weights();
    let ensemble = EnsemblePredictor::new(transformer, lstm, nbeats, weights.clone());

    let variant = test_variant();
    let embedding = test_embedding();
    let score = ensemble.predict(&variant, &embedding).unwrap();

    // safety_score = 1.0 - min(0.8, 0.7, 0.9) = 1.0 - 0.7 = 0.3
    // composite = 0.35*0.8 + 0.25*0.7 + 0.25*0.9 + 0.15*(1.0 - 0.3)
    //           = 0.28 + 0.175 + 0.225 + 0.105 = 0.785
    let expected_safety = 0.3;
    let expected_composite = weights.reprogramming * 0.8
        + weights.stability * 0.7
        + weights.plausibility * 0.9
        + weights.safety * (1.0 - expected_safety);

    assert!((score.reprogramming_efficiency() - 0.8).abs() < 1e-9);
    assert!((score.expression_stability() - 0.7).abs() < 1e-9);
    assert!((score.structural_plausibility() - 0.9).abs() < 1e-9);
    assert!((score.safety_score() - expected_safety).abs() < 1e-9);
    assert!(
        (score.composite() - expected_composite).abs() < 1e-9,
        "expected composite {expected_composite}, got {}",
        score.composite()
    );
}

#[test]
fn predict_returns_error_when_transformer_fails() {
    let transformer = mock_scorer_err("transformer");
    let lstm = mock_scorer("lstm", 0.7);
    let nbeats = mock_scorer("nbeats", 0.9);

    let ensemble = EnsemblePredictor::new(transformer, lstm, nbeats, default_weights());
    let result = ensemble.predict(&test_variant(), &test_embedding());

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.to_string().contains("transformer"));
}

#[test]
fn predict_returns_error_when_lstm_fails() {
    let transformer = mock_scorer("transformer", 0.8);
    let lstm = mock_scorer_err("lstm");
    let nbeats = mock_scorer("nbeats", 0.9);

    let ensemble = EnsemblePredictor::new(transformer, lstm, nbeats, default_weights());
    let result = ensemble.predict(&test_variant(), &test_embedding());

    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("lstm"));
}

#[test]
fn predict_returns_error_when_nbeats_fails() {
    let transformer = mock_scorer("transformer", 0.8);
    let lstm = mock_scorer("lstm", 0.7);
    let nbeats = mock_scorer_err("nbeats");

    let ensemble = EnsemblePredictor::new(transformer, lstm, nbeats, default_weights());
    let result = ensemble.predict(&test_variant(), &test_embedding());

    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("nbeats"));
}

#[test]
fn all_mocks_returning_half_produces_composite_half() {
    // When all sub-scores = 0.5:
    // safety = 1.0 - 0.5 = 0.5
    // composite = 0.35*0.5 + 0.25*0.5 + 0.25*0.5 + 0.15*(1.0-0.5)
    //           = 0.175 + 0.125 + 0.125 + 0.075 = 0.5
    let transformer = mock_scorer("transformer", 0.5);
    let lstm = mock_scorer("lstm", 0.5);
    let nbeats = mock_scorer("nbeats", 0.5);

    let ensemble = EnsemblePredictor::new(transformer, lstm, nbeats, default_weights());
    let score = ensemble.predict(&test_variant(), &test_embedding()).unwrap();

    assert!(
        (score.composite() - 0.5).abs() < 1e-9,
        "expected 0.5, got {}",
        score.composite()
    );
}

#[test]
fn safety_score_inversion_in_composite() {
    // High sub-scores → low safety_score → higher composite safety contribution
    let transformer = mock_scorer("transformer", 0.9);
    let lstm = mock_scorer("lstm", 0.9);
    let nbeats = mock_scorer("nbeats", 0.9);

    let ensemble = EnsemblePredictor::new(transformer, lstm, nbeats, default_weights());
    let score = ensemble.predict(&test_variant(), &test_embedding()).unwrap();

    // safety = 1.0 - 0.9 = 0.1
    assert!((score.safety_score() - 0.1).abs() < 1e-9);

    // In composite: safety contribution = weight * (1.0 - safety_score) = 0.15 * 0.9 = 0.135
    // Total = 0.35*0.9 + 0.25*0.9 + 0.25*0.9 + 0.15*0.9 = 0.9 * 1.0 = 0.9
    assert!(
        (score.composite() - 0.9).abs() < 1e-9,
        "expected 0.9, got {}",
        score.composite()
    );
}

#[test]
fn safety_score_high_risk_lowers_composite() {
    // Low sub-scores → high safety → lower composite
    let transformer = mock_scorer("transformer", 0.2);
    let lstm = mock_scorer("lstm", 0.3);
    let nbeats = mock_scorer("nbeats", 0.1);

    let ensemble = EnsemblePredictor::new(transformer, lstm, nbeats, default_weights());
    let score = ensemble.predict(&test_variant(), &test_embedding()).unwrap();

    // safety = 1.0 - min(0.2, 0.3, 0.1) = 0.9
    assert!((score.safety_score() - 0.9).abs() < 1e-9);
    // composite should be low
    assert!(score.composite() < 0.3);
}

// ────────────────────────────────────────────────────────────────────
// SubModelScorer model_name tests
// ────────────────────────────────────────────────────────────────────

#[test]
fn model_name_returns_correct_identifier_transformer() {
    let scorer = TransformerScorer::new(0.0);
    assert_eq!(scorer.model_name(), "transformer");
}

#[test]
fn model_name_returns_correct_identifier_lstm() {
    let scorer = LstmScorer::new(0.5);
    assert_eq!(scorer.model_name(), "lstm");
}

#[test]
fn model_name_returns_correct_identifier_nbeats() {
    let scorer = NBeatsScorer::new(0.5);
    assert_eq!(scorer.model_name(), "nbeats");
}

// ────────────────────────────────────────────────────────────────────
// Stub scorers produce valid scores
// ────────────────────────────────────────────────────────────────────

#[test]
fn transformer_scorer_returns_score_in_range() {
    let scorer = TransformerScorer::new(0.0);
    let variant = test_variant();
    let embedding = test_embedding();
    let score = scorer.score(&variant, &embedding).unwrap();
    assert!((0.0..=1.0).contains(&score), "score {score} out of range");
}

#[test]
fn lstm_scorer_returns_score_in_range() {
    let scorer = LstmScorer::new(0.6);
    let variant = test_variant();
    let embedding = test_embedding();
    let score = scorer.score(&variant, &embedding).unwrap();
    assert!((0.0..=1.0).contains(&score), "score {score} out of range");
}

#[test]
fn nbeats_scorer_returns_configured_baseline() {
    let scorer = NBeatsScorer::new(0.75);
    let variant = test_variant();
    let embedding = test_embedding();
    let score = scorer.score(&variant, &embedding).unwrap();
    assert!((score - 0.75).abs() < 1e-9);
}

#[test]
fn nbeats_scorer_clamps_to_unit_range() {
    let scorer = NBeatsScorer::new(1.5);
    let score = scorer.score(&test_variant(), &test_embedding()).unwrap();
    assert!((score - 1.0).abs() < 1e-9);

    let scorer_neg = NBeatsScorer::new(-0.5);
    let score_neg = scorer_neg.score(&test_variant(), &test_embedding()).unwrap();
    assert!(score_neg.abs() < 1e-9);
}

// ────────────────────────────────────────────────────────────────────
// EnsemblePredictor with real stub scorers
// ────────────────────────────────────────────────────────────────────

#[test]
fn ensemble_with_real_stubs_produces_valid_fitness_score() {
    let transformer = TransformerScorer::new(0.0);
    let lstm = LstmScorer::new(0.6);
    let nbeats = NBeatsScorer::new(0.7);

    let ensemble = EnsemblePredictor::new(transformer, lstm, nbeats, default_weights());
    let score = ensemble.predict(&test_variant(), &test_embedding()).unwrap();

    assert!((0.0..=1.0).contains(&score.reprogramming_efficiency()));
    assert!((0.0..=1.0).contains(&score.expression_stability()));
    assert!((0.0..=1.0).contains(&score.structural_plausibility()));
    assert!((0.0..=1.0).contains(&score.safety_score()));
    assert!((0.0..=1.0).contains(&score.composite()));
}

// ────────────────────────────────────────────────────────────────────
// ModelLoader + SegmentProducer round-trip
// ────────────────────────────────────────────────────────────────────

#[test]
fn model_weights_round_trip_serialize_deserialize() {
    let weights = ModelWeights {
        transformer: TransformerScorer::new(0.1),
        lstm: LstmScorer::new(0.6),
        nbeats: NBeatsScorer::new(0.75),
    };

    let bytes = weights.to_bytes().unwrap();
    let restored = ModelWeights::from_bytes(&bytes).unwrap();

    // Verify via scoring: same outputs
    let variant = test_variant();
    let embedding = test_embedding();

    let t1 = weights.transformer.score(&variant, &embedding).unwrap();
    let t2 = restored.transformer.score(&variant, &embedding).unwrap();
    assert!((t1 - t2).abs() < 1e-9);

    let l1 = weights.lstm.score(&variant, &embedding).unwrap();
    let l2 = restored.lstm.score(&variant, &embedding).unwrap();
    assert!((l1 - l2).abs() < 1e-9);

    let n1 = weights.nbeats.score(&variant, &embedding).unwrap();
    let n2 = restored.nbeats.score(&variant, &embedding).unwrap();
    assert!((n1 - n2).abs() < 1e-9);
}

#[test]
fn quant_seg_producer_returns_quant_seg_type() {
    let weights = ModelWeights {
        transformer: TransformerScorer::new(0.0),
        lstm: LstmScorer::new(0.5),
        nbeats: NBeatsScorer::new(0.5),
    };
    let producer = QuantSegProducer::new(weights);

    assert_eq!(producer.segment_type(), pe_rvf::SegmentType::QuantSeg);
    let data = producer.produce().unwrap();
    assert!(!data.is_empty());
}

#[test]
fn quant_seg_producer_output_deserializes_to_model_weights() {
    let weights = ModelWeights {
        transformer: TransformerScorer::new(0.05),
        lstm: LstmScorer::new(0.65),
        nbeats: NBeatsScorer::new(0.8),
    };
    let producer = QuantSegProducer::new(weights);
    let data = producer.produce().unwrap();

    let restored = ModelWeights::from_bytes(&data).unwrap();
    assert_eq!(restored.nbeats.model_name(), "nbeats");
}
