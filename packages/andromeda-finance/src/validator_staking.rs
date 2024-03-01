use andromeda_std::{andr_exec, andr_instantiate, andr_query, error::ContractError};
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, DepsMut, FullDelegation};

#[andr_instantiate]
#[cw_serde]
pub struct InstantiateMsg {
    pub default_validator: Addr,
}

#[andr_exec]
#[cw_serde]
pub enum ExecuteMsg {
    Stake { validator: Option<Addr> },
}

#[andr_query]
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(Option<FullDelegation>)]
    StakedTokens { validator: Option<Addr> },
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
