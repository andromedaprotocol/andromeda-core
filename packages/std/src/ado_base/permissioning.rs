use core::fmt;

use cosmwasm_schema::cw_serde;
use cosmwasm_std::Env;
use cw_utils::Expiration;

#[cw_serde]
pub struct PermissionInfo {
    pub permission: Permission,
    pub action: String,
    pub actor: String,
}

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

impl std::default::Default for Permission {
    fn default() -> Self {
        Self::Whitelisted(None)
    }
}

impl Permission {
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

    pub fn consume_use(&mut self) {
        if let Self::Limited { uses, .. } = self {
            *uses -= 1
        }
    }
}

impl fmt::Display for Permission {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let self_as_string = match self {
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
        };
        write!(f, "{}", self_as_string)
    }
}
