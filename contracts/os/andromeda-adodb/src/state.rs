use andromeda_std::{
    error::ContractError,
    os::adodb::{ADOVersion, ActionFee},
};
use cosmwasm_std::{ensure, Api, StdResult, Storage};
use cw_storage_plus::Map;

/// Stores a mapping from an ADO type/version to its code ID
pub const CODE_ID: Map<&str, u64> = Map::new("code_id");
/// Stores unpublished code IDs to prevent resubmission of malicious contracts
pub const UNPUBLISHED_CODE_IDS: Map<u64, bool> = Map::new("unpublished_code_ids");
/// Stores whether unpublished or not for a pair of (ADO type, ADO version). True means unpublished
pub const UNPUBLISHED_VERSIONS: Map<(&str, &str), bool> = Map::new("unpublished_versions");
/// Stores the latest version for a given ADO
pub const LATEST_VERSION: Map<&str, (String, u64)> = Map::new("latest_version");
/// Stores a mapping from code ID to ADO
pub const ADO_TYPE: Map<&str, ADOVersion> = Map::new("ado_type");
/// Stores a mapping from ADO to its publisher
pub const PUBLISHER: Map<&str, String> = Map::new("publisher");
/// Stores a mapping from an (ADO,Action) to its action fees
pub const ACTION_FEES: Map<&(String, String), ActionFee> = Map::new("action_fees");

pub fn store_code_id(
    storage: &mut dyn Storage,
    ado_version: &ADOVersion,
    code_id: u64,
) -> Result<(), ContractError> {
    let curr_type = ADO_TYPE.may_load(storage, &code_id.to_string())?;
    ensure!(
        curr_type.is_none() || &curr_type.unwrap() == ado_version,
        ContractError::Unauthorized {}
    );
    ADO_TYPE
        .save(storage, &code_id.to_string(), ado_version)
        .unwrap();
    let version = semver::Version::parse(&ado_version.get_version()).unwrap();
    let prerelease = version.pre.parse::<String>().unwrap_or_default();
    if prerelease.is_empty() {
        LATEST_VERSION
            .save(
                storage,
                &ado_version.get_type(),
                &(ado_version.get_version(), code_id),
            )
            .unwrap();
    }
    CODE_ID
        .save(storage, ado_version.as_str(), &code_id)
        .unwrap();

    Ok(())
}

pub fn remove_code_id(
    storage: &mut dyn Storage,
    ado_version: &ADOVersion,
    code_id: u64,
) -> Result<(), ContractError> {
    let curr_type = ADO_TYPE.may_load(storage, &code_id.to_string())?;
    ensure!(
        curr_type.is_none() || &curr_type.unwrap() == ado_version,
        ContractError::Unauthorized {}
    );
    ADO_TYPE.remove(storage, &code_id.to_string());
    let version_code = LATEST_VERSION.may_load(storage, &ado_version.get_type())?;
    if let Some(version_code) = version_code {
        // This means that the code_id we're trying to unpublish is also the latest
        if version_code.1 == code_id {
            let mut penultimate_version = semver::Version::new(0, 0, 0);
            let latest_version = semver::Version::parse(&ado_version.get_version()).unwrap();
            CODE_ID
                .keys(storage, None, None, cosmwasm_std::Order::Descending)
                .map(|v| v.unwrap())
                // Filter out the keys by type first
                .filter(|v| v.starts_with(&ado_version.get_type()))
                // We want to get the penultimate version, since it will replace the latest version
                .for_each(|v| {
                    if let Some((_, version)) = v.split_once('@') {
                        let current_version = semver::Version::parse(version).unwrap();
                        if penultimate_version < current_version && current_version < latest_version
                        {
                            penultimate_version = current_version;
                        };
                    };
                });
            // In that case, the version we're removing is the only one for that ADO type.
            if penultimate_version == semver::Version::new(0, 0, 0) {
                LATEST_VERSION.remove(storage, &ado_version.get_type());
            } else {
                let version_type = ADOVersion::from_type(ado_version.get_type())
                    .with_version(penultimate_version.to_string());
                let penultimate_version_id = CODE_ID.load(storage, version_type.as_str())?;
                LATEST_VERSION.save(
                    storage,
                    &ado_version.get_type(),
                    &(penultimate_version.to_string(), penultimate_version_id),
                )?;
            }
        }
    }
    CODE_ID.remove(storage, ado_version.as_str());

    // Check if there is any default ado set for this ado type. Defaults do not have versions appended to them.
    let default_ado = ADOVersion::from_type(ado_version.get_type());
    let default_code_id = read_code_id(storage, &default_ado);

    if default_code_id.is_ok() {
        CODE_ID.remove(storage, default_ado.as_str());
    }
    Ok(())
}

pub fn read_code_id(storage: &dyn Storage, ado_version: &ADOVersion) -> StdResult<u64> {
    if ado_version.get_version() == "latest" {
        let (_version, code_id) = read_latest_code_id(storage, ado_version.get_type())?;
        Ok(code_id)
    } else {
        CODE_ID.load(storage, ado_version.as_str())
    }
}

pub fn read_latest_code_id(storage: &dyn Storage, ado_type: String) -> StdResult<(String, u64)> {
    LATEST_VERSION.load(storage, &ado_type)
}

pub fn save_action_fees(
    storage: &mut dyn Storage,
    api: &dyn Api,
    ado_version: &ADOVersion,
    fees: Vec<ActionFee>,
) -> Result<(), ContractError> {
    for action_fee in fees {
        action_fee.validate_asset(api)?;
        ACTION_FEES.save(
            storage,
            &(ado_version.get_type(), action_fee.clone().action),
            &action_fee.clone(),
        )?;
    }

    Ok(())
}
