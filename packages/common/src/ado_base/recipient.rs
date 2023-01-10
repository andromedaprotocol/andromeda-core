use crate::{
    ado_base::{AndromedaMsg, ExecuteMsg},
    app::AndrAddress,
    encode_binary,
    error::ContractError,
};
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Api, BankMsg, Binary, Coin, CosmosMsg, QuerierWrapper, SubMsg, WasmMsg};

use cw20::{Cw20Coin, Cw20ExecuteMsg};
use cw_asset::{AssetInfo, AssetInfoBase};

/// ADOs use a default Receive message for handling funds,
/// this struct states that the recipient is an ADO and may attach the data field to the Receive message
#[cw_serde]
pub struct ADORecipient {
    /// Addr can also be a human-readable identifier used in a app contract.
    pub address: AndrAddress,
    pub msg: Option<Binary>,
}

#[cw_serde]
pub enum KernelMessage {}

const DEFAULT: u32 = 1;

#[cw_serde]
pub enum Recipient {
    /// An address that is not another ADO. It is assumed that it is a valid address.
    Addr(String),
    ADO(ADORecipient),
}

impl Recipient {
    /// Creates an Addr Recipient from the given string
    pub fn from_string(addr: String) -> Recipient {
        Recipient::Addr(addr)
    }

    /// Gets the address of the recipient. If the is an ADORecipient it will query the app
    /// contract to get its address if it fails address validation.
    pub fn get_addr(
        &self,
        api: &dyn Api,
        querier: &QuerierWrapper,
        app_contract: Option<Addr>,
    ) -> Result<String, ContractError> {
        match &self {
            Recipient::Addr(string) => Ok(string.to_owned()),
            Recipient::ADO(recip) => recip.address.get_address(api, querier, app_contract),
        }
    }

    /// Generates the sub message depending on the type of the recipient.
    pub fn generate_msg_native(
        &self,
        api: &dyn Api,
        querier: &QuerierWrapper,
        app_contract: Option<Addr>,
        funds: Vec<Coin>,
    ) -> Result<SubMsg, ContractError> {
        Ok(match &self {
            Recipient::ADO(recip) => SubMsg::new(WasmMsg::Execute {
                contract_addr: self.get_addr(api, querier, app_contract)?,
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
        app_contract: Option<Addr>,
        cw20_coin: Cw20Coin,
    ) -> Result<SubMsg, ContractError> {
        Ok(match &self {
            Recipient::ADO(recip) => SubMsg::new(WasmMsg::Execute {
                contract_addr: cw20_coin.address,
                msg: encode_binary(&Cw20ExecuteMsg::Send {
                    contract: self.get_addr(api, querier, app_contract)?,
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

    pub fn generate_msg_from_asset(
        &self,
        api: &dyn Api,
        querier: &QuerierWrapper,
        app_contract: Option<Addr>,
        asset: AssetInfo,
        funds: Vec<Coin>,
    ) -> Result<SubMsg, ContractError> {
        match asset {
            AssetInfoBase::Cw20(ref contract_addr) => self.generate_msg_cw20(
                api,
                querier,
                app_contract,
                Cw20Coin {
                    address: contract_addr.to_string(),
                    amount: asset.query_balance(querier, contract_addr)?,
                },
            ),
            AssetInfoBase::Native(_denom) => {
                self.generate_msg_native(api, querier, app_contract, funds)
            }
            _ => Err(ContractError::InvalidAsset {
                asset: asset.to_string(),
            }),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::testing::mock_querier::{mock_dependencies_custom, MOCK_APP_CONTRACT};
    use cosmwasm_std::{coins, testing::mock_dependencies, BankMsg, CosmosMsg, SubMsg, WasmMsg};

    fn andr_address(identifer: impl Into<String>) -> AndrAddress {
        AndrAddress {
            identifier: identifer.into(),
        }
    }

    #[test]
    fn test_recipient_addr_generate_msg_native() {
        let deps = mock_dependencies();
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
        let deps = mock_dependencies();
        let recipient = Recipient::ADO(ADORecipient {
            address: andr_address("address"),
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
    fn test_recipient_ado_generate_msg_native_app() {
        let deps = mock_dependencies_custom(&[]);
        let recipient = Recipient::ADO(ADORecipient {
            address: andr_address("ab"),
            msg: None,
        });
        let funds = coins(100, "uusd");
        let msg = recipient
            .generate_msg_native(
                deps.as_ref().api,
                &deps.as_ref().querier,
                Some(Addr::unchecked(MOCK_APP_CONTRACT)),
                funds.clone(),
            )
            .unwrap();
        let expected_msg = SubMsg::new(WasmMsg::Execute {
            contract_addr: "actual_address".to_string(),
            msg: encode_binary(&ExecuteMsg::AndrReceive(AndromedaMsg::Receive(None))).unwrap(),
            funds,
        });
        assert_eq!(expected_msg, msg);
    }

    #[test]
    fn test_recipient_addr_generate_msg_cw20() {
        let deps = mock_dependencies();
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
        let deps = mock_dependencies();
        let recipient = Recipient::ADO(ADORecipient {
            address: andr_address("address"),
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

    #[test]
    fn test_recipient_ado_generate_msg_cw20_app() {
        let deps = mock_dependencies_custom(&[]);
        let recipient = Recipient::ADO(ADORecipient {
            address: andr_address("ab"),
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
                Some(Addr::unchecked(MOCK_APP_CONTRACT)),
                cw20_coin.clone(),
            )
            .unwrap();
        let expected_msg = SubMsg::new(WasmMsg::Execute {
            contract_addr: "cw20_address".to_string(),
            msg: encode_binary(&Cw20ExecuteMsg::Send {
                contract: "actual_address".to_string(),
                amount: cw20_coin.amount,
                msg: encode_binary(&ExecuteMsg::AndrReceive(AndromedaMsg::Receive(None))).unwrap(),
            })
            .unwrap(),
            funds: vec![],
        });
        assert_eq!(expected_msg, msg);
    }

    #[test]
    fn test_recipient_get_addr_addr_recipient() {
        let deps = mock_dependencies();
        let recipient = Recipient::Addr("address".to_string());
        assert_eq!(
            "address",
            recipient
                .get_addr(deps.as_ref().api, &deps.as_ref().querier, None)
                .unwrap()
        );
    }

    #[test]
    fn test_recipient_get_addr_ado_recipient_not_app() {
        let deps = mock_dependencies();
        let recipient = Recipient::ADO(ADORecipient {
            address: andr_address("address"),
            msg: None,
        });
        assert_eq!(
            "address",
            recipient
                .get_addr(deps.as_ref().api, &deps.as_ref().querier, None)
                .unwrap()
        );
    }

    #[test]
    fn test_recipient_get_addr_ado_recipient_app() {
        let deps = mock_dependencies_custom(&[]);
        let recipient = Recipient::ADO(ADORecipient {
            // Since MockApi treats strings under 3 length invalid we use this.
            address: andr_address("ab"),
            msg: None,
        });
        assert_eq!(
            "actual_address",
            recipient
                .get_addr(
                    deps.as_ref().api,
                    &deps.as_ref().querier,
                    Some(Addr::unchecked(MOCK_APP_CONTRACT))
                )
                .unwrap()
        );
    }
}
