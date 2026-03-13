//! Concrete LifecycleManager implementation backed by the daa governance framework.

use tracing::debug;

use pe_core::YamanakaFactor;

use crate::traits::LifecycleManager;
use crate::types::{
    AgentMetrics, BudgetAllocation, BudgetEntry, CycleConfig, CycleResult,
};

/// Configuration for retirement and allocation decisions.
#[derive(Debug, Clone)]
pub struct DaaConfig {
    /// An agent must complete at least this many cycles before it can be retired.
    pub min_cycles_before_retirement: u64,
    /// Quality score below this threshold makes an agent eligible for retirement.
    pub retirement_quality_threshold: f64,
    /// Base priority weight for exploration agents (SequenceExplorer).
    pub exploration_priority: f64,
}

impl Default for DaaConfig {
    fn default() -> Self {
        Self {
            min_cycles_before_retirement: 5,
            retirement_quality_threshold: 0.3,
            exploration_priority: 1.0,
        }
    }
}

/// Concrete LifecycleManager backed by the daa governance framework.
///
/// Implements:
/// - Retirement: retire if avg_quality_score < threshold AND cycles_completed > min_cycles
/// - Budget allocation: proportional to avg_quality_score * priority_weight, with a floor
/// - Priority adjustment: increase exploration budget for under-represented Yamanaka factors
pub struct DaaLifecycleManager {
    config: DaaConfig,
    /// Per-factor exploration boost multiplier. Increases for under-explored factors.
    factor_boosts: [f64; 4],
}

impl DaaLifecycleManager {
    pub fn new(config: DaaConfig) -> Self {
        Self {
            config,
            factor_boosts: [1.0; 4],
        }
    }

    /// Get the exploration boost for a given Yamanaka factor.
    pub fn factor_boost(&self, factor: &YamanakaFactor) -> f64 {
        let idx = Self::factor_index(factor);
        self.factor_boosts[idx]
    }

    fn factor_index(factor: &YamanakaFactor) -> usize {
        match factor {
            YamanakaFactor::OCT4 => 0,
            YamanakaFactor::SOX2 => 1,
            YamanakaFactor::KLF4 => 2,
            YamanakaFactor::CMYC => 3,
        }
    }
}

impl LifecycleManager for DaaLifecycleManager {
    fn should_retire(&self, agent: &AgentMetrics) -> bool {
        // Only retire agents that have completed enough cycles to be fairly evaluated
        if agent.cycles_completed < self.config.min_cycles_before_retirement {
            return false;
        }

        // Retire if quality is below threshold
        agent.avg_quality_score < self.config.retirement_quality_threshold
    }

    fn allocate_budget(
        &self,
        agents: &[AgentMetrics],
        cycle_config: &CycleConfig,
    ) -> BudgetAllocation {
        let mut allocation = BudgetAllocation::new();

        if agents.is_empty() {
            return allocation;
        }

        let n = agents.len() as u64;

        // Floor: every agent gets at least the minimum
        let floor_compute = cycle_config.min_compute_ms_per_agent;
        let floor_variants = cycle_config.min_variants_per_agent;

        // Compute remaining budget after floors
        let total_floor_compute = floor_compute.saturating_mul(n);
        let total_floor_variants = floor_variants.saturating_mul(n as u32);

        let remaining_compute = cycle_config
            .total_compute_ms
            .saturating_sub(total_floor_compute);
        let remaining_variants = cycle_config
            .total_variants
            .saturating_sub(total_floor_variants);

        // Compute weighted scores for proportional allocation.
        // Weight = max(avg_quality_score, 0.01) to avoid zero-weight agents.
        let weights: Vec<f64> = agents
            .iter()
            .map(|a| a.avg_quality_score.max(0.01))
            .collect();
        let total_weight: f64 = weights.iter().sum();

        for (i, agent) in agents.iter().enumerate() {
            let proportion = weights[i] / total_weight;
            let extra_compute = (remaining_compute as f64 * proportion) as u64;
            let extra_variants = (remaining_variants as f64 * proportion) as u32;

            allocation.allocations.insert(
                agent.agent_id,
                BudgetEntry {
                    max_compute_ms: floor_compute + extra_compute,
                    max_variants: floor_variants + extra_variants,
                    priority_weight: weights[i],
                },
            );
        }

        debug!(
            agent_count = agents.len(),
            total_compute = cycle_config.total_compute_ms,
            "allocated budget"
        );

        allocation
    }

    fn adjust_priorities(&mut self, cycle_result: &CycleResult) {
        let factors = [
            YamanakaFactor::OCT4,
            YamanakaFactor::SOX2,
            YamanakaFactor::KLF4,
            YamanakaFactor::CMYC,
        ];

        // Find the max coverage across all factors
        let coverages: Vec<u32> = factors
            .iter()
            .map(|f| cycle_result.coverage_for(f))
            .collect();
        let max_coverage = *coverages.iter().max().unwrap_or(&1);

        if max_coverage == 0 {
            return;
        }

        // Boost factors that are under-represented relative to the max.
        // Under-explored factors get a proportionally higher boost.
        for (i, &coverage) in coverages.iter().enumerate() {
            let ratio = coverage as f64 / max_coverage as f64;
            // Inverse ratio: under-explored → higher boost
            // Clamp to [1.0, 3.0] to avoid runaway boosts
            let boost = (1.0 / ratio.max(0.1)).min(3.0);
            self.factor_boosts[i] = boost;
        }

        debug!(
            boosts = ?self.factor_boosts,
            "adjusted factor priorities"
        );
    }
}
