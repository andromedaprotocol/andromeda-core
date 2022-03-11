pub mod ado_base;
pub mod error;
pub mod primitive;
pub mod withdraw;

use crate::error::ContractError;
use cosmwasm_std::{from_binary, to_binary, Binary, Coin};
use cw20::Cw20Coin;
use schemars::JsonSchema;
use serde::{de::DeserializeOwned, Deserialize, Serialize};

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

pub fn parse_message<T: DeserializeOwned>(data: &Option<Binary>) -> Result<T, ContractError> {
    let data = unwrap_or_err(data, ContractError::MissingRequiredMessageData {})?;
    parse_struct::<T>(data)
}

pub fn encode_binary<T>(val: &T) -> Result<Binary, ContractError>
where
    T: Serialize,
{
    match to_binary(val) {
        Ok(encoded_val) => Ok(encoded_val),
        Err(err) => Err(ContractError::ParsingError {
            err: err.to_string(),
        }),
    }
}

pub fn unwrap_or_err<T>(val_opt: &Option<T>, err: ContractError) -> Result<&T, ContractError> {
    match val_opt {
        Some(val) => Ok(val),
        None => Err(err),
    }
}

/// A simple implementation of Solidity's "require" function. Takes a precondition and an error to return if the precondition is not met.
///
/// ## Arguments
///
/// * `precond` - The required precondition, will return provided "err" parameter if precondition is false
/// * `err` - The error to return if the required precondition is false
///
/// ## Example
/// ```
/// use andromeda_protocol::error::ContractError;
/// use cosmwasm_std::StdError;
/// use andromeda_protocol::require;
/// require(false, ContractError::Std(StdError::generic_err("Some boolean condition was not met")));
/// ```
pub fn require(precond: bool, err: ContractError) -> Result<bool, ContractError> {
    match precond {
        true => Ok(true),
        false => Err(err),
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub enum Funds {
    Native(Coin),
    Cw20(Cw20Coin),
}

impl Funds {
    // There is probably a more idiomatic way of doing this with From and Into...
    pub fn try_get_coin(&self) -> Result<Coin, ContractError> {
        match self {
            Funds::Native(coin) => Ok(coin.clone()),
            Funds::Cw20(_) => Err(ContractError::ParsingError {
                err: "Funds is not of type Native".to_string(),
            }),
        }
    }
}

#[cfg(test)]
mod test {
    use cosmwasm_std::to_binary;
    use cw20::Expiration;

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
