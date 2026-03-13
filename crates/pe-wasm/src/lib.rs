//! pe-wasm: WASM browser entry point for Protein-Engine.
//!
//! Provides `wasm_bindgen`-exported functions for sequence scoring,
//! evolution steps, local quantum simulation, and ledger verification
//! — all running entirely in the browser from a single `.rvf` file.

use std::cell::RefCell;

use wasm_bindgen::prelude::*;

pub mod embedder;
pub mod engine;
pub mod error;

#[cfg(test)]
mod tests;

use engine::WasmEngine;
use error::to_js_result;

// ── Thread-local engine (WASM is single-threaded) ────────────────────

thread_local! {
    static ENGINE: RefCell<WasmEngine> = RefCell::new(WasmEngine::new());
}

fn with_engine<F, R>(f: F) -> Result<R, String>
where
    F: FnOnce(&mut WasmEngine) -> Result<R, String>,
{
    ENGINE.with(|cell| {
        let mut engine = cell.borrow_mut();
        f(&mut engine)
    })
}

// ── WASM-exported functions ──────────────────────────────────────────

/// Initialize the WASM module: set up panic hook.
#[wasm_bindgen(start)]
pub fn init() {
    console_error_panic_hook::set_once();
}

/// Score a protein sequence, returning fitness components as JSON.
///
/// Returns: `{reprogramming_efficiency, expression_stability, structural_plausibility, safety_score, composite}`
/// Error: `{error: "..."}`
#[wasm_bindgen]
pub fn score_sequence(sequence: &str) -> Result<JsValue, JsValue> {
    to_js_result(with_engine(|e| e.score_sequence(sequence)))
}

/// Run one evolution generation on a population.
///
/// `population_json`: JSON array of `{name, sequence, target_factor}` objects.
/// `config_json`: `{generation, population_size, mutation_rate, crossover_rate, top_k}`
///
/// Returns: `{generation, variants_created, variants_scored, promoted: [{name, sequence, composite}]}`
#[wasm_bindgen]
pub fn run_evolution_step(population_json: &str, config_json: &str) -> Result<JsValue, JsValue> {
    to_js_result(with_engine(|e| {
        e.run_evolution_step(population_json, config_json)
    }))
}

/// Run VQE on the local quantum simulator.
///
/// `hamiltonian_json`: JSON `MolecularHamiltonian` with `{num_qubits, terms: [{coefficient, operators: [[qubit, op]]}]}`
///
/// Returns: `{ground_state_energy, optimal_parameters, converged, iterations}`
#[wasm_bindgen]
pub fn run_local_quantum_sim(hamiltonian_json: &str) -> Result<JsValue, JsValue> {
    to_js_result(with_engine(|e| e.run_quantum_sim(hamiltonian_json)))
}

/// Search for similar sequences in the vector store.
///
/// Returns: JSON array of `{id, similarity}` objects sorted by descending similarity.
#[wasm_bindgen]
pub fn search_similar(sequence: &str, k: u32) -> Result<JsValue, JsValue> {
    to_js_result(with_engine(|e| {
        e.search_similar(sequence, k as usize)
    }))
}

/// Verify the local journal chain integrity.
///
/// Returns: `{valid: true/false}`
#[wasm_bindgen]
pub fn verify_ledger() -> Result<JsValue, JsValue> {
    to_js_result(with_engine(|e| e.verify_ledger()))
}

/// Load an `.rvf` file, populating the engine from its segments.
///
/// Extracts VEC_SEG + INDEX_SEG → vector store, JOURNAL_SEG → ledger chain.
///
/// Returns: `{vectors_loaded, journal_entries}`
#[wasm_bindgen]
pub fn load_rvf(data: &[u8]) -> Result<JsValue, JsValue> {
    to_js_result(with_engine(|e| e.load_rvf(data)))
}
