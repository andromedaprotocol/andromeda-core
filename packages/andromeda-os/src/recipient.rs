use common::{encode_binary, error::ContractError};
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, BankMsg, Binary, Coin, CosmosMsg, QuerierWrapper, SubMsg, WasmMsg};
use cw20::{Cw20Coin, Cw20ExecuteMsg};

use crate::kernel::ExecuteMsg as KernelExecuteMsg;
use crate::messages::{AMPMsg, AMPPkt};
use crate::vfs::vfs_resolve_path;

#[cw_serde]
pub struct Recipient {
    /// Addr can also be a human-readable identifier used in a app contract.
    pub address: String,
    pub msg: Option<Binary>,
}

impl Recipient {
    /// Creates an Addr Recipient from the given string
    pub fn from_string(addr: impl Into<String>) -> Recipient {
        Recipient {
            address: addr.into(),
            msg: None,
        }
    }

    /// Gets the address of the recipient. If the is an ADORecipient it will query the app
    /// contract to get its address if it fails address validation.
    pub fn get_addr(&self) -> String {
        self.address.clone()
    }

    pub fn get_resolved_address(
        &self,
        querier: &QuerierWrapper,
        vfs_contract: Option<Addr>,
    ) -> Result<Addr, ContractError> {
        match vfs_contract {
            None => Err(ContractError::VFSContractNotSpecified {}),
            Some(addr) => vfs_resolve_path(self.address.clone(), addr, querier),
        }
    }

    pub fn get_message(&self) -> Option<Binary> {
        self.msg.clone()
    }

    /// Generates a new AMP Packet for the recipient with the attached message
    pub fn generate_direct_msg(
        &self,
        querier: &QuerierWrapper,
        vfs_contract: Option<Addr>,
        funds: Vec<Coin>,
    ) -> Result<SubMsg, ContractError> {
        let resolved_addr = self.get_resolved_address(querier, vfs_contract)?;
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
    /// Generates the sub message depending on the type of the recipient.
    pub fn generate_msg_cw20(
        &self,
        querier: &QuerierWrapper,
        vfs_contract: Option<Addr>,
        cw20_coin: Cw20Coin,
    ) -> Result<SubMsg, ContractError> {
        let resolved_addr = self.get_resolved_address(querier, vfs_contract)?;
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

    // pub fn generate_msg_from_asset(
    //     &self,
    //     api: &dyn Api,
    //     querier: &QuerierWrapper,
    //     app_contract: Option<Addr>,
    //     asset: AssetInfo,
    //     funds: Vec<Coin>,
    // ) -> Result<SubMsg, ContractError> {
    //     match asset {
    //         AssetInfoBase::Cw20(ref contract_addr) => self.generate_msg_cw20(
    //             api,
    //             querier,
    //             app_contract,
    //             Cw20Coin {
    //                 address: contract_addr.to_string(),
    //                 amount: asset.query_balance(querier, contract_addr)?,
    //             },
    //         ),
    //         AssetInfoBase::Native(_denom) => {
    //             self.generate_msg_native(api, querier, app_contract, funds)
    //         }
    //         _ => Err(ContractError::InvalidAsset {
    //             asset: asset.to_string(),
    //         }),
    //     }
    // }
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
