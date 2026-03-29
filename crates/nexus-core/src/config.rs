use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct NetworkConfig {
    pub nodes: Vec<NodeConfig>,
    pub contracts: Vec<ContractConfig>,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct NodeConfig {
    pub name: String,
    #[serde(default)]
    pub sends: Vec<EdgeConfig>,
    #[serde(default)]
    pub receives: Vec<EdgeConfig>,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct EdgeConfig {
    pub contract: String,
    pub to: Option<String>,
    pub from: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct ContractConfig {
    pub name: String,
    pub transport: String,
    pub schema: Option<String>,
    #[serde(default)]
    pub fields: Vec<InlineFieldConfig>,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct InlineFieldConfig {
    pub name: String,
    #[serde(rename = "type")]
    pub typ: String,
}
