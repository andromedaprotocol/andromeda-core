use andromeda_fungible_tokens::cw20_exchange::{
    Cw20HookMsg, ExecuteMsg, InstantiateMsg, QueryMsg, Sale, SaleAssetsResponse, SaleResponse,
    TokenAddressResponse,
};
use andromeda_std::{
    ado_base::{InstantiateMsg as BaseInstantiateMsg, MigrateMsg},
    ado_contract::ADOContract,
    andr_execute_fn,
    common::{
        context::ExecuteContext, expiration::Expiry, msg_generation::generate_transfer_message,
        Milliseconds, MillisecondsDuration,
    },
    error::ContractError,
};
use cosmwasm_std::{
    attr, ensure, entry_point, from_json, to_json_binary, wasm_execute, Binary, CosmosMsg, Deps,
    DepsMut, Env, MessageInfo, Reply, Response, StdError, SubMsg, Uint128,
};
use cw20::{Cw20ExecuteMsg, Cw20ReceiveMsg};
use cw_asset::AssetInfo;
use cw_storage_plus::Bound;
use cw_utils::one_coin;

use crate::state::{SALE, TOKEN_ADDRESS};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:andromeda-cw20-exchange";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

/// ID used for any refund sub messgaes
const REFUND_REPLY_ID: u64 = 1;
/// ID used for any purchased token transfer sub messages
const PURCHASE_REPLY_ID: u64 = 2;
/// ID used for transfer to sale recipient
const RECIPIENT_REPLY_ID: u64 = 3;

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
        ExecuteMsg::Purchase { recipient } => execute_purchase_native(ctx, recipient),
        ExecuteMsg::Receive(cw20_msg) => execute_receive(ctx, cw20_msg),
        _ => ADOContract::default().execute(ctx, msg),
    }
}

pub fn execute_receive(
    ctx: ExecuteContext,
    receive_msg: Cw20ReceiveMsg,
) -> Result<Response, ContractError> {
    let ExecuteContext { ref info, .. } = ctx;
    let asset_sent = AssetInfo::Cw20(info.sender.clone());
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
        Cw20HookMsg::Purchase { recipient } => execute_purchase(
            ctx,
            amount_sent,
            asset_sent,
            recipient.unwrap_or_else(|| sender.to_string()).as_str(),
            &sender,
        ),
    }
}

#[allow(clippy::too_many_arguments)]
pub fn execute_start_sale(
    ctx: ExecuteContext,
    amount: Uint128,
    asset: AssetInfo,
    exchange_rate: Uint128,
    // The original sender of the CW20::Send message
    sender: String,
    // The recipient of the sale proceeds
    recipient: Option<String>,
    start_time: Option<Expiry>,
    duration: Option<MillisecondsDuration>,
) -> Result<Response, ContractError> {
    let ExecuteContext {
        deps, env, info, ..
    } = ctx;

    let token_addr = TOKEN_ADDRESS
        .load(deps.storage)?
        .get_raw_address(&deps.as_ref())?;

    ensure!(
        asset != AssetInfo::Cw20(token_addr.clone()),
        ContractError::InvalidAsset {
            asset: asset.to_string()
        }
    );
    ensure!(
        !exchange_rate.is_zero(),
        ContractError::InvalidZeroAmount {}
    );
    ensure!(
        ctx.contract.is_contract_owner(deps.storage, &sender)?,
        ContractError::Unauthorized {}
    );
    // Message sender in this case should be the token address
    ensure!(
        info.sender == token_addr,
        ContractError::InvalidFunds {
            msg: "Incorrect CW20 provided for sale".to_string()
        }
    );

    let start_time = match start_time {
        Some(s) => {
            // Check that the start time is in the future
            s.validate(&env.block)?
        }
        // Set start time to current time if not provided
        None => Expiry::FromNow(Milliseconds::zero()),
    }
    .get_time(&env.block);

    let end_time = match duration {
        Some(e) => {
            if e.is_zero() {
                // If duration is 0, set end time to none
                None
            } else {
                // Set end time to current time + duration
                Some(Expiry::FromNow(e).get_time(&env.block))
            }
        }
        None => None,
    };

    // Do not allow duplicate sales
    let current_sale = SALE.may_load(deps.storage, &asset.to_string())?;
    ensure!(current_sale.is_none(), ContractError::SaleNotEnded {});

    let sale = Sale {
        amount,
        exchange_rate,
        recipient: recipient.unwrap_or(sender),
        start_time,
        end_time,
    };
    SALE.save(deps.storage, &asset.to_string(), &sale)?;

    Ok(Response::default().add_attributes(vec![
        attr("action", "start_sale"),
        attr("asset", asset.to_string()),
        attr("rate", exchange_rate),
        attr("amount", amount),
        attr("start_time", start_time.to_string()),
        attr("end_time", end_time.unwrap_or_default().to_string()),
    ]))
}

pub fn execute_purchase(
    ctx: ExecuteContext,
    amount_sent: Uint128,
    asset_sent: AssetInfo,
    recipient: &str,
    // For refund purposes
    sender: &str,
) -> Result<Response, ContractError> {
    let ExecuteContext { deps, .. } = ctx;
    deps.api.addr_validate(recipient)?;
    let mut resp = Response::default();

    let Some(mut sale) = SALE.may_load(deps.storage, &asset_sent.to_string())? else {
        return Err(ContractError::NoOngoingSale {});
    };

    // Check if sale has started
    ensure!(
        sale.start_time.is_expired(&ctx.env.block),
        ContractError::SaleNotStarted {}
    );
    // Check if sale has ended
    if let Some(end_time) = sale.end_time {
        ensure!(
            !end_time.is_expired(&ctx.env.block),
            ContractError::SaleEnded {}
        );
    }

    let purchased = amount_sent.checked_div(sale.exchange_rate).unwrap();
    let remainder = amount_sent.checked_sub(purchased.checked_mul(sale.exchange_rate)?)?;

    ensure!(
        !purchased.is_zero(),
        ContractError::InvalidFunds {
            msg: "Not enough funds sent to purchase a token".to_string()
        }
    );
    ensure!(sale.amount >= purchased, ContractError::NotEnoughTokens {});

    // If purchase was rounded down return funds to purchaser
    if !remainder.is_zero() {
        resp = resp
            .add_submessage(generate_transfer_message(
                asset_sent.clone(),
                remainder,
                sender.to_string(),
                Some(REFUND_REPLY_ID),
            )?)
            .add_attribute("refunded_amount", remainder);
    }

    // Transfer tokens to purchaser recipient
    let token_addr = TOKEN_ADDRESS
        .load(deps.storage)?
        .get_raw_address(&deps.as_ref())?;
    let transfer_msg = Cw20ExecuteMsg::Transfer {
        recipient: recipient.to_string(),
        amount: purchased,
    };
    let wasm_msg = wasm_execute(token_addr, &transfer_msg, vec![])?;
    resp = resp.add_submessage(SubMsg::reply_on_error(
        CosmosMsg::Wasm(wasm_msg),
        PURCHASE_REPLY_ID,
    ));

    // Update sale amount remaining
    sale.amount = sale.amount.checked_sub(purchased)?;
    SALE.save(deps.storage, &asset_sent.to_string(), &sale)?;

    // Transfer exchanged asset to recipient
    resp = resp.add_submessage(generate_transfer_message(
        asset_sent.clone(),
        amount_sent - remainder,
        sale.recipient.clone(),
        Some(RECIPIENT_REPLY_ID),
    )?);

    Ok(resp.add_attributes(vec![
        attr("action", "purchase"),
        attr("purchaser", sender),
        attr("recipient", recipient),
        attr("amount", purchased),
        attr("purchase_asset", asset_sent.to_string()),
        attr("purchase_asset_amount_send", amount_sent - remainder),
        attr("recipient", sale.recipient),
    ]))
}

pub fn execute_purchase_native(
    ctx: ExecuteContext,
    recipient: Option<String>,
) -> Result<Response, ContractError> {
    let ExecuteContext {
        ref deps, ref info, ..
    } = ctx;

    // Default to sender as recipient
    let recipient = recipient.unwrap_or_else(|| info.sender.to_string());
    deps.api.addr_validate(&recipient)?;
    let sender = info.sender.to_string();

    // Only allow one coin for purchasing
    let payment = one_coin(info)?;
    let asset = AssetInfo::Native(payment.denom.to_string());
    let amount = payment.amount;

    execute_purchase(ctx, amount, asset, &recipient, &sender)
}

pub fn execute_cancel_sale(
    ctx: ExecuteContext,
    asset: AssetInfo,
) -> Result<Response, ContractError> {
    let ExecuteContext { deps, info, .. } = ctx;

    let Some(sale) = SALE.may_load(deps.storage, &asset.to_string())? else {
        return Err(ContractError::NoOngoingSale {});
    };

    let mut resp = Response::default();

    // Refund any remaining amount
    if !sale.amount.is_zero() {
        let token_addr = TOKEN_ADDRESS
            .load(deps.storage)?
            .get_raw_address(&deps.as_ref())?;

        let token = AssetInfo::Cw20(token_addr);
        resp = resp
            .add_submessage(generate_transfer_message(
                token,
                sale.amount,
                info.sender.to_string(),
                Some(REFUND_REPLY_ID),
            )?)
            .add_attribute("refunded_amount", sale.amount);
    }

    // Sale can now be removed
    SALE.remove(deps.storage, &asset.to_string());

    Ok(resp.add_attributes(vec![
        attr("action", "cancel_sale"),
        attr("asset", asset.to_string()),
    ]))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    ADOContract::default().migrate(deps, env, CONTRACT_NAME, CONTRACT_VERSION)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
    match msg {
        QueryMsg::Sale { asset } => query_sale(deps, asset),
        QueryMsg::TokenAddress {} => query_token_address(deps),
        QueryMsg::SaleAssets { start_after, limit } => {
            query_sale_assets(deps, start_after.as_deref(), limit)
        }
        _ => ADOContract::default().query(deps, env, msg),
    }
}

fn query_sale(deps: Deps, asset: impl ToString) -> Result<Binary, ContractError> {
    let sale = SALE.may_load(deps.storage, &asset.to_string())?;

    Ok(to_json_binary(&SaleResponse { sale })?)
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
