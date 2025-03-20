use andromeda_fungible_tokens::cw20_redeem::{
    Cw20HookMsg, ExecuteMsg, InstantiateMsg, QueryMsg, RedemptionAssetResponse, RedemptionClause,
    RedemptionResponse, TokenAddressResponse,
};
use andromeda_std::{
    ado_base::{InstantiateMsg as BaseInstantiateMsg, MigrateMsg},
    ado_contract::ADOContract,
    andr_execute_fn,
    common::{
        context::ExecuteContext,
        expiration::{expiration_from_milliseconds, get_and_validate_start_time, Expiry},
        Milliseconds, MillisecondsDuration,
    },
    error::ContractError,
};
use cosmwasm_std::{
    attr, coin, ensure, entry_point, from_json, to_json_binary, wasm_execute, BankMsg, Binary,
    CosmosMsg, Deps, DepsMut, Env, MessageInfo, Reply, Response, StdError, SubMsg, Uint128,
};
use cw20::{Cw20ExecuteMsg, Cw20ReceiveMsg};
use cw_asset::AssetInfo;
use cw_utils::{one_coin, Expiration};

use crate::state::{REDEMPTION_CLAUSE, TOKEN_ADDRESS};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:andromeda-cw20-redeem";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

/// ID used for any refund sub messgaes
const REFUND_REPLY_ID: u64 = 1;
/// ID used for any purchased token transfer sub messages
// const PURCHASE_REPLY_ID: u64 = 2;
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
        ExecuteMsg::Receive(cw20_msg) => execute_receive(ctx, cw20_msg),
        ExecuteMsg::SetRedemptionClause {
            exchange_rate,
            start_time,
            duration,
        } => execute_set_redemption_clause_native(ctx, exchange_rate, start_time, duration),
        ExecuteMsg::CancelRedemptionClause {} => execute_cancel_redemption_clause(ctx),
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
        Cw20HookMsg::StartRedemptionClause {
            exchange_rate,
            start_time,
            duration,
        } => execute_set_redemption_clause_cw20(
            ctx,
            amount_sent,
            asset_sent,
            sender,
            exchange_rate,
            start_time,
            duration,
        ),
        Cw20HookMsg::Redeem {} => execute_redeem(ctx, amount_sent, asset_sent, &sender),
    }
}

#[allow(clippy::too_many_arguments)]
pub fn execute_set_redemption_clause_cw20(
    ctx: ExecuteContext,
    amount_sent: Uint128,
    asset_sent: AssetInfo,
    sender: String,
    exchange_rate: Uint128,
    start_time: Option<Expiry>,
    duration: Option<MillisecondsDuration>,
) -> Result<Response, ContractError> {
    let ExecuteContext { deps, env, .. } = ctx;

    ensure!(
        !exchange_rate.is_zero(),
        ContractError::InvalidZeroAmount {}
    );

    // Check if the creator of the redemption clause is either the cw20 contract owner or admin
    let cw20_address = TOKEN_ADDRESS
        .load(deps.storage)?
        .get_raw_address(&deps.as_ref())?;
    let contract_info = deps
        .querier
        .query_wasm_contract_info(cw20_address.clone())?;
    let cw20_owner = contract_info.creator;

    ensure!(
        cw20_owner == sender || contract_info.admin == Some(sender.to_string()),
        ContractError::Unauthorized {}
    );

    // If start time wasn't provided, it will be set as the current_time
    let (start_expiration, _current_time) = get_and_validate_start_time(&env, start_time.clone())?;

    let end_expiration = if let Some(duration) = duration {
        ensure!(!duration.is_zero(), ContractError::InvalidExpiration {});
        expiration_from_milliseconds(
            start_time
                // If start time isn't provided, it is set one second in advance from the current time
                .unwrap_or(Expiry::FromNow(Milliseconds::from_seconds(1)))
                .get_time(&env.block)
                .plus_milliseconds(duration),
        )?
    } else {
        Expiration::Never {}
    };

    // Do not allow duplicate sales
    let redemption_clause = REDEMPTION_CLAUSE.may_load(deps.storage)?;
    ensure!(
        redemption_clause.is_none(),
        ContractError::RedemptionClauseAlreadyExists {}
    );

    let redemption_clause = RedemptionClause {
        recipient: sender.to_string(),
        asset: asset_sent.clone(),
        amount: amount_sent,
        exchange_rate,
        start_time: start_expiration,
        end_time: end_expiration,
    };
    REDEMPTION_CLAUSE.save(deps.storage, &redemption_clause)?;

    Ok(Response::default().add_attributes(vec![
        attr("action", "start_redemption_clause"),
        attr("asset", asset_sent.to_string()),
        attr("rate", exchange_rate),
        attr("amount", amount_sent),
        attr("start_time", start_expiration.to_string()),
        attr("end_time", end_expiration.to_string()),
    ]))
}

#[allow(clippy::too_many_arguments)]
pub fn execute_set_redemption_clause_native(
    ctx: ExecuteContext,
    exchange_rate: Uint128,
    start_time: Option<Expiry>,
    duration: Option<MillisecondsDuration>,
) -> Result<Response, ContractError> {
    let ExecuteContext {
        deps, env, info, ..
    } = ctx;

    let payment = one_coin(&info)?;

    let asset = AssetInfo::Native(payment.denom.to_string());
    let amount = payment.amount;
    ensure!(
        !amount.is_zero(),
        ContractError::InvalidFunds {
            msg: "Cannot send a 0 amount".to_string()
        }
    );

    ensure!(
        !exchange_rate.is_zero(),
        ContractError::InvalidZeroAmount {}
    );

    // Check if the creator of the redemption clause is either the cw20 contract owner or admin
    let cw20_address = TOKEN_ADDRESS
        .load(deps.storage)?
        .get_raw_address(&deps.as_ref())?;
    let contract_info = deps
        .querier
        .query_wasm_contract_info(cw20_address.clone())?;
    let cw20_owner = contract_info.creator;

    println!("cw20_owner: {}", cw20_owner);
    println!("sender: {}", info.sender);
    ensure!(
        cw20_owner == info.sender || contract_info.admin == Some(info.sender.to_string()),
        ContractError::Unauthorized {}
    );

    // If start time wasn't provided, it will be set as the current_time
    let (start_expiration, _current_time) = get_and_validate_start_time(&env, start_time.clone())?;

    let end_expiration = if let Some(duration) = duration {
        ensure!(!duration.is_zero(), ContractError::InvalidExpiration {});
        expiration_from_milliseconds(
            start_time
                // If start time isn't provided, it is set one second in advance from the current time
                .unwrap_or(Expiry::FromNow(Milliseconds::from_seconds(1)))
                .get_time(&env.block)
                .plus_milliseconds(duration),
        )?
    } else {
        Expiration::Never {}
    };

    // Do not allow duplicate sales
    let redemption_clause = REDEMPTION_CLAUSE.may_load(deps.storage)?;
    ensure!(
        redemption_clause.is_none(),
        ContractError::RedemptionClauseAlreadyExists {}
    );

    let redemption_clause = RedemptionClause {
        recipient: info.sender.to_string(),
        asset: asset.clone(),
        amount,
        exchange_rate,
        start_time: start_expiration,
        end_time: end_expiration,
    };
    REDEMPTION_CLAUSE.save(deps.storage, &redemption_clause)?;

    Ok(Response::default().add_attributes(vec![
        attr("action", "start_redemption_clause"),
        attr("asset", asset.to_string()),
        attr("rate", exchange_rate),
        attr("amount", amount),
        attr("start_time", start_expiration.to_string()),
        attr("end_time", end_expiration.to_string()),
    ]))
}

/// Generates a transfer message given an asset and an amount
fn generate_transfer_message(
    asset: AssetInfo,
    amount: Uint128,
    recipient: String,
    id: u64,
) -> Result<SubMsg, ContractError> {
    match asset.clone() {
        AssetInfo::Native(denom) => {
            let bank_msg = BankMsg::Send {
                to_address: recipient,
                amount: vec![coin(amount.u128(), denom)],
            };

            Ok(SubMsg::reply_on_error(CosmosMsg::Bank(bank_msg), id))
        }
        AssetInfo::Cw20(addr) => {
            let transfer_msg = Cw20ExecuteMsg::Transfer { recipient, amount };
            let wasm_msg = wasm_execute(addr, &transfer_msg, vec![])?;
            Ok(SubMsg::reply_on_error(CosmosMsg::Wasm(wasm_msg), id))
        }
        // Does not support 1155 currently
        _ => Err(ContractError::InvalidAsset {
            asset: asset.to_string(),
        }),
    }
}

pub fn execute_redeem(
    ctx: ExecuteContext,
    amount_sent: Uint128,
    asset_sent: AssetInfo,
    // For refund purposes
    sender: &str,
) -> Result<Response, ContractError> {
    let ExecuteContext { deps, .. } = ctx;

    let Some(mut redemption_clause) = REDEMPTION_CLAUSE.may_load(deps.storage)? else {
        return Err(ContractError::NoOngoingSale {});
    };

    // Check if sale has started
    ensure!(
        redemption_clause.start_time.is_expired(&ctx.env.block),
        ContractError::SaleNotStarted {}
    );
    // Check if sale has ended
    ensure!(
        !redemption_clause.end_time.is_expired(&ctx.env.block),
        ContractError::SaleEnded {}
    );

    let redeemed = amount_sent.checked_mul(redemption_clause.exchange_rate)?;

    ensure!(
        !redeemed.is_zero(),
        ContractError::InvalidFunds {
            msg: "Not enough funds sent to redeem".to_string()
        }
    );
    ensure!(
        redemption_clause.amount >= redeemed,
        ContractError::NotEnoughTokens {}
    );

    // Transfer tokens to the user redeeming the cw20 token
    let transfer_msg_to_user = generate_transfer_message(
        redemption_clause.asset.clone(),
        redeemed,
        sender.to_string(),
        RECIPIENT_REPLY_ID,
    )?;

    // Tranfer the redeemed tokens to the redemtion clause recipient
    let transfer_msg_to_redemption_clause_recipient = generate_transfer_message(
        asset_sent.clone(),
        amount_sent,
        redemption_clause.recipient.clone(),
        RECIPIENT_REPLY_ID,
    )?;

    // Update sale amount remaining
    redemption_clause.amount = redemption_clause.amount.checked_sub(redeemed)?;
    REDEMPTION_CLAUSE.save(deps.storage, &redemption_clause)?;

    Ok(Response::default()
        .add_submessage(transfer_msg_to_user)
        .add_submessage(transfer_msg_to_redemption_clause_recipient)
        .add_attributes(vec![
            attr("action", "redeem"),
            attr("purchaser", sender),
            attr("amount", redeemed),
            attr("purchase_asset", asset_sent.to_string()),
            attr("purchase_asset_amount_send", amount_sent),
        ]))
}

pub fn execute_cancel_redemption_clause(ctx: ExecuteContext) -> Result<Response, ContractError> {
    let ExecuteContext { deps, info, .. } = ctx;

    let Some(redemption_clause) = REDEMPTION_CLAUSE.may_load(deps.storage)? else {
        return Err(ContractError::NoOngoingSale {});
    };

    let mut resp = Response::default();

    // Refund any remaining amount
    if !redemption_clause.amount.is_zero() {
        resp = resp
            .add_submessage(generate_transfer_message(
                redemption_clause.asset.clone(),
                redemption_clause.amount,
                info.sender.to_string(),
                REFUND_REPLY_ID,
            )?)
            .add_attribute("refunded_amount", redemption_clause.amount);
    }

    // Redemption clause can now be removed
    REDEMPTION_CLAUSE.remove(deps.storage);

    Ok(resp.add_attributes(vec![attr("action", "cancel_redemption_clause")]))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    ADOContract::default().migrate(deps, env, CONTRACT_NAME, CONTRACT_VERSION)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
    match msg {
        QueryMsg::RedemptionClause {} => query_redemption(deps),
        QueryMsg::TokenAddress {} => query_token_address(deps),
        QueryMsg::RedemptionAsset {} => query_redemption_asset(deps),
        _ => ADOContract::default().query(deps, env, msg),
    }
}

fn query_redemption(deps: Deps) -> Result<Binary, ContractError> {
    let redemption = REDEMPTION_CLAUSE.may_load(deps.storage)?;

    Ok(to_json_binary(&RedemptionResponse { redemption })?)
}

fn query_token_address(deps: Deps) -> Result<Binary, ContractError> {
    let address = TOKEN_ADDRESS.load(deps.storage)?.get_raw_address(&deps)?;

    Ok(to_json_binary(&TokenAddressResponse {
        address: address.to_string(),
    })?)
}

fn query_redemption_asset(deps: Deps) -> Result<Binary, ContractError> {
    let redemption_clause = REDEMPTION_CLAUSE.load(deps.storage)?;

    Ok(to_json_binary(&RedemptionAssetResponse {
        asset: redemption_clause.asset.to_string(),
    })?)
}
