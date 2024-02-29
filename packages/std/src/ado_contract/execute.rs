use crate::ado_base::rates::RatesMessage;
use crate::ado_contract::ADOContract;
use crate::amp::addresses::AndrAddr;
use crate::amp::messages::AMPPkt;
use crate::common::context::ExecuteContext;
use crate::os::{aos_querier::AOSQuerier, economics::ExecuteMsg as EconomicsExecuteMsg};
use crate::{
    ado_base::{AndromedaMsg, InstantiateMsg},
    error::ContractError,
};
use cosmwasm_std::{
    attr, from_json, to_json_binary, Addr, Api, CosmosMsg, Deps, DepsMut, Env, MessageInfo,
    QuerierWrapper, Response, Storage, SubMsg, WasmMsg,
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
            &api.addr_validate(&msg.owner.unwrap_or(info.sender.to_string()))?,
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
        let msg = to_json_binary(&msg)?;
        match from_json::<AndromedaMsg>(&msg) {
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

                AndromedaMsg::SetPermission {
                    actor,
                    action,
                    permission,
                } => self.execute_set_permission(ctx, actor, action, permission),
                AndromedaMsg::RemovePermission { action, actor } => {
                    self.execute_remove_permission(ctx, actor, action)
                }
                AndromedaMsg::PermissionAction { action } => {
                    self.execute_permission_action(ctx, action)
                }
                #[cfg(feature = "rates")]
                AndromedaMsg::Rates(rates_message) => match rates_message {
                    RatesMessage::SetRate { action, rate } => {
                        self.execute_set_rates(ctx, action, rate)
                    }
                    RatesMessage::RemoveRate { action } => self.execute_remove_rates(ctx, action),
                },

                AndromedaMsg::AMPReceive(_) => panic!("AMP Receive should be handled separately"),
                AndromedaMsg::Deposit { .. } => Err(ContractError::NotImplemented { msg: None }),
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

    #[inline]
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
        if let Some(msg_opt) = msg_opt {
            let msg: E = from_json(msg_opt.message)?;
            let response = handler(ctx, msg)?;
            Ok(response)
        } else {
            Err(ContractError::InvalidPacket {
                error: Some("AMP Packet received with no messages".to_string()),
            })
        }
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
            9999,
        );

        Ok(msg)
    }
}

#[cfg(test)]

mod tests {
    use super::*;
    use crate::testing::mock_querier::MOCK_KERNEL_CONTRACT;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};

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
}
