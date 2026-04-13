use thiserror::Error;

#[derive(Error, Debug)]
pub enum HermesError {
    #[error("Tool not found: {0}")]
    ToolNotFound(String),
    #[error("Iteration budget exceeded")]
    IterationBudgetExceeded,
    #[error("Interrupted")]
    Interrupted,
    #[error("Configuration error: {0}")]
    ConfigError(String),
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}
