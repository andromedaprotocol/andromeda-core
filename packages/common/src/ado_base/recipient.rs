use crate::{
    ado_base::{query_get, AndromedaMsg, ExecuteMsg},
    encode_binary,
    error::ContractError,
};
use cosmwasm_std::{Addr, Api, BankMsg, Binary, Coin, CosmosMsg, QuerierWrapper, SubMsg, WasmMsg};
use cw20::{Cw20Coin, Cw20ExecuteMsg};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// ADOs use a default Receive message for handling funds,
/// this struct states that the recipient is an ADO and may attach the data field to the Receive message
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ADORecipient {
    /// Addr can also be a human-readable identifier used in a mission contract.
    pub addr: String,
    pub msg: Option<Binary>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Recipient {
    /// An address that is not another ADO. It is assumed that it is a valid address
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
    pub fn get_addr(
        &self,
        api: &dyn Api,
        querier: &QuerierWrapper,
        mission_contract: Option<Addr>,
    ) -> Result<String, ContractError> {
        match &self {
            Recipient::Addr(string) => Ok(string.to_owned()),
            Recipient::ADO(recip) => {
                let addr = api.addr_validate(&recip.addr);
                match addr {
                    Ok(addr) => Ok(addr.to_string()),
                    Err(_) => match mission_contract {
                        Some(mission_contract) => query_get::<String>(
                            Some(encode_binary(&recip.addr)?),
                            mission_contract.to_string(),
                            querier,
                        ),
                        // TODO: Make error more descriptive.
                        None => Err(ContractError::InvalidAddress {}),
                    },
                }
            }
        }
    }

    /// Generates the sub message depending on the type of the recipient.
    pub fn generate_msg_native(
        &self,
        api: &dyn Api,
        querier: &QuerierWrapper,
        mission_contract: Option<Addr>,
        funds: Vec<Coin>,
    ) -> Result<SubMsg, ContractError> {
        Ok(match &self {
            Recipient::ADO(recip) => SubMsg::new(WasmMsg::Execute {
                contract_addr: self.get_addr(api, querier, mission_contract)?,
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
        querier: &QuerierWrapper,
        mission_contract: Option<Addr>,
        cw20_coin: Cw20Coin,
    ) -> Result<SubMsg, ContractError> {
        Ok(match &self {
            Recipient::ADO(recip) => SubMsg::new(WasmMsg::Execute {
                contract_addr: cw20_coin.address,
                msg: encode_binary(&Cw20ExecuteMsg::Send {
                    contract: self.get_addr(api, querier, mission_contract)?,
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

#[cfg(test)]
mod test {
    use super::*;
    use cosmwasm_std::{coins, testing::mock_dependencies, BankMsg, CosmosMsg, SubMsg, WasmMsg};

    #[test]
    fn test_recipient_addr_generate_msg_native() {
        let deps = mock_dependencies(&[]);
        let recipient = Recipient::Addr("address".to_string());
        let funds = coins(100, "uusd");
        let msg = recipient
            .generate_msg_native(
                deps.as_ref().api,
                &deps.as_ref().querier,
                None,
                funds.clone(),
            )
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
            .generate_msg_native(
                deps.as_ref().api,
                &deps.as_ref().querier,
                None,
                funds.clone(),
            )
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
            .generate_msg_cw20(
                deps.as_ref().api,
                &deps.as_ref().querier,
                None,
                cw20_coin.clone(),
            )
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
            .generate_msg_cw20(
                deps.as_ref().api,
                &deps.as_ref().querier,
                None,
                cw20_coin.clone(),
            )
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
