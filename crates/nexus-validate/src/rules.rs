use std::collections::HashSet;

use nexus_core::{EdgeOrigin, FieldType, Network, Transport};

use crate::error::ValidationError;

/// For each Send edge, verify a matching Receive edge exists with the same
/// (from_node, to_node, contract) triple.
pub fn check_unmatched_sends(network: &Network, errors: &mut Vec<ValidationError>) {
    let receive_set: HashSet<(usize, usize, usize)> = network
        .edges
        .iter()
        .filter(|e| e.origin == EdgeOrigin::Receive)
        .map(|e| (e.from_node.0, e.to_node.0, e.contract.0))
        .collect();

    for edge in network
        .edges
        .iter()
        .filter(|e| e.origin == EdgeOrigin::Send)
    {
        let key = (edge.from_node.0, edge.to_node.0, edge.contract.0);
        if !receive_set.contains(&key) {
            errors.push(ValidationError::UnmatchedSend {
                from_node: network.nodes[edge.from_node.0].name.clone(),
                to_node: network.nodes[edge.to_node.0].name.clone(),
                contract: network.contracts[edge.contract.0].name.clone(),
            });
        }
    }
}

/// For each Receive edge, verify a matching Send edge exists with the same
/// (from_node, to_node, contract) triple.
pub fn check_unmatched_receives(network: &Network, errors: &mut Vec<ValidationError>) {
    let send_set: HashSet<(usize, usize, usize)> = network
        .edges
        .iter()
        .filter(|e| e.origin == EdgeOrigin::Send)
        .map(|e| (e.from_node.0, e.to_node.0, e.contract.0))
        .collect();

    for edge in network
        .edges
        .iter()
        .filter(|e| e.origin == EdgeOrigin::Receive)
    {
        let key = (edge.from_node.0, edge.to_node.0, edge.contract.0);
        if !send_set.contains(&key) {
            errors.push(ValidationError::UnmatchedReceive {
                from_node: network.nodes[edge.from_node.0].name.clone(),
                to_node: network.nodes[edge.to_node.0].name.clone(),
                contract: network.contracts[edge.contract.0].name.clone(),
            });
        }
    }
}

/// A node is orphaned if it appears in no edges at all (neither as sender nor receiver).
pub fn check_orphan_nodes(network: &Network, errors: &mut Vec<ValidationError>) {
    let referenced: HashSet<usize> = network
        .edges
        .iter()
        .flat_map(|e| [e.from_node.0, e.to_node.0])
        .collect();

    for (idx, node) in network.nodes.iter().enumerate() {
        if !referenced.contains(&idx) {
            errors.push(ValidationError::OrphanNode {
                node: node.name.clone(),
            });
        }
    }
}

/// Returns true if the FieldType is considered POD (plain old data) for iceoryx.
/// Nested types are not POD because they reference another named struct type
/// and cannot be guaranteed to be fixed-layout / trivially copyable at this
/// level of analysis.
fn is_pod(typ: &FieldType) -> bool {
    match typ {
        FieldType::Nested(_) => false,
        FieldType::Array { elem, .. } => is_pod(elem),
        _ => true,
    }
}

/// For every contract using Iceoryx transport, all schema fields must be POD.
pub fn check_iceoryx_pod(network: &Network, errors: &mut Vec<ValidationError>) {
    for contract in network
        .contracts
        .iter()
        .filter(|c| c.transport == Transport::Iceoryx)
    {
        let schema = &network.schemas[contract.schema.0];
        for struct_def in &schema.structs {
            for field in &struct_def.fields {
                if !is_pod(&field.typ) {
                    errors.push(ValidationError::IceoryxPodViolation {
                        contract: contract.name.clone(),
                        field: field.name.clone(),
                    });
                }
            }
        }
    }
}

/// Check for duplicate node names and duplicate contract names.
pub fn check_duplicate_names(network: &Network, errors: &mut Vec<ValidationError>) {
    let mut seen_nodes: HashSet<&str> = HashSet::new();
    for node in &network.nodes {
        if !seen_nodes.insert(node.name.as_str()) {
            errors.push(ValidationError::DuplicateNodeName {
                name: node.name.clone(),
            });
        }
    }

    let mut seen_contracts: HashSet<&str> = HashSet::new();
    for contract in &network.contracts {
        if !seen_contracts.insert(contract.name.as_str()) {
            errors.push(ValidationError::DuplicateContractName {
                name: contract.name.clone(),
            });
        }
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use nexus_core::{
        Contract, ContractIdx, Edge, EdgeOrigin, Field, FieldType, Network, Node, NodeIdx, Schema,
        SchemaIdx, StructDef, Transport,
    };

    use super::*;
    use crate::error::ValidationError;

    // -------------------------------------------------------------------------
    // Helpers
    // -------------------------------------------------------------------------

    /// Build a minimal Network from scratch.
    fn empty_network() -> Network {
        Network {
            nodes: Vec::new(),
            contracts: Vec::new(),
            schemas: Vec::new(),
            edges: Vec::new(),
            node_index: HashMap::new(),
            contract_index: HashMap::new(),
        }
    }

    fn add_node(net: &mut Network, name: &str) -> NodeIdx {
        let idx = NodeIdx(net.nodes.len());
        net.node_index.insert(name.to_string(), idx);
        net.nodes.push(Node {
            name: name.to_string(),
        });
        idx
    }

    fn add_schema(net: &mut Network, name: &str, fields: Vec<Field>) -> SchemaIdx {
        let idx = SchemaIdx(net.schemas.len());
        net.schemas.push(Schema {
            name: name.to_string(),
            structs: vec![StructDef {
                name: name.to_string(),
                fields,
            }],
        });
        idx
    }

    fn add_contract(
        net: &mut Network,
        name: &str,
        transport: Transport,
        schema: SchemaIdx,
    ) -> ContractIdx {
        let idx = ContractIdx(net.contracts.len());
        net.contract_index.insert(name.to_string(), idx);
        net.contracts.push(Contract {
            name: name.to_string(),
            transport,
            schema,
        });
        idx
    }

    fn add_edge(
        net: &mut Network,
        from: NodeIdx,
        to: NodeIdx,
        contract: ContractIdx,
        origin: EdgeOrigin,
    ) {
        net.edges.push(Edge {
            from_node: from,
            to_node: to,
            contract,
            origin,
        });
    }

    /// Construct a small valid network: A --msg--> B, with both Send and Receive edges.
    fn make_valid_network() -> Network {
        let mut net = empty_network();
        let a = add_node(&mut net, "A");
        let b = add_node(&mut net, "B");
        let s = add_schema(
            &mut net,
            "msg",
            vec![Field {
                name: "x".into(),
                typ: FieldType::U32,
            }],
        );
        let c = add_contract(&mut net, "msg", Transport::UnixSocket, s);
        add_edge(&mut net, a, b, c, EdgeOrigin::Send);
        add_edge(&mut net, a, b, c, EdgeOrigin::Receive);
        net
    }

    // -------------------------------------------------------------------------
    // 1. Valid network
    // -------------------------------------------------------------------------

    #[test]
    fn test_valid_network() {
        let net = make_valid_network();
        let mut errors = Vec::new();
        check_unmatched_sends(&net, &mut errors);
        check_unmatched_receives(&net, &mut errors);
        check_orphan_nodes(&net, &mut errors);
        check_iceoryx_pod(&net, &mut errors);
        check_duplicate_names(&net, &mut errors);
        assert!(errors.is_empty(), "Expected no errors, got: {errors:?}");
    }

    // -------------------------------------------------------------------------
    // 2. Unmatched send
    // -------------------------------------------------------------------------

    #[test]
    fn test_unmatched_send() {
        let mut net = empty_network();
        let a = add_node(&mut net, "A");
        let b = add_node(&mut net, "B");
        let s = add_schema(&mut net, "msg", vec![]);
        let c = add_contract(&mut net, "msg", Transport::UnixSocket, s);
        // Only Send edge, no matching Receive
        add_edge(&mut net, a, b, c, EdgeOrigin::Send);

        let mut errors = Vec::new();
        check_unmatched_sends(&net, &mut errors);

        assert_eq!(errors.len(), 1);
        assert_eq!(
            errors[0],
            ValidationError::UnmatchedSend {
                from_node: "A".into(),
                to_node: "B".into(),
                contract: "msg".into(),
            }
        );
    }

    // -------------------------------------------------------------------------
    // 3. Unmatched receive
    // -------------------------------------------------------------------------

    #[test]
    fn test_unmatched_receive() {
        let mut net = empty_network();
        let a = add_node(&mut net, "A");
        let b = add_node(&mut net, "B");
        let s = add_schema(&mut net, "msg", vec![]);
        let c = add_contract(&mut net, "msg", Transport::UnixSocket, s);
        // Only Receive edge, no matching Send
        add_edge(&mut net, a, b, c, EdgeOrigin::Receive);

        let mut errors = Vec::new();
        check_unmatched_receives(&net, &mut errors);

        assert_eq!(errors.len(), 1);
        assert_eq!(
            errors[0],
            ValidationError::UnmatchedReceive {
                from_node: "A".into(),
                to_node: "B".into(),
                contract: "msg".into(),
            }
        );
    }

    // -------------------------------------------------------------------------
    // 4. Orphan node
    // -------------------------------------------------------------------------

    #[test]
    fn test_orphan_node() {
        let mut net = make_valid_network();
        // Add a node that participates in no edges
        add_node(&mut net, "orphan");

        let mut errors = Vec::new();
        check_orphan_nodes(&net, &mut errors);

        assert_eq!(errors.len(), 1);
        assert_eq!(
            errors[0],
            ValidationError::OrphanNode {
                node: "orphan".into()
            }
        );
    }

    // -------------------------------------------------------------------------
    // 5. Multiple errors collected
    // -------------------------------------------------------------------------

    #[test]
    fn test_multiple_errors() {
        let mut net = empty_network();
        let a = add_node(&mut net, "A");
        let b = add_node(&mut net, "B");
        let _orphan = add_node(&mut net, "orphan");
        let s = add_schema(&mut net, "msg", vec![]);
        let c = add_contract(&mut net, "msg", Transport::UnixSocket, s);
        // Send without matching Receive
        add_edge(&mut net, a, b, c, EdgeOrigin::Send);
        // No Receive edge so UnmatchedSend fires; orphan node fires too.

        let mut errors = Vec::new();
        check_unmatched_sends(&net, &mut errors);
        check_unmatched_receives(&net, &mut errors);
        check_orphan_nodes(&net, &mut errors);

        // Expect at least: one UnmatchedSend + one OrphanNode
        assert!(
            errors.len() >= 2,
            "Expected at least 2 errors, got: {errors:?}"
        );
        assert!(errors
            .iter()
            .any(|e| matches!(e, ValidationError::UnmatchedSend { .. })));
        assert!(errors
            .iter()
            .any(|e| matches!(e, ValidationError::OrphanNode { .. })));
    }

    // -------------------------------------------------------------------------
    // 6. Iceoryx POD – valid (all primitive fields)
    // -------------------------------------------------------------------------

    #[test]
    fn test_iceoryx_pod_valid() {
        let mut net = empty_network();
        let a = add_node(&mut net, "A");
        let b = add_node(&mut net, "B");
        let s = add_schema(
            &mut net,
            "pod_msg",
            vec![
                Field {
                    name: "x".into(),
                    typ: FieldType::U32,
                },
                Field {
                    name: "y".into(),
                    typ: FieldType::F64,
                },
                Field {
                    name: "flag".into(),
                    typ: FieldType::Bool,
                },
            ],
        );
        let c = add_contract(&mut net, "pod_msg", Transport::Iceoryx, s);
        add_edge(&mut net, a, b, c, EdgeOrigin::Send);
        add_edge(&mut net, a, b, c, EdgeOrigin::Receive);

        let mut errors = Vec::new();
        check_iceoryx_pod(&net, &mut errors);

        assert!(errors.is_empty(), "Expected no POD errors, got: {errors:?}");
    }

    // -------------------------------------------------------------------------
    // 7. Iceoryx POD – violation (Nested field)
    // -------------------------------------------------------------------------

    #[test]
    fn test_iceoryx_pod_violation() {
        let mut net = empty_network();
        let a = add_node(&mut net, "A");
        let b = add_node(&mut net, "B");
        let s = add_schema(
            &mut net,
            "bad_msg",
            vec![
                Field {
                    name: "header".into(),
                    typ: FieldType::U32,
                },
                Field {
                    name: "nested_field".into(),
                    typ: FieldType::Nested("SomeStruct".into()),
                },
            ],
        );
        let c = add_contract(&mut net, "bad_msg", Transport::Iceoryx, s);
        add_edge(&mut net, a, b, c, EdgeOrigin::Send);
        add_edge(&mut net, a, b, c, EdgeOrigin::Receive);

        let mut errors = Vec::new();
        check_iceoryx_pod(&net, &mut errors);

        assert_eq!(errors.len(), 1);
        assert_eq!(
            errors[0],
            ValidationError::IceoryxPodViolation {
                contract: "bad_msg".into(),
                field: "nested_field".into(),
            }
        );
    }

    // -------------------------------------------------------------------------
    // 8. Duplicate names
    // -------------------------------------------------------------------------

    #[test]
    fn test_duplicate_node_names() {
        let mut net = empty_network();
        // Insert two nodes with the same name manually to simulate a bad state
        net.nodes.push(Node { name: "dup".into() });
        net.nodes.push(Node { name: "dup".into() });

        let mut errors = Vec::new();
        check_duplicate_names(&net, &mut errors);

        assert_eq!(errors.len(), 1);
        assert_eq!(
            errors[0],
            ValidationError::DuplicateNodeName { name: "dup".into() }
        );
    }

    #[test]
    fn test_duplicate_contract_names() {
        let mut net = empty_network();
        let s = add_schema(&mut net, "s", vec![]);
        net.contracts.push(Contract {
            name: "dup_contract".into(),
            transport: Transport::UnixSocket,
            schema: s,
        });
        net.contracts.push(Contract {
            name: "dup_contract".into(),
            transport: Transport::UnixSocket,
            schema: s,
        });

        let mut errors = Vec::new();
        check_duplicate_names(&net, &mut errors);

        assert_eq!(errors.len(), 1);
        assert_eq!(
            errors[0],
            ValidationError::DuplicateContractName {
                name: "dup_contract".into(),
            }
        );
    }
}
