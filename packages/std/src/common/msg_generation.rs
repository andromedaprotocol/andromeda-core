use cw_asset::AssetInfo;

use crate::error::ContractError;
use cosmwasm_std::{coin, wasm_execute, BankMsg, CosmosMsg, SubMsg, Uint128};
use cw20::Cw20ExecuteMsg;

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
