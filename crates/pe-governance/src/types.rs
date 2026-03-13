use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use uuid::Uuid;

use pe_core::YamanakaFactor;

/// The 6 specialized agent roles from ADR-007.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AgentRole {
    SequenceExplorer,
    FitnessScorerAgent,
    StructuralValidator,
    ToxicityScreener,
    ExperimentDesigner,
    QuantumDispatcher,
}

/// Performance stats for a swarm agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentMetrics {
    pub agent_id: Uuid,
    pub role: AgentRole,
    pub cycles_completed: u64,
    pub avg_quality_score: f64,
    pub avg_latency_ms: f64,
    pub error_count: u64,
}

impl AgentMetrics {
    pub fn new(agent_id: Uuid, role: AgentRole) -> Self {
        Self {
            agent_id,
            role,
            cycles_completed: 0,
            avg_quality_score: 0.0,
            avg_latency_ms: 0.0,
            error_count: 0,
        }
    }
}

/// A single agent's compute budget for one cycle.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BudgetEntry {
    pub max_compute_ms: u64,
    pub max_variants: u32,
    pub priority_weight: f64,
}

/// Compute budget assigned per agent per cycle.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BudgetAllocation {
    pub allocations: BTreeMap<Uuid, BudgetEntry>,
}

impl BudgetAllocation {
    pub fn new() -> Self {
        Self {
            allocations: BTreeMap::new(),
        }
    }

    pub fn get(&self, agent_id: &Uuid) -> Option<&BudgetEntry> {
        self.allocations.get(agent_id)
    }
}

impl Default for BudgetAllocation {
    fn default() -> Self {
        Self::new()
    }
}

/// Configuration for a design cycle.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CycleConfig {
    /// Total compute budget in milliseconds for the entire cycle.
    pub total_compute_ms: u64,
    /// Total variant slots available across all agents.
    pub total_variants: u32,
    /// Minimum compute budget per agent (floor).
    pub min_compute_ms_per_agent: u64,
    /// Minimum variant slots per agent (floor).
    pub min_variants_per_agent: u32,
}

impl Default for CycleConfig {
    fn default() -> Self {
        Self {
            total_compute_ms: 60_000,
            total_variants: 100,
            min_compute_ms_per_agent: 1_000,
            min_variants_per_agent: 2,
        }
    }
}

/// Result of a completed design cycle, used for priority adjustment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CycleResult {
    /// Per-factor coverage: how many variants targeted each Yamanaka factor.
    pub factor_coverage: BTreeMap<String, u32>,
    /// The best composite fitness achieved this cycle.
    pub best_fitness: f64,
    /// Number of variants promoted to HotSeg.
    pub promoted_count: u32,
}

impl CycleResult {
    /// Get coverage count for a specific Yamanaka factor.
    pub fn coverage_for(&self, factor: &YamanakaFactor) -> u32 {
        self.factor_coverage
            .get(&factor.to_string())
            .copied()
            .unwrap_or(0)
    }
}
