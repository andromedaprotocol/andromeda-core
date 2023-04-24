use crate::ado_contract::{modules::Module, ADOContract};
use crate::error::ContractError;
use cosmwasm_std::{Deps, Order, Uint64};
use cw_storage_plus::Bound;

impl<'a> ADOContract<'a> {
    /// Queries a module by its id.
    pub fn query_module(&self, deps: Deps, id: Uint64) -> Result<Module, ContractError> {
        let id = id.to_string();
        let module = self.module_info.load(deps.storage, &id)?;
        Ok(module)
    }

    /// Queries all of the module ids.
    pub fn query_module_ids(&self, deps: Deps) -> Result<Vec<String>, ContractError> {
        let module_idx = self.module_idx.may_load(deps.storage)?.unwrap_or(1);
        let min = Some(Bound::inclusive("1"));
        let module_ids: Result<Vec<String>, _> = self
            .module_info
            .keys(deps.storage, min, None, Order::Ascending)
            .take(module_idx as usize)
            .collect();
        Ok(module_ids?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ado_contract::modules::Module;
    use cosmwasm_std::{testing::mock_dependencies, Addr};

    #[test]
    fn test_query_module() {
        let contract = ADOContract::default();
        let mut deps = mock_dependencies();

        contract
            .owner
            .save(deps.as_mut().storage, &Addr::unchecked("owner"))
            .unwrap();

        let module1 = Module {
            module_name: Some("module_name1".to_string()),
            address: "address1".to_string(),

            is_mutable: true,
        };

        let module2 = Module {
            module_name: Some("module_name2".to_string()),
            address: "address2".to_string(),

            is_mutable: true,
        };

        contract
            .module_info
            .save(deps.as_mut().storage, "1", &module1)
            .unwrap();

        contract
            .module_info
            .save(deps.as_mut().storage, "2", &module2)
            .unwrap();

        contract.module_idx.save(deps.as_mut().storage, &2).unwrap();

        let res = contract
            .query_module(deps.as_ref(), Uint64::from(1u64))
            .unwrap();

        assert_eq!(module1, res);

        let res = contract
            .query_module(deps.as_ref(), Uint64::from(2u64))
            .unwrap();

        assert_eq!(module2, res);

        let res = contract.query_module_ids(deps.as_ref()).unwrap();
        assert_eq!(vec![String::from("1"), String::from("2")], res);
    }
}
