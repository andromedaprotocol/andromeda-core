use andromeda_std::andr_execute_fn;
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{Binary, Deps, DepsMut, Env, MessageInfo, Reply, Response, StdError, Storage};

use crate::state::{DEFAULT_KEY, KEY_OWNER, MATRIX};
use andromeda_math::matrix::{ExecuteMsg, InstantiateMsg, QueryMsg};
use andromeda_math::matrix::{GetMatrixResponse, Matrix};
use andromeda_std::{
    ado_base::{
        permissioning::{LocalPermission, Permission},
        InstantiateMsg as BaseInstantiateMsg, MigrateMsg,
    },
    ado_contract::ADOContract,
    amp::AndrAddr,
    common::{context::ExecuteContext, encode_binary},
    error::ContractError,
};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:andromeda-matrix";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

pub const STORE_MATRIX_ACTION: &str = "store_matrix";
pub const DELETE_MATRIX_ACTION: &str = "delete_matrix";

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

    if let Some(authorized_operator_addresses) = msg.authorized_operator_addresses {
        if !authorized_operator_addresses.is_empty() {
            ADOContract::default().permission_action(deps.storage, STORE_MATRIX_ACTION)?;
            ADOContract::default().permission_action(deps.storage, DELETE_MATRIX_ACTION)?;
        }

        for address in authorized_operator_addresses {
            let addr = address.get_raw_address(&deps.as_ref())?;
            ADOContract::set_permission(
                deps.storage,
                STORE_MATRIX_ACTION,
                addr.clone(),
                Permission::Local(LocalPermission::Whitelisted(None)),
            )?;
            ADOContract::set_permission(
                deps.storage,
                DELETE_MATRIX_ACTION,
                addr.clone(),
                Permission::Local(LocalPermission::Whitelisted(None)),
            )?;
        }
    }

    Ok(resp)
}

#[andr_execute_fn]
pub fn execute(ctx: ExecuteContext, msg: ExecuteMsg) -> Result<Response, ContractError> {
    match msg.clone() {
        ExecuteMsg::StoreMatrix { key, data } => store_matrix(ctx, key, data),
        ExecuteMsg::DeleteMatrix { key } => delete_matrix(ctx, key),
        _ => ADOContract::default().execute(ctx, msg),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
    match msg {
        QueryMsg::GetMatrix { key } => encode_binary(&get_matrix(deps.storage, key)?),
        QueryMsg::AllKeys {} => encode_binary(&all_keys(deps.storage)?),
        QueryMsg::OwnerKeys { owner } => encode_binary(&owner_keys(&deps, owner)?),
        _ => ADOContract::default().query(deps, env, msg),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    ADOContract::default().migrate(deps, CONTRACT_NAME, CONTRACT_VERSION)
}

/// ============================== Execution Functions ============================== ///
pub fn store_matrix(
    mut ctx: ExecuteContext,
    key: Option<String>,
    data: Matrix,
) -> Result<Response, ContractError> {
    let sender = ctx.info.sender.clone();

    ADOContract::default().is_permissioned(
        ctx.deps.branch(),
        ctx.env.clone(),
        STORE_MATRIX_ACTION,
        sender.clone(),
    )?;

    // Validate the data
    data.validate_matrix()?;

    let key: &str = get_key_or_default(&key);

    MATRIX.update::<_, StdError>(ctx.deps.storage, key, |old| match old {
        Some(_) => Ok(data.clone()),
        None => Ok(data.clone()),
    })?;
    // Update the owner of the key
    KEY_OWNER.update::<_, StdError>(ctx.deps.storage, key, |old| match old {
        Some(old) => Ok(old),
        None => Ok(sender.clone()),
    })?;

    let response = Response::new()
        .add_attribute("method", "store_matrix")
        .add_attribute("sender", sender)
        .add_attribute("key", key)
        .add_attribute("data", format!("{data:?}"));

    Ok(response)
}

pub fn delete_matrix(
    mut ctx: ExecuteContext,
    key: Option<String>,
) -> Result<Response, ContractError> {
    let sender = ctx.info.sender;

    ADOContract::default().is_permissioned(
        ctx.deps.branch(),
        ctx.env.clone(),
        DELETE_MATRIX_ACTION,
        sender.clone(),
    )?;

    let key = get_key_or_default(&key);

    MATRIX.remove(ctx.deps.storage, key);
    KEY_OWNER.remove(ctx.deps.storage, key);
    Ok(Response::new()
        .add_attribute("method", "delete_matrix")
        .add_attribute("sender", sender)
        .add_attribute("key", key))
}

/// ============================== Query Functions ============================== ///
pub fn get_matrix(
    storage: &dyn Storage,
    key: Option<String>,
) -> Result<GetMatrixResponse, ContractError> {
    let key = get_key_or_default(&key);
    let data = MATRIX.load(storage, key)?;
    Ok(GetMatrixResponse {
        key: key.to_string(),
        data,
    })
}

pub fn all_keys(storage: &dyn Storage) -> Result<Vec<String>, ContractError> {
    let keys = MATRIX
        .keys(storage, None, None, cosmwasm_std::Order::Ascending)
        .map(|key| key.unwrap())
        .collect();
    Ok(keys)
}

pub fn owner_keys(deps: &Deps, owner: AndrAddr) -> Result<Vec<String>, ContractError> {
    let owner = owner.get_raw_address(deps)?;
    let keys = KEY_OWNER
        .range(deps.storage, None, None, cosmwasm_std::Order::Ascending)
        .filter(|x| x.as_ref().unwrap().1 == owner)
        .map(|key| key.unwrap().0)
        .collect();
    Ok(keys)
}

pub fn get_key_or_default(name: &Option<String>) -> &str {
    match name {
        None => DEFAULT_KEY,
        Some(s) => s,
    }
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
