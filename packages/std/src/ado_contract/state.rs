#[cfg(feature = "rates")]
use crate::ado_base::rates::Rate;
use cosmwasm_std::Addr;
use cw_storage_plus::{Item, Map};

pub struct ADOContract<'a> {
    pub(crate) owner: Item<'a, Addr>,
    pub(crate) original_publisher: Item<'a, Addr>,
    pub(crate) block_height: Item<'a, u64>,
    pub(crate) ado_type: Item<'a, String>,
    pub(crate) app_contract: Item<'a, Addr>,
    pub(crate) kernel_address: Item<'a, Addr>,
    pub(crate) permissioned_actions: Map<'a, String, bool>,
    #[cfg(feature = "rates")]
    /// Mapping of action to rate
    pub rates: Map<'a, &'a str, Rate>,
}

impl<'a> Default for ADOContract<'a> {
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
