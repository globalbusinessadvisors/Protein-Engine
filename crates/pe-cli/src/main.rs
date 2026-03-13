//! Protein-Engine CLI — the native command-line entry point.

use anyhow::Result;
use clap::Parser;

use pe_cli::cli::{Cli, Command, LedgerCommand, QuantumCommand, RvfCommand};
use pe_cli::commands;
use pe_cli::format;
use pe_cli::wiring::{self, HashEmbedder, SignedLedger};

use pe_vector::InMemoryVectorStore;

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    if cli.verbose {
        tracing_subscriber::fmt()
            .with_env_filter("protein_engine=debug,pe_=debug")
            .init();
    } else {
        tracing_subscriber::fmt()
            .with_env_filter("protein_engine=info,pe_=warn")
            .init();
    }

    match cli.command {
        Command::Init { output } => {
            let result = commands::cmd_init(&output)?;
            if cli.json {
                println!("{}", format::as_json(&result));
            } else {
                println!("{}", format::kv("Created:", &result.path));
                println!("{}", format::kv("File hash:", &result.file_hash));
            }
        }

        Command::Score { sequence } => {
            let result = commands::cmd_score(&sequence)?;
            if cli.json {
                println!("{}", format::as_json(&result));
            } else {
                println!("{}", format::kv("Reprogramming efficiency:", &format!("{:.4}", result.reprogramming_efficiency)));
                println!("{}", format::kv("Expression stability:", &format!("{:.4}", result.expression_stability)));
                println!("{}", format::kv("Structural plausibility:", &format!("{:.4}", result.structural_plausibility)));
                println!("{}", format::kv("Safety score:", &format!("{:.4}", result.safety_score)));
                println!("{}", format::kv("Composite:", &format!("{:.4}", result.composite)));
            }
        }

        Command::Evolve {
            generations,
            population_size,
            seed_sequence,
            mutation_rate,
            crossover_rate,
            top_k,
        } => {
            let summaries = commands::cmd_evolve(
                generations,
                population_size,
                &seed_sequence,
                mutation_rate,
                crossover_rate,
                top_k,
            )?;

            if cli.json {
                println!("{}", format::as_json(&summaries));
            } else {
                for s in &summaries {
                    println!("Generation {}", s.generation);
                    println!("{}", format::kv("  Variants created:", &s.variants_created.to_string()));
                    println!("{}", format::kv("  Variants scored:", &s.variants_scored.to_string()));
                    println!("{}", format::kv("  Top composite:", &format!("{:.4}", s.top_composite)));
                    println!("{}", format::kv("  Avg composite:", &format!("{:.4}", s.avg_composite)));
                    println!("{}", format::kv("  Promoted:", &s.promoted.len().to_string()));
                    println!();
                }
            }
        }

        Command::Search { sequence, k } => {
            let store = InMemoryVectorStore::new();
            let embedder = HashEmbedder;
            let results = commands::cmd_search(&sequence, k, &store, &embedder)?;

            if cli.json {
                println!("{}", format::as_json(&results));
            } else if results.is_empty() {
                println!("No results (store is empty)");
            } else {
                let headers = &["ID", "Similarity"];
                let rows: Vec<Vec<String>> = results
                    .iter()
                    .map(|h| vec![h.id.clone(), format!("{:.4}", h.similarity)])
                    .collect();
                print!("{}", format::table(headers, &rows));
            }
        }

        Command::Quantum { cmd } => match cmd {
            QuantumCommand::Vqe { molecule } => {
                let result = commands::cmd_quantum_vqe(&molecule)?;
                if cli.json {
                    println!("{}", format::as_json(&result));
                } else {
                    println!("{}", format::kv("Ground state energy:", &format!("{:.6}", result.ground_state_energy)));
                    println!("{}", format::kv("Converged:", &result.converged.to_string()));
                    println!("{}", format::kv("Iterations:", &result.iterations.to_string()));
                    println!("{}", format::kv("Parameters:", &format!("{:?}", result.optimal_parameters)));
                }
            }
            QuantumCommand::Qaoa { qubo_file } => {
                let result = commands::cmd_quantum_qaoa(&qubo_file)?;
                if cli.json {
                    println!("{}", format::as_json(&result));
                } else {
                    println!("{}", format::kv("Best bitstring:", &format!("{:#b}", result.best_bitstring)));
                    println!("{}", format::kv("Best cost:", &format!("{:.6}", result.best_cost)));
                    println!("{}", format::kv("Converged:", &result.converged.to_string()));
                    println!("{}", format::kv("Iterations:", &result.iterations.to_string()));
                }
            }
        },

        Command::Ledger { cmd } => {
            let ledger = SignedLedger::new();
            match cmd {
                LedgerCommand::Verify => {
                    let result = commands::cmd_ledger_verify(&ledger)?;
                    if cli.json {
                        println!("{}", format::as_json(&result));
                    } else {
                        println!("{}", format::kv("Valid:", &result.valid.to_string()));
                        println!("{}", format::kv("Entry count:", &result.entry_count.to_string()));
                    }
                }
                LedgerCommand::Show { limit } => {
                    let entries = commands::cmd_ledger_show(&ledger, limit)?;
                    if cli.json {
                        println!("{}", format::as_json(&entries));
                    } else if entries.is_empty() {
                        println!("No journal entries");
                    } else {
                        let headers = &["Seq", "Type", "Timestamp", "Hash"];
                        let rows: Vec<Vec<String>> = entries
                            .iter()
                            .map(|e| {
                                vec![
                                    e.sequence_number.to_string(),
                                    e.entry_type.clone(),
                                    e.timestamp.clone(),
                                    e.hash[..16].to_string() + "...",
                                ]
                            })
                            .collect();
                        print!("{}", format::table(headers, &rows));
                    }
                }
            }
        }

        Command::Rvf { cmd } => match cmd {
            RvfCommand::Build { output } => {
                let result = commands::cmd_rvf_build(&output)?;
                if cli.json {
                    println!("{}", format::as_json(&result));
                } else {
                    println!("{}", format::kv("Built:", &result.path));
                    println!("{}", format::kv("File hash:", &result.file_hash));
                }
            }
            RvfCommand::Inspect { path } => {
                let result = commands::cmd_rvf_inspect(&path)?;
                if cli.json {
                    println!("{}", format::as_json(&result));
                } else {
                    println!("{}", format::kv("Name:", &result.name));
                    println!("{}", format::kv("Version:", &result.version));
                    println!("{}", format::kv("Segments:", &result.segment_count.to_string()));
                    println!("{}", format::kv("Capabilities:", &result.capabilities.join(", ")));
                    println!("{}", format::kv("File hash:", &result.file_hash));
                    println!();
                    let headers = &["Segment", "Size"];
                    let rows: Vec<Vec<String>> = result
                        .segments
                        .iter()
                        .map(|s| vec![s.segment_type.clone(), format!("{} bytes", s.size_bytes)])
                        .collect();
                    print!("{}", format::table(headers, &rows));
                }
            }
        },

        Command::Serve { port } => {
            tracing::info!("Starting Protein-Engine HTTP server on port {port}");
            let state = wiring::build_app_state();
            let app = pe_api::router::build_router(state);

            let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{port}")).await?;
            tracing::info!("Listening on http://0.0.0.0:{port}");
            axum::serve(listener, app).await?;
        }
    }

    Ok(())
}
