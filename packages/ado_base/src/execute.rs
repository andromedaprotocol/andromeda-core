use crate::state::ADOContract;
use common::{
    ado_base::{AndromedaMsg, InstantiateMsg},
    error::ContractError,
    parse_message, require,
};
use cosmwasm_std::{attr, DepsMut, Env, MessageInfo, Order, Response};
use serde::de::DeserializeOwned;

type ExecuteFunction<E> = fn(DepsMut, Env, MessageInfo, E) -> Result<Response, ContractError>;

impl<'a> ADOContract<'a> {
    pub fn instantiate(
        &self,
        deps: DepsMut,
        info: MessageInfo,
        msg: InstantiateMsg,
    ) -> Result<Response, ContractError> {
        self.owner.save(deps.storage, &info.sender)?;
        self.ado_type.save(deps.storage, &msg.ado_type)?;
        if let Some(operators) = msg.operators {
            self.initialize_operators(deps.storage, operators)?;
        }
        Ok(Response::new()
            .add_attribute("method", "instantiate")
            .add_attribute("type", msg.ado_type))
    }

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
                require(!self.is_nested(&data), ContractError::NestedAndromedaMsg {})?;
                let received: E = parse_message(&data)?;
                (execute_function)(deps, env, info, received)
            }
            AndromedaMsg::UpdateOwner { address } => self.execute_update_owner(deps, info, address),
            AndromedaMsg::UpdateOperators { operators } => {
                self.execute_update_operators(deps, info, operators)
            }
            AndromedaMsg::Withdraw {
                recipient,
                tokens_to_withdraw,
            } => {
                #[cfg(feature = "withdraw")]
                return self.execute_withdraw(deps, env, info, recipient, tokens_to_withdraw);

                #[cfg(not(feature = "withdraw"))]
                return Err(ContractError::UnsupportedOperation {});
            }
            AndromedaMsg::RegisterModule { module } => {
                #[cfg(feature = "modules")]
                return self.execute_register_module(
                    &deps.querier,
                    deps.storage,
                    deps.api,
                    info.sender.as_str(),
                    &module,
                    true,
                );
                #[cfg(not(feature = "modules"))]
                return Err(ContractError::UnsupportedOperation {});
            }

            AndromedaMsg::DeregisterModule { module_idx } => {
                #[cfg(feature = "modules")]
                return self.execute_deregister_module(deps, info, module_idx);

                #[cfg(not(feature = "modules"))]
                return Err(ContractError::UnsupportedOperation {});
            }
            AndromedaMsg::AlterModule { module_idx, module } => {
                #[cfg(feature = "modules")]
                return self.execute_alter_module(deps, info, module_idx, &module);

                #[cfg(not(feature = "modules"))]
                return Err(ContractError::UnsupportedOperation {});
            }
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
}
