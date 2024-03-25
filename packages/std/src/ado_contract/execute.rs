use crate::ado_contract::ADOContract;
use crate::amp::addresses::AndrAddr;
use crate::amp::messages::AMPPkt;
use crate::common::context::ExecuteContext;
use crate::common::reply::ReplyId;
use crate::os::{aos_querier::AOSQuerier, economics::ExecuteMsg as EconomicsExecuteMsg};
use crate::{
    ado_base::{AndromedaMsg, InstantiateMsg},
    error::ContractError,
};
use cosmwasm_std::{
    attr, ensure, from_json, to_json_binary, Addr, Api, ContractInfoResponse, CosmosMsg, Deps,
    DepsMut, Env, MessageInfo, QuerierWrapper, Response, Storage, SubMsg, WasmMsg,
};
use serde::de::DeserializeOwned;
use serde::Serialize;

type ExecuteContextFunction<M, E = ContractError> = fn(ExecuteContext, M) -> Result<Response, E>;

impl<'a> ADOContract<'a> {
    pub fn instantiate(
        &self,
        storage: &mut dyn Storage,
        env: Env,
        api: &dyn Api,
        querier: &QuerierWrapper,
        info: MessageInfo,
        msg: InstantiateMsg,
    ) -> Result<Response, ContractError> {
        let ado_type = if msg.ado_type.starts_with("crates.io:andromeda-") {
            msg.ado_type.strip_prefix("crates.io:andromeda-").unwrap()
        } else if msg.ado_type.starts_with("crates.io:") {
            msg.ado_type.strip_prefix("crates.io:").unwrap()
        } else {
            &msg.ado_type
        };
        cw2::set_contract_version(storage, ado_type, msg.ado_version)?;
        self.owner.save(
            storage,
            &api.addr_validate(&msg.owner.unwrap_or(info.sender.to_string()))?,
        )?;
        self.original_publisher.save(storage, &info.sender)?;
        self.block_height.save(storage, &env.block.height)?;
        self.ado_type.save(storage, &msg.ado_type)?;
        self.kernel_address
            .save(storage, &api.addr_validate(&msg.kernel_address)?)?;
        let attributes = [attr("method", "instantiate"), attr("type", ado_type)];

        // We do not want to store app contracts for the kernel, exit early if current contract is kernel
        let is_kernel_contract = ado_type.contains("kernel");
        if is_kernel_contract {
            return Ok(Response::new().add_attributes(attributes));
        }

        // Check if the sender is an app contract to allow for automatic storage of app contrcat reference
        let maybe_contract_info = querier.query_wasm_contract_info(info.sender.clone());
        let is_sender_contract = maybe_contract_info.is_ok();
        if is_sender_contract {
            let ContractInfoResponse { code_id, .. } = maybe_contract_info?;
            let sender_ado_type = AOSQuerier::ado_type_getter(
                querier,
                &self.get_adodb_address(storage, querier)?,
                code_id,
            )?;
            let is_sender_app = Some("app-contract".to_string()) == sender_ado_type;
            // Automatically save app contract reference if creator is an app contract
            if is_sender_app {
                self.app_contract
                    .save(storage, &Addr::unchecked(info.sender.to_string()))?;
                let app_owner = AOSQuerier::ado_owner_getter(querier, &info.sender)?;
                self.owner.save(storage, &app_owner)?;
            }
        }
        Ok(Response::new().add_attributes(attributes))
    }

    /// Handles execution of ADO specific messages.
    pub fn execute(
        &self,
        ctx: ExecuteContext,
        msg: impl Serialize,
    ) -> Result<Response, ContractError> {
        let msg = to_json_binary(&msg)?;
        match from_json::<AndromedaMsg>(&msg) {
            Ok(msg) => match msg {
                AndromedaMsg::Ownership(msg) => {
                    self.execute_ownership(ctx.deps, ctx.env, ctx.info, msg)
                }
                AndromedaMsg::UpdateAppContract { address } => {
                    self.execute_update_app_contract(ctx.deps, ctx.info, address, None)
                }
                AndromedaMsg::UpdateKernelAddress { address } => {
                    self.update_kernel_address(ctx.deps, ctx.info, address)
                }
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
                AndromedaMsg::Permissioning(msg) => self.execute_permissioning(ctx, msg),
                AndromedaMsg::AMPReceive(_) => panic!("AMP Receive should be handled separately"),
            },
            _ => Err(ContractError::NotImplemented { msg: None }),
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
                    ensure!(address.is_addr(deps.api), ContractError::InvalidAddress {});
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
        address.validate(deps.api)?;
        if !address.is_addr(deps.api) {
            address.get_raw_address_from_vfs(deps, vfs_address)?;
        }
        Ok(())
    }

    #[inline]
    /// Gets the stored address for the Kernel contract
    pub fn get_kernel_address(&self, storage: &dyn Storage) -> Result<Addr, ContractError> {
        let kernel_address = self.kernel_address.load(storage)?;
        Ok(kernel_address)
    }

    #[inline]
    /// Gets the current address for the VFS contract.
    pub fn get_vfs_address(
        &self,
        storage: &dyn Storage,
        querier: &QuerierWrapper,
    ) -> Result<Addr, ContractError> {
        let kernel_address = self.get_kernel_address(storage)?;
        AOSQuerier::vfs_address_getter(querier, &kernel_address)
    }

    #[inline]
    /// Gets the current address for the VFS contract.
    pub fn get_adodb_address(
        &self,
        storage: &dyn Storage,
        querier: &QuerierWrapper,
    ) -> Result<Addr, ContractError> {
        let kernel_address = self.get_kernel_address(storage)?;
        AOSQuerier::adodb_address_getter(querier, &kernel_address)
    }

    /// Handles receiving and verifies an AMPPkt from the Kernel before executing the appropriate messages.
    ///
    /// Calls the provided handler with the AMP packet attached within the context.
    pub fn execute_amp_receive<M: DeserializeOwned>(
        &self,
        ctx: ExecuteContext,
        mut packet: AMPPkt,
        handler: ExecuteContextFunction<M>,
    ) -> Result<Response, ContractError> {
        packet.verify_origin(&ctx.info, &ctx.deps.as_ref())?;
        let ctx = ctx.with_ctx(packet.clone());
        ensure!(
            packet.messages.len() == 1,
            ContractError::InvalidPacket {
                error: Some("Invalid packet length".to_string())
            }
        );
        let msg = packet.messages.pop().unwrap();
        let msg: M = from_json(msg.message)?;
        let response = handler(ctx, msg)?;
        Ok(response)
    }

    /// Generates a message to pay a fee for a given action by the given payee
    ///
    /// Fees are paid in the following fallthrough priority:
    /// 1. ADO Contract
    /// 2. App Contract for sending ADO
    /// 3. Provided Payee
    ///
    /// If any of the above cannot pay the fee the remainder is paid by the next in the list until no remainder remains.
    /// If there is still a remainder after all 3 payments then the fee cannot be paid and the message will error.
    pub fn pay_fee(
        &self,
        storage: &dyn Storage,
        querier: &QuerierWrapper,
        action: String,
        payee: Addr,
    ) -> Result<SubMsg, ContractError> {
        let kernel_address = self.get_kernel_address(storage)?;
        let economics_contract_address =
            AOSQuerier::kernel_address_getter(querier, &kernel_address, "economics")?;
        let economics_msg = EconomicsExecuteMsg::PayFee { action, payee };
        let msg = SubMsg::reply_on_error(
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: economics_contract_address.to_string(),
                msg: to_json_binary(&economics_msg)?,
                funds: vec![],
            }),
            ReplyId::PayFee.repr(),
        );

        Ok(msg)
    }

    /// Updates the current kernel address used by the ADO
    /// Requires the sender to be the owner of the ADO
    pub fn update_kernel_address(
        &self,
        deps: DepsMut,
        info: MessageInfo,
        address: Addr,
    ) -> Result<Response, ContractError> {
        ensure!(
            self.is_contract_owner(deps.storage, info.sender.as_str())?,
            ContractError::Unauthorized {}
        );
        self.kernel_address.save(deps.storage, &address)?;
        Ok(Response::new()
            .add_attribute("action", "update_kernel_address")
            .add_attribute("address", address))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(feature = "modules")]
    use crate::ado_base::modules::Module;
    use crate::testing::mock_querier::MOCK_KERNEL_CONTRACT;
    #[cfg(feature = "modules")]
    use crate::testing::mock_querier::{mock_dependencies_custom, MOCK_APP_CONTRACT};
    #[cfg(feature = "modules")]
    use cosmwasm_std::Uint64;
    use cosmwasm_std::{
        testing::{mock_dependencies, mock_env, mock_info},
        Addr,
    };

    #[test]
    #[cfg(feature = "modules")]
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
                &deps_mut.querier,
                info.clone(),
                InstantiateMsg {
                    ado_type: "type".to_string(),

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
    #[cfg(feature = "modules")]
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
                &deps_mut.querier,
                info.clone(),
                InstantiateMsg {
                    ado_type: "type".to_string(),
                    ado_version: "version".to_string(),

                    kernel_address: MOCK_KERNEL_CONTRACT.to_string(),
                    owner: None,
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
                &deps_mut.querier,
                info.clone(),
                InstantiateMsg {
                    ado_type: "type".to_string(),
                    ado_version: "version".to_string(),

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
                &deps_mut.querier,
                info.clone(),
                InstantiateMsg {
                    ado_type: "type".to_string(),
                    ado_version: "version".to_string(),
                    owner: None,

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

    #[test]
    fn test_update_kernel_address() {
        let contract = ADOContract::default();
        let mut deps = mock_dependencies();

        let info = mock_info("owner", &[]);
        let deps_mut = deps.as_mut();
        contract
            .instantiate(
                deps_mut.storage,
                mock_env(),
                deps_mut.api,
                &deps_mut.querier,
                info.clone(),
                InstantiateMsg {
                    ado_type: "type".to_string(),
                    ado_version: "version".to_string(),
                    owner: None,

                    kernel_address: MOCK_KERNEL_CONTRACT.to_string(),
                },
            )
            .unwrap();

        let address = String::from("address");

        let msg = AndromedaMsg::UpdateKernelAddress {
            address: Addr::unchecked(address.clone()),
        };

        let res = contract
            .execute(ExecuteContext::new(deps.as_mut(), info, mock_env()), msg)
            .unwrap();

        let msg = AndromedaMsg::UpdateKernelAddress {
            address: Addr::unchecked(address.clone()),
        };

        assert_eq!(
            Response::new()
                .add_attribute("action", "update_kernel_address")
                .add_attribute("address", address),
            res
        );

        let res = contract.execute(
            ExecuteContext::new(deps.as_mut(), mock_info("not_owner", &[]), mock_env()),
            msg,
        );
        assert!(res.is_err())
    }
}
