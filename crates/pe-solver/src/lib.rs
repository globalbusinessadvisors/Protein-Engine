//! pe-solver: Sublinear-time sparse energy minimization solver.
//!
//! Provides energy landscape minimization for protein conformational
//! space exploration with both native (rayon-parallel) and WASM targets.

pub mod error;
pub mod gradient;
pub mod landscape;
pub mod result;
pub mod segment;
pub mod sublinear;
pub mod traits;

pub use error::SolverError;
pub use gradient::SimpleGradientSolver;
pub use landscape::EnergyLandscape;
pub use result::MinimizationResult;
pub use segment::SolverSegmentProducer;
pub use sublinear::SublinearSolver;
pub use traits::EnergySolver;

#[cfg(test)]
mod tests;
