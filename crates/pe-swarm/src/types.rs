use serde::{Deserialize, Serialize};

use pe_core::{ProteinVariant, ScoredVariant};

/// The 6 specialized agent roles from ADR-007.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AgentRole {
    SequenceExplorer,
    FitnessScorer,
    StructuralValidator,
    ToxicityScreener,
    ExperimentDesigner,
    QuantumDispatcher,
}

/// Input task dispatched to a SwarmAgent.
#[derive(Debug, Clone)]
pub enum AgentTask {
    Explore {
        population: Vec<ProteinVariant>,
        mutation_rate: f64,
        crossover_rate: f64,
    },
    Score {
        candidates: Vec<ProteinVariant>,
    },
    Validate {
        scored: Vec<ScoredVariant>,
    },
    Screen {
        validated: Vec<ScoredVariant>,
    },
    DesignExperiment {
        top_candidates: Vec<ScoredVariant>,
    },
    QuantumDispatch {
        candidates: Vec<ScoredVariant>,
    },
}

/// Output result from a SwarmAgent.
#[derive(Debug, Clone)]
pub enum AgentResult {
    Explored { variants: Vec<ProteinVariant> },
    Scored { scored: Vec<ScoredVariant> },
    Validated { passed: Vec<ScoredVariant> },
    Screened { safe: Vec<ScoredVariant> },
    ExperimentDesigned { protocol_count: usize },
    QuantumDispatched { jobs_submitted: usize },
}

/// Configuration for a single design cycle.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CycleConfig {
    pub generation: u32,
    pub population_size: usize,
    pub mutation_rate: f64,
    pub crossover_rate: f64,
    pub quantum_enabled: bool,
    pub top_k: usize,
}

impl Default for CycleConfig {
    fn default() -> Self {
        Self {
            generation: 0,
            population_size: 50,
            mutation_rate: 0.1,
            crossover_rate: 0.3,
            quantum_enabled: false,
            top_k: 10,
        }
    }
}

/// Result of a completed design cycle.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CycleResult {
    pub promoted: Vec<ScoredVariant>,
    pub generation: u32,
    pub variants_created: usize,
    pub variants_scored: usize,
    pub variants_validated: usize,
    pub variants_screened: usize,
}
