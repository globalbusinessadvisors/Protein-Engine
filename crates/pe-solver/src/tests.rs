use crate::error::SolverError;
use crate::gradient::SimpleGradientSolver;
use crate::landscape::EnergyLandscape;
use crate::result::MinimizationResult;
use crate::segment::SolverSegmentProducer;
use crate::sublinear::SublinearSolver;
use crate::traits::{EnergySolver, MockEnergySolver};
use pe_rvf::SegmentProducer;

// ────────────────────────────────────────────────────────────────────
// EnergyLandscape validation
// ────────────────────────────────────────────────────────────────────

#[test]
fn landscape_rejects_zero_dimensions() {
    let result = EnergyLandscape::new(0, vec![]);
    assert!(matches!(result, Err(SolverError::ZeroDimensions)));
}

#[test]
fn landscape_rejects_out_of_bounds_coordinate() {
    let result = EnergyLandscape::new(3, vec![(vec![5], -1.0)]);
    assert!(matches!(
        result,
        Err(SolverError::CoordinateOutOfBounds { .. })
    ));
}

#[test]
fn landscape_accepts_valid_entries() {
    let landscape = EnergyLandscape::new(5, vec![(vec![0, 2, 4], -3.5)]).unwrap();
    assert_eq!(landscape.dimensions(), 5);
    assert_eq!(landscape.sparse_entries().len(), 1);
}

#[test]
fn landscape_empty_has_zero_energy() {
    let landscape = EnergyLandscape::new(3, vec![]).unwrap();
    assert!(landscape.is_empty());
    assert_eq!(landscape.evaluate(&[0.0, 0.0, 0.0]), 0.0);
}

// ────────────────────────────────────────────────────────────────────
// MockEnergySolver (London School)
// ────────────────────────────────────────────────────────────────────

#[test]
fn mock_solver_returns_canned_result() {
    let expected = MinimizationResult {
        minimum_energy: -5.0,
        optimal_coordinates: vec![1.0, 0.0, 1.0],
        iterations: 42,
        converged: true,
    };
    let expected_clone = expected.clone();

    let mut mock = MockEnergySolver::new();
    mock.expect_minimize()
        .returning(move |_| Ok(expected_clone.clone()));

    let landscape = EnergyLandscape::new(3, vec![(vec![0, 2], -5.0)]).unwrap();
    let result = mock.minimize(&landscape).unwrap();
    assert_eq!(result, expected);
}

// ────────────────────────────────────────────────────────────────────
// SimpleGradientSolver
// ────────────────────────────────────────────────────────────────────

#[test]
fn gradient_solver_converges_on_convex_surface() {
    // Simple convex landscape: single minimum at coordinate [0]
    // with energy -10.0
    let landscape = EnergyLandscape::new(
        3,
        vec![
            (vec![0], -10.0),
            (vec![1], -2.0),
            (vec![2], -1.0),
        ],
    )
    .unwrap();

    let solver = SimpleGradientSolver::default();
    let result = solver.minimize(&landscape).unwrap();

    assert!(result.converged, "solver should converge");
    assert!(result.iterations > 0, "should take at least one iteration");
    assert!(
        result.minimum_energy <= -2.0,
        "should find energy below -2.0, got {}",
        result.minimum_energy
    );
}

#[test]
fn gradient_solver_handles_empty_landscape() {
    let landscape = EnergyLandscape::new(5, vec![]).unwrap();
    let solver = SimpleGradientSolver::default();
    let result = solver.minimize(&landscape).unwrap();

    assert!(result.converged);
    assert_eq!(result.iterations, 0);
    assert_eq!(result.minimum_energy, 0.0);
    assert_eq!(result.optimal_coordinates.len(), 5);
}

#[test]
fn gradient_solver_result_has_correct_dimension_count() {
    let landscape = EnergyLandscape::new(7, vec![(vec![3], -1.0)]).unwrap();
    let solver = SimpleGradientSolver::default();
    let result = solver.minimize(&landscape).unwrap();

    assert_eq!(result.optimal_coordinates.len(), 7);
}

#[test]
fn gradient_solver_rejects_zero_iterations() {
    let result = SimpleGradientSolver::new(0, 0.01, 1e-8);
    assert!(result.is_err());
}

#[test]
fn gradient_solver_rejects_negative_learning_rate() {
    let result = SimpleGradientSolver::new(100, -0.01, 1e-8);
    assert!(result.is_err());
}

// ────────────────────────────────────────────────────────────────────
// SublinearSolver
// ────────────────────────────────────────────────────────────────────

#[test]
fn sublinear_solver_converges_on_convex_surface() {
    let landscape = EnergyLandscape::new(
        4,
        vec![
            (vec![0], -10.0),
            (vec![1], -3.0),
            (vec![2], -1.0),
            (vec![3], 0.5),
        ],
    )
    .unwrap();

    let solver = SublinearSolver::default();
    let result = solver.minimize(&landscape).unwrap();

    assert!(result.converged, "solver should converge");
    assert!(
        result.minimum_energy <= -3.0,
        "should find energy below -3.0, got {}",
        result.minimum_energy
    );
}

#[test]
fn sublinear_solver_handles_empty_landscape() {
    let landscape = EnergyLandscape::new(3, vec![]).unwrap();
    let solver = SublinearSolver::default();
    let result = solver.minimize(&landscape).unwrap();

    assert!(result.converged);
    assert_eq!(result.iterations, 0);
    assert_eq!(result.minimum_energy, 0.0);
}

#[test]
fn sublinear_solver_finds_global_minimum_entry() {
    // The solver should at minimum find the sparse entry with lowest energy
    let landscape = EnergyLandscape::new(
        5,
        vec![
            (vec![0, 1], 5.0),
            (vec![2, 3], -20.0),
            (vec![4], 1.0),
        ],
    )
    .unwrap();

    let solver = SublinearSolver::default();
    let result = solver.minimize(&landscape).unwrap();

    assert!(
        result.minimum_energy <= -10.0,
        "should find near the global minimum, got {}",
        result.minimum_energy
    );
}

#[test]
fn sublinear_solver_rejects_zero_iterations() {
    let result = SublinearSolver::new(0, 1e-8);
    assert!(result.is_err());
}

#[test]
fn sublinear_solver_iteration_count_is_positive() {
    let landscape = EnergyLandscape::new(2, vec![(vec![0], -1.0)]).unwrap();
    let solver = SublinearSolver::default();
    let result = solver.minimize(&landscape).unwrap();

    assert!(result.iterations > 0);
}

// ────────────────────────────────────────────────────────────────────
// MinimizationResult serialization
// ────────────────────────────────────────────────────────────────────

#[test]
fn minimization_result_round_trip_serialization() {
    let result = MinimizationResult {
        minimum_energy: -7.5,
        optimal_coordinates: vec![0.1, 0.9, 0.3, 0.0, 0.7],
        iterations: 123,
        converged: true,
    };

    let bytes = result.to_bytes().unwrap();
    let restored = MinimizationResult::from_bytes(&bytes).unwrap();
    assert_eq!(result, restored);
}

#[test]
fn minimization_result_round_trip_unconverged() {
    let result = MinimizationResult {
        minimum_energy: 2.5,
        optimal_coordinates: vec![0.5],
        iterations: 1000,
        converged: false,
    };

    let bytes = result.to_bytes().unwrap();
    let restored = MinimizationResult::from_bytes(&bytes).unwrap();
    assert_eq!(result, restored);
}

#[test]
fn minimization_result_deserialize_rejects_garbage() {
    let result = MinimizationResult::from_bytes(b"not json");
    assert!(result.is_err());
}

// ────────────────────────────────────────────────────────────────────
// SegmentProducer
// ────────────────────────────────────────────────────────────────────

#[test]
fn segment_producer_returns_journal_seg_type() {
    let producer = SolverSegmentProducer::new(vec![]);
    assert_eq!(producer.segment_type(), pe_rvf::SegmentType::JournalSeg);
}

#[test]
fn segment_producer_serializes_results() {
    let results = vec![
        MinimizationResult {
            minimum_energy: -5.0,
            optimal_coordinates: vec![1.0, 0.0],
            iterations: 50,
            converged: true,
        },
        MinimizationResult {
            minimum_energy: -3.0,
            optimal_coordinates: vec![0.5, 0.5],
            iterations: 100,
            converged: false,
        },
    ];

    let producer = SolverSegmentProducer::new(results.clone());
    let data = producer.produce().unwrap();

    let restored: Vec<MinimizationResult> = serde_json::from_slice(&data).unwrap();
    assert_eq!(restored, results);
}
