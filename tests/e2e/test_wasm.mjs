#!/usr/bin/env node
/**
 * E2E smoke test: WASM module under Node.js
 *
 * Loads the pe-wasm package built by wasm-pack and exercises the core
 * exported functions.
 *
 * Prerequisites: run `./build-wasm.sh` first to produce web/pkg/.
 *
 * Usage: node tests/e2e/test_wasm.mjs
 */

import { readFile } from "node:fs/promises";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = dirname(fileURLToPath(import.meta.url));
const ROOT = join(__dirname, "../..");
const PKG_DIR = join(ROOT, "web/pkg");

let passed = 0;
let failed = 0;

function pass(description) {
  passed++;
  console.log(`  PASS: ${description}`);
}

function fail(description, detail) {
  failed++;
  console.log(`  FAIL: ${description}`);
  console.log(`        ${detail}`);
}

function assertType(description, value, expectedType) {
  if (typeof value === expectedType) {
    pass(`${description} (typeof === ${expectedType})`);
  } else {
    fail(description, `expected typeof ${expectedType}, got ${typeof value}`);
  }
}

function assertRange(description, value, min, max) {
  if (typeof value === "number" && value >= min && value <= max) {
    pass(`${description} (${value} in [${min}, ${max}])`);
  } else {
    fail(description, `${value} not in [${min}, ${max}]`);
  }
}

function assertField(description, obj, field) {
  if (obj != null && field in obj) {
    pass(`${description} (has .${field})`);
  } else {
    fail(description, `missing field '${field}'`);
  }
}

// ── Load WASM ────────────────────────────────────────────────────────

async function loadWasm() {
  // wasm-pack --target web produces an ES module with an init() default export
  // that accepts a WebAssembly.Module or fetch Response. Under Node, we
  // provide the raw bytes.
  const wasmPath = join(PKG_DIR, "pe_wasm_bg.wasm");
  const jsPath = join(PKG_DIR, "pe_wasm.js");

  // Check if pkg exists
  try {
    await readFile(wasmPath);
  } catch {
    console.log("SKIP: web/pkg/ not found. Run ./build-wasm.sh first.");
    process.exit(0);
  }

  // Dynamic import of the wasm-pack generated JS module
  const mod = await import(jsPath);

  // Initialize WASM with raw bytes
  const wasmBytes = await readFile(wasmPath);
  await mod.default(wasmBytes);

  return mod;
}

// ── Tests ────────────────────────────────────────────────────────────

async function main() {
  console.log("==> Loading WASM module...");
  const wasm = await loadWasm();
  console.log("  WASM module loaded.");

  console.log("");
  console.log("==> Running WASM smoke tests...");

  // Test: score_sequence
  console.log("");
  console.log("--- score_sequence ---");
  try {
    const score = wasm.score_sequence("MKWVTFISLLLLFSSAYS");
    assertField("score result", score, "composite");
    assertField("score result", score, "reprogramming_efficiency");
    assertField("score result", score, "expression_stability");
    assertField("score result", score, "structural_plausibility");
    assertField("score result", score, "safety_score");
    assertRange("composite in [0,1]", score.composite, 0.0, 1.0);
    assertRange("reprogramming in [0,1]", score.reprogramming_efficiency, 0.0, 1.0);
    assertRange("stability in [0,1]", score.expression_stability, 0.0, 1.0);
    assertRange("plausibility in [0,1]", score.structural_plausibility, 0.0, 1.0);
    assertRange("safety in [0,1]", score.safety_score, 0.0, 1.0);
  } catch (err) {
    fail("score_sequence", String(err));
  }

  // Test: verify_ledger (empty chain is valid)
  console.log("");
  console.log("--- verify_ledger ---");
  try {
    const result = wasm.verify_ledger();
    assertField("verify result", result, "valid");
    if (result.valid === true) {
      pass("ledger is valid");
    } else {
      fail("ledger is valid", `got valid=${result.valid}`);
    }
  } catch (err) {
    fail("verify_ledger", String(err));
  }

  // Test: search_similar (empty store returns empty)
  console.log("");
  console.log("--- search_similar ---");
  try {
    const results = wasm.search_similar("MKWVTFISLLLLFSSAYS", 5);
    if (Array.isArray(results)) {
      pass("search returns array");
    } else {
      fail("search returns array", `got ${typeof results}`);
    }
  } catch (err) {
    fail("search_similar", String(err));
  }

  // Test: run_evolution_step
  console.log("");
  console.log("--- run_evolution_step ---");
  try {
    const population = JSON.stringify([
      { name: "wt-1", sequence: "MKWVTFISLLLLFSSAYS", target_factor: "OCT4" },
      { name: "wt-2", sequence: "ACDEFGHIKLMNPQRSTV", target_factor: "SOX2" },
    ]);
    const config = JSON.stringify({
      generation: 0,
      population_size: 2,
      mutation_rate: 0.3,
      crossover_rate: 0.2,
      top_k: 2,
    });
    const result = wasm.run_evolution_step(population, config);
    assertField("evolution result", result, "generation");
    assertField("evolution result", result, "variants_created");
    assertField("evolution result", result, "promoted");
  } catch (err) {
    fail("run_evolution_step", String(err));
  }

  // ── Summary ────────────────────────────────────────────────────────

  console.log("");
  console.log("========================================");
  console.log(`  E2E WASM: ${passed} passed, ${failed} failed`);
  console.log("========================================");

  process.exit(failed > 0 ? 1 : 0);
}

main().catch((err) => {
  console.error("FATAL:", err);
  process.exit(1);
});
