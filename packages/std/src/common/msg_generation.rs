use cw_asset::AssetInfo;

use crate::{amp::Recipient, error::ContractError};
use cosmwasm_std::{coin, wasm_execute, BankMsg, CosmosMsg, Deps, ReplyOn, SubMsg, Uint128};
use cw20::{Cw20Coin, Cw20ExecuteMsg};

/// Used in CW20 Redeem and CW20 Exchange
/// Generates a transfer message given an asset and an amount
pub fn generate_transfer_message(
    asset: AssetInfo,
    amount: Uint128,
    recipient: String,
    reply_id: Option<u64>,
) -> Result<SubMsg, ContractError> {
    match asset.clone() {
        AssetInfo::Native(denom) => {
            let bank_msg = BankMsg::Send {
                to_address: recipient,
                amount: vec![coin(amount.u128(), denom)],
            };

            let cosmos_msg = CosmosMsg::Bank(bank_msg);
            Ok(if let Some(id) = reply_id {
                SubMsg::reply_on_error(cosmos_msg, id)
            } else {
                SubMsg::new(cosmos_msg)
            })
        }
        AssetInfo::Cw20(addr) => {
            let transfer_msg = Cw20ExecuteMsg::Transfer { recipient, amount };
            let wasm_msg = wasm_execute(addr, &transfer_msg, vec![])?;
            Ok(if let Some(id) = reply_id {
                SubMsg::reply_on_error(CosmosMsg::Wasm(wasm_msg), id)
            } else {
                SubMsg::new(CosmosMsg::Wasm(wasm_msg))
            })
        }
        // Does not support 1155 currently
        _ => Err(ContractError::InvalidAsset {
            asset: asset.to_string(),
        }),
    }
}

pub fn generate_transfer_message_recipient(
    deps: &Deps,
    asset: AssetInfo,
    amount: Uint128,
    recipient: Recipient,
    reply_id: Option<u64>,
) -> Result<SubMsg, ContractError> {
    match asset.clone() {
        AssetInfo::Native(denom) => {
            let mut msg = recipient.generate_direct_msg(deps, vec![coin(amount.u128(), denom)])?;

            Ok(if let Some(id) = reply_id {
                msg.reply_on = ReplyOn::Error;
                msg.id = id;
                msg
            } else {
                msg
            })
        }
        AssetInfo::Cw20(addr) => {
            // let transfer_msg = Cw20ExecuteMsg::Transfer { recipient, amount };
            // let wasm_msg = wasm_execute(addr, &transfer_msg, vec![])?;
            let mut msg = recipient.generate_msg_cw20(
                deps,
                Cw20Coin {
                    address: addr.to_string(),
                    amount,
                },
            )?;
            Ok(if let Some(id) = reply_id {
                msg.reply_on = ReplyOn::Error;
                msg.id = id;
                msg
            } else {
                msg
            })
        }
        // Does not support 1155 currently
        _ => Err(ContractError::InvalidAsset {
            asset: asset.to_string(),
        }),
    }
}
