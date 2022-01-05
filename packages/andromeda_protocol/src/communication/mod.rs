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

// pub fn is_valid_json(val: &str) -> Result<(), ContractError> {
//     if serde_json::from_str::<Value>(val).is_err() {
//         Err(ContractError::InvalidJSON {})
//     } else {
//         Ok(())
//     }
// }

pub fn parse_optional_data(val: Option<String>) -> Result<Option<Value>, ContractError> {
    if let Some(json_string) = val {
        if let Ok(val) = serde_json::from_str::<Value>(json_string.as_str()) {
            Ok(Some(val))
        } else {
            Err(ContractError::InvalidJSON {})
        }
    } else {
        Ok(None)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_parse_optional_data() {
        let valid_json = "{ \"field\": \"value\" }";

        assert!(parse_optional_data(Some(valid_json.to_string())).is_ok());

        let invalid_json = "notjson";

        assert!(parse_optional_data(Some(invalid_json.to_string())).is_err());

        assert!(parse_optional_data(None).is_ok());
    }
}
