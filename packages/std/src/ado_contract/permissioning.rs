use cosmwasm_schema::cw_serde;
use cosmwasm_std::{ensure, DepsMut, Env, MessageInfo, Response, StdError, Storage};
use cw_storage_plus::{Index, IndexList, IndexedMap, Map, MultiIndex};
use cw_utils::Expiration;

use crate::{
    amp::messages::{AMPCtx, AMPPkt},
    common::context::ExecuteContext,
    error::ContractError,
};

use super::ADOContract;

pub const PERMISSIONED_ACTIONS: Map<String, bool> = Map::new("andr_permissioned_actions");

/// An enum to represent a user's permission for an action
///
/// - **Blacklisted** - The user cannot perform the action until after the provided expiration
/// - **Limited** - The user can perform the action while uses are remaining and before the provided expiration **for a permissioned action**
/// - **Whitelisted** - The user can perform the action until the provided expiration **for a permissioned action**
///
/// Expiration defaults to `Never` if not provided
#[cw_serde]
pub enum Permission {
    Blacklisted(Option<Expiration>),
    Limited {
        expiration: Option<Expiration>,
        uses: u32,
    },
    Whitelisted(Option<Expiration>),
}

impl Permission {
    pub fn default() -> Self {
        Self::Whitelisted(None)
    }

    pub fn blacklisted(expiration: Option<Expiration>) -> Self {
        Self::Blacklisted(expiration)
    }

    pub fn whitelisted(expiration: Option<Expiration>) -> Self {
        Self::Whitelisted(expiration)
    }

    pub fn limited(expiration: Option<Expiration>, uses: u32) -> Self {
        Self::Limited { expiration, uses }
    }

    pub fn is_permissioned(&self, env: &Env, strict: bool) -> bool {
        match self {
            Self::Blacklisted(expiration) => {
                if let Some(expiration) = expiration {
                    if expiration.is_expired(&env.block) {
                        return true;
                    }
                }
                false
            }
            Self::Limited { expiration, uses } => {
                if let Some(expiration) = expiration {
                    if expiration.is_expired(&env.block) {
                        return !strict;
                    }
                }
                if *uses == 0 {
                    return !strict;
                }
                true
            }
            Self::Whitelisted(expiration) => {
                if let Some(expiration) = expiration {
                    if expiration.is_expired(&env.block) {
                        return !strict;
                    }
                }
                true
            }
        }
    }

    pub fn get_expiration(&self) -> Expiration {
        match self {
            Self::Blacklisted(expiration) => expiration.unwrap_or_default(),
            Self::Limited { expiration, .. } => expiration.unwrap_or_default(),
            Self::Whitelisted(expiration) => expiration.unwrap_or_default(),
        }
    }

    pub fn to_string(&self) -> String {
        match self {
            Self::Blacklisted(expiration) => {
                if let Some(expiration) = expiration {
                    format!("blacklisted:{}", expiration)
                } else {
                    "blacklisted".to_string()
                }
            }
            Self::Limited { expiration, uses } => {
                if let Some(expiration) = expiration {
                    format!("limited:{}:{}", expiration, uses)
                } else {
                    format!("limited:{}", uses)
                }
            }
            Self::Whitelisted(expiration) => {
                if let Some(expiration) = expiration {
                    format!("whitelisted:{}", expiration)
                } else {
                    "whitelisted".to_string()
                }
            }
        }
    }

    pub fn consume_use(&mut self) {
        match self {
            Self::Limited { uses, .. } => *uses -= 1,
            _ => {}
        }
    }
}

pub struct PermissionsIndices<'a> {
    /// PK: action + identifier
    ///
    /// Secondary key: identifier
    pub permissions: MultiIndex<'a, String, Permission, String>,
}

impl<'a> IndexList<Permission> for PermissionsIndices<'a> {
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<Permission>> + '_> {
        let v: Vec<&dyn Index<Permission>> = vec![&self.permissions];
        Box::new(v.into_iter())
    }
}

/// Permissions for the ADO contract
///
/// Permissions are stored in a multi-indexed map with the primary key being the action and identifier
pub fn permissions<'a>() -> IndexedMap<'a, &'a str, Permission, PermissionsIndices<'a>> {
    let indexes = PermissionsIndices {
        permissions: MultiIndex::new(
            |_pk: &[u8], r| r.to_string(),
            "andr_permissions",
            "identifier",
        ),
    };
    IndexedMap::new("andr_permissions", indexes)
}

impl<'a> ADOContract<'a> {
    /// Determines if the provided identifier is authorised to perform the given action
    ///
    /// Returns an error if the given action is not permissioned for the given identifier
    pub fn is_permissioned(
        &self,
        store: &mut dyn Storage,
        env: Env,
        action: impl Into<String>,
        identifier: impl Into<String>,
    ) -> Result<(), ContractError> {
        // Converted to strings for cloning
        let action_string: String = action.into();
        let identifier_string: String = identifier.into();

        if self.is_contract_owner(store, identifier_string.as_str())? {
            return Ok(());
        }

        let permission =
            Self::get_permission(store, action_string.clone(), identifier_string.clone())?;
        let permissioned_action = PERMISSIONED_ACTIONS
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
                    permission.consume_use();
                    permissions().save(
                        store,
                        (action_string + &identifier_string.as_str()).as_str(),
                        &permission,
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

    /// Determines if the provided identifier is authorised to perform the given action
    ///
    /// **Ignores the `PERMISSIONED_ACTIONS` map**
    ///
    /// Returns an error if the permission has expired or if no permission exists for a restricted ADO
    pub fn is_permissioned_strict(
        &self,
        store: &mut dyn Storage,
        env: Env,
        action: impl Into<String>,
        identifier: impl Into<String>,
    ) -> Result<(), ContractError> {
        // Converted to strings for cloning
        let action_string: String = action.into();
        let identifier_string: String = identifier.into();

        if self.is_contract_owner(store, identifier_string.as_str())? {
            return Ok(());
        }

        let permission =
            Self::get_permission(store, action_string.clone(), identifier_string.clone())?;
        match permission {
            Some(mut permission) => {
                ensure!(
                    permission.is_permissioned(&env, true),
                    ContractError::Unauthorized {}
                );

                // Consume a use for a limited permission
                if let Permission::Limited { .. } = permission {
                    permission.consume_use();
                    permissions().save(
                        store,
                        (action_string + &identifier_string.as_str()).as_str(),
                        &permission,
                    )?;
                }

                Ok(())
            }
            None => Err(ContractError::Unauthorized {}),
        }
    }

    /// Gets the permission for the given action and identifier
    pub fn get_permission(
        store: &dyn Storage,
        action: impl Into<String>,
        identifier: impl Into<String>,
    ) -> Result<Option<Permission>, ContractError> {
        let action = action.into();
        let identifier = identifier.into();
        let key = action + &identifier;
        Ok(permissions().may_load(store, &key)?)
    }

    /// Sets the permission for the given action and identifier
    pub fn set_permission(
        store: &mut dyn Storage,
        action: impl Into<String>,
        identifier: impl Into<String>,
        permission: Permission,
    ) -> Result<(), ContractError> {
        let action = action.into();
        let identifier = identifier.into();
        let key = action + &identifier;
        permissions().save(store, &key, &permission)?;
        Ok(())
    }

    /// Removes the permission for the given action and identifier
    pub fn remove_permission(
        store: &mut dyn Storage,
        action: impl Into<String>,
        identifier: impl Into<String>,
    ) -> Result<(), ContractError> {
        let action = action.into();
        let identifier = identifier.into();
        let key = action + &identifier;
        permissions().remove(store, &key)?;
        Ok(())
    }

    /// Execute handler for setting permission
    ///
    /// **Whitelisted/Limited permissions will only work for permissioned actions**
    ///
    /// TODO: Add permission for execute context
    pub fn execute_set_permission(
        &self,
        ctx: ExecuteContext,
        identifier: impl Into<String>,
        action: impl Into<String>,
        permission: Permission,
    ) -> Result<Response, ContractError> {
        Self::is_contract_owner(&self, ctx.deps.storage, ctx.info.sender.as_str())?;
        let identifier = identifier.into();
        let action = action.into();
        Self::set_permission(
            ctx.deps.storage,
            action.clone(),
            identifier.clone(),
            permission.clone(),
        )?;

        Ok(Response::default().add_attributes(vec![
            ("action", "set_permission"),
            ("identifier", identifier.as_str()),
            ("action", action.as_str()),
            ("permission", permission.to_string().as_str()),
        ]))
    }

    /// Execute handler for setting permission
    /// TODO: Add permission for execute context
    pub fn execute_remove_permission(
        &self,
        ctx: ExecuteContext,
        identifier: impl Into<String>,
        action: impl Into<String>,
    ) -> Result<Response, ContractError> {
        Self::is_contract_owner(&self, ctx.deps.storage, ctx.info.sender.as_str())?;
        let identifier = identifier.into();
        let action = action.into();
        Self::remove_permission(ctx.deps.storage, action.clone(), identifier.clone())?;

        Ok(Response::default().add_attributes(vec![
            ("action", "remove_permission"),
            ("identifier", identifier.as_str()),
            ("action", action.as_str()),
        ]))
    }

    /// Enables permissioning for a given action
    pub fn permission_action(
        action: impl Into<String>,
        store: &mut dyn Storage,
    ) -> Result<(), ContractError> {
        PERMISSIONED_ACTIONS.save(store, action.into(), &true)?;
        Ok(())
    }

    /// Disables permissioning for a given action
    pub fn disable_action_permission(&self, action: impl Into<String>, store: &mut dyn Storage) {
        PERMISSIONED_ACTIONS.remove(store, action.into());
    }

    pub fn execute_permission_action(
        &self,
        ctx: ExecuteContext,
        action: impl Into<String>,
    ) -> Result<Response, ContractError> {
        let action_string: String = action.into();
        Self::is_contract_owner(&self, ctx.deps.storage, ctx.info.sender.as_str())?;
        Self::permission_action(action_string.clone(), ctx.deps.storage)?;
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
        Self::is_contract_owner(&self, ctx.deps.storage, ctx.info.sender.as_str())?;
        Self::disable_action_permission(&self, action_string.clone(), ctx.deps.storage);
        Ok(Response::default().add_attributes(vec![
            ("action", "disable_action_permission"),
            ("action", action_string.as_str()),
        ]))
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

    use crate::amp::messages::AMPPkt;

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

        ADOContract::permission_action(action, deps.as_mut().storage).unwrap();

        // Test Whitelisting
        let res = contract.is_permissioned(deps.as_mut().storage, env.clone(), action, actor);

        assert!(res.is_err());
        let permission = Permission::whitelisted(None);
        ADOContract::set_permission(deps.as_mut().storage, action, actor, permission.clone())
            .unwrap();

        let res = contract.is_permissioned(deps.as_mut().storage, env.clone(), action, actor);

        assert!(res.is_ok());

        ADOContract::remove_permission(deps.as_mut().storage, action, actor).unwrap();

        // Test Limited
        let res = contract.is_permissioned(deps.as_mut().storage, env.clone(), action, actor);

        assert!(res.is_err());
        let permission = Permission::limited(None, 1);
        ADOContract::set_permission(deps.as_mut().storage, action, actor, permission.clone())
            .unwrap();

        let res = contract.is_permissioned(deps.as_mut().storage, env.clone(), action, actor);

        assert!(res.is_ok());

        // Ensure use is consumed
        let res = contract.is_permissioned(deps.as_mut().storage, env.clone(), action, actor);
        assert!(res.is_err());

        ADOContract::remove_permission(deps.as_mut().storage, action, actor).unwrap();

        // Test Blacklisted
        let permission = Permission::blacklisted(None);
        ADOContract::set_permission(deps.as_mut().storage, action, actor, permission.clone())
            .unwrap();

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

        ADOContract::permission_action(action, deps.as_mut().storage).unwrap();

        // Test Blacklisted
        let permission = Permission::blacklisted(None);
        ADOContract::set_permission(deps.as_mut().storage, action, actor, permission.clone())
            .unwrap();

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
        ADOContract::set_permission(deps.as_mut().storage, action, actor, permission.clone())
            .unwrap();

        let res =
            contract.is_permissioned_strict(deps.as_mut().storage, env.clone(), action, actor);
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
            .save(deps.as_mut().storage, &Addr::unchecked(actor.clone()))
            .unwrap();

        let res =
            contract.is_permissioned_strict(deps.as_mut().storage, env.clone(), action, actor);
        assert!(res.is_ok());

        let res = contract.is_permissioned(deps.as_mut().storage, env.clone(), action, actor);
        assert!(res.is_ok());
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

        ADOContract::permission_action(action, deps.as_mut().storage).unwrap();

        let res = contract.is_permissioned(deps.as_mut().storage, env.clone(), action, actor);

        assert!(res.is_err());

        // Test Whitelist
        let permission = Permission::Whitelisted(Some(expiration));
        ADOContract::set_permission(deps.as_mut().storage, action, actor, permission.clone())
            .unwrap();

        let res = contract.is_permissioned(deps.as_mut().storage, env.clone(), action, actor);
        assert!(res.is_ok());

        env.block.height = block + 1;

        let res = contract.is_permissioned(deps.as_mut().storage, env.clone(), action, actor);
        assert!(res.is_err());

        env.block.height = 0;
        // Test Blacklist
        let permission = Permission::Blacklisted(Some(expiration));
        ADOContract::set_permission(deps.as_mut().storage, action, actor, permission.clone())
            .unwrap();

        let res = contract.is_permissioned(deps.as_mut().storage, env.clone(), action, actor);
        assert!(res.is_err());

        env.block.height = block + 1;

        let res = contract.is_permissioned(deps.as_mut().storage, env.clone(), action, actor);
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
            action.clone()
        )
        .unwrap());

        let context = ExecuteContext::new(deps.as_mut(), info.clone(), env.clone());
        ADOContract::permission_action(action, context.deps.storage).unwrap();

        assert!(!is_context_permissioned(
            context.deps.storage,
            &context.info,
            &context.env,
            &context.amp_ctx,
            action.clone()
        )
        .unwrap());

        let context = ExecuteContext::new(deps.as_mut(), info.clone(), env.clone());
        let permission = Permission::whitelisted(None);
        ADOContract::set_permission(context.deps.storage, action, actor, permission.clone())
            .unwrap();

        assert!(is_context_permissioned(
            context.deps.storage,
            &context.info,
            &context.env,
            &context.amp_ctx,
            action.clone()
        )
        .unwrap());

        let unauth_info = mock_info("mock_actor", &[]);
        let context = ExecuteContext::new(deps.as_mut(), unauth_info.clone(), env.clone());

        assert!(!is_context_permissioned(
            context.deps.storage,
            &context.info,
            &context.env,
            &context.amp_ctx,
            action.clone()
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
            action.clone()
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
            action.clone()
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
            action.clone()
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
            action.clone()
        )
        .unwrap());

        let amp_ctx = AMPPkt::new("mock_actor", "owner", vec![]);
        let context =
            ExecuteContext::new(deps.as_mut(), unauth_info.clone(), env.clone()).with_ctx(amp_ctx);

        assert!(is_context_permissioned(
            context.deps.storage,
            &context.info,
            &context.env,
            &context.amp_ctx,
            action.clone()
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
            action.clone()
        )
        .unwrap());

        let context = ExecuteContext::new(deps.as_mut(), info.clone(), env.clone());
        let permission = Permission::whitelisted(None);
        ADOContract::set_permission(context.deps.storage, action, actor, permission.clone())
            .unwrap();

        assert!(is_context_permissioned_strict(
            context.deps.storage,
            &context.info,
            &context.env,
            &context.amp_ctx,
            action.clone()
        )
        .unwrap());

        let unauth_info = mock_info("mock_actor", &[]);
        let context = ExecuteContext::new(deps.as_mut(), unauth_info.clone(), env.clone());

        assert!(!is_context_permissioned_strict(
            context.deps.storage,
            &context.info,
            &context.env,
            &context.amp_ctx,
            action.clone()
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
            action.clone()
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
            action.clone()
        )
        .unwrap());

        let amp_ctx = AMPPkt::new("mock_actor", "mock_actor", vec![]);
        let context =
            ExecuteContext::new(deps.as_mut(), unauth_info.clone(), env.clone()).with_ctx(amp_ctx);

        assert!(!is_context_permissioned_strict(
            context.deps.storage,
            &context.info,
            &context.env,
            &context.amp_ctx,
            action.clone()
        )
        .unwrap());
    }
}
