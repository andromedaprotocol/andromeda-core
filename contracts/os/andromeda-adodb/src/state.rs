use andromeda_std::os::adodb::{ADOVersion, ActionFee};
use cosmwasm_std::{StdResult, Storage};
use cw_storage_plus::Map;

/// Stores a mapping between ADO type and its latest code ID
pub const CODE_ID: Map<&str, u64> = Map::new("code_id");
/// Stores a mapping between a code ID and its type
pub const ADO_TYPE: Map<u64, String> = Map::new("ado_type");
/// Stores a mapping between a code ID and its publisher
pub const PUBLISHER: Map<String, String> = Map::new("publisher");
/// Stores a mapping between an ADO type/version and its code ID
pub const VERSION_CODE_ID: Map<(String, String), u64> = Map::new("version_code_id");
/// Stores a mapping between an ADO type and its action fees
pub const ACTION_FEES: Map<(String, String), ActionFee> = Map::new("action_fees");
/// Stores the latest version for a given ADO type
pub const LATEST_VERSION: Map<String, String> = Map::new("latest_version");

pub fn store_code_id(
    storage: &mut dyn Storage,
    ado_version: &ADOVersion,
    code_id: u64,
) -> StdResult<()> {
    CODE_ID.save(storage, &ado_version.get_type(), &code_id)?;
    ADO_TYPE.save(storage, code_id, &ado_version.get_type())?;
    LATEST_VERSION.save(storage, ado_version.get_type(), &ado_version.get_version())?;
    VERSION_CODE_ID.save(
        storage,
        (ado_version.get_type(), ado_version.get_version()),
        &code_id,
    )
}

pub fn read_code_id(storage: &dyn Storage, code_id_key: &str) -> StdResult<u64> {
    CODE_ID.load(storage, code_id_key)
}
