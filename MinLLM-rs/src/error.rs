use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Node execution error: {0}")]
    NodeExecution(String),
    
    #[error("Flow execution error: {0}")]
    FlowExecution(String),
    
    #[error("Invalid action: {0}")]
    InvalidAction(String),
    
    #[error("Missing successor for action: {0}")]
    MissingSuccessor(String),
    
    #[error("Invalid operation: {0}")]
    InvalidOperation(String),
    
    #[cfg(feature = "python")]
    #[error("Python error: {0}")]
    Python(#[from] pyo3::PyErr),
    
    #[error("Async runtime error: {0}")]
    AsyncRuntime(#[from] tokio::task::JoinError),
    
    #[error("Unknown error: {0}")]
    Unknown(String),
} 