#[cfg(feature = "modules")]
use common::ado_base::modules::Module;
use common::{error::ContractError, parse_message};
use cosmwasm_std::{Addr, Binary, Storage};
#[cfg(feature = "withdraw")]
use cw_asset::AssetInfo;
use cw_storage_plus::{Item, Map};
use serde::de::DeserializeOwned;

pub struct ADOContract<'a> {
    pub(crate) owner: Item<'a, Addr>,
    pub(crate) operators: Map<'a, &'a str, bool>,
    pub(crate) ado_type: Item<'a, String>,
    pub(crate) mission_contract: Item<'a, Addr>,
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
            operators: Map::new("operators"),
            ado_type: Item::new("ado_type"),
            mission_contract: Item::new("mission_contract"),
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
    pub fn get_mission_contract(
        &self,
        storage: &dyn Storage,
    ) -> Result<Option<Addr>, ContractError> {
        Ok(self.mission_contract.may_load(storage)?)
    }

    pub(crate) fn initialize_operators(
        &self,
        storage: &mut dyn Storage,
        operators: Vec<String>,
    ) -> Result<(), ContractError> {
        for operator in operators.iter() {
            self.operators.save(storage, operator, &true)?;
        }
        Ok(())
    }

    pub(crate) fn is_nested<T: DeserializeOwned>(&self, data: &Option<Binary>) -> bool {
        let res: Result<T, ContractError> = parse_message(data);
        res.is_ok()
    }
}
