use serde::{Deserialize, Serialize};

use crate::error::QuantumError;
use crate::statevector::StateVector;

/// Symmetric matrix encoding a QUBO optimization problem.
///
/// Minimize x^T Q x where x ∈ {0, 1}^n.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuboInstance {
    pub num_variables: usize,
    pub matrix: Vec<Vec<f64>>,
}

impl QuboInstance {
    pub fn new(matrix: Vec<Vec<f64>>) -> Result<Self, QuantumError> {
        let rows = matrix.len();
        if rows == 0 {
            return Err(QuantumError::EmptyCircuit);
        }
        for (i, row) in matrix.iter().enumerate() {
            if row.len() != rows {
                return Err(QuantumError::QuboNotSquare {
                    rows,
                    cols: row.len(),
                });
            }
            // Verify symmetry is close enough
            for j in (i + 1)..rows {
                if (matrix[i][j] - matrix[j][i]).abs() > 1e-10 {
                    return Err(QuantumError::QuboNotSquare { rows, cols: rows });
                }
            }
        }
        Ok(Self {
            num_variables: rows,
            matrix,
        })
    }

    /// Evaluate the QUBO cost for a given bitstring (as usize).
    pub fn evaluate(&self, bitstring: usize) -> f64 {
        let n = self.num_variables;
        let mut cost = 0.0;
        for i in 0..n {
            if bitstring & (1 << i) == 0 {
                continue;
            }
            for j in 0..n {
                if bitstring & (1 << j) != 0 {
                    cost += self.matrix[i][j];
                }
            }
        }
        cost
    }

    /// Brute-force find the optimal bitstring (for small instances).
    pub fn optimal_brute_force(&self) -> (usize, f64) {
        let n = self.num_variables;
        let mut best_bits = 0usize;
        let mut best_cost = f64::INFINITY;

        for bits in 0..(1 << n) {
            let cost = self.evaluate(bits);
            if cost < best_cost {
                best_cost = cost;
                best_bits = bits;
            }
        }
        (best_bits, best_cost)
    }
}

/// Result of a QAOA computation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QaoaResult {
    pub best_bitstring: usize,
    pub best_cost: f64,
    pub converged: bool,
    pub iterations: usize,
}

/// Configuration for a QAOA run.
#[derive(Debug, Clone)]
pub struct QaoaConfig {
    pub num_layers: usize,
    pub max_iterations: usize,
    pub convergence_threshold: f64,
    pub initial_step_size: f64,
}

impl Default for QaoaConfig {
    fn default() -> Self {
        Self {
            num_layers: 2,
            max_iterations: 100,
            convergence_threshold: 1e-4,
            initial_step_size: 0.3,
        }
    }
}

/// QAOA solver with p layers and Nelder-Mead parameter optimization.
pub struct QaoaRunner {
    config: QaoaConfig,
}

impl QaoaRunner {
    pub fn new(config: QaoaConfig) -> Self {
        Self { config }
    }

    /// Run QAOA on the given QUBO instance.
    pub fn run(&self, qubo: &QuboInstance) -> Result<QaoaResult, QuantumError> {
        let n = qubo.num_variables;
        let p = self.config.num_layers;
        let num_params = 2 * p; // gamma_1..p, beta_1..p

        // Nelder-Mead on 2p parameters
        let simplex_size = num_params + 1;
        let mut simplex: Vec<Vec<f64>> = Vec::with_capacity(simplex_size);

        simplex.push(vec![0.3; num_params]);
        for i in 0..num_params {
            let mut vertex = vec![0.3; num_params];
            vertex[i] += self.config.initial_step_size;
            simplex.push(vertex);
        }

        let mut values: Vec<f64> = simplex
            .iter()
            .map(|params| self.evaluate_qaoa(n, qubo, params))
            .collect::<Result<Vec<_>, _>>()?;

        let mut iterations = 0;
        let mut converged = false;

        for _ in 0..self.config.max_iterations {
            iterations += 1;

            let mut indices: Vec<usize> = (0..simplex_size).collect();
            indices.sort_by(|&a, &b| values[a].partial_cmp(&values[b]).unwrap());

            let sorted_simplex: Vec<Vec<f64>> =
                indices.iter().map(|&i| simplex[i].clone()).collect();
            let sorted_values: Vec<f64> = indices.iter().map(|&i| values[i]).collect();
            simplex = sorted_simplex;
            values = sorted_values;

            let range = values[simplex_size - 1] - values[0];
            if range.abs() < self.config.convergence_threshold {
                converged = true;
                break;
            }

            let centroid = centroid_of(&simplex[..simplex_size - 1]);
            let worst = &simplex[simplex_size - 1];
            let reflected = reflect_point(&centroid, worst, 1.0);
            let reflected_val = self.evaluate_qaoa(n, qubo, &reflected)?;

            if reflected_val < values[simplex_size - 2] && reflected_val >= values[0] {
                simplex[simplex_size - 1] = reflected;
                values[simplex_size - 1] = reflected_val;
                continue;
            }

            if reflected_val < values[0] {
                let expanded = reflect_point(&centroid, worst, 2.0);
                let expanded_val = self.evaluate_qaoa(n, qubo, &expanded)?;
                if expanded_val < reflected_val {
                    simplex[simplex_size - 1] = expanded;
                    values[simplex_size - 1] = expanded_val;
                } else {
                    simplex[simplex_size - 1] = reflected;
                    values[simplex_size - 1] = reflected_val;
                }
                continue;
            }

            let contracted = reflect_point(&centroid, worst, -0.5);
            let contracted_val = self.evaluate_qaoa(n, qubo, &contracted)?;
            if contracted_val < values[simplex_size - 1] {
                simplex[simplex_size - 1] = contracted;
                values[simplex_size - 1] = contracted_val;
                continue;
            }

            let best = simplex[0].clone();
            for i in 1..simplex_size {
                for j in 0..num_params {
                    simplex[i][j] = best[j] + 0.5 * (simplex[i][j] - best[j]);
                }
                values[i] = self.evaluate_qaoa(n, qubo, &simplex[i])?;
            }
        }

        let best_idx = values
            .iter()
            .enumerate()
            .min_by(|a, b| a.1.partial_cmp(b.1).unwrap())
            .map(|(i, _)| i)
            .unwrap_or(0);

        // Sample the best circuit to get the actual bitstring
        let best_params = &simplex[best_idx];
        let sv = self.build_qaoa_state(n, qubo, best_params)?;

        // Pick the bitstring with highest probability
        let probs = sv.probabilities();
        let (best_bitstring, _) = probs
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
            .unwrap_or((0, &0.0));

        let best_cost = qubo.evaluate(best_bitstring);

        Ok(QaoaResult {
            best_bitstring,
            best_cost,
            converged,
            iterations,
        })
    }

    fn evaluate_qaoa(
        &self,
        n: usize,
        qubo: &QuboInstance,
        params: &[f64],
    ) -> Result<f64, QuantumError> {
        let sv = self.build_qaoa_state(n, qubo, params)?;
        let probs = sv.probabilities();

        // Expected cost = Σ_x P(x) * C(x)
        let mut expected_cost = 0.0;
        for (bits, &prob) in probs.iter().enumerate() {
            if prob > 1e-15 {
                expected_cost += prob * qubo.evaluate(bits);
            }
        }
        Ok(expected_cost)
    }

    fn build_qaoa_state(
        &self,
        n: usize,
        qubo: &QuboInstance,
        params: &[f64],
    ) -> Result<StateVector, QuantumError> {
        let p = self.config.num_layers;

        // Start with uniform superposition
        let mut sv = StateVector::new(n)?;
        for q in 0..n {
            sv.h(q)?;
        }

        // Apply p layers of (cost unitary, mixer unitary)
        for layer in 0..p {
            let gamma = params[layer];
            let beta = params[p + layer];

            // Cost unitary: exp(-i * gamma * C)
            // For QUBO: apply RZ(2*gamma*Q_ii) for diagonal terms
            // and RZZ(2*gamma*Q_ij) for off-diagonal (via CNOT-RZ-CNOT)
            self.apply_cost_unitary(&mut sv, n, qubo, gamma)?;

            // Mixer unitary: exp(-i * beta * B) where B = Σ X_i
            for q in 0..n {
                sv.rx(q, 2.0 * beta)?;
            }
        }

        Ok(sv)
    }

    fn apply_cost_unitary(
        &self,
        sv: &mut StateVector,
        n: usize,
        qubo: &QuboInstance,
        gamma: f64,
    ) -> Result<(), QuantumError> {
        // Diagonal terms
        for i in 0..n {
            let angle = 2.0 * gamma * qubo.matrix[i][i];
            if angle.abs() > 1e-15 {
                sv.rz(i, angle)?;
            }
        }

        // Off-diagonal terms (ZZ interaction via CNOT-RZ-CNOT)
        for i in 0..n {
            for j in (i + 1)..n {
                let coeff = qubo.matrix[i][j] + qubo.matrix[j][i];
                if coeff.abs() > 1e-15 {
                    let angle = gamma * coeff;
                    sv.cnot(i, j)?;
                    sv.rz(j, 2.0 * angle)?;
                    sv.cnot(i, j)?;
                }
            }
        }
        Ok(())
    }
}

fn centroid_of(points: &[Vec<f64>]) -> Vec<f64> {
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

fn reflect_point(centroid: &[f64], point: &[f64], alpha: f64) -> Vec<f64> {
    centroid
        .iter()
        .zip(point.iter())
        .map(|(&c, &p)| c + alpha * (c - p))
        .collect()
}
