use cosmwasm_std::{Coin, DepsMut, Response, StdResult};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use terraswap::asset::{Asset, AssetInfo};

pub fn merge_responses(resp_a: Response, resp_b: Response) -> Response {
    resp_a
        .add_attributes(resp_b.attributes)
        .add_submessages(resp_b.messages)
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
