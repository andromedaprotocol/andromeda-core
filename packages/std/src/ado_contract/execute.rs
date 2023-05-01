#[cfg(feature = "modules")]
use crate::ado_base::modules::Module;
use crate::ado_contract::ADOContract;
use crate::amp::addresses::AndrAddr;
use crate::{
    ado_base::{AndromedaMsg, ExecuteMsg, InstantiateMsg},
    common::parse_message,
    error::ContractError,
};
#[cfg(feature = "modules")]
use cosmwasm_std::Deps;
use cosmwasm_std::{attr, ensure, Api, DepsMut, Env, MessageInfo, Order, Response, Storage};
use serde::de::DeserializeOwned;

type ExecuteFunction<E> = fn(DepsMut, Env, MessageInfo, E) -> Result<Response, ContractError>;

impl<'a> ADOContract<'a> {
    pub fn instantiate(
        &self,
        storage: &mut dyn Storage,
        env: Env,
        api: &dyn Api,
        info: MessageInfo,
        msg: InstantiateMsg,
    ) -> Result<Response, ContractError> {
        self.owner.save(
            storage,
            &api.addr_validate(&msg.owner.unwrap_or_else(|| info.sender.to_string()))?,
        )?;
        self.original_publisher.save(storage, &info.sender)?;
        self.block_height.save(storage, &env.block.height)?;
        self.ado_type.save(storage, &msg.ado_type)?;
        self.version.save(storage, &msg.ado_version)?;
        self.kernel_address
            .save(storage, &api.addr_validate(&msg.kernel_address)?)?;
        let attributes = [attr("method", "instantiate"), attr("type", &msg.ado_type)];
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
                self.validate_module_address(&deps.as_ref(), &module)?;
                self.execute_register_module(deps.storage, info.sender.as_str(), module, true)
            }
            #[cfg(feature = "modules")]
            AndromedaMsg::DeregisterModule { module_idx } => {
                self.execute_deregister_module(deps, info, module_idx)
            }
            #[cfg(feature = "modules")]
            AndromedaMsg::AlterModule { module_idx, module } => {
                self.validate_module_address(&deps.as_ref(), &module)?;
                self.execute_alter_module(deps, info, module_idx, module)
            }
            _ => Err(ContractError::UnsupportedOperation {}),
        }
    }

    #[cfg(feature = "modules")]
    fn validate_module_address(&self, deps: &Deps, module: &Module) -> Result<(), ContractError> {
        self.validate_andr_addresses(deps, vec![module.address.to_owned()])?;
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

    pub fn execute_update_version(&self, deps: DepsMut) -> Result<Response, ContractError> {
        self.version
            .save(deps.storage, &env!("CARGO_PKG_VERSION").to_string())?;
        Ok(Response::new()
            .add_attribute("action", "update_version")
            .add_attribute("version", env!("CARGO_PKG_VERSION").to_string()))
    }
}

#[cfg(test)]
#[cfg(feature = "modules")]
mod tests {
    use super::*;
    use crate::ado_base::modules::Module;
    use crate::testing::mock_querier::{
        mock_dependencies_custom, MOCK_APP_CONTRACT, MOCK_KERNEL_CONTRACT,
    };
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
                    operators: None,
                    ado_version: "version".to_string(),
                    kernel_address: MOCK_KERNEL_CONTRACT.to_string(),
                    owner: None,
                },
            )
            .unwrap();

        contract
            .app_contract
            .save(deps_mut.storage, &Addr::unchecked(MOCK_APP_CONTRACT))
            .unwrap();

        let module = Module::new("module".to_owned(), "z".to_string(), false);

        let msg = AndromedaMsg::RegisterModule { module };

        let res = contract.execute(deps_mut, mock_env(), info, msg, dummy_function);
        assert!(res.is_err())
    }

    #[test]
    fn test_alter_module_invalid_identifier() {
        let contract = ADOContract::default();
        let mut deps = mock_dependencies_custom(&[]);

        let info = mock_info("owner", &[]);
        let deps_mut = deps.as_mut();
        contract
            .register_modules(
                info.sender.as_str(),
                deps_mut.storage,
                Some(vec![Module::new("module", "cosmos1...".to_string(), false)]),
            )
            .unwrap();
        contract
            .instantiate(
                deps_mut.storage,
                mock_env(),
                deps_mut.api,
                info.clone(),
                InstantiateMsg {
                    ado_type: "type".to_string(),
                    ado_version: "version".to_string(),
                    operators: None,
                    kernel_address: MOCK_KERNEL_CONTRACT.to_string(),
                    owner: None,
                },
            )
            .unwrap();

        contract
            .app_contract
            .save(deps_mut.storage, &Addr::unchecked(MOCK_APP_CONTRACT))
            .unwrap();

        let module = Module::new("module".to_owned(), "z".to_string(), false);

        let msg = AndromedaMsg::AlterModule {
            module_idx: Uint64::new(1),
            module,
        };

        let res = contract.execute(deps_mut, mock_env(), info, msg, dummy_function);
        assert!(res.is_err())
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
                    operators: None,
                    kernel_address: MOCK_KERNEL_CONTRACT.to_string(),
                    owner: None,
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

    // // TODO Commented out until we decided how to handle modules
    // #[test]
    // fn test_update_app_contract_invalid_module() {
    //     let contract = ADOContract::default();
    //     let mut deps = mock_dependencies_custom(&[]);

    //     let info = mock_info("owner", &[]);
    //     let deps_mut = deps.as_mut();
    //     contract
    //         .instantiate(
    //             deps_mut.storage,
    //             mock_env(),
    //             deps_mut.api,
    //             info.clone(),
    //             InstantiateMsg {
    //                 ado_type: "type".to_string(),
    //                 ado_version: "version".to_string(),
    //                 modules: Some(vec![Module {
    //                     module_name: Some("address_list".to_string()),
    //                     is_mutable: true,
    //                     address: "z".to_string(),
    //                 }]),
    //                 operators: None,
    //                 kernel_address: None,
    //             },
    //         )
    //         .unwrap();

    //     let msg = AndromedaMsg::UpdateAppContract {
    //         address: MOCK_APP_CONTRACT.to_owned(),
    //     };

    //     let res = contract.execute(deps_mut, mock_env(), info, msg, dummy_function);

    //     assert_eq!(ContractError::InvalidAddress {}, res.unwrap_err());
    // }
}
