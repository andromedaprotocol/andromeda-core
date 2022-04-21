use cosmwasm_std::{Addr, Deps, Env, MessageInfo, QuerierWrapper, Response, Storage};
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
    pub fn validate_andr_addresses(
        &self,
        deps: Deps,
        env: Env,
        info: MessageInfo,
        addresses: Vec<&AndrAddress>,
    ) -> Result<Response, ContractError> {
        require(
            info.sender == env.contract.address,
            ContractError::Unauthorized {},
        )?;
        let mission_contract = self.get_mission_contract(deps.storage)?;
        require(
            mission_contract.is_some(),
            ContractError::MissionContractNotSpecified {},
        )?;
        let mission_contract = mission_contract.unwrap();
        for address in addresses {
            require(
                self.component_exists(
                    &deps.querier,
                    address.identifier.clone(),
                    mission_contract.clone(),
                )?,
                ContractError::InvalidComponent {
                    name: address.identifier.clone(),
                },
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
}
