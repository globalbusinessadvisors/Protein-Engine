use serde::{Deserialize, Serialize};

use crate::error::SolverError;

/// Output of an energy minimization run.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MinimizationResult {
    pub minimum_energy: f64,
    pub optimal_coordinates: Vec<f64>,
    pub iterations: usize,
    pub converged: bool,
}

impl MinimizationResult {
    pub fn to_bytes(&self) -> Result<Vec<u8>, SolverError> {
        serde_json::to_vec(self)
            .map_err(|e| SolverError::SerializationFailed(e.to_string()))
    }

    pub fn from_bytes(data: &[u8]) -> Result<Self, SolverError> {
        serde_json::from_slice(data)
            .map_err(|e| SolverError::DeserializationFailed(e.to_string()))
    }
}
