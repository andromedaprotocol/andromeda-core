use crate::ado_contract::ADOContract;
use crate::error::ContractError;
use cosmwasm_std::{attr, ensure, DepsMut, MessageInfo, Order, Response, Storage};

impl<'a> ADOContract<'a> {
    /// Updates the current contract owner. **Only executable by the current contract owner.**
    pub fn execute_update_owner(
        &self,
        deps: DepsMut,
        info: MessageInfo,
        new_owner: String,
    ) -> Result<Response, ContractError> {
        ensure!(
            self.is_contract_owner(deps.storage, info.sender.as_str())?,
            ContractError::Unauthorized {}
        );
        let new_owner_addr = deps.api.addr_validate(&new_owner)?;
        self.owner.save(deps.storage, &new_owner_addr)?;

        Ok(Response::new().add_attributes(vec![
            attr("action", "update_owner"),
            attr("value", new_owner),
        ]))
    }

    pub fn execute_update_operators(
        &self,
        deps: DepsMut,
        info: MessageInfo,
        operators: Vec<String>,
    ) -> Result<Response, ContractError> {
        ensure!(
            self.is_contract_owner(deps.storage, info.sender.as_str())?,
            ContractError::Unauthorized {}
        );

        let keys: Vec<String> = self
            .operators
            .keys(deps.storage, None, None, Order::Ascending)
            .collect::<Result<Vec<String>, _>>()?;
        for key in keys.iter() {
            self.operators.remove(deps.storage, key);
        }

        for op in operators.iter() {
            self.operators.save(deps.storage, op, &true)?;
        }

        Ok(Response::new().add_attributes(vec![attr("action", "update_operators")]))
    }

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

    pub fn is_owner_or_operator(
        &self,
        storage: &dyn Storage,
        addr: &str,
    ) -> Result<bool, ContractError> {
        Ok(self.is_contract_owner(storage, addr)? || self.is_operator(storage, addr))
    }
}
