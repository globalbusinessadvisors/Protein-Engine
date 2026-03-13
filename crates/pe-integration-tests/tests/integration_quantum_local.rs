//! Integration test: QuantumRouter + LocalSimulatorBackend.
//!
//! No remote backends — router falls back to the local simulator.

use std::collections::HashSet;

use pe_quantum::{LocalSimulatorBackend, QuantumBackend, QuantumRouter};
use pe_quantum_wasm::{GateType, MolecularHamiltonian, QuboInstance};

// ── Tests ────────────────────────────────────────────────────────────

#[tokio::test]
async fn vqe_h2_energy_near_expected() {
    let backend = LocalSimulatorBackend::new();
    let router = QuantumRouter::new(vec![Box::new(backend)]);

    let hamiltonian = MolecularHamiltonian::h2_molecule();
    let gates: HashSet<GateType> = GateType::all();

    let result = router
        .submit_vqe(&hamiltonian, 2, &gates)
        .await
        .expect("VQE should succeed");

    // H2 ground state ≈ -1.137 Ha
    assert!(
        result.ground_state_energy < 0.0,
        "energy must be negative, got {}",
        result.ground_state_energy
    );
    assert!(
        (result.ground_state_energy - (-1.137)).abs() < 1.0,
        "energy {} too far from expected -1.137 Ha",
        result.ground_state_energy
    );
    assert!(result.iterations > 0);
}

#[tokio::test]
async fn qaoa_trivial_qubo_optimal_solution() {
    let backend = LocalSimulatorBackend::new();
    let router = QuantumRouter::new(vec![Box::new(backend)]);

    // Diagonal QUBO: minimize x0 + x1 → optimal is (0,0), cost=0
    let qubo = QuboInstance::new(vec![vec![1.0, 0.0], vec![0.0, 1.0]]).expect("valid QUBO");
    let gates: HashSet<GateType> = GateType::all();

    let result = router
        .submit_qaoa(&qubo, 2, &gates)
        .await
        .expect("QAOA should succeed");

    assert_eq!(result.best_bitstring, 0b00, "optimal should be all zeros");
    assert!(
        result.best_cost.abs() < 1e-6,
        "optimal cost should be 0.0, got {}",
        result.best_cost
    );
}

#[tokio::test]
async fn qaoa_negative_diagonal_prefers_ones() {
    let backend = LocalSimulatorBackend::new();
    let router = QuantumRouter::new(vec![Box::new(backend)]);

    // QUBO [[-1,0],[0,-1]]: both variables want to be 1 (cost=-2)
    let qubo = QuboInstance::new(vec![vec![-1.0, 0.0], vec![0.0, -1.0]]).expect("valid QUBO");
    let gates: HashSet<GateType> = GateType::all();

    let result = router
        .submit_qaoa(&qubo, 2, &gates)
        .await
        .expect("QAOA should succeed");

    assert_eq!(result.best_bitstring, 0b11, "optimal should be all ones");
    assert!(
        (result.best_cost - (-2.0)).abs() < 1e-6,
        "optimal cost should be -2.0, got {}",
        result.best_cost
    );
}

#[tokio::test]
async fn router_with_only_local_backend_is_reachable() {
    let backend = LocalSimulatorBackend::new();
    assert!(backend.capabilities().is_simulator);

    let router = QuantumRouter::new(vec![Box::new(backend)]);
    // Router should successfully route even with only one backend
    let hamiltonian = MolecularHamiltonian::h2_molecule();
    let gates: HashSet<GateType> = GateType::all();
    let result = router.submit_vqe(&hamiltonian, 2, &gates).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn vqe_result_has_parameters() {
    let backend = LocalSimulatorBackend::new();
    let router = QuantumRouter::new(vec![Box::new(backend)]);

    let hamiltonian = MolecularHamiltonian::h2_molecule();
    let gates: HashSet<GateType> = GateType::all();

    let result = router
        .submit_vqe(&hamiltonian, 2, &gates)
        .await
        .expect("VQE");

    assert!(
        !result.optimal_parameters.is_empty(),
        "VQE should produce parameters"
    );
}
