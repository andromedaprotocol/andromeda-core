use common::ado_base::query_get;
use common::app::GetAddress;
use common::{encode_binary, error::ContractError};
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Api, BankMsg, Binary, Coin, CosmosMsg, QuerierWrapper, SubMsg, WasmMsg};

use crate::kernel::ExecuteMsg as KernelExecuteMsg;
use crate::messages::{AMPMsg, AMPPkt, ExecuteMsg as AMPExecuteMsg};

#[cw_serde]
pub struct ADORecipient {
    /// Addr can also be a human-readable identifier used in a app contract.
    pub address: String,
    pub msg: Option<Binary>,
}

#[cw_serde]
pub enum AMPRecipient {
    /// An address that is not another ADO. It is assumed that it is a valid address.
    Addr(String),
    ADO(ADORecipient),
}

impl AMPRecipient {
    pub fn ado(address: impl Into<String>, msg: Option<Binary>) -> AMPRecipient {
        let ado_recipient = ADORecipient {
            address: address.into(),
            msg,
        };

        AMPRecipient::ADO(ado_recipient)
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

impl AMPRecipient {
    /// Creates an Addr AMPRecipient from the given string
    pub fn from_string(addr: String) -> AMPRecipient {
        AMPRecipient::Addr(addr)
    }

    /// Gets the address of the recipient. If the is an ADORecipient it will query the app
    /// contract to get its address if it fails address validation.
    pub fn get_addr(&self) -> Result<String, ContractError> {
        match &self {
            AMPRecipient::Addr(string) => Ok(string.to_owned()),
            AMPRecipient::ADO(recip) => Ok(recip.address.clone()),
        }
    }

    pub fn get_validated_addr(
        &self,
        api: &dyn Api,
        querier: &QuerierWrapper,
        app_contract: Option<Addr>,
    ) -> Result<String, ContractError> {
        match &self {
            AMPRecipient::Addr(string) => Ok(string.to_owned().get_address(api, querier, None)?),
            AMPRecipient::ADO(recip) => {
                Ok(recip
                    .address
                    .clone()
                    .get_address(api, querier, app_contract)?)
            }
        }
    }

    pub fn get_message(&self) -> Result<Option<Binary>, ContractError> {
        match &self {
            AMPRecipient::Addr(_string) => Ok(None),
            AMPRecipient::ADO(recip) => Ok(recip.msg.to_owned()),
        }
    }

    pub fn validate_address(
        &self,
        api: &dyn Api,
        querier: &QuerierWrapper,
        app_contract: Option<Addr>,
    ) -> Result<(), ContractError> {
        let address = self.get_addr()?;
        let addr = api.addr_validate(&address);
        match addr {
            Ok(_) => Ok(()),
            Err(_) => match app_contract {
                Some(app_contract) => {
                    query_get::<String>(
                        Some(encode_binary(&self)?),
                        app_contract.to_string(),
                        querier,
                    )?;
                    Ok(())
                }
                // TODO: Make error more descriptive.
                None => Err(ContractError::InvalidAddress {}),
            },
        }
    }

    /// Generates the sub message depending on the type of the recipient.
    pub fn generate_msg_native(
        &self,
        funds: Vec<Coin>,
        origin: String,
        previous_sender: String,
        messages: Vec<AMPMsg>,
        _kernel_address: String,
    ) -> Result<SubMsg, ContractError> {
        Ok(match &self {
            AMPRecipient::ADO(recip) => SubMsg::new(WasmMsg::Execute {
                contract_addr: recip.address.to_owned(),
                msg: encode_binary(&AMPExecuteMsg::AMPReceive(AMPPkt::new(
                    origin,
                    previous_sender,
                    messages,
                )))?,
                funds,
            }),
            AMPRecipient::Addr(addr) => SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
                to_address: addr.clone(),
                amount: funds,
            })),
        })
    }

    // /// Generates the sub message depending on the type of the recipient.
    // pub fn generate_msg_cw20(
    //     &self,
    //     cw20_coin: Cw20Coin,
    //     origin: String,
    //     previous_sender: String,
    //     messages: Vec<AMPMsg>,
    //     kernel_address: String,
    // ) -> Result<SubMsg, ContractError> {
    //     Ok(match &self {
    //         AMPRecipient::ADO(_recip) => SubMsg::new(WasmMsg::Execute {
    //             contract_addr: cw20_coin.address,
    //             msg: encode_binary(&Cw20ExecuteMsg::Send {
    //                 contract: self.get_addr()?,
    //                 amount: cw20_coin.amount,
    //                 msg: encode_binary(&ExecuteMsg::AndrReceive(AndromedaMsg::Receive(
    //                     recip.msg.clone(),
    //                 )))?,
    //             })?,
    //             funds: vec![],
    //         }),
    //         AMPRecipient::Addr(addr) => SubMsg::new(WasmMsg::Execute {
    //             contract_addr: cw20_coin.address,
    //             msg: encode_binary(&Cw20ExecuteMsg::Transfer {
    //                 recipient: addr.to_string(),
    //                 amount: cw20_coin.amount,
    //             })?,
    //             funds: vec![],
    //         }),
    //     })
    // }

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
