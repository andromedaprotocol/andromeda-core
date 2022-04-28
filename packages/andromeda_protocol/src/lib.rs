use cosmwasm_std::{StdError, StdResult};

pub mod address_list;
pub mod factory;
pub mod modules;
pub mod ownership;
pub mod receipt;
pub mod response;
pub mod splitter;

#[cfg(not(target_arch = "wasm32"))]
pub mod testing;

pub mod timelock;
pub mod token;

/// A simple implementation of Solidity's "require" function. Takes a precondition and an error to return if the precondition is not met.
///
/// ## Arguments
///
/// * `precond` - The required precondition, will return provided "err" parameter if precondition is false
/// * `err` - The error to return if the required precondition is false
///
/// ## Example
/// ```
/// use cosmwasm_std::StdError;
/// use andromeda_protocol::require;
/// require(false, StdError::generic_err("Some boolean condition was not met"));
/// ```
pub fn require(precond: bool, err: StdError) -> StdResult<bool> {
    match precond {
        true => Ok(true),
        false => Err(err),
    }
}
