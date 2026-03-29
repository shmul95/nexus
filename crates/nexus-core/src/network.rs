use std::collections::HashMap;
use std::path::Path;

use crate::config::{ContractConfig, NetworkConfig};
use crate::error::NexusCoreError;
use crate::schema::parse_nxs_file;
use crate::types::{
    Contract, ContractIdx, Edge, EdgeOrigin, Field, FieldType, Network, Node, NodeIdx, Schema,
    SchemaIdx, StructDef, Transport,
};

pub(crate) fn parse_config(toml_str: &str) -> Result<NetworkConfig, NexusCoreError> {
    let config: NetworkConfig = toml::from_str(toml_str)?;
    Ok(config)
}

pub(crate) fn resolve(config: &NetworkConfig, base_dir: &Path) -> Result<Network, NexusCoreError> {
    // Step 1: Build nodes
    let mut nodes: Vec<Node> = Vec::new();
    let mut node_index: HashMap<String, NodeIdx> = HashMap::new();
    for nc in &config.nodes {
        let idx = NodeIdx(nodes.len());
        node_index.insert(nc.name.clone(), idx);
        nodes.push(Node {
            name: nc.name.clone(),
        });
    }

    // Step 2 + 3 + 4 + 5 + 6: Build schemas and contracts
    // Deduplicate schemas referenced by path
    let mut schemas: Vec<Schema> = Vec::new();
    let mut schema_path_index: HashMap<String, SchemaIdx> = HashMap::new();

    let mut contracts: Vec<Contract> = Vec::new();
    let mut contract_index: HashMap<String, ContractIdx> = HashMap::new();

    for cc in &config.contracts {
        let transport = parse_transport(&cc.transport)?;
        let schema_idx = resolve_schema(cc, base_dir, &mut schemas, &mut schema_path_index)?;

        let idx = ContractIdx(contracts.len());
        contract_index.insert(cc.name.clone(), idx);
        contracts.push(Contract {
            name: cc.name.clone(),
            transport,
            schema: schema_idx,
        });
    }

    // Step 7: Build edges
    let mut edges: Vec<Edge> = Vec::new();

    for nc in &config.nodes {
        let from_idx = *node_index
            .get(&nc.name)
            .expect("just inserted, always present");

        for send in &nc.sends {
            let to_name = send.to.as_deref().unwrap_or("");
            let to_idx =
                node_index
                    .get(to_name)
                    .copied()
                    .ok_or_else(|| NexusCoreError::UndefinedNode {
                        node: to_name.to_string(),
                    })?;
            let contract_idx = contract_index.get(&send.contract).copied().ok_or_else(|| {
                NexusCoreError::UndefinedContract {
                    contract: send.contract.clone(),
                }
            })?;
            edges.push(Edge {
                from_node: from_idx,
                to_node: to_idx,
                contract: contract_idx,
                origin: EdgeOrigin::Send,
            });
        }

        for recv in &nc.receives {
            let from_name = recv.from.as_deref().unwrap_or("");
            let from_recv_idx = node_index.get(from_name).copied().ok_or_else(|| {
                NexusCoreError::UndefinedNode {
                    node: from_name.to_string(),
                }
            })?;
            let contract_idx = contract_index.get(&recv.contract).copied().ok_or_else(|| {
                NexusCoreError::UndefinedContract {
                    contract: recv.contract.clone(),
                }
            })?;
            edges.push(Edge {
                from_node: from_recv_idx,
                to_node: from_idx,
                contract: contract_idx,
                origin: EdgeOrigin::Receive,
            });
        }
    }

    Ok(Network {
        nodes,
        contracts,
        schemas,
        edges,
        node_index,
        contract_index,
    })
}

fn resolve_schema(
    cc: &ContractConfig,
    base_dir: &Path,
    schemas: &mut Vec<Schema>,
    schema_path_index: &mut HashMap<String, SchemaIdx>,
) -> Result<SchemaIdx, NexusCoreError> {
    if let Some(schema_path) = &cc.schema {
        // External .nxs file — deduplicate by path string
        if let Some(&idx) = schema_path_index.get(schema_path) {
            return Ok(idx);
        }
        let full_path = base_dir.join(schema_path);
        if !full_path.exists() {
            return Err(NexusCoreError::SchemaNotFound(schema_path.clone()));
        }
        let structs = parse_nxs_file(&full_path)?;
        let schema_name = full_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or(schema_path)
            .to_string();
        let idx = SchemaIdx(schemas.len());
        schemas.push(Schema {
            name: schema_name,
            structs,
        });
        schema_path_index.insert(schema_path.clone(), idx);
        Ok(idx)
    } else if !cc.fields.is_empty() {
        // Inline fields — build a synthetic schema
        let struct_name = to_pascal_case(&cc.name);
        let mut fields = Vec::new();
        for f in &cc.fields {
            let typ = parse_inline_field_type(&f.typ)?;
            fields.push(Field {
                name: f.name.clone(),
                typ,
            });
        }
        let idx = SchemaIdx(schemas.len());
        schemas.push(Schema {
            name: cc.name.clone(),
            structs: vec![StructDef {
                name: struct_name,
                fields,
            }],
        });
        Ok(idx)
    } else {
        Err(NexusCoreError::NoSchema {
            contract: cc.name.clone(),
        })
    }
}

fn parse_transport(s: &str) -> Result<Transport, NexusCoreError> {
    match s {
        "unix_socket" | "unix" => Ok(Transport::UnixSocket),
        "grpc" => Ok(Transport::Grpc),
        "http" => Ok(Transport::Http),
        "iceoryx" => Ok(Transport::Iceoryx),
        "shared_memory" | "shm" => Ok(Transport::SharedMemory),
        "message_queue" | "mq" => Ok(Transport::MessageQueue),
        other => Err(NexusCoreError::UnknownTransport(other.to_string())),
    }
}

fn parse_inline_field_type(s: &str) -> Result<FieldType, NexusCoreError> {
    match s {
        "u8" => Ok(FieldType::U8),
        "u16" => Ok(FieldType::U16),
        "u32" => Ok(FieldType::U32),
        "u64" => Ok(FieldType::U64),
        "i8" => Ok(FieldType::I8),
        "i16" => Ok(FieldType::I16),
        "i32" => Ok(FieldType::I32),
        "i64" => Ok(FieldType::I64),
        "f32" => Ok(FieldType::F32),
        "f64" => Ok(FieldType::F64),
        "bool" => Ok(FieldType::Bool),
        other => Err(NexusCoreError::UnknownFieldType(other.to_string())),
    }
}

/// Convert snake_case to PascalCase: "game_info" -> "GameInfo"
pub(crate) fn to_pascal_case(s: &str) -> String {
    s.split('_')
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_TOML: &str = r#"
[[nodes]]
name = "backend"
receives = [{ contract = "game_info", from = "game_engine" }]
sends    = [{ contract = "display",   to   = "frontend"    }]

[[nodes]]
name = "game_engine"
sends = [{ contract = "game_info", to = "backend" }]

[[nodes]]
name = "frontend"
receives = [{ contract = "display", from = "backend" }]

[[contracts]]
name      = "game_info"
transport = "unix_socket"
fields = [
  { name = "player_id",  type = "u32" },
  { name = "position_x", type = "f32" },
  { name = "position_y", type = "f32" },
  { name = "timestamp",  type = "u64" },
]

[[contracts]]
name      = "display"
transport = "unix_socket"
fields = [
  { name = "frame_id", type = "u32" },
  { name = "width",    type = "u32" },
  { name = "height",   type = "u32" },
]
"#;

    #[test]
    fn test_parse_sample_config() {
        let config = parse_config(SAMPLE_TOML).expect("parse should succeed");
        assert_eq!(config.nodes.len(), 3);
        assert_eq!(config.contracts.len(), 2);
    }

    #[test]
    fn test_resolve_with_inline_fields() {
        let config = parse_config(SAMPLE_TOML).expect("parse should succeed");
        let base_dir = Path::new(".");
        let network = resolve(&config, base_dir).expect("resolve should succeed");

        // Should have one schema per contract (2 inline)
        assert_eq!(network.schemas.len(), 2);

        // Find the game_info contract and check its schema
        let gi_idx = network.contract_index["game_info"];
        let gi_contract = &network.contracts[gi_idx.0];
        let gi_schema = &network.schemas[gi_contract.schema.0];
        assert_eq!(gi_schema.structs.len(), 1);
        assert_eq!(gi_schema.structs[0].name, "GameInfo");
        assert_eq!(gi_schema.structs[0].fields.len(), 4);

        // Find display contract
        let d_idx = network.contract_index["display"];
        let d_contract = &network.contracts[d_idx.0];
        let d_schema = &network.schemas[d_contract.schema.0];
        assert_eq!(d_schema.structs[0].name, "Display");
        assert_eq!(d_schema.structs[0].fields.len(), 3);
    }

    #[test]
    fn test_unknown_transport() {
        let toml_str = r#"
[[nodes]]
name = "a"
sends = [{ contract = "c", to = "b" }]

[[nodes]]
name = "b"
receives = [{ contract = "c", from = "a" }]

[[contracts]]
name = "c"
transport = "foobar"
fields = [{ name = "x", type = "u8" }]
"#;
        let config = parse_config(toml_str).expect("toml parse ok");
        let result = resolve(&config, Path::new("."));
        assert!(matches!(result, Err(NexusCoreError::UnknownTransport(_))));
    }

    #[test]
    fn test_pascal_case_conversion() {
        assert_eq!(to_pascal_case("game_info"), "GameInfo");
        assert_eq!(to_pascal_case("display"), "Display");
        assert_eq!(to_pascal_case("player_state"), "PlayerState");
    }
}
