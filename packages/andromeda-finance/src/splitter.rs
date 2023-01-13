use amp::kernel::ExecuteMsg as KernelExecuteMsg;
use amp::messages::{AMPMsg, AMPPkt, MessagePath, ReplyGas};
use common::{
    ado_base::{modules::Module, recipient::Recipient, AndromedaMsg, AndromedaQuery},
    encode_binary,
    error::ContractError,
};
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{ensure, BankMsg, Binary, Coin, CosmosMsg, Decimal, ReplyOn, SubMsg, WasmMsg};
use cw_utils::Expiration;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[cw_serde]
pub struct AddressPercent {
    pub recipient: Recipient,
    pub percent: Decimal,
}

#[cw_serde]
pub struct UpdatedAddressPercent {
    pub recipient: UpdatedRecipient,
    pub percent: Decimal,
}

#[cw_serde]
/// A config struct for a `Splitter` contract.
pub struct Splitter {
    /// The vector of recipients for the contract. Anytime a `Send` execute message is sent the amount sent will be divided amongst these recipients depending on their assigned percentage.
    pub recipients: Vec<AddressPercent>,
    /// Whether or not the contract is currently locked. This restricts updating any config related fields.
    pub lock: Expiration,
}

#[cw_serde]
/// A config struct for a `Splitter` contract.
pub struct UpdatedSplitter {
    /// The vector of recipients for the contract. Anytime a `Send` execute message is sent the amount sent will be divided amongst these recipients depending on their assigned percentage.
    pub recipients: Vec<UpdatedAddressPercent>,
    /// Whether or not the contract is currently locked. This restricts updating any config related fields.
    pub lock: Expiration,
}

#[cw_serde]
pub struct UpdatedADORecipient {
    /// Addr can also be a human-readable identifier used in a app contract.
    pub address: String,
    pub msg: Option<Binary>,
}

#[cw_serde]
pub enum UpdatedRecipient {
    /// An address that is not another ADO. It is assumed that it is a valid address.
    Addr(String),
    ADO(UpdatedADORecipient),
}

impl UpdatedRecipient {
    /// Creates an Addr Recipient from the given string
    pub fn from_string(addr: String) -> UpdatedRecipient {
        UpdatedRecipient::Addr(addr)
    }

    /// Gets the address of the recipient. If the is an ADORecipient it will query the app
    /// contract to get its address if it fails address validation.
    pub fn updated_get_addr(&self) -> Result<String, ContractError> {
        match &self {
            UpdatedRecipient::Addr(string) => Ok(string.to_owned()),
            UpdatedRecipient::ADO(recip) => Ok(recip.address.clone()),
        }
    }

    pub fn updated_get_message(&self) -> Result<Option<Binary>, ContractError> {
        match &self {
            UpdatedRecipient::Addr(_string) => Ok(None),
            UpdatedRecipient::ADO(recip) => Ok(recip.msg.to_owned()),
        }
    }

    /// Generates the sub message depending on the type of the recipient.
    pub fn updated_generate_msg_native(
        &self,
        funds: Vec<Coin>,
        origin: String,
        previous_sender: String,
        messages: Vec<AMPMsg>,
        kernel_address: String,
    ) -> Result<SubMsg, ContractError> {
        Ok(match &self {
            UpdatedRecipient::ADO(recip) => SubMsg::new(WasmMsg::Execute {
                contract_addr: kernel_address,
                msg: encode_binary(&KernelExecuteMsg::Receive(AMPPkt::new(
                    origin,
                    previous_sender,
                    messages,
                )))?,
                funds,
            }),
            UpdatedRecipient::Addr(addr) => SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
                to_address: addr.clone(),
                amount: funds,
            })),
        })
    }

    // /// Generates the sub message depending on the type of the recipient.
    // pub fn generate_msg_cw20(&self, cw20_coin: Cw20Coin) -> Result<SubMsg, ContractError> {
    //     Ok(match &self {
    //         UpdatedRecipient::ADO(recip) => SubMsg::new(WasmMsg::Execute {
    //             contract_addr: cw20_coin.address,
    //             msg: encode_binary(&Cw20ExecuteMsg::Send {
    //                 contract: self.updated_get_addr()?,
    //                 amount: cw20_coin.amount,
    //                 msg: encode_binary(&ExecuteMsg::AndrReceive(AndromedaMsg::Receive(
    //                     recip.msg.clone(),
    //                 )))?,
    //             })?,
    //             funds: vec![],
    //         }),
    //         UpdatedRecipient::Addr(addr) => SubMsg::new(WasmMsg::Execute {
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

#[cw_serde]
pub struct InstantiateMsg {
    /// The vector of recipients for the contract. Anytime a `Send` execute message is
    /// sent the amount sent will be divided amongst these recipients depending on their assigned percentage.
    pub recipients: Vec<UpdatedAddressPercent>,
    pub lock_time: Option<u64>,
    pub modules: Option<Vec<Module>>,
    pub kernel_address: String,
}

impl InstantiateMsg {
    pub fn validate(&self) -> Result<bool, ContractError> {
        validate_recipient_list(self.recipients.clone())?;
        Ok(true)
    }
}

#[cw_serde]
pub enum ExecuteMsg {
    /// Update the recipients list. Only executable by the contract owner when the contract is not locked.
    UpdateRecipients {
        recipients: Vec<UpdatedAddressPercent>,
    },
    /// Used to lock/unlock the contract allowing the config to be updated.
    UpdateLock {
        lock_time: u64,
    },
    /// Divides any attached funds to the message amongst the recipients list.
    Send {},

    SendKernel {
        reply_gas: ReplyGas,
    },

    AndrReceive(AndromedaMsg),

    Receive(MessagePath),
}

#[cw_serde]
#[serde(rename_all = "snake_case")]
pub struct MigrateMsg {}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(AndromedaQuery)]
    AndrQuery(AndromedaQuery),
    /// The current config of the Splitter contract
    #[returns(GetSplitterConfigResponse)]
    GetSplitterConfig {},

    #[returns(UpdatedGetSplitterConfigResponse)]
    UpdatedGetSplitterConfig {},
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema)]
pub struct GetSplitterConfigResponse {
    pub config: Splitter,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema)]
pub struct UpdatedGetSplitterConfigResponse {
    pub config: UpdatedSplitter,
}

/// Ensures that a given list of recipients for a `splitter` contract is valid:
///
/// * Must include at least one recipient
/// * The combined percentage of the recipients must not exceed 100
pub fn validate_recipient_list(
    recipients: Vec<UpdatedAddressPercent>,
) -> Result<bool, ContractError> {
    ensure!(
        !recipients.is_empty(),
        ContractError::EmptyRecipientsList {}
    );

    let mut percent_sum: Decimal = Decimal::zero();
    for rec in recipients {
        // += operation is not supported for decimal.
        percent_sum += rec.percent;
    }

    ensure!(
        percent_sum <= Decimal::one(),
        ContractError::AmountExceededHundredPrecent {}
    );

    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_recipient_list() {
        let empty_recipients = vec![];
        let res = validate_recipient_list(empty_recipients).unwrap_err();
        assert_eq!(res, ContractError::EmptyRecipientsList {});

        let inadequate_recipients = vec![UpdatedAddressPercent {
            recipient: UpdatedRecipient::from_string(String::from("Some Address")),
            percent: Decimal::percent(150),
        }];
        let res = validate_recipient_list(inadequate_recipients).unwrap_err();
        assert_eq!(res, ContractError::AmountExceededHundredPrecent {});

        let valid_recipients = vec![
            UpdatedAddressPercent {
                recipient: UpdatedRecipient::from_string(String::from("Some Address")),
                percent: Decimal::percent(50),
            },
            UpdatedAddressPercent {
                recipient: UpdatedRecipient::from_string(String::from("Some Address")),
                percent: Decimal::percent(50),
            },
        ];

        let res = validate_recipient_list(valid_recipients).unwrap();
        assert!(res);
    }
}
