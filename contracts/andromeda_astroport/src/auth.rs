use andromeda_protocol::{
    error::ContractError, operators::is_operator, ownership::is_contract_owner, require,
};
use cosmwasm_std::Storage;

pub fn require_is_authorized(storage: &dyn Storage, sender: &str) -> Result<(), ContractError> {
    require(
        is_contract_owner(storage, sender)? || is_operator(storage, sender)?,
        ContractError::Unauthorized {},
    )?;
    Ok(())
}
