use thiserror::Error;

#[derive(Error, Debug)]
pub enum ClixError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    
    #[error("Command not found: {0}")]
    CommandNotFound(String),
    
    #[error("Command execution failed: {0}")]
    CommandExecutionFailed(String),
    
    #[error("Invalid command format: {0}")]
    InvalidCommandFormat(String),
}

pub type Result<T> = std::result::Result<T, ClixError>;