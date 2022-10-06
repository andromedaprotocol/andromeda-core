use cosmwasm_std::QuerierWrapper;

use crate::error::ContractError;

/// Queries contract info for a given address. 
/// If the query errors the assumption is that the address is not a contract, if not then the address must be a contract.
/// 
/// Returns a result containing a boolean as to whether the given address is a contract or not
pub fn is_contract(querier: QuerierWrapper, addr: &String) -> Result<bool, ContractError> {
    match querier.query_wasm_contract_info(addr) {
        Ok(_) => Ok(true),
        Err(_) => Ok(false)
    }
}