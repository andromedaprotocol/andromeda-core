use cosmwasm_schema::cw_serde;
use cosmwasm_std::{ensure, Env, Response, StdError, Storage};
use cw_storage_plus::{Index, IndexList, IndexedMap, Map, MultiIndex};
use cw_utils::Expiration;

use crate::{common::context::ExecuteContext, error::ContractError};

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
                if !permissioned_action {
                    Ok(())
                } else {
                    Err(ContractError::Unauthorized {})
                }
            }
        }
    }

    /// Determines if the provided identifier is authorised to perform the given action
    ///
    /// **Ignores the `PERMISSIONED_ACTIONS` map**
    ///
    /// Returns an error if the permission has expired or if no permission exists for a restricted ADO
    pub fn is_permissioned_strict(
        store: &dyn Storage,
        env: Env,
        action: impl Into<String>,
        identifier: impl Into<String>,
    ) -> Result<(), ContractError> {
        let permission = Self::get_permission(store, action, identifier)?;
        match permission {
            Some(permission) => {
                ensure!(
                    permission.is_permissioned(&env, true),
                    ContractError::Unauthorized {}
                );
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

#[cfg(test)]
mod tests {
    use cosmwasm_std::testing::{mock_dependencies, mock_env};

    use super::*;

    #[test]
    fn test_permissioned_action() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let action = "action";
        let actor = "actor";

        ADOContract::permission_action(action, deps.as_mut().storage).unwrap();

        // Test Whitelisting
        let res = ADOContract::is_permissioned(deps.as_mut().storage, env.clone(), action, actor);

        assert!(res.is_err());
        let permission = Permission::whitelisted(None);
        ADOContract::set_permission(deps.as_mut().storage, action, actor, permission.clone())
            .unwrap();

        let res = ADOContract::is_permissioned(deps.as_mut().storage, env.clone(), action, actor);

        assert!(res.is_ok());

        ADOContract::remove_permission(deps.as_mut().storage, action, actor).unwrap();

        // Test Limited
        let res = ADOContract::is_permissioned(deps.as_mut().storage, env.clone(), action, actor);

        assert!(res.is_err());
        let permission = Permission::limited(None, 1);
        ADOContract::set_permission(deps.as_mut().storage, action, actor, permission.clone())
            .unwrap();

        let res = ADOContract::is_permissioned(deps.as_mut().storage, env.clone(), action, actor);

        assert!(res.is_ok());

        // Ensure use is consumed
        let res = ADOContract::is_permissioned(deps.as_mut().storage, env.clone(), action, actor);
        assert!(res.is_err());

        ADOContract::remove_permission(deps.as_mut().storage, action, actor).unwrap();

        // Test Blacklisted
        let permission = Permission::blacklisted(None);
        ADOContract::set_permission(deps.as_mut().storage, action, actor, permission.clone())
            .unwrap();

        let res = ADOContract::is_permissioned(deps.as_mut().storage, env, action, actor);

        assert!(res.is_err());
    }

    #[test]
    fn test_unpermissioned_action_blacklisted() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let action = "action";
        let actor = "actor";

        ADOContract::permission_action(action, deps.as_mut().storage).unwrap();

        // Test Blacklisted
        let permission = Permission::blacklisted(None);
        ADOContract::set_permission(deps.as_mut().storage, action, actor, permission.clone())
            .unwrap();

        let res = ADOContract::is_permissioned(deps.as_mut().storage, env, action, actor);

        assert!(res.is_err());
    }

    #[test]
    fn test_strict_permissioning() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let action = "action";
        let actor = "actor";

        let res =
            ADOContract::is_permissioned_strict(deps.as_mut().storage, env.clone(), action, actor);
        assert!(res.is_err());

        let permission = Permission::whitelisted(None);
        ADOContract::set_permission(deps.as_mut().storage, action, actor, permission.clone())
            .unwrap();

        let res =
            ADOContract::is_permissioned_strict(deps.as_mut().storage, env.clone(), action, actor);
        assert!(res.is_ok());
    }
}
