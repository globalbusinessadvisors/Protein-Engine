//! Clap CLI argument definitions.

use clap::{Parser, Subcommand};

/// AI-native, quantum-aware, distributed protein engineering platform.
#[derive(Parser, Debug)]
#[command(name = "protein-engine", version, about)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,

    /// Output as JSON instead of human-readable table
    #[arg(long, global = true)]
    pub json: bool,

    /// Enable verbose logging
    #[arg(short, long, global = true)]
    pub verbose: bool,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Create a new empty .rvf file with MANIFEST_SEG
    Init {
        /// Output file path
        #[arg(long, default_value = "protein-engine.rvf")]
        output: String,
    },

    /// Score a protein sequence
    Score {
        /// Amino acid sequence (e.g. ACDEFGHIKLMNPQRSTVWY)
        sequence: String,
    },

    /// Run N evolution cycles
    Evolve {
        /// Number of generations to run
        #[arg(long, default_value_t = 1)]
        generations: u32,

        /// Population size per generation
        #[arg(long, default_value_t = 20)]
        population_size: usize,

        /// Base sequence to seed the population
        #[arg(long, default_value = "ACDEFGHIKLMNPQRSTVWY")]
        seed_sequence: String,

        /// Mutation rate [0.0, 1.0]
        #[arg(long, default_value_t = 0.3)]
        mutation_rate: f64,

        /// Crossover rate [0.0, 1.0]
        #[arg(long, default_value_t = 0.2)]
        crossover_rate: f64,

        /// Number of top variants to promote per generation
        #[arg(long, default_value_t = 5)]
        top_k: usize,
    },

    /// Find K nearest neighbors by embedding similarity
    Search {
        /// Query amino acid sequence
        sequence: String,

        /// Number of results
        #[arg(long, default_value_t = 5)]
        k: usize,
    },

    /// Quantum simulation commands
    Quantum {
        #[command(subcommand)]
        cmd: QuantumCommand,
    },

    /// Ledger management commands
    Ledger {
        #[command(subcommand)]
        cmd: LedgerCommand,
    },

    /// RVF file commands
    Rvf {
        #[command(subcommand)]
        cmd: RvfCommand,
    },

    /// Start the axum HTTP server
    Serve {
        /// Port to listen on
        #[arg(long, default_value_t = 8080)]
        port: u16,
    },
}

#[derive(Subcommand, Debug)]
pub enum QuantumCommand {
    /// Run VQE simulation (molecule: "h2" or JSON file path)
    Vqe {
        /// Molecule name ("h2") or path to JSON hamiltonian
        molecule: String,
    },

    /// Run QAOA optimization on a QUBO file (JSON)
    Qaoa {
        /// Path to QUBO matrix JSON file
        qubo_file: String,
    },
}

#[derive(Subcommand, Debug)]
pub enum LedgerCommand {
    /// Verify journal chain integrity
    Verify,

    /// Show last N journal entries
    Show {
        /// Maximum entries to display
        #[arg(long, default_value_t = 10)]
        limit: usize,
    },
}

#[derive(Subcommand, Debug)]
pub enum RvfCommand {
    /// Assemble .rvf from current state
    Build {
        /// Output file path
        #[arg(long, default_value = "output.rvf")]
        output: String,
    },

    /// Inspect an .rvf file
    Inspect {
        /// Path to .rvf file
        path: String,
    },
}
