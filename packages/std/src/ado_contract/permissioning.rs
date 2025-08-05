use crate::ado_base::permissioning::LocalPermission;
use crate::common::Milliseconds;
use crate::os::aos_querier::AOSQuerier;
use crate::{
    ado_base::permissioning::{Permission, PermissionInfo, PermissioningMessage},
    amp::{messages::AMPPkt, AndrAddr},
    common::{context::ExecuteContext, expiration::Expiry, schedule::Schedule, OrderBy},
    error::ContractError,
};
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{
    ensure, to_json_binary, Addr, CosmosMsg, Deps, DepsMut, Env, MessageInfo, Order, Response,
    Storage, SubMsg, WasmMsg,
};
use cw_storage_plus::{Bound, Index, IndexList, IndexedMap, MultiIndex};

use super::ADOContract;

const MAX_QUERY_LIMIT: u32 = 50;
const DEFAULT_QUERY_LIMIT: u32 = 25;

#[cw_serde]
// Importing this enum from the address list contract would result in a circular dependency
pub enum AddressListExecuteMsg {
    /// Adds an actor key and a permission value
    PermissionActors {
        actors: Vec<AndrAddr>,
        permission: LocalPermission,
    },
}

pub struct PermissionsIndices<'a> {
    /// PK: action + actor
    ///
    /// Secondary key: actor
    pub actor: MultiIndex<'a, String, PermissionInfo, String>,
    pub action: MultiIndex<'a, String, PermissionInfo, String>,
}

impl IndexList<PermissionInfo> for PermissionsIndices<'static> {
    fn get_indexes(&self) -> Box<dyn Iterator<Item = &dyn Index<PermissionInfo>> + '_> {
        let v: Vec<&dyn Index<PermissionInfo>> = vec![&self.action, &self.actor];
        Box::new(v.into_iter())
    }
}

/// Permissions for the ADO contract
///
/// Permissions are stored in a multi-indexed map with the primary key being the action and actor
pub fn permissions() -> IndexedMap<&'static str, PermissionInfo, PermissionsIndices<'static>> {
    let indexes = PermissionsIndices {
        actor: MultiIndex::new(|_pk: &[u8], r| r.actor.clone(), "andr_permissions", "actor"),
        action: MultiIndex::new(
            |_pk: &[u8], r| r.action.clone(),
            "andr_permissions",
            "action",
        ),
    };
    IndexedMap::new("andr_permissions", indexes)
}

impl ADOContract {
    pub fn execute_permissioning(
        &self,
        ctx: ExecuteContext,
        msg: PermissioningMessage,
    ) -> Result<Response, ContractError> {
        match msg {
            PermissioningMessage::SetPermission {
                actors,
                action,
                permission,
            } => self.execute_set_permission(ctx, actors, action, permission),
            PermissioningMessage::RemovePermission { action, actors } => {
                self.execute_remove_permission(ctx, actors, action)
            }
            PermissioningMessage::PermissionAction { action, expiration } => {
                self.execute_permission_action(ctx, action, expiration)
            }
            PermissioningMessage::DisableActionPermissioning { action } => {
                self.execute_disable_action_permission(ctx, action)
            }
        }
    }
    /// Determines if the provided actor is authorised to perform the given action
    ///
    /// Returns an error if the given action is not permissioned for the given actor
    pub fn is_permissioned(
        &self,
        deps: DepsMut,
        env: Env,
        action: impl Into<String>,
        actor: impl Into<String>,
    ) -> Result<Option<SubMsg>, ContractError> {
        // Converted to strings for cloning
        let action_string: String = action.into();
        let actor_string: String = actor.into();

        if self.is_contract_owner(deps.as_ref().storage, actor_string.as_str())? {
            return Ok(None);
        }

        let permission = Self::get_permission(
            deps.as_ref().storage,
            action_string.clone(),
            actor_string.clone(),
        )?;
        let permissioned_action = self
            .permissioned_actions
            .may_load(deps.storage, action_string.clone())?;

        let permissioned_action = match permissioned_action {
            Some(expiry) => match expiry {
                // If the time is expired, it means that the permission is expired and that the action is not longer permissioned
                Some(expiry) => !expiry.is_expired(&env.block),
                // If no expiry is provided by the user, that means that the permission will never expire
                None => true,
            },
            // If there's no entry at all for the permissioned action, that means that the action is not permissioned
            None => false,
        };

        match permission {
            Some(mut some_permission) => {
                match some_permission {
                    Permission::Local(ref mut local_permission) => {
                        handle_local_permission(&env, local_permission, permissioned_action)?;

                        // Consume a use for a limited permission
                        consume_and_save_limited_permission(
                            deps,
                            local_permission,
                            &action_string,
                            &actor_string,
                            permissioned_action,
                        )?;
                        Ok(None)
                    }
                    Permission::Contract(contract_address) => {
                        // Query contract that we'll be referencing the permissions from
                        let addr = contract_address.get_raw_address(&deps.as_ref())?;
                        let mut local_permission =
                            AOSQuerier::get_permission(&deps.querier, &addr, &actor_string)?;

                        handle_local_permission(&env, &mut local_permission, permissioned_action)?;

                        // Limited section
                        if let LocalPermission::Limited { .. } = local_permission {
                            // Only consume a use if the action is permissioned
                            if permissioned_action {
                                local_permission.consume_use()?;
                            }
                        }
                        // Contsruct Sub Msg to update the permission in the address list contract
                        let sub_msg =
                            construct_addresslist_sub_msg(&addr, &actor_string, &local_permission)?;
                        Ok(Some(sub_msg))
                    }
                }
            }
            None => {
                let wildcard = "*";
                let permission = Self::get_permission(
                    deps.as_ref().storage,
                    action_string.clone(),
                    wildcard.to_string(),
                )?;
                let sub_msg = if let Some(mut permission) = permission {
                    match permission {
                        Permission::Local(ref mut local_permission) => {
                            handle_local_permission(&env, local_permission, permissioned_action)?;

                            // Consume a use for a limited permission
                            consume_and_save_limited_permission(
                                deps,
                                local_permission,
                                &action_string,
                                wildcard,
                                permissioned_action,
                            )?;
                            None
                        }
                        Permission::Contract(contract_address) => {
                            // Query contract that we'll be referencing the permissions from
                            let addr = contract_address.get_raw_address(&deps.as_ref())?;
                            let mut local_permission =
                                AOSQuerier::get_permission(&deps.querier, &addr, wildcard)?;

                            handle_local_permission(
                                &env,
                                &mut local_permission,
                                permissioned_action,
                            )?;

                            // Limited section
                            if let LocalPermission::Limited { .. } = local_permission {
                                // Only consume a use if the action is permissioned
                                if permissioned_action {
                                    local_permission.consume_use()?;
                                }
                            }
                            // Contsruct Sub Msg to update the permission in the address list contract
                            let sub_msg =
                                construct_addresslist_sub_msg(&addr, wildcard, &local_permission)?;
                            Some(sub_msg)
                        }
                    }
                } else {
                    ensure!(!permissioned_action, ContractError::Unauthorized {});
                    None
                };
                Ok(sub_msg)
            }
        }
    }

    /// Determines if the provided actor is authorised to perform the given action
    ///
    /// **Ignores the `PERMISSIONED_ACTIONS` map**
    ///
    /// Returns an error if the permission has expired or if no permission exists for a restricted ADO
    pub fn is_permissioned_strict(
        &self,
        deps: DepsMut,
        env: Env,
        action: impl Into<String>,
        actor: impl Into<String>,
    ) -> Result<Option<SubMsg>, ContractError> {
        // Converted to strings for cloning
        let action_string: String = action.into();
        let actor_string: String = actor.into();

        if self.is_contract_owner(deps.storage, actor_string.as_str())? {
            return Ok(None);
        }

        let permission =
            Self::get_permission(deps.storage, action_string.clone(), actor_string.clone())?;
        match permission {
            Some(mut some_permission) => {
                match some_permission {
                    Permission::Local(ref mut local_permission) => {
                        handle_local_permission(&env, local_permission, true)?;

                        consume_and_save_limited_permission(
                            deps,
                            local_permission,
                            &action_string,
                            &actor_string,
                            true,
                        )?;
                        Ok(None)
                    }
                    Permission::Contract(ref contract_address) => {
                        let addr = contract_address.get_raw_address(&deps.as_ref())?;
                        let mut local_permission =
                            AOSQuerier::get_permission(&deps.querier, &addr, &actor_string)?;
                        handle_local_permission(&env, &mut local_permission, true)?;

                        // Limited section
                        if let LocalPermission::Limited { .. } = local_permission {
                            // Always consume a use due to strict setting
                            local_permission.consume_use()?;
                        }
                        // Contsruct Sub Msg to update the permission in the address list contract
                        let sub_msg =
                            construct_addresslist_sub_msg(&addr, &actor_string, &local_permission)?;
                        Ok(Some(sub_msg))
                    }
                }
            }
            None => {
                let wildcard = "*";
                let permission = Self::get_permission(
                    deps.as_ref().storage,
                    action_string.clone(),
                    wildcard.to_string(),
                )?;
                let sub_msg = if let Some(mut permission) = permission {
                    match permission {
                        Permission::Local(ref mut local_permission) => {
                            handle_local_permission(&env, local_permission, true)?;

                            consume_and_save_limited_permission(
                                deps,
                                local_permission,
                                &action_string,
                                wildcard,
                                true,
                            )?;

                            None
                        }
                        Permission::Contract(contract_address) => {
                            // Query contract that we'll be referencing the permissions from
                            let addr = contract_address.get_raw_address(&deps.as_ref())?;
                            let mut local_permission =
                                AOSQuerier::get_permission(&deps.querier, &addr, wildcard)?;

                            handle_local_permission(&env, &mut local_permission, true)?;

                            // Limited section
                            if let LocalPermission::Limited { .. } = local_permission {
                                // Only consume a use if the action is permissioned
                                local_permission.consume_use()?;
                            }
                            // Contsruct Sub Msg to update the permission in the address list contract
                            let sub_msg =
                                construct_addresslist_sub_msg(&addr, wildcard, &local_permission)?;
                            Some(sub_msg)
                        }
                    }
                } else {
                    return Err(ContractError::Unauthorized {});
                };
                Ok(sub_msg)
            }
        }
    }

    /// Gets the permission for the given action and actor
    pub fn get_permission(
        store: &dyn Storage,
        action: impl Into<String>,
        actor: impl Into<String>,
    ) -> Result<Option<Permission>, ContractError> {
        let action = action.into();
        let actor = actor.into();
        let key = action + &actor;
        if let Some(PermissionInfo { permission, .. }) = permissions().may_load(store, &key)? {
            Ok(Some(permission))
        } else {
            Ok(None)
        }
    }

    /// Sets the permission for the given action and actor
    pub fn set_permission(
        store: &mut dyn Storage,
        action: impl Into<String>,
        actor: impl Into<String>,
        permission: Permission,
    ) -> Result<(), ContractError> {
        let action = action.into();
        let actor = actor.into();
        let key = action.clone() + &actor;
        permissions().save(
            store,
            &key,
            &PermissionInfo {
                action,
                actor,
                permission,
            },
        )?;
        Ok(())
    }

    /// Removes the permission for the given action and actor
    pub fn remove_permission(
        store: &mut dyn Storage,
        action: impl Into<String>,
        actor: impl Into<String>,
    ) -> Result<(), ContractError> {
        let action = action.into();
        let actor = actor.into();
        let key = action + &actor;
        permissions().remove(store, &key)?;
        Ok(())
    }

    /// Removes the permission for the given action and actor
    pub fn clear_all_permissions(store: &mut dyn Storage) -> Result<(), ContractError> {
        permissions().clear(store);
        Ok(())
    }

    /// Execute handler for setting permission
    ///
    /// **Whitelisted/Limited permissions will only work for permissioned actions**
    ///
    pub fn execute_set_permission(
        &self,
        ctx: ExecuteContext,
        actors: Vec<AndrAddr>,
        action: impl Into<String>,
        mut permission: Permission,
    ) -> Result<Response, ContractError> {
        ensure!(
            Self::is_contract_owner(self, ctx.deps.storage, ctx.info.sender.as_str())?,
            ContractError::Unauthorized {}
        );
        ensure!(!actors.is_empty(), ContractError::NoActorsProvided {});
        let action = action.into();

        let mut actor_addrs = Vec::new();

        // The asterisk represents a wildcard, it signifies "all addresses"
        if actors.len() == 1 && actors[0].as_str() == "*" {
            actor_addrs.push(Addr::unchecked("*"));
        } else {
            for actor in actors {
                let actor_addr = actor.get_raw_address(&ctx.deps.as_ref())?;
                actor_addrs.push(actor_addr);
            }
        }
        // Last used should always be set at None in the beginning
        permission = match permission {
            Permission::Local(LocalPermission::Whitelisted {
                schedule, window, ..
            }) => Permission::Local(LocalPermission::whitelisted(schedule, window, None)),
            _ => permission,
        };

        let (start, end) = permission.validate_times(&ctx.env)?;
        let verified_schedule = Schedule::new(Some(Expiry::AtTime(start)), end.map(Expiry::AtTime));

        match permission {
            Permission::Local(LocalPermission::Whitelisted {
                window, last_used, ..
            }) => Permission::Local(LocalPermission::whitelisted(
                verified_schedule,
                window,
                last_used,
            )),
            Permission::Local(LocalPermission::Blacklisted { .. }) => {
                Permission::Local(LocalPermission::blacklisted(verified_schedule))
            }
            Permission::Local(LocalPermission::Limited { uses, .. }) => {
                Permission::Local(LocalPermission::limited(verified_schedule, uses))
            }
            Permission::Contract(ref andr_addr) => Permission::Contract(andr_addr.clone()),
        };

        for actor_addr in actor_addrs.clone() {
            Self::set_permission(
                ctx.deps.storage,
                action.clone(),
                actor_addr.clone(),
                permission.clone(),
            )?;
        }

        let actor_strs = actor_addrs
            .iter()
            .map(|addr| addr.as_str())
            .collect::<Vec<_>>()
            .join(", ");

        Ok(Response::default().add_attributes(vec![
            ("action", "set_permission"),
            ("actors", &actor_strs),
            ("action", action.as_str()),
            ("permission", permission.to_string().as_str()),
        ]))
    }

    /// Execute handler for setting permission
    pub fn execute_remove_permission(
        &self,
        ctx: ExecuteContext,
        actors: Vec<AndrAddr>,
        action: impl Into<String>,
    ) -> Result<Response, ContractError> {
        ensure!(
            Self::is_contract_owner(self, ctx.deps.storage, ctx.info.sender.as_str())?,
            ContractError::Unauthorized {}
        );
        ensure!(!actors.is_empty(), ContractError::NoActorsProvided {});

        let action = action.into();
        let mut actor_addrs = Vec::new();

        for actor in actors {
            let actor_addr = actor.get_raw_address(&ctx.deps.as_ref())?;
            actor_addrs.push(actor_addr.clone());
            Self::remove_permission(ctx.deps.storage, action.clone(), actor_addr)?;
        }

        let actor_strs = actor_addrs
            .iter()
            .map(|addr| addr.as_str())
            .collect::<Vec<_>>()
            .join(", ");

        Ok(Response::default().add_attributes(vec![
            ("action", "remove_permission"),
            ("actors", &actor_strs),
            ("action", action.as_str()),
        ]))
    }

    /// Execute handler for clearing all permissions
    pub fn execute_clear_all_permissions(
        &self,
        ctx: ExecuteContext,
    ) -> Result<Response, ContractError> {
        ensure!(
            Self::is_contract_owner(self, ctx.deps.storage, ctx.info.sender.as_str())?,
            ContractError::Unauthorized {}
        );
        Self::clear_all_permissions(ctx.deps.storage)?;

        Ok(Response::default().add_attributes(vec![("action", "clear_all_permissions")]))
    }

    /// Enables permissioning for a given action
    pub fn permission_action(
        &self,
        store: &mut dyn Storage,
        action: impl Into<String>,
        expiration: Option<Milliseconds>,
    ) -> Result<(), ContractError> {
        self.permissioned_actions
            .save(store, action.into(), &expiration)?;
        Ok(())
    }

    /// Disables permissioning for a given action
    pub fn disable_action_permission(&self, action: impl Into<String>, store: &mut dyn Storage) {
        self.permissioned_actions.remove(store, action.into());
    }

    pub fn execute_permission_action(
        &self,
        ctx: ExecuteContext,
        action: impl Into<String>,
        expiration: Option<Expiry>,
    ) -> Result<Response, ContractError> {
        let action_string: String = action.into();
        ensure!(
            Self::is_contract_owner(self, ctx.deps.storage, ctx.info.sender.as_str())?,
            ContractError::Unauthorized {}
        );
        let expiration = expiration.map(|expiry| expiry.get_time(&ctx.env.block));
        self.permission_action(ctx.deps.storage, action_string.clone(), expiration)?;
        Ok(Response::default().add_attributes(vec![
            ("action", "permission_action"),
            ("action", action_string.as_str()),
        ]))
    }

    pub fn execute_disable_action_permission(
        &self,
        ctx: ExecuteContext,
        action: impl Into<String>,
    ) -> Result<Response, ContractError> {
        let action_string: String = action.into();
        ensure!(
            Self::is_contract_owner(self, ctx.deps.storage, ctx.info.sender.as_str())?,
            ContractError::Unauthorized {}
        );
        Self::disable_action_permission(self, action_string.clone(), ctx.deps.storage);
        Ok(Response::default().add_attributes(vec![
            ("action", "disable_action_permission"),
            ("action", action_string.as_str()),
        ]))
    }

    /// Queries all permissions for a given actor
    pub fn query_permissions(
        &self,
        deps: Deps,
        actor: impl Into<String>,
        limit: Option<u32>,
        start_after: Option<String>,
    ) -> Result<Vec<PermissionInfo>, ContractError> {
        let actor = actor.into();
        let min = start_after.map(Bound::inclusive);
        let limit = limit.unwrap_or(DEFAULT_QUERY_LIMIT).min(MAX_QUERY_LIMIT) as usize;
        let permissions = permissions()
            .idx
            .actor
            .prefix(actor)
            .range(deps.storage, min, None, Order::Ascending)
            .take(limit)
            .map(|p| p.unwrap().1)
            .collect::<Vec<PermissionInfo>>();
        Ok(permissions)
    }

    pub fn query_permissioned_actions(&self, deps: Deps) -> Result<Vec<String>, ContractError> {
        let actions = self
            .permissioned_actions
            .keys(deps.storage, None, None, Order::Ascending)
            .map(|p| p.unwrap())
            .collect::<Vec<String>>();
        Ok(actions)
    }

    pub fn query_permissioned_actors(
        &self,
        deps: Deps,
        action: impl Into<String>,
        start_after: Option<String>,
        limit: Option<u32>,
        order_by: Option<OrderBy>,
    ) -> Result<Vec<String>, ContractError> {
        let action_string: String = action.into();
        let order_by = match order_by {
            Some(OrderBy::Desc) => Order::Descending,
            _ => Order::Ascending,
        };

        let actors = permissions()
            .idx
            .action
            .prefix(action_string.clone())
            .keys(
                deps.storage,
                start_after.map(Bound::inclusive),
                None,
                order_by,
            )
            .take((limit).unwrap_or(DEFAULT_QUERY_LIMIT).min(MAX_QUERY_LIMIT) as usize)
            .map(|p| {
                p.unwrap()
                    .strip_prefix(action_string.as_str())
                    .unwrap()
                    .to_string()
            })
            .collect::<Vec<String>>();

        Ok(actors)
    }
}

fn handle_local_permission(
    env: &Env,
    local_permission: &mut LocalPermission,
    permissioned_action: bool,
) -> Result<(), ContractError> {
    ensure!(
        local_permission.is_permissioned(env, permissioned_action),
        ContractError::Unauthorized {}
    );
    // Update last used
    if let LocalPermission::Whitelisted { last_used, .. } = local_permission {
        last_used.replace(Milliseconds::from_seconds(env.block.time.seconds()));
    }

    Ok(())
}

fn consume_and_save_limited_permission(
    deps: DepsMut,
    local_permission: &mut LocalPermission,
    action_string: &str,
    actor_string: &str,
    permissioned_action: bool,
) -> Result<(), ContractError> {
    if let LocalPermission::Limited { .. } = local_permission {
        let mut new_local_permission = local_permission.clone();
        // Consume a use
        new_local_permission.consume_use()?;

        if permissioned_action {
            // Save updated permission info
            permissions().save(
                deps.storage,
                &(action_string.to_string() + actor_string),
                &PermissionInfo {
                    action: action_string.to_string(),
                    actor: actor_string.to_string(),
                    permission: Permission::Local(new_local_permission),
                },
            )?;
        }
    }

    Ok(())
}

/// Contsruct Sub Msg to update the permission in the address list contract
fn construct_addresslist_sub_msg(
    addr: impl Into<String>,
    actor_string: &str,
    local_permission: &LocalPermission,
) -> Result<SubMsg, ContractError> {
    Ok(SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: addr.into(),
        msg: to_json_binary(&AddressListExecuteMsg::PermissionActors {
            actors: vec![AndrAddr::from_string(actor_string.to_string())],
            permission: local_permission.clone(),
        })?,
        funds: vec![],
    })))
}

/// Checks if the provided context is authorised to perform the provided action.
///
/// Two scenarios exist:
/// - The context does not contain any AMP context and the **sender** is the actor
/// - The context contains AMP context and the **previous sender** or **origin** are considered the actor
pub fn is_context_permissioned(
    deps: &mut DepsMut,
    info: &MessageInfo,
    env: &Env,
    ctx: &Option<AMPPkt>,
    action: impl Into<String>,
) -> Result<(bool, Option<SubMsg>), ContractError> {
    let contract = ADOContract::default();

    match ctx {
        Some(amp_ctx) => {
            let action: String = action.into();
            let is_origin_permissioned = contract.is_permissioned(
                deps.branch(),
                env.clone(),
                action.clone(),
                amp_ctx.ctx.get_origin().as_str(),
            );
            if let Ok(submsg) = is_origin_permissioned {
                return Ok((true, submsg));
            }
            let is_previous_sender_permissioned = contract.is_permissioned(
                deps.branch(),
                env.clone(),
                action,
                amp_ctx.ctx.get_previous_sender().as_str(),
            );
            match is_previous_sender_permissioned {
                Ok(Some(submsg)) => Ok((true, Some(submsg))),
                Ok(None) => Ok((true, None)),
                Err(_) => Ok((false, None)),
            }
        }
        None => {
            let is_sender_permissioned = contract.is_permissioned(
                deps.branch(),
                env.clone(),
                action,
                info.sender.to_string(),
            );
            match is_sender_permissioned {
                Ok(Some(submsg)) => Ok((true, Some(submsg))),
                Ok(None) => Ok((true, None)),
                Err(_) => Ok((false, None)),
            }
        }
    }
}

/// Checks if the provided context is authorised to perform the provided action ignoring `PERMISSIONED_ACTIONS`
///
/// Two scenarios exist:
/// - The context does not contain any AMP context and the **sender** is the actor
/// - The context contains AMP context and the **previous sender** or **origin** are considered the actor
pub fn is_context_permissioned_strict(
    mut deps: DepsMut,
    info: &MessageInfo,
    env: &Env,
    ctx: &Option<AMPPkt>,
    action: impl Into<String>,
) -> Result<bool, ContractError> {
    let contract = ADOContract::default();

    match ctx {
        Some(amp_ctx) => {
            let action: String = action.into();
            let is_origin_permissioned = contract.is_permissioned_strict(
                deps.branch(),
                env.clone(),
                action.clone(),
                amp_ctx.ctx.get_origin().as_str(),
            );
            if is_origin_permissioned.is_ok() {
                return Ok(true);
            }
            let is_previous_sender_permissioned = contract.is_permissioned_strict(
                deps.branch(),
                env.clone(),
                action,
                amp_ctx.ctx.get_previous_sender().as_str(),
            );
            Ok(is_previous_sender_permissioned.is_ok())
        }
        None => Ok(contract
            .is_permissioned_strict(deps.branch(), env.clone(), action, info.sender.to_string())
            .is_ok()),
    }
}

pub mod migrate {
    use cosmwasm_schema::cw_serde;
    use cw_storage_plus::Map;

    use crate::common::{expiration::Expiry, schedule::Schedule};

    use super::*;

    /**
     * To migrate from v1 to modern we need to be able to convert the old permission format to the new permission format.
     * We have several wrappers around the raw permission format itself so we must be able to convert each of these to the new format.
     * First we must be able to convert from the raw permission format to the modern raw permission format.
     * Then we must be able to convert the old permission info format to the new permission info format.
     */
    #[cw_serde]
    pub enum PermissionV1 {
        Whitelisted(Option<Expiry>),
        Limited {
            expiration: Option<Expiry>,
            uses: u32,
        },
        Blacklisted(Option<Expiry>),
    }

    #[cw_serde]
    enum PermissionTypeV1 {
        Local(PermissionV1),
        Contract(AndrAddr),
    }

    #[cw_serde]
    struct PermissionV1Info {
        actor: String,
        action: String,
        permission: PermissionTypeV1,
    }

    /**
     * Converts a v1 permission info to a modern permission info
     */
    impl TryFrom<PermissionV1Info> for PermissionInfo {
        type Error = ContractError;

        /// Converts a v1 permission to a modern permission
        fn try_from(value: PermissionV1Info) -> Result<Self, Self::Error> {
            let new_permission = match value.permission {
                PermissionTypeV1::Local(permission) => {
                    let new_permission = LocalPermission::try_from(permission)?;
                    Permission::Local(new_permission)
                }
                PermissionTypeV1::Contract(contract) => Permission::Contract(contract),
            };

            Ok(PermissionInfo {
                actor: value.actor,
                action: value.action,
                permission: new_permission,
            })
        }
    }

    /**
     * Converts a v1 permission to a modern permission
     */
    impl TryFrom<PermissionV1> for LocalPermission {
        type Error = ContractError;

        /// Converts a v1 permission to a modern permission
        fn try_from(value: PermissionV1) -> Result<Self, Self::Error> {
            match value {
                PermissionV1::Whitelisted(exp) => {
                    Ok(Self::whitelisted(Schedule::new(None, exp), None, None))
                }
                PermissionV1::Limited { expiration, uses } => {
                    Ok(Self::limited(Schedule::new(None, expiration), uses))
                }
                PermissionV1::Blacklisted(exp) => Ok(Self::blacklisted(Schedule::new(None, exp))),
            }
        }
    }

    const PERMISSIONS_V1: Map<String, PermissionV1Info> = Map::new("andr_permissions");

    pub fn migrate(storage: &mut dyn Storage) -> Result<(), ContractError> {
        migrate_permissions_v1(storage)?;
        Ok(())
    }

    /// Migrates permissions from the v1 format to the modern format
    fn migrate_permissions_v1(storage: &mut dyn Storage) -> Result<(), ContractError> {
        let old_permissions = PERMISSIONS_V1
            .range(storage, None, None, Order::Ascending)
            // We only care about permissions that match the v1 format
            .filter_map(|p| p.ok())
            .collect::<Vec<(String, PermissionV1Info)>>();
        for (key, old_permission) in old_permissions {
            // Map old permission format to new permission format
            let new_permission = PermissionInfo::try_from(old_permission)?;
            permissions().replace(storage, &key, Some(&new_permission), None)?;
        }
        Ok(())
    }

    #[cfg(test)]
    mod tests {

        use cosmwasm_std::testing::mock_dependencies;

        use super::*;

        #[test]
        pub fn test_migrate_permissions_v1() {
            let mut deps = mock_dependencies();
            // Validate each permission is migrated correctly
            let old_permissions = vec![
                PermissionV1Info {
                    actor: "actor1".to_string(),
                    action: "action".to_string(),
                    permission: PermissionTypeV1::Local(PermissionV1::Whitelisted(None)),
                },
                PermissionV1Info {
                    actor: "actor2".to_string(),
                    action: "action".to_string(),
                    permission: PermissionTypeV1::Local(PermissionV1::Limited {
                        expiration: None,
                        uses: 1,
                    }),
                },
                PermissionV1Info {
                    actor: "actor3".to_string(),
                    action: "action".to_string(),
                    permission: PermissionTypeV1::Local(PermissionV1::Blacklisted(None)),
                },
            ];

            // Save old permissions
            for permission in old_permissions.clone() {
                PERMISSIONS_V1
                    .save(deps.as_mut().storage, permission.actor.clone(), &permission)
                    .unwrap();
            }

            // We also want to check that modern permissions are not migrated
            let modern_permission = PermissionInfo {
                actor: "actor4".to_string(),
                action: "action".to_string(),
                permission: Permission::Local(LocalPermission::Whitelisted {
                    schedule: Schedule::new(None, None),
                    window: None,
                    last_used: None,
                }),
            };
            // Save modern permission
            permissions()
                .save(
                    deps.as_mut().storage,
                    &modern_permission.actor,
                    &modern_permission,
                )
                .unwrap();

            // Migrate permissions
            migrate_permissions_v1(deps.as_mut().storage).unwrap();

            // Check that the permissions have been migrated
            for permission in old_permissions {
                let migrated_permission = permissions()
                    .load(deps.as_ref().storage, &permission.actor)
                    .unwrap();
                assert_eq!(migrated_permission.action, permission.action);
                assert_eq!(migrated_permission.actor, permission.actor);
                assert_eq!(
                    migrated_permission,
                    PermissionInfo::try_from(permission).unwrap(),
                );
            }

            // Check that modern permission is not affected
            let post_modern_permission = permissions()
                .load(deps.as_ref().storage, &modern_permission.actor)
                .unwrap();
            assert_eq!(post_modern_permission, modern_permission);
        }
    }
}

#[cfg(test)]
mod tests {
    pub const OWNER: &str = "cosmwasm1fsgzj6t7udv8zhf6zj32mkqhcjcpv52yph5qsdcl0qt94jgdckqs2g053y";

    use cosmwasm_std::{
        testing::{message_info, mock_dependencies, mock_env},
        Addr,
    };

    use crate::{
        ado_base::AndromedaMsg,
        amp::messages::AMPPkt,
        common::{expiration::Expiry, schedule::Schedule, MillisecondsExpiration},
    };

    use super::*;
    use rstest::*;

    #[test]
    fn test_permissioned_action() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let action = "action";
        let actor = "actor";
        let contract = ADOContract::default();
        contract
            .owner
            .save(deps.as_mut().storage, &Addr::unchecked(OWNER))
            .unwrap();

        ADOContract::default()
            .permission_action(deps.as_mut().storage, action, None)
            .unwrap();

        // Test Whitelisting
        let res = contract.is_permissioned(deps.as_mut(), env.clone(), action, actor);

        assert!(res.is_err());
        let permission = Permission::Local(LocalPermission::Whitelisted {
            schedule: Schedule::new(None, None),
            window: None,
            last_used: None,
        });
        ADOContract::set_permission(deps.as_mut().storage, action, actor, permission).unwrap();

        let res = contract.is_permissioned(deps.as_mut(), env.clone(), action, actor);

        assert!(res.is_ok());

        ADOContract::remove_permission(deps.as_mut().storage, action, actor).unwrap();

        // Test Limited
        let res = contract.is_permissioned(deps.as_mut(), env.clone(), action, actor);

        assert!(res.is_err());
        let permission = Permission::Local(LocalPermission::Limited {
            schedule: Schedule::new(None, None),
            uses: 1,
        });
        ADOContract::set_permission(deps.as_mut().storage, action, actor, permission).unwrap();

        let res = contract.is_permissioned(deps.as_mut(), env.clone(), action, actor);

        assert!(res.is_ok());

        // Ensure use is consumed
        let res = contract.is_permissioned(deps.as_mut(), env.clone(), action, actor);
        assert!(res.is_err());

        ADOContract::default().disable_action_permission(action, deps.as_mut().storage);

        // Ensure limited use does not interfere with non-permissioned action
        let res = contract.is_permissioned(deps.as_mut(), env.clone(), action, actor);
        assert!(res.is_ok());

        ADOContract::remove_permission(deps.as_mut().storage, action, actor).unwrap();

        ADOContract::default()
            .permission_action(deps.as_mut().storage, action, None)
            .unwrap();
        // Test Blacklisted
        let permission = Permission::Local(LocalPermission::Blacklisted {
            schedule: Schedule::new(None, None),
        });
        ADOContract::set_permission(deps.as_mut().storage, action, actor, permission).unwrap();
        let res = contract.is_permissioned(deps.as_mut(), env.clone(), action, actor);

        assert!(res.is_err());
    }

    #[test]
    fn test_unpermissioned_action_blacklisted() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let action = "action";
        let actor = "actor";
        let contract = ADOContract::default();
        contract
            .owner
            .save(deps.as_mut().storage, &Addr::unchecked(OWNER))
            .unwrap();

        ADOContract::default()
            .permission_action(deps.as_mut().storage, action, None)
            .unwrap();

        // Test Blacklisted
        let permission = Permission::Local(LocalPermission::Blacklisted {
            schedule: Schedule::new(None, None),
        });
        ADOContract::set_permission(deps.as_mut().storage, action, actor, permission).unwrap();

        let res = contract.is_permissioned(deps.as_mut(), env, action, actor);

        assert!(res.is_err());
    }

    #[test]
    fn test_strict_permissioning() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let action = "action";
        let actor = "actor";
        let contract = ADOContract::default();
        contract
            .owner
            .save(deps.as_mut().storage, &Addr::unchecked(OWNER))
            .unwrap();

        let res = contract.is_permissioned_strict(deps.as_mut(), env.clone(), action, actor);
        assert!(res.is_err());

        let permission = Permission::Local(LocalPermission::Whitelisted {
            schedule: Schedule::new(None, None),
            window: None,
            last_used: None,
        });
        ADOContract::set_permission(deps.as_mut().storage, action, actor, permission).unwrap();

        let res = contract.is_permissioned_strict(deps.as_mut(), env, action, actor);
        assert!(res.is_ok());
    }

    #[test]
    fn test_owner_escape_clause() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let action = "action";
        let actor = "actor";
        let contract = ADOContract::default();
        contract
            .owner
            .save(deps.as_mut().storage, &Addr::unchecked(actor))
            .unwrap();

        let res = contract.is_permissioned_strict(deps.as_mut(), env.clone(), action, actor);
        assert!(res.is_ok());

        let res = contract.is_permissioned(deps.as_mut(), env, action, actor);
        assert!(res.is_ok());
    }

    #[test]
    fn test_set_permission_unauthorized() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let contract = ADOContract::default();
        contract
            .owner
            .save(deps.as_mut().storage, &Addr::unchecked(OWNER))
            .unwrap();
        let msg = AndromedaMsg::Permissioning(PermissioningMessage::SetPermission {
            actors: vec![AndrAddr::from_string("actor")],
            action: "action".to_string(),
            permission: Permission::Local(LocalPermission::Whitelisted {
                schedule: Schedule::new(None, None),
                window: None,
                last_used: None,
            }),
        });
        let attacker = deps.api.addr_make("attacker");
        let ctx = ExecuteContext::new(deps.as_mut(), message_info(&attacker, &[]), env);
        let res = contract.execute(ctx, msg);

        assert!(res.is_err());
        assert_eq!(res.unwrap_err(), ContractError::Unauthorized {});
    }

    #[test]
    fn test_permission_action_unauthorized() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let contract = ADOContract::default();
        contract
            .owner
            .save(deps.as_mut().storage, &Addr::unchecked(OWNER))
            .unwrap();
        let msg = AndromedaMsg::Permissioning(PermissioningMessage::PermissionAction {
            action: "action".to_string(),
            expiration: None,
        });
        let attacker = deps.api.addr_make("attacker");
        let ctx = ExecuteContext::new(deps.as_mut(), message_info(&attacker, &[]), env);
        let res = contract.execute(ctx, msg);

        assert!(res.is_err());
        assert_eq!(res.unwrap_err(), ContractError::Unauthorized {});
    }

    #[test]
    fn test_disable_permissioning_unauthorized() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let contract = ADOContract::default();
        contract
            .owner
            .save(deps.as_mut().storage, &Addr::unchecked(OWNER))
            .unwrap();
        let msg = AndromedaMsg::Permissioning(PermissioningMessage::DisableActionPermissioning {
            action: "action".to_string(),
        });
        let attacker = deps.api.addr_make("attacker");
        let ctx = ExecuteContext::new(deps.as_mut(), message_info(&attacker, &[]), env);
        let res = contract.execute(ctx, msg);

        assert!(res.is_err());
        assert_eq!(res.unwrap_err(), ContractError::Unauthorized {});
    }

    #[test]
    fn test_remove_permission_unauthorized() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let contract = ADOContract::default();
        contract
            .owner
            .save(deps.as_mut().storage, &Addr::unchecked(OWNER))
            .unwrap();
        let msg = AndromedaMsg::Permissioning(PermissioningMessage::RemovePermission {
            action: "action".to_string(),
            actors: vec![AndrAddr::from_string("actor")],
        });
        let attacker = deps.api.addr_make("attacker");
        let ctx = ExecuteContext::new(deps.as_mut(), message_info(&attacker, &[]), env);
        let res = contract.execute(ctx, msg);

        assert!(res.is_err());
        assert_eq!(res.unwrap_err(), ContractError::Unauthorized {});
    }

    #[test]
    fn test_permission_expiration() {
        let mut deps = mock_dependencies();
        let mut env = mock_env();
        env.block.height = 0;
        let action = "action";
        let actor = "actor";
        let contract = ADOContract::default();
        let time = 2;
        let expiration = Expiry::AtTime(MillisecondsExpiration::from_seconds(time));

        env.block.time = MillisecondsExpiration::from_seconds(0).into();
        contract
            .owner
            .save(deps.as_mut().storage, &Addr::unchecked(OWNER))
            .unwrap();

        ADOContract::default()
            .permission_action(deps.as_mut().storage, action, None)
            .unwrap();

        let res = contract.is_permissioned(deps.as_mut(), env.clone(), action, actor);

        assert!(res.is_err());

        // Test Whitelist
        let permission = Permission::Local(LocalPermission::Whitelisted {
            schedule: Schedule::new(None, Some(expiration.clone())),
            window: None,
            last_used: None,
        });
        ADOContract::set_permission(deps.as_mut().storage, action, actor, permission).unwrap();

        let res = contract.is_permissioned(deps.as_mut(), env.clone(), action, actor);
        assert!(res.is_ok());

        env.block.time = MillisecondsExpiration::from_seconds(time + 1).into();

        let res = contract.is_permissioned(deps.as_mut(), env.clone(), action, actor);
        assert!(res.is_err());

        env.block.time = MillisecondsExpiration::from_seconds(0).into();
        // Test Blacklist
        let permission = Permission::Local(LocalPermission::Blacklisted {
            schedule: Schedule::new(None, Some(expiration.clone())),
        });
        ADOContract::set_permission(deps.as_mut().storage, action, actor, permission).unwrap();

        let res = contract.is_permissioned(deps.as_mut(), env.clone(), action, actor);
        assert!(res.is_err());

        env.block.time = MillisecondsExpiration::from_seconds(time + 1).into();

        let res = contract.is_permissioned(deps.as_mut(), env.clone(), action, actor);
        //Action is still permissioned so this should error
        assert!(res.is_err());

        ADOContract::default().disable_action_permission(action, deps.as_mut().storage);

        // Action is no longer permissioned so this should pass
        let res = contract.is_permissioned(deps.as_mut(), env, action, actor);
        assert!(res.is_ok());
    }

    #[rstest]
    #[case(true, true, false, true)] // Whitelist, at start time, should succeed
    #[case(true, false, false, false)] // Whitelist, before start time, should error
    #[case(true, false, true, true)] // Whitelist, after start time, should succeed
    #[case(false, false, false, true)] // Blacklist, before start time, should succeed
    #[case(false, true, false, false)] // Blacklist, at start time, should error
    #[case(false, false, true, false)] // Blacklist, after start time, should error
    fn test_permission_start_time(
        #[case] is_whitelisted: bool,
        #[case] is_at_start_time: bool,
        #[case] is_after_start_time: bool,
        #[case] expected_success: bool,
    ) {
        let contract = ADOContract::default();
        let action = "action";
        let actor = "actor";
        let start_time = 2;
        let start = Some(Expiry::AtTime(MillisecondsExpiration::from_seconds(
            start_time,
        )));

        let mut deps = mock_dependencies();
        let mut env = mock_env();

        env.block.time = MillisecondsExpiration::from_seconds(if is_at_start_time {
            start_time
        } else if is_after_start_time {
            start_time + 1
        } else {
            0
        })
        .into();

        contract
            .owner
            .save(deps.as_mut().storage, &Addr::unchecked(OWNER))
            .unwrap();

        ADOContract::default()
            .permission_action(deps.as_mut().storage, action, None)
            .unwrap();

        let permission = if is_whitelisted {
            Permission::Local(LocalPermission::Whitelisted {
                schedule: Schedule::new(start, None),
                window: None,
                last_used: None,
            })
        } else {
            Permission::Local(LocalPermission::Blacklisted {
                schedule: Schedule::new(start, None),
            })
        };

        ADOContract::set_permission(deps.as_mut().storage, action, actor, permission).unwrap();

        let res = contract.is_permissioned(deps.as_mut(), env.clone(), action, actor);

        if expected_success {
            assert!(res.is_ok());
        } else {
            assert!(res.is_err());
        }
    }

    #[rstest]
    fn test_permission_start_time_disabled_action() {
        let mut deps = mock_dependencies();
        let mut env = mock_env();
        let contract = ADOContract::default();
        let action = "action";
        let actor = "actor";

        env.block.time = MillisecondsExpiration::from_seconds(0).into();

        ADOContract::default().disable_action_permission(action, deps.as_mut().storage);

        let res = contract.is_permissioned(deps.as_mut(), env, action, actor);
        assert!(res.is_err());
    }

    #[test]
    fn test_context_permissions() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let actor = deps.api.addr_make("actor");
        let info = message_info(&actor, &[]);
        let action = "action";

        let mut context = ExecuteContext::new(deps.as_mut(), info.clone(), env.clone());
        let contract = ADOContract::default();

        contract
            .owner
            .save(context.deps.storage, &Addr::unchecked(OWNER))
            .unwrap();

        assert!(
            is_context_permissioned(
                &mut context.deps,
                &context.info,
                &context.env,
                &context.amp_ctx,
                action
            )
            .unwrap()
            .0
        );

        let mut context = ExecuteContext::new(deps.as_mut(), info.clone(), env.clone());
        ADOContract::default()
            .permission_action(context.deps.storage, action, None)
            .unwrap();

        assert!(
            !is_context_permissioned(
                &mut context.deps,
                &context.info,
                &context.env,
                &context.amp_ctx,
                action
            )
            .unwrap()
            .0
        );

        let mut context = ExecuteContext::new(deps.as_mut(), info.clone(), env.clone());
        let permission = Permission::Local(LocalPermission::Whitelisted {
            schedule: Schedule::new(None, None),
            window: None,
            last_used: None,
        });
        ADOContract::set_permission(context.deps.storage, action, &actor, permission).unwrap();

        assert!(
            is_context_permissioned(
                &mut context.deps,
                &context.info,
                &context.env,
                &context.amp_ctx,
                action
            )
            .unwrap()
            .0
        );

        let mock_actor = deps.api.addr_make("mock_actor");
        let unauth_info = message_info(&mock_actor, &[]);
        let mut context = ExecuteContext::new(deps.as_mut(), unauth_info.clone(), env.clone());

        assert!(
            !is_context_permissioned(
                &mut context.deps,
                &context.info,
                &context.env,
                &context.amp_ctx,
                action
            )
            .unwrap()
            .0
        );

        let amp_ctx = AMPPkt::new(mock_actor.clone(), actor.as_str(), vec![]);
        let mut context =
            ExecuteContext::new(deps.as_mut(), unauth_info.clone(), env.clone()).with_ctx(amp_ctx);

        assert!(
            is_context_permissioned(
                &mut context.deps,
                &context.info,
                &context.env,
                &context.amp_ctx,
                action
            )
            .unwrap()
            .0
        );

        let amp_ctx = AMPPkt::new(&actor, mock_actor.clone(), vec![]);
        let mut context =
            ExecuteContext::new(deps.as_mut(), unauth_info.clone(), env.clone()).with_ctx(amp_ctx);

        assert!(
            is_context_permissioned(
                &mut context.deps,
                &context.info,
                &context.env,
                &context.amp_ctx,
                action
            )
            .unwrap()
            .0
        );

        let amp_ctx = AMPPkt::new(mock_actor.clone(), mock_actor.clone(), vec![]);
        let mut context =
            ExecuteContext::new(deps.as_mut(), unauth_info.clone(), env.clone()).with_ctx(amp_ctx);

        assert!(
            !is_context_permissioned(
                &mut context.deps,
                &context.info,
                &context.env,
                &context.amp_ctx,
                action
            )
            .unwrap()
            .0
        );

        let amp_ctx = AMPPkt::new(OWNER.to_string(), mock_actor.clone(), vec![]);
        let mut context =
            ExecuteContext::new(deps.as_mut(), unauth_info.clone(), env.clone()).with_ctx(amp_ctx);

        assert!(
            is_context_permissioned(
                &mut context.deps,
                &context.info,
                &context.env,
                &context.amp_ctx,
                action
            )
            .unwrap()
            .0
        );

        let amp_ctx = AMPPkt::new(mock_actor.clone(), OWNER.to_string(), vec![]);
        let mut context =
            ExecuteContext::new(deps.as_mut(), unauth_info.clone(), env.clone()).with_ctx(amp_ctx);

        assert!(
            is_context_permissioned(
                &mut context.deps,
                &context.info,
                &context.env,
                &context.amp_ctx,
                action
            )
            .unwrap()
            .0
        );

        let previous_sender = deps.api.addr_make("previous_sender");
        let mut context = ExecuteContext::new(deps.as_mut(), info.clone(), env.clone())
            .with_ctx(AMPPkt::new(info.sender, previous_sender.clone(), vec![]));
        let permission = Permission::Local(LocalPermission::Limited {
            schedule: Schedule::new(None, None),
            uses: 1,
        });
        ADOContract::set_permission(context.deps.storage, action, &actor, permission.clone())
            .unwrap();
        ADOContract::set_permission(
            context.deps.storage,
            action,
            previous_sender.clone(),
            permission.clone(),
        )
        .unwrap();

        assert!(
            is_context_permissioned(
                &mut context.deps,
                &context.info,
                &context.env,
                &context.amp_ctx,
                action
            )
            .unwrap()
            .0
        );

        let actor_permission =
            ADOContract::get_permission(context.deps.storage, action, actor).unwrap();
        let previous_sender_permission =
            ADOContract::get_permission(context.deps.storage, action, previous_sender).unwrap();
        assert_eq!(previous_sender_permission, Some(permission));
        assert_eq!(
            actor_permission,
            Some(Permission::Local(LocalPermission::limited(
                Schedule::new(None, None),
                0,
            )))
        );
    }

    #[test]
    fn test_context_permissions_strict() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let actor = deps.api.addr_make("actor");
        let info = message_info(&actor, &[]);
        let action = "action";

        let context = ExecuteContext::new(deps.as_mut(), info.clone(), env.clone());
        let contract = ADOContract::default();

        contract
            .owner
            .save(context.deps.storage, &Addr::unchecked(OWNER))
            .unwrap();

        assert!(!is_context_permissioned_strict(
            context.deps,
            &context.info,
            &context.env,
            &context.amp_ctx,
            action
        )
        .unwrap());

        let context = ExecuteContext::new(deps.as_mut(), info, env.clone());
        let permission = Permission::Local(LocalPermission::whitelisted(
            Schedule::new(None, None),
            None,
            None,
        ));
        ADOContract::set_permission(context.deps.storage, action, actor.clone(), permission)
            .unwrap();

        assert!(is_context_permissioned_strict(
            context.deps,
            &context.info,
            &context.env,
            &context.amp_ctx,
            action
        )
        .unwrap());

        let mock_actor = deps.api.addr_make("mock_actor");
        let unauth_info = message_info(&mock_actor, &[]);
        let context = ExecuteContext::new(deps.as_mut(), unauth_info.clone(), env.clone());

        assert!(!is_context_permissioned_strict(
            context.deps,
            &context.info,
            &context.env,
            &context.amp_ctx,
            action
        )
        .unwrap());

        let amp_ctx = AMPPkt::new(&mock_actor, &actor, vec![]);
        let context =
            ExecuteContext::new(deps.as_mut(), unauth_info.clone(), env.clone()).with_ctx(amp_ctx);

        assert!(is_context_permissioned_strict(
            context.deps,
            &context.info,
            &context.env,
            &context.amp_ctx,
            action
        )
        .unwrap());

        let amp_ctx = AMPPkt::new(actor, &mock_actor, vec![]);
        let context =
            ExecuteContext::new(deps.as_mut(), unauth_info.clone(), env.clone()).with_ctx(amp_ctx);

        assert!(is_context_permissioned_strict(
            context.deps,
            &context.info,
            &context.env,
            &context.amp_ctx,
            action
        )
        .unwrap());

        let amp_ctx = AMPPkt::new(&mock_actor, &mock_actor, vec![]);
        let context = ExecuteContext::new(deps.as_mut(), unauth_info, env).with_ctx(amp_ctx);

        assert!(!is_context_permissioned_strict(
            context.deps,
            &context.info,
            &context.env,
            &context.amp_ctx,
            action
        )
        .unwrap());
    }

    #[test]
    fn test_query_permissions() {
        let actor = "actor";
        let mut deps = mock_dependencies();

        let permissions = ADOContract::default()
            .query_permissions(deps.as_ref(), actor, None, None)
            .unwrap();

        assert!(permissions.is_empty());

        let permission = Permission::Local(LocalPermission::whitelisted(
            Schedule::new(None, None),
            None,
            None,
        ));
        let action = "action";

        ADOContract::set_permission(deps.as_mut().storage, action, actor, permission.clone())
            .unwrap();

        let permissions = ADOContract::default()
            .query_permissions(deps.as_ref(), actor, None, None)
            .unwrap();

        assert_eq!(permissions.len(), 1);
        assert_eq!(permissions[0].action, action);
        assert_eq!(permissions[0].permission, permission);

        let multi_permissions = vec![
            (
                "action2".to_string(),
                Permission::Local(LocalPermission::blacklisted(Schedule::new(None, None))),
            ),
            (
                "action3".to_string(),
                Permission::Local(LocalPermission::whitelisted(
                    Schedule::new(None, None),
                    None,
                    None,
                )),
            ),
            (
                "action4".to_string(),
                Permission::Local(LocalPermission::blacklisted(Schedule::new(None, None))),
            ),
            (
                "action5".to_string(),
                Permission::Local(LocalPermission::whitelisted(
                    Schedule::new(None, None),
                    None,
                    None,
                )),
            ),
        ];

        for (action, permission) in multi_permissions {
            ADOContract::set_permission(deps.as_mut().storage, &action, actor, permission).unwrap();
        }

        let permissions = ADOContract::default()
            .query_permissions(deps.as_ref(), actor, None, None)
            .unwrap();

        assert_eq!(permissions.len(), 5);
    }

    #[test]
    fn test_query_permissioned_actions() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let owner = deps.api.addr_make("owner");
        let info = message_info(&owner, &[]);
        let ctx = ExecuteContext::new(deps.as_mut(), info.clone(), env);

        let contract = ADOContract::default();

        contract.owner.save(ctx.deps.storage, &info.sender).unwrap();

        let actions = ADOContract::default()
            .query_permissioned_actions(ctx.deps.as_ref())
            .unwrap();

        assert!(actions.is_empty());

        ADOContract::default()
            .execute_permission_action(ctx, "action", None)
            .unwrap();

        let actions = ADOContract::default()
            .query_permissioned_actions(deps.as_ref())
            .unwrap();

        assert_eq!(actions.len(), 1);
        assert_eq!(actions[0], "action");
    }

    #[test]
    fn test_query_permissioned_actors() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let owner = deps.api.addr_make("owner");
        let info = message_info(&owner, &[]);
        let ctx = ExecuteContext::new(deps.as_mut(), info.clone(), env);

        let contract = ADOContract::default();

        contract.owner.save(ctx.deps.storage, &info.sender).unwrap();

        let actor = "actor";
        let actor2 = "actor2";
        let action = "action";
        ADOContract::default()
            .execute_permission_action(ctx, action, None)
            .unwrap();

        ADOContract::set_permission(
            deps.as_mut().storage,
            action,
            actor,
            Permission::Local(LocalPermission::default()),
        )
        .unwrap();
        ADOContract::set_permission(
            deps.as_mut().storage,
            action,
            actor2,
            Permission::Local(LocalPermission::default()),
        )
        .unwrap();
        let actors = ADOContract::default()
            .query_permissioned_actors(deps.as_ref(), action, None, None, None)
            .unwrap();

        assert_eq!(actors.len(), 2);
        assert_eq!(actors[0], actor);
        assert_eq!(actors[1], actor2);
    }

    #[rstest]
    #[case("whitelisted", Permission::Local(LocalPermission::Whitelisted {
        schedule: Schedule::new(None, None),
        window: None,
        last_used: None,
    }), true)]
    #[case("blacklisted", Permission::Local(LocalPermission::Blacklisted {
        schedule: Schedule::new(None, None),
    }), false)]
    #[case("limited", Permission::Local(LocalPermission::Limited {
        schedule: Schedule::new(None, None),
        uses: 4, // Number of actors we're testing with
    }), true)]
    fn test_wildcard_actor_permissions(
        #[case] permission_type: &str,
        #[case] permission: Permission,
        #[case] expected_success: bool,
    ) {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let action = "test_action";
        let test_actors = vec!["actor1", "actor2", "actor3", "different_actor"];
        let contract = ADOContract::default();

        // Set up contract owner
        contract
            .owner
            .save(deps.as_mut().storage, &Addr::unchecked(OWNER))
            .unwrap();

        // Enable permissioning for the action
        ADOContract::default()
            .permission_action(deps.as_mut().storage, action, None)
            .unwrap();

        // Set wildcard permission
        ADOContract::set_permission(deps.as_mut().storage, action, "*", permission.clone())
            .unwrap();

        // Test that all actors are affected by the wildcard permission
        for actor in test_actors {
            let res = contract.is_permissioned(deps.as_mut(), env.clone(), action, actor);

            if expected_success {
                assert!(
                    res.is_ok(),
                    "Actor {} should be allowed with {} wildcard permission",
                    actor,
                    permission_type
                );
            } else {
                assert!(
                    res.is_err(),
                    "Actor {} should be denied with {} wildcard permission",
                    actor,
                    permission_type
                );
            }
        }
    }

    #[rstest]
    #[case( Permission::Local(LocalPermission::Whitelisted {
        schedule: Schedule::new(None, None),
        window: None,
        last_used: None,
    }), true)]
    #[case( Permission::Local(LocalPermission::Blacklisted {
        schedule: Schedule::new(None, None),
    }), false)]
    fn test_wildcard_vs_specific_actor_priority(
        #[case] wildcard_permission: Permission,
        #[case] wildcard_should_allow: bool,
    ) {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let action = "test_action";
        let specific_actor = "specific_actor";
        let other_actor = "other_actor";
        let contract = ADOContract::default();

        // Set up contract owner
        contract
            .owner
            .save(deps.as_mut().storage, &Addr::unchecked(OWNER))
            .unwrap();

        // Enable permissioning for the action
        ADOContract::default()
            .permission_action(deps.as_mut().storage, action, None)
            .unwrap();

        // Set wildcard permission first
        ADOContract::set_permission(
            deps.as_mut().storage,
            action,
            "*",
            wildcard_permission.clone(),
        )
        .unwrap();

        // Set specific actor permission (opposite of wildcard)
        let specific_permission = if wildcard_should_allow {
            Permission::Local(LocalPermission::Blacklisted {
                schedule: Schedule::new(None, None),
            })
        } else {
            Permission::Local(LocalPermission::Whitelisted {
                schedule: Schedule::new(None, None),
                window: None,
                last_used: None,
            })
        };
        ADOContract::set_permission(
            deps.as_mut().storage,
            action,
            specific_actor,
            specific_permission,
        )
        .unwrap();

        // Test that specific actor permission takes precedence over wildcard
        let specific_res =
            contract.is_permissioned(deps.as_mut(), env.clone(), action, specific_actor);
        let other_res = contract.is_permissioned(deps.as_mut(), env.clone(), action, other_actor);

        // Specific actor should have opposite permission of wildcard
        if wildcard_should_allow {
            assert!(
                specific_res.is_err(),
                "Specific actor should be denied despite whitelisted wildcard"
            );
            assert!(
                other_res.is_ok(),
                "Other actors should be allowed by whitelisted wildcard"
            );
        } else {
            assert!(
                specific_res.is_ok(),
                "Specific actor should be allowed despite blacklisted wildcard"
            );
            assert!(
                other_res.is_err(),
                "Other actors should be denied by blacklisted wildcard"
            );
        }
    }

    #[test]
    fn test_wildcard_limited_permission_consumption() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let action = "test_action";
        let test_actors = ["actor1", "actor2", "actor3"];
        let contract = ADOContract::default();

        // Set up contract owner
        contract
            .owner
            .save(deps.as_mut().storage, &Addr::unchecked(OWNER))
            .unwrap();

        // Enable permissioning for the action
        ADOContract::default()
            .permission_action(deps.as_mut().storage, action, None)
            .unwrap();

        // Set wildcard limited permission with 2 uses
        let wildcard_permission = Permission::Local(LocalPermission::Limited {
            schedule: Schedule::new(None, None),
            uses: 2,
        });
        ADOContract::set_permission(deps.as_mut().storage, action, "*", wildcard_permission)
            .unwrap();

        // First two uses should succeed
        for (i, ..) in test_actors.iter().enumerate().take(2) {
            let actor = test_actors[i];
            let res = contract.is_permissioned(deps.as_mut(), env.clone(), action, actor);
            assert!(
                res.is_ok(),
                "Actor {} should be allowed for use {}",
                actor,
                i + 1
            );
        }

        // Third use should fail (limited to 2 uses)
        let res = contract.is_permissioned(deps.as_mut(), env.clone(), action, test_actors[2]);
        assert!(
            res.is_err(),
            "Third use should be denied due to limited uses"
        );
    }

    #[test]
    fn test_wildcard_permission_with_no_specific_permission() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let action = "test_action";
        let test_actor = "test_actor";
        let contract = ADOContract::default();

        // Set up contract owner
        contract
            .owner
            .save(deps.as_mut().storage, &Addr::unchecked(OWNER))
            .unwrap();

        // Enable permissioning for the action
        ADOContract::default()
            .permission_action(deps.as_mut().storage, action, None)
            .unwrap();

        // No specific permission set, should fail
        let res = contract.is_permissioned(deps.as_mut(), env.clone(), action, test_actor);
        assert!(res.is_err(), "Should fail when no permission is set");

        // Set wildcard whitelist permission
        let wildcard_permission = Permission::Local(LocalPermission::Whitelisted {
            schedule: Schedule::new(None, None),
            window: None,
            last_used: None,
        });
        ADOContract::set_permission(deps.as_mut().storage, action, "*", wildcard_permission)
            .unwrap();

        // Now should succeed due to wildcard permission
        let res = contract.is_permissioned(deps.as_mut(), env.clone(), action, test_actor);
        assert!(
            res.is_ok(),
            "Should succeed with wildcard whitelist permission"
        );
    }

    #[test]
    fn test_wildcard_permission_disabled_action() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let action = "test_action";
        let test_actor = "test_actor";
        let contract = ADOContract::default();

        // Set up contract owner
        contract
            .owner
            .save(deps.as_mut().storage, &Addr::unchecked(OWNER))
            .unwrap();

        // Set wildcard whitelist permission
        let wildcard_permission = Permission::Local(LocalPermission::Whitelisted {
            schedule: Schedule::new(None, None),
            window: None,
            last_used: None,
        });
        ADOContract::set_permission(deps.as_mut().storage, action, "*", wildcard_permission)
            .unwrap();

        // Disable action permissioning
        ADOContract::default().disable_action_permission(action, deps.as_mut().storage);

        // Should succeed even with wildcard permission when action is disabled
        let res = contract.is_permissioned(deps.as_mut(), env.clone(), action, test_actor);
        assert!(
            res.is_ok(),
            "Should succeed when action permissioning is disabled"
        );
    }

    #[rstest]
    #[case("whitelisted", Permission::Local(LocalPermission::Whitelisted {
        schedule: Schedule::new(None, None),
        window: None,
        last_used: None,
    }))]
    #[case("blacklisted", Permission::Local(LocalPermission::Blacklisted {
        schedule: Schedule::new(None, None),
    }))]
    #[case("limited", Permission::Local(LocalPermission::Limited {
        schedule: Schedule::new(None, None),
        uses: 5,
    }))]
    fn test_wildcard_permission_removal(
        #[case] permission_type: &str,
        #[case] permission: Permission,
    ) {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let action = "test_action";
        let test_actor = "test_actor";
        let contract = ADOContract::default();

        // Set up contract owner
        contract
            .owner
            .save(deps.as_mut().storage, &Addr::unchecked(OWNER))
            .unwrap();

        // Enable permissioning for the action
        ADOContract::default()
            .permission_action(deps.as_mut().storage, action, None)
            .unwrap();

        // Set wildcard permission
        ADOContract::set_permission(deps.as_mut().storage, action, "*", permission.clone())
            .unwrap();

        // Verify permission works
        let res = contract.is_permissioned(deps.as_mut(), env.clone(), action, test_actor);
        if permission_type == "blacklisted" {
            assert!(
                res.is_err(),
                "Should be denied with blacklisted wildcard permission"
            );
        } else {
            assert!(
                res.is_ok(),
                "Should be allowed with {} wildcard permission",
                permission_type
            );
        }

        // Remove wildcard permission
        ADOContract::remove_permission(deps.as_mut().storage, action, "*").unwrap();

        // Should fail now that wildcard permission is removed
        let res = contract.is_permissioned(deps.as_mut(), env.clone(), action, test_actor);
        assert!(
            res.is_err(),
            "Should fail after wildcard permission is removed"
        );
    }
}
