use crate::state::{
    read_code_id, read_latest_code_id, ACTION_FEES, ADO_TYPE, CODE_ID, PUBLISHER,
    UNPUBLISHED_CODE_IDS,
};

use andromeda_std::error::ContractError;
use andromeda_std::os::adodb::{ADOMetadata, ADOVersion, ActionFee, IsUnpublishedCodeIdResponse};
use cosmwasm_std::{Deps, Order, StdResult, Storage};

use cw_storage_plus::Bound;
use semver::Version;

pub fn code_id(deps: Deps, key: String) -> Result<u64, ContractError> {
    let code_id = read_code_id(deps.storage, &ADOVersion::from_string(key))?;
    Ok(code_id)
}

pub fn unpublished_code_ids(deps: Deps) -> Result<Vec<u64>, ContractError> {
    let unpublished_code_ids = UNPUBLISHED_CODE_IDS.may_load(deps.storage)?;
    if let Some(ids) = unpublished_code_ids {
        Ok(ids)
    } else {
        Ok(vec![])
    }
}

pub fn is_unpublished_code_id(
    deps: Deps,
    code_id: u64,
) -> Result<IsUnpublishedCodeIdResponse, ContractError> {
    let unpublished_code_ids = UNPUBLISHED_CODE_IDS.may_load(deps.storage)?;
    if let Some(ids) = unpublished_code_ids {
        Ok(IsUnpublishedCodeIdResponse {
            is_unpublished_code_id: ids.contains(&code_id),
        })
    } else {
        Ok(IsUnpublishedCodeIdResponse {
            is_unpublished_code_id: false,
        })
    }
}

pub fn ado_type(deps: Deps, code_id: u64) -> Result<Option<String>, ContractError> {
    let ado_version = ADO_TYPE.may_load(deps.storage, code_id)?;
    Ok(ado_version)
}

const DEFAULT_LIMIT: u32 = 10;
const MAX_LIMIT: u32 = 100;

pub fn all_ado_types(
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

pub fn ado_versions(
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

pub fn ado_metadata(deps: Deps, ado_type: String) -> Result<ADOMetadata, ContractError> {
    let ado_version = ADOVersion::from_string(ado_type);
    let publisher = PUBLISHER.load(deps.storage, ado_version.as_str())?;
    let latest_version = read_latest_code_id(deps.storage, ado_version.get_type())?;

    Ok(ADOMetadata {
        publisher,
        latest_version: latest_version.0,
    })
}

pub fn action_fee(
    deps: Deps,
    ado_type: String,
    action: String,
) -> Result<Option<ActionFee>, ContractError> {
    let ado_version = ADOVersion::from_string(ado_type);
    Ok(ACTION_FEES.may_load(deps.storage, &(ado_version.into_string(), action))?)
}

pub fn action_fee_by_code_id(
    deps: Deps,
    code_id: u64,
    action: String,
) -> Result<Option<ActionFee>, ContractError> {
    let ado_version = ADO_TYPE.load(deps.storage, code_id)?;
    Ok(ACTION_FEES.may_load(deps.storage, &(ado_version, action))?)
}
