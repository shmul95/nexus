use minijinja::{context, Environment};
use nexus_core::Network;

use crate::{CodegenError, GeneratedFile};

// ── templates (embedded as const &str) ────────────────────────────────────────

const NODE_NIX_TMPL: &str = r#"{ pkgs ? import <nixpkgs> {} }:

pkgs.stdenv.mkDerivation {
  pname = "nexus-{{ node_name }}";
  version = "0.1.0";

  src = ../.;

  buildPhase = ''
    gcc -shared -fPIC \
      -I include \
      {% for contract_name in contract_names %}src/nexus_{{ contract_name }}.c {% endfor %}\
      -o libnexus-{{ node_name }}.so
  '';

  installPhase = ''
    mkdir -p $out/lib $out/include $out/lib/pkgconfig
    cp libnexus-{{ node_name }}.so $out/lib/
    {% for contract_name in contract_names %}cp include/nexus_{{ contract_name }}.h $out/include/
    {% endfor %}cp include/nexus_{{ node_name }}.h $out/include/

    cat > $out/lib/pkgconfig/nexus-{{ node_name }}.pc << PKGEOF
Name: nexus-{{ node_name }}
Version: 0.1.0
Libs: -L$out/lib -lnexus-{{ node_name }}
Cflags: -I$out/include
PKGEOF
  '';
}
"#;

const NEXUS_NIX_TMPL: &str = r#"{ pkgs ? import <nixpkgs> {} }:

{
  {% for node_name in node_names %}{{ node_name }} = import ./nix/{{ node_name }}.nix { inherit pkgs; };
  {% endfor %}
}
"#;

// ── public API ────────────────────────────────────────────────────────────────

/// Generate all Nix derivation files for a network:
/// - One `nix/<node>.nix` per node
/// - A top-level `nexus.nix`
pub fn generate_nix(network: &Network) -> Result<Vec<GeneratedFile>, CodegenError> {
    let mut files = Vec::new();

    let mut env = Environment::new();
    env.add_template("node_nix", NODE_NIX_TMPL)?;
    env.add_template("nexus_nix", NEXUS_NIX_TMPL)?;

    // Per-node derivations
    for (node_idx, node) in network.nodes.iter().enumerate() {
        // Collect unique contract names for this node (sorted for determinism)
        let mut contract_indices: Vec<usize> = network
            .edges
            .iter()
            .filter(|e| e.from_node.0 == node_idx || e.to_node.0 == node_idx)
            .map(|e| e.contract.0)
            .collect();
        contract_indices.sort();
        contract_indices.dedup();

        let contract_names: Vec<&str> = contract_indices
            .iter()
            .map(|&idx| network.contracts[idx].name.as_str())
            .collect();

        let tmpl = env.get_template("node_nix")?;
        let rendered = tmpl.render(context! {
            node_name => node.name,
            contract_names => contract_names,
        })?;

        files.push(GeneratedFile {
            path: format!("nix/{}.nix", node.name),
            content: rendered,
        });
    }

    // Top-level nexus.nix
    let node_names: Vec<&str> = network.nodes.iter().map(|n| n.name.as_str()).collect();
    let tmpl = env.get_template("nexus_nix")?;
    let rendered = tmpl.render(context! {
        node_names => node_names,
    })?;

    files.push(GeneratedFile {
        path: "nexus.nix".to_string(),
        content: rendered,
    });

    Ok(files)
}

// ── tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use nexus_core::{
        Contract, ContractIdx, Edge, EdgeOrigin, Network, Node, NodeIdx, Schema, SchemaIdx,
        StructDef, Transport,
    };

    use super::*;

    /// Build a 3-node test network:
    ///   backend  --game_info-->  game_engine
    ///   frontend --display-->    game_engine
    fn three_node_network() -> Network {
        let nodes = vec![
            Node {
                name: "backend".to_string(),
            },
            Node {
                name: "game_engine".to_string(),
            },
            Node {
                name: "frontend".to_string(),
            },
        ];
        let contracts = vec![
            Contract {
                name: "game_info".to_string(),
                transport: Transport::UnixSocket,
                schema: SchemaIdx(0),
            },
            Contract {
                name: "display".to_string(),
                transport: Transport::UnixSocket,
                schema: SchemaIdx(1),
            },
        ];
        let schemas = vec![
            Schema {
                name: "game_info".to_string(),
                structs: vec![StructDef {
                    name: "game_info".to_string(),
                    fields: vec![],
                }],
            },
            Schema {
                name: "display".to_string(),
                structs: vec![StructDef {
                    name: "display".to_string(),
                    fields: vec![],
                }],
            },
        ];
        let edges = vec![
            Edge {
                from_node: NodeIdx(0), // backend
                to_node: NodeIdx(1),   // game_engine
                contract: ContractIdx(0),
                origin: EdgeOrigin::Send,
            },
            Edge {
                from_node: NodeIdx(2), // frontend
                to_node: NodeIdx(1),   // game_engine
                contract: ContractIdx(1),
                origin: EdgeOrigin::Send,
            },
        ];

        let mut node_index = HashMap::new();
        node_index.insert("backend".to_string(), NodeIdx(0));
        node_index.insert("game_engine".to_string(), NodeIdx(1));
        node_index.insert("frontend".to_string(), NodeIdx(2));

        let mut contract_index = HashMap::new();
        contract_index.insert("game_info".to_string(), ContractIdx(0));
        contract_index.insert("display".to_string(), ContractIdx(1));

        Network {
            nodes,
            contracts,
            schemas,
            edges,
            node_index,
            contract_index,
        }
    }

    #[test]
    fn test_nix_generation() {
        let network = three_node_network();
        let files = generate_nix(&network).unwrap();

        // Expect 4 files: 3 node .nix + nexus.nix
        assert_eq!(files.len(), 4, "expected 4 nix files");

        // Collect into a map for easy lookup
        let file_map: HashMap<&str, &str> = files
            .iter()
            .map(|f| (f.path.as_str(), f.content.as_str()))
            .collect();

        // Per-node files must exist
        assert!(
            file_map.contains_key("nix/backend.nix"),
            "missing backend.nix"
        );
        assert!(
            file_map.contains_key("nix/game_engine.nix"),
            "missing game_engine.nix"
        );
        assert!(
            file_map.contains_key("nix/frontend.nix"),
            "missing frontend.nix"
        );
        assert!(file_map.contains_key("nexus.nix"), "missing nexus.nix");

        // backend.nix references game_info contract
        let backend_nix = file_map["nix/backend.nix"];
        assert!(
            backend_nix.contains("nexus-backend"),
            "backend pname missing"
        );
        assert!(
            backend_nix.contains("nexus_game_info.c"),
            "backend: game_info.c missing"
        );
        assert!(
            backend_nix.contains("nexus_game_info.h"),
            "backend: game_info.h missing"
        );

        // frontend.nix references display contract
        let frontend_nix = file_map["nix/frontend.nix"];
        assert!(
            frontend_nix.contains("nexus-frontend"),
            "frontend pname missing"
        );
        assert!(
            frontend_nix.contains("nexus_display.c"),
            "frontend: display.c missing"
        );
        assert!(
            frontend_nix.contains("nexus_display.h"),
            "frontend: display.h missing"
        );

        // game_engine.nix references both contracts (both edges point to it)
        let engine_nix = file_map["nix/game_engine.nix"];
        assert!(
            engine_nix.contains("nexus-game_engine"),
            "engine pname missing"
        );
        assert!(
            engine_nix.contains("nexus_game_info.c"),
            "engine: game_info.c missing"
        );
        assert!(
            engine_nix.contains("nexus_display.c"),
            "engine: display.c missing"
        );

        // nexus.nix imports all nodes
        let nexus_nix = file_map["nexus.nix"];
        assert!(nexus_nix.contains("backend"), "nexus.nix: backend missing");
        assert!(
            nexus_nix.contains("game_engine"),
            "nexus.nix: game_engine missing"
        );
        assert!(
            nexus_nix.contains("frontend"),
            "nexus.nix: frontend missing"
        );
        assert!(
            nexus_nix.contains("import ./nix/backend.nix"),
            "nexus.nix: import backend missing"
        );
    }
}
