use thiserror::Error;
use std::fmt;

#[derive(Error, Debug)]
pub enum MinLLMError {
    #[error("Flow execution error: {0}")]
    FlowError(String),
    
    #[error("Node execution error: {0}")]
    NodeError(String),
    
    #[error("Store access error: {0}")]
    StoreError(String),
    
    #[error("Python conversion error: {0}")]
    PyConversionError(String),
    
    #[error("Unknown error: {0}")]
    Unknown(String),
}

pub type Result<T> = std::result::Result<T, MinLLMError>;

// Common result type for node execution
pub struct ActionName(pub String);

impl Default for ActionName {
    fn default() -> Self {
        ActionName("default".to_string())
    }
}

impl From<&str> for ActionName {
    fn from(s: &str) -> Self {
        ActionName(s.to_string())
    }
}

impl From<String> for ActionName {
    fn from(s: String) -> Self {
        ActionName(s)
    }
}

impl fmt::Display for ActionName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl AsRef<str> for ActionName {
    fn as_ref(&self) -> &str {
        &self.0
    }
} 