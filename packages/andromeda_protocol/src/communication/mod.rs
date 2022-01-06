use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

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

pub fn parse_struct<'a, T>(val: &'a str) -> Result<T, ContractError>
where
    T: Deserialize<'a>,
{
    let data_res = serde_json::from_str::<'a, T>(val);
    match data_res {
        Ok(data) => Ok(data),
        Err(err) => Err(ContractError::ParsingError {
            err: err.to_string(),
        }),
    }
}

pub fn to_json_string<T>(val: &T) -> Result<String, ContractError>
where
    T: Serialize,
{
    match serde_json::to_string(val) {
        Ok(val_string) => Ok(val_string),
        Err(err) => Err(ContractError::ParsingError {
            err: err.to_string(),
        }),
    }
}

pub fn parse_u64(data: Value, key: String) -> Result<u64, ContractError> {
    match data[key.clone()].as_u64() {
        Some(val) => Ok(val),
        None => Err(ContractError::InvalidJSONField {
            key,
            expected: "u64".to_string(),
        }),
    }
}

pub fn parse_string(data: &Value, key: &str) -> Result<String, ContractError> {
    match data[key].as_str() {
        Some(val) => Ok(val.to_string()),
        None => Err(ContractError::InvalidJSONField {
            key: key.to_string(),
            expected: "string".to_string(),
        }),
    }
}

pub fn parse_object(data: &Value, key: &str) -> Result<Map<String, Value>, ContractError> {
    match data[key].as_object() {
        Some(val) => Ok(val.clone()),
        None => Err(ContractError::InvalidJSONField {
            key: key.to_string(),
            expected: "object".to_string(),
        }),
    }
}

#[cfg(test)]
mod test {
    use cw721::Expiration;

    use super::*;

    #[test]
    fn test_parse_optional_data() {
        let valid_json = "{ \"field\": \"value\" }";

        assert!(parse_optional_data(Some(valid_json.to_string())).is_ok());

        let invalid_json = "notjson";

        assert!(parse_optional_data(Some(invalid_json.to_string())).is_err());

        assert!(parse_optional_data(None).is_ok());
    }

    #[derive(Deserialize)]
    struct TestStruct {
        name: String,
        expiration: Expiration,
    }

    #[test]
    fn test_parse_struct() {
        let valid_json = "{ \"name\": \"John Doe\", \"expiration\": { \"at_height\": 123 }}";

        let test_struct: TestStruct = parse_struct(valid_json).unwrap();
        assert_eq!(test_struct.name, "John Doe");
        assert_eq!(test_struct.expiration, Expiration::AtHeight(123));

        let invalid_json = "notavalidteststruct";

        assert!(parse_struct::<TestStruct>(invalid_json).is_err())
    }
}
