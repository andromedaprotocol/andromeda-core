use andromeda_fungible_tokens::exchange::{
    Cw20HookMsg, ExecuteMsg, InstantiateMsg, QueryMsg, RedeemResponse, SaleAssetsResponse,
    SaleResponse, TokenAddressResponse,
};
use andromeda_std::{
    ado_base::{InstantiateMsg as BaseInstantiateMsg, MigrateMsg},
    ado_contract::ADOContract,
    amp::{AndrAddr, Recipient},
    andr_execute_fn,
    common::context::ExecuteContext,
    common::denom::Asset,
    error::ContractError,
};
use cosmwasm_std::{
    ensure, entry_point, from_json, to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo, Reply,
    Response, StdError,
};
use cw20::Cw20ReceiveMsg;

use cw_storage_plus::Bound;

use crate::{
    execute_redeem::{
        execute_cancel_redeem, execute_redeem, execute_redeem_native, execute_replenish_redeem,
        execute_replenish_redeem_native, execute_start_redeem, execute_start_redeem_native,
    },
    execute_sale::{
        execute_cancel_sale, execute_purchase, execute_purchase_native, execute_start_sale,
    },
    state::{REDEEM, SALE, TOKEN_ADDRESS},
};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:andromeda-exchange";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    TOKEN_ADDRESS.save(deps.storage, &msg.token_address)?;

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
        ExecuteMsg::CancelSale { asset } => execute_cancel_sale(ctx, asset),
        ExecuteMsg::CancelRedeem { asset } => execute_cancel_redeem(ctx, asset),
        ExecuteMsg::Purchase { recipient } => execute_purchase_native(ctx, recipient),
        ExecuteMsg::StartRedeem {
            redeem_asset,
            exchange_rate,
            recipient,
            start_time,
            end_time,
        } => execute_start_redeem_native(
            ctx,
            redeem_asset,
            exchange_rate,
            recipient,
            start_time,
            end_time,
        ),
        ExecuteMsg::ReplenishRedeem { redeem_asset } => {
            execute_replenish_redeem_native(ctx, redeem_asset)
        }
        ExecuteMsg::Redeem { recipient } => execute_redeem_native(ctx, recipient),
        ExecuteMsg::Receive(cw20_msg) => execute_receive(ctx, cw20_msg),
        _ => ADOContract::default().execute(ctx, msg),
    }
}

pub fn execute_receive(
    ctx: ExecuteContext,
    receive_msg: Cw20ReceiveMsg,
) -> Result<Response, ContractError> {
    let ExecuteContext { ref info, .. } = ctx;
    let asset_sent = Asset::Cw20Token(AndrAddr::from_string(info.sender.clone()));
    let amount_sent = receive_msg.amount;
    let sender = receive_msg.sender;

    ensure!(
        !amount_sent.is_zero(),
        ContractError::InvalidFunds {
            msg: "Cannot send a 0 amount".to_string()
        }
    );

    match from_json(&receive_msg.msg)? {
        Cw20HookMsg::StartSale {
            asset,
            exchange_rate,
            recipient,
            start_time,
            duration,
        } => execute_start_sale(
            ctx,
            amount_sent,
            asset,
            exchange_rate,
            sender,
            recipient,
            start_time,
            duration,
        ),
        Cw20HookMsg::Purchase { recipient } => {
            let recipient = Recipient::validate_or_default(recipient, &ctx, sender.as_str())?;
            execute_purchase(ctx, amount_sent, asset_sent, recipient, &sender)
        }
        Cw20HookMsg::StartRedeem {
            redeem_asset,
            exchange_rate,
            recipient,
            start_time,
            end_time,
        } => execute_start_redeem(
            ctx,
            amount_sent,
            asset_sent,
            redeem_asset,
            exchange_rate,
            sender,
            recipient,
            start_time,
            end_time,
        ),
        Cw20HookMsg::ReplenishRedeem { redeem_asset } => {
            execute_replenish_redeem(ctx, amount_sent, asset_sent, redeem_asset)
        }
        Cw20HookMsg::Redeem { recipient } => {
            let recipient = Recipient::validate_or_default(recipient, &ctx, sender.as_str())?;
            execute_redeem(ctx, amount_sent, asset_sent, recipient, &sender)
        }
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    ADOContract::default().migrate(deps, env, CONTRACT_NAME, CONTRACT_VERSION)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
    match msg {
        QueryMsg::Sale { asset } => query_sale(deps, asset),
        QueryMsg::Redeem { asset } => query_redeem(deps, asset),
        QueryMsg::TokenAddress {} => query_token_address(deps),
        QueryMsg::SaleAssets { start_after, limit } => {
            query_sale_assets(deps, start_after.as_deref(), limit)
        }
        _ => ADOContract::default().query(deps, env, msg),
    }
}

fn query_sale(deps: Deps, asset: String) -> Result<Binary, ContractError> {
    let sale = SALE.may_load(deps.storage, &asset)?;
    Ok(to_json_binary(&SaleResponse { sale })?)
}

fn query_redeem(deps: Deps, asset: Asset) -> Result<Binary, ContractError> {
    let asset_str = asset.inner(&deps)?;
    let redeem = REDEEM.may_load(deps.storage, &asset_str)?;
    Ok(to_json_binary(&RedeemResponse { redeem })?)
}

fn query_token_address(deps: Deps) -> Result<Binary, ContractError> {
    let address = TOKEN_ADDRESS.load(deps.storage)?.get_raw_address(&deps)?;
    Ok(to_json_binary(&TokenAddressResponse {
        address: address.to_string(),
    })?)
}

const DEFAULT_LIMIT: u32 = 50;
const MAX_LIMIT: u32 = 100;

fn query_sale_assets(
    deps: Deps,
    start_after: Option<&str>,
    limit: Option<u32>,
) -> Result<Binary, ContractError> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let start = start_after.map(Bound::exclusive);

    let assets: Vec<String> = SALE
        .keys(deps.storage, start, None, cosmwasm_std::Order::Ascending)
        .take(limit)
        .collect::<Result<Vec<String>, StdError>>()?;

    Ok(to_json_binary(&SaleAssetsResponse { assets })?)
}
