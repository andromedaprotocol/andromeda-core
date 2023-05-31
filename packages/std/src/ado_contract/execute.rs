use crate::ado_contract::ADOContract;
use crate::amp::addresses::AndrAddr;
use crate::amp::messages::AMPPkt;
use crate::common::context::ExecuteContext;
use crate::os::aos_querier::AOSQuerier;
use crate::{
    ado_base::{AndromedaMsg, InstantiateMsg},
    error::ContractError,
};
use cosmwasm_std::{
    attr, from_binary, to_binary, Addr, Api, Deps, DepsMut, Env, MessageInfo, QuerierWrapper,
    Response, Storage,
};
use serde::de::DeserializeOwned;
use serde::Serialize;

type ExecuteContextFunction<E> = fn(ExecuteContext, E) -> Result<Response, ContractError>;

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

    /// Handles execution of ADO specific messages.
    pub fn execute(
        &self,
        ctx: ExecuteContext,
        msg: impl Serialize,
    ) -> Result<Response, ContractError> {
        let msg = to_binary(&msg)?;
        match from_binary::<AndromedaMsg>(&msg) {
            Ok(msg) => match msg {
                AndromedaMsg::UpdateOwner { address } => {
                    self.execute_update_owner(ctx.deps, ctx.info, address)
                }
                AndromedaMsg::UpdateOperators { operators } => {
                    self.execute_update_operators(ctx.deps, ctx.info, operators)
                }
                AndromedaMsg::UpdateAppContract { address } => {
                    self.execute_update_app_contract(ctx.deps, ctx.info, address, None)
                }
                #[cfg(feature = "withdraw")]
                AndromedaMsg::Withdraw {
                    recipient,
                    tokens_to_withdraw,
                } => self.execute_withdraw(ctx, recipient, tokens_to_withdraw),
                #[cfg(feature = "modules")]
                AndromedaMsg::RegisterModule { module } => {
                    self.validate_module_address(&ctx.deps.as_ref(), &module)?;
                    self.execute_register_module(
                        ctx.deps.storage,
                        ctx.info.sender.as_str(),
                        module,
                        true,
                    )
                }
                #[cfg(feature = "modules")]
                AndromedaMsg::DeregisterModule { module_idx } => {
                    self.execute_deregister_module(ctx.deps, ctx.info, module_idx)
                }
                #[cfg(feature = "modules")]
                AndromedaMsg::AlterModule { module_idx, module } => {
                    self.validate_module_address(&ctx.deps.as_ref(), &module)?;
                    self.execute_alter_module(ctx.deps, ctx.info, module_idx, module)
                }
                AndromedaMsg::SetPermission {
                    actor,
                    action,
                    permission,
                } => self.execute_set_permission(ctx, actor, action, permission),
                AndromedaMsg::RemovePermission { action, actor } => {
                    self.execute_remove_permission(ctx, actor, action)
                }
                AndromedaMsg::AMPReceive(_) => panic!("AMP Receive should be handled separately"),
                AndromedaMsg::Deposit { .. } => Err(ContractError::NotImplemented { msg: None }),
            },
            _ => Err(ContractError::NotImplemented { msg: None }),
        }
    }

    /// Handles execution of ADO specific messages with a fallback function in the case that the provided message must be handled by an external package.
    pub fn execute_with_fallback<E: DeserializeOwned>(
        &self,
        ctx: ExecuteContext,
        msg: impl Serialize,
        fallback_execute_function: ExecuteContextFunction<E>,
    ) -> Result<Response, ContractError> {
        let deserialized = from_binary::<E>(&to_binary(&msg)?);

        match deserialized {
            Ok(deserialized) => fallback_execute_function(ctx, deserialized),
            Err(_) => self.execute(ctx, msg),
        }
    }

    /// Validates all provided `AndrAddr` addresses.
    ///
    /// Requires the VFS address to be set if any address is a VFS path.
    /// Automatically validates all stored modules.
    pub fn validate_andr_addresses(
        &self,
        deps: &Deps,
        addresses: Vec<AndrAddr>,
    ) -> Result<(), ContractError> {
        let vfs_address = self.get_vfs_address(deps.storage, &deps.querier);
        match vfs_address {
            Ok(vfs_address) => {
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
                    self.validate_andr_address(deps, address, vfs_address.clone())?;
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

    /// Validates the given `AndrAddr` address.
    pub(crate) fn validate_andr_address(
        &self,
        deps: &Deps,
        address: AndrAddr,
        vfs_address: Addr,
    ) -> Result<(), ContractError> {
        // Validate address string is valid
        address.validate(deps.api)?;
        if !address.is_addr(deps.api) {
            address.get_raw_address_from_vfs(deps, vfs_address)?;
        }
        Ok(())
    }

    /// Gets the stored address for the Kernel contract
    pub fn get_kernel_address(&self, storage: &dyn Storage) -> Result<Addr, ContractError> {
        let kernel_address = self.kernel_address.load(storage)?;
        Ok(kernel_address)
    }

    /// Gets the current address for the VFS contract.
    pub fn get_vfs_address(
        &self,
        storage: &dyn Storage,
        querier: &QuerierWrapper,
    ) -> Result<Addr, ContractError> {
        let kernel_address = self.get_kernel_address(storage)?;
        AOSQuerier::vfs_address_getter(querier, &kernel_address)
    }

    /// Gets the current address for the VFS contract.
    pub fn get_adodb_address(
        &self,
        storage: &dyn Storage,
        querier: &QuerierWrapper,
    ) -> Result<Addr, ContractError> {
        let kernel_address = self.get_kernel_address(storage)?;
        AOSQuerier::adodb_address_getter(querier, &kernel_address)
    }

    /// Updates the current version of the contract.
    pub fn execute_update_version(&self, deps: DepsMut) -> Result<Response, ContractError> {
        self.version
            .save(deps.storage, &env!("CARGO_PKG_VERSION").to_string())?;
        Ok(Response::new()
            .add_attribute("action", "update_version")
            .add_attribute("version", env!("CARGO_PKG_VERSION").to_string()))
    }

    /// Handles receiving and verifies an AMPPkt from the Kernel before executing the appropriate messages.
    ///
    /// Calls the provided handler with the AMP packet attached within the context.
    pub fn execute_amp_receive<E: DeserializeOwned>(
        &self,
        ctx: ExecuteContext,
        mut packet: AMPPkt,
        handler: ExecuteContextFunction<E>,
    ) -> Result<Response, ContractError> {
        packet.verify_origin(&ctx.info, &ctx.deps.as_ref())?;
        let ctx = ctx.with_ctx(packet.clone());
        let msg_opt = packet.messages.pop();
        if msg_opt.is_none() {
            Err(ContractError::InvalidPacket {
                error: Some("AMP Packet received with no messages".to_string()),
            })
        } else {
            let amp_msg = msg_opt.unwrap();
            let msg: E = from_binary(&amp_msg.message)?;
            let response = handler(ctx, msg)?;
            Ok(response)
        }
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

        let res = contract.execute(ExecuteContext::new(deps.as_mut(), info, mock_env()), msg);
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

        let module = Module::new("/m".to_owned(), "z".to_string(), false);

        let msg = AndromedaMsg::AlterModule {
            module_idx: Uint64::new(1),
            module,
        };

        let res = contract.execute(ExecuteContext::new(deps.as_mut(), info, mock_env()), msg);
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
            .execute(ExecuteContext::new(deps.as_mut(), info, mock_env()), msg)
            .unwrap();

        assert_eq!(
            Response::new()
                .add_attribute("action", "update_app_contract")
                .add_attribute("address", address),
            res
        );
    }

    #[test]
    #[cfg(feature = "modules")]
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
                    owner: None,
                    operators: None,
                    kernel_address: MOCK_KERNEL_CONTRACT.to_string(),
                },
            )
            .unwrap();
        contract
            .register_modules(
                info.sender.as_str(),
                deps_mut.storage,
                Some(vec![Module::new("module", "cosmos1...".to_string(), false)]),
            )
            .unwrap();
    }
}
