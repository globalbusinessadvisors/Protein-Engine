//! Comprehensive London School TDD unit tests for pe-core.
//!
//! pe-core is a leaf crate with no dependencies to mock — all tests exercise
//! the public API directly against real domain types.

use alloc::collections::BTreeMap;
use alloc::string::{String, ToString};
use crate::*;

// ===========================================================================
// AminoAcid tests
// ===========================================================================

#[test]
fn test_amino_acid_from_valid_char() {
    let valid = "ACDEFGHIKLMNPQRSTVWY";
    for c in valid.chars() {
        let aa = AminoAcid::from_char(c);
        assert!(aa.is_ok(), "Expected valid amino acid for '{}'", c);
    }
}

#[test]
fn test_amino_acid_from_lowercase() {
    let lowercase = "acdefghiklmnpqrstvwy";
    let uppercase = "ACDEFGHIKLMNPQRSTVWY";
    for (lo, up) in lowercase.chars().zip(uppercase.chars()) {
        let aa_lo = AminoAcid::from_char(lo).unwrap();
        let aa_up = AminoAcid::from_char(up).unwrap();
        assert_eq!(aa_lo, aa_up, "Lowercase '{}' should match uppercase '{}'", lo, up);
    }
}

#[test]
fn test_amino_acid_from_invalid_char() {
    for c in ['X', 'B', 'Z', '1', ' '] {
        let result = AminoAcid::from_char(c);
        assert!(result.is_err(), "Expected error for invalid char '{}'", c);
        assert_eq!(result.unwrap_err(), CoreError::InvalidResidue(c));
    }
}

#[test]
fn test_amino_acid_roundtrip() {
    let chars = "ACDEFGHIKLMNPQRSTVWY";
    for c in chars.chars() {
        let aa = AminoAcid::from_char(c).unwrap();
        assert_eq!(aa.to_char(), c, "Roundtrip failed for '{}'", c);
    }
}

// ===========================================================================
// AminoAcidSequence tests
// ===========================================================================

#[test]
fn test_sequence_valid() {
    let seq = AminoAcidSequence::new("MKWVTFISLLLLFSSAYS");
    assert!(seq.is_ok());
}

#[test]
fn test_sequence_empty_rejected() {
    let seq = AminoAcidSequence::new("");
    assert_eq!(seq.unwrap_err(), CoreError::EmptySequence);
}

#[test]
fn test_sequence_invalid_char_rejected() {
    let seq = AminoAcidSequence::new("MKXV");
    assert_eq!(seq.unwrap_err(), CoreError::InvalidResidue('X'));
}

#[test]
fn test_sequence_len() {
    let seq = AminoAcidSequence::new("MKWV").unwrap();
    assert_eq!(seq.len(), 4);
}

#[test]
fn test_sequence_deref() {
    let seq = AminoAcidSequence::new("MKW").unwrap();
    // Deref allows using as slice
    let slice: &[AminoAcid] = &*seq;
    assert_eq!(slice.len(), 3);
    assert_eq!(slice[0], AminoAcid::from_char('M').unwrap());
    assert_eq!(slice[1], AminoAcid::from_char('K').unwrap());
    assert_eq!(slice[2], AminoAcid::from_char('W').unwrap());
}

#[test]
fn test_sequence_display() {
    let input = "MKWVTFIS";
    let seq = AminoAcidSequence::new(input).unwrap();
    let displayed = alloc::format!("{}", seq);
    assert_eq!(displayed, input);
}

#[test]
fn test_sequence_residue_at() {
    let seq = AminoAcidSequence::new("MKW").unwrap();
    assert_eq!(seq.residue_at(0), Some(AminoAcid::from_char('M').unwrap()));
    assert_eq!(seq.residue_at(1), Some(AminoAcid::from_char('K').unwrap()));
    assert_eq!(seq.residue_at(2), Some(AminoAcid::from_char('W').unwrap()));
    assert_eq!(seq.residue_at(3), None);
    assert_eq!(seq.residue_at(100), None);
}

// ===========================================================================
// YamanakaFactor tests
// ===========================================================================

#[test]
fn test_yamanaka_factor_display() {
    assert_eq!(alloc::format!("{}", YamanakaFactor::OCT4), "OCT4");
    assert_eq!(alloc::format!("{}", YamanakaFactor::SOX2), "SOX2");
    assert_eq!(alloc::format!("{}", YamanakaFactor::KLF4), "KLF4");
    assert_eq!(alloc::format!("{}", YamanakaFactor::CMYC), "CMYC");
}

#[test]
fn test_yamanaka_factor_equality() {
    assert_eq!(YamanakaFactor::OCT4, YamanakaFactor::OCT4);
    assert_ne!(YamanakaFactor::OCT4, YamanakaFactor::SOX2);
    assert_ne!(YamanakaFactor::KLF4, YamanakaFactor::CMYC);
}

// ===========================================================================
// Mutation tests
// ===========================================================================

#[test]
fn test_mutation_valid() {
    let from = AminoAcid::from_char('M').unwrap();
    let to = AminoAcid::from_char('A').unwrap();
    let mutation = Mutation::new(0, from, to);
    assert!(mutation.is_ok());
}

#[test]
fn test_mutation_identical_residues_rejected() {
    let aa = AminoAcid::from_char('M').unwrap();
    let result = Mutation::new(0, aa, aa);
    assert_eq!(result.unwrap_err(), CoreError::IdenticalMutationResidues);
}

#[test]
fn test_mutation_apply_valid() {
    let seq = AminoAcidSequence::new("MKW").unwrap();
    let mutation = Mutation::new(
        0,
        AminoAcid::from_char('M').unwrap(),
        AminoAcid::from_char('A').unwrap(),
    )
    .unwrap();

    let new_seq = mutation.apply(&seq).unwrap();
    assert_eq!(alloc::format!("{}", new_seq), "AKW");
}

#[test]
fn test_mutation_apply_out_of_bounds() {
    let seq = AminoAcidSequence::new("MKW").unwrap();
    let mutation = Mutation::new(
        10,
        AminoAcid::from_char('M').unwrap(),
        AminoAcid::from_char('A').unwrap(),
    )
    .unwrap();

    let result = mutation.apply(&seq);
    assert!(result.is_err());
    assert_eq!(
        result.unwrap_err(),
        CoreError::MutationOutOfBounds {
            position: 10,
            length: 3,
        }
    );
}

#[test]
fn test_mutation_apply_residue_mismatch() {
    let seq = AminoAcidSequence::new("MKW").unwrap();
    // Position 0 is 'M', but we claim from_residue is 'K'
    let mutation = Mutation::new(
        0,
        AminoAcid::from_char('K').unwrap(),
        AminoAcid::from_char('A').unwrap(),
    )
    .unwrap();

    let result = mutation.apply(&seq);
    assert!(result.is_err());
    assert_eq!(
        result.unwrap_err(),
        CoreError::MutationResidueMismatch {
            position: 0,
            expected: 'K',
            actual: 'M',
        }
    );
}

#[test]
fn test_mutation_getters() {
    let from = AminoAcid::from_char('M').unwrap();
    let to = AminoAcid::from_char('A').unwrap();
    let mutation = Mutation::new(5, from, to).unwrap();

    assert_eq!(mutation.position(), 5);
    assert_eq!(mutation.from_residue(), from);
    assert_eq!(mutation.to_residue(), to);
}

// ===========================================================================
// ProteinVariant tests
// ===========================================================================

#[test]
fn test_wild_type_generation_zero() {
    let seq = AminoAcidSequence::new("MKWV").unwrap();
    let wt = ProteinVariant::wild_type("WT-OCT4", seq, YamanakaFactor::OCT4);
    assert_eq!(wt.generation(), 0);
    assert!(wt.parent_id().is_none());
    assert!(wt.mutations().is_empty());
}

#[test]
fn test_wild_type_has_correct_fields() {
    let seq = AminoAcidSequence::new("MKWV").unwrap();
    let wt = ProteinVariant::wild_type("WT-OCT4", seq.clone(), YamanakaFactor::OCT4);
    assert_eq!(wt.name(), "WT-OCT4");
    assert_eq!(wt.sequence(), &seq);
    assert_eq!(*wt.target_factor(), YamanakaFactor::OCT4);
}

#[test]
fn test_from_mutation_increments_generation() {
    let seq = AminoAcidSequence::new("MKWV").unwrap();
    let wt = ProteinVariant::wild_type("WT", seq, YamanakaFactor::OCT4);

    let mutation = Mutation::new(
        0,
        AminoAcid::from_char('M').unwrap(),
        AminoAcid::from_char('A').unwrap(),
    )
    .unwrap();

    let child = ProteinVariant::from_mutation(&wt, mutation).unwrap();
    assert_eq!(child.generation(), wt.generation() + 1);
    assert_eq!(child.generation(), 1);
}

#[test]
fn test_from_mutation_sets_parent_id() {
    let seq = AminoAcidSequence::new("MKWV").unwrap();
    let wt = ProteinVariant::wild_type("WT", seq, YamanakaFactor::OCT4);
    let parent_id = wt.id();

    let mutation = Mutation::new(
        0,
        AminoAcid::from_char('M').unwrap(),
        AminoAcid::from_char('A').unwrap(),
    )
    .unwrap();

    let child = ProteinVariant::from_mutation(&wt, mutation).unwrap();
    assert_eq!(child.parent_id(), Some(parent_id));
}

#[test]
fn test_from_mutation_accumulates_mutations() {
    let seq = AminoAcidSequence::new("MKWV").unwrap();
    let wt = ProteinVariant::wild_type("WT", seq, YamanakaFactor::OCT4);
    assert_eq!(wt.mutations().len(), 0);

    let m1 = Mutation::new(
        0,
        AminoAcid::from_char('M').unwrap(),
        AminoAcid::from_char('A').unwrap(),
    )
    .unwrap();
    let gen1 = ProteinVariant::from_mutation(&wt, m1).unwrap();
    assert_eq!(gen1.mutations().len(), 1);

    // Second mutation on the child
    let m2 = Mutation::new(
        1,
        AminoAcid::from_char('K').unwrap(),
        AminoAcid::from_char('R').unwrap(),
    )
    .unwrap();
    let gen2 = ProteinVariant::from_mutation(&gen1, m2).unwrap();
    assert_eq!(gen2.mutations().len(), 2);
}

#[test]
fn test_from_mutation_invalid_position() {
    let seq = AminoAcidSequence::new("MKWV").unwrap();
    let wt = ProteinVariant::wild_type("WT", seq, YamanakaFactor::OCT4);

    let mutation = Mutation::new(
        100,
        AminoAcid::from_char('M').unwrap(),
        AminoAcid::from_char('A').unwrap(),
    )
    .unwrap();

    let result = ProteinVariant::from_mutation(&wt, mutation);
    assert!(result.is_err());
}

#[test]
fn test_from_mutation_residue_mismatch() {
    let seq = AminoAcidSequence::new("MKWV").unwrap();
    let wt = ProteinVariant::wild_type("WT", seq, YamanakaFactor::OCT4);

    // Position 0 is 'M', but we say from_residue is 'K'
    let mutation = Mutation::new(
        0,
        AminoAcid::from_char('K').unwrap(),
        AminoAcid::from_char('A').unwrap(),
    )
    .unwrap();

    let result = ProteinVariant::from_mutation(&wt, mutation);
    assert!(result.is_err());
}

#[test]
fn test_from_crossover_valid() {
    let seq_a = AminoAcidSequence::new("MMMM").unwrap();
    let seq_b = AminoAcidSequence::new("AAAA").unwrap();
    let parent_a = ProteinVariant::wild_type("A", seq_a, YamanakaFactor::OCT4);
    let parent_b = ProteinVariant::wild_type("B", seq_b, YamanakaFactor::OCT4);

    let child = ProteinVariant::from_crossover(&parent_a, &parent_b, 2).unwrap();
    let child_seq = alloc::format!("{}", child.sequence());
    assert_eq!(child_seq, "MMAA");
}

#[test]
fn test_from_crossover_length_mismatch() {
    let seq_a = AminoAcidSequence::new("MMMM").unwrap();
    let seq_b = AminoAcidSequence::new("AAA").unwrap();
    let parent_a = ProteinVariant::wild_type("A", seq_a, YamanakaFactor::OCT4);
    let parent_b = ProteinVariant::wild_type("B", seq_b, YamanakaFactor::OCT4);

    let result = ProteinVariant::from_crossover(&parent_a, &parent_b, 2);
    assert!(result.is_err());
}

#[test]
fn test_from_crossover_point_out_of_bounds() {
    let seq_a = AminoAcidSequence::new("MMMM").unwrap();
    let seq_b = AminoAcidSequence::new("AAAA").unwrap();
    let parent_a = ProteinVariant::wild_type("A", seq_a, YamanakaFactor::OCT4);
    let parent_b = ProteinVariant::wild_type("B", seq_b, YamanakaFactor::OCT4);

    // crossover_point >= length should fail
    let result = ProteinVariant::from_crossover(&parent_a, &parent_b, 4);
    assert!(result.is_err());

    let result2 = ProteinVariant::from_crossover(&parent_a, &parent_b, 100);
    assert!(result2.is_err());
}

#[test]
fn test_from_crossover_point_zero() {
    let seq_a = AminoAcidSequence::new("MMMM").unwrap();
    let seq_b = AminoAcidSequence::new("AAAA").unwrap();
    let parent_a = ProteinVariant::wild_type("A", seq_a, YamanakaFactor::OCT4);
    let parent_b = ProteinVariant::wild_type("B", seq_b, YamanakaFactor::OCT4);

    let result = ProteinVariant::from_crossover(&parent_a, &parent_b, 0);
    assert!(result.is_err());
}

#[test]
fn test_from_crossover_generation() {
    let seq_a = AminoAcidSequence::new("MMMM").unwrap();
    let seq_b = AminoAcidSequence::new("AAAA").unwrap();
    let parent_a = ProteinVariant::wild_type("A", seq_a.clone(), YamanakaFactor::OCT4);

    // Create a gen-1 variant for parent_b by mutating
    let parent_b_wt = ProteinVariant::wild_type("B", seq_b, YamanakaFactor::OCT4);
    let m = Mutation::new(
        0,
        AminoAcid::from_char('A').unwrap(),
        AminoAcid::from_char('G').unwrap(),
    )
    .unwrap();
    let parent_b_gen1 = ProteinVariant::from_mutation(&parent_b_wt, m).unwrap();
    assert_eq!(parent_b_gen1.generation(), 1);

    // Crossover: max(0, 1) + 1 = 2
    // Need same-length sequences for crossover
    let seq_c = AminoAcidSequence::new("GGGG").unwrap();
    let parent_c = ProteinVariant::wild_type("C", seq_c, YamanakaFactor::OCT4);
    let m2 = Mutation::new(
        0,
        AminoAcid::from_char('G').unwrap(),
        AminoAcid::from_char('A').unwrap(),
    )
    .unwrap();
    let parent_c_gen1 = ProteinVariant::from_mutation(&parent_c, m2).unwrap();

    let child = ProteinVariant::from_crossover(&parent_a, &parent_c_gen1, 2).unwrap();
    // max(0, 1) + 1 = 2
    assert_eq!(child.generation(), 2);
}

// ===========================================================================
// FitnessWeights tests
// ===========================================================================

#[test]
fn test_weights_valid() {
    let w = FitnessWeights::new(0.35, 0.25, 0.25, 0.15);
    assert!(w.is_ok());
}

#[test]
fn test_weights_not_sum_to_one() {
    let w = FitnessWeights::new(0.5, 0.5, 0.5, 0.5);
    assert!(w.is_err());
}

#[test]
fn test_default_weights() {
    let w = FitnessWeights::default_weights();
    let sum = w.reprogramming + w.stability + w.plausibility + w.safety;
    assert!((sum - 1.0).abs() < 1e-9, "Default weights must sum to 1.0, got {}", sum);
}

// ===========================================================================
// FitnessScore tests
// ===========================================================================

#[test]
fn test_fitness_score_valid() {
    let weights = FitnessWeights::default_weights();
    let score = FitnessScore::new(0.8, 0.7, 0.9, 0.2, &weights);
    assert!(score.is_ok());
    let s = score.unwrap();
    assert!((s.reprogramming_efficiency() - 0.8).abs() < 1e-9);
    assert!((s.expression_stability() - 0.7).abs() < 1e-9);
    assert!((s.structural_plausibility() - 0.9).abs() < 1e-9);
    assert!((s.safety_score() - 0.2).abs() < 1e-9);
}

#[test]
fn test_fitness_score_out_of_range() {
    let weights = FitnessWeights::default_weights();
    let result = FitnessScore::new(1.5, 0.7, 0.9, 0.2, &weights);
    assert!(result.is_err());
}

#[test]
fn test_fitness_score_negative() {
    let weights = FitnessWeights::default_weights();
    let result = FitnessScore::new(-0.1, 0.7, 0.9, 0.2, &weights);
    assert!(result.is_err());
}

#[test]
fn test_fitness_score_nan() {
    let weights = FitnessWeights::default_weights();
    let result = FitnessScore::new(f64::NAN, 0.7, 0.9, 0.2, &weights);
    assert!(result.is_err());
}

#[test]
fn test_fitness_score_safety_inverted() {
    let weights = FitnessWeights::default_weights();
    // Lower safety_score should give higher composite (safety contributes 1.0 - safety_score)
    let low_safety = FitnessScore::new(0.5, 0.5, 0.5, 0.1, &weights).unwrap();
    let high_safety = FitnessScore::new(0.5, 0.5, 0.5, 0.9, &weights).unwrap();
    assert!(
        low_safety.composite() > high_safety.composite(),
        "Lower safety_score should yield higher composite: {} vs {}",
        low_safety.composite(),
        high_safety.composite()
    );
}

#[test]
fn test_fitness_score_composite_calculation() {
    let weights = FitnessWeights::new(0.35, 0.25, 0.25, 0.15).unwrap();
    let score = FitnessScore::new(0.8, 0.7, 0.9, 0.2, &weights).unwrap();
    // composite = 0.35*0.8 + 0.25*0.7 + 0.25*0.9 + 0.15*(1.0-0.2)
    //           = 0.28 + 0.175 + 0.225 + 0.12
    //           = 0.8
    let expected = 0.8;
    assert!(
        (score.composite() - expected).abs() < 1e-9,
        "Expected composite {}, got {}",
        expected,
        score.composite()
    );
}

#[test]
fn test_fitness_score_all_half() {
    let weights = FitnessWeights::new(0.25, 0.25, 0.25, 0.25).unwrap();
    let score = FitnessScore::new(0.5, 0.5, 0.5, 0.5, &weights).unwrap();
    // composite = 0.25*0.5 + 0.25*0.5 + 0.25*0.5 + 0.25*(1.0-0.5)
    //           = 0.125 + 0.125 + 0.125 + 0.125
    //           = 0.5
    let expected = 0.5;
    assert!(
        (score.composite() - expected).abs() < 1e-9,
        "Expected composite {}, got {}",
        expected,
        score.composite()
    );
}

#[test]
fn test_fitness_score_equality() {
    let weights = FitnessWeights::default_weights();
    let score1 = FitnessScore::new(0.8, 0.7, 0.9, 0.2, &weights).unwrap();
    let score2 = FitnessScore::new(0.8, 0.7, 0.9, 0.2, &weights).unwrap();
    assert_eq!(score1, score2);
}

// ===========================================================================
// ExperimentResult tests
// ===========================================================================

#[test]
fn test_experiment_result_valid() {
    let mut values = BTreeMap::new();
    values.insert("expression_level".to_string(), 0.85);
    values.insert("fold_change".to_string(), 2.3);

    let variant_id = uuid::Uuid::new_v4();
    let timestamp = chrono::Utc::now();

    let result = ExperimentResult::new(
        variant_id,
        AssayType::FlowCytometry,
        values,
        timestamp,
        "instrument-001".to_string(),
        None,
    );
    assert!(result.is_ok());
    let r = result.unwrap();
    assert_eq!(r.variant_id(), variant_id);
    assert_eq!(*r.assay_type(), AssayType::FlowCytometry);
    assert_eq!(r.instrument_id(), "instrument-001");
    assert!(r.notes().is_none());
}

#[test]
fn test_experiment_result_empty_values() {
    let values = BTreeMap::new();
    let result = ExperimentResult::new(
        uuid::Uuid::new_v4(),
        AssayType::QPCR,
        values,
        chrono::Utc::now(),
        "instr-1".to_string(),
        None,
    );
    assert_eq!(result.unwrap_err(), CoreError::EmptyMeasuredValues);
}

#[test]
fn test_experiment_result_nan_value() {
    let mut values = BTreeMap::new();
    values.insert("bad_metric".to_string(), f64::NAN);

    let result = ExperimentResult::new(
        uuid::Uuid::new_v4(),
        AssayType::PlateReader,
        values,
        chrono::Utc::now(),
        "instr-1".to_string(),
        None,
    );
    assert!(result.is_err());
    assert_eq!(
        result.unwrap_err(),
        CoreError::NonFiniteMeasuredValue {
            key: "bad_metric".to_string(),
        }
    );
}

#[test]
fn test_experiment_result_infinite_value() {
    let mut values = BTreeMap::new();
    values.insert("inf_metric".to_string(), f64::INFINITY);

    let result = ExperimentResult::new(
        uuid::Uuid::new_v4(),
        AssayType::CellViability,
        values,
        chrono::Utc::now(),
        "instr-1".to_string(),
        None,
    );
    assert!(result.is_err());
    assert_eq!(
        result.unwrap_err(),
        CoreError::NonFiniteMeasuredValue {
            key: "inf_metric".to_string(),
        }
    );
}

#[test]
fn test_experiment_result_empty_instrument_id() {
    let mut values = BTreeMap::new();
    values.insert("metric".to_string(), 1.0);

    let result = ExperimentResult::new(
        uuid::Uuid::new_v4(),
        AssayType::WesternBlot,
        values,
        chrono::Utc::now(),
        String::new(),
        None,
    );
    assert_eq!(result.unwrap_err(), CoreError::EmptyInstrumentId);
}

// ===========================================================================
// Embedding320 tests
// ===========================================================================

#[test]
fn test_embedding_zeros() {
    let emb = Embedding320::zeros();
    for &val in emb.as_slice() {
        assert_eq!(val, 0.0_f32);
    }
}

#[test]
fn test_embedding_cosine_similarity_identical() {
    let mut data = [0.0_f32; 320];
    data[0] = 1.0;
    data[1] = 2.0;
    data[2] = 3.0;
    let emb = Embedding320::new(data);
    let sim = emb.cosine_similarity(&emb);
    assert!(
        (sim - 1.0_f32).abs() < 1e-6,
        "Identical vectors should have cosine similarity 1.0, got {}",
        sim
    );
}

#[test]
fn test_embedding_cosine_similarity_orthogonal() {
    let mut data_a = [0.0_f32; 320];
    data_a[0] = 1.0;
    let emb_a = Embedding320::new(data_a);

    let mut data_b = [0.0_f32; 320];
    data_b[1] = 1.0;
    let emb_b = Embedding320::new(data_b);

    let sim = emb_a.cosine_similarity(&emb_b);
    assert!(
        sim.abs() < 1e-6,
        "Orthogonal vectors should have cosine similarity 0.0, got {}",
        sim
    );
}

#[test]
fn test_embedding_cosine_similarity_zero_norm() {
    let zero = Embedding320::zeros();
    let mut data = [0.0_f32; 320];
    data[0] = 1.0;
    let nonzero = Embedding320::new(data);

    let sim = zero.cosine_similarity(&nonzero);
    assert_eq!(sim, 0.0_f32, "Zero vector should yield cosine similarity 0.0");

    let sim2 = nonzero.cosine_similarity(&zero);
    assert_eq!(sim2, 0.0_f32, "Cosine with zero vector should be 0.0");
}

#[test]
fn test_embedding_norm() {
    let mut data = [0.0_f32; 320];
    data[0] = 3.0;
    data[1] = 4.0;
    let emb = Embedding320::new(data);
    let norm = emb.norm();
    // sqrt(9 + 16) = sqrt(25) = 5.0
    assert!(
        (norm - 5.0_f32).abs() < 1e-6,
        "Expected norm 5.0, got {}",
        norm
    );
}

#[test]
fn test_embedding_dot() {
    let mut data_a = [0.0_f32; 320];
    data_a[0] = 1.0;
    data_a[1] = 2.0;
    data_a[2] = 3.0;
    let emb_a = Embedding320::new(data_a);

    let mut data_b = [0.0_f32; 320];
    data_b[0] = 4.0;
    data_b[1] = 5.0;
    data_b[2] = 6.0;
    let emb_b = Embedding320::new(data_b);

    let dot = emb_a.dot(&emb_b);
    // 1*4 + 2*5 + 3*6 = 4 + 10 + 18 = 32
    assert!(
        (dot - 32.0_f32).abs() < 1e-6,
        "Expected dot product 32.0, got {}",
        dot
    );
}

#[test]
fn test_embedding_dim() {
    assert_eq!(Embedding320::dim(), 320);
}
