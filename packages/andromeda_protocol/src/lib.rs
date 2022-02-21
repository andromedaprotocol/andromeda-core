use crate::error::ContractError;

pub mod address_list;
pub mod anchor;
pub mod astroport_wrapped_cdp;
pub mod auction;
pub mod common;
pub mod communication;
pub mod cw20;
pub mod error;
pub mod factory;
pub mod mirror_wrapped_cdp;
pub mod modules;
pub mod operators;
pub mod ownership;
pub mod primitive;
pub mod rates;
pub mod receipt;
pub mod response;
pub mod splitter;
pub mod swapper;
pub mod withdraw;

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
/// use andromeda_protocol::error::ContractError;
/// use cosmwasm_std::StdError;
/// use andromeda_protocol::require;
/// require(false, ContractError::Std(StdError::generic_err("Some boolean condition was not met")));
/// ```
pub fn require(precond: bool, err: ContractError) -> Result<bool, ContractError> {
    match precond {
        true => Ok(true),
        false => Err(err),
    }
}
