use crate::state::{
    read_sale_infos, sale_infos, SaleInfo, TokenSaleState, NEXT_SALE_ID, TOKEN_SALE_STATE,
};
use andromeda_std::common::call_action::call_action;
use std::vec;

use andromeda_non_fungible_tokens::marketplace::{
    Cw721HookMsg, ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg, SaleIdsResponse,
    SaleStateResponse, Status,
};
use andromeda_std::ado_contract::ADOContract;

use andromeda_std::common::call_action::get_action_name;
use andromeda_std::common::context::ExecuteContext;
use andromeda_std::common::expiration::{
    expiration_from_milliseconds, MILLISECONDS_TO_NANOSECONDS_RATIO,
};
use andromeda_std::common::rates::get_tax_amount;
use andromeda_std::common::Funds;
use andromeda_std::{
    ado_base::InstantiateMsg as BaseInstantiateMsg,
    common::encode_binary,
    error::{from_semver, ContractError},
};
use cw2::{get_contract_version, set_contract_version};
use cw721::{Cw721ExecuteMsg, Cw721QueryMsg, Cw721ReceiveMsg, OwnerOfResponse};
use semver::Version;

#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    attr, ensure, from_json, has_coins, BankMsg, Binary, Coin, CosmosMsg, Deps, DepsMut, Env,
    MessageInfo, QuerierWrapper, QueryRequest, Response, Storage, SubMsg, Uint128, WasmMsg,
    WasmQuery,
};

use cw_utils::{nonpayable, Expiration};

const CONTRACT_NAME: &str = "crates.io:andromeda-marketplace";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    NEXT_SALE_ID.save(deps.storage, &Uint128::from(1u128))?;
    let inst_resp = ADOContract::default().instantiate(
        deps.storage,
        env,
        deps.api,
        info,
        BaseInstantiateMsg {
            ado_type: "marketplace".to_string(),
            ado_version: CONTRACT_VERSION.to_string(),
            operators: None,
            kernel_address: msg.kernel_address,
            owner: msg.owner,
        },
    )?;

    Ok(inst_resp)
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
    let action = get_action_name(CONTRACT_NAME, msg.as_ref());
    call_action(
        &mut ctx.deps,
        &ctx.info,
        &ctx.env,
        &ctx.amp_ctx,
        msg.as_ref(),
    )?;

    match msg {
        ExecuteMsg::ReceiveNft(msg) => handle_receive_cw721(ctx, msg),
        ExecuteMsg::UpdateSale {
            token_id,
            token_address,
            coin_denom,
            price,
        } => execute_update_sale(ctx, token_id, token_address, price, coin_denom),
        ExecuteMsg::Buy {
            token_id,
            token_address,
        } => execute_buy(ctx, token_id, token_address, action),
        ExecuteMsg::CancelSale {
            token_id,
            token_address,
        } => execute_cancel(ctx, token_id, token_address),
        _ => ADOContract::default().execute(ctx, msg),
    }
}

fn handle_receive_cw721(
    ctx: ExecuteContext,
    msg: Cw721ReceiveMsg,
) -> Result<Response, ContractError> {
    let ExecuteContext {
        deps, info, env, ..
    } = ctx;

    match from_json(&msg.msg)? {
        Cw721HookMsg::StartSale {
            price,
            coin_denom,
            start_time,
            duration,
        } => execute_start_sale(
            deps,
            env,
            msg.sender,
            msg.token_id,
            info.sender.to_string(),
            price,
            coin_denom,
            start_time,
            duration,
        ),
    }
}

#[allow(clippy::too_many_arguments)]
fn execute_start_sale(
    deps: DepsMut,
    env: Env,
    sender: String,
    token_id: String,
    token_address: String,
    price: Uint128,
    coin_denom: String,
    start_time: Option<u64>,
    duration: Option<u64>,
) -> Result<Response, ContractError> {
    // Price can't be zero
    ensure!(price > Uint128::zero(), ContractError::InvalidZeroAmount {});
    let current_time = env.block.time.nanos() / MILLISECONDS_TO_NANOSECONDS_RATIO;
    // If start time wasn't provided, it will be set as the current_time
    let start_expiration = if let Some(start_time) = start_time {
        expiration_from_milliseconds(start_time)?
    } else {
        expiration_from_milliseconds(current_time)?
    };

    // If no duration is provided, the exipration will be set as Never
    let end_expiration = if let Some(duration) = duration {
        expiration_from_milliseconds(start_time.unwrap_or(current_time) + duration)?
    } else {
        Expiration::Never {}
    };

    // To guard against misleading start times
    // Subtracting one second from the current block because the unit tests fail otherwise. The current time slightly differed from the block time.
    let recent_past_timestamp = env.block.time.minus_seconds(1);
    let recent_past_expiration = expiration_from_milliseconds(recent_past_timestamp.seconds())?;
    ensure!(
        start_expiration.gt(&recent_past_expiration),
        ContractError::StartTimeInThePast {
            current_time: env.block.time.nanos() / MILLISECONDS_TO_NANOSECONDS_RATIO,
            current_block: env.block.height,
        }
    );

    let sale_id = get_and_increment_next_sale_id(deps.storage, &token_id, &token_address)?;

    TOKEN_SALE_STATE.save(
        deps.storage,
        sale_id.u128(),
        &TokenSaleState {
            coin_denom: coin_denom.clone(),
            sale_id,
            owner: sender,
            token_id: token_id.clone(),
            token_address: token_address.clone(),
            price,
            status: Status::Open,
            start_time: start_expiration,
            end_time: end_expiration,
        },
    )?;
    Ok(Response::new().add_attributes(vec![
        attr("action", "start_sale"),
        attr("status", "Open"),
        attr("coin_denom", coin_denom),
        attr("price", price),
        attr("sale_id", sale_id.to_string()),
        attr("token_id", token_id),
        attr("token_address", token_address),
        attr("start_time", start_expiration.to_string()),
        attr("end_time", end_expiration.to_string()),
    ]))
}

#[allow(clippy::too_many_arguments)]
fn execute_update_sale(
    ctx: ExecuteContext,
    token_id: String,
    token_address: String,
    price: Uint128,
    coin_denom: String,
) -> Result<Response, ContractError> {
    let ExecuteContext { deps, info, .. } = ctx;

    nonpayable(&info)?;

    let mut token_sale_state =
        get_existing_token_sale_state(deps.storage, &token_id, &token_address)?;
    // Only token owner is authorized to update the sale

    ensure!(
        info.sender == token_sale_state.owner,
        ContractError::Unauthorized {}
    );

    // New price can't be zero
    ensure!(price > Uint128::zero(), ContractError::InvalidZeroAmount {});

    token_sale_state.price = price;
    token_sale_state.coin_denom = coin_denom.clone();
    TOKEN_SALE_STATE.save(
        deps.storage,
        token_sale_state.sale_id.u128(),
        &token_sale_state,
    )?;
    Ok(Response::new().add_attributes(vec![
        attr("action", "update_sale"),
        attr("coin_denom", coin_denom),
        attr("price", price),
        attr("sale_id", token_sale_state.sale_id.to_string()),
        attr("token_id", token_id),
        attr("token_address", token_address),
    ]))
}

fn execute_buy(
    ctx: ExecuteContext,
    token_id: String,
    token_address: String,
    action: String,
) -> Result<Response, ContractError> {
    let ExecuteContext {
        mut deps,
        info,
        env,
        ..
    } = ctx;

    let mut token_sale_state =
        get_existing_token_sale_state(deps.storage, &token_id, &token_address)?;

    let key = token_sale_state.sale_id.u128();

    match token_sale_state.status {
        Status::Open => {
            // Make sure the end time isn't expired, if it is we'll return an error and change the Status to expired in case if it's set as Open or Pending
            ensure!(
                !token_sale_state.end_time.is_expired(&env.block),
                ContractError::SaleExpired {}
            );

            // If start time hasn't expired, it means that the sale hasn't started yet.
            ensure!(
                token_sale_state.start_time.is_expired(&env.block),
                ContractError::SaleNotOpen {}
            );
        }
        Status::Expired => return Err(ContractError::SaleExpired {}),
        Status::Executed => return Err(ContractError::SaleExecuted {}),
        Status::Cancelled => return Err(ContractError::SaleCancelled {}),
    }

    // The owner can't buy his own NFT
    ensure!(
        token_sale_state.owner != info.sender,
        ContractError::TokenOwnerCannotBuy {}
    );

    // Only one coin can be sent
    ensure!(
        info.funds.len() == 1,
        ContractError::InvalidFunds {
            msg: "Sales ensure! exactly one coin to be sent.".to_string(),
        }
    );

    let token_owner = query_owner_of(
        deps.querier,
        token_sale_state.token_address.clone(),
        token_id.clone(),
    )?
    .owner;
    ensure!(
        // If this is false then the token is no longer held by the contract so the token has been
        // claimed.
        token_owner == env.contract.address,
        ContractError::SaleAlreadyConducted {}
    );

    let coin_denom = token_sale_state.coin_denom.clone();
    let payment: &Coin = &info.funds[0];

    // Make sure funds are equal to the price and in the correct denomination
    ensure!(
        payment.denom == coin_denom,
        ContractError::InvalidFunds {
            msg: format!("No {coin_denom} assets are provided to sale"),
        }
    );
    ensure!(
        payment.amount >= token_sale_state.price,
        ContractError::InsufficientFunds {}
    );

    // Change sale status from Open to Executed
    token_sale_state.status = Status::Executed;

    TOKEN_SALE_STATE.save(deps.storage, key, &token_sale_state)?;

    // Calculate the funds to be received after tax
    let after_tax_payment = purchase_token(&mut deps, &info, token_sale_state.clone(), action)?;

    let mut resp = Response::new();

    match after_tax_payment {
        Some(after_tax_payment) => {
            resp = resp
                .add_submessages(after_tax_payment.1)
                // Send funds to the original owner.
                .add_message(CosmosMsg::Bank(BankMsg::Send {
                    to_address: token_sale_state.owner,
                    amount: vec![after_tax_payment.0],
                }))
        }
        None => {
            let after_tax_payment = Coin::new(
                token_sale_state.price.u128(),
                token_sale_state.coin_denom.clone(),
            );
            // Send funds to the original owner.
            resp = resp.add_message(CosmosMsg::Bank(BankMsg::Send {
                to_address: token_sale_state.owner,
                amount: vec![after_tax_payment],
            }))
        }
    }

    Ok(resp
        // Send NFT to buyer.
        .add_message(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: token_sale_state.token_address.clone(),
            msg: encode_binary(&Cw721ExecuteMsg::TransferNft {
                recipient: info.sender.to_string(),
                token_id: token_id.clone(),
            })?,
            funds: vec![],
        }))
        .add_attribute("action", "buy")
        .add_attribute("token_id", token_id)
        .add_attribute("token_contract", token_sale_state.token_address)
        .add_attribute("recipient", info.sender.to_string())
        .add_attribute("sale_id", token_sale_state.sale_id))
}

fn execute_cancel(
    ctx: ExecuteContext,
    token_id: String,
    token_address: String,
) -> Result<Response, ContractError> {
    let ExecuteContext { deps, info, .. } = ctx;
    nonpayable(&info)?;

    let mut token_sale_state =
        get_existing_token_sale_state(deps.storage, &token_id, &token_address)?;

    ensure!(
        info.sender == token_sale_state.owner,
        ContractError::Unauthorized {}
    );

    // Sale needs to be open or pending to be cancelled
    ensure!(
        token_sale_state.status == Status::Open,
        ContractError::SaleNotOpen {}
    );

    let messages: Vec<CosmosMsg> = vec![CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: token_sale_state.token_address.clone(),
        msg: encode_binary(&Cw721ExecuteMsg::TransferNft {
            recipient: info.sender.to_string(),
            token_id: token_id.clone(),
        })?,
        funds: vec![],
    })];

    token_sale_state.status = Status::Cancelled;
    TOKEN_SALE_STATE.save(
        deps.storage,
        token_sale_state.sale_id.u128(),
        &token_sale_state,
    )?;

    Ok(Response::new()
        .add_messages(messages)
        .add_attribute("action", "cancel")
        .add_attribute("status", "Cancelled")
        .add_attribute("token_id", token_id)
        .add_attribute("token_contract", token_sale_state.token_address)
        .add_attribute("sale_id", token_sale_state.sale_id)
        .add_attribute("recipient", info.sender))
}

fn purchase_token(
    deps: &mut DepsMut,
    info: &MessageInfo,
    state: TokenSaleState,
    action: String,
) -> Result<Option<(Coin, Vec<SubMsg>)>, ContractError> {
    let total_cost = Coin::new(state.price.u128(), state.coin_denom.clone());

    let mut total_tax_amount = Uint128::zero();

    let transfer_response = ADOContract::default().query_deducted_funds(
        deps.as_ref(),
        action,
        Funds::Native(total_cost),
    )?;
    match transfer_response {
        Some(transfer_response) => {
            let remaining_amount = transfer_response.leftover_funds.try_get_coin()?;

            let tax_amount = get_tax_amount(
                &transfer_response.msgs,
                state.price,
                remaining_amount.amount,
            );

            // Calculate total tax
            total_tax_amount += tax_amount;

            let required_payment = Coin {
                denom: state.coin_denom.clone(),
                amount: state.price + total_tax_amount,
            };
            ensure!(
                has_coins(&info.funds, &required_payment),
                ContractError::InsufficientFunds {}
            );

            let after_tax_payment = Coin {
                denom: state.coin_denom,
                amount: remaining_amount.amount,
            };
            Ok(Some((after_tax_payment, transfer_response.msgs)))
        }
        None => Ok(None),
    }
}

fn get_existing_token_sale_state(
    storage: &dyn Storage,
    token_id: &str,
    token_address: &str,
) -> Result<TokenSaleState, ContractError> {
    let key = token_id.to_owned() + token_address;
    let latest_sale_id: Uint128 = match sale_infos().may_load(storage, &key)? {
        None => return Err(ContractError::SaleDoesNotExist {}),
        Some(sale_info) => *sale_info.last().unwrap(),
    };
    let token_sale_state = TOKEN_SALE_STATE.load(storage, latest_sale_id.u128())?;

    Ok(token_sale_state)
}

fn get_and_increment_next_sale_id(
    storage: &mut dyn Storage,
    token_id: &str,
    token_address: &str,
) -> Result<Uint128, ContractError> {
    let next_sale_id = NEXT_SALE_ID.load(storage)?;
    let incremented_next_sale_id = next_sale_id.checked_add(Uint128::from(1u128))?;
    NEXT_SALE_ID.save(storage, &incremented_next_sale_id)?;

    let key = token_id.to_owned() + token_address;

    let mut sale_info = sale_infos().load(storage, &key).unwrap_or_default();
    sale_info.push(next_sale_id);
    if sale_info.token_address.is_empty() {
        sale_info.token_address = token_address.to_owned();
        sale_info.token_id = token_id.to_owned();
    }
    sale_infos().save(storage, &key, &sale_info)?;
    Ok(next_sale_id)
}

#[entry_point]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
    match msg {
        QueryMsg::LatestSaleState {
            token_id,
            token_address,
        } => encode_binary(&query_latest_sale_state(deps, token_id, token_address)?),
        QueryMsg::SaleState { sale_id } => encode_binary(&query_sale_state(deps, sale_id)?),
        QueryMsg::SaleIds {
            token_id,
            token_address,
        } => encode_binary(&query_sale_ids(deps, token_id, token_address)?),
        QueryMsg::SaleInfosForAddress {
            token_address,
            start_after,
            limit,
        } => encode_binary(&query_sale_infos_for_address(
            deps,
            token_address,
            start_after,
            limit,
        )?),
        _ => ADOContract::default().query(deps, env, msg),
    }
}

fn query_sale_ids(
    deps: Deps,
    token_id: String,
    token_address: String,
) -> Result<SaleIdsResponse, ContractError> {
    let key = token_id + &token_address;
    let sale_info = sale_infos().may_load(deps.storage, &key)?;
    if let Some(sale_info) = sale_info {
        return Ok(SaleIdsResponse {
            sale_ids: sale_info.sale_ids,
        });
    }
    Ok(SaleIdsResponse { sale_ids: vec![] })
}

pub fn query_sale_infos_for_address(
    deps: Deps,
    token_address: String,
    start_after: Option<String>,
    limit: Option<u64>,
) -> Result<Vec<SaleInfo>, ContractError> {
    read_sale_infos(deps.storage, token_address, start_after, limit)
}

fn query_latest_sale_state(
    deps: Deps,
    token_id: String,
    token_address: String,
) -> Result<SaleStateResponse, ContractError> {
    let token_sale_state_result =
        get_existing_token_sale_state(deps.storage, &token_id, &token_address);
    if let Ok(token_sale_state) = token_sale_state_result {
        return Ok(token_sale_state.into());
    }
    Err(ContractError::SaleDoesNotExist {})
}

fn query_sale_state(deps: Deps, sale_id: Uint128) -> Result<SaleStateResponse, ContractError> {
    let token_sale_state = TOKEN_SALE_STATE.load(deps.storage, sale_id.u128())?;
    Ok(token_sale_state.into())
}

fn query_owner_of(
    querier: QuerierWrapper,
    token_addr: String,
    token_id: String,
) -> Result<OwnerOfResponse, ContractError> {
    let res: OwnerOfResponse = querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: token_addr,
        msg: encode_binary(&Cw721QueryMsg::OwnerOf {
            token_id,
            include_expired: None,
        })?,
    }))?;

    Ok(res)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    // New version
    let version: Version = CONTRACT_VERSION.parse().map_err(from_semver)?;

    // Old version
    let stored = get_contract_version(deps.storage)?;
    let storage_version: Version = stored.version.parse().map_err(from_semver)?;

    let contract = ADOContract::default();

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

    // Update the ADOContract's version
    contract.execute_update_version(deps)?;

    Ok(Response::default())
}
