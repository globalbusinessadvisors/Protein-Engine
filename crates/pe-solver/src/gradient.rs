use crate::error::SolverError;
use crate::landscape::EnergyLandscape;
use crate::result::MinimizationResult;
use crate::traits::EnergySolver;

/// Pure-Rust gradient descent solver. Works on all targets including WASM.
///
/// Uses numerical gradient estimation with central differences and
/// adaptive step size for convergence on smooth energy surfaces.
#[derive(Debug, Clone)]
pub struct SimpleGradientSolver {
    max_iterations: usize,
    learning_rate: f64,
    convergence_threshold: f64,
    gradient_step: f64,
}

impl Default for SimpleGradientSolver {
    fn default() -> Self {
        Self {
            max_iterations: 1000,
            learning_rate: 0.01,
            convergence_threshold: 1e-8,
            gradient_step: 1e-5,
        }
    }
}

impl SimpleGradientSolver {
    pub fn new(
        max_iterations: usize,
        learning_rate: f64,
        convergence_threshold: f64,
    ) -> Result<Self, SolverError> {
        if max_iterations == 0 {
            return Err(SolverError::InvalidConfig(
                "max_iterations must be > 0".into(),
            ));
        }
        if learning_rate <= 0.0 {
            return Err(SolverError::InvalidConfig(
                "learning_rate must be > 0".into(),
            ));
        }
        if convergence_threshold <= 0.0 {
            return Err(SolverError::InvalidConfig(
                "convergence_threshold must be > 0".into(),
            ));
        }
        Ok(Self {
            max_iterations,
            learning_rate,
            convergence_threshold,
            gradient_step: 1e-5,
        })
    }

    fn numerical_gradient(&self, landscape: &EnergyLandscape, point: &[f64]) -> Vec<f64> {
        let dims = point.len();
        let mut grad = vec![0.0; dims];

        for d in 0..dims {
            let mut p_plus = point.to_vec();
            let mut p_minus = point.to_vec();
            p_plus[d] += self.gradient_step;
            p_minus[d] -= self.gradient_step;

            let e_plus = landscape.evaluate(&p_plus);
            let e_minus = landscape.evaluate(&p_minus);
            grad[d] = (e_plus - e_minus) / (2.0 * self.gradient_step);
        }
        grad
    }
}

impl EnergySolver for SimpleGradientSolver {
    fn minimize(
        &self,
        energy_landscape: &EnergyLandscape,
    ) -> Result<MinimizationResult, SolverError> {
        let dims = energy_landscape.dimensions();

        // Empty landscape: minimum is at origin with zero energy
        if energy_landscape.is_empty() {
            return Ok(MinimizationResult {
                minimum_energy: 0.0,
                optimal_coordinates: vec![0.0; dims],
                iterations: 0,
                converged: true,
            });
        }

        // Initialize at the centroid of sparse entry coordinates
        let mut position = vec![0.0_f64; dims];
        let entry_count = energy_landscape.sparse_entries().len() as f64;
        for (coords, _) in energy_landscape.sparse_entries() {
            for &idx in coords {
                if idx < dims {
                    position[idx] += 1.0 / entry_count;
                }
            }
        }

        let mut energy = energy_landscape.evaluate(&position);
        let mut iterations = 0;
        let mut converged = false;

        for i in 0..self.max_iterations {
            iterations = i + 1;
            let grad = self.numerical_gradient(energy_landscape, &position);

            // Gradient norm for convergence check
            let grad_norm: f64 = grad.iter().map(|g| g * g).sum::<f64>().sqrt();
            if grad_norm < self.convergence_threshold {
                converged = true;
                break;
            }

            // Adaptive step: reduce learning rate as gradient shrinks
            let effective_lr = self.learning_rate / (1.0 + 0.001 * i as f64);

            // Update position
            for (p, g) in position.iter_mut().zip(grad.iter()) {
                *p -= effective_lr * g;
            }

            let new_energy = energy_landscape.evaluate(&position);

            // Check energy convergence
            if (new_energy - energy).abs() < self.convergence_threshold {
                energy = new_energy;
                converged = true;
                break;
            }
            energy = new_energy;
        }

        Ok(MinimizationResult {
            minimum_energy: energy,
            optimal_coordinates: position,
            iterations,
            converged,
        })
    }
}
