use cosmwasm_std::{StdError, StdResult};

pub fn require(precond: bool, err: StdError) -> StdResult<bool> {
    match precond {
        true => Ok(true),
        false => Err(err),
    }
}
