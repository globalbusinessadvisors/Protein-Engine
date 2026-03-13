use async_trait::async_trait;

use pe_core::{ProteinVariant, ScoredVariant};

use crate::error::SwarmError;
use crate::types::{AgentResult, AgentRole, AgentTask, CycleConfig, CycleResult};

/// A single agent in the SAFLA swarm.
#[async_trait]
#[cfg_attr(test, mockall::automock)]
pub trait SwarmAgent: Send + Sync {
    async fn execute(&self, task: AgentTask) -> Result<AgentResult, SwarmError>;
    fn role(&self) -> AgentRole;
}

/// Evolutionary operations on protein variants.
#[cfg_attr(test, mockall::automock)]
pub trait EvolutionEngine: Send + Sync {
    fn mutate(&self, variant: &ProteinVariant) -> Result<ProteinVariant, SwarmError>;
    fn crossover(
        &self,
        a: &ProteinVariant,
        b: &ProteinVariant,
    ) -> Result<ProteinVariant, SwarmError>;
    fn select(&self, population: &[ScoredVariant], top_k: usize) -> Vec<ScoredVariant>;
}

/// Orchestrates the full SAFLA design cycle.
#[async_trait]
pub trait SwarmCoordinator: Send + Sync {
    async fn run_design_cycle(
        &mut self,
        config: CycleConfig,
    ) -> Result<CycleResult, SwarmError>;
}
