use crate::amp::{ADO_DB_KEY, VFS_KEY};
use crate::error::ContractError;
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{from_json, Addr, QuerierWrapper};
use cw_storage_plus::Path;
use lazy_static::__Deref;
use serde::de::DeserializeOwned;
use std::str::from_utf8;

use super::adodb::{ADOVersion, ActionFee, QueryMsg as ADODBQueryMsg};
use super::kernel::ChannelInfo;

#[cw_serde]
pub struct AOSQuerier();

impl AOSQuerier {
    // namespace -> storage key
    // key_name -> item key
    // Taken from: https://github.com/KompleTeam/komple-framework/blob/387d333af03e794927b8ef8ac536d2a42ae7a1ff/packages/utils/src/storage.rs#L25
    pub fn get_map_storage_key(
        namespace: &str,
        key_bytes: &[&[u8]],
    ) -> Result<String, ContractError> {
        let namespace_bytes = namespace.as_bytes();
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
                let res = from_json(res.as_bytes())?;
                Ok(Some(res))
            }
            None => Ok(None),
        }
    }

    pub fn ado_type_getter(
        querier: &QuerierWrapper,
        adodb_addr: &Addr,
        code_id: u64,
    ) -> Result<Option<String>, ContractError> {
        let key = AOSQuerier::get_map_storage_key("ado_type", &[&code_id.to_be_bytes()])?;
        let ado_type: Option<ADOVersion> = AOSQuerier::query_storage(querier, adodb_addr, &key)?;
        Ok(ado_type.map(|v| v.get_type()))
    }

    pub fn ado_type_getter_smart(
        querier: &QuerierWrapper,
        adodb_addr: &Addr,
        code_id: u64,
    ) -> Result<Option<String>, ContractError> {
        let query = ADODBQueryMsg::ADOType { code_id };
        let ado_type: Option<String> = querier.query_wasm_smart(adodb_addr, &query)?;
        Ok(ado_type)
    }

    pub fn ado_publisher_getter(
        querier: &QuerierWrapper,
        adodb_addr: &Addr,
        ado_type: &str,
    ) -> Result<String, ContractError> {
        let key = AOSQuerier::get_map_storage_key("publisher", &[ado_type.as_bytes()])?;
        let verify: Option<String> = AOSQuerier::query_storage(querier, adodb_addr, &key)?;

        match verify {
            Some(publisher) => Ok(publisher),
            None => Err(ContractError::InvalidAddress {}),
        }
    }

    /// Checks if the code id exists in the ADODB by querying its raw storage for the code id's ado type
    pub fn verify_code_id(
        querier: &QuerierWrapper,
        adodb_addr: &Addr,
        code_id: u64,
    ) -> Result<(), ContractError> {
        let key = AOSQuerier::get_map_storage_key("ado_type", &[&code_id.to_be_bytes()])?;
        let verify: Option<String> = AOSQuerier::query_storage(querier, adodb_addr, &key)?;

        if verify.is_some() {
            Ok(())
        } else {
            Err(ContractError::Unauthorized {})
        }
    }

    pub fn code_id_getter_raw(
        querier: &QuerierWrapper,
        adodb_addr: &Addr,
        ado_type: &str,
    ) -> Result<u64, ContractError> {
        let key = AOSQuerier::get_map_storage_key("code_id", &[ado_type.as_bytes()])?;
        let verify: Option<u64> = AOSQuerier::query_storage(querier, adodb_addr, &key)?;

        match verify {
            Some(code_id) => Ok(code_id),
            None => Err(ContractError::InvalidAddress {}),
        }
    }

    pub fn code_id_getter(
        querier: &QuerierWrapper,
        adodb_addr: &Addr,
        ado_type: &str,
    ) -> Result<u64, ContractError> {
        let query = ADODBQueryMsg::CodeId {
            key: ado_type.to_string(),
        };
        let code_id: u64 = querier.query_wasm_smart(adodb_addr, &query)?;
        Ok(code_id)
    }

    /// Queries the kernel's raw storage for the VFS's address
    pub fn vfs_address_getter(
        querier: &QuerierWrapper,
        kernel_addr: &Addr,
    ) -> Result<Addr, ContractError> {
        AOSQuerier::kernel_address_getter(querier, kernel_addr, VFS_KEY)
    }

    /// Queries the kernel's raw storage for the ADODB's address
    pub fn adodb_address_getter(
        querier: &QuerierWrapper,
        kernel_addr: &Addr,
    ) -> Result<Addr, ContractError> {
        AOSQuerier::kernel_address_getter(querier, kernel_addr, ADO_DB_KEY)
    }

    /// Queries the kernel's raw storage for the VFS's address
    pub fn kernel_address_getter(
        querier: &QuerierWrapper,
        kernel_addr: &Addr,
        key: &str,
    ) -> Result<Addr, ContractError> {
        let key = AOSQuerier::get_map_storage_key("kernel_addresses", &[key.as_bytes()])?;
        let verify: Option<Addr> = AOSQuerier::query_storage(querier, kernel_addr, &key)?;
        match verify {
            Some(address) => Ok(address),
            None => Err(ContractError::InvalidAddress {}),
        }
    }

    pub fn action_fee_getter(
        querier: &QuerierWrapper,
        adodb_addr: &Addr,
        ado_type: &str,
        action: &str,
    ) -> Result<Option<ActionFee>, ContractError> {
        let key = AOSQuerier::get_map_storage_key(
            "action_fees",
            &[ado_type.as_bytes(), action.as_bytes()],
        )?;
        let fee: Option<ActionFee> = AOSQuerier::query_storage(querier, adodb_addr, &key)?;

        Ok(fee)
    }

    /// Queries the kernel's raw storage for the VFS's address
    pub fn ado_owner_getter(
        querier: &QuerierWrapper,
        ado_addr: &Addr,
    ) -> Result<Addr, ContractError> {
        let verify: Option<Addr> = AOSQuerier::query_storage(querier, ado_addr, "owner")?;
        match verify {
            Some(address) => Ok(address),
            None => Err(ContractError::InvalidAddress {}),
        }
    }

    /// Queries the current chain name from the kernel
    pub fn get_current_chain(
        querier: &QuerierWrapper,
        kernel_addr: &Addr,
    ) -> Result<String, ContractError> {
        let verify: Option<String> =
            AOSQuerier::query_storage(querier, kernel_addr, "kernel_curr_chain")?;
        match verify {
            Some(chain) => Ok(chain),
            None => Err(ContractError::InvalidAddress {}),
        }
    }

    /// Queries the current chain name from the kernel
    pub fn get_chain_info(
        querier: &QuerierWrapper,
        kernel_addr: &Addr,
        chain_name: &str,
    ) -> Result<ChannelInfo, ContractError> {
        let key = AOSQuerier::get_map_storage_key("kernel_channels", &[chain_name.as_bytes()])?;
        let verify: Option<ChannelInfo> =
            AOSQuerier::query_storage(querier, kernel_addr, key.as_str())?;
        match verify {
            Some(chain) => Ok(chain),
            None => Err(ContractError::InvalidAddress {}),
        }
    }
}
