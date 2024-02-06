use crate::state::{
    read_code_id, save_action_fees, store_code_id, ACTION_FEES, LATEST_VERSION, PUBLISHER,
};

use andromeda_std::ado_contract::ADOContract;

use andromeda_std::error::ContractError;
use andromeda_std::os::adodb::{ADOVersion, ActionFee};
use cosmwasm_std::{attr, ensure, DepsMut, Env, MessageInfo, Response};

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
    ensure!(
        // Using trim to rule out non-empty strings made up of only spaces
        !ado_type.trim().is_empty(),
        ContractError::InvalidADOType {
            msg: Some("ado_type can't be an empty string".to_string())
        }
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
        save_action_fees(deps.storage, &version, fees)?;
    }

    Ok(Response::default().add_attributes(vec![
        attr("action", "publish_ado"),
        attr("ado_type", version.into_string()),
        attr("code_id", code_id.to_string()),
        attr("publisher", publisher.unwrap_or(info.sender.to_string())),
    ]))
}

pub fn update_action_fees(
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

    save_action_fees(deps.storage, ado_version, action_fees)?;

    Ok(Response::default().add_attributes(vec![
        attr("action", "update_action_fees"),
        attr("ado_type", ado_version.clone().into_string()),
    ]))
}

pub fn remove_actions(
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

pub fn update_publisher(
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
