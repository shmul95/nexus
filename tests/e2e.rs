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

// ── mixed transport example ──────────────────────────────────────────────────

#[test]
fn e2e_mixed_load_validate_generate() {
    let config = workspace_root().join("examples/mixed/network.toml");

    // 1. Load
    let network = nexus_core::load(&config).expect("load should succeed for mixed/network.toml");

    assert_eq!(network.nodes.len(), 4, "mixed: expected 4 nodes");
    assert_eq!(network.contracts.len(), 3, "mixed: expected 3 contracts");
    assert_eq!(network.edges.len(), 6, "mixed: expected 6 edges");

    // 2. Validate
    nexus_validate::validate(&network).expect("mixed network should pass all validation rules");

    // 3. Generate
    let output = nexus_codegen::generate(&network).expect("codegen should succeed for mixed network");

    let paths: HashSet<&str> = output.files.iter().map(|f| f.path.as_str()).collect();

    // Per-contract headers
    assert!(paths.contains("include/nexus_readings.h"), "missing nexus_readings.h");
    assert!(paths.contains("include/nexus_summary.h"), "missing nexus_summary.h");
    assert!(paths.contains("include/nexus_dashboard.h"), "missing nexus_dashboard.h");

    // Per-contract C implementations
    assert!(paths.contains("src/nexus_readings.c"), "missing nexus_readings.c (iceoryx)");
    assert!(paths.contains("src/nexus_summary.c"), "missing nexus_summary.c (grpc)");
    assert!(paths.contains("src/nexus_dashboard.c"), "missing nexus_dashboard.c (http)");

    // Per-node umbrella headers
    assert!(paths.contains("include/nexus_sensor.h"), "missing nexus_sensor.h");
    assert!(paths.contains("include/nexus_aggregator.h"), "missing nexus_aggregator.h");
    assert!(paths.contains("include/nexus_api_server.h"), "missing nexus_api_server.h");
    assert!(paths.contains("include/nexus_frontend.h"), "missing nexus_frontend.h");

    // Nix derivation files
    assert!(paths.contains("nix/sensor.nix"), "missing nix/sensor.nix");
    assert!(paths.contains("nix/aggregator.nix"), "missing nix/aggregator.nix");
    assert!(paths.contains("nix/api_server.nix"), "missing nix/api_server.nix");
    assert!(paths.contains("nix/frontend.nix"), "missing nix/frontend.nix");
    assert!(paths.contains("nexus.nix"), "missing nexus.nix");

    // TypeScript file (only for HTTP transport)
    assert!(paths.contains("ts/nexus_dashboard.ts"), "missing ts/nexus_dashboard.ts (HTTP contract)");
    // iceoryx and grpc contracts should NOT have TypeScript
    assert!(!paths.contains("ts/nexus_readings.ts"), "unexpected TS for iceoryx contract");
    assert!(!paths.contains("ts/nexus_summary.ts"), "unexpected TS for grpc contract");

    // Verify transport-specific content in C implementations
    let readings_c = output.files.iter().find(|f| f.path == "src/nexus_readings.c").unwrap();
    assert!(readings_c.content.contains("shm_open"), "iceoryx impl should use shm_open");

    let summary_c = output.files.iter().find(|f| f.path == "src/nexus_summary.c").unwrap();
    assert!(summary_c.content.contains("grpc_channel"), "grpc impl should use grpc_channel");

    let dashboard_c = output.files.iter().find(|f| f.path == "src/nexus_dashboard.c").unwrap();
    assert!(dashboard_c.content.contains("curl_easy_init"), "http impl should use curl");

    // Verify nix derivation has correct transport deps
    let api_server_nix = output.files.iter().find(|f| f.path == "nix/api_server.nix").unwrap();
    assert!(api_server_nix.content.contains("grpc"), "api_server.nix should have grpc dep");
    assert!(api_server_nix.content.contains("curl"), "api_server.nix should have curl dep");

    let sensor_nix = output.files.iter().find(|f| f.path == "nix/sensor.nix").unwrap();
    assert!(sensor_nix.content.contains("-lrt"), "sensor.nix should have -lrt for iceoryx");
}
