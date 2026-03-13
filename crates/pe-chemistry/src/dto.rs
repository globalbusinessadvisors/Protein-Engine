//! Internal request/response DTOs for the pyChemiQ sidecar HTTP API.
//!
//! These types mirror the sidecar's Python-centric JSON schema and are
//! NOT part of the public API. Domain types never leak sidecar-specific fields.

use serde::{Deserialize, Serialize};

// ── VQE ──────────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub(crate) struct ChemiqVqeRequest {
    pub molecule: String,
    pub basis_set: String,
    pub ansatz: String,
    pub max_iterations: usize,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ChemiqVqeResponse {
    pub energy: f64,
    pub parameters: Vec<f64>,
    pub iterations: usize,
    pub converged: bool,
}

// ── QAOA ─────────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub(crate) struct ChemiqQaoaRequest {
    pub qubo_matrix: Vec<Vec<f64>>,
    pub p_layers: usize,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ChemiqQaoaResponse {
    pub solution: Vec<usize>,
    pub cost: f64,
    pub iterations: usize,
}

// ── Health ────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub(crate) struct ChemiqHealthResponse {
    pub status: String,
    pub backend: String,
}

// ── Capabilities ─────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub(crate) struct ChemiqCapabilitiesResponse {
    pub max_qubits: u32,
    pub available_ansatze: Vec<String>,
    pub backend: String,
}
