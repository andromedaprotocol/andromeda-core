use andromeda_std::{andr_instantiate, error::ContractError};
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, DepsMut};

#[andr_instantiate]
#[cw_serde]
pub struct InstantiateMsg {
    pub default_validator: Addr,
}

impl InstantiateMsg {
    pub fn validate(&self, deps: &DepsMut) -> Result<bool, ContractError> {
        is_validator(deps, &self.default_validator)
    }
}

pub fn is_validator(deps: &DepsMut, validator: &Addr) -> Result<bool, ContractError> {
    let validator = deps.querier.query_validator(validator)?;
    if validator.is_none() {
        return Err(ContractError::InvalidValidator {});
    }
    Ok(true)
}
