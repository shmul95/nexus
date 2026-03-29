use thiserror::Error;

#[derive(Debug, Error, PartialEq)]
pub enum ValidationError {
    #[error("node '{from_node}' sends contract '{contract}' to '{to_node}', but '{to_node}' does not declare receiving it")]
    UnmatchedSend {
        from_node: String,
        to_node: String,
        contract: String,
    },

    #[error("node '{to_node}' receives contract '{contract}' from '{from_node}', but '{from_node}' does not declare sending it")]
    UnmatchedReceive {
        from_node: String,
        to_node: String,
        contract: String,
    },

    #[error("node '{node}' is orphaned — it has no sends or receives")]
    OrphanNode { node: String },

    #[error(
        "contract '{contract}' uses iceoryx transport but schema contains non-POD field '{field}'"
    )]
    IceoryxPodViolation { contract: String, field: String },

    #[error("duplicate node name: '{name}'")]
    DuplicateNodeName { name: String },

    #[error("duplicate contract name: '{name}'")]
    DuplicateContractName { name: String },
}
