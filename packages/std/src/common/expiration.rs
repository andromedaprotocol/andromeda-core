use cosmwasm_std::{ensure, BlockInfo, Env, Timestamp};
use cw_utils::Expiration;

use crate::error::ContractError;

use super::Milliseconds;

pub const MILLISECONDS_TO_NANOSECONDS_RATIO: u64 = 1_000_000;

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
    start_time: Option<Milliseconds>,
) -> Result<(Expiration, Milliseconds), ContractError> {
    let current_time = Milliseconds::from_nanos(env.block.time.nanos()).milliseconds();

    let start_expiration = if let Some(start_time) = start_time {
        expiration_from_milliseconds(start_time)?
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
}
