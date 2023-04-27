use cosmwasm_schema::cw_serde;
use cosmwasm_std::{ensure, Addr, Deps, QuerierWrapper, Storage};

use crate::ado_contract::ADOContract;
use crate::amp::addresses::AndrAddr;
use crate::amp::VFS_KEY;
use crate::error::ContractError;
use crate::os::kernel::QueryMsg as KernelQueryMsg;

#[cw_serde]
enum AppQueryMsg {
    ComponentExists { name: String },
    GetAddress { name: String },
}

impl<'a> ADOContract<'a> {
    pub fn get_app_contract(&self, storage: &dyn Storage) -> Result<Option<Addr>, ContractError> {
        Ok(self.app_contract.may_load(storage)?)
    }

    pub(crate) fn validate_andr_addresses(
        &self,
        deps: &Deps,
        addresses: Vec<AndrAddr>,
    ) -> Result<(), ContractError> {
        let app_contract = self.get_app_contract(deps.storage)?;
        let vfs_address = self.get_vfs_address(deps.storage, &deps.querier);
        match vfs_address {
            Ok(vfs_address) => {
                ensure!(
                    app_contract.is_some(),
                    ContractError::AppContractNotSpecified {}
                );
                #[cfg(feature = "modules")]
                {
                    let mut addresses = addresses.clone();
                    let modules = self.load_modules(deps.storage)?;
                    if !modules.is_empty() {
                        let andr_addresses: Vec<AndrAddr> =
                            modules.into_iter().map(|m| m.address).collect();
                        addresses.extend(andr_addresses);
                    }
                }
                for address in addresses {
                    self.validate_andr_address(&deps, address, vfs_address.clone())?;
                }
                Ok(())
            }
            Err(_) => {
                for address in addresses {
                    address.is_addr(deps.api);
                }
                Ok(())
            }
        }
    }

    pub fn validate_andr_address(
        &self,
        deps: &Deps,
        address: AndrAddr,
        vfs_address: Addr,
    ) -> Result<(), ContractError> {
        // Validate address string is valid
        address.validate(deps.api)?;
        if address.is_vfs_path() {
            address.get_raw_address_from_vfs(deps, vfs_address)?;
        }
        Ok(())
    }

    pub fn get_vfs_address(
        &self,
        storage: &dyn Storage,
        querier: &QuerierWrapper,
    ) -> Result<Addr, ContractError> {
        let query = KernelQueryMsg::KeyAddress {
            key: VFS_KEY.to_string(),
        };
        let kernel_address = self.get_kernel_address(storage)?;
        Ok(querier.query_wasm_smart(kernel_address, &query)?)
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
}
