use crate::ADOContract;

use common::{
    ado_base::query_get, encode_binary, error::ContractError, primitive::GetValueResponse,
};
use cosmwasm_std::{QuerierWrapper, Storage};

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

    pub(crate) fn get_address_from_primitive(
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
}
