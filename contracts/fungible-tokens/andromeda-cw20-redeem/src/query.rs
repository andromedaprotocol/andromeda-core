use andromeda_fungible_tokens::cw20_redeem::{RedemptionAssetResponse, RedemptionResponse};
use andromeda_std::error::ContractError;
use cosmwasm_std::{to_json_binary, Deps, Env, Uint128};
use cw20::{BalanceResponse, Cw20QueryMsg};
use cw_asset::AssetInfo;

use crate::state::REDEMPTION_CONDITION;

pub fn query_redemption_condition(deps: Deps) -> Result<RedemptionResponse, ContractError> {
    let redemption = REDEMPTION_CONDITION.may_load(deps.storage)?;

    Ok(RedemptionResponse { redemption })
}

pub fn query_redemption_asset(deps: Deps) -> Result<RedemptionAssetResponse, ContractError> {
    let redemption_condition = REDEMPTION_CONDITION.load(deps.storage)?;

    Ok(RedemptionAssetResponse {
        asset: redemption_condition.asset.to_string(),
    })
}

pub fn query_redemption_asset_balance(deps: Deps, env: Env) -> Result<Uint128, ContractError> {
    let asset = REDEMPTION_CONDITION.load(deps.storage)?.asset;

    match asset {
        AssetInfo::Native(denom) => {
            let balance = deps.querier.query_balance(env.contract.address, denom)?;
            Ok(balance.amount)
        }
        AssetInfo::Cw20(addr) => {
            let balance_msg = Cw20QueryMsg::Balance {
                address: env.contract.address.into(),
            };
            let balance_response: BalanceResponse = deps
                .querier
                .query_wasm_smart(addr, &to_json_binary(&balance_msg)?)?;
            Ok(balance_response.balance)
        }
        // Does not support 1155 currently
        _ => Err(ContractError::InvalidAsset {
            asset: asset.to_string(),
        }),
    }
}
