use crate::amp::AndrAddr;
use crate::error::ContractError;
use cosmwasm_std::DepsMut;

/// Retrieves the code ID for a given contract address.
///
/// This function verifies that the provided address is a valid smart contract by querying
/// its contract info. If the address is not a contract, it returns an error.
///
/// # Arguments
///
/// * `deps` - A reference to the contract's dependencies, used for querying contract info
/// * `recipient` - The address to check and get the code ID for
///
/// # Returns
///
/// * `Result<u64, ContractError>` - The code ID if successful, or a ContractError if:
///   * The address is not a contract
///   * The query fails
pub fn get_code_id(deps: &DepsMut, recipient: &AndrAddr) -> Result<u64, ContractError> {
    deps.querier
        .query_wasm_contract_info(recipient.get_raw_address(&deps.as_ref())?)
        .ok()
        .ok_or(ContractError::InvalidPacket {
            error: Some("Recipient is not a contract".to_string()),
        })
        .map(|info| info.code_id)
}
