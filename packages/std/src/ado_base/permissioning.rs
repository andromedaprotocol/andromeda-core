use core::fmt;

use cosmwasm_schema::cw_serde;
use cosmwasm_std::Env;

use crate::{
    amp::AndrAddr,
    common::{expiration::Expiry, MillisecondsExpiration},
    error::ContractError,
};

#[cw_serde]
pub enum PermissioningMessage {
    SetPermission {
        actors: Vec<AndrAddr>,
        action: String,
        permission: Permission,
    },
    RemovePermission {
        action: String,
        actors: Vec<AndrAddr>,
    },
    PermissionAction {
        action: String,
    },
    DisableActionPermissioning {
        action: String,
    },
}

#[cw_serde]
pub struct PermissionInfo {
    pub permission: Permission,
    pub action: String,
    pub actor: String,
}

#[cw_serde]
pub struct PermissionedActionsResponse {
    pub actions: Vec<String>,
}

/// An enum to represent a user's permission for an action
///
/// - **Blacklisted** - The user cannot perform the action until after the provided expiration
/// - **Limited** - The user can perform the action while uses are remaining and before the provided expiration **for a permissioned action**
/// - **Whitelisted** - The user can perform the action until the provided expiration **for a permissioned action**
///
/// Expiration defaults to `Never` if not provided
#[cw_serde]
pub enum LocalPermission {
    Blacklisted(Option<Expiry>),
    Limited {
        expiration: Option<Expiry>,
        uses: u32,
    },
    Whitelisted(Option<Expiry>),
}

impl std::default::Default for LocalPermission {
    fn default() -> Self {
        Self::Whitelisted(None)
    }
}

impl LocalPermission {
    pub fn blacklisted(expiration: Option<Expiry>) -> Self {
        Self::Blacklisted(expiration)
    }

    pub fn whitelisted(expiration: Option<Expiry>) -> Self {
        Self::Whitelisted(expiration)
    }

    pub fn limited(expiration: Option<Expiry>, uses: u32) -> Self {
        Self::Limited { expiration, uses }
    }

    pub fn is_permissioned(&self, env: &Env, strict: bool) -> bool {
        match self {
            Self::Blacklisted(expiration) => {
                if let Some(expiration) = expiration {
                    if expiration.get_time(&env.block).is_expired(&env.block) {
                        return true;
                    }
                }
                false
            }
            Self::Limited { expiration, uses } => {
                if let Some(expiration) = expiration {
                    if expiration.get_time(&env.block).is_expired(&env.block) {
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
                    if expiration.get_time(&env.block).is_expired(&env.block) {
                        return !strict;
                    }
                }
                true
            }
        }
    }

    pub fn get_expiration(&self, env: Env) -> MillisecondsExpiration {
        match self {
            Self::Blacklisted(expiration) => {
                expiration.clone().unwrap_or_default().get_time(&env.block)
            }
            Self::Limited { expiration, .. } => {
                expiration.clone().unwrap_or_default().get_time(&env.block)
            }
            Self::Whitelisted(expiration) => {
                expiration.clone().unwrap_or_default().get_time(&env.block)
            }
        }
    }

    pub fn consume_use(&mut self) -> Result<(), ContractError> {
        if let Self::Limited { uses, .. } = self {
            if let Some(remaining_uses) = uses.checked_sub(1) {
                *uses = remaining_uses;
                Ok(())
            } else {
                Err(ContractError::Underflow {})
            }
        } else {
            Ok(())
        }
    }
}

impl fmt::Display for LocalPermission {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let self_as_string = match self {
            Self::Blacklisted(expiration) => {
                if let Some(expiration) = expiration {
                    format!("blacklisted:{expiration}")
                } else {
                    "blacklisted".to_string()
                }
            }
            Self::Limited { expiration, uses } => {
                if let Some(expiration) = expiration {
                    format!("limited:{expiration}:{uses}")
                } else {
                    format!("limited:{uses}")
                }
            }
            Self::Whitelisted(expiration) => {
                if let Some(expiration) = expiration {
                    format!("whitelisted:{expiration}")
                } else {
                    "whitelisted".to_string()
                }
            }
        };
        write!(f, "{self_as_string}")
    }
}

#[cw_serde]
pub enum Permission {
    Local(LocalPermission),
    Contract(AndrAddr),
}

impl fmt::Display for Permission {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let self_as_string = match self {
            Self::Local(local_permission) => local_permission.to_string(),
            Self::Contract(address_list) => address_list.to_string(),
        };
        write!(f, "{self_as_string}")
    }
}
