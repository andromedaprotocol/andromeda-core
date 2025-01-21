#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    ensure, Addr, Binary, Deps, DepsMut, Env, MessageInfo, Reply, Response, StdError,
};

use crate::{
    execute::{delete_value, set_value, update_restriction},
    query::{get_data_owner, get_value},
    state::RESTRICTION,
};
use andromeda_data_storage::boolean::{BooleanRestriction, ExecuteMsg, InstantiateMsg, QueryMsg};
use andromeda_std::{
    ado_base::{
        permissioning::{LocalPermission, Permission},
        rates::{Rate, RatesMessage},
        InstantiateMsg as BaseInstantiateMsg, MigrateMsg,
    },
    ado_contract::ADOContract,
    andr_execute_fn,
    common::encode_binary,
    error::ContractError,
};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:andromeda-boolean";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

pub const SET_DELETE_VALUE_ACTION: &str = "set_delete_value";

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
        info.clone(),
        BaseInstantiateMsg {
            ado_type: CONTRACT_NAME.to_string(),
            ado_version: CONTRACT_VERSION.to_string(),
            kernel_address: msg.kernel_address,
            owner: msg.owner.clone(),
        },
    )?;
    RESTRICTION.save(deps.storage, &msg.restriction)?;

    if msg.restriction == BooleanRestriction::Private {
        ADOContract::default().permission_action(deps.storage, SET_DELETE_VALUE_ACTION)?;

        ADOContract::set_permission(
            deps.storage,
            SET_DELETE_VALUE_ACTION,
            match msg.owner.clone() {
                None => info.sender,
                Some(owner) => Addr::unchecked(owner),
            },
            Permission::Local(LocalPermission::Whitelisted(None)),
        )?;
    }

    Ok(resp)
}

#[andr_execute_fn]
pub fn execute(ctx: ExecuteContext, msg: ExecuteMsg) -> Result<Response, ContractError> {
    let action = msg.as_ref().to_string();
    match msg.clone() {
        ExecuteMsg::UpdateRestriction { restriction } => update_restriction(ctx, restriction),
        ExecuteMsg::SetValue { value } => set_value(ctx, value, action),
        ExecuteMsg::DeleteValue {} => delete_value(ctx),
        ExecuteMsg::Rates(rates_message) => match rates_message {
            RatesMessage::SetRate { rate, .. } => match rate {
                Rate::Local(local_rate) => {
                    // Percent rates aren't applicable in this case, so we enforce Flat rates
                    ensure!(local_rate.value.is_flat(), ContractError::InvalidRate {});
                    ADOContract::default().execute(ctx, msg)
                }
                Rate::Contract(_) => ADOContract::default().execute(ctx, msg),
            },
            RatesMessage::RemoveRate { .. } => ADOContract::default().execute(ctx, msg),
        },
        _ => ADOContract::default().execute(ctx, msg),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
    match msg {
        QueryMsg::GetValue {} => encode_binary(&get_value(deps.storage)?),
        QueryMsg::GetDataOwner {} => encode_binary(&get_data_owner(deps.storage)?),
        _ => ADOContract::default().query(deps, env, msg),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    ADOContract::default().migrate(deps, CONTRACT_NAME, CONTRACT_VERSION)
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
