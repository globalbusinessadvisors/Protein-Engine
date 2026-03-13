use serde::{Deserialize, Serialize};

use crate::error::SolverError;

/// A sparse energy surface over protein conformational space.
///
/// Energy values are defined at specific coordinate indices within an
/// N-dimensional space. Undefined coordinates are treated as having
/// zero energy (vacuum baseline).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnergyLandscape {
    dimensions: usize,
    sparse_entries: Vec<(Vec<usize>, f64)>,
}

impl EnergyLandscape {
    /// Create a new energy landscape.
    ///
    /// # Errors
    /// - `SolverError::ZeroDimensions` if `dimensions` is 0
    /// - `SolverError::CoordinateOutOfBounds` if any coordinate index >= dimensions
    pub fn new(
        dimensions: usize,
        sparse_entries: Vec<(Vec<usize>, f64)>,
    ) -> Result<Self, SolverError> {
        if dimensions == 0 {
            return Err(SolverError::ZeroDimensions);
        }

        for (coords, _) in &sparse_entries {
            for &idx in coords {
                if idx >= dimensions {
                    return Err(SolverError::CoordinateOutOfBounds {
                        index: idx,
                        dimensions,
                    });
                }
            }
        }

        Ok(Self {
            dimensions,
            sparse_entries,
        })
    }

    pub fn dimensions(&self) -> usize {
        self.dimensions
    }

    pub fn sparse_entries(&self) -> &[(Vec<usize>, f64)] {
        &self.sparse_entries
    }

    pub fn is_empty(&self) -> bool {
        self.sparse_entries.is_empty()
    }

    /// Evaluate the energy at a given point in the landscape.
    ///
    /// Uses inverse-distance weighted interpolation from sparse entries.
    /// Returns 0.0 when there are no entries.
    pub fn evaluate(&self, point: &[f64]) -> f64 {
        if self.sparse_entries.is_empty() {
            return 0.0;
        }

        let mut total_weight = 0.0_f64;
        let mut weighted_energy = 0.0_f64;

        for (coords, energy) in &self.sparse_entries {
            let dist_sq: f64 = coords
                .iter()
                .map(|&idx| {
                    let p = if idx < point.len() { point[idx] } else { 0.0 };
                    (p - 1.0) * (p - 1.0) // distance from coord being "active" (1.0)
                })
                .sum();

            if dist_sq < 1e-12 {
                return *energy;
            }
            let w = 1.0 / dist_sq;
            total_weight += w;
            weighted_energy += w * energy;
        }

        if total_weight < 1e-12 {
            0.0
        } else {
            weighted_energy / total_weight
        }
    }
}
