use andromeda_protocol::{
    communication::{parse_message, QueryMsg},
    error::ContractError,
};
use cosmwasm_std::{Addr, Binary, Storage};
use cw_storage_plus::{Item, Map};

pub struct ADOContract<'a> {
    pub owner: Item<'a, Addr>,
    pub operators: Map<'a, &'a str, bool>,
}

impl<'a> Default for ADOContract<'a> {
    fn default() -> Self {
        ADOContract {
            owner: Item::new("owner"),
            operators: Map::new("operators"),
        }
    }
}

impl<'a> ADOContract<'a> {
    /// Helper function to query if a given address is a operator.
    ///
    /// Returns a boolean value indicating if the given address is a operator.
    pub fn is_operator(&self, storage: &dyn Storage, addr: &str) -> bool {
        self.operators.has(storage, addr)
    }

    /// Helper function to query if a given address is the current contract owner.
    ///
    /// Returns a boolean value indicating if the given address is the contract owner.
    pub fn is_contract_owner(
        &self,
        storage: &dyn Storage,
        addr: &str,
    ) -> Result<bool, ContractError> {
        let owner = self.owner.load(storage)?;
        Ok(addr == owner)
    }

    pub fn initialize_operators(
        &self,
        storage: &mut dyn Storage,
        operators: Vec<String>,
    ) -> Result<(), ContractError> {
        for operator in operators.iter() {
            self.operators.save(storage, operator, &true)?;
        }
        Ok(())
    }

    pub(crate) fn is_nested(&self, data: &Option<Binary>) -> bool {
        let res: Result<QueryMsg, ContractError> = parse_message(data);
        return res.is_ok();
    }
}
