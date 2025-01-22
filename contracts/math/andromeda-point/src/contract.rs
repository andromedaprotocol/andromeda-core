#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{Binary, Deps, DepsMut, Env, MessageInfo, Reply, Response, StdError};

use crate::{
    execute::{delete_point, set_point, update_restriction},
    query::{get_data_owner, get_point},
    state::RESTRICTION,
};
use andromeda_math::point::{ExecuteMsg, InstantiateMsg, QueryMsg};
use andromeda_std::{
    ado_base::{InstantiateMsg as BaseInstantiateMsg, MigrateMsg},
    ado_contract::ADOContract,
    andr_execute_fn,
    common::encode_binary,
    error::ContractError,
};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:andromeda-point";
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
    Ok(resp)
}

#[andr_execute_fn]
pub fn execute(ctx: ExecuteContext, msg: ExecuteMsg) -> Result<Response, ContractError> {
    match msg.clone() {
        ExecuteMsg::UpdateRestriction { restriction } => update_restriction(ctx, restriction),
        ExecuteMsg::SetPoint { point } => set_point(ctx, point),
        ExecuteMsg::DeletePoint {} => delete_point(ctx),
        _ => ADOContract::default().execute(ctx, msg),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
    match msg {
        QueryMsg::GetPoint {} => encode_binary(&get_point(deps.storage)?),
        QueryMsg::GetDataOwner {} => encode_binary(&get_data_owner(deps.storage)?),
        _ => ADOContract::default().query(deps, env, msg),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    ADOContract::default().migrate(deps, env, CONTRACT_NAME, CONTRACT_VERSION)
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
