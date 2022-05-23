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
        // as_deref does Option<String> to Option<&str>
        let start = start_after.as_deref().map(Bound::exclusive);
        let keys: Result<Vec<String>, _> = self
            .cached_addresses
            .keys(deps.storage, start, None, Order::Ascending)
            .take(limit)
            .collect();

        let keys = keys?;
        for key in keys.iter() {
            self.cache_address(deps.storage, &deps.querier, key)?;
        }

        Ok(Response::new()
            .add_attribute("action", "refresh_addresses")
            .add_attribute("last_key", keys.last().unwrap_or(&String::from("None"))))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mock_querier::{mock_dependencies_custom, MOCK_PRIMITIVE_CONTRACT};
    use cosmwasm_std::Addr;

    #[test]
    fn test_cache_address() {
        let mut deps = mock_dependencies_custom(&[]);

        let contract = ADOContract::default();
        contract
            .primitive_contract
            .save(
                deps.as_mut().storage,
                &Addr::unchecked(MOCK_PRIMITIVE_CONTRACT),
            )
            .unwrap();

        let deps_mut = deps.as_mut();
        contract
            .cache_address(deps_mut.storage, &deps_mut.querier, "key1")
            .unwrap();

        assert_eq!(
            "address1",
            contract
                .get_cached_address(deps.as_ref().storage, "key1")
                .unwrap()
        );
    }

    #[test]
    fn test_execute_refresh_address() {
        let mut deps = mock_dependencies_custom(&[]);

        let contract = ADOContract::default();
        contract
            .primitive_contract
            .save(
                deps.as_mut().storage,
                &Addr::unchecked(MOCK_PRIMITIVE_CONTRACT),
            )
            .unwrap();

        let res = contract
            .execute_refresh_address(deps.as_mut(), "key1".to_string())
            .unwrap();

        assert_eq!(
            Response::new()
                .add_attribute("action", "refresh_address")
                .add_attribute("contract", "key1"),
            res
        );

        assert_eq!(
            "address1",
            contract
                .get_cached_address(deps.as_ref().storage, "key1")
                .unwrap()
        );
    }

    #[test]
    fn test_execute_refresh_addresses() {
        let mut deps = mock_dependencies_custom(&[]);

        let contract = ADOContract::default();
        contract
            .primitive_contract
            .save(
                deps.as_mut().storage,
                &Addr::unchecked(MOCK_PRIMITIVE_CONTRACT),
            )
            .unwrap();

        contract
            .cached_addresses
            .save(deps.as_mut().storage, "key1", &"stale_address1".to_string())
            .unwrap();
        contract
            .cached_addresses
            .save(deps.as_mut().storage, "key2", &"stale_address2".to_string())
            .unwrap();

        let res = contract
            .execute_refresh_addresses(deps.as_mut(), None, None)
            .unwrap();

        assert_eq!(
            Response::new()
                .add_attribute("action", "refresh_addresses")
                .add_attribute("last_key", "key2"),
            res
        );

        assert_eq!(
            "address1",
            contract
                .get_cached_address(deps.as_ref().storage, "key1")
                .unwrap()
        );
        assert_eq!(
            "address2",
            contract
                .get_cached_address(deps.as_ref().storage, "key2")
                .unwrap()
        );
    }

    #[test]
    fn test_get_address_from_primitive() {
        let mut deps = mock_dependencies_custom(&[]);

        let contract = ADOContract::default();
        contract
            .primitive_contract
            .save(
                deps.as_mut().storage,
                &Addr::unchecked(MOCK_PRIMITIVE_CONTRACT),
            )
            .unwrap();

        assert_eq!(
            "address1",
            contract
                .get_address_from_primitive(deps.as_ref().storage, &deps.as_ref().querier, "key1")
                .unwrap()
        )
    }
}
