use cosmwasm_std::{Env, Response, Storage};
use cw_storage_plus::{Index, IndexList, IndexedMap, MultiIndex};
use cw_utils::Expiration;

use crate::{common::context::ExecuteContext, error::ContractError};

use super::ADOContract;

pub struct PermissionsIndices<'a> {
    /// PK: action + address
    /// Secondary key: address
    pub permissions: MultiIndex<'a, String, Expiration, String>,
}

impl<'a> IndexList<Expiration> for PermissionsIndices<'a> {
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<Expiration>> + '_> {
        let v: Vec<&dyn Index<Expiration>> = vec![&self.permissions];
        Box::new(v.into_iter())
    }
}

pub fn permissions<'a>() -> IndexedMap<'a, &'a str, Expiration, PermissionsIndices<'a>> {
    let indexes = PermissionsIndices {
        permissions: MultiIndex::new(|_pk: &[u8], r| r.to_string(), "permissions", "address"),
    };
    IndexedMap::new("permissions", indexes)
}

impl<'a> ADOContract<'a> {
    /// Determines if the provided address is authorised to perform the given action
    ///
    /// Returns an error if the permission has expired or if no permission exists for a restricted ADO
    pub fn is_permissioned(
        store: &dyn Storage,
        env: Env,
        action: impl Into<String>,
        address: impl Into<String>,
    ) -> Result<(), ContractError> {
        let permission = Self::get_permission(store, action, address)?;
        match permission {
            Some(expiration) => {
                if expiration.is_expired(&env.block) {
                    Err(ContractError::Unauthorized {})
                } else {
                    Ok(())
                }
            }
            None => Ok(()),
        }
    }

    /// Gets the permission for the given action and address
    pub fn get_permission(
        store: &dyn Storage,
        action: impl Into<String>,
        address: impl Into<String>,
    ) -> Result<Option<Expiration>, ContractError> {
        let action = action.into();
        let address = address.into();
        let key = action + &address;
        Ok(permissions().may_load(store, &key)?)
    }

    /// Sets the permission for the given action and address
    pub fn set_permission(
        store: &mut dyn Storage,
        action: impl Into<String>,
        address: impl Into<String>,
        permission: Expiration,
    ) -> Result<(), ContractError> {
        let action = action.into();
        let address = address.into();
        let key = action + &address;
        permissions().save(store, &key, &permission)?;
        Ok(())
    }

    /// Removes the permission for the given action and address
    pub fn remove_permission(
        store: &mut dyn Storage,
        action: impl Into<String>,
        address: impl Into<String>,
    ) -> Result<(), ContractError> {
        let action = action.into();
        let address = address.into();
        let key = action + &address;
        permissions().remove(store, &key)?;
        Ok(())
    }

    /// Execute handler for setting permission
    /// TODO: Add permission for execute context
    pub fn execute_set_permission(
        &self,
        ctx: ExecuteContext,
        address: impl Into<String>,
        action: impl Into<String>,
        expiration: Option<Expiration>,
    ) -> Result<Response, ContractError> {
        Self::is_contract_owner(&self, ctx.deps.storage, ctx.info.sender.as_str())?;
        let address = address.into();
        let action = action.into();
        let expiration = expiration.unwrap_or_default();
        Self::set_permission(
            ctx.deps.storage,
            action.clone(),
            address.clone(),
            expiration,
        )?;

        Ok(Response::default().add_attributes(vec![
            ("action", "set_permission"),
            ("address", address.as_str()),
            ("action", action.as_str()),
            ("expiration", expiration.to_string().as_str()),
        ]))
    }

    /// Execute handler for setting permission
    /// TODO: Add permission for execute context
    pub fn execute_remove_permission(
        &self,
        ctx: ExecuteContext,
        address: impl Into<String>,
        action: impl Into<String>,
    ) -> Result<Response, ContractError> {
        Self::is_contract_owner(&self, ctx.deps.storage, ctx.info.sender.as_str())?;
        let address = address.into();
        let action = action.into();
        Self::remove_permission(ctx.deps.storage, action.clone(), address.clone())?;

        Ok(Response::default().add_attributes(vec![
            ("action", "remove_permission"),
            ("address", address.as_str()),
            ("action", action.as_str()),
        ]))
    }
}
