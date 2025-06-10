use andromeda_socket::astroport::{AssetEntry, AssetInfo, PairExecuteMsg};
use andromeda_std::{common::denom::Asset, error::ContractError};
use cosmwasm_std::{wasm_execute, Deps, Env, Uint128};
use cw20::{BalanceResponse, Cw20QueryMsg};

pub const ASTROPORT_MSG_SWAP_ID: u64 = 1;
pub const ASTROPORT_MSG_FORWARD_ID: u64 = 2;
pub const ASTROPORT_MSG_CREATE_PAIR_ID: u64 = 3;
pub const ASTROPORT_MSG_CREATE_PAIR_AND_PROVIDE_LIQUIDITY_ID: u64 = 4;
pub const ASTROPORT_MSG_PROVIDE_LIQUIDITY_ID: u64 = 5;
pub const ASTROPORT_MSG_WITHDRAW_LIQUIDITY_ID: u64 = 6;

#[derive(Clone, Debug, PartialEq)]
pub struct AstroportSwapResponse {
    pub spread_amount: Uint128, // remaining Asset that is not consumed by the swap operation
    pub return_amount: Uint128, // amount of token_out swapped from astroport
}

pub fn generate_asset_info_from_asset(
    deps: &Deps,
    asset: Asset,
) -> Result<AssetInfo, ContractError> {
    match asset {
        Asset::Cw20Token(andr_addr) => {
            let contract_addr = andr_addr.get_raw_address(deps)?;
            Ok(AssetInfo::Token { contract_addr })
        }
        Asset::NativeToken(denom) => Ok(AssetInfo::NativeToken { denom }),
    }
}

pub(crate) fn query_balance(
    deps: &Deps,
    env: &Env,
    asset: &Asset,
) -> Result<Uint128, ContractError> {
    let balance = match &asset {
        Asset::Cw20Token(andr_addr) => {
            let contract_addr = andr_addr.get_raw_address(deps)?;
            let res: BalanceResponse = deps.querier.query_wasm_smart(
                contract_addr,
                &Cw20QueryMsg::Balance {
                    address: env.contract.address.to_string(),
                },
            )?;
            res.balance
        }
        Asset::NativeToken(denom) => {
            deps.querier
                .query_balance(env.contract.address.to_string(), denom)?
                .amount
        }
    };
    Ok(balance)
}

/// Handles both native coins and CW20 token allowances
pub fn build_liquidity_messages(
    assets: &[AssetEntry],
    pair_addr: impl Into<String>,
    provide_liquidity_msg: PairExecuteMsg,
) -> Result<Vec<cosmwasm_std::WasmMsg>, ContractError> {
    let mut native_coins = vec![];
    let mut msgs = vec![];
    let pair_addr_str = pair_addr.into();

    for asset in assets {
        match &asset.info {
            AssetInfo::NativeToken { denom } => {
                native_coins.push(cosmwasm_std::Coin {
                    denom: denom.clone(),
                    amount: asset.amount,
                });
            }
            AssetInfo::Token { contract_addr } => {
                let allowance_msg = cw20::Cw20ExecuteMsg::IncreaseAllowance {
                    spender: pair_addr_str.clone(),
                    amount: asset.amount,
                    expires: None,
                };
                let msg = wasm_execute(contract_addr, &allowance_msg, vec![])?;
                msgs.push(msg);
            }
        }
    }

    let liquidity_msg = wasm_execute(pair_addr_str, &provide_liquidity_msg, native_coins)?;
    msgs.push(liquidity_msg);

    Ok(msgs)
}
