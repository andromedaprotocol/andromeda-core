use andromeda_std::{amp::AndrAddr, andr_exec, andr_instantiate, andr_query, error::ContractError};
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Coin, DepsMut, Timestamp, Uint128};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[andr_instantiate]
#[cw_serde]
pub struct InstantiateMsg {
    pub default_validator: Addr,
}

#[andr_exec]
#[cw_serde]
pub enum ExecuteMsg {
    Stake {
        validator: Option<Addr>,
    },
    Unstake {
        validator: Option<Addr>,
        amount: Option<Uint128>,
    },
    Claim {
        validator: Option<Addr>,
        recipient: Option<AndrAddr>,
    },
    WithdrawFunds {
        denom: Option<String>,
        recipient: Option<AndrAddr>,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct UnstakingTokens {
    pub fund: Coin,
    pub payout_at: Timestamp,
}

#[andr_query]
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(Option<::cosmwasm_std::FullDelegation>)]
    StakedTokens { validator: Option<Addr> },

    #[returns(Option<Vec<UnstakingTokens>>)]
    UnstakedTokens {},
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
