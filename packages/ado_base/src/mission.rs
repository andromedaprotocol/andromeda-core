use cosmwasm_std::{Addr, Api, Deps, Env, MessageInfo, QuerierWrapper, Response, Storage};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ADOContract;
use common::{error::ContractError, mission::AndrAddress, require};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
enum MissionQueryMsg {
    ComponentExists { name: String },
}

impl<'a> ADOContract<'a> {
    pub(crate) fn validate_andr_addresses(
        &self,
        deps: Deps,
        mut addresses: Vec<AndrAddress>,
    ) -> Result<Response, ContractError> {
        let mission_contract = self.get_mission_contract(deps.storage)?;
        require(
            mission_contract.is_some(),
            ContractError::MissionContractNotSpecified {},
        )?;
        #[cfg(feature = "modules")]
        {
            let modules = self.load_modules(deps.storage)?;
            if !modules.is_empty() {
                let andr_addresses: Vec<AndrAddress> =
                    modules.into_iter().map(|m| m.address).collect();
                addresses.extend(andr_addresses);
            }
        }
        let mission_contract = mission_contract.unwrap();
        for address in addresses {
            self.validate_andr_address(
                deps.api,
                &deps.querier,
                address.identifier,
                mission_contract.clone(),
            )?;
        }
        Ok(Response::new())
    }

    pub fn get_mission_contract(
        &self,
        storage: &dyn Storage,
    ) -> Result<Option<Addr>, ContractError> {
        Ok(self.mission_contract.may_load(storage)?)
    }

    fn component_exists(
        &self,
        querier: &QuerierWrapper,
        name: String,
        mission_contract: Addr,
    ) -> Result<bool, ContractError> {
        Ok(querier
            .query_wasm_smart(mission_contract, &MissionQueryMsg::ComponentExists { name })?)
    }

    pub(crate) fn validate_andr_address(
        &self,
        api: &dyn Api,
        querier: &QuerierWrapper,
        identifier: String,
        mission_contract: Addr,
    ) -> Result<(), ContractError> {
        // If the address passes this check then it doesn't refer to a mission component by
        // name.
        if api.addr_validate(&identifier).is_err() {
            require(
                self.component_exists(&querier, identifier.clone(), mission_contract.clone())?,
                ContractError::InvalidComponent { name: identifier },
            )?;
        }
        Ok(())
    }
}
