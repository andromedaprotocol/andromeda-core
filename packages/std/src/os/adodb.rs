use crate::ado_base::{AndromedaMsg, AndromedaQuery};
use crate::error::ContractError;
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{from_slice, Addr, QuerierWrapper};
use cw_storage_plus::Path;
use lazy_static::__Deref;
use serde::de::DeserializeOwned;
use std::str::from_utf8;

#[cw_serde]
pub struct InstantiateMsg {
    pub kernel_address: String,
    pub owner: Option<String>,
}

#[cw_serde]
pub enum ExecuteMsg {
    AndrReceive(AndromedaMsg),
    UpdateCodeId { code_id_key: String, code_id: u64 },
}

#[cw_serde]
pub struct MigrateMsg {}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(AndromedaQuery)]
    AndrQuery(AndromedaQuery),
    /// All code IDs for Andromeda contracts
    #[returns(u64)]
    CodeId { key: String },
    #[returns(Option<String>)]
    ADOType { code_id: u64 },
}

#[cw_serde]
pub struct StorageHelper();

impl StorageHelper {
    // namespace -> storage key
    // key_name -> item key
    pub fn get_map_storage_key(
        namepspace: &str,
        key_bytes: &[&[u8]],
    ) -> Result<String, ContractError> {
        let namespace_bytes = namepspace.as_bytes();
        let path: Path<Vec<u32>> = Path::new(namespace_bytes, key_bytes);
        let path_str = from_utf8(path.deref())?;
        Ok(path_str.to_string())
    }

    // To find the key value in storage, we need to construct a path to the key
    // For Map storage this key is generated with get_map_storage_key
    // For Item storage this key is the namespace value
    pub fn query_storage<T>(
        querier: &QuerierWrapper,
        addr: &Addr,
        key: &str,
    ) -> Result<Option<T>, ContractError>
    where
        T: DeserializeOwned,
    {
        let data = querier.query_wasm_raw(addr, key.as_bytes())?;
        match data {
            Some(data) => {
                let res = from_utf8(&data)?;
                let res = from_slice(res.as_bytes())?;
                Ok(Some(res))
            }
            None => Ok(None),
        }
    }
}
