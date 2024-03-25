use andromeda_fungible_tokens::cw20_exchange::{
    Cw20HookMsg, ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg, Sale, SaleAssetsResponse,
    SaleResponse, TokenAddressResponse,
};
use andromeda_std::{
    ado_base::InstantiateMsg as BaseInstantiateMsg,
    ado_contract::ADOContract,
    common::{
        actions::call_action,
        context::ExecuteContext,
        expiration::{expiration_from_milliseconds, MILLISECONDS_TO_NANOSECONDS_RATIO},
    },
    error::{from_semver, ContractError},
};
use cosmwasm_std::{
    attr, coin, ensure, entry_point, from_json, to_json_binary, wasm_execute, BankMsg, Binary,
    BlockInfo, CosmosMsg, Deps, DepsMut, Env, MessageInfo, Reply, Response, StdError, SubMsg,
    Uint128,
};
use cw2::{get_contract_version, set_contract_version};
use cw20::{Cw20ExecuteMsg, Cw20ReceiveMsg};
use cw_asset::AssetInfo;
use cw_storage_plus::Bound;
use cw_utils::{nonpayable, one_coin, Expiration};
use semver::Version;

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
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    TOKEN_ADDRESS.save(deps.storage, &msg.token_address)?;

    let contract = ADOContract::default();
    let resp = contract.instantiate(
        deps.storage,
        env,
        deps.api,
        &deps.querier,
        info,
        BaseInstantiateMsg {
            ado_type: "cw20-exchange".to_string(),
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

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    let ctx = ExecuteContext::new(deps, info, env);

    match msg {
        ExecuteMsg::AMPReceive(pkt) => {
            ADOContract::default().execute_amp_receive(ctx, pkt, handle_execute)
        }
        _ => handle_execute(ctx, msg),
    }
}

pub fn handle_execute(mut ctx: ExecuteContext, msg: ExecuteMsg) -> Result<Response, ContractError> {
    let action_response = call_action(
        &mut ctx.deps,
        &ctx.info,
        &ctx.env,
        &ctx.amp_ctx,
        msg.as_ref(),
    )?;
    let res = match msg {
        ExecuteMsg::CancelSale { asset } => execute_cancel_sale(ctx, asset),
        ExecuteMsg::Purchase { recipient } => execute_purchase_native(ctx, recipient),
        ExecuteMsg::Receive(cw20_msg) => execute_receive(ctx, cw20_msg),
        _ => ADOContract::default().execute(ctx, msg),
    }?;
    Ok(res
        .add_submessages(action_response.messages)
        .add_attributes(action_response.attributes)
        .add_events(action_response.events))
}

pub fn execute_receive(
    ctx: ExecuteContext,
    receive_msg: Cw20ReceiveMsg,
) -> Result<Response, ContractError> {
    let ExecuteContext { ref info, .. } = ctx;
    nonpayable(info)?;

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
    start_time: Option<u64>,
    duration: Option<u64>,
) -> Result<Response, ContractError> {
    let ExecuteContext { deps, info, .. } = ctx;

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
        ADOContract::default().is_contract_owner(deps.storage, &sender)?,
        ContractError::Unauthorized {}
    );
    // Message sender in this case should be the token address
    ensure!(
        info.sender == token_addr,
        ContractError::InvalidFunds {
            msg: "Incorrect CW20 provided for sale".to_string()
        }
    );

    let current_time = ctx.env.block.time.nanos() / MILLISECONDS_TO_NANOSECONDS_RATIO;

    let start_expiration = if let Some(start_time) = start_time {
        expiration_from_milliseconds(start_time)?
    } else {
        // Set as current time + 1 so that it isn't expired from the very start
        expiration_from_milliseconds(current_time + 1)?
    };

    // Validate start time
    let block_time = block_to_expiration(&ctx.env.block, start_expiration).unwrap();
    ensure!(
        start_expiration.gt(&block_time),
        ContractError::StartTimeInThePast {
            current_time,
            current_block: ctx.env.block.height,
        }
    );

    let end_expiration = if let Some(duration) = duration {
        // If there's no start time, consider it as now
        expiration_from_milliseconds(start_time.unwrap_or(current_time) + duration)?
    } else {
        Expiration::Never {}
    };

    // Do not allow duplicate sales
    let current_sale = SALE.may_load(deps.storage, &asset.to_string())?;
    ensure!(current_sale.is_none(), ContractError::SaleNotEnded {});

    let sale = Sale {
        amount,
        exchange_rate,
        recipient: recipient.unwrap_or(sender),
        start_time: start_expiration,
        end_time: end_expiration,
        start_amount: amount,
    };
    SALE.save(deps.storage, &asset.to_string(), &sale)?;

    Ok(Response::default().add_attributes(vec![
        attr("action", "start_sale"),
        attr("asset", asset.to_string()),
        attr("rate", exchange_rate),
        attr("amount", amount),
        attr("start_time", start_expiration.to_string()),
        attr("end_time", end_expiration.to_string()),
        attr("start_amount", amount),
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
    ensure!(
        !sale.end_time.is_expired(&ctx.env.block),
        ContractError::SaleEnded {}
    );

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
                REFUND_REPLY_ID,
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
        RECIPIENT_REPLY_ID,
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
    one_coin(info)?;

    let payment = info.funds.first().unwrap();
    let asset = AssetInfo::Native(payment.denom.to_string());
    let amount = payment.amount;

    execute_purchase(ctx, amount, asset, &recipient, &sender)
}

pub fn execute_cancel_sale(
    ctx: ExecuteContext,
    asset: AssetInfo,
) -> Result<Response, ContractError> {
    let ExecuteContext { deps, info, .. } = ctx;

    let contract = ADOContract::default();
    ensure!(
        contract.is_contract_owner(deps.storage, info.sender.as_str())?,
        ContractError::Unauthorized {}
    );

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
                REFUND_REPLY_ID,
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

fn block_to_expiration(block: &BlockInfo, model: Expiration) -> Option<Expiration> {
    match model {
        Expiration::AtTime(_) => Some(Expiration::AtTime(block.time)),
        Expiration::AtHeight(_) => Some(Expiration::AtHeight(block.height)),
        Expiration::Never {} => None,
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    // New version
    let version: Version = CONTRACT_VERSION.parse().map_err(from_semver)?;

    // Old version
    let stored = get_contract_version(deps.storage)?;
    let storage_version: Version = stored.version.parse().map_err(from_semver)?;

    ensure!(
        stored.contract == CONTRACT_NAME,
        ContractError::CannotMigrate {
            previous_contract: stored.contract,
        }
    );

    // New version has to be newer/greater than the old version
    ensure!(
        storage_version < version,
        ContractError::CannotMigrate {
            previous_contract: stored.version,
        }
    );

    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    Ok(Response::default())
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
