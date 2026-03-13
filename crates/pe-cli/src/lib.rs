//! pe-cli: Native CLI entry point and composition root for Protein-Engine.
//!
//! Wires all domain crates into concrete trait objects (ADR-004) and
//! exposes subcommands for scoring, evolution, quantum simulation,
//! ledger management, RVF assembly, and HTTP serving.

pub mod cli;
pub mod commands;
pub mod format;
pub mod wiring;

#[cfg(test)]
mod tests;
