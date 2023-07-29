use andromeda_std::os::adodb::{ADOVersion, ActionFee};
use cosmwasm_std::{StdResult, Storage, Order};
use cw_storage_plus::Map;

/// Stores a mapping from an ADO type/version to its code ID
pub const CODE_ID: Map<&(String, String), u64> = Map::new("code_id");
/// Stores the latest version for a given ADO
pub const LATEST_VERSION: Map<&String, (String,u64)> = Map::new("latest_version");
/// Stores a mapping from code ID to ADO
pub const ADO_TYPE: Map<u64, ADOVersion> = Map::new("ado_type");
/// Stores a mapping from ADO to its publisher
pub const PUBLISHER: Map<&(String, String), String> = Map::new("publisher");
/// Stores a mapping from an ADO to its action fees
pub const ACTION_FEES: Map<&(String, String, String), ActionFee> = Map::new("action_fees");

pub fn store_code_id(
    storage: &mut dyn Storage,
    ado_version: &ADOVersion,
    code_id: u64,
) -> StdResult<()> {
    let _ = ADO_TYPE.save(storage, code_id, &ado_version.clone());
    let _ = LATEST_VERSION.save(storage, &ado_version.get_type(), &(ado_version.get_version(),code_id));
    CODE_ID.save(
        storage,
        &ado_version.get_tuple(),
        &code_id,
    )
}

pub fn read_code_id(storage: &dyn Storage, ado_version: &ADOVersion) -> StdResult<u64> {
    CODE_ID.load(storage, &ado_version.get_tuple())
}

pub fn read_latest_code_id(storage: &dyn Storage, ado_type: String) -> StdResult<(String,u64)> {
    LATEST_VERSION.load(storage, &ado_type)
}

pub fn read_all_ado_types(storage: &dyn Storage) -> StdResult<Vec<String>> {
    let ado_types = ADO_TYPE.range(storage, None, None, Order::Ascending)
    .flatten()
    .map(|(_, ado_version)|ado_version.clone().into_string()).collect();
    Ok(ado_types)
}
