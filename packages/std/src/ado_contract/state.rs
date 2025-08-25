#[cfg(feature = "rates")]
use crate::ado_base::rates::Rate;
use crate::common::Milliseconds;
use cosmwasm_std::Addr;
use cw_storage_plus::{Item, Map};

pub struct ADOContract {
    pub(crate) owner: Item<Addr>,
    pub(crate) original_publisher: Item<Addr>,
    pub(crate) block_height: Item<u64>,
    pub(crate) ado_type: Item<String>,
    pub(crate) app_contract: Item<Addr>,
    pub(crate) kernel_address: Item<Addr>,
    pub(crate) permissioned_actions: Map<String, Option<Milliseconds>>,
    #[cfg(feature = "rates")]
    /// Mapping of action to rate
    pub rates: Map<String, Rate>,
}

impl Default for ADOContract {
    fn default() -> Self {
        ADOContract {
            owner: Item::new("owner"),
            original_publisher: Item::new("original_publisher"),
            block_height: Item::new("block_height"),
            ado_type: Item::new("ado_type"),
            app_contract: Item::new("app_contract"),
            kernel_address: Item::new("kernel_address"),
            permissioned_actions: Map::new("andr_permissioned_actions"),
            #[cfg(feature = "rates")]
            rates: Map::new("rates"),
        }
    }
}
