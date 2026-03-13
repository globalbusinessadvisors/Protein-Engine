//! Composition root (ADR-004): construct all real implementations and
//! wire them into trait objects.

use std::sync::Arc;
use tokio::sync::RwLock;

use async_trait::async_trait;

use pe_core::{
    AminoAcidSequence, Embedding320, FitnessWeights, ScoredVariant,
};
use pe_ledger::{
    EntryHash, EntryType, JournalChain, LedgerError, LedgerWriter, MlDsaSigner,
};
use pe_neural::traits::FitnessPredictor;
use pe_neural::{EnsemblePredictor, LstmScorer, NBeatsScorer, TransformerScorer};
use pe_swarm::{
    AgentResult, AgentRole, AgentTask, DefaultCoordinator, EvolutionEngine,
    SimpleEvolutionEngine, SwarmAgent, SwarmError,
};
use pe_vector::traits::EmbeddingModel;
use pe_vector::{InMemoryVectorStore, VectorError};

use pe_api::state::AppState;

// ── HashEmbedder (deterministic, no ESM-2) ───────────────────────────

/// Deterministic hash-based embedder for CLI use when no model weights are loaded.
pub struct HashEmbedder;

impl EmbeddingModel for HashEmbedder {
    fn embed(&self, sequence: &AminoAcidSequence) -> Result<Embedding320, VectorError> {
        let mut data = [0.0f32; 320];
        let residues = sequence.as_slice();

        for (i, &aa) in residues.iter().enumerate() {
            let aa_code = aa.to_char() as u32;
            let seed = aa_code.wrapping_mul(2654435761).wrapping_add(i as u32);
            for j in 0..4 {
                let idx = (i * 4 + j) % 320;
                let hash = seed.wrapping_mul((j as u32 + 1).wrapping_mul(0x9E3779B9));
                let val = (hash as f32 / u32::MAX as f32) * 2.0 - 1.0;
                data[idx] += val;
            }
        }

        let norm: f32 = data.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm > 1e-8 {
            for val in &mut data {
                *val /= norm;
            }
        }

        Ok(Embedding320::new(data))
    }
}

// ── SignedLedger (JournalChain + MlDsaSigner → LedgerWriter) ─────────

/// Adapter: wraps JournalChain + MlDsaSigner behind the LedgerWriter trait.
pub struct SignedLedger {
    chain: JournalChain,
    signer: MlDsaSigner,
}

impl SignedLedger {
    pub fn new() -> Self {
        Self {
            chain: JournalChain::new(),
            signer: MlDsaSigner::generate(),
        }
    }

    pub fn chain(&self) -> &JournalChain {
        &self.chain
    }
}

impl LedgerWriter for SignedLedger {
    fn append_entry(
        &mut self,
        entry_type: EntryType,
        payload: Vec<u8>,
    ) -> Result<EntryHash, LedgerError> {
        self.chain.append_entry(entry_type, payload, &self.signer)
    }

    fn verify_chain(&self) -> Result<bool, LedgerError> {
        self.chain.verify_chain(&self.signer)
    }

    fn len(&self) -> usize {
        self.chain.len()
    }
}

// ── Stub SwarmAgents ─────────────────────────────────────────────────

/// Explorer agent: generates mutated variants from a seed population.
struct ExplorerAgent {
    engine: SimpleEvolutionEngine,
}

#[async_trait]
impl SwarmAgent for ExplorerAgent {
    async fn execute(&self, task: AgentTask) -> Result<AgentResult, SwarmError> {
        match task {
            AgentTask::Explore { population, .. } => {
                let mut variants = Vec::new();
                for p in &population {
                    if let Ok(m) = self.engine.mutate(p) {
                        variants.push(m);
                    }
                }
                Ok(AgentResult::Explored { variants })
            }
            _ => Err(SwarmError::AgentFailed("wrong task for explorer".into())),
        }
    }

    fn role(&self) -> AgentRole {
        AgentRole::SequenceExplorer
    }
}

/// Scorer agent: scores candidates using the neural ensemble.
struct ScorerAgent {
    predictor: Arc<dyn FitnessPredictor>,
    embedder: Arc<dyn EmbeddingModel>,
}

#[async_trait]
impl SwarmAgent for ScorerAgent {
    async fn execute(&self, task: AgentTask) -> Result<AgentResult, SwarmError> {
        match task {
            AgentTask::Score { candidates } => {
                let mut scored = Vec::new();
                for v in candidates {
                    let emb = self
                        .embedder
                        .embed(v.sequence())
                        .map_err(|e| SwarmError::AgentFailed(e.to_string()))?;
                    let score = self
                        .predictor
                        .predict(&v, &emb)
                        .map_err(|e| SwarmError::AgentFailed(e.to_string()))?;
                    scored.push(ScoredVariant { variant: v, score });
                }
                Ok(AgentResult::Scored { scored })
            }
            _ => Err(SwarmError::AgentFailed("wrong task for scorer".into())),
        }
    }

    fn role(&self) -> AgentRole {
        AgentRole::FitnessScorer
    }
}

/// Passthrough validator: accepts all variants.
struct PassthroughValidator;

#[async_trait]
impl SwarmAgent for PassthroughValidator {
    async fn execute(&self, task: AgentTask) -> Result<AgentResult, SwarmError> {
        match task {
            AgentTask::Validate { scored } => Ok(AgentResult::Validated { passed: scored }),
            _ => Err(SwarmError::AgentFailed(
                "wrong task for validator".into(),
            )),
        }
    }

    fn role(&self) -> AgentRole {
        AgentRole::StructuralValidator
    }
}

/// Passthrough screener: accepts all variants as safe.
struct PassthroughScreener;

#[async_trait]
impl SwarmAgent for PassthroughScreener {
    async fn execute(&self, task: AgentTask) -> Result<AgentResult, SwarmError> {
        match task {
            AgentTask::Screen { validated } => Ok(AgentResult::Screened { safe: validated }),
            _ => Err(SwarmError::AgentFailed(
                "wrong task for screener".into(),
            )),
        }
    }

    fn role(&self) -> AgentRole {
        AgentRole::ToxicityScreener
    }
}

// ── Factory functions ────────────────────────────────────────────────

/// Build the default EnsemblePredictor with stub scorers.
pub fn build_predictor() -> EnsemblePredictor<TransformerScorer, LstmScorer, NBeatsScorer> {
    let weights = FitnessWeights::default_weights();
    EnsemblePredictor::new(
        TransformerScorer::new(0.0),
        LstmScorer::new(0.75),
        NBeatsScorer::new(0.7),
        weights,
    )
}

/// Build the complete AppState for the HTTP server.
pub fn build_app_state() -> AppState {
    let predictor = build_predictor();
    let embedder = Arc::new(HashEmbedder) as Arc<dyn EmbeddingModel>;

    let scorer_agent = ScorerAgent {
        predictor: Arc::new(build_predictor()),
        embedder: embedder.clone(),
    };

    let ledger = SignedLedger::new();

    let coordinator = DefaultCoordinator::new(
        Box::new(ExplorerAgent {
            engine: SimpleEvolutionEngine::new(),
        }),
        Box::new(scorer_agent),
        Box::new(PassthroughValidator),
        Box::new(PassthroughScreener),
        None,
        Box::new(SignedLedger::new()),
    );

    AppState {
        scorer: Arc::new(predictor),
        embedder,
        store: Arc::new(RwLock::new(InMemoryVectorStore::new())),
        ledger: Arc::new(RwLock::new(ledger)),
        coordinator: Arc::new(RwLock::new(coordinator)),
    }
}
