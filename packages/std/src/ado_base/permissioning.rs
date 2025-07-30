use core::fmt;

use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Deps, Env};

use crate::{
    amp::AndrAddr,
    common::{expiration::Expiry, schedule::Schedule, Milliseconds, MillisecondsExpiration},
    error::ContractError,
    os::aos_querier::AOSQuerier,
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
        expiration: Option<Expiry>,
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

#[cw_serde]
pub struct PermissionedActorsResponse {
    pub actors: Vec<String>,
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
    Blacklisted {
        #[serde(default)]
        schedule: Schedule,
    },
    Limited {
        #[serde(default)]
        schedule: Schedule,
        uses: u32,
    },
    Whitelisted {
        #[serde(default)]
        schedule: Schedule,
        window: Option<Milliseconds>,
        last_used: Option<Milliseconds>,
    },
}

impl std::default::Default for LocalPermission {
    fn default() -> Self {
        Self::Whitelisted {
            schedule: Schedule::new(None, None),
            window: None,
            last_used: None,
        }
    }
}

impl LocalPermission {
    pub fn blacklisted(schedule: Schedule) -> Self {
        Self::Blacklisted { schedule }
    }

    pub fn whitelisted(
        schedule: Schedule,
        window: Option<Milliseconds>,
        last_used: Option<Milliseconds>,
    ) -> Self {
        Self::Whitelisted {
            schedule,
            window,
            last_used,
        }
    }

    pub fn limited(schedule: Schedule, uses: u32) -> Self {
        Self::Limited { schedule, uses }
    }

    pub fn is_permissioned(&self, env: &Env, strict: bool) -> bool {
        match self {
            Self::Blacklisted { schedule } => {
                // If start time hasn't started yet, then it should return true
                if let Some(start) = &schedule.start {
                    if !start.get_time(&env.block).is_expired(&env.block) {
                        return true;
                    }
                }
                if let Some(expiration) = &schedule.end {
                    if expiration.get_time(&env.block).is_expired(&env.block) {
                        return !strict;
                    }
                }
                false
            }
            Self::Limited { schedule, uses } => {
                if let Some(start) = &schedule.start {
                    if !start.get_time(&env.block).is_expired(&env.block) {
                        return true;
                    }
                }
                if let Some(expiration) = &schedule.end {
                    if expiration.get_time(&env.block).is_expired(&env.block) {
                        return !strict;
                    }
                }
                if *uses == 0 {
                    return !strict;
                }
                true
            }
            Self::Whitelisted {
                schedule,
                window,
                last_used,
            } => {
                let start = &schedule.start;
                let expiration = &schedule.end;
                if let Some(start) = start {
                    if !start.get_time(&env.block).is_expired(&env.block) {
                        return !strict;
                    }
                }
                if let Some(expiration) = expiration {
                    if expiration.get_time(&env.block).is_expired(&env.block) {
                        return !strict;
                    }
                }
                if let Some(window) = window {
                    // Check if last used is set
                    if let Some(last_used) = last_used {
                        // Get current time
                        let current_time = env.block.time.seconds();
                        let time_elapsed_since_last_use = current_time - last_used.seconds();

                        if time_elapsed_since_last_use < window.seconds() {
                            return !strict;
                        }
                    }
                }
                true
            }
        }
    }

    pub fn get_expiration(&self, env: Env) -> MillisecondsExpiration {
        match self {
            Self::Blacklisted { schedule } => schedule
                .end
                .clone()
                .unwrap_or_default()
                .get_time(&env.block),
            Self::Limited { schedule, .. } => schedule
                .end
                .clone()
                .unwrap_or_default()
                .get_time(&env.block),
            Self::Whitelisted { schedule, .. } => schedule
                .end
                .clone()
                .unwrap_or_default()
                .get_time(&env.block),
        }
    }

    pub fn get_start_time(&self, env: Env) -> MillisecondsExpiration {
        match self {
            Self::Blacklisted { schedule } => schedule
                .start
                .clone()
                .unwrap_or_default()
                .get_time(&env.block),
            Self::Limited { schedule, .. } => schedule
                .start
                .clone()
                .unwrap_or_default()
                .get_time(&env.block),
            Self::Whitelisted { schedule, .. } => schedule
                .start
                .clone()
                .unwrap_or_default()
                .get_time(&env.block),
        }
    }

    pub fn consume_use(&mut self) -> Result<(), ContractError> {
        if let Self::Limited { uses, .. } = self {
            *uses = uses.saturating_sub(1);
        }

        Ok(())
    }

    pub fn validate_times(
        &self,
        env: &Env,
    ) -> Result<(Milliseconds, Option<Milliseconds>), ContractError> {
        let (start, expiration) = match self {
            Self::Blacklisted { schedule } => (schedule.start.clone(), schedule.end.clone()),
            Self::Limited { schedule, .. } => (schedule.start.clone(), schedule.end.clone()),
            Self::Whitelisted { schedule, .. } => (schedule.start.clone(), schedule.end.clone()),
        };

        let start = start
            .map(|s| s.get_time(&env.block))
            .unwrap_or(Milliseconds::from_nanos(env.block.time.nanos())); // Defaults to current time
        let end = expiration.map(|e| e.get_time(&env.block));

        // Check if start time is after current time
        if start.is_in_past(&env.block) {
            return Err(ContractError::StartTimeInThePast {
                current_time: env.block.time.seconds(),
                current_block: env.block.height,
            });
        }

        if let Some(end) = end {
            // Check if expiration time is after current time
            if end.is_expired(&env.block) {
                return Err(ContractError::ExpirationInPast {});
            }
            // Check if start time is before expiration time
            if start > end {
                return Err(ContractError::StartTimeAfterEndTime {});
            }
        }

        Ok((start, end))
    }
}

impl fmt::Display for LocalPermission {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let self_as_string = match self {
            Self::Blacklisted { schedule } => {
                match (schedule.start.clone(), schedule.end.clone()) {
                    (Some(s), Some(e)) => format!("blacklisted starting from:{s} until:{e}"),
                    (Some(s), None) => format!("blacklisted starting from:{s}"),
                    (None, Some(e)) => format!("blacklisted until:{e}"),
                    (None, None) => "blacklisted".to_string(),
                }
            }
            Self::Limited { schedule, uses } => {
                match (schedule.start.clone(), schedule.end.clone()) {
                    (Some(s), Some(e)) => {
                        format!("limited starting from:{s} until:{e} uses:{uses}")
                    }
                    (Some(s), None) => format!("limited starting from:{s} uses:{uses}"),
                    (None, Some(e)) => format!("limited until:{e} uses:{uses}"),
                    (None, None) => format!("limited uses:{uses}"),
                }
            }
            Self::Whitelisted {
                schedule,
                window,
                last_used,
            } => match (
                schedule.start.clone(),
                schedule.end.clone(),
                window,
                last_used,
            ) {
                (Some(s), Some(e), Some(f), Some(l)) => {
                    format!("whitelisted starting from:{s} until:{e} window:{f} last_used:{l}")
                }
                (Some(s), Some(e), Some(f), None) => {
                    format!("whitelisted starting from:{s} until:{e} window:{f}")
                }
                (Some(s), Some(e), None, Some(l)) => {
                    format!("whitelisted starting from:{s} until:{e} last_used:{l}")
                }
                (Some(s), Some(e), None, None) => {
                    format!("whitelisted starting from:{s} until:{e}")
                }
                (Some(s), None, Some(f), Some(l)) => {
                    format!("whitelisted starting from:{s} window:{f} last_used:{l}")
                }
                (Some(s), None, Some(f), None) => {
                    format!("whitelisted starting from:{s} window:{f}")
                }
                (Some(s), None, None, Some(l)) => {
                    format!("whitelisted starting from:{s} last_used:{l}")
                }
                (Some(s), None, None, None) => {
                    format!("whitelisted starting from:{s}")
                }
                (None, Some(e), Some(f), Some(l)) => {
                    format!("whitelisted until:{e} window:{f} last_used:{l}")
                }
                (None, Some(e), Some(f), None) => {
                    format!("whitelisted until:{e} window:{f}")
                }
                (None, Some(e), None, Some(l)) => {
                    format!("whitelisted until:{e} last_used:{l}")
                }
                (None, Some(e), None, None) => {
                    format!("whitelisted until:{e}")
                }
                (None, None, Some(f), Some(l)) => {
                    format!("whitelisted window:{f} last_used:{l}")
                }
                (None, None, Some(f), None) => {
                    format!("whitelisted window:{f}")
                }
                (None, None, None, Some(l)) => {
                    format!("whitelisted last_used:{l}")
                }
                (None, None, None, None) => "whitelisted".to_string(),
            },
        };
        write!(f, "{self_as_string}")
    }
}

#[cw_serde]
pub enum Permission {
    Local(LocalPermission),
    Contract(AndrAddr),
}

impl Permission {
    pub fn get_permission(
        &mut self,
        deps: Deps,
        actor: &str,
    ) -> Result<LocalPermission, ContractError> {
        match self {
            Self::Local(local_permission) => Ok(local_permission.clone()),
            Self::Contract(contract_address) => {
                let addr = contract_address.get_raw_address(&deps)?;
                let local_permission = AOSQuerier::get_permission(&deps.querier, &addr, actor)?;
                Ok(local_permission)
            }
        }
    }
    pub fn validate_times(
        &self,
        env: &Env,
    ) -> Result<(Milliseconds, Option<Milliseconds>), ContractError> {
        match self {
            Self::Local(local_permission) => local_permission.validate_times(env),
            //TODO these are default values
            Self::Contract(_) => Ok((Milliseconds::from_nanos(env.block.time.nanos()), None)),
        }
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::expiration::Expiry;
    use crate::common::Milliseconds;

    use cosmwasm_std::testing::mock_env;
    use rstest::rstest;
    #[rstest]
    #[case::valid_future_times(1000, 2000)] // start in 100s, expire in 200s
    #[case::same_start_and_end(1000, 1000)] // edge case: start and end at same time
    #[case::far_future(10000, 20000)] // times far in the future
    fn test_valid_time_combinations(#[case] start_offset: u64, #[case] exp_offset: u64) {
        let env = mock_env();
        let current_time = env.block.time.seconds();

        let permission = LocalPermission::Whitelisted {
            schedule: Schedule::new(
                Some(Expiry::AtTime(
                    Milliseconds(start_offset).plus_seconds(current_time),
                )),
                Some(Expiry::AtTime(
                    Milliseconds(exp_offset).plus_seconds(current_time),
                )),
            ),
            window: None,
            last_used: None,
        };

        let result = permission.validate_times(&env);
        println!("result: {:?}", result);
        assert!(result.is_ok());
    }

    #[rstest]
    #[case::start_in_past(
        0,  // start 100s in past
        1000,   // expire 100s in future
        ContractError::StartTimeInThePast { current_time: 1571797419, current_block: 12345 }
    )]
    #[case::expiration_in_past(
        1000,   // start 100s in future
        0,  // expire 100s in past
        ContractError::ExpirationInPast {}
    )]
    #[case::start_after_end(
        2000,   // start 200s in future
        1000,   // expire 100s in future
        ContractError::StartTimeAfterEndTime {}
    )]
    fn test_invalid_time_combinations(
        #[case] start_offset: u64,
        #[case] exp_offset: u64,
        #[case] expected_error: ContractError,
    ) {
        let env = mock_env();
        let current_time = env.block.time.seconds();

        let permission = LocalPermission::Whitelisted {
            schedule: Schedule::new(
                Some(Expiry::AtTime(
                    Milliseconds(start_offset).plus_seconds(current_time),
                )),
                Some(Expiry::AtTime(
                    Milliseconds(exp_offset).plus_seconds(current_time),
                )),
            ),
            window: None,
            last_used: None,
        };

        let result = permission.validate_times(&env);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().to_string(), expected_error.to_string());
    }

    #[rstest]
    fn test_no_times_specified() {
        let env = mock_env();

        let permission = LocalPermission::Whitelisted {
            schedule: Schedule::new(None, None),
            window: None,
            last_used: None,
        };

        let result = permission.validate_times(&env);
        assert!(result.is_ok());
    }

    #[rstest]
    fn test_only_start_time() {
        let env = mock_env();
        let current_time = Milliseconds::from_seconds(env.block.time.seconds());

        let permission = LocalPermission::Whitelisted {
            schedule: Schedule::new(
                Some(Expiry::AtTime(
                    current_time.plus_milliseconds(Milliseconds(100000)),
                )),
                None,
            ),
            window: None,
            last_used: None,
        };

        permission.validate_times(&env).unwrap();
    }

    #[rstest]
    fn test_only_expiration_time() {
        let env = mock_env();
        let current_time = Milliseconds::from_seconds(env.block.time.seconds());

        let permission = LocalPermission::Whitelisted {
            schedule: Schedule::new(
                None,
                Some(Expiry::AtTime(
                    current_time.plus_milliseconds(Milliseconds(100000)),
                )),
            ),
            window: None,
            last_used: None,
        };

        permission.validate_times(&env).unwrap();
    }

    #[rstest]
    // Test cases for whitelisted permissions
    #[case::no_times_authorized(
        None, // start
        None, // expiration
        None, // window
        true  // expected authorized
    )]
    #[case::future_start_unauthorized(
        Some(1000), // start (in future)
        None,
        None,
        false
    )]
    #[case::future_start_authorized(
        Some(100), // start (in future)
        None,
        None,
        true
    )]
    #[case::expired_unauthorized(
        None,
        Some(100), // expiration (in past)
        None,
        false
    )]
    #[case::valid_time_window_authorized(
        Some(100), // start (in future)
        Some(2000), // expiration (further in future)
        None,
        true
    )]
    #[case::window_not_met_unauthorized(None, None, Some(1571797419), false)]
    #[case::window_met_authorized(
        None,
        None,
        Some(100), // window (0.1 seconds)
        true
    )]
    fn test_whitelisted_permission(
        #[case] start_offset: Option<u64>,
        #[case] exp_offset: Option<u64>,
        #[case] window_ms: Option<u64>,
        #[case] expected_authorized: bool,
    ) {
        let env = mock_env();
        let current_time = env.block.time.seconds();

        // Create permission with provided parameters
        let permission = LocalPermission::Whitelisted {
            schedule: Schedule::new(
                start_offset
                    .map(|offset| Expiry::AtTime(Milliseconds(offset).plus_seconds(current_time))),
                exp_offset
                    .map(|offset| Expiry::AtTime(Milliseconds(offset).plus_seconds(current_time))),
            ),
            window: window_ms.map(Milliseconds),
            last_used: if window_ms.is_some() {
                Some(Milliseconds::from_seconds(current_time - 200)) // Set last used to 200ms ago
            } else {
                None
            },
        };

        // Test the permission
        let is_authorized = permission.is_permissioned(&env, true);
        assert_eq!(is_authorized, expected_authorized);
    }

    // Test cases for window-based permissions
    #[rstest]
    #[case::window_not_met(
        1000, // window (1 second)
        1571797419,  // time since last use (0.5 seconds)
        false // should not be authorized
    )]
    #[case::window_met(
        1000, // window (1 second)
        1571797419 - 10000000, // time since last use (1.5 seconds)
        true  // should be authorized
    )]
    fn test_window_based_permission(
        #[case] window_ms: u64,
        #[case] last_used: u64,
        #[case] expected_authorized: bool,
    ) {
        let env = mock_env();

        let permission = LocalPermission::Whitelisted {
            schedule: Schedule::new(None, None),
            window: Some(Milliseconds(window_ms)),
            last_used: Some(Milliseconds::from_seconds(last_used)),
        };

        let is_authorized = permission.is_permissioned(&env, true);
        assert_eq!(is_authorized, expected_authorized);
    }

    // Test cases for time window permissions
    #[rstest]
    #[case::before_start_window(
        1000, // start offset (1 second in future)
        2000, // expiration offset (2 seconds in future)
        false // should not be authorized
    )]
    #[case::within_window(
        100,  // start offset (0.1 seconds in future)
        2000, // expiration offset (2 seconds in future)
        true  // should be authorized
    )]
    #[case::after_expiration(
        100,  // start offset (0.1 seconds in future)
        200,  // expiration offset (0.2 seconds in future)
        false // should not be authorized
    )]
    fn test_time_window_permission(
        #[case] start_offset: u64,
        #[case] exp_offset: u64,
        #[case] expected_authorized: bool,
    ) {
        let env = mock_env();
        let current_time = env.block.time.seconds();

        let permission = LocalPermission::Whitelisted {
            schedule: Schedule::new(
                Some(Expiry::AtTime(
                    Milliseconds(start_offset).plus_seconds(current_time),
                )),
                Some(Expiry::AtTime(
                    Milliseconds(exp_offset).plus_seconds(current_time),
                )),
            ),
            window: None,
            last_used: None,
        };

        let is_authorized = permission.is_permissioned(&env, true);
        assert_eq!(is_authorized, expected_authorized);
    }
}
