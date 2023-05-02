#[cfg(feature = "modules")]
use crate::ado_base::modules::Module;
use cosmwasm_std::Addr;
#[cfg(feature = "withdraw")]
use cw_asset::AssetInfo;
use cw_storage_plus::{Item, Map};

pub struct ADOContract<'a> {
    pub(crate) owner: Item<'a, Addr>,
    pub(crate) original_publisher: Item<'a, Addr>,
    pub(crate) block_height: Item<'a, u64>,
    pub(crate) operators: Map<'a, &'a str, bool>,
    pub(crate) ado_type: Item<'a, String>,
    pub(crate) version: Item<'a, String>,
    pub(crate) app_contract: Item<'a, Addr>,
    pub(crate) kernel_address: Item<'a, Addr>,
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
            #[cfg(feature = "modules")]
            module_info: Map::new("andr_modules"),
            #[cfg(feature = "modules")]
            module_idx: Item::new("andr_module_idx"),
            #[cfg(feature = "withdraw")]
            withdrawable_tokens: Map::new("withdrawable_tokens"),
        }
    }
}
