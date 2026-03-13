use serde::{Deserialize, Serialize};

use crate::circuit::CircuitBuilder;
use crate::error::QuantumError;
use crate::hamiltonian::MolecularHamiltonian;

/// Result of a VQE computation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VqeResult {
    pub ground_state_energy: f64,
    pub optimal_parameters: Vec<f64>,
    pub converged: bool,
    pub iterations: usize,
}

/// Configuration for a VQE run.
#[derive(Debug, Clone)]
pub struct VqeConfig {
    pub max_iterations: usize,
    pub convergence_threshold: f64,
    pub initial_step_size: f64,
}

impl Default for VqeConfig {
    fn default() -> Self {
        Self {
            max_iterations: 200,
            convergence_threshold: 1e-6,
            initial_step_size: 0.4,
        }
    }
}

/// Variational Quantum Eigensolver using Nelder-Mead optimization.
///
/// Minimizes ⟨ψ(θ)|H|ψ(θ)⟩ by varying the parameters θ of a
/// hardware-efficient ansatz circuit.
pub struct VqeRunner {
    config: VqeConfig,
}

impl VqeRunner {
    pub fn new(config: VqeConfig) -> Self {
        Self { config }
    }

    /// Run VQE for the given Hamiltonian.
    ///
    /// Uses a hardware-efficient ansatz: RY on each qubit + CNOT ladder.
    /// Number of parameters = num_qubits.
    pub fn run(&self, hamiltonian: &MolecularHamiltonian) -> Result<VqeResult, QuantumError> {
        let n = hamiltonian.num_qubits;
        let h_matrix = hamiltonian.to_matrix();
        let num_params = n;

        // Nelder-Mead simplex with num_params + 1 vertices
        let simplex_size = num_params + 1;
        let mut simplex: Vec<Vec<f64>> = Vec::with_capacity(simplex_size);

        // Initial simplex: zero vector + unit perturbations
        simplex.push(vec![0.0; num_params]);
        for i in 0..num_params {
            let mut vertex = vec![0.0; num_params];
            vertex[i] = self.config.initial_step_size;
            simplex.push(vertex);
        }

        let mut values: Vec<f64> = simplex
            .iter()
            .map(|params| self.evaluate_energy(n, params, &h_matrix))
            .collect::<Result<Vec<_>, _>>()?;

        let mut iterations = 0;
        let mut converged = false;

        for _ in 0..self.config.max_iterations {
            iterations += 1;

            // Sort simplex by energy value
            let mut indices: Vec<usize> = (0..simplex_size).collect();
            indices.sort_by(|&a, &b| values[a].partial_cmp(&values[b]).unwrap());

            let sorted_simplex: Vec<Vec<f64>> = indices.iter().map(|&i| simplex[i].clone()).collect();
            let sorted_values: Vec<f64> = indices.iter().map(|&i| values[i]).collect();
            simplex = sorted_simplex;
            values = sorted_values;

            // Convergence check: range of values in simplex
            let range = values[simplex_size - 1] - values[0];
            if range.abs() < self.config.convergence_threshold {
                converged = true;
                break;
            }

            // Centroid of all points except worst
            let centroid = self.centroid(&simplex[..simplex_size - 1]);

            // Reflection
            let worst = &simplex[simplex_size - 1];
            let reflected = self.reflect(&centroid, worst, 1.0);
            let reflected_val = self.evaluate_energy(n, &reflected, &h_matrix)?;

            if reflected_val < values[simplex_size - 2] && reflected_val >= values[0] {
                // Accept reflection
                simplex[simplex_size - 1] = reflected;
                values[simplex_size - 1] = reflected_val;
                continue;
            }

            if reflected_val < values[0] {
                // Expansion
                let expanded = self.reflect(&centroid, worst, 2.0);
                let expanded_val = self.evaluate_energy(n, &expanded, &h_matrix)?;
                if expanded_val < reflected_val {
                    simplex[simplex_size - 1] = expanded;
                    values[simplex_size - 1] = expanded_val;
                } else {
                    simplex[simplex_size - 1] = reflected;
                    values[simplex_size - 1] = reflected_val;
                }
                continue;
            }

            // Contraction
            let contracted = self.reflect(&centroid, worst, -0.5);
            let contracted_val = self.evaluate_energy(n, &contracted, &h_matrix)?;
            if contracted_val < values[simplex_size - 1] {
                simplex[simplex_size - 1] = contracted;
                values[simplex_size - 1] = contracted_val;
                continue;
            }

            // Shrink toward best
            let best = simplex[0].clone();
            for i in 1..simplex_size {
                for j in 0..num_params {
                    simplex[i][j] = best[j] + 0.5 * (simplex[i][j] - best[j]);
                }
                values[i] = self.evaluate_energy(n, &simplex[i], &h_matrix)?;
            }
        }

        // Find best
        let best_idx = values
            .iter()
            .enumerate()
            .min_by(|a, b| a.1.partial_cmp(b.1).unwrap())
            .map(|(i, _)| i)
            .unwrap_or(0);

        Ok(VqeResult {
            ground_state_energy: values[best_idx],
            optimal_parameters: simplex[best_idx].clone(),
            converged,
            iterations,
        })
    }

    fn evaluate_energy(
        &self,
        num_qubits: usize,
        params: &[f64],
        h_matrix: &[Vec<num_complex::Complex64>],
    ) -> Result<f64, QuantumError> {
        // Hardware-efficient ansatz: RY on each qubit, then CNOT ladder
        let mut builder = CircuitBuilder::new(num_qubits)?;
        for (q, &theta) in params.iter().enumerate().take(num_qubits) {
            builder = builder.ry(q, theta)?;
        }
        for q in 0..num_qubits.saturating_sub(1) {
            builder = builder.cnot(q, q + 1)?;
        }
        let circuit = builder.build();
        let sv = circuit.execute()?;
        Ok(sv.expectation_value(h_matrix))
    }

    fn centroid(&self, points: &[Vec<f64>]) -> Vec<f64> {
        let n = points[0].len();
        let count = points.len() as f64;
        let mut c = vec![0.0; n];
        for p in points {
            for (i, &v) in p.iter().enumerate() {
                c[i] += v / count;
            }
        }
        c
    }

    fn reflect(&self, centroid: &[f64], point: &[f64], alpha: f64) -> Vec<f64> {
        centroid
            .iter()
            .zip(point.iter())
            .map(|(&c, &p)| c + alpha * (c - p))
            .collect()
    }
}
