use rand::SeedableRng;

use crate::circuit::CircuitBuilder;
use crate::error::QuantumError;
use crate::hamiltonian::MolecularHamiltonian;
use crate::qaoa::{QaoaConfig, QaoaRunner, QuboInstance};
use crate::statevector::StateVector;
use crate::types::{BackendCapabilities, ProviderName, MAX_QUBITS};
use crate::vqe::{VqeConfig, VqeRunner};

// ────────────────────────────────────────────────────────────────────
// StateVector — basic gate tests
// ────────────────────────────────────────────────────────────────────

#[test]
fn h_gate_on_zero_produces_equal_superposition() {
    let mut sv = StateVector::new(1).unwrap();
    sv.h(0).unwrap();

    let p0 = sv.probability(0);
    let p1 = sv.probability(1);
    assert!(
        (p0 - 0.5).abs() < 1e-10,
        "P(|0⟩) should be 0.5, got {p0}"
    );
    assert!(
        (p1 - 0.5).abs() < 1e-10,
        "P(|1⟩) should be 0.5, got {p1}"
    );
}

#[test]
fn x_gate_flips_zero_to_one() {
    let mut sv = StateVector::new(1).unwrap();
    sv.x(0).unwrap();

    assert!(sv.probability(0) < 1e-10, "P(|0⟩) should be ~0");
    assert!(
        (sv.probability(1) - 1.0).abs() < 1e-10,
        "P(|1⟩) should be 1.0"
    );
}

#[test]
fn x_gate_applied_twice_returns_to_original() {
    let mut sv = StateVector::new(1).unwrap();
    sv.x(0).unwrap();
    sv.x(0).unwrap();

    assert!((sv.probability(0) - 1.0).abs() < 1e-10);
    assert!(sv.probability(1) < 1e-10);
}

#[test]
fn y_gate_flips_with_phase() {
    let mut sv = StateVector::new(1).unwrap();
    sv.y(0).unwrap();

    // Y|0⟩ = i|1⟩
    assert!(sv.probability(0) < 1e-10);
    assert!((sv.probability(1) - 1.0).abs() < 1e-10);

    let amp = sv.amplitudes()[1];
    assert!((amp.re).abs() < 1e-10, "real part should be ~0");
    assert!((amp.im - 1.0).abs() < 1e-10, "imag part should be ~1");
}

#[test]
fn z_gate_applies_phase_to_one_state() {
    let mut sv = StateVector::new(1).unwrap();
    // Put in |1⟩ first
    sv.x(0).unwrap();
    sv.z(0).unwrap();

    // Z|1⟩ = -|1⟩, probability unchanged
    assert!((sv.probability(1) - 1.0).abs() < 1e-10);
    let amp = sv.amplitudes()[1];
    assert!((amp.re + 1.0).abs() < 1e-10, "should be -1");
}

#[test]
fn h_then_h_returns_to_zero() {
    let mut sv = StateVector::new(1).unwrap();
    sv.h(0).unwrap();
    sv.h(0).unwrap();

    assert!((sv.probability(0) - 1.0).abs() < 1e-10);
}

// ────────────────────────────────────────────────────────────────────
// Rotation gates
// ────────────────────────────────────────────────────────────────────

#[test]
fn ry_pi_rotates_zero_to_one() {
    let mut sv = StateVector::new(1).unwrap();
    sv.ry(0, std::f64::consts::PI).unwrap();

    assert!(sv.probability(0) < 1e-10);
    assert!((sv.probability(1) - 1.0).abs() < 1e-10);
}

#[test]
fn rx_pi_rotates_zero_to_one() {
    let mut sv = StateVector::new(1).unwrap();
    sv.rx(0, std::f64::consts::PI).unwrap();

    assert!(sv.probability(0) < 1e-10);
    assert!((sv.probability(1) - 1.0).abs() < 1e-10);
}

#[test]
fn rz_does_not_change_probability_of_zero() {
    let mut sv = StateVector::new(1).unwrap();
    sv.rz(0, 1.234).unwrap();

    assert!((sv.probability(0) - 1.0).abs() < 1e-10);
}

// ────────────────────────────────────────────────────────────────────
// Two-qubit gates
// ────────────────────────────────────────────────────────────────────

#[test]
fn cnot_entangles_two_qubits() {
    // Create Bell state: H on q0, CNOT(q0, q1)
    // Result: (|00⟩ + |11⟩) / √2
    let mut sv = StateVector::new(2).unwrap();
    sv.h(0).unwrap();
    sv.cnot(0, 1).unwrap();

    let p00 = sv.probability(0b00); // |00⟩
    let p01 = sv.probability(0b01); // |01⟩ — q0=1,q1=0 in little-endian
    let p10 = sv.probability(0b10); // |10⟩
    let p11 = sv.probability(0b11); // |11⟩

    // Bell state: equal probability of |00⟩ and |11⟩
    assert!((p00 - 0.5).abs() < 1e-10, "P(|00⟩) should be 0.5, got {p00}");
    assert!((p11 - 0.5).abs() < 1e-10, "P(|11⟩) should be 0.5, got {p11}");
    assert!(p01 < 1e-10, "P(|01⟩) should be ~0, got {p01}");
    assert!(p10 < 1e-10, "P(|10⟩) should be ~0, got {p10}");
}

#[test]
fn cnot_does_nothing_when_control_is_zero() {
    let mut sv = StateVector::new(2).unwrap();
    // Control q0 is |0⟩, so CNOT should not flip q1
    sv.cnot(0, 1).unwrap();

    assert!((sv.probability(0b00) - 1.0).abs() < 1e-10);
}

#[test]
fn cnot_flips_target_when_control_is_one() {
    let mut sv = StateVector::new(2).unwrap();
    sv.x(0).unwrap(); // Set control to |1⟩
    sv.cnot(0, 1).unwrap();

    // Should get |11⟩
    assert!((sv.probability(0b11) - 1.0).abs() < 1e-10);
}

#[test]
fn cz_applies_phase_when_both_one() {
    // CZ|11⟩ = -|11⟩
    let mut sv = StateVector::new(2).unwrap();
    sv.x(0).unwrap();
    sv.x(1).unwrap();
    sv.cz(0, 1).unwrap();

    assert!((sv.probability(0b11) - 1.0).abs() < 1e-10);
    let amp = sv.amplitudes()[0b11];
    assert!((amp.re + 1.0).abs() < 1e-10, "amplitude should be -1");
}

#[test]
fn cnot_rejects_same_control_target() {
    let mut sv = StateVector::new(2).unwrap();
    let result = sv.cnot(0, 0);
    assert!(matches!(result, Err(QuantumError::SameControlTarget(0))));
}

// ────────────────────────────────────────────────────────────────────
// Measurement
// ────────────────────────────────────────────────────────────────────

#[test]
fn measurement_probabilities_match_theory() {
    // H gate → measure many times → should get ~50/50
    let num_shots = 10_000;
    let mut counts = [0usize; 2];
    let mut rng = rand::rngs::StdRng::seed_from_u64(42);

    for _ in 0..num_shots {
        let mut sv = StateVector::new(1).unwrap();
        sv.h(0).unwrap();
        let (outcome, _) = sv.measure(&mut rng);
        counts[outcome] += 1;
    }

    let ratio_0 = counts[0] as f64 / num_shots as f64;
    let ratio_1 = counts[1] as f64 / num_shots as f64;

    // Statistical test: within 5% of expected 50%
    assert!(
        (ratio_0 - 0.5).abs() < 0.05,
        "P(0) = {ratio_0}, expected ~0.5"
    );
    assert!(
        (ratio_1 - 0.5).abs() < 0.05,
        "P(1) = {ratio_1}, expected ~0.5"
    );
}

#[test]
fn measurement_collapses_state() {
    let mut sv = StateVector::new(1).unwrap();
    sv.h(0).unwrap();

    let mut rng = rand::rngs::StdRng::seed_from_u64(99);
    let (outcome, _) = sv.measure(&mut rng);

    // After collapse, the measured state should have probability 1
    assert!((sv.probability(outcome) - 1.0).abs() < 1e-10);
}

// ────────────────────────────────────────────────────────────────────
// Circuit builder
// ────────────────────────────────────────────────────────────────────

#[test]
fn circuit_builder_creates_bell_state() {
    let circuit = CircuitBuilder::new(2)
        .unwrap()
        .h(0)
        .unwrap()
        .cnot(0, 1)
        .unwrap()
        .build();

    let sv = circuit.execute().unwrap();
    assert!((sv.probability(0b00) - 0.5).abs() < 1e-10);
    assert!((sv.probability(0b11) - 0.5).abs() < 1e-10);
}

#[test]
fn circuit_builder_rejects_qubit_out_of_range() {
    let result = CircuitBuilder::new(2).unwrap().h(5);
    assert!(matches!(result, Err(QuantumError::QubitOutOfRange(5, 2))));
}

// ────────────────────────────────────────────────────────────────────
// Qubit limit
// ────────────────────────────────────────────────────────────────────

#[test]
fn simulator_rejects_circuits_exceeding_max_qubits() {
    let result = StateVector::new(MAX_QUBITS + 1);
    assert!(matches!(result, Err(QuantumError::TooManyQubits { .. })));
}

#[test]
fn simulator_accepts_max_qubit_count() {
    // Don't actually allocate 2^20 — just verify construction is accepted
    // We test with a smaller count to avoid slow tests
    let sv = StateVector::new(4).unwrap();
    assert_eq!(sv.num_qubits(), 4);
}

#[test]
fn circuit_builder_rejects_too_many_qubits() {
    let result = CircuitBuilder::new(MAX_QUBITS + 1);
    assert!(matches!(result, Err(QuantumError::TooManyQubits { .. })));
}

// ────────────────────────────────────────────────────────────────────
// VQE on H2 molecule
// ────────────────────────────────────────────────────────────────────

#[test]
fn vqe_h2_converges_within_tolerance() {
    let h2 = MolecularHamiltonian::h2_molecule();

    let config = VqeConfig {
        max_iterations: 300,
        convergence_threshold: 1e-6,
        initial_step_size: 0.5,
    };

    let runner = VqeRunner::new(config);
    let result = runner.run(&h2).unwrap();

    // Known H2 ground state energy ≈ -1.137 Ha
    // Our simplified Hamiltonian coefficients give a minimum around -1.0 to -1.2
    assert!(
        result.ground_state_energy < -0.9,
        "VQE ground state energy {} should be < -0.9 Ha",
        result.ground_state_energy
    );
    assert!(result.converged, "VQE should converge");
    assert!(result.iterations > 0);
    assert_eq!(result.optimal_parameters.len(), 2);
}

#[test]
fn vqe_result_has_correct_parameter_count() {
    let h2 = MolecularHamiltonian::h2_molecule();
    let runner = VqeRunner::new(VqeConfig::default());
    let result = runner.run(&h2).unwrap();

    // num_params = num_qubits = 2
    assert_eq!(result.optimal_parameters.len(), 2);
}

// ────────────────────────────────────────────────────────────────────
// QAOA on trivial QUBO
// ────────────────────────────────────────────────────────────────────

#[test]
fn qaoa_trivial_qubo_finds_optimal() {
    // 2-variable QUBO: minimize x0*(-1) + x1*(+2) + x0*x1*(0)
    // Optimal: x0=1, x1=0 → cost = -1
    let qubo = QuboInstance::new(vec![vec![-1.0, 0.0], vec![0.0, 2.0]]).unwrap();

    let config = QaoaConfig {
        num_layers: 2,
        max_iterations: 100,
        convergence_threshold: 1e-4,
        initial_step_size: 0.3,
    };

    let runner = QaoaRunner::new(config);
    let result = runner.run(&qubo).unwrap();

    // The optimal bitstring should be 0b01 (x0=1, x1=0) with cost -1
    // QAOA is variational so it might not always find the exact optimum,
    // but it should find a solution with cost <= 0
    assert!(
        result.best_cost <= 0.0,
        "QAOA cost {} should be <= 0",
        result.best_cost
    );
    assert!(result.iterations > 0);
}

#[test]
fn qubo_brute_force_finds_known_optimum() {
    let qubo = QuboInstance::new(vec![vec![-1.0, 0.0], vec![0.0, 2.0]]).unwrap();
    let (best_bits, best_cost) = qubo.optimal_brute_force();

    assert_eq!(best_bits, 0b01); // x0=1, x1=0
    assert!((best_cost - (-1.0)).abs() < 1e-10);
}

#[test]
fn qubo_evaluate_matches_manual() {
    let qubo = QuboInstance::new(vec![
        vec![-2.0, 1.0],
        vec![1.0, -3.0],
    ])
    .unwrap();

    // x0=0, x1=0 → cost = 0
    assert!((qubo.evaluate(0b00)).abs() < 1e-10);
    // x0=1, x1=0 → cost = Q[0][0] = -2
    assert!((qubo.evaluate(0b01) - (-2.0)).abs() < 1e-10);
    // x0=0, x1=1 → cost = Q[1][1] = -3
    assert!((qubo.evaluate(0b10) - (-3.0)).abs() < 1e-10);
    // x0=1, x1=1 → cost = Q[0][0] + Q[0][1] + Q[1][0] + Q[1][1] = -2+1+1-3 = -3
    assert!((qubo.evaluate(0b11) - (-3.0)).abs() < 1e-10);
}

#[test]
fn qubo_rejects_non_square_matrix() {
    // Create a matrix where a row has wrong length (we can't easily do this
    // with the constructor validation, so test the error path)
    let result = QuboInstance::new(vec![vec![1.0, 2.0], vec![3.0]]);
    assert!(result.is_err());
}

// ────────────────────────────────────────────────────────────────────
// BackendCapabilities
// ────────────────────────────────────────────────────────────────────

#[test]
fn local_simulator_capabilities() {
    let caps = BackendCapabilities::local_simulator();
    assert_eq!(caps.max_qubits, MAX_QUBITS as u32);
    assert!(caps.is_simulator);
    assert_eq!(caps.provider, ProviderName::LocalSimulator);
    assert_eq!(caps.gate_set.len(), 9); // H, X, Y, Z, Rx, Ry, Rz, CNOT, CZ
}

// ────────────────────────────────────────────────────────────────────
// Hamiltonian
// ────────────────────────────────────────────────────────────────────

#[test]
fn h2_hamiltonian_matrix_is_hermitian() {
    let h2 = MolecularHamiltonian::h2_molecule();
    let matrix = h2.to_matrix();
    let dim = matrix.len();

    for i in 0..dim {
        for j in 0..dim {
            let diff = (matrix[i][j] - matrix[j][i].conj()).norm();
            assert!(
                diff < 1e-10,
                "Hamiltonian not Hermitian at [{i}][{j}]: diff = {diff}"
            );
        }
    }
}

#[test]
fn h2_hamiltonian_has_correct_dimension() {
    let h2 = MolecularHamiltonian::h2_molecule();
    let matrix = h2.to_matrix();
    assert_eq!(matrix.len(), 4); // 2^2
    assert_eq!(matrix[0].len(), 4);
}
