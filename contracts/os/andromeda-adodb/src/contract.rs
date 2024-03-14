use crate::{execute, query};
use andromeda_std::ado_base::InstantiateMsg as BaseInstantiateMsg;
use andromeda_std::ado_contract::ADOContract;
use andromeda_std::common::encode_binary;
use andromeda_std::error::{from_semver, ContractError};
use andromeda_std::os::adodb::{ADOVersion, ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg};
use cosmwasm_std::{
    ensure, entry_point, Binary, Deps, DepsMut, Env, MessageInfo, Reply, Response, StdError,
};
use cw2::{get_contract_version, set_contract_version};

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
        } => execute::publish(
            deps,
            env,
            info,
            code_id,
            ado_type,
            version,
            action_fees,
            publisher,
        ),
        ExecuteMsg::Unpublish { ado_type, version } => {
            execute::unpublish(deps, env, info, ado_type, version)
        }
        ExecuteMsg::UpdateActionFees {
            action_fees,
            ado_type,
        } => {
            execute::update_action_fees(deps, info, &ADOVersion::from_string(ado_type), action_fees)
        }
        ExecuteMsg::RemoveActionFees { ado_type, actions } => {
            execute::remove_actions(deps, info, &ADOVersion::from_string(ado_type), actions)
        }
        ExecuteMsg::UpdatePublisher {
            ado_type,
            publisher,
        } => execute::update_publisher(deps, info, &ADOVersion::from_string(ado_type), publisher),
        // Base message
        ExecuteMsg::Ownership(ownership_message) => {
            ADOContract::default().execute_ownership(deps, env, info, ownership_message)
        }
    }
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
        QueryMsg::CodeId { key } => encode_binary(&query::code_id(deps, key)?),
        // QueryMsg::UnpublishedCodeIds {} => encode_binary(&query::unpublished_code_ids(deps)?),
        QueryMsg::IsUnpublishedCodeId { code_id } => {
            encode_binary(&query::is_unpublished_code_id(deps, code_id)?)
        }
        QueryMsg::ADOType { code_id } => encode_binary(&query::ado_type(deps, code_id)?),
        QueryMsg::AllADOTypes { start_after, limit } => {
            encode_binary(&query::all_ado_types(deps.storage, start_after, limit)?)
        }
        QueryMsg::ADOVersions {
            ado_type,
            start_after,
            limit,
        } => encode_binary(&query::ado_versions(
            deps.storage,
            &ado_type,
            start_after,
            limit,
        )?),
        // QueryMsg::UnpublishedADOVersions { ado_type } => {
        //     encode_binary(&query::unpublished_ado_versions(deps.storage, &ado_type)?)
        // }
        QueryMsg::ADOMetadata { ado_type } => encode_binary(&query::ado_metadata(deps, ado_type)?),
        QueryMsg::ActionFee { ado_type, action } => {
            encode_binary(&query::action_fee(deps, ado_type, action)?)
        }
        QueryMsg::ActionFeeByCodeId { code_id, action } => {
            encode_binary(&query::action_fee_by_code_id(deps, code_id, action)?)
        }
        // Base queries
        QueryMsg::Version {} => encode_binary(&ADOContract::default().query_version(deps)?),
        QueryMsg::Type {} => encode_binary(&ADOContract::default().query_type(deps)?),
        QueryMsg::Owner {} => encode_binary(&ADOContract::default().query_contract_owner(deps)?),
        QueryMsg::KernelAddress {} => {
            encode_binary(&ADOContract::default().query_kernel_address(deps)?)
        }
    }
}
