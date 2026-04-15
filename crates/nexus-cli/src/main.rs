use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::process;

#[derive(Parser)]
#[command(
    name = "nexus-gen",
    version,
    about = "Code generator for declarative IPC topology"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Validate network.toml and schemas
    Validate {
        /// Path to network.toml
        #[arg(short, long, default_value = "network.toml")]
        config: PathBuf,
    },
    /// Generate C headers, implementations, and build files
    Build {
        /// Output format: nix or cmake
        #[arg(short, long)]
        emit: EmitFormat,
        /// Path to network.toml
        #[arg(short, long, default_value = "network.toml")]
        config: PathBuf,
        /// Output directory
        #[arg(short, long, default_value = "nexus-gen-out")]
        output: PathBuf,
    },
    /// Detect breaking changes between two network.toml files
    Diff {
        /// Old network.toml
        old: PathBuf,
        /// New network.toml
        new: PathBuf,
    },
    /// Launch the visual network editor in the browser
    Studio {
        /// Path to network.toml
        #[arg(short, long, default_value = "network.toml")]
        config: PathBuf,
        /// Port to listen on
        #[arg(short, long, default_value_t = 3000)]
        port: u16,
    },
}

#[derive(Clone, clap::ValueEnum)]
enum EmitFormat {
    Nix,
    Cmake,
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Validate { config } => {
            let network = match nexus_core::load(&config) {
                Ok(n) => n,
                Err(e) => {
                    eprintln!("error: {}", e);
                    process::exit(1);
                }
            };

            match nexus_validate::validate(&network) {
                Ok(()) => {
                    let n = network.nodes.len();
                    let c = network.contracts.len();
                    let e = network.edges.len();
                    println!(
                        "Validation passed. {} nodes, {} contracts, {} edges.",
                        n, c, e
                    );
                }
                Err(errors) => {
                    eprintln!("Validation failed:");
                    for err in &errors {
                        eprintln!("  - {}", err);
                    }
                    process::exit(1);
                }
            }
        }

        Commands::Build {
            emit,
            config,
            output,
        } => {
            let network = match nexus_core::load(&config) {
                Ok(n) => n,
                Err(e) => {
                    eprintln!("error: {}", e);
                    process::exit(1);
                }
            };

            if let Err(errors) = nexus_validate::validate(&network) {
                eprintln!("Validation failed:");
                for err in &errors {
                    eprintln!("  - {}", err);
                }
                process::exit(1);
            }

            if let EmitFormat::Cmake = emit {
                eprintln!("cmake output is not yet implemented, using nix");
            }

            let generated = match nexus_codegen::generate(&network) {
                Ok(g) => g,
                Err(e) => {
                    eprintln!("error: {}", e);
                    process::exit(1);
                }
            };

            let n = generated.files.len();

            if let Err(e) = nexus_codegen::write_output(&generated, &output) {
                eprintln!("error: {}", e);
                process::exit(1);
            }

            println!("Generated {} files in {}", n, output.display());
        }

        Commands::Diff { old: _, new: _ } => {
            eprintln!("diff command not yet implemented");
        }

        Commands::Studio { config, port } => {
            tokio::runtime::Runtime::new()
                .unwrap()
                .block_on(nexus_studio::run(config, port))
                .unwrap_or_else(|e| {
                    eprintln!("error: {}", e);
                    process::exit(1);
                });
        }
    }
}
