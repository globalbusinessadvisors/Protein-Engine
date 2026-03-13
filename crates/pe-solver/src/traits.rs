use crate::error::SolverError;
use crate::landscape::EnergyLandscape;
use crate::result::MinimizationResult;

/// Minimizes energy over a sparse protein conformational landscape.
#[cfg_attr(test, mockall::automock)]
pub trait EnergySolver: Send + Sync {
    fn minimize(&self, energy_landscape: &EnergyLandscape)
        -> Result<MinimizationResult, SolverError>;
}
