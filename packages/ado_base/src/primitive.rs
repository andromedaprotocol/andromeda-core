use crate::ADOContract;

use common::{
    ado_base::query_get, encode_binary, error::ContractError, primitive::GetValueResponse,
};
use cosmwasm_std::{DepsMut, Order, QuerierWrapper, Response, Storage};
use cw_storage_plus::Bound;

const DEFAULT_LIMIT: u32 = 10;
const MAX_LIMIT: u32 = 20;

impl<'a> ADOContract<'a> {
    /// Gets the value of `contract` in `self.cached_addresses`.
    pub fn get_cached_address(
        &self,
        storage: &dyn Storage,
        contract: &str,
    ) -> Result<String, ContractError> {
        Ok(self.cached_addresses.load(storage, contract)?)
    }

    /// Queries the primitive contract to get the [`String`] stored with key `contract`. The result
    /// is then stored in `self.cached_addresses` to be read later.
    pub fn cache_address(
        &self,
        storage: &mut dyn Storage,
        querier: &QuerierWrapper,
        contract: &str,
    ) -> Result<(), ContractError> {
        let address = self.get_address_from_primitive(storage, querier, contract)?;
        self.cached_addresses.save(storage, contract, &address)?;
        Ok(())
    }

    /// Gets the address for `contract` stored in the primitive contract.
    pub fn get_address_from_primitive(
        &self,
        storage: &dyn Storage,
        querier: &QuerierWrapper,
        contract: &str,
    ) -> Result<String, ContractError> {
        let primitive_address = self.primitive_contract.load(storage)?;
        let data = encode_binary(&contract)?;
        let res: GetValueResponse = query_get(Some(data), primitive_address.to_string(), querier)?;
        let address = res.value.try_get_string()?;

        Ok(address)
    }

    pub(crate) fn execute_refresh_address(
        &self,
        deps: DepsMut,
        contract: String,
    ) -> Result<Response, ContractError> {
        self.cache_address(deps.storage, &deps.querier, &contract)?;
        Ok(Response::new()
            .add_attribute("action", "refresh_address")
            .add_attribute("contract", contract))
    }

    pub(crate) fn execute_refresh_addresses(
        &self,
        deps: DepsMut,
        start_after: Option<String>,
        limit: Option<u32>,
    ) -> Result<Response, ContractError> {
        let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
        let start = start_after.map(Bound::exclusive);
        let keys: Vec<String> = self
            .cached_addresses
            .keys(deps.storage, start, None, Order::Ascending)
            .take(limit)
            .map(String::from_utf8)
            .collect::<Result<Vec<String>, _>>()?;

        for key in keys.iter() {
            self.cache_address(deps.storage, &deps.querier, &key)?;
        }

        Ok(Response::new()
            .add_attribute("action", "refresh_addresses")
            .add_attribute("last_key", keys.last().unwrap_or(&String::from("None"))))
    }
}
