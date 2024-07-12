#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{Binary, Deps, DepsMut, Env, MessageInfo, Response, ensure, Storage, Addr};

use andromeda_data_storage::counter::{ExecuteMsg, InstantiateMsg, QueryMsg, CounterRestriction};
use andromeda_data_storage::counter::{
    GetInitialAmountResponse, GetCurrentAmountResponse, GetIncreaseAmountResponse, GetDecreaseAmountResponse, GetRestrictionResponse,
};
use andromeda_std::{
    ado_base::{InstantiateMsg as BaseInstantiateMsg, MigrateMsg},
    ado_contract::ADOContract,
    common::{context::ExecuteContext, encode_binary},
    error::ContractError,
};
use cw_utils::nonpayable;

use crate::state::{RESTRICTION, CURRENT_AMOUNT, DEFAULT_INITIAL_AMOUNT, DEFAULT_INCREASE_AMOUNT, DEFAULT_DECREASE_AMOUNT, INITIAL_AMOUNT, INCREASE_AMOUNT, DECREASE_AMOUNT};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:andromeda-counter";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    let resp = ADOContract::default().instantiate(
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

    RESTRICTION.save(deps.storage, &msg.restriction)?;

    if let Some(initial_amount) = msg.initial_amount {
        INITIAL_AMOUNT.save(deps.storage, &initial_amount)?;
        CURRENT_AMOUNT.save(deps.storage, &initial_amount)?;
    } else {
        INITIAL_AMOUNT.save(deps.storage, &DEFAULT_INITIAL_AMOUNT)?;
        CURRENT_AMOUNT.save(deps.storage, &DEFAULT_INITIAL_AMOUNT)?;
    }

    if let Some(increase_amount) = msg.increase_amount {
        INCREASE_AMOUNT.save(deps.storage, &increase_amount)?;
    } else {
        INCREASE_AMOUNT.save(deps.storage, &DEFAULT_INCREASE_AMOUNT)?;
    }

    if let Some(decrease_amount) = msg.decrease_amount {
        DECREASE_AMOUNT.save(deps.storage, &decrease_amount)?;
    } else {
        DECREASE_AMOUNT.save(deps.storage, &DEFAULT_DECREASE_AMOUNT)?;
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
    let ctx = ExecuteContext::new(deps, info, env);
    match msg {
        ExecuteMsg::AMPReceive(pkt) => {
            ADOContract::default().execute_amp_receive(ctx, pkt, handle_execute)
        },
        _ => handle_execute(ctx, msg),
    }
}

fn handle_execute(ctx: ExecuteContext, msg: ExecuteMsg) -> Result<Response, ContractError> {

    match msg.clone() {
        ExecuteMsg::Increment {} => execute_increment(ctx),
        ExecuteMsg::Decrement {} => execute_decrement(ctx),
        ExecuteMsg::Reset {} => execute_reset(ctx),
        ExecuteMsg::UpdateRestriction { restriction } => execute_update_restriction(ctx, restriction),
        ExecuteMsg::SetIncreaseAmount { increase_amount } => execute_set_increase_amount(ctx, increase_amount),
        ExecuteMsg::SetDecreaseAmount { decrease_amount } => execute_set_decrease_amount(ctx, decrease_amount),
        _ => ADOContract::default().execute(ctx, msg),

    }
}

pub fn execute_increment(ctx: ExecuteContext) -> Result<Response, ContractError> {
    nonpayable(&ctx.info)?;

    let sender = ctx.info.sender.clone();
    ensure!(
        has_permission(ctx.deps.storage, &sender)?,
        ContractError::Unauthorized {}
    );
    let current_amount = CURRENT_AMOUNT.load(ctx.deps.storage)?;
    let increase_amount = INCREASE_AMOUNT.load(ctx.deps.storage)?;
    CURRENT_AMOUNT.save(ctx.deps.storage, &(current_amount + increase_amount))?;

    Ok(
        Response::new()
        .add_attribute("method", "execute_increment")
        .add_attribute("sender", sender)
        .add_attribute("current_amount", &CURRENT_AMOUNT.load(ctx.deps.storage)?.to_string())
    )
}

pub fn execute_decrement(ctx: ExecuteContext) -> Result<Response, ContractError> {
    nonpayable(&ctx.info)?;

    let sender = ctx.info.sender.clone();
    ensure!(
        has_permission(ctx.deps.storage, &sender)?,
        ContractError::Unauthorized {}
    );
    let current_amount = CURRENT_AMOUNT.load(ctx.deps.storage)?;
    let decrease_amount = DECREASE_AMOUNT.load(ctx.deps.storage)?;
    if decrease_amount > current_amount {
        CURRENT_AMOUNT.save(ctx.deps.storage, &0)?;
    } else {
        CURRENT_AMOUNT.save(ctx.deps.storage, &(current_amount - decrease_amount))?;
    }

    Ok(
        Response::new()
        .add_attribute("method", "execute_decrement")
        .add_attribute("sender", sender)
        .add_attribute("current_amount", &CURRENT_AMOUNT.load(ctx.deps.storage)?.to_string())
    )
}

pub fn execute_reset(
    ctx: ExecuteContext,
) -> Result<Response, ContractError> {
    let sender = ctx.info.sender.clone();
    ensure!(
        has_permission(ctx.deps.storage, &sender)?,
        ContractError::Unauthorized {}
    );

    let initial_amount = INITIAL_AMOUNT.load(ctx.deps.storage)?;
    CURRENT_AMOUNT.save(ctx.deps.storage, &initial_amount)?;

    Ok(
        Response::new()
        .add_attribute("method", "execute_reset")
        .add_attribute("sender", sender)
        .add_attribute("current_amount", initial_amount.to_string())
    )
}

pub fn execute_update_restriction(
    ctx: ExecuteContext,
    restriction: CounterRestriction,
) -> Result<Response, ContractError> {
    nonpayable(&ctx.info)?;
    let sender = ctx.info.sender;
    ensure!(
        ADOContract::default().is_owner_or_operator(ctx.deps.storage, sender.as_ref())?,
        ContractError::Unauthorized {}
    );
    RESTRICTION.save(ctx.deps.storage, &restriction)?;

    Ok(
        Response::new()
        .add_attribute("method", "update_restriction")
        .add_attribute("sender", sender)
    )
}

pub fn execute_set_increase_amount(
    ctx: ExecuteContext,
    increase_amount: u64,
) -> Result<Response, ContractError> {
    let sender = ctx.info.sender;
    ensure!(
        ADOContract::default().is_owner_or_operator(ctx.deps.storage, sender.as_ref())?,
        ContractError::Unauthorized {}
    );

    INCREASE_AMOUNT.save(ctx.deps.storage, &increase_amount)?;

    Ok(
        Response::new()
        .add_attribute("method", "execute_set_increase_amount")
        .add_attribute("sender", sender)
    )
}

pub fn execute_set_decrease_amount(
    ctx: ExecuteContext,
    decrease_amount: u64,
) -> Result<Response, ContractError> {
    let sender = ctx.info.sender;
    ensure!(
        ADOContract::default().is_owner_or_operator(ctx.deps.storage, sender.as_ref())?,
        ContractError::Unauthorized {}
    );

    DECREASE_AMOUNT.save(ctx.deps.storage, &decrease_amount)?;

    Ok(
        Response::new()
        .add_attribute("method", "execute_set_decrease_amount")
        .add_attribute("sender", sender)
    )
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
    match msg {
        QueryMsg::GetInitialAmount {} => encode_binary(&get_initial_amount(deps.storage)?),
        QueryMsg::GetCurrentAmount {} => encode_binary(&get_current_amount(deps.storage)?),
        QueryMsg::GetIncreaseAmount {} => encode_binary(&get_increase_amount(deps.storage)?),
        QueryMsg::GetDecreaseAmount {} => encode_binary(&get_decrease_amount(deps.storage)?),
        QueryMsg::GetRestriction {} => encode_binary(&get_restriction(deps.storage)?),
        _ => ADOContract::default().query(deps, env, msg),
    }
}

pub fn get_initial_amount(storage: &dyn Storage) -> Result<GetInitialAmountResponse, ContractError> {
    let initial_amount = INITIAL_AMOUNT.load(storage)?;
    Ok(GetInitialAmountResponse { initial_amount })
}

pub fn get_current_amount(storage: &dyn Storage) -> Result<GetCurrentAmountResponse, ContractError> {
    let current_amount = CURRENT_AMOUNT.load(storage)?;
    Ok(GetCurrentAmountResponse { current_amount })
}

pub fn get_increase_amount(storage: &dyn Storage) -> Result<GetIncreaseAmountResponse, ContractError> {
    let increase_amount = INCREASE_AMOUNT.load(storage)?;
    Ok(GetIncreaseAmountResponse { increase_amount })
}

pub fn get_decrease_amount(storage: &dyn Storage) -> Result<GetDecreaseAmountResponse, ContractError> {
    let decrease_amount = DECREASE_AMOUNT.load(storage)?;
    Ok(GetDecreaseAmountResponse { decrease_amount })
}

pub fn get_restriction(storage: &dyn Storage) -> Result<GetRestrictionResponse, ContractError> {
    let restriction = RESTRICTION.load(storage)?;
    Ok(GetRestrictionResponse { restriction })
}

pub fn has_permission(
    storage: &dyn Storage,
    addr: &Addr,
) -> Result<bool, ContractError> {
    let is_operator = ADOContract::default().is_owner_or_operator(storage, addr.as_str())?;
    let allowed = match RESTRICTION.load(storage)? {
        CounterRestriction::Private => is_operator,
        CounterRestriction::Public => true,
    };
    Ok(is_operator || allowed)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    ADOContract::default().migrate(deps, CONTRACT_NAME, CONTRACT_VERSION)
}
