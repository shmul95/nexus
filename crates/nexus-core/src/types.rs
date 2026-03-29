use std::collections::HashMap;

// === Index newtypes ===

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NodeIdx(pub usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ContractIdx(pub usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SchemaIdx(pub usize);

// === Resolved network graph ===

#[derive(Debug, Clone)]
pub struct Network {
    pub nodes: Vec<Node>,
    pub contracts: Vec<Contract>,
    pub schemas: Vec<Schema>,
    pub edges: Vec<Edge>,
    pub node_index: HashMap<String, NodeIdx>,
    pub contract_index: HashMap<String, ContractIdx>,
}

#[derive(Debug, Clone)]
pub struct Node {
    pub name: String,
}

#[derive(Debug, Clone)]
pub struct Contract {
    pub name: String,
    pub transport: Transport,
    pub schema: SchemaIdx,
}

#[derive(Debug, Clone)]
pub struct Edge {
    pub from_node: NodeIdx,
    pub to_node: NodeIdx,
    pub contract: ContractIdx,
    pub origin: EdgeOrigin,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EdgeOrigin {
    Send,
    Receive,
}

// === Schema types ===

#[derive(Debug, Clone)]
pub struct Schema {
    pub name: String,
    pub structs: Vec<StructDef>,
}

#[derive(Debug, Clone)]
pub struct StructDef {
    pub name: String,
    pub fields: Vec<Field>,
}

#[derive(Debug, Clone)]
pub struct Field {
    pub name: String,
    pub typ: FieldType,
}

#[derive(Debug, Clone, PartialEq)]
pub enum FieldType {
    U8,
    U16,
    U32,
    U64,
    I8,
    I16,
    I32,
    I64,
    F32,
    F64,
    Bool,
    Bytes(usize),
    StringFixed(usize),
    Array { elem: Box<FieldType>, len: usize },
    Nested(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Transport {
    UnixSocket,
    Grpc,
    Http,
    Iceoryx,
    SharedMemory,
    MessageQueue,
}
