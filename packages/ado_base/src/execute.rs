use crate::state::ADOContract;
use common::{
    ado_base::{AndromedaMsg, InstantiateMsg},
    error::ContractError,
    parse_message, require,
};
use cosmwasm_std::{
    attr, Api, DepsMut, Env, MessageInfo, Order, QuerierWrapper, Response, Storage,
};
use serde::de::DeserializeOwned;

type ExecuteFunction<E> = fn(DepsMut, Env, MessageInfo, E) -> Result<Response, ContractError>;

impl<'a> ADOContract<'a> {
    pub fn instantiate(
        &self,
        storage: &mut dyn Storage,
        api: &dyn Api,
        querier: &QuerierWrapper,
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
                .register_modules(info.sender.as_str(), querier, storage, api, modules)?
                .add_attributes(attributes));
        }
        Ok(Response::new().add_attributes(attributes))
    }

    #[allow(clippy::unreachable)]
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
            #[cfg(feature = "withdraw")]
            AndromedaMsg::Withdraw {
                recipient,
                tokens_to_withdraw,
            } => self.execute_withdraw(deps, env, info, recipient, tokens_to_withdraw),
            #[cfg(feature = "modules")]
            AndromedaMsg::RegisterModule { module } => self.execute_register_module(
                &deps.querier,
                deps.storage,
                deps.api,
                info.sender.as_str(),
                module,
                true,
            ),
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
}
