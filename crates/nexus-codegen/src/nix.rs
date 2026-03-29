use minijinja::{context, Environment};
use nexus_core::{Network, Transport};

use crate::{CodegenError, GeneratedFile};

// ── templates (embedded as const &str) ────────────────────────────────────────

const NODE_NIX_TMPL: &str = r#"{ pkgs ? import <nixpkgs> {}, src ? ./.. }:

pkgs.stdenv.mkDerivation {
  pname = "nexus-{{ node_name }}";
  version = "0.1.0";

  inherit src;
{% if has_grpc or has_http or has_http_server %}
  nativeBuildInputs = [ pkgs.pkg-config ];
{% endif %}
{% if has_grpc or has_http or has_http_server %}
  buildInputs = [
    {% if has_grpc %}pkgs.grpc{% endif %}
    {% if has_http %}pkgs.curl{% endif %}
    {% if has_http_server %}pkgs.libmicrohttpd{% endif %}
  ];
{% endif %}

  buildPhase = ''
    gcc -shared -fPIC \
      -I include \
{% if has_grpc %}      $(pkg-config --cflags grpc) \
{% endif %}{% if has_http %}      $(pkg-config --cflags libcurl) \
{% endif %}{% if has_http_server %}      $(pkg-config --cflags libmicrohttpd) \
{% endif %}      {% for c in server_contracts %}src/nexus_{{ c }}_server.c {% endfor %}\
      {% for c in client_contracts %}src/nexus_{{ c }}.c {% endfor %}\
{% if has_grpc_server %}      src/nexus_grpc_runtime.c \
{% endif %}{% if has_http_server %}      src/nexus_http_runtime.c \
{% endif %}{% if has_grpc %}      $(pkg-config --libs grpc) \
{% endif %}{% if has_http %}      $(pkg-config --libs libcurl) \
{% endif %}{% if has_http_server %}      $(pkg-config --libs libmicrohttpd) \
{% endif %}{% if has_iceoryx %}      -lrt \
{% endif %}{% if has_grpc_server or has_http_server %}      -lpthread \
{% endif %}      -o libnexus-{{ node_name }}.so
  '';

  installPhase = ''
    mkdir -p $out/lib $out/include $out/lib/pkgconfig
    cp libnexus-{{ node_name }}.so $out/lib/
    {% for contract_name in all_contract_names %}cp include/nexus_{{ contract_name }}.h $out/include/
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

const NEXUS_NIX_TMPL: &str = r#"{ pkgs ? import <nixpkgs> {}, src ? null }:

{
  {% for node_name in node_names %}{{ node_name }} = import ./nix/{{ node_name }}.nix { inherit pkgs src; };
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
        // Collect edges for this node, split into sender (from) and receiver (to) roles
        let mut server_contracts: Vec<&str> = Vec::new(); // sender on gRPC/HTTP → _server.c
        let mut client_contracts: Vec<&str> = Vec::new(); // receiver on gRPC/HTTP → .c, or unix_socket/iceoryx → .c

        let mut has_grpc = false;
        let mut has_http = false;
        let mut has_iceoryx = false;
        let mut has_grpc_server = false;
        let mut has_http_server = false;

        let mut seen: std::collections::HashSet<usize> = std::collections::HashSet::new();

        for edge in &network.edges {
            let contract_idx = edge.contract.0;
            if !seen.insert(contract_idx) {
                continue; // deduplicate
            }

            let is_sender = edge.from_node.0 == node_idx;
            let is_receiver = edge.to_node.0 == node_idx;
            if !is_sender && !is_receiver {
                continue;
            }

            let contract = &network.contracts[contract_idx];
            let name = contract.name.as_str();

            match (&contract.transport, is_sender) {
                (Transport::Grpc, true) => {
                    server_contracts.push(name);
                    has_grpc = true;
                    has_grpc_server = true;
                }
                (Transport::Grpc, false) => {
                    client_contracts.push(name);
                    has_grpc = true;
                }
                (Transport::Http, true) => {
                    server_contracts.push(name);
                    has_http_server = true;
                }
                (Transport::Http, false) => {
                    client_contracts.push(name);
                    has_http = true;
                }
                (Transport::Iceoryx, _) => {
                    client_contracts.push(name); // iceoryx impl is symmetric, use regular .c
                    has_iceoryx = true;
                }
                (Transport::UnixSocket, _) => {
                    client_contracts.push(name); // unix socket impl is symmetric
                }
                _ => {}
            }
        }

        server_contracts.sort();
        client_contracts.sort();

        // All contract names for header installation
        let mut all_contract_names: Vec<&str> = Vec::new();
        all_contract_names.extend(&server_contracts);
        all_contract_names.extend(&client_contracts);
        all_contract_names.sort();
        all_contract_names.dedup();

        let tmpl = env.get_template("node_nix")?;
        let rendered = tmpl.render(context! {
            node_name => node.name,
            server_contracts => server_contracts,
            client_contracts => client_contracts,
            all_contract_names => all_contract_names,
            has_grpc => has_grpc,
            has_http => has_http,
            has_iceoryx => has_iceoryx,
            has_grpc_server => has_grpc_server,
            has_http_server => has_http_server,
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

    /// Build a 2-node network with mixed transports:
    ///   sender --grpc_data--> receiver   (gRPC)
    ///   sender --http_data--> receiver   (HTTP)
    ///   sender --shm_data-->  receiver   (Iceoryx)
    fn mixed_transport_network() -> Network {
        let nodes = vec![
            Node {
                name: "sender".to_string(),
            },
            Node {
                name: "receiver".to_string(),
            },
        ];
        let contracts = vec![
            Contract {
                name: "grpc_data".to_string(),
                transport: Transport::Grpc,
                schema: SchemaIdx(0),
            },
            Contract {
                name: "http_data".to_string(),
                transport: Transport::Http,
                schema: SchemaIdx(0),
            },
            Contract {
                name: "shm_data".to_string(),
                transport: Transport::Iceoryx,
                schema: SchemaIdx(0),
            },
        ];
        let schemas = vec![Schema {
            name: "payload".to_string(),
            structs: vec![StructDef {
                name: "payload".to_string(),
                fields: vec![],
            }],
        }];
        let edges = vec![
            Edge {
                from_node: NodeIdx(0),
                to_node: NodeIdx(1),
                contract: ContractIdx(0),
                origin: EdgeOrigin::Send,
            },
            Edge {
                from_node: NodeIdx(0),
                to_node: NodeIdx(1),
                contract: ContractIdx(1),
                origin: EdgeOrigin::Send,
            },
            Edge {
                from_node: NodeIdx(0),
                to_node: NodeIdx(1),
                contract: ContractIdx(2),
                origin: EdgeOrigin::Send,
            },
        ];

        let mut node_index = HashMap::new();
        node_index.insert("sender".to_string(), NodeIdx(0));
        node_index.insert("receiver".to_string(), NodeIdx(1));

        let mut contract_index = HashMap::new();
        contract_index.insert("grpc_data".to_string(), ContractIdx(0));
        contract_index.insert("http_data".to_string(), ContractIdx(1));
        contract_index.insert("shm_data".to_string(), ContractIdx(2));

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

        // UnixSocket-only nodes must NOT have transport deps
        assert!(
            !backend_nix.contains("pkg-config"),
            "backend: unexpected pkg-config (unix socket only)"
        );
        assert!(
            !backend_nix.contains("pkgs.grpc"),
            "backend: unexpected pkgs.grpc"
        );
        assert!(
            !backend_nix.contains("pkgs.curl"),
            "backend: unexpected pkgs.curl"
        );
        assert!(
            !backend_nix.contains("-lrt"),
            "backend: unexpected -lrt"
        );

        // All nodes use `inherit src;` instead of `src = ../.;`
        assert!(
            backend_nix.contains("inherit src;"),
            "backend: missing inherit src"
        );
        assert!(
            frontend_nix.contains("inherit src;"),
            "frontend: missing inherit src"
        );
        assert!(
            engine_nix.contains("inherit src;"),
            "engine: missing inherit src"
        );
    }

    #[test]
    fn test_nix_generation_mixed_transports() {
        let network = mixed_transport_network();
        let files = generate_nix(&network).unwrap();

        // Expect 3 files: 2 node .nix + nexus.nix
        assert_eq!(files.len(), 3, "expected 3 nix files");

        let file_map: HashMap<&str, &str> = files
            .iter()
            .map(|f| (f.path.as_str(), f.content.as_str()))
            .collect();

        let sender_nix = *file_map.get("nix/sender.nix").expect("missing sender.nix");
        let receiver_nix = *file_map.get("nix/receiver.nix").expect("missing receiver.nix");

        // Sender node: gRPC and HTTP contracts get _server.c files
        assert!(
            sender_nix.contains("nexus_grpc_data_server.c"),
            "sender: missing grpc_data_server.c"
        );
        assert!(
            sender_nix.contains("nexus_http_data_server.c"),
            "sender: missing http_data_server.c"
        );
        assert!(
            sender_nix.contains("nexus_shm_data.c"),
            "sender: missing shm_data.c (iceoryx uses regular impl)"
        );
        // Sender should have server runtimes
        assert!(
            sender_nix.contains("nexus_grpc_runtime.c"),
            "sender: missing grpc_runtime.c"
        );
        assert!(
            sender_nix.contains("nexus_http_runtime.c"),
            "sender: missing http_runtime.c"
        );
        // Sender server deps
        assert!(sender_nix.contains("pkgs.grpc"), "sender: missing pkgs.grpc");
        assert!(
            sender_nix.contains("pkgs.libmicrohttpd"),
            "sender: missing pkgs.libmicrohttpd"
        );
        assert!(sender_nix.contains("-lpthread"), "sender: missing -lpthread");
        assert!(sender_nix.contains("-lrt"), "sender: missing -lrt");

        // Receiver node: gRPC and HTTP contracts get regular .c (client) files
        assert!(
            receiver_nix.contains("nexus_grpc_data.c"),
            "receiver: missing grpc_data.c (client)"
        );
        assert!(
            receiver_nix.contains("nexus_http_data.c"),
            "receiver: missing http_data.c (client)"
        );
        assert!(
            receiver_nix.contains("nexus_shm_data.c"),
            "receiver: missing shm_data.c"
        );
        // Receiver should NOT have server runtimes
        assert!(
            !receiver_nix.contains("nexus_grpc_runtime.c"),
            "receiver: unexpected grpc_runtime.c"
        );
        assert!(
            !receiver_nix.contains("nexus_http_runtime.c"),
            "receiver: unexpected http_runtime.c"
        );
        // Receiver client deps
        assert!(
            receiver_nix.contains("pkgs.grpc"),
            "receiver: missing pkgs.grpc"
        );
        assert!(
            receiver_nix.contains("pkgs.curl"),
            "receiver: missing pkgs.curl"
        );
        // Receiver should NOT have server-only deps
        assert!(
            !receiver_nix.contains("pkgs.libmicrohttpd"),
            "receiver: unexpected pkgs.libmicrohttpd"
        );
        assert!(
            !receiver_nix.contains("-lpthread"),
            "receiver: unexpected -lpthread"
        );
    }
}
