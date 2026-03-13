//! pe-api: Axum HTTP/WebSocket API server for Protein-Engine.
//!
//! Provides REST endpoints and WebSocket streams for variant scoring,
//! evolution cycles, similarity search, ledger verification, and health checks.

pub mod error;
pub mod handlers;
pub mod router;
pub mod state;

#[cfg(test)]
mod tests;