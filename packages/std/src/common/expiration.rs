use cosmwasm_std::{ensure, Timestamp};
use cw_utils::Expiration;

use crate::error::ContractError;

pub const MILLISECONDS_TO_NANOSECONDS_RATIO: u64 = 1_000_000;

/// Creates a CosmWasm Expiration struct given a time in milliseconds
/// # Arguments
///
/// * `time` - The expiration time in milliseconds since the Epoch
///
/// Returns a `cw_utils::Expiration::AtTime` struct with the given expiration time
pub fn expiration_from_milliseconds(time: u64) -> Result<Expiration, ContractError> {
    // Make sure that multiplying by above ratio does not exceed u64 limit
    ensure!(
        time <= u64::MAX / MILLISECONDS_TO_NANOSECONDS_RATIO,
        ContractError::InvalidExpirationTime {}
    );

    Ok(Expiration::AtTime(Timestamp::from_nanos(
        time * MILLISECONDS_TO_NANOSECONDS_RATIO,
    )))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_expiration_from_milliseconds() {
        let time = u64::MAX;
        let result = expiration_from_milliseconds(time).unwrap_err();
        assert_eq!(result, ContractError::InvalidExpirationTime {});

        let valid_time = 100;
        let result = expiration_from_milliseconds(valid_time).unwrap();
        assert_eq!(
            Expiration::AtTime(Timestamp::from_nanos(100000000u64)),
            result
        )
    }
}
