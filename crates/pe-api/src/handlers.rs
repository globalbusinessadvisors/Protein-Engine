//! Axum request handlers for all REST endpoints.

use axum::extract::{Path, Query, State};
use axum::response::Json;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use pe_core::{AminoAcidSequence, FitnessScore, ProteinVariant, YamanakaFactor};
use pe_swarm::CycleConfig;

use crate::error::ApiError;
use crate::state::AppState;

// ── Request / Response DTOs ─────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct CreateVariantRequest {
    pub name: String,
    pub sequence: String,
    pub target_factor: String,
}

#[derive(Debug, Deserialize)]
pub struct ScoreVariantRequest {
    pub name: String,
    pub sequence: String,
    pub target_factor: String,
}

#[derive(Debug, Serialize)]
pub struct ScoreResponse {
    pub reprogramming_efficiency: f64,
    pub expression_stability: f64,
    pub structural_plausibility: f64,
    pub safety_score: f64,
    pub composite: f64,
}

impl From<&FitnessScore> for ScoreResponse {
    fn from(s: &FitnessScore) -> Self {
        Self {
            reprogramming_efficiency: s.reprogramming_efficiency(),
            expression_stability: s.expression_stability(),
            structural_plausibility: s.structural_plausibility(),
            safety_score: s.safety_score(),
            composite: s.composite(),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct SearchQuery {
    pub sequence: String,
    #[serde(default = "default_k")]
    pub k: usize,
}

fn default_k() -> usize {
    5
}

#[derive(Debug, Serialize)]
pub struct SearchResult {
    pub id: Uuid,
    pub similarity: f32,
}

#[derive(Debug, Deserialize)]
pub struct TopQuery {
    #[serde(default = "default_top_k")]
    pub k: usize,
}

fn default_top_k() -> usize {
    10
}

#[derive(Debug, Deserialize)]
pub struct LedgerQuery {
    #[serde(default = "default_limit")]
    pub limit: usize,
    #[serde(default)]
    pub offset: usize,
}

fn default_limit() -> usize {
    50
}

#[derive(Debug, Serialize)]
pub struct VerifyResponse {
    pub valid: bool,
}

#[derive(Debug, Serialize)]
pub struct LedgerInfoResponse {
    pub entry_count: usize,
}

#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub status: &'static str,
}

// ── Helpers ─────────────────────────────────────────────────────────

fn parse_factor(s: &str) -> Result<YamanakaFactor, ApiError> {
    match s.to_uppercase().as_str() {
        "OCT4" => Ok(YamanakaFactor::OCT4),
        "SOX2" => Ok(YamanakaFactor::SOX2),
        "KLF4" => Ok(YamanakaFactor::KLF4),
        "CMYC" => Ok(YamanakaFactor::CMYC),
        _ => Err(ApiError::bad_request(format!(
            "unknown target factor: {}",
            s
        ))),
    }
}

// ── Handlers ────────────────────────────────────────────────────────

/// POST /api/variants — create a new ProteinVariant
pub async fn create_variant(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateVariantRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let seq = AminoAcidSequence::new(&req.sequence).map_err(ApiError::from)?;
    let factor = parse_factor(&req.target_factor)?;
    let variant = ProteinVariant::wild_type(req.name, seq, factor);

    // Store embedding
    let embedding = state
        .embedder
        .embed(variant.sequence())
        .map_err(|e| ApiError::internal(e.to_string()))?;

    let meta = pe_vector::meta::VariantMeta {
        variant_id: variant.id(),
        target_factor: *variant.target_factor(),
        generation: variant.generation(),
        composite_score: None,
        design_method: pe_vector::meta::DesignMethod::WildType,
    };

    state
        .store
        .write()
        .await
        .insert(variant.id(), embedding, meta)
        .map_err(|e| ApiError::internal(e.to_string()))?;

    let json = serde_json::to_value(&variant)
        .map_err(|e| ApiError::internal(e.to_string()))?;
    Ok(Json(json))
}

/// POST /api/variants/score — score a variant
pub async fn score_variant(
    State(state): State<Arc<AppState>>,
    Json(req): Json<ScoreVariantRequest>,
) -> Result<Json<ScoreResponse>, ApiError> {
    let seq = AminoAcidSequence::new(&req.sequence).map_err(ApiError::from)?;
    let factor = parse_factor(&req.target_factor)?;
    let variant = ProteinVariant::wild_type(req.name, seq, factor);

    let embedding = state
        .embedder
        .embed(variant.sequence())
        .map_err(|e| ApiError::internal(e.to_string()))?;

    let score = state
        .scorer
        .predict(&variant, &embedding)
        .map_err(|e| ApiError::internal(e.to_string()))?;

    Ok(Json(ScoreResponse::from(&score)))
}

/// GET /api/variants/:id — get variant by ID
pub async fn get_variant(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let store = state.store.read().await;
    let meta = store
        .get_meta(id)
        .map_err(|e| ApiError::internal(e.to_string()))?;

    match meta {
        Some(m) => {
            let json = serde_json::to_value(&m)
                .map_err(|e| ApiError::internal(e.to_string()))?;
            Ok(Json(json))
        }
        None => Err(ApiError::not_found(format!("variant {} not found", id))),
    }
}

/// GET /api/variants/search?sequence=...&k=5
pub async fn search_variants(
    State(state): State<Arc<AppState>>,
    Query(query): Query<SearchQuery>,
) -> Result<Json<Vec<SearchResult>>, ApiError> {
    let seq = AminoAcidSequence::new(&query.sequence).map_err(ApiError::from)?;

    let embedding = state
        .embedder
        .embed(&seq)
        .map_err(|e| ApiError::internal(e.to_string()))?;

    let store = state.store.read().await;
    let results = store
        .search_nearest(&embedding, query.k)
        .map_err(|e| ApiError::internal(e.to_string()))?;

    let response: Vec<SearchResult> = results
        .into_iter()
        .map(|(id, similarity)| SearchResult { id, similarity })
        .collect();

    Ok(Json(response))
}

/// POST /api/evolution/cycle — trigger a design cycle
pub async fn run_cycle(
    State(state): State<Arc<AppState>>,
    Json(config): Json<CycleConfig>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let mut coord = state.coordinator.write().await;
    let result = coord
        .run_design_cycle(config)
        .await
        .map_err(|e| ApiError::internal(e.to_string()))?;

    let json = serde_json::to_value(&result)
        .map_err(|e| ApiError::internal(e.to_string()))?;
    Ok(Json(json))
}

/// GET /api/ledger/verify — verify journal chain integrity
pub async fn verify_ledger(
    State(state): State<Arc<AppState>>,
) -> Result<Json<VerifyResponse>, ApiError> {
    let ledger = state.ledger.read().await;
    let valid = ledger
        .verify_chain()
        .map_err(|e| ApiError::internal(e.to_string()))?;

    Ok(Json(VerifyResponse { valid }))
}

/// GET /api/ledger/entries — paginated ledger info
pub async fn ledger_entries(
    State(state): State<Arc<AppState>>,
    Query(_query): Query<LedgerQuery>,
) -> Result<Json<LedgerInfoResponse>, ApiError> {
    let ledger = state.ledger.read().await;
    let count = ledger.len();

    Ok(Json(LedgerInfoResponse {
        entry_count: count,
    }))
}

/// GET /api/health — health check
pub async fn health() -> Json<HealthResponse> {
    Json(HealthResponse { status: "ok" })
}
