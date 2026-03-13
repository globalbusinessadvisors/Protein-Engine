use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use pe_quantum_wasm::{
    MolecularHamiltonian, ProviderName, QaoaResult, QuboInstance, VqeResult,
};

use crate::error::QuantumRouterError;

/// The type of quantum computation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum QuantumJobType {
    Vqe,
    Qaoa,
}

/// Current status of a quantum job.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum JobStatus {
    Created,
    Submitted,
    Running,
    Completed,
    Failed,
}

impl std::fmt::Display for JobStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            JobStatus::Created => write!(f, "Created"),
            JobStatus::Submitted => write!(f, "Submitted"),
            JobStatus::Running => write!(f, "Running"),
            JobStatus::Completed => write!(f, "Completed"),
            JobStatus::Failed => write!(f, "Failed"),
        }
    }
}

/// The input to a quantum job.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum QuantumJobInput {
    Hamiltonian(MolecularHamiltonian),
    Qubo(QuboInstance),
}

/// The result of a quantum job.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum QuantumJobResult {
    Vqe(VqeResult),
    Qaoa(QaoaResult),
}

/// A quantum computation job with state-machine lifecycle.
///
/// Invariants (DDD-003 Aggregate 5):
/// - QJ-1: Status transitions: Created → Submitted → Running → Completed|Failed
/// - QJ-2: `backend` is set when status moves to Submitted
/// - QJ-3: `result` is set when status moves to Completed
/// - QJ-4: `result` is None when status is Failed
/// - QJ-5: Cannot transition backward
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuantumJob {
    id: Uuid,
    job_type: QuantumJobType,
    status: JobStatus,
    backend: Option<ProviderName>,
    submitted_at: Option<DateTime<Utc>>,
    completed_at: Option<DateTime<Utc>>,
    input: QuantumJobInput,
    result: Option<QuantumJobResult>,
}

impl QuantumJob {
    /// Create a new VQE job.
    pub fn new_vqe(hamiltonian: MolecularHamiltonian) -> Self {
        Self {
            id: Uuid::new_v4(),
            job_type: QuantumJobType::Vqe,
            status: JobStatus::Created,
            backend: None,
            submitted_at: None,
            completed_at: None,
            input: QuantumJobInput::Hamiltonian(hamiltonian),
            result: None,
        }
    }

    /// Create a new QAOA job.
    pub fn new_qaoa(qubo: QuboInstance) -> Self {
        Self {
            id: Uuid::new_v4(),
            job_type: QuantumJobType::Qaoa,
            status: JobStatus::Created,
            backend: None,
            submitted_at: None,
            completed_at: None,
            input: QuantumJobInput::Qubo(qubo),
            result: None,
        }
    }

    // ── State machine transitions ────────────────────────────────────

    /// Transition: Created → Submitted (QJ-1, QJ-2)
    pub fn submit(&mut self, backend: ProviderName) -> Result<(), QuantumRouterError> {
        if self.status != JobStatus::Created {
            return Err(QuantumRouterError::InvalidTransition {
                from: self.status.to_string(),
                to: "Submitted".into(),
            });
        }
        self.status = JobStatus::Submitted;
        self.backend = Some(backend);
        self.submitted_at = Some(Utc::now());
        Ok(())
    }

    /// Transition: Submitted → Running (QJ-1)
    pub fn start(&mut self) -> Result<(), QuantumRouterError> {
        if self.status != JobStatus::Submitted {
            return Err(QuantumRouterError::InvalidTransition {
                from: self.status.to_string(),
                to: "Running".into(),
            });
        }
        self.status = JobStatus::Running;
        Ok(())
    }

    /// Transition: Running → Completed (QJ-1, QJ-3)
    pub fn complete(&mut self, result: QuantumJobResult) -> Result<(), QuantumRouterError> {
        if self.status != JobStatus::Running {
            return Err(QuantumRouterError::InvalidTransition {
                from: self.status.to_string(),
                to: "Completed".into(),
            });
        }
        self.status = JobStatus::Completed;
        self.result = Some(result);
        self.completed_at = Some(Utc::now());
        Ok(())
    }

    /// Transition: Running → Failed (QJ-1, QJ-4)
    pub fn fail(&mut self) -> Result<(), QuantumRouterError> {
        if self.status != JobStatus::Running {
            return Err(QuantumRouterError::InvalidTransition {
                from: self.status.to_string(),
                to: "Failed".into(),
            });
        }
        self.status = JobStatus::Failed;
        self.result = None;
        self.completed_at = Some(Utc::now());
        Ok(())
    }

    // ── Getters ──────────────────────────────────────────────────────

    pub fn id(&self) -> Uuid {
        self.id
    }

    pub fn job_type(&self) -> &QuantumJobType {
        &self.job_type
    }

    pub fn status(&self) -> JobStatus {
        self.status
    }

    pub fn backend(&self) -> Option<&ProviderName> {
        self.backend.as_ref()
    }

    pub fn submitted_at(&self) -> Option<DateTime<Utc>> {
        self.submitted_at
    }

    pub fn completed_at(&self) -> Option<DateTime<Utc>> {
        self.completed_at
    }

    pub fn input(&self) -> &QuantumJobInput {
        &self.input
    }

    pub fn result(&self) -> Option<&QuantumJobResult> {
        self.result.as_ref()
    }
}
