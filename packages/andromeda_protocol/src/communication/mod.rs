use cosmwasm_std::{
    from_binary, to_binary, BankMsg, Binary, Coin, CosmosMsg, DepsMut, SubMsg, WasmMsg,
};
use schemars::JsonSchema;
use serde::{de::DeserializeOwned, Deserialize, Serialize};

use crate::{common::unwrap_or_err, error::ContractError};

pub mod msg;

// ADOs use a default Receive message for handling funds, this struct states that the recipient is an ADO and may attach the data field to the Receive message
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ADORecipient {
    pub addr: String,
    pub msg: Option<Binary>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Recipient {
    Addr(String),
    ADO(ADORecipient),
}

impl Recipient {
    /// Creates an Addr Recipient from the given string
    pub fn from_string(addr: String) -> Recipient {
        Recipient::Addr(addr)
    }

    /// Creates an ADO Recipient from the given string with an empty Data field
    pub fn ado_from_string(addr: String) -> Recipient {
        Recipient::ADO(ADORecipient { addr, msg: None })
    }

    pub fn generate_msg(&self, deps: &DepsMut, funds: Vec<Coin>) -> Result<SubMsg, ContractError> {
        Ok(match &self {
            Recipient::ADO(recip) => SubMsg::new(WasmMsg::Execute {
                contract_addr: deps.api.addr_validate(&recip.addr)?.to_string(),
                msg: encode_binary(&ExecuteMsg::AndrReceive(AndromedaMsg::Receive(
                    recip.clone().msg,
                )))?,
                funds,
            }),
            Recipient::Addr(addr) => SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
                to_address: addr.clone(),
                amount: funds,
            })),
        })
    }
}

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
    IsOperator { address: String },
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    AndrReceive(AndromedaMsg),
}

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
    let data = unwrap_or_err(data, ContractError::MissingRequiredMessageData {})?;
    parse_struct::<T>(&data)
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
