use crate::state::{read_code_id, store_code_id};
use ado_base::state::ADOContract;
use andromeda_app::adodb::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg};
use common::{
    ado_base::{AndromedaQuery, InstantiateMsg as BaseInstantiateMsg},
    encode_binary,
    error::ContractError,
    parse_message,
};
use cosmwasm_std::{
    attr, ensure, entry_point, Binary, Deps, DepsMut, Env, MessageInfo, Reply, Response, StdError,
};
use cw2::{get_contract_version, set_contract_version};
use semver::Version;

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:andromeda-kernel";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    _msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    ADOContract::default().instantiate(
        deps.storage,
        env,
        deps.api,
        info,
        BaseInstantiateMsg {
            ado_type: "adodb".to_string(),
            ado_version: CONTRACT_VERSION.to_string(),
            operators: None,
            modules: None,
            primitive_contract: None,
        },
    )
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
    match msg {
        ExecuteMsg::UpdateCodeId {
            code_id_key,
            code_id,
        } => add_update_code_id(deps, env, info, code_id_key, code_id),
        ExecuteMsg::AndrReceive(msg) => {
            ADOContract::default().execute(deps, env, info, msg, execute)
        }
    }
}

pub fn add_update_code_id(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    code_id_key: String,
    code_id: u64,
) -> Result<Response, ContractError> {
    ensure!(
        ADOContract::default().is_owner_or_operator(deps.storage, info.sender.as_str())?,
        ContractError::Unauthorized {}
    );
    store_code_id(deps.storage, &code_id_key, code_id)?;

    Ok(Response::default().add_attributes(vec![
        attr("action", "add_update_code_id"),
        attr("code_id_key", code_id_key),
        attr("code_id", code_id.to_string()),
    ]))
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

fn from_semver(err: semver::Error) -> StdError {
    StdError::generic_err(format!("Semver: {}", err))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
    match msg {
        QueryMsg::CodeId { key } => encode_binary(&query_code_id(deps, key)?),
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
            let code_id_key: String = parse_message(&data)?;
            encode_binary(&query_code_id(deps, code_id_key)?)
        }
        _ => ADOContract::default().query(deps, env, msg, query),
    }
}

fn query_code_id(deps: Deps, key: String) -> Result<u64, ContractError> {
    let code_id = read_code_id(deps.storage, &key)?;
    Ok(code_id)
}
