//! pe-swarm: Multi-agent orchestration with SAFLA closed-loop design.
//!
//! Coordinates six specialized agents (SequenceExplorer, FitnessScorer,
//! StructuralValidator, ToxicityScreener, ExperimentDesigner, QuantumDispatcher)
//! through the design-score-validate-screen-measure-learn-redesign cycle.

pub mod coordinator;
pub mod error;
pub mod evolution;
pub mod segment;
pub mod traits;
pub mod types;

pub use coordinator::DefaultCoordinator;
pub use error::SwarmError;
pub use evolution::SimpleEvolutionEngine;
pub use segment::HotSegProducer;
pub use traits::{EvolutionEngine, SwarmAgent, SwarmCoordinator};
pub use types::{AgentResult, AgentRole, AgentTask, CycleConfig, CycleResult};

#[cfg(test)]
mod tests;
