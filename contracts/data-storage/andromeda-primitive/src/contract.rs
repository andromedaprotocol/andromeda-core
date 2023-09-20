#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{ensure, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdError, Storage};
use cw2::{get_contract_version, set_contract_version};

use crate::state::{
    get_key_or_default, has_key_permission, query_value, DATA, KEY_OWNER, RESTRICTION,
};
use andromeda_data_storage::primitive::{
    ExecuteMsg, InstantiateMsg, MigrateMsg, Primitive, PrimitiveRestriction, QueryMsg,
};
use andromeda_std::{
    ado_base::InstantiateMsg as BaseInstantiateMsg,
    ado_contract::ADOContract,
    amp::AndrAddr,
    common::{context::ExecuteContext, encode_binary},
    error::{from_semver, ContractError},
};
use cw_utils::nonpayable;
use semver::Version;

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:andromeda-primitive";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    let resp = ADOContract::default().instantiate(
        deps.storage,
        env,
        deps.api,
        info,
        BaseInstantiateMsg {
            ado_type: "primitive".to_string(),
            ado_version: CONTRACT_VERSION.to_string(),
            operators: None,
            kernel_address: msg.kernel_address,
            owner: msg.owner,
        },
    )?;
    RESTRICTION.save(deps.storage, &msg.restriction)?;
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
        }
        _ => handle_execute(ctx, msg),
    }
}

pub fn handle_execute(ctx: ExecuteContext, msg: ExecuteMsg) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::SetValue { key, value } => execute_set_value(ctx.deps, ctx.info, key, value),
        ExecuteMsg::DeleteValue { key } => execute_delete_value(ctx.deps, ctx.info, key),
        ExecuteMsg::UpdateRestriction { restriction } => {
            execute_update_restriction(ctx.deps, ctx.info, restriction)
        }
        _ => ADOContract::default().execute(ctx, msg),
    }
}

pub fn execute_update_restriction(
    deps: DepsMut,
    info: MessageInfo,
    restriction: PrimitiveRestriction,
) -> Result<Response, ContractError> {
    nonpayable(&info)?;
    let sender = info.sender;
    ensure!(
        ADOContract::default().is_owner_or_operator(deps.storage, sender.as_ref())?,
        ContractError::Unauthorized {}
    );
    RESTRICTION.save(deps.storage, &restriction)?;
    Ok(Response::new()
        .add_attribute("method", "update_restriction")
        .add_attribute("sender", sender))
}

pub fn execute_set_value(
    deps: DepsMut,
    info: MessageInfo,
    key: Option<String>,
    value: Primitive,
) -> Result<Response, ContractError> {
    nonpayable(&info)?;
    let sender = info.sender;
    let key: &str = get_key_or_default(&key);
    ensure!(
        has_key_permission(deps.storage, &sender, key)?,
        ContractError::Unauthorized {}
    );
    DATA.update::<_, StdError>(deps.storage, key, |old| match old {
        Some(_) => Ok(value.clone()),
        None => Ok(value.clone()),
    })?;
    // Update the owner of the key
    KEY_OWNER.update::<_, StdError>(deps.storage, key, |old| match old {
        Some(old) => Ok(old),
        None => Ok(sender.clone()),
    })?;

    Ok(Response::new()
        .add_attribute("method", "set_value")
        .add_attribute("sender", sender)
        .add_attribute("key", key)
        .add_attribute("value", format!("{value:?}")))
}

pub fn execute_delete_value(
    deps: DepsMut,
    info: MessageInfo,
    key: Option<String>,
) -> Result<Response, ContractError> {
    nonpayable(&info)?;
    let sender = info.sender;

    let key = get_key_or_default(&key);
    ensure!(
        has_key_permission(deps.storage, &sender, key)?,
        ContractError::Unauthorized {}
    );
    DATA.remove(deps.storage, key);
    KEY_OWNER.remove(deps.storage, key);
    Ok(Response::new()
        .add_attribute("method", "delete_value")
        .add_attribute("sender", sender)
        .add_attribute("key", key))
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

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
    match msg {
        QueryMsg::GetValue { key } => encode_binary(&query_value(deps.storage, key)?),
        QueryMsg::AllKeys {} => encode_binary(&query_all_keys(deps.storage)?),
        QueryMsg::OwnerKeys { owner } => encode_binary(&query_owner_keys(&deps, owner)?),
        _ => ADOContract::default().query(deps, env, msg),
    }
}

fn query_all_keys(storage: &dyn Storage) -> Result<Vec<String>, ContractError> {
    let keys = DATA
        .keys(storage, None, None, cosmwasm_std::Order::Ascending)
        .map(|key| key.unwrap())
        .collect();
    Ok(keys)
}

fn query_owner_keys(deps: &Deps, owner: AndrAddr) -> Result<Vec<String>, ContractError> {
    let owner = owner.get_raw_address(deps)?;
    let keys = KEY_OWNER
        .range(deps.storage, None, None, cosmwasm_std::Order::Ascending)
        .filter(|x| x.as_ref().unwrap().1 == owner)
        .map(|key| key.unwrap().0)
        .collect();
    Ok(keys)
}
