use thiserror::Error;

#[derive(Debug, Error)]
pub enum CodegenError {
    #[error("transport '{0}' is not yet implemented")]
    UnsupportedTransport(String),

    #[error("template rendering error: {0}")]
    TemplateError(#[from] minijinja::Error),
}
