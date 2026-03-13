use std::sync::Arc;
use tokio::sync::RwLock;

use pe_ledger::LedgerWriter;
use pe_neural::traits::FitnessPredictor;
use pe_swarm::SwarmCoordinator;
use pe_vector::traits::{EmbeddingModel, VectorStore};

/// Application state holding all domain trait objects via DI (ADR-004).
#[derive(Clone)]
pub struct AppState {
    pub scorer: Arc<dyn FitnessPredictor>,
    pub store: Arc<RwLock<dyn VectorStore>>,
    pub embedder: Arc<dyn EmbeddingModel>,
    pub ledger: Arc<RwLock<dyn LedgerWriter>>,
    pub coordinator: Arc<RwLock<dyn SwarmCoordinator>>,
}
