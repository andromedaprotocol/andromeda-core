use cosmwasm_std::QuerierWrapper;

use crate::error::ContractError;

pub fn is_contract(querier: QuerierWrapper, addr: &String) -> Result<bool, ContractError> {
    match querier.query_wasm_contract_info(addr) {
        Ok(_) => Ok(true),
        Err(_) => Ok(false)
    }
}