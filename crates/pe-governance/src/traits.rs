use crate::types::{AgentMetrics, BudgetAllocation, CycleConfig, CycleResult};

/// Manages agent lifecycle: retirement, budget allocation, priority adjustment.
///
/// Mockable for downstream crates that depend on governance decisions.
#[cfg_attr(test, mockall::automock)]
pub trait LifecycleManager: Send + Sync {
    /// Determine whether an agent should be retired based on its metrics.
    fn should_retire(&self, agent: &AgentMetrics) -> bool;

    /// Allocate compute budgets across all active agents for the next cycle.
    fn allocate_budget(
        &self,
        agents: &[AgentMetrics],
        cycle_config: &CycleConfig,
    ) -> BudgetAllocation;

    /// Adjust internal priorities based on the results of a completed cycle.
    fn adjust_priorities(&mut self, cycle_result: &CycleResult);
}
