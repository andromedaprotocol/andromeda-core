use cosmwasm_std::{from_binary, Binary};
use schemars::{JsonSchema, _serde_json::to_string as serde_to_string};
use serde::{de::DeserializeOwned, Deserialize, Serialize};

use crate::{common::unwrap_or_err, error::ContractError};

pub mod msg;

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub enum AndromedaMsg {
    Receive(Option<Binary>),
    UpdateOwner { address: String },
    UpdateOperators { operators: Vec<String> },
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub enum AndromedaQuery {
    Get(Option<Binary>),
    Owner {},
    Operators {},
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    AndrReceive(AndromedaMsg),
}

// pub fn parse_optional_data(val: Option<String>) -> Result<Option<Value>, ContractError> {
//     if let Some(json_string) = val {
//         if let Ok(val) = serde_json::from_str::<Value>(json_string.as_str()) {
//             Ok(Some(val))
//         } else {
//             Err(ContractError::InvalidJSON {})
//         }
//     } else {
//         Ok(None)
//     }
// }

pub fn parse_struct<T>(val: &Binary) -> Result<T, ContractError>
where
    T: DeserializeOwned,
{
    let data_res = from_binary(val);
    match data_res {
        Ok(data) => Ok(data),
        Err(err) => Err(ContractError::ParsingError {
            err: err.to_string(),
        }),
    }
}

pub fn parse_message<T: DeserializeOwned>(data: Option<Binary>) -> Result<T, ContractError> {
    let data = unwrap_or_err(data, ContractError::MissingJSON {})?;
    parse_struct::<T>(&data)
}

pub fn to_json_string<T>(val: &T) -> Result<String, ContractError>
where
    T: Serialize,
{
    match serde_to_string(val) {
        Ok(val_string) => Ok(val_string),
        Err(err) => Err(ContractError::ParsingError {
            err: err.to_string(),
        }),
    }
}

// pub fn parse_u64(data: Value, key: String) -> Result<u64, ContractError> {
//     match data[key.clone()].as_u64() {
//         Some(val) => Ok(val),
//         None => Err(ContractError::InvalidJSONField {
//             key,
//             expected: "u64".to_string(),
//         }),
//     }
// }

// pub fn parse_string(data: &Value, key: &str) -> Result<String, ContractError> {
//     match data[key].as_str() {
//         Some(val) => Ok(val.to_string()),
//         None => Err(ContractError::InvalidJSONField {
//             key: key.to_string(),
//             expected: "string".to_string(),
//         }),
//     }
// }

// pub fn parse_object(data: &Value, key: &str) -> Result<Map<String, Value>, ContractError> {
//     match data[key].as_object() {
//         Some(val) => Ok(val.clone()),
//         None => Err(ContractError::InvalidJSONField {
//             key: key.to_string(),
//             expected: "object".to_string(),
//         }),
//     }
// }

#[cfg(test)]
mod test {
    use cosmwasm_std::to_binary;
    use cw721::Expiration;

    use super::*;
    #[derive(Deserialize, Serialize)]
    struct TestStruct {
        name: String,
        expiration: Expiration,
    }

    #[test]
    fn test_parse_struct() {
        let valid_json = to_binary(&TestStruct {
            name: "John Doe".to_string(),
            expiration: Expiration::AtHeight(123),
        })
        .unwrap();

        let test_struct: TestStruct = parse_struct(&valid_json).unwrap();
        assert_eq!(test_struct.name, "John Doe");
        assert_eq!(test_struct.expiration, Expiration::AtHeight(123));

        let invalid_json = to_binary("notavalidteststruct").unwrap();

        assert!(parse_struct::<TestStruct>(&invalid_json).is_err())
    }
}
