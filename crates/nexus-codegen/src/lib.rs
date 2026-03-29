// nexus-codegen: C header/impl, TypeScript client, and Nix derivation generation via minijinja templates

pub mod error;
pub mod header;
pub mod impl_grpc;
pub mod impl_http;
pub mod impl_iceoryx;
pub mod impl_unix_socket;
pub mod nix;
pub mod typescript;

use std::path::Path;

use nexus_core::{Network, Transport};

pub use error::CodegenError;

// ── output types ──────────────────────────────────────────────────────────────

pub struct GeneratedOutput {
    pub files: Vec<GeneratedFile>,
}

pub struct GeneratedFile {
    /// Relative path within the output directory.
    pub path: String,
    pub content: String,
}

// ── public API ────────────────────────────────────────────────────────────────

/// Generate all output files for a validated network.
pub fn generate(network: &Network) -> Result<GeneratedOutput, CodegenError> {
    let mut files = Vec::new();

    // Per-contract C headers and implementations
    for contract in &network.contracts {
        let schema = &network.schemas[contract.schema.0];

        files.push(header::generate_header(contract, schema)?);

        match &contract.transport {
            Transport::UnixSocket => {
                files.push(impl_unix_socket::generate_impl(contract, schema)?);
            }
            Transport::Grpc => {
                files.push(impl_grpc::generate_impl(contract, schema)?);
            }
            Transport::Http => {
                files.push(impl_http::generate_impl(contract, schema)?);
            }
            Transport::Iceoryx => {
                files.push(impl_iceoryx::generate_impl(contract, schema)?);
            }
            other => {
                return Err(CodegenError::UnsupportedTransport(format!("{:?}", other)));
            }
        }
    }

    // Per-node umbrella headers
    for (node_idx, node) in network.nodes.iter().enumerate() {
        // Collect unique contract indices that involve this node, sorted for
        // deterministic output.
        let mut contract_indices: Vec<usize> = network
            .edges
            .iter()
            .filter(|e| e.from_node.0 == node_idx || e.to_node.0 == node_idx)
            .map(|e| e.contract.0)
            .collect();
        contract_indices.sort();
        contract_indices.dedup();

        let node_contracts: Vec<&nexus_core::Contract> = contract_indices
            .iter()
            .map(|&idx| &network.contracts[idx])
            .collect();

        files.push(header::generate_umbrella(node, &node_contracts)?);
    }

    // Nix derivation files
    files.extend(nix::generate_nix(network)?);

    // TypeScript clients (only for HTTP transport contracts)
    for contract in &network.contracts {
        if contract.transport == Transport::Http {
            let schema = &network.schemas[contract.schema.0];
            files.push(typescript::generate_typescript(contract, schema)?);
        }
    }

    Ok(GeneratedOutput { files })
}

/// Write all generated files to `output_dir`, creating subdirectories as needed.
pub fn write_output(output: &GeneratedOutput, output_dir: &Path) -> Result<(), std::io::Error> {
    for file in &output.files {
        let path = output_dir.join(&file.path);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&path, &file.content)?;
    }
    Ok(())
}
