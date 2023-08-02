#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    ensure, to_binary, Binary, CosmosMsg, Deps, DepsMut, Empty, Env, MessageInfo, Response,
    StdError, SubMsg, WasmMsg,
};
use cw2::{get_contract_version, set_contract_version};

use crate::state::{get_key_or_default, query_value, DATA};
use andromeda_data_storage::primitive::{
    ExecuteMsg, InstantiateMsg, MigrateMsg, Primitive, QueryMsg,
};
use andromeda_std::os::vfs::ExecuteMsg as VFSExecuteMsg;
use andromeda_std::{
    ado_base::InstantiateMsg as BaseInstantiateMsg,
    ado_contract::ADOContract,
    common::{context::ExecuteContext, encode_binary},
    error::{from_semver, ContractError},
};
use cw_utils::nonpayable;
use semver::Version;

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:andromeda-primitive";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

const REGISTER_PARENT_PATH_MSG: u64 = 1001;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    let mut msgs: Vec<SubMsg> = vec![];
    let resp = ADOContract::default().instantiate(
        deps.storage,
        env,
        deps.api,
        info.clone(),
        BaseInstantiateMsg {
            ado_type: "primitive".to_string(),
            ado_version: CONTRACT_VERSION.to_string(),
            operators: None,
            kernel_address: msg.kernel_address,
            owner: msg.owner,
        },
    )?;
    if msg.vfs_name.is_some() {
        let vfs_address = ADOContract::default().get_vfs_address(deps.storage, &deps.querier)?;

        let add_path_msg = VFSExecuteMsg::AddParentPath {
            name: msg.vfs_name.unwrap(),
            parent_address: info.sender,
        };
        let cosmos_msg: CosmosMsg<Empty> = CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: vfs_address.to_string(),
            msg: to_binary(&add_path_msg)?,
            funds: vec![],
        });

        let register_msg = SubMsg::reply_on_error(cosmos_msg, REGISTER_PARENT_PATH_MSG);
        msgs.push(register_msg);
    }

    Ok(resp.add_submessages(msgs))
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
        _ => ADOContract::default().execute(ctx, msg),
    }
}

pub fn execute_set_value(
    deps: DepsMut,
    info: MessageInfo,
    key: Option<String>,
    value: Primitive,
) -> Result<Response, ContractError> {
    nonpayable(&info)?;

    let sender = info.sender.to_string();
    ensure!(
        ADOContract::default().is_owner_or_operator(deps.storage, &sender)?,
        ContractError::Unauthorized {}
    );
    if value.is_invalid() {
        return Err(ContractError::InvalidPrimitive {});
    }
    let key: &str = get_key_or_default(&key);
    DATA.update::<_, StdError>(deps.storage, key, |old| match old {
        Some(_) => Ok(value.clone()),
        None => Ok(value.clone()),
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
    let sender = info.sender.to_string();
    ensure!(
        ADOContract::default().is_owner_or_operator(deps.storage, &sender)?,
        ContractError::Unauthorized {}
    );
    let key = get_key_or_default(&key);
    DATA.remove(deps.storage, key);
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
        _ => ADOContract::default().query(deps, env, msg),
    }
}
