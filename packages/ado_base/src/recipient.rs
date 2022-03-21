use common::{
    ado_base::{recipient::Recipient, AndromedaMsg, ExecuteMsg},
    encode_binary,
    error::ContractError,
};
use cosmwasm_std::{Api, BankMsg, Coin, CosmosMsg, SubMsg, WasmMsg};
use cw20::{Cw20Coin, Cw20ExecuteMsg};

pub trait MessageGenerator {
    fn generate_msg_native(&self, api: &dyn Api, funds: Vec<Coin>)
        -> Result<SubMsg, ContractError>;

    fn generate_msg_cw20(
        &self,
        api: &dyn Api,
        cw20_coin: Cw20Coin,
    ) -> Result<SubMsg, ContractError>;
}

impl MessageGenerator for Recipient {
    /// Generates the sub message depending on the type of the recipient.
    fn generate_msg_native(
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
    fn generate_msg_cw20(
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

#[cfg(test)]
mod test {
    use super::*;
    use common::ado_base::recipient::ADORecipient;
    use cosmwasm_std::{coins, testing::mock_dependencies, BankMsg, CosmosMsg, SubMsg, WasmMsg};

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
