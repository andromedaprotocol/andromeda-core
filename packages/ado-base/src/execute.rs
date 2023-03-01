use crate::ADOContract;
#[cfg(feature = "modules")]
use common::ado_base::modules::Module;
use common::{
    ado_base::{AndromedaMsg, ExecuteMsg, InstantiateMsg},
    error::ContractError,
    parse_message,
};
#[cfg(feature = "modules")]
use cosmwasm_std::QuerierWrapper;
use cosmwasm_std::{attr, ensure, Api, DepsMut, Env, MessageInfo, Order, Response, Storage};
use serde::de::DeserializeOwned;

type ExecuteFunction<E> = fn(DepsMut, Env, MessageInfo, E) -> Result<Response, ContractError>;

impl<'a> ADOContract<'a> {
    pub fn instantiate(
        &self,
        storage: &mut dyn Storage,
        env: Env,
        #[cfg(feature = "primitive")] api: &dyn Api,
        #[cfg(not(feature = "primitive"))] api: &dyn Api,
        info: MessageInfo,
        msg: InstantiateMsg,
    ) -> Result<Response, ContractError> {
        self.owner.save(storage, &info.sender)?;
        self.original_publisher.save(storage, &info.sender)?;
        self.block_height.save(storage, &env.block.height)?;
        self.ado_type.save(storage, &msg.ado_type)?;
        self.version.save(storage, &msg.ado_version)?;
        if let Some(kernel_address) = msg.kernel_address {
            self.kernel_address
                .save(storage, &api.addr_validate(&kernel_address)?)?;
        }
        let attributes = [attr("method", "instantiate"), attr("type", &msg.ado_type)];
        #[cfg(feature = "modules")]
        if let Some(modules) = msg.modules {
            return Ok(self
                .register_modules(info.sender.as_str(), storage, modules)?
                .add_attributes(attributes));
        }
        Ok(Response::new().add_attributes(attributes))
    }

    #[allow(unreachable_patterns)]
    pub fn execute<E: DeserializeOwned>(
        &self,
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        msg: AndromedaMsg,
        execute_function: ExecuteFunction<E>,
    ) -> Result<Response, ContractError> {
        match msg {
            AndromedaMsg::Receive(data) => {
                ensure!(
                    !self.is_nested::<ExecuteMsg>(&data),
                    ContractError::NestedAndromedaMsg {}
                );
                let received: E = parse_message(&data)?;
                (execute_function)(deps, env, info, received)
            }
            AndromedaMsg::UpdateOwner { address } => self.execute_update_owner(deps, info, address),
            AndromedaMsg::UpdateOperators { operators } => {
                self.execute_update_operators(deps, info, operators)
            }
            AndromedaMsg::UpdateAppContract { address } => {
                self.execute_update_app_contract(deps, info, address, None)
            }
            #[cfg(feature = "withdraw")]
            AndromedaMsg::Withdraw {
                recipient,
                tokens_to_withdraw,
            } => self.execute_withdraw(deps, env, info, recipient, tokens_to_withdraw),
            #[cfg(feature = "modules")]
            AndromedaMsg::RegisterModule { module } => {
                self.validate_module_address(deps.storage, deps.api, &deps.querier, &module)?;
                self.execute_register_module(deps.storage, info.sender.as_str(), module, true)
            }
            #[cfg(feature = "modules")]
            AndromedaMsg::DeregisterModule { module_idx } => {
                self.execute_deregister_module(deps, info, module_idx)
            }
            #[cfg(feature = "modules")]
            AndromedaMsg::AlterModule { module_idx, module } => {
                self.validate_module_address(deps.storage, deps.api, &deps.querier, &module)?;
                self.execute_alter_module(deps, info, module_idx, module)
            }
            #[cfg(feature = "primitive")]
            AndromedaMsg::RefreshAddress { contract } => {
                self.execute_refresh_address(deps, contract)
            }
            #[cfg(feature = "primitive")]
            AndromedaMsg::RefreshAddresses { start_after, limit } => {
                self.execute_refresh_addresses(deps, start_after, limit)
            }
            _ => Err(ContractError::UnsupportedOperation {}),
        }
    }

    #[cfg(feature = "modules")]
    fn validate_module_address(
        &self,
        storage: &dyn Storage,
        api: &dyn Api,
        querier: &QuerierWrapper,
        module: &Module,
    ) -> Result<(), ContractError> {
        if let Some(app_contract) = self.get_app_contract(storage)? {
            // api.addr_validate(&module.address)?;
            self.validate_andr_address(api, querier, module.address.to_owned(), app_contract)?;
        }
        Ok(())
    }
}

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

    pub fn execute_update_app_contract(
        &self,
        deps: DepsMut,
        info: MessageInfo,
        address: String,
        addresses: Option<Vec<String>>,
    ) -> Result<Response, ContractError> {
        ensure!(
            self.is_contract_owner(deps.storage, info.sender.as_str())?,
            ContractError::Unauthorized {}
        );
        self.app_contract
            .save(deps.storage, &deps.api.addr_validate(&address)?)?;
        self.validate_andr_addresses(deps.as_ref(), addresses.unwrap_or_default())?;
        Ok(Response::new()
            .add_attribute("action", "update_app_contract")
            .add_attribute("address", address))
    }

    pub fn execute_update_version(&self, deps: DepsMut) -> Result<Response, ContractError> {
        self.version
            .save(deps.storage, &env!("CARGO_PKG_VERSION").to_string())?;
        Ok(Response::new()
            .add_attribute("action", "update_version")
            .add_attribute("version", env!("CARGO_PKG_VERSION").to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mock_querier::{mock_dependencies_custom, MOCK_APP_CONTRACT};
    use common::ado_base::modules::Module;
    use cosmwasm_std::{
        testing::{mock_dependencies, mock_env, mock_info},
        Addr, Uint64,
    };

    fn dummy_function(
        _deps: DepsMut,
        _env: Env,
        _info: MessageInfo,
        _msg: AndromedaMsg,
    ) -> Result<Response, ContractError> {
        Ok(Response::new())
    }

    #[test]
    fn test_register_module_invalid_identifier() {
        let contract = ADOContract::default();
        let mut deps = mock_dependencies_custom(&[]);

        let info = mock_info("owner", &[]);
        let deps_mut = deps.as_mut();
        contract
            .instantiate(
                deps_mut.storage,
                mock_env(),
                deps_mut.api,
                info.clone(),
                InstantiateMsg {
                    ado_type: "type".to_string(),
                    modules: None,
                    operators: None,
                    ado_version: "version".to_string(),
                    kernel_address: None,
                },
            )
            .unwrap();

        contract
            .app_contract
            .save(deps_mut.storage, &Addr::unchecked(MOCK_APP_CONTRACT))
            .unwrap();

        let module = Module {
            module_name: Some("module".to_owned()),
            address: "z".to_string(),

            is_mutable: false,
        };

        let msg = AndromedaMsg::RegisterModule { module };

        let res = contract.execute(deps_mut, mock_env(), info, msg, dummy_function);

        assert_eq!(
            ContractError::InvalidComponent {
                name: "z".to_string()
            },
            res.unwrap_err()
        );
    }

    #[test]
    fn test_alter_module_invalid_identifier() {
        let contract = ADOContract::default();
        let mut deps = mock_dependencies_custom(&[]);

        let info = mock_info("owner", &[]);
        let deps_mut = deps.as_mut();
        contract
            .instantiate(
                deps_mut.storage,
                mock_env(),
                deps_mut.api,
                info.clone(),
                InstantiateMsg {
                    ado_type: "type".to_string(),
                    ado_version: "version".to_string(),
                    modules: Some(vec![Module {
                        module_name: Some("module".to_owned()),
                        address: "terra1...".to_string(),

                        is_mutable: true,
                    }]),
                    operators: None,
                    kernel_address: None,
                },
            )
            .unwrap();

        contract
            .app_contract
            .save(deps_mut.storage, &Addr::unchecked(MOCK_APP_CONTRACT))
            .unwrap();

        let module = Module {
            module_name: Some("module".to_owned()),
            address: "z".to_string(),

            is_mutable: false,
        };

        let msg = AndromedaMsg::AlterModule {
            module_idx: Uint64::new(1),
            module,
        };

        let res = contract.execute(deps_mut, mock_env(), info, msg, dummy_function);

        assert_eq!(
            ContractError::InvalidComponent {
                name: "z".to_string()
            },
            res.unwrap_err()
        );
    }

    #[test]
    fn test_update_app_contract() {
        let contract = ADOContract::default();
        let mut deps = mock_dependencies();

        let info = mock_info("owner", &[]);
        let deps_mut = deps.as_mut();
        contract
            .instantiate(
                deps_mut.storage,
                mock_env(),
                deps_mut.api,
                info.clone(),
                InstantiateMsg {
                    ado_type: "type".to_string(),
                    ado_version: "version".to_string(),
                    modules: None,
                    operators: None,
                    kernel_address: None,
                },
            )
            .unwrap();

        let address = String::from("address");

        let msg = AndromedaMsg::UpdateAppContract {
            address: address.clone(),
        };

        let res = contract
            .execute(deps_mut, mock_env(), info, msg, dummy_function)
            .unwrap();

        assert_eq!(
            Response::new()
                .add_attribute("action", "update_app_contract")
                .add_attribute("address", address),
            res
        );
    }

    #[test]
    fn test_update_app_contract_invalid_module() {
        let contract = ADOContract::default();
        let mut deps = mock_dependencies_custom(&[]);

        let info = mock_info("owner", &[]);
        let deps_mut = deps.as_mut();
        contract
            .instantiate(
                deps_mut.storage,
                mock_env(),
                deps_mut.api,
                info.clone(),
                InstantiateMsg {
                    ado_type: "type".to_string(),
                    ado_version: "version".to_string(),
                    modules: Some(vec![Module {
                        module_name: Some("address_list".to_string()),
                        is_mutable: true,
                        address: "z".to_string(),
                    }]),
                    operators: None,
                    kernel_address: None,
                },
            )
            .unwrap();

        let msg = AndromedaMsg::UpdateAppContract {
            address: MOCK_APP_CONTRACT.to_owned(),
        };

        let res = contract.execute(deps_mut, mock_env(), info, msg, dummy_function);

        assert_eq!(
            ContractError::InvalidComponent {
                name: "z".to_string()
            },
            res.unwrap_err()
        );
    }
}
