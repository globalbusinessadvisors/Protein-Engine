//! pe-chemistry: HTTP bridge to the pyChemiQ quantum chemistry sidecar.
//!
//! Provides an anti-corruption layer translating between Protein-Engine
//! domain types and the pyChemiQ/pyqpanda Python sidecar's HTTP/JSON API.

pub mod bridge;
pub(crate) mod dto;
pub mod error;
pub mod http_client;

#[cfg(test)]
mod tests;
