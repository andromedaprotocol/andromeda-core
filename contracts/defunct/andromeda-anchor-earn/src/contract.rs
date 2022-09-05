#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    ensure, Binary, CosmosMsg, Deps, DepsMut, Env, MessageInfo, Reply, Response, SubMsg, Uint128,
    WasmMsg,
};
use cw2::{get_contract_version, set_contract_version};
use cw20::Cw20ExecuteMsg;
use cw_asset::{Asset, AssetInfo};
use moneymarket::market::{Cw20HookMsg as MarketCw20HookMsg, ExecuteMsg as MarketExecuteMsg};

use crate::{
    primitive_keys::{ADDRESSES_TO_CACHE, ANCHOR_AUST, ANCHOR_MARKET},
    state::{Position, POSITION, PREV_AUST_BALANCE, PREV_UUSD_BALANCE, RECIPIENT_ADDR},
};

use ado_base::ADOContract;
use andromeda_ecosystem::anchor_earn::{
    ExecuteMsg, InstantiateMsg, MigrateMsg, PositionResponse, QueryMsg,
};
use common::{
    ado_base::{
        recipient::Recipient, AndromedaMsg, AndromedaQuery, InstantiateMsg as BaseInstantiateMsg,
    },
    encode_binary,
    error::ContractError,
    parse_message,
    withdraw::Withdrawal,
};

const UUSD_DENOM: &str = "uusd";
pub const DEPOSIT_ID: u64 = 1;
pub const WITHDRAW_ID: u64 = 2;

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:andromeda-anchor-earn";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    PREV_AUST_BALANCE.save(deps.storage, &Uint128::zero())?;
    PREV_UUSD_BALANCE.save(deps.storage, &Uint128::zero())?;

    let contract = ADOContract::default();

    let resp = contract.instantiate(
        deps.storage,
        env,
        deps.api,
        info,
        BaseInstantiateMsg {
            ado_type: "anchor-earn".to_string(),
            ado_version: CONTRACT_VERSION.to_string(),
            operators: None,
            modules: None,
            primitive_contract: Some(msg.primitive_contract),
        },
    )?;

    for address in ADDRESSES_TO_CACHE {
        contract.cache_address(deps.storage, &deps.querier, address)?;
    }

    Ok(resp)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::AndrReceive(msg) => execute_andr_receive(deps, env, info, msg),
    }
}

fn execute_andr_receive(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: AndromedaMsg,
) -> Result<Response, ContractError> {
    match msg {
        AndromedaMsg::Receive(data) => match data {
            None => execute_deposit(deps, env, info, None),
            Some(_) => {
                let recipient: Recipient = parse_message(&data)?;
                execute_deposit(deps, env, info, Some(recipient))
            }
        },
        AndromedaMsg::Withdraw {
            recipient,
            tokens_to_withdraw,
        } => execute_withdraw(deps, env, info, recipient, tokens_to_withdraw),
        _ => ADOContract::default().execute(deps, env, info, msg, execute),
    }
}

pub fn execute_deposit(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    recipient: Option<Recipient>,
) -> Result<Response, ContractError> {
    let contract = ADOContract::default();
    let anchor_market = contract.get_cached_address(deps.storage, ANCHOR_MARKET)?;
    let anchor_aust_token = contract.get_cached_address(deps.storage, ANCHOR_AUST)?;

    ensure!(
        info.funds.len() == 1,
        ContractError::InvalidFunds {
            msg: "Must deposit exactly 1 type of native coin.".to_string(),
        },
    )?;

    let recipient = match recipient {
        Some(recipient) => recipient,
        None => Recipient::Addr(info.sender.to_string()),
    };

    let payment = &info.funds[0];
    ensure!(
        payment.denom == UUSD_DENOM && payment.amount > Uint128::zero(),
        ContractError::InvalidFunds {
            msg: "Must deposit a non-zero quantity of uusd".to_string(),
        },
    )?;
    let aust = AssetInfo::cw20(deps.api.addr_validate(&anchor_aust_token)?);
    let aust_balance = aust.query_balance(&deps.querier, env.contract.address)?;
    let recipient_addr = recipient.get_addr(
        deps.api,
        &deps.querier,
        ADOContract::default().get_app_contract(deps.storage)?,
    )?;
    PREV_AUST_BALANCE.save(deps.storage, &aust_balance)?;
    RECIPIENT_ADDR.save(deps.storage, &recipient_addr)?;
    let payment_amount = payment.amount;

    if !POSITION.has(deps.storage, &recipient_addr) {
        POSITION.save(
            deps.storage,
            &recipient_addr,
            &Position {
                recipient,
                aust_amount: Uint128::zero(),
            },
        )?;
    }

    //deposit Anchor Mint
    Ok(Response::new()
        .add_submessage(SubMsg::reply_on_success(
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: anchor_market,
                msg: encode_binary(&MarketExecuteMsg::DepositStable {})?,
                funds: vec![payment.to_owned()],
            }),
            DEPOSIT_ID,
        ))
        .add_attribute("action", "deposit")
        .add_attribute("deposit_amount", payment_amount))
}

pub fn execute_withdraw(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    recipient: Option<Recipient>,
    tokens_to_withdraw: Option<Vec<Withdrawal>>,
) -> Result<Response, ContractError> {
    let contract = ADOContract::default();

    let aust_address = contract.get_cached_address(deps.storage, ANCHOR_AUST)?;

    let recipient = recipient.unwrap_or_else(|| Recipient::Addr(info.sender.to_string()));

    let recipient_addr = recipient.get_addr(
        deps.api,
        &deps.querier,
        ADOContract::default().get_app_contract(deps.storage)?,
    )?;

    let authorized = recipient_addr == info.sender
        || ADOContract::default().is_owner_or_operator(deps.storage, info.sender.as_str())?;
    ensure!(authorized, ContractError::Unauthorized {})?;

    ensure!(
        matches!(recipient, Recipient::Addr(_)),
        ContractError::InvalidRecipientType {
            msg: "Only recipients of type Addr are allowed as it only specifies the owner of the position to withdraw from".to_string()
        },
    )?;

    ensure!(
        tokens_to_withdraw.is_some(),
        ContractError::InvalidWithdrawal {
            msg: Some("Withdrawal must be non-empty".to_string()),
        },
    )?;
    let tokens_to_withdraw = tokens_to_withdraw.unwrap();

    ensure!(
        tokens_to_withdraw.len() == 1,
        ContractError::InvalidWithdrawal {
            msg: Some("Must only withdraw a single token".to_string()),
        },
    )?;

    // If we are here then there is always exactly a single token to withdraw.
    let token_to_withdraw = &tokens_to_withdraw[0];

    let token = token_to_withdraw.token.to_lowercase();
    if token == UUSD_DENOM {
        withdraw_uusd(deps, env, info, token_to_withdraw, recipient_addr)
    } else if token == "aust" || token == aust_address {
        withdraw_aust(deps, info, token_to_withdraw, recipient_addr)
    } else {
        Err(ContractError::InvalidTokensToWithdraw {
            msg: "Can only withdraw uusd or aUST".to_string(),
        })
    }
}

// The amount to withdraw specified in `withdrawal` is denominated in aUST. So if the
// amount is say 50, that would signify exchanging 50 aUST for however much UST that produces.
fn withdraw_uusd(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    withdrawal: &Withdrawal,
    recipient_addr: String,
) -> Result<Response, ContractError> {
    let contract = ADOContract::default();
    let anchor_aust_token = contract.get_cached_address(deps.storage, ANCHOR_AUST)?;
    let anchor_market = contract.get_cached_address(deps.storage, ANCHOR_MARKET)?;

    let mut position = POSITION.load(deps.storage, &recipient_addr)?;

    let authorized = recipient_addr == info.sender
        || contract.is_owner_or_operator(deps.storage, info.sender.as_str())?;

    ensure!(authorized, ContractError::Unauthorized {})?;

    let uusd = AssetInfo::native(UUSD_DENOM);
    let contract_balance = uusd.query_balance(&deps.querier, env.contract.address)?;

    PREV_UUSD_BALANCE.save(deps.storage, &contract_balance)?;
    RECIPIENT_ADDR.save(deps.storage, &recipient_addr)?;

    let amount_to_redeem = withdrawal.get_amount(position.aust_amount)?;

    position.aust_amount = position.aust_amount.checked_sub(amount_to_redeem)?;
    POSITION.save(deps.storage, &recipient_addr, &position)?;

    Ok(Response::new()
        .add_submessage(SubMsg::reply_on_success(
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: anchor_aust_token,
                msg: encode_binary(&Cw20ExecuteMsg::Send {
                    contract: anchor_market,
                    amount: amount_to_redeem,
                    msg: encode_binary(&MarketCw20HookMsg::RedeemStable {})?,
                })?,
                funds: vec![],
            }),
            WITHDRAW_ID,
        ))
        .add_attribute("action", "withdraw_uusd")
        .add_attribute("recipient_addr", recipient_addr))
}

fn withdraw_aust(
    deps: DepsMut,
    info: MessageInfo,
    withdrawal: &Withdrawal,
    recipient_addr: String,
) -> Result<Response, ContractError> {
    let contract = ADOContract::default();
    let anchor_aust_token = contract.get_cached_address(deps.storage, ANCHOR_AUST)?;

    let mut position = POSITION.load(deps.storage, &recipient_addr)?;

    let authorized = recipient_addr == info.sender
        || contract.is_owner_or_operator(deps.storage, info.sender.as_str())?;

    ensure!(authorized, ContractError::Unauthorized {})?;

    let amount = withdrawal.get_amount(position.aust_amount)?;

    position.aust_amount = position.aust_amount.checked_sub(amount)?;
    POSITION.save(deps.storage, &recipient_addr, &position)?;

    let msg = position.recipient.generate_msg_from_asset(
        deps.api,
        &deps.querier,
        ADOContract::default().get_app_contract(deps.storage)?,
        Asset::cw20(deps.api.addr_validate(&anchor_aust_token)?, amount),
    )?;

    Ok(Response::new()
        .add_submessage(msg)
        .add_attribute("action", "withdraw_aust")
        .add_attribute("recipient_addr", recipient_addr))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, env: Env, msg: Reply) -> Result<Response, ContractError> {
    match msg.id {
        DEPOSIT_ID => reply_update_position(deps, env),
        WITHDRAW_ID => reply_withdraw_ust(deps, env),
        _ => Err(ContractError::InvalidReplyId {}),
    }
}

fn reply_update_position(deps: DepsMut, env: Env) -> Result<Response, ContractError> {
    // stores aUST amount to position
    let contract = ADOContract::default();
    let anchor_aust_token = contract.get_cached_address(deps.storage, ANCHOR_AUST)?;

    let aust = AssetInfo::cw20(deps.api.addr_validate(&anchor_aust_token)?);
    let aust_balance = aust.query_balance(&deps.querier, env.contract.address)?;

    let prev_aust_balance = PREV_AUST_BALANCE.load(deps.storage)?;
    let new_aust_balance = aust_balance.checked_sub(prev_aust_balance)?;
    ensure!(
        new_aust_balance > Uint128::zero(),
        ContractError::InvalidFunds {
            msg: "No aUST tokens minted".to_string(),
        },
    )?;

    let recipient_addr = RECIPIENT_ADDR.load(deps.storage)?;
    let mut position = POSITION.load(deps.storage, &recipient_addr)?;
    position.aust_amount += new_aust_balance;
    POSITION.save(deps.storage, &recipient_addr, &position)?;
    Ok(Response::new()
        .add_attribute("action", "reply_update_position")
        .add_attribute("recipient_addr", recipient_addr)
        .add_attribute("aust_amount", new_aust_balance.to_string()))
}

fn reply_withdraw_ust(deps: DepsMut, env: Env) -> Result<Response, ContractError> {
    let uusd = AssetInfo::native(UUSD_DENOM);
    let current_balance = uusd.query_balance(&deps.querier, env.contract.address)?;

    let prev_balance = PREV_UUSD_BALANCE.load(deps.storage)?;
    let transfer_amount = current_balance - prev_balance;

    let recipient_addr = RECIPIENT_ADDR.load(deps.storage)?;
    let recipient = POSITION.load(deps.storage, &recipient_addr)?.recipient;
    let mut msgs = vec![];
    if transfer_amount > Uint128::zero() {
        msgs.push(recipient.generate_msg_from_asset(
            deps.api,
            &deps.querier,
            ADOContract::default().get_app_contract(deps.storage)?,
            Asset::native(UUSD_DENOM, transfer_amount.u128()),
        )?);
    }
    Ok(Response::new()
        .add_submessages(msgs)
        .add_attribute("action", "reply_withdraw_ust")
        .add_attribute("recipient", recipient_addr)
        .add_attribute("amount", transfer_amount))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
    match msg {
        QueryMsg::AndrQuery(msg) => handle_andromeda_query(deps, env, msg),
    }
}

fn handle_andromeda_query(
    deps: Deps,
    env: Env,
    msg: AndromedaQuery,
) -> Result<Binary, ContractError> {
    match msg {
        AndromedaQuery::Get(data) => {
            let recipient: String = parse_message(&data)?;
            encode_binary(&query_position(deps, recipient)?)
        }
        _ => ADOContract::default().query(deps, env, msg, query),
    }
}

fn query_position(deps: Deps, recipient: String) -> Result<PositionResponse, ContractError> {
    let position = POSITION.load(deps.storage, &recipient)?;
    Ok(PositionResponse {
        recipient: position.recipient,
        aust_amount: position.aust_amount,
    })
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
        },
    )?;

    // New version has to be newer/greater than the old version
    ensure!(
        storage_version < version,
        ContractError::CannotMigrate {
            previous_contract: stored.version,
        },
    )?;

    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    // Update the ADOContract's version
    contract.execute_update_version(deps)?;

    Ok(Response::default())
}

fn from_semver(err: semver::Error) -> StdError {
    StdError::generic_err(format!("Semver: {}", err))
}
