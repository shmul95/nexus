/// Workspace-level end-to-end integration tests.
///
/// Each test exercises the full pipeline:
///   load (nexus-core) -> validate (nexus-validate) -> generate (nexus-codegen)
/// and asserts that the expected output files are produced.
use std::collections::HashSet;
use std::path::Path;

fn workspace_root() -> &'static Path {
    // CARGO_MANIFEST_DIR for this package is the workspace root itself.
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

// ── sample example ────────────────────────────────────────────────────────────

#[test]
fn e2e_sample_load_validate_generate() {
    let config = workspace_root().join("examples/sample/network.toml");

    // 1. Load
    let network = nexus_core::load(&config).expect("load should succeed for sample/network.toml");

    assert_eq!(network.nodes.len(), 3, "sample: expected 3 nodes");
    assert_eq!(network.contracts.len(), 2, "sample: expected 2 contracts");
    assert_eq!(network.edges.len(), 4, "sample: expected 4 edges");

    // 2. Validate
    nexus_validate::validate(&network).expect("sample network should pass all validation rules");

    // 3. Generate
    let output =
        nexus_codegen::generate(&network).expect("codegen should succeed for sample network");

    let paths: HashSet<&str> = output.files.iter().map(|f| f.path.as_str()).collect();

    // Per-contract headers and implementations
    assert!(
        paths.contains("include/nexus_game_info.h"),
        "missing nexus_game_info.h"
    );
    assert!(
        paths.contains("include/nexus_display.h"),
        "missing nexus_display.h"
    );
    assert!(
        paths.contains("src/nexus_game_info.c"),
        "missing nexus_game_info.c"
    );
    assert!(
        paths.contains("src/nexus_display.c"),
        "missing nexus_display.c"
    );

    // Per-node umbrella headers
    assert!(
        paths.contains("include/nexus_backend.h"),
        "missing nexus_backend.h"
    );
    assert!(
        paths.contains("include/nexus_game_engine.h"),
        "missing nexus_game_engine.h"
    );
    assert!(
        paths.contains("include/nexus_frontend.h"),
        "missing nexus_frontend.h"
    );

    // Nix derivation files
    assert!(paths.contains("nix/backend.nix"), "missing nix/backend.nix");
    assert!(
        paths.contains("nix/game_engine.nix"),
        "missing nix/game_engine.nix"
    );
    assert!(
        paths.contains("nix/frontend.nix"),
        "missing nix/frontend.nix"
    );
    assert!(paths.contains("nexus.nix"), "missing nexus.nix");

    // Sanity-check a generated header contains expected C declarations
    let game_info_h = output
        .files
        .iter()
        .find(|f| f.path == "include/nexus_game_info.h")
        .expect("nexus_game_info.h must exist");
    assert!(
        game_info_h.content.contains("NexusGameInfo"),
        "header missing struct NexusGameInfo"
    );
    assert!(
        game_info_h.content.contains("nexus_send_game_info"),
        "header missing send function"
    );
    assert!(
        game_info_h.content.contains("nexus_recv_game_info"),
        "header missing recv function"
    );
}

// ── minimal example ───────────────────────────────────────────────────────────

#[test]
fn e2e_minimal_load_validate_generate() {
    let config = workspace_root().join("examples/minimal/network.toml");

    // 1. Load
    let network = nexus_core::load(&config).expect("load should succeed for minimal/network.toml");

    assert_eq!(network.nodes.len(), 2, "minimal: expected 2 nodes");
    assert_eq!(network.contracts.len(), 1, "minimal: expected 1 contract");
    assert_eq!(network.edges.len(), 2, "minimal: expected 2 edges");

    assert!(
        network.node_index.contains_key("producer"),
        "missing producer node"
    );
    assert!(
        network.node_index.contains_key("consumer"),
        "missing consumer node"
    );
    assert!(
        network.contract_index.contains_key("data"),
        "missing data contract"
    );

    // 2. Validate
    nexus_validate::validate(&network).expect("minimal network should pass all validation rules");

    // 3. Generate
    let output =
        nexus_codegen::generate(&network).expect("codegen should succeed for minimal network");

    let paths: HashSet<&str> = output.files.iter().map(|f| f.path.as_str()).collect();

    // Per-contract header and implementation
    assert!(
        paths.contains("include/nexus_data.h"),
        "missing nexus_data.h"
    );
    assert!(paths.contains("src/nexus_data.c"), "missing nexus_data.c");

    // Per-node umbrella headers
    assert!(
        paths.contains("include/nexus_producer.h"),
        "missing nexus_producer.h"
    );
    assert!(
        paths.contains("include/nexus_consumer.h"),
        "missing nexus_consumer.h"
    );

    // Nix derivation files
    assert!(
        paths.contains("nix/producer.nix"),
        "missing nix/producer.nix"
    );
    assert!(
        paths.contains("nix/consumer.nix"),
        "missing nix/consumer.nix"
    );
    assert!(paths.contains("nexus.nix"), "missing nexus.nix");

    // Total file count
    assert_eq!(output.files.len(), 7, "minimal: expected 7 generated files");

    // Sanity-check the data header
    let data_h = output
        .files
        .iter()
        .find(|f| f.path == "include/nexus_data.h")
        .expect("nexus_data.h must exist");
    assert!(
        data_h.content.contains("NexusData"),
        "header missing struct NexusData"
    );
    assert!(
        data_h.content.contains("nexus_send_data"),
        "header missing send function"
    );
    assert!(
        data_h.content.contains("nexus_recv_data"),
        "header missing recv function"
    );
}
