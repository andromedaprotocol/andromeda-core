use crate::ADOContract;
use common::{
    ado_base::{AndromedaMsg, ExecuteMsg, InstantiateMsg},
    error::ContractError,
    mission::AndrAddress,
    parse_message, require,
};
use cosmwasm_std::{attr, Api, DepsMut, Env, MessageInfo, Order, Response, Storage};
use serde::de::DeserializeOwned;

type ExecuteFunction<E> = fn(DepsMut, Env, MessageInfo, E) -> Result<Response, ContractError>;

impl<'a> ADOContract<'a> {
    pub fn instantiate(
        &self,
        storage: &mut dyn Storage,
        #[cfg(feature = "primitive")] api: &dyn Api,
        #[cfg(not(feature = "primitive"))] _api: &dyn Api,
        info: MessageInfo,
        msg: InstantiateMsg,
    ) -> Result<Response, ContractError> {
        self.owner.save(storage, &info.sender)?;
        self.ado_type.save(storage, &msg.ado_type)?;
        if let Some(operators) = msg.operators {
            self.initialize_operators(storage, operators)?;
        }
        let attributes = [attr("method", "instantiate"), attr("type", &msg.ado_type)];
        #[cfg(feature = "primitive")]
        if let Some(primitive_contract) = msg.primitive_contract {
            self.primitive_contract
                .save(storage, &api.addr_validate(&primitive_contract)?)?;
        }
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
                require(
                    !self.is_nested::<ExecuteMsg>(&data),
                    ContractError::NestedAndromedaMsg {},
                )?;
                let received: E = parse_message(&data)?;
                (execute_function)(deps, env, info, received)
            }
            AndromedaMsg::UpdateOwner { address } => self.execute_update_owner(deps, info, address),
            AndromedaMsg::UpdateOperators { operators } => {
                self.execute_update_operators(deps, info, operators)
            }
            AndromedaMsg::UpdateMissionContract { address } => {
                self.execute_update_mission_contract(deps, info, address, None)
            }
            #[cfg(feature = "withdraw")]
            AndromedaMsg::Withdraw {
                recipient,
                tokens_to_withdraw,
            } => self.execute_withdraw(deps, env, info, recipient, tokens_to_withdraw),
            #[cfg(feature = "modules")]
            AndromedaMsg::RegisterModule { module } => {
                self.execute_register_module(deps.storage, info.sender.as_str(), module, true)
            }
            #[cfg(feature = "modules")]
            AndromedaMsg::DeregisterModule { module_idx } => {
                self.execute_deregister_module(deps, info, module_idx)
            }
            #[cfg(feature = "modules")]
            AndromedaMsg::AlterModule { module_idx, module } => {
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
}

impl<'a> ADOContract<'a> {
    /// Updates the current contract owner. **Only executable by the current contract owner.**
    pub fn execute_update_owner(
        &self,
        deps: DepsMut,
        info: MessageInfo,
        new_owner: String,
    ) -> Result<Response, ContractError> {
        require(
            self.is_contract_owner(deps.storage, info.sender.as_str())?,
            ContractError::Unauthorized {},
        )?;
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
        require(
            self.is_contract_owner(deps.storage, info.sender.as_str())?,
            ContractError::Unauthorized {},
        )?;

        let keys: Vec<Vec<u8>> = self
            .operators
            .keys(deps.storage, None, None, Order::Ascending)
            .collect();
        for key in keys.iter() {
            self.operators
                .remove(deps.storage, &String::from_utf8(key.clone())?);
        }

        for op in operators.iter() {
            self.operators.save(deps.storage, op, &true)?;
        }

        Ok(Response::new().add_attributes(vec![attr("action", "update_operators")]))
    }

    pub fn execute_update_mission_contract(
        &self,
        deps: DepsMut,
        info: MessageInfo,
        address: String,
        addresses: Option<Vec<AndrAddress>>,
    ) -> Result<Response, ContractError> {
        require(
            self.is_contract_owner(deps.storage, info.sender.as_str())?,
            ContractError::Unauthorized {},
        )?;
        self.mission_contract
            .save(deps.storage, &deps.api.addr_validate(&address)?)?;
        self.validate_andr_addresses(deps.as_ref(), addresses.unwrap_or_default())?;
        Ok(Response::new()
            .add_attribute("action", "update_mission_contract")
            .add_attribute("address", address))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};

    fn dummy_function(
        _deps: DepsMut,
        _env: Env,
        _info: MessageInfo,
        _msg: AndromedaMsg,
    ) -> Result<Response, ContractError> {
        Ok(Response::new())
    }

    #[test]
    fn test_update_mission_contract() {
        let contract = ADOContract::default();
        let mut deps = mock_dependencies(&[]);

        let info = mock_info("owner", &[]);
        let deps_mut = deps.as_mut();
        contract
            .instantiate(
                deps_mut.storage,
                deps_mut.api,
                info.clone(),
                InstantiateMsg {
                    ado_type: "type".to_string(),
                    modules: None,
                    primitive_contract: None,
                    operators: None,
                },
            )
            .unwrap();

        let address = String::from("address");

        let msg = AndromedaMsg::UpdateMissionContract {
            address: address.clone(),
        };

        let res = contract
            .execute(deps_mut, mock_env(), info, msg, dummy_function)
            .unwrap();

        assert_eq!(
            Response::new()
                .add_attribute("action", "update_mission_contract")
                .add_attribute("address", address),
            res
        );
    }
}
