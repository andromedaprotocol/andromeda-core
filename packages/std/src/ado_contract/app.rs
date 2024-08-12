use cosmwasm_schema::cw_serde;
use cosmwasm_std::{ensure, Addr, CustomQuery, DepsMut, MessageInfo, Response, Storage};

use crate::ado_contract::ADOContract;
use crate::amp::addresses::AndrAddr;
use crate::error::ContractError;

#[cw_serde]
enum AppQueryMsg {
    ComponentExists { name: String },
    GetAddress { name: String },
}

impl<'a> ADOContract<'a> {
    #[inline]
    pub fn get_app_contract(&self, storage: &dyn Storage) -> Result<Option<Addr>, ContractError> {
        Ok(self.app_contract.may_load(storage)?)
    }

    pub fn execute_update_app_contract<C: CustomQuery>(
        &self,
        deps: DepsMut<C>,
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
        #[cfg(feature = "modules")]
        {
            let modules = self.load_modules(deps.storage)?;
            for module in modules {
                self.validate_module_address(&deps.as_ref(), &module)?;
            }
        }
        Ok(Response::new()
            .add_attribute("action", "update_app_contract")
            .add_attribute("address", address))
    }
}
