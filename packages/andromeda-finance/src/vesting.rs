use andromeda_std::{
    amp::Recipient,
    andr_exec, andr_instantiate, andr_query,
    common::{denom::validate_native_denom, withdraw::WithdrawalType, Milliseconds},
    error::ContractError,
};
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{DepsMut, Uint128};

#[andr_instantiate]
#[cw_serde]
pub struct InstantiateMsg {
    /// The recipient of all funds locked in this contract.
    pub recipient: Recipient,
    /// The denom of the coin being vested.
    pub denom: String,
}

#[andr_exec]
#[cw_serde]
pub enum ExecuteMsg {
    /// Claim the number of batches specified starting from the beginning. If not
    /// specified then the max will be claimed.
    #[attrs(restricted, nonpayable)]
    Claim {
        number_of_claims: Option<u64>,
        batch_id: u64,
    },
    /// Claims tokens from all batches using a paginated approach. If `up_to_time`
    /// is specified then it will only claim up to a specific time, otherwise it
    /// it will claim to the most recent release.
    #[attrs(restricted, nonpayable)]
    ClaimAll {
        up_to_time: Option<Milliseconds>,
        limit: Option<u32>,
    },
    /// Creates a new batch
    #[attrs(restricted)]
    CreateBatch {
        /// Specifying None would mean no lock up period and funds start vesting right away.
        lockup_duration: Option<Milliseconds>,
        /// How often releases occur in seconds.
        release_duration: Milliseconds,
        /// Specifies how much is to be released after each `release_duration`. If
        /// it is a percentage, it would be the percentage of the original amount.
        release_amount: WithdrawalType,
    },
}

#[andr_query]
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    /// Queries the config.
    #[returns(Config)]
    Config {},
    /// Queries the batch with the given id.
    #[returns(BatchResponse)]
    Batch { id: u64 },
    /// Queries the batches with pagination.
    #[returns(Vec<BatchResponse>)]
    Batches {
        start_after: Option<u64>,
        limit: Option<u32>,
    },
}

#[cw_serde]
pub struct Config {
    /// The recipient of each batch.
    pub recipient: Recipient,
    /// The denom of the coin being vested.
    pub denom: String,
}

#[cw_serde]
pub struct BatchResponse {
    /// The id.
    pub id: u64,
    /// The amount of tokens in the batch
    pub amount: Uint128,
    /// The amount of tokens that have been claimed.
    pub amount_claimed: Uint128,
    /// The amount of tokens available to claim right now.
    pub amount_available_to_claim: Uint128,
    /// The number of available claims.
    pub number_of_available_claims: Uint128,
    /// When the lockup ends.
    pub lockup_end: Milliseconds,
    /// How often releases occur.
    pub release_duration: Milliseconds,
    /// Specifies how much is to be released after each `release_duration`. If
    /// it is a percentage, it would be the percentage of the original amount.
    pub release_amount: WithdrawalType,
    /// The time at which the last claim took place in seconds.
    pub last_claimed_release_time: Milliseconds,
}

impl InstantiateMsg {
    pub fn validate(&self, deps: &DepsMut) -> Result<(), ContractError> {
        validate_native_denom(deps.as_ref(), self.denom.clone())?;
        self.recipient.validate(&deps.as_ref())
    }
}
