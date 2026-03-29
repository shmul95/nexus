mod config;
pub mod error;
pub mod network;
pub mod schema;
pub mod types;

pub use error::NexusCoreError;
pub use types::*;

use std::path::Path;

/// Load and resolve a network.toml file into a fully resolved Network.
pub fn load(path: &Path) -> Result<Network, NexusCoreError> {
    let content = std::fs::read_to_string(path).map_err(|e| NexusCoreError::FileRead {
        path: path.display().to_string(),
        source: e,
    })?;
    let config = network::parse_config(&content)?;
    let base_dir = path.parent().unwrap_or(Path::new("."));
    network::resolve(&config, base_dir)
}
