use cosmwasm_schema::cw_serde;
use cosmwasm_std::{ensure, Addr, Api, Deps, QuerierWrapper, Storage};

use crate::ADOContract;
use common::error::ContractError;

#[cw_serde]
enum AppQueryMsg {
    ComponentExists { name: String },
}

impl<'a> ADOContract<'a> {
    pub fn get_app_contract(&self, storage: &dyn Storage) -> Result<Option<Addr>, ContractError> {
        Ok(self.app_contract.may_load(storage)?)
    }

    pub(crate) fn validate_andr_addresses(
        &self,
        deps: Deps,
        mut addresses: Vec<String>,
    ) -> Result<(), ContractError> {
        let app_contract = self.get_app_contract(deps.storage)?;
        ensure!(
            app_contract.is_some(),
            ContractError::AppContractNotSpecified {}
        );
        #[cfg(feature = "modules")]
        {
            let modules = self.load_modules(deps.storage)?;
            if !modules.is_empty() {
                let andr_addresses: Vec<String> = modules.into_iter().map(|m| m.address).collect();
                addresses.extend(andr_addresses);
            }
        }
        let app_contract = app_contract.unwrap();
        for address in addresses {
            self.validate_andr_address(deps.api, &deps.querier, address, app_contract.clone())?;
        }
        Ok(())
    }

    pub(crate) fn validate_andr_address(
        &self,
        api: &dyn Api,
        querier: &QuerierWrapper,
        identifier: String,
        app_contract: Addr,
    ) -> Result<(), ContractError> {
        // If the address passes this check then it doesn't refer to a app component by
        // name.
        if api.addr_validate(&identifier).is_err() {
            ensure!(
                self.component_exists(querier, identifier.clone(), app_contract)?,
                ContractError::InvalidComponent { name: identifier }
            );
        }
        Ok(())
    }

    // pub(crate) fn validate_address(
    //     &self,
    //     api: &dyn Api,
    //     querier: &QuerierWrapper,
    //     address: String,
    //     app_contract: Addr,
    // ) -> Result<(), ContractError> {
    //     // If the address passes this check then it doesn't refer to a app component by
    //     // name.
    //     if api.addr_validate(&address).is_err() {
    //         ensure!(
    //             self.component_exists(querier, address.clone(), app_contract)?,
    //             ContractError::InvalidComponent { name: address }
    //         );
    //     }
    //     Ok(())
    // }

    fn component_exists(
        &self,
        querier: &QuerierWrapper,
        name: String,
        app_contract: Addr,
    ) -> Result<bool, ContractError> {
        Ok(querier.query_wasm_smart(app_contract, &AppQueryMsg::ComponentExists { name })?)
    }
}
