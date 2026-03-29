use thiserror::Error;

#[derive(Debug, Error)]
pub enum NexusCoreError {
    #[error("failed to read file {path}: {source}")]
    FileRead {
        path: String,
        #[source]
        source: std::io::Error,
    },

    #[error("failed to parse network.toml: {0}")]
    TomlParse(#[from] toml::de::Error),

    #[error("failed to parse schema file {path}: {message}")]
    SchemaParse { path: String, message: String },

    #[error("unknown transport type: {0}")]
    UnknownTransport(String),

    #[error("unknown field type: {0}")]
    UnknownFieldType(String),

    #[error("schema file not found: {0}")]
    SchemaNotFound(String),

    #[error("contract '{contract}' has neither schema file nor inline fields")]
    NoSchema { contract: String },

    #[error("node '{node}' referenced in edge but not defined")]
    UndefinedNode { node: String },

    #[error("contract '{contract}' referenced in edge but not defined")]
    UndefinedContract { contract: String },
}
