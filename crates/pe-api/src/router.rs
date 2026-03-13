//! Router construction with CORS configuration.

use axum::routing::{get, post};
use axum::Router;
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;

use crate::handlers;
use crate::state::AppState;

/// Build the complete axum Router with all endpoints and middleware.
pub fn build_router(state: AppState) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let state = Arc::new(state);

    Router::new()
        // Variant endpoints
        .route("/api/variants", post(handlers::create_variant))
        .route("/api/variants/score", post(handlers::score_variant))
        .route("/api/variants/search", get(handlers::search_variants))
        .route("/api/variants/{id}", get(handlers::get_variant))
        // Evolution endpoints
        .route("/api/evolution/cycle", post(handlers::run_cycle))
        // Ledger endpoints
        .route("/api/ledger/verify", get(handlers::verify_ledger))
        .route("/api/ledger/entries", get(handlers::ledger_entries))
        // Health
        .route("/api/health", get(handlers::health))
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}
