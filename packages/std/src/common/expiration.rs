use std::fmt::{self, Display};

use cosmwasm_schema::cw_serde;
use cosmwasm_std::{ensure, BlockInfo, Env, Timestamp};
use cw_utils::Expiration;

use crate::error::ContractError;

use super::Milliseconds;

pub const MILLISECONDS_TO_NANOSECONDS_RATIO: u64 = 1_000_000;

/// The Expiry type is used to define an expiry time using milliseconds
///
/// There are two types:
/// 1. FromNow(Milliseconds) - The expiry time is relative to the current time
/// 2. AtTime(Milliseconds) - The expiry time is absolute
#[cw_serde]
pub enum Expiry {
    FromNow(Milliseconds),
    AtTime(Milliseconds),
}

impl Expiry {
    /// Gets the expected expiry time provided the given block
    pub fn get_time(&self, block: &BlockInfo) -> Milliseconds {
        match self {
            Expiry::FromNow(milliseconds) => {
                // Get current time from block
                let current_time = Milliseconds::from_nanos(block.time.nanos());
                // Add the expected expiry time from now
                current_time.plus_milliseconds(*milliseconds)
            }
            // Given time is absolute
            Expiry::AtTime(milliseconds) => *milliseconds,
        }
    }
}

/// Expiry defaults to an absolute time of 0
impl Default for Expiry {
    fn default() -> Self {
        Expiry::AtTime(Milliseconds::default())
    }
}

impl Display for Expiry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Expiry::FromNow(milliseconds) => write!(f, "{} milliseconds from now", milliseconds),
            Expiry::AtTime(milliseconds) => write!(f, "At time: {}", milliseconds),
        }
    }
}

/// Creates a CosmWasm Expiration struct given a time in milliseconds
/// # Arguments
///
/// * `time` - The expiration time in milliseconds since the Epoch
///
/// Returns a `cw_utils::Expiration::AtTime` struct with the given expiration time
pub fn expiration_from_milliseconds(time: Milliseconds) -> Result<Expiration, ContractError> {
    // Make sure that multiplying by above ratio does not exceed u64 limit
    ensure!(
        time.milliseconds() <= u64::MAX / MILLISECONDS_TO_NANOSECONDS_RATIO,
        ContractError::InvalidExpirationTime {}
    );

    Ok(Expiration::AtTime(Timestamp::from_nanos(time.nanos())))
}

pub fn block_to_expiration(block: &BlockInfo, model: Expiration) -> Option<Expiration> {
    match model {
        Expiration::AtTime(_) => Some(Expiration::AtTime(block.time)),
        Expiration::AtHeight(_) => Some(Expiration::AtHeight(block.height)),
        Expiration::Never {} => None,
    }
}

pub fn get_and_validate_start_time(
    env: &Env,
    start_time: Option<Expiry>,
) -> Result<(Expiration, Milliseconds), ContractError> {
    let current_time = Milliseconds::from_nanos(env.block.time.nanos()).milliseconds();

    let start_expiration = if let Some(start_time) = start_time {
        expiration_from_milliseconds(start_time.get_time(&env.block))?
    } else {
        // Set as current time + 1 so that it isn't expired from the very start
        expiration_from_milliseconds(Milliseconds(current_time + 1))?
    };

    // Validate start time
    let block_time = block_to_expiration(&env.block, start_expiration).unwrap();
    ensure!(
        start_expiration.gt(&block_time),
        ContractError::StartTimeInThePast {
            current_time,
            current_block: env.block.height,
        }
    );

    Ok((start_expiration, Milliseconds(current_time)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_expiration_from_milliseconds() {
        let time = u64::MAX;
        let result = expiration_from_milliseconds(Milliseconds(time)).unwrap_err();
        assert_eq!(result, ContractError::InvalidExpirationTime {});

        let valid_time = 100;
        let result = expiration_from_milliseconds(Milliseconds(valid_time)).unwrap();
        assert_eq!(
            Expiration::AtTime(Timestamp::from_nanos(100000000u64)),
            result
        )
    }

    #[test]
    fn test_expiry_from_now() {
        let block = BlockInfo {
            height: 100,
            time: Timestamp::from_nanos(100000000u64),
            chain_id: "test-chain".to_string(),
        };

        let expiry = Expiry::FromNow(Milliseconds(100));
        assert_eq!(expiry.get_time(&block), Milliseconds(200));
    }
}
