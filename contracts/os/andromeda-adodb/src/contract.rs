use crate::state::{
    read_code_id, read_latest_code_id, store_code_id, ACTION_FEES, ADO_TYPE, CODE_ID,
    LATEST_VERSION, PUBLISHER,
};
use andromeda_std::ado_base::os_querriers::{kernel_address, owner, version};
use andromeda_std::ado_base::InstantiateMsg as BaseInstantiateMsg;
use andromeda_std::ado_contract::ADOContract;
use andromeda_std::common::encode_binary;
use andromeda_std::error::{from_semver, ContractError};
use andromeda_std::os::adodb::{
    ADOMetadata, ADOVersion, ActionFee, ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg,
};
use cosmwasm_std::{
    attr, ensure, entry_point, Binary, Deps, DepsMut, Env, MessageInfo, Order, Reply, Response,
    StdError, StdResult, Storage,
};
use cw2::{get_contract_version, set_contract_version};
use cw_storage_plus::Bound;
use semver::Version;

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:andromeda-adodb";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
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
            kernel_address: msg.kernel_address,
            owner: msg.owner,
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
        ExecuteMsg::Publish {
            code_id,
            ado_type,
            action_fees,
            version,
            publisher,
        } => publish(
            deps,
            env,
            info,
            code_id,
            ado_type,
            version,
            action_fees,
            publisher,
        ),
        ExecuteMsg::UpdateActionFees {
            action_fees,
            ado_type,
        } => {
            execute_update_action_fees(deps, info, &ADOVersion::from_string(ado_type), action_fees)
        }
        ExecuteMsg::RemoveActionFees { ado_type, actions } => {
            execute_remove_actions(deps, info, &ADOVersion::from_string(ado_type), actions)
        }
        ExecuteMsg::UpdatePublisher {
            ado_type,
            publisher,
        } => execute_update_publisher(deps, info, &ADOVersion::from_string(ado_type), publisher),
    }
}

pub fn update_action_fees(
    storage: &mut dyn Storage,
    ado_version: &ADOVersion,
    fees: Vec<ActionFee>,
) -> Result<(), ContractError> {
    for action_fee in fees {
        ACTION_FEES.save(
            storage,
            &(ado_version.clone().into_string(), action_fee.clone().action),
            &action_fee.clone(),
        )?;
    }

    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub fn publish(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    code_id: u64,
    ado_type: String,
    version: String,
    action_fees: Option<Vec<ActionFee>>,
    publisher: Option<String>,
) -> Result<Response, ContractError> {
    ensure!(
        ADOContract::default().is_owner_or_operator(deps.storage, info.sender.as_str())?,
        ContractError::Unauthorized {}
    );
    let current_ado_version = LATEST_VERSION.may_load(deps.storage, &ado_type)?;
    if let Some(ado_version) = current_ado_version {
        let new_version = semver::Version::parse(&version).unwrap();
        let current_version = semver::Version::parse(&ado_version.0).unwrap();
        ensure!(
            new_version > current_version,
            ContractError::InvalidADOVersion {
                msg: Some("Version must be newer than the current version".to_string())
            }
        );
    }

    //TODO: Get Code ID info with cosmwasm 1.2

    let version = ADOVersion::from_type(ado_type).with_version(version);
    ensure!(
        version.validate(),
        ContractError::InvalidADOVersion { msg: None }
    );

    // Ensure version is not already published
    let curr_code_id = read_code_id(deps.storage, &version);
    ensure!(
        curr_code_id.is_err(),
        ContractError::InvalidADOVersion {
            msg: Some(String::from("Version already published"))
        }
    );

    store_code_id(deps.storage, &version, code_id)?;
    PUBLISHER.save(
        deps.storage,
        version.as_str(),
        &publisher.clone().unwrap_or(info.sender.to_string()),
    )?;

    if let Some(fees) = action_fees {
        update_action_fees(deps.storage, &version, fees)?;
    }

    Ok(Response::default().add_attributes(vec![
        attr("action", "publish_ado"),
        attr("ado_type", version.into_string()),
        attr("code_id", code_id.to_string()),
        attr("publisher", publisher.unwrap_or(info.sender.to_string())),
    ]))
}

fn execute_update_action_fees(
    deps: DepsMut,
    info: MessageInfo,
    ado_version: &ADOVersion,
    action_fees: Vec<ActionFee>,
) -> Result<Response, ContractError> {
    ensure!(
        ADOContract::default().is_contract_owner(deps.storage, info.sender.as_str())?,
        ContractError::Unauthorized {}
    );
    let ado_type_exists = read_code_id(deps.storage, ado_version);
    ensure!(
        ado_type_exists.is_ok(),
        ContractError::InvalidADOVersion {
            msg: Some("ADO type does not exist".to_string())
        }
    );

    update_action_fees(deps.storage, ado_version, action_fees)?;

    Ok(Response::default().add_attributes(vec![
        attr("action", "update_action_fees"),
        attr("ado_type", ado_version.clone().into_string()),
    ]))
}

fn execute_remove_actions(
    deps: DepsMut,
    info: MessageInfo,
    ado_version: &ADOVersion,
    actions: Vec<String>,
) -> Result<Response, ContractError> {
    ensure!(
        ADOContract::default().is_contract_owner(deps.storage, info.sender.as_str())?,
        ContractError::Unauthorized {}
    );
    let ado_type_exists = read_code_id(deps.storage, ado_version);
    ensure!(
        ado_type_exists.is_ok(),
        ContractError::InvalidADOVersion {
            msg: Some("ADO type does not exist".to_string())
        }
    );

    let mut res = Response::default().add_attributes(vec![
        attr("action", "remove_actions"),
        attr("ado_type", ado_version.clone().into_string()),
    ]);

    for action in actions {
        ACTION_FEES.remove(
            deps.storage,
            &(ado_version.clone().into_string(), action.clone()),
        );
        res = res.add_attribute("action_fee_removed", action);
    }

    Ok(res)
}

fn execute_update_publisher(
    deps: DepsMut,
    info: MessageInfo,
    ado_version: &ADOVersion,
    publisher: String,
) -> Result<Response, ContractError> {
    ensure!(
        ADOContract::default().is_contract_owner(deps.storage, info.sender.as_str())?,
        ContractError::Unauthorized {}
    );

    let ado_type_exists = read_code_id(deps.storage, ado_version);
    ensure!(
        ado_type_exists.is_ok(),
        ContractError::InvalidADOVersion {
            msg: Some("ADO type does not exist".to_string())
        }
    );

    PUBLISHER.save(deps.storage, ado_version.as_str(), &publisher)?;

    Ok(Response::default().add_attributes(vec![
        attr("action", "update_publisher"),
        attr("ado_type", ado_version.clone().into_string()),
        attr("publisher", publisher),
    ]))
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
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
    match msg {
        QueryMsg::CodeId { key } => encode_binary(&query_code_id(deps, key)?),
        QueryMsg::ADOType { code_id } => encode_binary(&query_ado_type(deps, code_id)?),
        QueryMsg::AllADOTypes { start_after, limit } => {
            encode_binary(&query_all_ado_types(deps.storage, start_after, limit)?)
        }
        QueryMsg::ADOVersions {
            ado_type,
            start_after,
            limit,
        } => encode_binary(&query_ado_versions(
            deps.storage,
            &ado_type,
            start_after,
            limit,
        )?),
        QueryMsg::ADOMetadata { ado_type } => encode_binary(&query_ado_metadata(deps, ado_type)?),
        QueryMsg::ActionFee { ado_type, action } => {
            encode_binary(&query_action_fee(deps, ado_type, action)?)
        }
        QueryMsg::ActionFeeByCodeId { code_id, action } => {
            encode_binary(&query_action_fee_by_code_id(deps, code_id, action)?)
        }
        // Base queries
        QueryMsg::Version {} => encode_binary(&version(deps)?),
        QueryMsg::Owner {} => encode_binary(&owner(deps)?),
        QueryMsg::KernelAddress {} => encode_binary(&kernel_address(deps)?),
    }
}

fn query_code_id(deps: Deps, key: String) -> Result<u64, ContractError> {
    let code_id = read_code_id(deps.storage, &ADOVersion::from_string(key))?;
    Ok(code_id)
}

fn query_ado_type(deps: Deps, code_id: u64) -> Result<Option<String>, ContractError> {
    let ado_version = ADO_TYPE.may_load(deps.storage, code_id)?;
    Ok(ado_version)
}

const DEFAULT_LIMIT: u32 = 10;
const MAX_LIMIT: u32 = 100;

pub fn query_all_ado_types(
    storage: &dyn Storage,
    start_after: Option<String>,
    limit: Option<u32>,
) -> Result<Vec<String>, ContractError> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let start = start_after.map(|s| Bound::ExclusiveRaw(s.into()));

    let ado_types: StdResult<Vec<String>> = CODE_ID
        .keys(storage, start, None, Order::Ascending)
        .take(limit)
        .collect();
    Ok(ado_types?)
}

pub fn query_ado_versions(
    storage: &dyn Storage,
    ado_type: &str,
    start_after: Option<String>,
    limit: Option<u32>,
) -> Result<Vec<String>, ContractError> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let start_after = start_after.unwrap_or(ado_type.to_string());
    let start = Some(Bound::exclusive(start_after.as_str()));

    // All versions have @ as starting point, we can add A which has higher ascii than @ to get the
    let end_ado_type = format!("{ado_type}A");
    let end = Some(Bound::exclusive(end_ado_type.as_str()));

    let mut versions: Vec<String> = CODE_ID
        .keys(storage, start, end, Order::Ascending)
        .take(limit)
        .map(|item| item.unwrap())
        .collect();
    versions.sort_by(|a, b| {
        let version_a: Version = ADOVersion::from_string(a).get_version().parse().unwrap();
        let version_b: Version = ADOVersion::from_string(b).get_version().parse().unwrap();
        version_b.cmp(&version_a)
    });
    Ok(versions)
}

fn query_ado_metadata(deps: Deps, ado_type: String) -> Result<ADOMetadata, ContractError> {
    let ado_version = ADOVersion::from_string(ado_type);
    let publisher = PUBLISHER.load(deps.storage, ado_version.as_str())?;
    let latest_version = read_latest_code_id(deps.storage, ado_version.get_type())?;

    Ok(ADOMetadata {
        publisher,
        latest_version: latest_version.0,
    })
}

fn query_action_fee(
    deps: Deps,
    ado_type: String,
    action: String,
) -> Result<Option<ActionFee>, ContractError> {
    let ado_version = ADOVersion::from_string(ado_type);
    Ok(ACTION_FEES.may_load(deps.storage, &(ado_version.into_string(), action))?)
}

fn query_action_fee_by_code_id(
    deps: Deps,
    code_id: u64,
    action: String,
) -> Result<Option<ActionFee>, ContractError> {
    let ado_version = ADO_TYPE.load(deps.storage, code_id)?;
    Ok(ACTION_FEES.may_load(deps.storage, &(ado_version, action))?)
}
