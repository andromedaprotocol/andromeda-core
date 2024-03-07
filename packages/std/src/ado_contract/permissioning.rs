use crate::{
    ado_base::permissioning::{Permission, PermissionInfo, PermissioningMessage},
    amp::{messages::AMPPkt, AndrAddr},
    common::context::ExecuteContext,
    error::ContractError,
};
use cosmwasm_std::{ensure, Deps, Env, MessageInfo, Order, Response, Storage};
use cw_storage_plus::{Bound, Index, IndexList, IndexedMap, MultiIndex};

use super::ADOContract;

const MAX_QUERY_LIMIT: u32 = 50;
const DEFAULT_QUERY_LIMIT: u32 = 25;

pub struct PermissionsIndices<'a> {
    /// PK: action + actor
    ///
    /// Secondary key: actor
    pub permissions: MultiIndex<'a, String, PermissionInfo, String>,
}

impl<'a> IndexList<PermissionInfo> for PermissionsIndices<'a> {
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<PermissionInfo>> + '_> {
        let v: Vec<&dyn Index<PermissionInfo>> = vec![&self.permissions];
        Box::new(v.into_iter())
    }
}

/// Permissions for the ADO contract
///
/// Permissions are stored in a multi-indexed map with the primary key being the action and actor
pub fn permissions<'a>() -> IndexedMap<'a, &'a str, PermissionInfo, PermissionsIndices<'a>> {
    let indexes = PermissionsIndices {
        permissions: MultiIndex::new(|_pk: &[u8], r| r.actor.clone(), "andr_permissions", "actor"),
    };
    IndexedMap::new("andr_permissions", indexes)
}

impl<'a> ADOContract<'a> {
    pub fn execute_permissioning(
        &self,
        ctx: ExecuteContext,
        msg: PermissioningMessage,
    ) -> Result<Response, ContractError> {
        match msg {
            PermissioningMessage::SetPermission {
                actor,
                action,
                permission,
            } => self.execute_set_permission(ctx, actor, action, permission),
            PermissioningMessage::RemovePermission { action, actor } => {
                self.execute_remove_permission(ctx, actor, action)
            }
            PermissioningMessage::PermissionAction { action } => {
                self.execute_permission_action(ctx, action)
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
        store: &mut dyn Storage,
        env: Env,
        action: impl Into<String>,
        actor: impl Into<String>,
    ) -> Result<(), ContractError> {
        // Converted to strings for cloning
        let action_string: String = action.into();
        let actor_string: String = actor.into();

        if self.is_contract_owner(store, actor_string.as_str())? {
            return Ok(());
        }

        let permission = Self::get_permission(store, action_string.clone(), actor_string.clone())?;
        let permissioned_action = self
            .permissioned_actions
            .may_load(store, action_string.clone())?
            .unwrap_or(false);
        match permission {
            Some(mut permission) => {
                ensure!(
                    permission.is_permissioned(&env, permissioned_action),
                    ContractError::Unauthorized {}
                );

                // Consume a use for a limited permission
                if let Permission::Limited { .. } = permission {
                    permission.consume_use()?;
                    permissions().save(
                        store,
                        (action_string.clone() + actor_string.as_str()).as_str(),
                        &PermissionInfo {
                            action: action_string,
                            actor: actor_string,
                            permission,
                        },
                    )?;
                }

                Ok(())
            }
            None => {
                ensure!(!permissioned_action, ContractError::Unauthorized {});
                Ok(())
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
        store: &mut dyn Storage,
        env: Env,
        action: impl Into<String>,
        actor: impl Into<String>,
    ) -> Result<(), ContractError> {
        // Converted to strings for cloning
        let action_string: String = action.into();
        let actor_string: String = actor.into();

        if self.is_contract_owner(store, actor_string.as_str())? {
            return Ok(());
        }

        let permission = Self::get_permission(store, action_string.clone(), actor_string.clone())?;
        match permission {
            Some(mut permission) => {
                ensure!(
                    permission.is_permissioned(&env, true),
                    ContractError::Unauthorized {}
                );

                // Consume a use for a limited permission
                if let Permission::Limited { .. } = permission {
                    permission.consume_use()?;
                    permissions().save(
                        store,
                        (action_string.clone() + actor_string.as_str()).as_str(),
                        &PermissionInfo {
                            action: action_string,
                            actor: actor_string,
                            permission,
                        },
                    )?;
                }

                Ok(())
            }
            None => Err(ContractError::Unauthorized {}),
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

    /// Execute handler for setting permission
    ///
    /// **Whitelisted/Limited permissions will only work for permissioned actions**
    ///
    pub fn execute_set_permission(
        &self,
        ctx: ExecuteContext,
        actor: AndrAddr,
        action: impl Into<String>,
        permission: Permission,
    ) -> Result<Response, ContractError> {
        ensure!(
            Self::is_contract_owner(self, ctx.deps.storage, ctx.info.sender.as_str())?,
            ContractError::Unauthorized {}
        );
        let actor_addr = actor.get_raw_address(&ctx.deps.as_ref())?;
        let action = action.into();
        Self::set_permission(
            ctx.deps.storage,
            action.clone(),
            actor_addr.clone(),
            permission.clone(),
        )?;

        Ok(Response::default().add_attributes(vec![
            ("action", "set_permission"),
            ("actor", actor_addr.as_str()),
            ("action", action.as_str()),
            ("permission", permission.to_string().as_str()),
        ]))
    }

    /// Execute handler for setting permission
    pub fn execute_remove_permission(
        &self,
        ctx: ExecuteContext,
        actor: AndrAddr,
        action: impl Into<String>,
    ) -> Result<Response, ContractError> {
        ensure!(
            Self::is_contract_owner(self, ctx.deps.storage, ctx.info.sender.as_str())?,
            ContractError::Unauthorized {}
        );
        let actor_addr = actor.get_raw_address(&ctx.deps.as_ref())?;
        let action = action.into();
        Self::remove_permission(ctx.deps.storage, action.clone(), actor_addr.clone())?;

        Ok(Response::default().add_attributes(vec![
            ("action", "remove_permission"),
            ("actor", actor_addr.as_str()),
            ("action", action.as_str()),
        ]))
    }

    /// Enables permissioning for a given action
    pub fn permission_action(
        &self,
        action: impl Into<String>,
        store: &mut dyn Storage,
    ) -> Result<(), ContractError> {
        self.permissioned_actions
            .save(store, action.into(), &true)?;
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
    ) -> Result<Response, ContractError> {
        let action_string: String = action.into();
        ensure!(
            Self::is_contract_owner(self, ctx.deps.storage, ctx.info.sender.as_str())?,
            ContractError::Unauthorized {}
        );
        self.permission_action(action_string.clone(), ctx.deps.storage)?;
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
            .permissions
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
}

/// Checks if the provided context is authorised to perform the provided action.
///
/// Two scenarios exist:
/// - The context does not contain any AMP context and the **sender** is the actor
/// - The context contains AMP context and the **previous sender** or **origin** are considered the actor
pub fn is_context_permissioned(
    storage: &mut dyn Storage,
    info: &MessageInfo,
    env: &Env,
    ctx: &Option<AMPPkt>,
    action: impl Into<String>,
) -> Result<bool, ContractError> {
    let contract = ADOContract::default();

    match ctx {
        Some(amp_ctx) => {
            let action: String = action.into();
            let is_origin_permissioned = contract.is_permissioned(
                storage,
                env.clone(),
                action.clone(),
                amp_ctx.ctx.get_origin().as_str(),
            );
            let is_previous_sender_permissioned = contract.is_permissioned(
                storage,
                env.clone(),
                action,
                amp_ctx.ctx.get_previous_sender().as_str(),
            );
            Ok(is_origin_permissioned.is_ok() || is_previous_sender_permissioned.is_ok())
        }
        None => Ok(contract
            .is_permissioned(storage, env.clone(), action, info.sender.to_string())
            .is_ok()),
    }
}

/// Checks if the provided context is authorised to perform the provided action ignoring `PERMISSIONED_ACTIONS`
///
/// Two scenarios exist:
/// - The context does not contain any AMP context and the **sender** is the actor
/// - The context contains AMP context and the **previous sender** or **origin** are considered the actor
pub fn is_context_permissioned_strict(
    storage: &mut dyn Storage,
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
                storage,
                env.clone(),
                action.clone(),
                amp_ctx.ctx.get_origin().as_str(),
            );
            let is_previous_sender_permissioned = contract.is_permissioned_strict(
                storage,
                env.clone(),
                action,
                amp_ctx.ctx.get_previous_sender().as_str(),
            );
            Ok(is_origin_permissioned.is_ok() || is_previous_sender_permissioned.is_ok())
        }
        None => Ok(contract
            .is_permissioned_strict(storage, env.clone(), action, info.sender.to_string())
            .is_ok()),
    }
}

#[cfg(test)]
mod tests {
    use cosmwasm_std::{
        testing::{mock_dependencies, mock_env, mock_info},
        Addr,
    };
    use cw_utils::Expiration;

    use crate::{ado_base::AndromedaMsg, amp::messages::AMPPkt};

    use super::*;

    #[test]
    fn test_permissioned_action() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let action = "action";
        let actor = "actor";
        let contract = ADOContract::default();
        contract
            .owner
            .save(deps.as_mut().storage, &Addr::unchecked("owner"))
            .unwrap();

        ADOContract::default()
            .permission_action(action, deps.as_mut().storage)
            .unwrap();

        // Test Whitelisting
        let res = contract.is_permissioned(deps.as_mut().storage, env.clone(), action, actor);

        assert!(res.is_err());
        let permission = Permission::whitelisted(None);
        ADOContract::set_permission(deps.as_mut().storage, action, actor, permission).unwrap();

        let res = contract.is_permissioned(deps.as_mut().storage, env.clone(), action, actor);

        assert!(res.is_ok());

        ADOContract::remove_permission(deps.as_mut().storage, action, actor).unwrap();

        // Test Limited
        let res = contract.is_permissioned(deps.as_mut().storage, env.clone(), action, actor);

        assert!(res.is_err());
        let permission = Permission::limited(None, 1);
        ADOContract::set_permission(deps.as_mut().storage, action, actor, permission).unwrap();

        let res = contract.is_permissioned(deps.as_mut().storage, env.clone(), action, actor);

        assert!(res.is_ok());

        // Ensure use is consumed
        let res = contract.is_permissioned(deps.as_mut().storage, env.clone(), action, actor);
        assert!(res.is_err());

        ADOContract::remove_permission(deps.as_mut().storage, action, actor).unwrap();

        // Test Blacklisted
        let permission = Permission::blacklisted(None);
        ADOContract::set_permission(deps.as_mut().storage, action, actor, permission).unwrap();

        let res = contract.is_permissioned(deps.as_mut().storage, env, action, actor);

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
            .save(deps.as_mut().storage, &Addr::unchecked("owner"))
            .unwrap();

        ADOContract::default()
            .permission_action(action, deps.as_mut().storage)
            .unwrap();

        // Test Blacklisted
        let permission = Permission::blacklisted(None);
        ADOContract::set_permission(deps.as_mut().storage, action, actor, permission).unwrap();

        let res = contract.is_permissioned(deps.as_mut().storage, env, action, actor);

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
            .save(deps.as_mut().storage, &Addr::unchecked("owner"))
            .unwrap();

        let res =
            contract.is_permissioned_strict(deps.as_mut().storage, env.clone(), action, actor);
        assert!(res.is_err());

        let permission = Permission::whitelisted(None);
        ADOContract::set_permission(deps.as_mut().storage, action, actor, permission).unwrap();

        let res = contract.is_permissioned_strict(deps.as_mut().storage, env, action, actor);
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

        let res =
            contract.is_permissioned_strict(deps.as_mut().storage, env.clone(), action, actor);
        assert!(res.is_ok());

        let res = contract.is_permissioned(deps.as_mut().storage, env, action, actor);
        assert!(res.is_ok());
    }

    #[test]
    fn test_set_permission_unauthorized() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let contract = ADOContract::default();
        contract
            .owner
            .save(deps.as_mut().storage, &Addr::unchecked("owner"))
            .unwrap();
        let msg = AndromedaMsg::Permissioning(PermissioningMessage::SetPermission {
            actor: AndrAddr::from_string("actor"),
            action: "action".to_string(),
            permission: Permission::Whitelisted(None),
        });
        let ctx = ExecuteContext::new(deps.as_mut(), mock_info("attacker", &[]), env);
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
            .save(deps.as_mut().storage, &Addr::unchecked("owner"))
            .unwrap();
        let msg = AndromedaMsg::Permissioning(PermissioningMessage::PermissionAction {
            action: "action".to_string(),
        });
        let ctx = ExecuteContext::new(deps.as_mut(), mock_info("attacker", &[]), env);
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
            .save(deps.as_mut().storage, &Addr::unchecked("owner"))
            .unwrap();
        let msg = AndromedaMsg::Permissioning(PermissioningMessage::DisableActionPermissioning {
            action: "action".to_string(),
        });
        let ctx = ExecuteContext::new(deps.as_mut(), mock_info("attacker", &[]), env);
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
            .save(deps.as_mut().storage, &Addr::unchecked("owner"))
            .unwrap();
        let msg = AndromedaMsg::Permissioning(PermissioningMessage::RemovePermission {
            action: "action".to_string(),
            actor: AndrAddr::from_string("actor"),
        });
        let ctx = ExecuteContext::new(deps.as_mut(), mock_info("attacker", &[]), env);
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
        let block = 100;
        let expiration = Expiration::AtHeight(block);
        contract
            .owner
            .save(deps.as_mut().storage, &Addr::unchecked("owner"))
            .unwrap();

        ADOContract::default()
            .permission_action(action, deps.as_mut().storage)
            .unwrap();

        let res = contract.is_permissioned(deps.as_mut().storage, env.clone(), action, actor);

        assert!(res.is_err());

        // Test Whitelist
        let permission = Permission::Whitelisted(Some(expiration));
        ADOContract::set_permission(deps.as_mut().storage, action, actor, permission).unwrap();

        let res = contract.is_permissioned(deps.as_mut().storage, env.clone(), action, actor);
        assert!(res.is_ok());

        env.block.height = block + 1;

        let res = contract.is_permissioned(deps.as_mut().storage, env.clone(), action, actor);
        assert!(res.is_err());

        env.block.height = 0;
        // Test Blacklist
        let permission = Permission::Blacklisted(Some(expiration));
        ADOContract::set_permission(deps.as_mut().storage, action, actor, permission).unwrap();

        let res = contract.is_permissioned(deps.as_mut().storage, env.clone(), action, actor);
        assert!(res.is_err());

        env.block.height = block + 1;

        let res = contract.is_permissioned(deps.as_mut().storage, env, action, actor);
        assert!(res.is_ok());
    }

    #[test]
    fn test_context_permissions() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let actor = "actor";
        let info = mock_info(actor, &[]);
        let action = "action";

        let context = ExecuteContext::new(deps.as_mut(), info.clone(), env.clone());
        let contract = ADOContract::default();

        contract
            .owner
            .save(context.deps.storage, &Addr::unchecked("owner"))
            .unwrap();

        assert!(is_context_permissioned(
            context.deps.storage,
            &context.info,
            &context.env,
            &context.amp_ctx,
            action
        )
        .unwrap());

        let context = ExecuteContext::new(deps.as_mut(), info.clone(), env.clone());
        ADOContract::default()
            .permission_action(action, context.deps.storage)
            .unwrap();

        assert!(!is_context_permissioned(
            context.deps.storage,
            &context.info,
            &context.env,
            &context.amp_ctx,
            action
        )
        .unwrap());

        let context = ExecuteContext::new(deps.as_mut(), info, env.clone());
        let permission = Permission::whitelisted(None);
        ADOContract::set_permission(context.deps.storage, action, actor, permission).unwrap();

        assert!(is_context_permissioned(
            context.deps.storage,
            &context.info,
            &context.env,
            &context.amp_ctx,
            action
        )
        .unwrap());

        let unauth_info = mock_info("mock_actor", &[]);
        let context = ExecuteContext::new(deps.as_mut(), unauth_info.clone(), env.clone());

        assert!(!is_context_permissioned(
            context.deps.storage,
            &context.info,
            &context.env,
            &context.amp_ctx,
            action
        )
        .unwrap());

        let amp_ctx = AMPPkt::new("mock_actor", actor, vec![]);
        let context =
            ExecuteContext::new(deps.as_mut(), unauth_info.clone(), env.clone()).with_ctx(amp_ctx);

        assert!(is_context_permissioned(
            context.deps.storage,
            &context.info,
            &context.env,
            &context.amp_ctx,
            action
        )
        .unwrap());

        let amp_ctx = AMPPkt::new(actor, "mock_actor", vec![]);
        let context =
            ExecuteContext::new(deps.as_mut(), unauth_info.clone(), env.clone()).with_ctx(amp_ctx);

        assert!(is_context_permissioned(
            context.deps.storage,
            &context.info,
            &context.env,
            &context.amp_ctx,
            action
        )
        .unwrap());

        let amp_ctx = AMPPkt::new("mock_actor", "mock_actor", vec![]);
        let context =
            ExecuteContext::new(deps.as_mut(), unauth_info.clone(), env.clone()).with_ctx(amp_ctx);

        assert!(!is_context_permissioned(
            context.deps.storage,
            &context.info,
            &context.env,
            &context.amp_ctx,
            action
        )
        .unwrap());

        let amp_ctx = AMPPkt::new("owner", "mock_actor", vec![]);
        let context =
            ExecuteContext::new(deps.as_mut(), unauth_info.clone(), env.clone()).with_ctx(amp_ctx);

        assert!(is_context_permissioned(
            context.deps.storage,
            &context.info,
            &context.env,
            &context.amp_ctx,
            action
        )
        .unwrap());

        let amp_ctx = AMPPkt::new("mock_actor", "owner", vec![]);
        let context = ExecuteContext::new(deps.as_mut(), unauth_info, env).with_ctx(amp_ctx);

        assert!(is_context_permissioned(
            context.deps.storage,
            &context.info,
            &context.env,
            &context.amp_ctx,
            action
        )
        .unwrap());
    }

    #[test]
    fn test_context_permissions_strict() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let actor = "actor";
        let info = mock_info(actor, &[]);
        let action = "action";

        let context = ExecuteContext::new(deps.as_mut(), info.clone(), env.clone());
        let contract = ADOContract::default();

        contract
            .owner
            .save(context.deps.storage, &Addr::unchecked("owner"))
            .unwrap();

        assert!(!is_context_permissioned_strict(
            context.deps.storage,
            &context.info,
            &context.env,
            &context.amp_ctx,
            action
        )
        .unwrap());

        let context = ExecuteContext::new(deps.as_mut(), info, env.clone());
        let permission = Permission::whitelisted(None);
        ADOContract::set_permission(context.deps.storage, action, actor, permission).unwrap();

        assert!(is_context_permissioned_strict(
            context.deps.storage,
            &context.info,
            &context.env,
            &context.amp_ctx,
            action
        )
        .unwrap());

        let unauth_info = mock_info("mock_actor", &[]);
        let context = ExecuteContext::new(deps.as_mut(), unauth_info.clone(), env.clone());

        assert!(!is_context_permissioned_strict(
            context.deps.storage,
            &context.info,
            &context.env,
            &context.amp_ctx,
            action
        )
        .unwrap());

        let amp_ctx = AMPPkt::new("mock_actor", actor, vec![]);
        let context =
            ExecuteContext::new(deps.as_mut(), unauth_info.clone(), env.clone()).with_ctx(amp_ctx);

        assert!(is_context_permissioned_strict(
            context.deps.storage,
            &context.info,
            &context.env,
            &context.amp_ctx,
            action
        )
        .unwrap());

        let amp_ctx = AMPPkt::new(actor, "mock_actor", vec![]);
        let context =
            ExecuteContext::new(deps.as_mut(), unauth_info.clone(), env.clone()).with_ctx(amp_ctx);

        assert!(is_context_permissioned_strict(
            context.deps.storage,
            &context.info,
            &context.env,
            &context.amp_ctx,
            action
        )
        .unwrap());

        let amp_ctx = AMPPkt::new("mock_actor", "mock_actor", vec![]);
        let context = ExecuteContext::new(deps.as_mut(), unauth_info, env).with_ctx(amp_ctx);

        assert!(!is_context_permissioned_strict(
            context.deps.storage,
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

        let permission = Permission::whitelisted(None);
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
            ("action2".to_string(), Permission::blacklisted(None)),
            ("action3".to_string(), Permission::whitelisted(None)),
            ("action4".to_string(), Permission::blacklisted(None)),
            ("action5".to_string(), Permission::whitelisted(None)),
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
        let info = mock_info("owner", &[]);
        let ctx = ExecuteContext {
            deps: deps.as_mut(),
            env,
            info: info.clone(),
            amp_ctx: None,
        };

        let contract = ADOContract::default();

        contract.owner.save(ctx.deps.storage, &info.sender).unwrap();

        let actions = ADOContract::default()
            .query_permissioned_actions(ctx.deps.as_ref())
            .unwrap();

        assert!(actions.is_empty());

        ADOContract::default()
            .execute_permission_action(ctx, "action")
            .unwrap();

        let actions = ADOContract::default()
            .query_permissioned_actions(deps.as_ref())
            .unwrap();

        assert_eq!(actions.len(), 1);
        assert_eq!(actions[0], "action");
    }
}
