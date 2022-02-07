use crate::error::ContractError;
use crate::modules::{hooks::HookResponse, Module};
use cosmwasm_std::{Coin, DepsMut, Env, MessageInfo, StdResult};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use terraswap::asset::{Asset, AssetInfo};

//Redundant? Can maybe use `Modules` struct?
pub fn generate_instantiate_msgs(
    deps: &DepsMut,
    info: MessageInfo,
    env: Env,
    modules: Vec<Option<impl Module>>,
) -> Result<HookResponse, ContractError> {
    let mut resp = HookResponse::default();

    for module in modules.into_iter().flatten() {
        let hook_resp = module.on_instantiate(deps, info.clone(), env.clone())?;
        resp = resp.add_resp(hook_resp);
    }

    Ok(resp)
}

pub fn unwrap_or_err<T>(val_opt: Option<T>, err: ContractError) -> Result<T, ContractError> {
    match val_opt {
        Some(val) => Ok(val),
        None => Err(err),
    }
}

pub fn get_tax_deducted_funds(deps: &DepsMut, coins: Vec<Coin>) -> StdResult<Vec<Coin>> {
    let mut tax_deducted_coins = vec![];
    for coin in coins.iter() {
        let asset = Asset {
            info: AssetInfo::NativeToken {
                denom: coin.denom.to_string(),
            },
            amount: coin.amount,
        };
        tax_deducted_coins.push(asset.deduct_tax(&deps.querier)?);
    }
    Ok(tax_deducted_coins)
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum OrderBy {
    Asc,
    Desc,
}
