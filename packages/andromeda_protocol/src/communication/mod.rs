use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::ContractError;

pub mod msg;

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub enum AndromedaMsg {
    Receive(Option<String>),
}

pub enum AndromedaQuery {
    Get(Option<String>),
    Owner {},
    Operators {},
}

pub fn is_valid_json(val: &str) -> Result<(), ContractError> {
    if serde_json::from_str::<Value>(val).is_err() {
        Err(ContractError::InvalidJSON {})
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_is_valid_json() {
        let valid_json = "{ \"field\": \"value\" }";

        assert!(is_valid_json(valid_json).is_ok());

        let invalid_json = "notjson";

        assert!(is_valid_json(invalid_json).is_err())
    }
}
