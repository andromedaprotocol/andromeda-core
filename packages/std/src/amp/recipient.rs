use crate::{encode_binary, error::ContractError};
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{BankMsg, Binary, Coin, CosmosMsg, Deps, SubMsg, WasmMsg};
use cw20::{Cw20Coin, Cw20ExecuteMsg};

use crate::amp::messages::{AMPMsg, AMPPkt};
use crate::os::kernel::ExecuteMsg as KernelExecuteMsg;

use super::addresses::AndrAddr;

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
}

impl Recipient {
    pub fn new(addr: impl Into<String>, msg: Option<Binary>) -> Recipient {
        Recipient {
            address: AndrAddr::from_string(addr),
            msg,
        }
    }

    /// Creates a Recipient from the given string with no attached message
    pub fn from_string(addr: impl Into<String>) -> Recipient {
        Recipient {
            address: AndrAddr::from_string(addr.into()),
            msg: None,
        }
    }

    pub fn get_addr(&self) -> String {
        self.address.to_string()
    }

    pub fn get_message(&self) -> Option<Binary> {
        self.msg.clone()
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
    pub fn generate_amp_msg(&self, funds: Option<Vec<Coin>>) -> AMPMsg {
        AMPMsg::new(
            self.address.to_string(),
            self.msg.clone().unwrap_or_default(),
            funds,
            None,
            None,
            None,
        )
    }
}

pub fn generate_msg_native_kernel(
    funds: Vec<Coin>,
    origin: String,
    previous_sender: String,
    messages: Vec<AMPMsg>,
    kernel_address: String,
) -> Result<SubMsg, ContractError> {
    Ok(SubMsg::new(WasmMsg::Execute {
        contract_addr: kernel_address,
        msg: encode_binary(&KernelExecuteMsg::AMPReceive(AMPPkt::new(
            origin,
            previous_sender,
            messages,
        )))?,
        funds,
    }))
}
