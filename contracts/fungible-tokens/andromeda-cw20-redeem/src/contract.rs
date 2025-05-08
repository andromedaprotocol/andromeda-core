use andromeda_fungible_tokens::cw20_redeem::{Cw20HookMsg, ExecuteMsg, InstantiateMsg, QueryMsg};
use andromeda_std::{
    ado_base::{InstantiateMsg as BaseInstantiateMsg, MigrateMsg},
    ado_contract::ADOContract,
    andr_execute_fn,
    common::{context::ExecuteContext, encode_binary},
    error::ContractError,
};
use cosmwasm_std::{
    ensure, entry_point, from_json, Binary, Deps, DepsMut, Env, MessageInfo, Reply, Response,
    StdError,
};
use cw20::Cw20ReceiveMsg;
use cw_asset::AssetInfo;

use crate::{
    execute::{
        execute_cancel_redemption_condition, execute_redeem_cw20, execute_redeem_native,
        execute_set_redemption_condition_cw20, execute_set_redemption_condition_native,
    },
    query::{query_redemption_asset, query_redemption_asset_balance, query_redemption_condition},
};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:andromeda-cw20-redeem";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    let contract = ADOContract::default();
    let resp = contract.instantiate(
        deps.storage,
        env,
        deps.api,
        &deps.querier,
        info,
        BaseInstantiateMsg {
            ado_type: CONTRACT_NAME.to_string(),
            ado_version: CONTRACT_VERSION.to_string(),
            kernel_address: msg.kernel_address,
            owner: msg.owner,
        },
    )?;

    Ok(resp)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(_deps: DepsMut, _env: Env, msg: Reply) -> Result<Response, ContractError> {
    if msg.result.is_err() {
        return Err(ContractError::Std(StdError::generic_err(
            msg.result.unwrap_err(),
        )));
    }

    Ok(Response::default())
}

#[andr_execute_fn]
pub fn execute(ctx: ExecuteContext, msg: ExecuteMsg) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Receive(cw20_msg) => execute_receive(ctx, cw20_msg),
        ExecuteMsg::Redeem {} => execute_redeem_native(ctx),
        ExecuteMsg::SetRedemptionCondition {
            redeemed_asset,
            exchange_rate,
            recipient,
            start_time,
            duration,
        } => execute_set_redemption_condition_native(
            ctx,
            redeemed_asset,
            exchange_rate,
            recipient,
            start_time,
            duration,
        ),
        ExecuteMsg::CancelRedemptionCondition {} => execute_cancel_redemption_condition(ctx),
        _ => ADOContract::default().execute(ctx, msg),
    }
}

pub fn execute_receive(
    ctx: ExecuteContext,
    receive_msg: Cw20ReceiveMsg,
) -> Result<Response, ContractError> {
    let ExecuteContext { ref info, .. } = ctx;

    let asset_info = AssetInfo::Cw20(info.sender.clone());
    let amount_sent = receive_msg.amount;
    let sender = receive_msg.sender;

    ensure!(
        !amount_sent.is_zero(),
        ContractError::InvalidFunds {
            msg: "Cannot send a 0 amount".to_string()
        }
    );

    match from_json(&receive_msg.msg)? {
        Cw20HookMsg::StartRedemptionCondition {
            redeemed_asset,
            exchange_rate,
            recipient,
            start_time,
            duration,
        } => execute_set_redemption_condition_cw20(
            ctx,
            amount_sent,
            asset_info,
            redeemed_asset,
            sender,
            exchange_rate,
            recipient,
            start_time,
            duration,
        ),
        Cw20HookMsg::Redeem {} => execute_redeem_cw20(ctx, amount_sent, asset_info, &sender),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    ADOContract::default().migrate(deps, env, CONTRACT_NAME, CONTRACT_VERSION)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
    match msg {
        QueryMsg::RedemptionCondition {} => encode_binary(&query_redemption_condition(deps)?),
        QueryMsg::RedemptionAsset {} => encode_binary(&query_redemption_asset(deps)?),
        QueryMsg::RedemptionAssetBalance {} => {
            encode_binary(&query_redemption_asset_balance(deps, env)?)
        }
        _ => ADOContract::default().query(deps, env, msg),
    }
}
