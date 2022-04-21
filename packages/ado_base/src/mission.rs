use cosmwasm_std::{Addr, Deps, Env, MessageInfo, Storage};

use crate::ADOContract;
use common::{error::ContractError, mission::AndrAddress, require};

impl<'a> ADOContract<'a> {
    pub fn validate_andr_addresses(
        &self,
        deps: Deps,
        env: Env,
        info: MessageInfo,
        addresses: Vec<&AndrAddress>,
    ) -> Result<Vec<Addr>, ContractError> {
        require(
            info.sender == env.contract.address,
            ContractError::Unauthorized {},
        )?;
        let mut true_addresses = vec![];
        let mission_contract = self.get_mission_contract(deps.storage)?;
        for address in addresses {
            // If this errors, the identifier was invalid.
            true_addresses.push(deps.api.addr_validate(&address.get_address(
                deps.api,
                &deps.querier,
                mission_contract.clone(),
            )?)?);
        }
        Ok(true_addresses)
    }

    pub fn get_mission_contract(
        &self,
        storage: &dyn Storage,
    ) -> Result<Option<Addr>, ContractError> {
        Ok(self.mission_contract.may_load(storage)?)
    }
}
