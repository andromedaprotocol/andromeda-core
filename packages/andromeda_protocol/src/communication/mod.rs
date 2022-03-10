use cosmwasm_std::{
    from_binary, to_binary, Api, BankMsg, Binary, Coin, CosmosMsg, QuerierWrapper, QueryRequest,
    SubMsg, WasmMsg, WasmQuery,
};
use cw20::{Cw20Coin, Cw20ExecuteMsg};
use schemars::JsonSchema;
use serde::{de::DeserializeOwned, Deserialize, Serialize};

use crate::{
    ado_base::{hooks::AndromedaHook, AndromedaMsg, AndromedaQuery, ExecuteMsg, QueryMsg},
    common::unwrap_or_err,
    error::ContractError,
};

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

    /// Gets the address of the recipient.
    pub fn get_addr(&self) -> String {
        match &self {
            Recipient::Addr(addr) => addr.clone(),
            Recipient::ADO(ado_recipient) => ado_recipient.addr.clone(),
        }
    }

    /// Generates the sub message depending on the type of the recipient.
    pub fn generate_msg_native(
        &self,
        api: &dyn Api,
        funds: Vec<Coin>,
    ) -> Result<SubMsg, ContractError> {
        Ok(match &self {
            Recipient::ADO(recip) => SubMsg::new(WasmMsg::Execute {
                contract_addr: api.addr_validate(&recip.addr)?.to_string(),
                msg: encode_binary(&ExecuteMsg::AndrReceive(AndromedaMsg::Receive(
                    recip.msg.clone(),
                )))?,
                funds,
            }),
            Recipient::Addr(addr) => SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
                to_address: addr.clone(),
                amount: funds,
            })),
        })
    }

    /// Generates the sub message depending on the type of the recipient.
    pub fn generate_msg_cw20(
        &self,
        api: &dyn Api,
        cw20_coin: Cw20Coin,
    ) -> Result<SubMsg, ContractError> {
        Ok(match &self {
            Recipient::ADO(recip) => SubMsg::new(WasmMsg::Execute {
                contract_addr: cw20_coin.address,
                msg: encode_binary(&Cw20ExecuteMsg::Send {
                    contract: api.addr_validate(&recip.addr)?.to_string(),
                    amount: cw20_coin.amount,
                    msg: encode_binary(&ExecuteMsg::AndrReceive(AndromedaMsg::Receive(
                        recip.msg.clone(),
                    )))?,
                })?,
                funds: vec![],
            }),
            Recipient::Addr(addr) => SubMsg::new(WasmMsg::Execute {
                contract_addr: cw20_coin.address,
                msg: encode_binary(&Cw20ExecuteMsg::Transfer {
                    recipient: addr.to_string(),
                    amount: cw20_coin.amount,
                })?,
                funds: vec![],
            }),
        })
    }
}

/// Helper enum for serialization
#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub enum HookMsg {
    AndrHook(AndromedaHook),
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

/// Helper function for querying a contract using AndromedaQuery::Get
pub fn query_get<T>(
    data: Option<Binary>,
    address: String,
    querier: &QuerierWrapper,
) -> Result<T, ContractError>
where
    T: DeserializeOwned,
{
    let query_msg = QueryMsg::AndrQuery(AndromedaQuery::Get(data));
    let resp: T = querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: address,
        msg: to_binary(&query_msg)?,
    }))?;

    Ok(resp)
}

#[cfg(test)]
mod test {
    use cosmwasm_std::testing::mock_dependencies;
    use cosmwasm_std::{coins, to_binary};
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

    #[test]
    fn test_recipient_addr_generate_msg_native() {
        let deps = mock_dependencies(&[]);
        let recipient = Recipient::Addr("address".to_string());
        let funds = coins(100, "uusd");
        let msg = recipient
            .generate_msg_native(deps.as_ref().api, funds.clone())
            .unwrap();
        let expected_msg = SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
            to_address: "address".to_string(),
            amount: funds,
        }));
        assert_eq!(expected_msg, msg);
    }

    #[test]
    fn test_recipient_ado_generate_msg_native() {
        let deps = mock_dependencies(&[]);
        let recipient = Recipient::ADO(ADORecipient {
            addr: "address".to_string(),
            msg: None,
        });
        let funds = coins(100, "uusd");
        let msg = recipient
            .generate_msg_native(deps.as_ref().api, funds.clone())
            .unwrap();
        let expected_msg = SubMsg::new(WasmMsg::Execute {
            contract_addr: "address".to_string(),
            msg: encode_binary(&ExecuteMsg::AndrReceive(AndromedaMsg::Receive(None))).unwrap(),
            funds,
        });
        assert_eq!(expected_msg, msg);
    }

    #[test]
    fn test_recipient_addr_generate_msg_cw20() {
        let deps = mock_dependencies(&[]);
        let recipient = Recipient::Addr("address".to_string());
        let cw20_coin = Cw20Coin {
            amount: 100u128.into(),
            address: "cw20_address".to_string(),
        };
        let msg = recipient
            .generate_msg_cw20(deps.as_ref().api, cw20_coin.clone())
            .unwrap();
        let expected_msg = SubMsg::new(WasmMsg::Execute {
            contract_addr: cw20_coin.address,
            msg: encode_binary(&Cw20ExecuteMsg::Transfer {
                recipient: "address".to_string(),
                amount: cw20_coin.amount,
            })
            .unwrap(),
            funds: vec![],
        });
        assert_eq!(expected_msg, msg);
    }

    #[test]
    fn test_recipient_ado_generate_msg_cw20() {
        let deps = mock_dependencies(&[]);
        let recipient = Recipient::ADO(ADORecipient {
            addr: "address".to_string(),
            msg: None,
        });
        let cw20_coin = Cw20Coin {
            amount: 100u128.into(),
            address: "cw20_address".to_string(),
        };
        let msg = recipient
            .generate_msg_cw20(deps.as_ref().api, cw20_coin.clone())
            .unwrap();
        let expected_msg = SubMsg::new(WasmMsg::Execute {
            contract_addr: "cw20_address".to_string(),
            msg: encode_binary(&Cw20ExecuteMsg::Send {
                contract: "address".to_string(),
                amount: cw20_coin.amount,
                msg: encode_binary(&ExecuteMsg::AndrReceive(AndromedaMsg::Receive(None))).unwrap(),
            })
            .unwrap(),
            funds: vec![],
        });
        assert_eq!(expected_msg, msg);
    }
}
