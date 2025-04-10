use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum JsonError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Invalid JSON format")]
    InvalidJsonFormat,

    #[error("Array index out of bounds: {0}")]
    ArrayIndexOutOfBounds(String),

    #[error("Invalid array index: {0}")]
    InvalidArrayIndex(String),

    #[error("Key not found: {0}")]
    KeyNotFound(String),

    #[error("Failed to deserialize: {0}")]
    DeserializationError(String),

    #[error("Custom error: {0}")]
    Custom(String),
}

impl From<&str> for JsonError {
    fn from(message: &str) -> Self {
        JsonError::Custom(message.to_string())
    }
}

impl From<serde_cw_value::SerializerError> for JsonError {
    fn from(err: serde_cw_value::SerializerError) -> Self {
        JsonError::DeserializationError(err.to_string())
    }
}

impl From<serde_cw_value::DeserializerError> for JsonError {
    fn from(err: serde_cw_value::DeserializerError) -> Self {
        JsonError::DeserializationError(err.to_string())
    }
}
