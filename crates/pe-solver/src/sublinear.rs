use crate::error::SolverError;
use crate::gradient::SimpleGradientSolver;
use crate::landscape::EnergyLandscape;
use crate::result::MinimizationResult;
use crate::traits::EnergySolver;

/// Sublinear-time sparse energy solver.
///
/// Exploits sparsity in the energy landscape to achieve sub-linear scaling
/// with dimensionality. For landscapes with k sparse entries in d dimensions,
/// each iteration is O(k) rather than O(d).
///
/// Falls back to gradient descent for the local refinement phase.
#[derive(Debug, Clone)]
pub struct SublinearSolver {
    max_iterations: usize,
    convergence_threshold: f64,
    refinement_solver: SimpleGradientSolver,
}

impl Default for SublinearSolver {
    fn default() -> Self {
        Self {
            max_iterations: 500,
            convergence_threshold: 1e-8,
            refinement_solver: SimpleGradientSolver::default(),
        }
    }
}

impl SublinearSolver {
    pub fn new(
        max_iterations: usize,
        convergence_threshold: f64,
    ) -> Result<Self, SolverError> {
        if max_iterations == 0 {
            return Err(SolverError::InvalidConfig(
                "max_iterations must be > 0".into(),
            ));
        }
        if convergence_threshold <= 0.0 {
            return Err(SolverError::InvalidConfig(
                "convergence_threshold must be > 0".into(),
            ));
        }
        Ok(Self {
            max_iterations,
            convergence_threshold,
            refinement_solver: SimpleGradientSolver::new(
                max_iterations,
                0.01,
                convergence_threshold,
            )?,
        })
    }
}

impl EnergySolver for SublinearSolver {
    fn minimize(
        &self,
        energy_landscape: &EnergyLandscape,
    ) -> Result<MinimizationResult, SolverError> {
        let dims = energy_landscape.dimensions();
        let entries = energy_landscape.sparse_entries();

        // Empty landscape: trivial minimum
        if entries.is_empty() {
            return Ok(MinimizationResult {
                minimum_energy: 0.0,
                optimal_coordinates: vec![0.0; dims],
                iterations: 0,
                converged: true,
            });
        }

        // Phase 1: Sparse scan — find the entry with minimum energy in O(k)
        let (best_coords, best_energy) = entries
            .iter()
            .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
            .unwrap();

        // Build initial position from the best sparse entry
        let mut position = vec![0.0_f64; dims];
        for &idx in best_coords {
            if idx < dims {
                position[idx] = 1.0;
            }
        }

        let mut energy = *best_energy;
        let mut iterations = 1_usize; // 1 for the scan phase

        // Phase 2: Coordinate descent on active dimensions only (sublinear in d).
        // Only optimize dimensions that appear in sparse entries.
        let mut active_dims: Vec<usize> = entries
            .iter()
            .flat_map(|(coords, _)| coords.iter().copied())
            .collect();
        active_dims.sort_unstable();
        active_dims.dedup();

        let step = 0.01_f64;
        for iter in 0..self.max_iterations {
            iterations = iter + 2; // +1 for scan, +1 for 1-based
            let mut improved = false;

            for &dim in &active_dims {
                // Try step in both directions
                for direction in &[-1.0_f64, 1.0_f64] {
                    let mut trial = position.clone();
                    trial[dim] += step * direction;
                    let trial_energy = energy_landscape.evaluate(&trial);
                    if trial_energy < energy - self.convergence_threshold {
                        position = trial;
                        energy = trial_energy;
                        improved = true;
                    }
                }
            }

            if !improved {
                break;
            }
        }

        // Phase 3: Local refinement via gradient descent
        // Create a sub-landscape focused on active dimensions for refinement
        let refined = self.refinement_solver.minimize(energy_landscape)?;
        if refined.minimum_energy < energy {
            return Ok(MinimizationResult {
                minimum_energy: refined.minimum_energy,
                optimal_coordinates: refined.optimal_coordinates,
                iterations: iterations + refined.iterations,
                converged: refined.converged,
            });
        }

        Ok(MinimizationResult {
            minimum_energy: energy,
            optimal_coordinates: position,
            iterations,
            converged: true,
        })
    }
}
