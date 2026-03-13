//! Command implementations returning structured output.

use std::fs;

use anyhow::{Context, Result};
use chrono::Utc;
use serde::Serialize;

use pe_core::{
    AminoAcidSequence, FitnessScore, ProteinVariant, ScoredVariant, YamanakaFactor,
};
use pe_neural::traits::FitnessPredictor;
use pe_quantum_wasm::{MolecularHamiltonian, QaoaConfig, QaoaRunner, QuboInstance, VqeConfig, VqeRunner};
use pe_rvf::{Manifest, RvfBuilder, RvfFile, SegmentType};
use pe_rvf::traits::RvfAssembler;
use pe_swarm::SimpleEvolutionEngine;
use pe_swarm::traits::EvolutionEngine;
use pe_vector::InMemoryVectorStore;
use pe_vector::traits::{EmbeddingModel, VectorStore};

use crate::wiring::{self, HashEmbedder, SignedLedger};

// ── Output DTOs ──────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct ScoreOutput {
    pub reprogramming_efficiency: f64,
    pub expression_stability: f64,
    pub structural_plausibility: f64,
    pub safety_score: f64,
    pub composite: f64,
}

impl From<&FitnessScore> for ScoreOutput {
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

#[derive(Debug, Serialize)]
pub struct GenerationSummary {
    pub generation: u32,
    pub variants_created: usize,
    pub variants_scored: usize,
    pub top_composite: f64,
    pub avg_composite: f64,
    pub promoted: Vec<PromotedVariant>,
}

#[derive(Debug, Serialize)]
pub struct PromotedVariant {
    pub name: String,
    pub sequence: String,
    pub composite: f64,
}

#[derive(Debug, Serialize)]
pub struct SearchHit {
    pub id: String,
    pub similarity: f32,
}

#[derive(Debug, Serialize)]
pub struct VqeOutput {
    pub ground_state_energy: f64,
    pub optimal_parameters: Vec<f64>,
    pub converged: bool,
    pub iterations: usize,
}

#[derive(Debug, Serialize)]
pub struct QaoaOutput {
    pub best_bitstring: usize,
    pub best_cost: f64,
    pub converged: bool,
    pub iterations: usize,
}

#[derive(Debug, Serialize)]
pub struct VerifyOutput {
    pub valid: bool,
    pub entry_count: usize,
}

#[derive(Debug, Serialize)]
pub struct LedgerEntryOutput {
    pub sequence_number: u64,
    pub entry_type: String,
    pub timestamp: String,
    pub hash: String,
}

#[derive(Debug, Serialize)]
pub struct InspectOutput {
    pub name: String,
    pub version: String,
    pub capabilities: Vec<String>,
    pub segment_count: usize,
    pub segments: Vec<SegmentInfo>,
    pub file_hash: String,
}

#[derive(Debug, Serialize)]
pub struct SegmentInfo {
    pub segment_type: String,
    pub size_bytes: usize,
}

#[derive(Debug, Serialize)]
pub struct InitOutput {
    pub path: String,
    pub file_hash: String,
}

// ── Commands ─────────────────────────────────────────────────────────

/// Create a new empty .rvf file with MANIFEST_SEG.
pub fn cmd_init(output: &str) -> Result<InitOutput> {
    let manifest = Manifest::new(
        "protein-engine".into(),
        env!("CARGO_PKG_VERSION").into(),
        None,
        None,
        Utc::now(),
    )?;

    let manifest_bytes = serde_json::to_vec(&manifest)?;

    let mut builder = RvfBuilder::new();
    builder.set_manifest(manifest);
    builder.add_segment(SegmentType::ManifestSeg, manifest_bytes)?;
    let rvf = builder.build()?;

    let serialized = rvf.serialize();
    fs::write(output, &serialized)
        .with_context(|| format!("failed to write {output}"))?;

    Ok(InitOutput {
        path: output.to_string(),
        file_hash: hex::encode(rvf.file_hash()),
    })
}

/// Score a protein sequence.
pub fn cmd_score(sequence: &str) -> Result<ScoreOutput> {
    let seq = AminoAcidSequence::new(sequence)?;
    let variant = ProteinVariant::wild_type("cli-query", seq, YamanakaFactor::OCT4);

    let embedder = HashEmbedder;
    let predictor = wiring::build_predictor();

    let embedding = embedder.embed(variant.sequence())
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    let score = predictor.predict(&variant, &embedding)
        .map_err(|e| anyhow::anyhow!("{e}"))?;

    Ok(ScoreOutput::from(&score))
}

/// Run N evolution generations.
pub fn cmd_evolve(
    generations: u32,
    population_size: usize,
    seed_sequence: &str,
    mutation_rate: f64,
    crossover_rate: f64,
    top_k: usize,
) -> Result<Vec<GenerationSummary>> {
    let base_seq = AminoAcidSequence::new(seed_sequence)?;
    let embedder = HashEmbedder;
    let predictor = wiring::build_predictor();
    let engine = SimpleEvolutionEngine::new();

    // Seed population from mutations of the base sequence
    let base = ProteinVariant::wild_type("seed", base_seq, YamanakaFactor::OCT4);
    let base_emb = embedder.embed(base.sequence()).map_err(|e| anyhow::anyhow!("{e}"))?;
    let base_score = predictor.predict(&base, &base_emb).map_err(|e| anyhow::anyhow!("{e}"))?;

    let mut population = vec![ScoredVariant {
        variant: base.clone(),
        score: base_score,
    }];

    // Fill initial population with mutations
    for _ in 1..population_size {
        let mutant = engine.mutate(&base).map_err(|e| anyhow::anyhow!("{e}"))?;
        let emb = embedder.embed(mutant.sequence()).map_err(|e| anyhow::anyhow!("{e}"))?;
        let sc = predictor.predict(&mutant, &emb).map_err(|e| anyhow::anyhow!("{e}"))?;
        population.push(ScoredVariant { variant: mutant, score: sc });
    }

    let mut summaries = Vec::new();

    for gen in 0..generations {
        let mut offspring = Vec::new();

        // Mutate
        for sv in &population {
            if rand::random::<f64>() < mutation_rate {
                if let Ok(m) = engine.mutate(&sv.variant) {
                    offspring.push(m);
                }
            }
        }

        // Crossover
        if population.len() >= 2 {
            for i in 0..population.len() - 1 {
                if rand::random::<f64>() < crossover_rate {
                    let j = (i + 1) % population.len();
                    if let Ok(child) = engine.crossover(&population[i].variant, &population[j].variant) {
                        offspring.push(child);
                    }
                }
            }
        }

        let variants_created = offspring.len();

        // Score offspring
        for child in offspring {
            let emb = embedder.embed(child.sequence()).map_err(|e| anyhow::anyhow!("{e}"))?;
            let sc = predictor.predict(&child, &emb).map_err(|e| anyhow::anyhow!("{e}"))?;
            population.push(ScoredVariant { variant: child, score: sc });
        }

        let variants_scored = population.len();

        // Select top-k
        population = engine.select(&population, top_k);

        let top_composite = population
            .first()
            .map(|sv| sv.score.composite())
            .unwrap_or(0.0);
        let avg_composite = if population.is_empty() {
            0.0
        } else {
            population.iter().map(|sv| sv.score.composite()).sum::<f64>() / population.len() as f64
        };

        let promoted: Vec<PromotedVariant> = population
            .iter()
            .map(|sv| PromotedVariant {
                name: sv.variant.name().to_string(),
                sequence: sv.variant.sequence().to_string(),
                composite: sv.score.composite(),
            })
            .collect();

        summaries.push(GenerationSummary {
            generation: gen,
            variants_created,
            variants_scored,
            top_composite,
            avg_composite,
            promoted,
        });
    }

    Ok(summaries)
}

/// Search nearest neighbors (requires a populated store).
pub fn cmd_search(
    sequence: &str,
    k: usize,
    store: &dyn VectorStore,
    embedder: &dyn EmbeddingModel,
) -> Result<Vec<SearchHit>> {
    let seq = AminoAcidSequence::new(sequence)?;
    let embedding = embedder.embed(&seq).map_err(|e| anyhow::anyhow!("{e}"))?;

    let results = store
        .search_nearest(&embedding, k)
        .map_err(|e| anyhow::anyhow!("{e}"))?;

    Ok(results
        .into_iter()
        .map(|(id, similarity)| SearchHit {
            id: id.to_string(),
            similarity,
        })
        .collect())
}

/// Run VQE simulation.
pub fn cmd_quantum_vqe(molecule: &str) -> Result<VqeOutput> {
    let hamiltonian = if molecule.eq_ignore_ascii_case("h2") {
        MolecularHamiltonian::h2_molecule()
    } else {
        let data = fs::read(molecule)
            .with_context(|| format!("failed to read {molecule}"))?;
        serde_json::from_slice(&data)
            .with_context(|| format!("invalid hamiltonian JSON in {molecule}"))?
    };

    let runner = VqeRunner::new(VqeConfig::default());
    let result = runner.run(&hamiltonian).map_err(|e| anyhow::anyhow!("{e}"))?;

    Ok(VqeOutput {
        ground_state_energy: result.ground_state_energy,
        optimal_parameters: result.optimal_parameters,
        converged: result.converged,
        iterations: result.iterations,
    })
}

/// Run QAOA optimization.
pub fn cmd_quantum_qaoa(qubo_file: &str) -> Result<QaoaOutput> {
    let data = fs::read(qubo_file)
        .with_context(|| format!("failed to read {qubo_file}"))?;
    let qubo: QuboInstance = serde_json::from_slice(&data)
        .with_context(|| format!("invalid QUBO JSON in {qubo_file}"))?;

    let runner = QaoaRunner::new(QaoaConfig::default());
    let result = runner.run(&qubo).map_err(|e| anyhow::anyhow!("{e}"))?;

    Ok(QaoaOutput {
        best_bitstring: result.best_bitstring,
        best_cost: result.best_cost,
        converged: result.converged,
        iterations: result.iterations,
    })
}

/// Verify the journal chain.
pub fn cmd_ledger_verify(ledger: &dyn pe_ledger::LedgerWriter) -> Result<VerifyOutput> {
    let valid = ledger.verify_chain().map_err(|e| anyhow::anyhow!("{e}"))?;
    let entry_count = ledger.len();
    Ok(VerifyOutput { valid, entry_count })
}

/// Show journal entries.
pub fn cmd_ledger_show(ledger: &SignedLedger, limit: usize) -> Result<Vec<LedgerEntryOutput>> {
    let entries = ledger.chain().entries();
    let start = entries.len().saturating_sub(limit);

    Ok(entries[start..]
        .iter()
        .map(|e| LedgerEntryOutput {
            sequence_number: e.sequence_number,
            entry_type: format!("{:?}", e.entry_type),
            timestamp: e.timestamp.to_rfc3339(),
            hash: hex::encode(e.compute_hash().as_bytes()),
        })
        .collect())
}

/// Build an .rvf file from current state.
pub fn cmd_rvf_build(output: &str) -> Result<InitOutput> {
    // Build with manifest + empty segments for a fresh state
    let manifest = Manifest::new(
        "protein-engine".into(),
        env!("CARGO_PKG_VERSION").into(),
        None,
        None,
        Utc::now(),
    )?;

    let manifest_bytes = serde_json::to_vec(&manifest)?;
    let store = InMemoryVectorStore::new();

    let mut builder = RvfBuilder::new();
    builder.set_manifest(manifest);
    builder.add_segment(SegmentType::ManifestSeg, manifest_bytes)?;
    builder.add_segment(SegmentType::VecSeg, store.to_vec_seg())?;
    builder.add_segment(SegmentType::IndexSeg, store.to_index_seg())?;

    let rvf = builder.build()?;
    let serialized = rvf.serialize();

    fs::write(output, &serialized)
        .with_context(|| format!("failed to write {output}"))?;

    Ok(InitOutput {
        path: output.to_string(),
        file_hash: hex::encode(rvf.file_hash()),
    })
}

/// Inspect an .rvf file.
pub fn cmd_rvf_inspect(path: &str) -> Result<InspectOutput> {
    let data = fs::read(path)
        .with_context(|| format!("failed to read {path}"))?;

    let rvf = RvfFile::deserialize(&data)
        .map_err(|e| anyhow::anyhow!("RVF deserialization failed: {e}"))?;

    let manifest = rvf.manifest();
    let segments = rvf.segments();

    let segment_infos: Vec<SegmentInfo> = segments
        .iter()
        .map(|(seg_type, data)| SegmentInfo {
            segment_type: format!("{:?}", seg_type),
            size_bytes: data.len(),
        })
        .collect();

    Ok(InspectOutput {
        name: manifest.name.clone(),
        version: manifest.version.clone(),
        capabilities: manifest.capabilities.iter().map(|c| format!("{:?}", c)).collect(),
        segment_count: segments.len(),
        segments: segment_infos,
        file_hash: hex::encode(rvf.file_hash()),
    })
}
