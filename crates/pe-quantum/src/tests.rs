use std::collections::HashSet;

use pe_quantum_wasm::{
    BackendCapabilities, GateType, MolecularHamiltonian, ProviderName, QaoaResult, QuboInstance,
    VqeResult,
};
use pe_rvf::SegmentProducer;

use crate::error::QuantumRouterError;
use crate::job::{JobStatus, QuantumJob, QuantumJobResult};
use crate::local_backend::LocalSimulatorBackend;
use crate::router::QuantumRouter;
use crate::segment::{SketchSegProducer, VqeSnapshotCache};
use crate::traits::{MockQuantumBackend, QuantumBackend};

// ────────────────────────────────────────────────────────────────────
// Mock helpers
// ────────────────────────────────────────────────────────────────────

fn all_gates() -> HashSet<GateType> {
    GateType::all()
}

fn mock_backend(
    provider: ProviderName,
    max_qubits: u32,
    reachable: bool,
) -> MockQuantumBackend {
    let caps = BackendCapabilities {
        max_qubits,
        gate_set: all_gates(),
        is_simulator: provider == ProviderName::LocalSimulator,
        provider: provider.clone(),
    };

    let mut mock = MockQuantumBackend::new();
    mock.expect_capabilities().return_const(caps);
    mock.expect_is_reachable()
        .returning(move || Box::pin(async move { reachable }));

    mock.expect_submit_vqe().returning(move |_| {
        Box::pin(async move {
            Ok(VqeResult {
                ground_state_energy: -1.0,
                optimal_parameters: vec![0.5],
                converged: true,
                iterations: 10,
            })
        })
    });
    mock.expect_submit_qaoa().returning(move |_| {
        Box::pin(async move {
            Ok(QaoaResult {
                best_bitstring: 0b01,
                best_cost: -1.0,
                converged: true,
                iterations: 5,
            })
        })
    });

    mock
}

fn mock_unreachable_backend(
    provider: ProviderName,
    max_qubits: u32,
) -> MockQuantumBackend {
    let caps = BackendCapabilities {
        max_qubits,
        gate_set: all_gates(),
        is_simulator: false,
        provider,
    };

    let mut mock = MockQuantumBackend::new();
    mock.expect_capabilities().return_const(caps);
    mock.expect_is_reachable()
        .returning(|| Box::pin(async { false }));
    mock
}

// ────────────────────────────────────────────────────────────────────
// Router: backend selection
// ────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn router_selects_best_backend_by_qubit_count() {
    // Origin=72, IBM=127, Local=20; job=50 qubits → Origin (closest)
    let origin = mock_backend(ProviderName::OriginQuantum, 72, true);
    let ibm = mock_backend(ProviderName::Ibm, 127, true);
    let local = mock_backend(ProviderName::LocalSimulator, 20, true);

    let router = QuantumRouter::new(vec![
        Box::new(origin),
        Box::new(ibm),
        Box::new(local),
    ]);

    let h2 = MolecularHamiltonian::h2_molecule();
    let result = router
        .submit_vqe(&h2, 50, &all_gates())
        .await
        .unwrap();

    // Origin (72 qubits) is the closest match for 50 qubits
    assert!(result.converged);
}

#[tokio::test]
async fn router_selects_smallest_sufficient_backend() {
    // 20 qubits needed: local (20) should be preferred over origin (72)
    let origin = mock_backend(ProviderName::OriginQuantum, 72, true);
    let local = mock_backend(ProviderName::LocalSimulator, 20, true);

    let router = QuantumRouter::new(vec![
        Box::new(origin),
        Box::new(local),
    ]);

    let h2 = MolecularHamiltonian::h2_molecule();
    let result = router
        .submit_vqe(&h2, 20, &all_gates())
        .await
        .unwrap();

    assert!(result.converged);
}

#[tokio::test]
async fn router_falls_back_to_local_when_remotes_unreachable() {
    let origin = mock_unreachable_backend(ProviderName::OriginQuantum, 72);
    let ibm = mock_unreachable_backend(ProviderName::Ibm, 127);
    let local = mock_backend(ProviderName::LocalSimulator, 20, true);

    let router = QuantumRouter::new(vec![
        Box::new(origin),
        Box::new(ibm),
        Box::new(local),
    ]);

    let h2 = MolecularHamiltonian::h2_molecule();
    let result = router
        .submit_vqe(&h2, 10, &all_gates())
        .await
        .unwrap();

    assert!(result.converged);
}

#[tokio::test]
async fn router_returns_no_suitable_backend_when_all_too_small() {
    let local = mock_backend(ProviderName::LocalSimulator, 20, true);

    let router = QuantumRouter::new(vec![Box::new(local)]);

    let h2 = MolecularHamiltonian::h2_molecule();
    let result = router.submit_vqe(&h2, 50, &all_gates()).await;

    assert!(matches!(
        result,
        Err(QuantumRouterError::NoSuitableBackend { .. })
    ));
}

#[tokio::test]
async fn router_returns_no_suitable_backend_when_all_unreachable_and_too_small() {
    let origin = mock_unreachable_backend(ProviderName::OriginQuantum, 72);
    let local = mock_backend(ProviderName::LocalSimulator, 20, true);

    let router = QuantumRouter::new(vec![
        Box::new(origin),
        Box::new(local),
    ]);

    // Job needs 50 qubits, origin unreachable, local too small
    let h2 = MolecularHamiltonian::h2_molecule();
    let result = router.submit_vqe(&h2, 50, &all_gates()).await;

    assert!(matches!(
        result,
        Err(QuantumRouterError::NoSuitableBackend { .. })
    ));
}

#[tokio::test]
async fn router_filters_by_gate_set() {
    // Backend only supports H and X — job requires CNOT
    let caps = BackendCapabilities {
        max_qubits: 100,
        gate_set: [GateType::H, GateType::X].into_iter().collect(),
        is_simulator: false,
        provider: ProviderName::OriginQuantum,
    };
    let mut limited = MockQuantumBackend::new();
    limited.expect_capabilities().return_const(caps);
    limited
        .expect_is_reachable()
        .returning(|| Box::pin(async { true }));

    let local = mock_backend(ProviderName::LocalSimulator, 20, true);

    let router = QuantumRouter::new(vec![
        Box::new(limited),
        Box::new(local),
    ]);

    // Require CNOT — limited backend doesn't have it, local does
    let required = [GateType::H, GateType::Cnot].into_iter().collect();
    let h2 = MolecularHamiltonian::h2_molecule();
    let result = router.submit_vqe(&h2, 2, &required).await.unwrap();

    assert!(result.converged);
}

#[tokio::test]
async fn router_empty_backends_returns_error() {
    let router = QuantumRouter::new(vec![]);

    let h2 = MolecularHamiltonian::h2_molecule();
    let result = router.submit_vqe(&h2, 2, &all_gates()).await;

    assert!(matches!(
        result,
        Err(QuantumRouterError::NoSuitableBackend { .. })
    ));
}

// ────────────────────────────────────────────────────────────────────
// LocalSimulatorBackend
// ────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn local_simulator_runs_vqe() {
    let backend = LocalSimulatorBackend::new();
    let h2 = MolecularHamiltonian::h2_molecule();
    let result = backend.submit_vqe(h2).await.unwrap();

    assert!(result.converged);
    assert!(result.ground_state_energy < 0.0);
}

#[tokio::test]
async fn local_simulator_runs_qaoa() {
    let backend = LocalSimulatorBackend::new();
    let qubo = QuboInstance::new(vec![vec![-1.0, 0.0], vec![0.0, 2.0]]).unwrap();
    let result = backend.submit_qaoa(qubo).await.unwrap();

    assert!(result.best_cost <= 0.0);
}

#[tokio::test]
async fn local_simulator_is_always_reachable() {
    let backend = LocalSimulatorBackend::new();
    assert!(backend.is_reachable().await);
}

#[tokio::test]
async fn local_simulator_capabilities_correct() {
    let backend = LocalSimulatorBackend::new();
    let caps = backend.capabilities();
    assert_eq!(caps.max_qubits, 20);
    assert!(caps.is_simulator);
    assert_eq!(caps.provider, ProviderName::LocalSimulator);
    assert_eq!(caps.gate_set.len(), 9);
}

// ────────────────────────────────────────────────────────────────────
// QuantumJob state machine
// ────────────────────────────────────────────────────────────────────

#[test]
fn job_transitions_through_full_lifecycle() {
    let h2 = MolecularHamiltonian::h2_molecule();
    let mut job = QuantumJob::new_vqe(h2);

    assert_eq!(job.status(), JobStatus::Created);
    assert!(job.backend().is_none());
    assert!(job.result().is_none());

    // Created → Submitted (QJ-2: backend set)
    job.submit(ProviderName::LocalSimulator).unwrap();
    assert_eq!(job.status(), JobStatus::Submitted);
    assert_eq!(job.backend(), Some(&ProviderName::LocalSimulator));
    assert!(job.submitted_at().is_some());

    // Submitted → Running
    job.start().unwrap();
    assert_eq!(job.status(), JobStatus::Running);

    // Running → Completed (QJ-3: result set)
    let result = QuantumJobResult::Vqe(VqeResult {
        ground_state_energy: -1.137,
        optimal_parameters: vec![0.5, 0.3],
        converged: true,
        iterations: 50,
    });
    job.complete(result).unwrap();
    assert_eq!(job.status(), JobStatus::Completed);
    assert!(job.result().is_some());
    assert!(job.completed_at().is_some());
}

#[test]
fn job_cannot_complete_without_submit() {
    let h2 = MolecularHamiltonian::h2_molecule();
    let mut job = QuantumJob::new_vqe(h2);

    let result = QuantumJobResult::Vqe(VqeResult {
        ground_state_energy: -1.0,
        optimal_parameters: vec![],
        converged: true,
        iterations: 0,
    });
    let err = job.complete(result);
    assert!(matches!(
        err,
        Err(QuantumRouterError::InvalidTransition { .. })
    ));
}

#[test]
fn job_cannot_submit_twice() {
    let h2 = MolecularHamiltonian::h2_molecule();
    let mut job = QuantumJob::new_vqe(h2);

    job.submit(ProviderName::LocalSimulator).unwrap();
    let err = job.submit(ProviderName::Ibm);
    assert!(matches!(
        err,
        Err(QuantumRouterError::InvalidTransition { .. })
    ));
}

#[test]
fn job_cannot_start_without_submit() {
    let h2 = MolecularHamiltonian::h2_molecule();
    let mut job = QuantumJob::new_vqe(h2);

    let err = job.start();
    assert!(matches!(
        err,
        Err(QuantumRouterError::InvalidTransition { .. })
    ));
}

#[test]
fn job_cannot_fail_without_running() {
    let h2 = MolecularHamiltonian::h2_molecule();
    let mut job = QuantumJob::new_vqe(h2);

    let err = job.fail();
    assert!(matches!(
        err,
        Err(QuantumRouterError::InvalidTransition { .. })
    ));
}

#[test]
fn failed_job_has_no_result() {
    let h2 = MolecularHamiltonian::h2_molecule();
    let mut job = QuantumJob::new_vqe(h2);

    job.submit(ProviderName::LocalSimulator).unwrap();
    job.start().unwrap();
    job.fail().unwrap();

    assert_eq!(job.status(), JobStatus::Failed);
    assert!(job.result().is_none()); // QJ-4
    assert!(job.completed_at().is_some());
}

#[test]
fn cannot_transition_backward_from_completed() {
    let h2 = MolecularHamiltonian::h2_molecule();
    let mut job = QuantumJob::new_vqe(h2);

    job.submit(ProviderName::LocalSimulator).unwrap();
    job.start().unwrap();
    job.complete(QuantumJobResult::Vqe(VqeResult {
        ground_state_energy: -1.0,
        optimal_parameters: vec![],
        converged: true,
        iterations: 0,
    }))
    .unwrap();

    // QJ-5: Cannot go backward
    assert!(job.start().is_err());
    assert!(job.submit(ProviderName::Ibm).is_err());
    assert!(job.fail().is_err());
}

#[test]
fn cannot_transition_backward_from_failed() {
    let h2 = MolecularHamiltonian::h2_molecule();
    let mut job = QuantumJob::new_vqe(h2);

    job.submit(ProviderName::LocalSimulator).unwrap();
    job.start().unwrap();
    job.fail().unwrap();

    assert!(job.start().is_err());
    assert!(job.submit(ProviderName::Ibm).is_err());
}

#[test]
fn qaoa_job_lifecycle() {
    let qubo = QuboInstance::new(vec![vec![-1.0, 0.0], vec![0.0, 2.0]]).unwrap();
    let mut job = QuantumJob::new_qaoa(qubo);

    job.submit(ProviderName::OriginQuantum).unwrap();
    job.start().unwrap();
    job.complete(QuantumJobResult::Qaoa(QaoaResult {
        best_bitstring: 0b01,
        best_cost: -1.0,
        converged: true,
        iterations: 5,
    }))
    .unwrap();

    assert_eq!(job.status(), JobStatus::Completed);
    assert!(matches!(job.result(), Some(QuantumJobResult::Qaoa(_))));
}

// ────────────────────────────────────────────────────────────────────
// VQE snapshot caching + SKETCH_SEG
// ────────────────────────────────────────────────────────────────────

#[test]
fn vqe_snapshots_round_trip_through_sketch_seg() {
    let mut cache = VqeSnapshotCache::new();
    cache.add(
        "H2-equilibrium".into(),
        VqeResult {
            ground_state_energy: -1.137,
            optimal_parameters: vec![0.5, 0.3],
            converged: true,
            iterations: 50,
        },
    );
    cache.add(
        "H2-stretched".into(),
        VqeResult {
            ground_state_energy: -0.8,
            optimal_parameters: vec![0.7, 0.1],
            converged: true,
            iterations: 80,
        },
    );

    let bytes = cache.to_bytes().unwrap();
    let restored = VqeSnapshotCache::from_bytes(&bytes).unwrap();

    assert_eq!(restored.snapshots.len(), 2);
    assert_eq!(restored.snapshots[0].label, "H2-equilibrium");
    assert_eq!(restored.snapshots[0], cache.snapshots[0]);
    assert_eq!(restored.snapshots[1], cache.snapshots[1]);
}

#[test]
fn sketch_seg_producer_returns_correct_type() {
    let cache = VqeSnapshotCache::new();
    let producer = SketchSegProducer::new(cache);

    assert_eq!(producer.segment_type(), pe_rvf::SegmentType::SketchSeg);
    let data = producer.produce().unwrap();
    assert!(!data.is_empty());
}

#[test]
fn sketch_seg_producer_output_deserializes_to_cache() {
    let mut cache = VqeSnapshotCache::new();
    cache.add(
        "test".into(),
        VqeResult {
            ground_state_energy: -2.0,
            optimal_parameters: vec![1.0],
            converged: false,
            iterations: 100,
        },
    );

    let producer = SketchSegProducer::new(cache);
    let data = producer.produce().unwrap();

    let restored = VqeSnapshotCache::from_bytes(&data).unwrap();
    assert_eq!(restored.snapshots.len(), 1);
    assert_eq!(restored.snapshots[0].label, "test");
}

#[test]
fn empty_cache_round_trip() {
    let cache = VqeSnapshotCache::new();
    let bytes = cache.to_bytes().unwrap();
    let restored = VqeSnapshotCache::from_bytes(&bytes).unwrap();
    assert!(restored.snapshots.is_empty());
}
