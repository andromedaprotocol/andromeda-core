use super::{addresses::AndrAddr, messages::AMPMsg};
use crate::{ado_contract::ADOContract, common::encode_binary, error::ContractError};
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{to_json_binary, BankMsg, Binary, Coin, CosmosMsg, Deps, SubMsg, WasmMsg};
use cw20::{Cw20Coin, Cw20ExecuteMsg};
use serde::Serialize;

/// A simple struct used for inter-contract communication. The struct can be used in two ways:
///
/// 1. Simply just providing an `AndrAddr` which will treat the communication as a transfer of any related funds
/// 2. Providing an `AndrAddr` and a `Binary` message which will be sent to the contract at the resolved address
///
/// The `Binary` message can be any message that the contract at the resolved address can handle.
#[cw_serde]
pub struct Recipient {
    pub address: AndrAddr,
    pub msg: Option<Binary>,
    pub ibc_recovery_address: Option<AndrAddr>,
}

impl Recipient {
    pub fn new(addr: impl Into<String>, msg: Option<Binary>) -> Recipient {
        Recipient {
            address: AndrAddr::from_string(addr),
            msg,
            ibc_recovery_address: None,
        }
    }

    /// Validates a recipient by validating its address and recovery address (if it is provided)
    pub fn validate(&self, deps: &Deps) -> Result<(), ContractError> {
        self.address.validate(deps.api)?;
        self.address.get_raw_address(deps)?;

        // Validate the recovery address if it is providedReci
        if let Some(ibc_recovery_address) = self.ibc_recovery_address.clone() {
            ibc_recovery_address.validate(deps.api)?;
            ibc_recovery_address.get_raw_address(deps)?;
        }

        Ok(())
    }

    /// Creates a Recipient from the given string with no attached message
    pub fn from_string(addr: impl Into<String>) -> Recipient {
        Recipient {
            address: AndrAddr::from_string(addr.into()),
            msg: None,
            ibc_recovery_address: None,
        }
    }

    pub fn get_addr(&self) -> String {
        self.address.to_string()
    }

    pub fn get_message(&self) -> Option<Binary> {
        self.msg.clone()
    }

    pub fn is_cross_chain(&self) -> bool {
        let protocol = self.address.get_protocol();
        matches!(protocol, Some("ibc"))
    }

    /// Generates a direct sub message for the given recipient.
    pub fn generate_direct_msg(
        &self,
        deps: &Deps,
        funds: Vec<Coin>,
    ) -> Result<SubMsg, ContractError> {
        let resolved_addr = self.address.get_raw_address(deps)?;
        Ok(match &self.msg {
            Some(message) => SubMsg::new(WasmMsg::Execute {
                contract_addr: resolved_addr.to_string(),
                msg: message.clone(),
                funds,
            }),
            None => SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
                to_address: resolved_addr.to_string(),
                amount: funds,
            })),
        })
    }

    // TODO: Enable ICS20 messages? Maybe send approval for Kernel address then send the message to Kernel?
    /// Generates a message to send a CW20 token to the recipient with the attached message.
    ///
    /// **Assumes the attached message is a valid CW20 Hook message for the receiving address**.
    pub fn generate_msg_cw20(
        &self,
        deps: &Deps,
        cw20_coin: Cw20Coin,
    ) -> Result<SubMsg, ContractError> {
        let resolved_addr = self.address.get_raw_address(deps)?;
        Ok(match &self.msg {
            Some(msg) => SubMsg::new(WasmMsg::Execute {
                contract_addr: cw20_coin.address,
                msg: encode_binary(&Cw20ExecuteMsg::Send {
                    contract: resolved_addr.to_string(),
                    amount: cw20_coin.amount,
                    msg: msg.clone(),
                })?,
                funds: vec![],
            }),
            None => SubMsg::new(WasmMsg::Execute {
                contract_addr: cw20_coin.address,
                msg: encode_binary(&Cw20ExecuteMsg::Transfer {
                    recipient: resolved_addr.to_string(),
                    amount: cw20_coin.amount,
                })?,
                funds: vec![],
            }),
        })
    }

    /// Generates an AMP message from the given Recipient.
    ///
    /// This can be attached to an AMP Packet for execution via the aOS.
    pub fn generate_amp_msg(
        &self,
        deps: &Deps,
        funds: Option<Vec<Coin>>,
    ) -> Result<AMPMsg, ContractError> {
        let mut address = self.address.clone();
        if address.is_local_path() {
            let vfs_addr = ADOContract::default().get_vfs_address(deps.storage, &deps.querier)?;
            address = address.local_path_to_vfs_path(deps.storage, &deps.querier, vfs_addr)?;
        }
        Ok(AMPMsg::new(
            address.to_string(),
            self.msg.clone().unwrap_or_default(),
            funds,
        )
        .with_ibc_recovery(self.ibc_recovery_address.clone()))
    }

    /// Adds an IBC recovery address to the recipient
    ///
    /// This address can be used to recover any funds on failed IBC messages
    pub fn with_ibc_recovery(self, addr: impl Into<String>) -> Self {
        let mut new_recip = self;
        new_recip.ibc_recovery_address = Some(AndrAddr::from_string(addr.into()));
        new_recip
    }

    /// Adds a message to the recipient to be sent alongside any funds
    pub fn with_msg(self, msg: impl Serialize) -> Self {
        let mut new_recip = self;
        new_recip.msg = Some(to_json_binary(&msg).unwrap());
        new_recip
    }
}

#[cfg(test)]
mod test {
    use cosmwasm_std::{from_json, testing::mock_dependencies, Addr, Uint128};

    use crate::testing::mock_querier::{mock_dependencies_custom, MOCK_APP_CONTRACT};
    const RECIPIENT: &str = "cosmwasm1vewsdxxmeraett7ztsaym88jsrv85kzm0xvjg09xqz8aqvjcja0syapxq9";
    use super::*;

    #[test]
    fn test_generate_direct_msg() {
        let deps = mock_dependencies();
        let recipient = Recipient::from_string(RECIPIENT);
        let funds = vec![Coin {
            denom: "test".to_string(),
            amount: Uint128::from(100u128),
        }];
        let msg = recipient
            .generate_direct_msg(&deps.as_ref(), funds.clone())
            .unwrap();
        match msg.msg {
            CosmosMsg::Bank(BankMsg::Send { to_address, amount }) => {
                assert_eq!(to_address, RECIPIENT);
                assert_eq!(amount, funds);
            }
            _ => panic!("Unexpected message type"),
        }

        let recipient = Recipient::new(RECIPIENT, Some(Binary::from(b"test".to_vec())));
        let msg = recipient
            .generate_direct_msg(&deps.as_ref(), funds.clone())
            .unwrap();
        match msg.msg {
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr,
                msg,
                funds: msg_funds,
            }) => {
                assert_eq!(contract_addr, RECIPIENT);
                assert_eq!(msg, Binary::from(b"test".to_vec()));
                assert_eq!(msg_funds, funds);
            }
            _ => panic!("Unexpected message type"),
        }
    }

    #[test]
    fn test_generate_msg_cw20() {
        let deps = mock_dependencies();
        let recipient = Recipient::from_string(RECIPIENT);
        let cw20_coin = Cw20Coin {
            address: "test".to_string(),
            amount: Uint128::from(100u128),
        };
        let msg = recipient
            .generate_msg_cw20(&deps.as_ref(), cw20_coin.clone())
            .unwrap();
        match msg.msg {
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr,
                msg,
                funds,
            }) => {
                assert_eq!(contract_addr, "test");
                assert_eq!(funds, vec![] as Vec<Coin>);
                match from_json(msg).unwrap() {
                    Cw20ExecuteMsg::Transfer { recipient, amount } => {
                        assert_eq!(recipient, RECIPIENT);
                        assert_eq!(amount, cw20_coin.amount);
                    }
                    _ => panic!("Unexpected message type"),
                }
            }
            _ => panic!("Unexpected message type"),
        }

        let recipient = Recipient::new(RECIPIENT, Some(Binary::from(b"test".to_vec())));
        let msg = recipient
            .generate_msg_cw20(&deps.as_ref(), cw20_coin.clone())
            .unwrap();
        match msg.msg {
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr,
                msg,
                funds,
            }) => {
                assert_eq!(contract_addr, "test");
                assert_eq!(funds, vec![] as Vec<Coin>);
                match from_json(msg).unwrap() {
                    Cw20ExecuteMsg::Send {
                        contract,
                        amount,
                        msg: send_msg,
                    } => {
                        assert_eq!(contract, RECIPIENT);
                        assert_eq!(amount, cw20_coin.amount);
                        assert_eq!(send_msg, Binary::from(b"test".to_vec()));
                    }
                    _ => panic!("Unexpected message type"),
                }
            }
            _ => panic!("Unexpected message type"),
        }
    }

    #[test]
    fn test_generate_amp_msg() {
        let recipient = Recipient::from_string(RECIPIENT);
        let mut deps = mock_dependencies_custom(&[]);
        let msg = recipient.generate_amp_msg(&deps.as_ref(), None).unwrap();
        assert_eq!(msg.recipient, RECIPIENT);
        assert_eq!(msg.message, Binary::default());
        assert_eq!(msg.funds, vec![] as Vec<Coin>);

        let recipient = Recipient::new(RECIPIENT, Some(Binary::from(b"test".to_vec())));
        let msg = recipient.generate_amp_msg(&deps.as_ref(), None).unwrap();
        assert_eq!(msg.recipient, RECIPIENT);
        assert_eq!(msg.message, Binary::from(b"test".to_vec()));
        assert_eq!(msg.funds, vec![] as Vec<Coin>);

        let funds = vec![Coin {
            denom: "test".to_string(),
            amount: Uint128::from(100u128),
        }];
        let recipient = Recipient::from_string(RECIPIENT);
        let msg = recipient
            .generate_amp_msg(&deps.as_ref(), Some(funds.clone()))
            .unwrap();
        assert_eq!(msg.recipient, RECIPIENT);
        assert_eq!(msg.message, Binary::default());
        assert_eq!(msg.funds, funds);

        ADOContract::default()
            .app_contract
            .save(deps.as_mut().storage, &Addr::unchecked(MOCK_APP_CONTRACT))
            .unwrap();
        let recipient = Recipient::from_string("./test");
        let msg = recipient
            .generate_amp_msg(&deps.as_ref(), Some(funds.clone()))
            .unwrap();
        assert_eq!(
            msg.recipient.to_string(),
            format!("~{MOCK_APP_CONTRACT}/test")
        );
        assert_eq!(msg.message, Binary::default());
        assert_eq!(msg.funds, funds);
    }
}
