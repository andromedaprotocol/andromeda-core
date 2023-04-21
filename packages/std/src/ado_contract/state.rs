#[cfg(feature = "modules")]
use crate::ado_base::modules::Module;
#[cfg(feature = "vfs")]
use crate::os::{kernel::QueryMsg as KernelQueryMsg, vfs::QueryMsg as VFSQueryMsg};
use crate::{error::ContractError, parse_message};
use cosmwasm_std::{Addr, Binary};
#[cfg(feature = "vfs")]
use cosmwasm_std::{Api, QuerierWrapper, Storage};
#[cfg(feature = "withdraw")]
use cw_asset::AssetInfo;
use cw_storage_plus::{Item, Map};
use serde::de::DeserializeOwned;

pub struct ADOContract<'a> {
    pub(crate) owner: Item<'a, Addr>,
    pub(crate) original_publisher: Item<'a, Addr>,
    pub(crate) block_height: Item<'a, u64>,
    pub(crate) operators: Map<'a, &'a str, bool>,
    pub(crate) ado_type: Item<'a, String>,
    pub(crate) version: Item<'a, String>,
    pub(crate) app_contract: Item<'a, Addr>,
    pub(crate) kernel_address: Item<'a, Addr>,
    #[cfg(feature = "primitive")]
    pub(crate) primitive_contract: Item<'a, Addr>,
    #[cfg(feature = "primitive")]
    pub(crate) cached_addresses: Map<'a, &'a str, String>,
    #[cfg(feature = "modules")]
    pub(crate) module_info: Map<'a, &'a str, Module>,
    #[cfg(feature = "modules")]
    pub(crate) module_idx: Item<'a, u64>,
    #[cfg(feature = "withdraw")]
    pub withdrawable_tokens: Map<'a, &'a str, AssetInfo>,
}

impl<'a> Default for ADOContract<'a> {
    fn default() -> Self {
        ADOContract {
            owner: Item::new("owner"),
            original_publisher: Item::new("original_publisher"),
            block_height: Item::new("block_height"),
            operators: Map::new("operators"),
            ado_type: Item::new("ado_type"),
            version: Item::new("version"),
            app_contract: Item::new("app_contract"),
            kernel_address: Item::new("kernel_address"),
            #[cfg(feature = "primitive")]
            primitive_contract: Item::new("primitive_contract"),
            #[cfg(feature = "primitive")]
            cached_addresses: Map::new("cached_addresses"),
            #[cfg(feature = "modules")]
            module_info: Map::new("andr_modules"),
            #[cfg(feature = "modules")]
            module_idx: Item::new("andr_module_idx"),
            #[cfg(feature = "withdraw")]
            withdrawable_tokens: Map::new("withdrawable_tokens"),
        }
    }
}

impl<'a> ADOContract<'a> {
    pub(crate) fn is_nested<T: DeserializeOwned>(&self, data: &Option<Binary>) -> bool {
        let res: Result<T, ContractError> = parse_message(data);
        res.is_ok()
    }

    #[cfg(feature = "vfs")]
    /// Resolves a given path by querying the VFS address from the kernel and then using the VFS to resolve the given path
    pub fn resolve_path(
        &self,
        storage: &dyn Storage,
        querier: &QuerierWrapper,
        api: &dyn Api,
        path: String,
    ) -> Result<String, ContractError> {
        let kernel_address = self.kernel_address.may_load(storage)?;
        if let Some(kernel_address) = kernel_address {
            let vfs_address_query = KernelQueryMsg::KeyAddress {
                key: "vfs".to_string(),
            };
            let vfs_address: Addr = querier.query_wasm_smart(kernel_address, &vfs_address_query)?;
            let resolve_path_query = VFSQueryMsg::ResolvePath { path };
            let res: String = querier.query_wasm_smart(vfs_address, &resolve_path_query)?;
            Ok(res)
        } else {
            let validated_address = api.addr_validate(&path)?.into_string();
            Ok(validated_address)
        }
    }
}
