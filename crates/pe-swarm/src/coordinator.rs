//! DefaultCoordinator — orchestrates the SAFLA closed loop (ADR-007).

use async_trait::async_trait;
use tracing::{debug, info};

use pe_ledger::EntryType;

use crate::error::SwarmError;
use crate::traits::{SwarmAgent, SwarmCoordinator};
use crate::types::{AgentResult, AgentTask, CycleConfig, CycleResult};

/// Orchestrates the SAFLA loop: DESIGN → SCORE → VALIDATE → SCREEN → (QUANTUM) → LOG → PROMOTE.
pub struct DefaultCoordinator {
    explorer: Box<dyn SwarmAgent>,
    scorer: Box<dyn SwarmAgent>,
    validator: Box<dyn SwarmAgent>,
    screener: Box<dyn SwarmAgent>,
    quantum: Option<Box<dyn SwarmAgent>>,
    ledger: Box<dyn pe_ledger::LedgerWriter>,
}

impl DefaultCoordinator {
    pub fn new(
        explorer: Box<dyn SwarmAgent>,
        scorer: Box<dyn SwarmAgent>,
        validator: Box<dyn SwarmAgent>,
        screener: Box<dyn SwarmAgent>,
        quantum: Option<Box<dyn SwarmAgent>>,
        ledger: Box<dyn pe_ledger::LedgerWriter>,
    ) -> Self {
        Self {
            explorer,
            scorer,
            validator,
            screener,
            quantum,
            ledger,
        }
    }
}

#[async_trait]
impl SwarmCoordinator for DefaultCoordinator {
    async fn run_design_cycle(
        &mut self,
        config: CycleConfig,
    ) -> Result<CycleResult, SwarmError> {
        // Handle empty population
        if config.population_size == 0 {
            return Ok(CycleResult {
                promoted: Vec::new(),
                generation: config.generation,
                variants_created: 0,
                variants_scored: 0,
                variants_validated: 0,
                variants_screened: 0,
            });
        }

        // 1. DESIGN — explore sequence space
        debug!(generation = config.generation, "DESIGN phase");
        let explore_task = AgentTask::Explore {
            population: Vec::new(), // coordinator provides seed population via task
            mutation_rate: config.mutation_rate,
            crossover_rate: config.crossover_rate,
        };
        let explore_result = self.explorer.execute(explore_task).await?;
        let candidates = match explore_result {
            AgentResult::Explored { variants } => variants,
            _ => return Err(SwarmError::AgentFailed("explorer returned wrong result type".into())),
        };
        let variants_created = candidates.len();

        // 2. SCORE — predict fitness
        debug!(count = variants_created, "SCORE phase");
        let score_task = AgentTask::Score { candidates };
        let score_result = self.scorer.execute(score_task).await?;
        let scored = match score_result {
            AgentResult::Scored { scored } => scored,
            _ => return Err(SwarmError::AgentFailed("scorer returned wrong result type".into())),
        };
        let variants_scored = scored.len();

        // 3. VALIDATE — structural plausibility
        debug!(count = variants_scored, "VALIDATE phase");
        let validate_task = AgentTask::Validate { scored };
        let validate_result = self.validator.execute(validate_task).await?;
        let validated = match validate_result {
            AgentResult::Validated { passed } => passed,
            _ => return Err(SwarmError::AgentFailed("validator returned wrong result type".into())),
        };
        let variants_validated = validated.len();

        // 4. SCREEN — toxicity/safety
        debug!(count = variants_validated, "SCREEN phase");
        let screen_task = AgentTask::Screen { validated };
        let screen_result = self.screener.execute(screen_task).await?;
        let safe = match screen_result {
            AgentResult::Screened { safe } => safe,
            _ => return Err(SwarmError::AgentFailed("screener returned wrong result type".into())),
        };
        let variants_screened = safe.len();

        // 5. QUANTUM (optional)
        if config.quantum_enabled {
            if let Some(ref quantum_agent) = self.quantum {
                debug!(count = variants_screened, "QUANTUM phase");
                let quantum_task = AgentTask::QuantumDispatch {
                    candidates: safe.clone(),
                };
                let _ = quantum_agent.execute(quantum_task).await?;
            }
        }

        // 6. LOG — commit to ledger
        let promoted = if safe.len() > config.top_k {
            safe[..config.top_k].to_vec()
        } else {
            safe
        };

        let log_payload = serde_json::to_vec(&CycleResult {
            promoted: promoted.clone(),
            generation: config.generation,
            variants_created,
            variants_scored,
            variants_validated,
            variants_screened,
        })
        .map_err(|e| SwarmError::SerializationFailed(e.to_string()))?;

        self.ledger
            .append_entry(EntryType::CycleCompleted, log_payload)
            .map_err(|e| SwarmError::LedgerFailed(e.to_string()))?;

        info!(
            generation = config.generation,
            promoted = promoted.len(),
            "cycle complete"
        );

        Ok(CycleResult {
            promoted,
            generation: config.generation,
            variants_created,
            variants_scored,
            variants_validated,
            variants_screened,
        })
    }
}
