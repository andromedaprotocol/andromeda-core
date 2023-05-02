use cosmwasm_schema::cw_serde;
use cosmwasm_std::{ensure, Addr, DepsMut, MessageInfo, QuerierWrapper, Response, Storage};

use crate::ado_contract::ADOContract;
use crate::amp::addresses::AndrAddr;
use crate::error::ContractError;

#[cw_serde]
enum AppQueryMsg {
    ComponentExists { name: String },
    GetAddress { name: String },
}

impl<'a> ADOContract<'a> {
    pub fn get_app_contract(&self, storage: &dyn Storage) -> Result<Option<Addr>, ContractError> {
        Ok(self.app_contract.may_load(storage)?)
    }

    /// Checks the given component name against the registered app contract to ensure it exists
    pub fn component_exists(
        &self,
        querier: &QuerierWrapper,
        name: String,
        app_contract: Addr,
    ) -> Result<bool, ContractError> {
        Ok(querier.query_wasm_smart(app_contract, &AppQueryMsg::ComponentExists { name })?)
    }

    /// Gets the address for a given component from the registered app contract
    pub fn get_app_component_address(
        &self,
        storage: &dyn Storage,
        querier: &QuerierWrapper,
        name: impl Into<String>,
    ) -> Addr {
        let app_contract = self
            .get_app_contract(storage)
            .expect("A problem occured retrieving the associated app contract")
            .expect("No Associated App Contract");

        let query = AppQueryMsg::GetAddress { name: name.into() };
        querier
            .query_wasm_smart(app_contract, &query)
            .expect("Failed to query app contract for component address")
    }

    pub fn execute_update_app_contract(
        &self,
        deps: DepsMut,
        info: MessageInfo,
        address: String,
        addresses: Option<Vec<AndrAddr>>,
    ) -> Result<Response, ContractError> {
        ensure!(
            self.is_contract_owner(deps.storage, info.sender.as_str())?,
            ContractError::Unauthorized {}
        );
        self.app_contract
            .save(deps.storage, &deps.api.addr_validate(&address)?)?;
        self.validate_andr_addresses(&deps.as_ref(), addresses.unwrap_or_default())?;
        Ok(Response::new()
            .add_attribute("action", "update_app_contract")
            .add_attribute("address", address))
    }
}
