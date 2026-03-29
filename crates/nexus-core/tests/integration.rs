use nexus_core::{load, EdgeOrigin};

#[test]
fn test_load_sample() {
    // The test binary runs from the workspace root, but the path to the sample
    // is relative to the workspace root.
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = manifest_dir
        .parent() // crates/
        .and_then(|p| p.parent()) // workspace root
        .expect("workspace root exists");

    let network_toml = workspace_root.join("examples/sample/network.toml");
    let network = load(&network_toml).expect("load should succeed");

    // 3 nodes
    assert_eq!(network.nodes.len(), 3, "expected 3 nodes");

    // Check node names and indices
    assert!(network.node_index.contains_key("backend"));
    assert!(network.node_index.contains_key("game_engine"));
    assert!(network.node_index.contains_key("frontend"));

    // 2 contracts
    assert_eq!(network.contracts.len(), 2, "expected 2 contracts");
    assert!(network.contract_index.contains_key("game_info"));
    assert!(network.contract_index.contains_key("display"));

    // 4 edges (2 Send from game_engine + backend, 2 Receive from backend + frontend)
    assert_eq!(network.edges.len(), 4, "expected 4 edges");

    let send_edges: Vec<_> = network
        .edges
        .iter()
        .filter(|e| e.origin == EdgeOrigin::Send)
        .collect();
    let recv_edges: Vec<_> = network
        .edges
        .iter()
        .filter(|e| e.origin == EdgeOrigin::Receive)
        .collect();
    assert_eq!(send_edges.len(), 2, "expected 2 Send edges");
    assert_eq!(recv_edges.len(), 2, "expected 2 Receive edges");

    // game_info schema has 4 fields (from .nxs file)
    let gi_idx = network.contract_index["game_info"];
    let gi_contract = &network.contracts[gi_idx.0];
    let gi_schema = &network.schemas[gi_contract.schema.0];
    assert_eq!(gi_schema.structs.len(), 1);
    let gi_struct = &gi_schema.structs[0];
    assert_eq!(gi_struct.name, "GameInfo");
    assert_eq!(
        gi_struct.fields.len(),
        4,
        "game_info schema should have 4 fields"
    );

    let field_names: Vec<&str> = gi_struct.fields.iter().map(|f| f.name.as_str()).collect();
    assert!(field_names.contains(&"player_id"));
    assert!(field_names.contains(&"position_x"));
    assert!(field_names.contains(&"position_y"));
    assert!(field_names.contains(&"timestamp"));

    // display schema has 3 fields (inline)
    let d_idx = network.contract_index["display"];
    let d_contract = &network.contracts[d_idx.0];
    let d_schema = &network.schemas[d_contract.schema.0];
    assert_eq!(d_schema.structs.len(), 1);
    let d_struct = &d_schema.structs[0];
    assert_eq!(d_struct.name, "Display");
    assert_eq!(
        d_struct.fields.len(),
        3,
        "display schema should have 3 fields"
    );

    let field_names: Vec<&str> = d_struct.fields.iter().map(|f| f.name.as_str()).collect();
    assert!(field_names.contains(&"frame_id"));
    assert!(field_names.contains(&"width"));
    assert!(field_names.contains(&"height"));

    // Verify index correctness
    let be_idx = network.node_index["backend"];
    let ge_idx = network.node_index["game_engine"];
    let fe_idx = network.node_index["frontend"];
    assert_eq!(network.nodes[be_idx.0].name, "backend");
    assert_eq!(network.nodes[ge_idx.0].name, "game_engine");
    assert_eq!(network.nodes[fe_idx.0].name, "frontend");

    let gi_contract_idx = network.contract_index["game_info"];
    let d_contract_idx = network.contract_index["display"];
    assert_eq!(network.contracts[gi_contract_idx.0].name, "game_info");
    assert_eq!(network.contracts[d_contract_idx.0].name, "display");
}
