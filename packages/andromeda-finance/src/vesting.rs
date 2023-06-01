use andromeda_std::{
    amp::Recipient, andr_exec, andr_instantiate, andr_query, common::withdraw::WithdrawalType,
};
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Uint128, VoteOption};
use cw_utils::Duration;

#[andr_instantiate]
#[cw_serde]
pub struct InstantiateMsg {
    /// The recipient of all funds locked in this contract.
    pub recipient: Recipient,
    /// Whether or not multi-batching has been enabled.
    pub is_multi_batch_enabled: bool,
    /// The denom of the coin being vested.
    pub denom: String,
    /// The unbonding duration of the native staking module.
    pub unbonding_duration: Duration,
}

#[andr_exec]
#[cw_serde]
pub enum ExecuteMsg {
    /// Claim the number of batches specified starting from the beginning. If not
    /// specified then the max will be claimed.
    Claim {
        number_of_claims: Option<u64>,
        batch_id: u64,
    },
    /// Claims tokens from all batches using a paginated approach. If `up_to_time`
    /// is specified then it will only claim up to a specific time, otherwise it
    /// it will claim to the most recent release.
    ClaimAll {
        up_to_time: Option<u64>,
        limit: Option<u32>,
    },
    /// Creates a new batch
    CreateBatch {
        /// Specifying None would mean no lock up period and funds start vesting right away.
        lockup_duration: Option<u64>,
        /// How often releases occur in seconds.
        release_unit: u64,
        /// Specifies how much is to be released after each `release_unit`. If
        /// it is a percentage, it would be the percentage of the original amount.
        release_amount: WithdrawalType,
        /// The validator to delegate to. If specified, funds will be delegated to it.
        validator_to_delegate_to: Option<String>,
    },
    /// Delegates the given amount of tokens, or all if not specified.
    Delegate {
        amount: Option<Uint128>,
        validator: String,
    },
    /// Redelegates the given amount of tokens, or all from the `from` validator to the `to`
    /// validator.
    Redelegate {
        amount: Option<Uint128>,
        from: String,
        to: String,
    },
    /// Undelegates the given amount of tokens, or all if not specified.
    Undelegate {
        amount: Option<Uint128>,
        validator: String,
    },
    /// Withdraws rewards from all delegations to the sender.
    WithdrawRewards {},
    /// Votes on the specified proposal with the specified vote.
    Vote { proposal_id: u64, vote: VoteOption },
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
    /// Whether or not multiple batches are supported.
    pub is_multi_batch_enabled: bool,
    /// The denom of the coin being vested.
    pub denom: String,
    /// The unbonding duration of the native staking module.
    pub unbonding_duration: Duration,
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
    pub lockup_end: u64,
    /// How often releases occur.
    pub release_unit: u64,
    /// Specifies how much is to be released after each `release_unit`. If
    /// it is a percentage, it would be the percentage of the original amount.
    pub release_amount: WithdrawalType,
    /// The time at which the last claim took place in seconds.
    pub last_claimed_release_time: u64,
}

#[cw_serde]
#[serde(rename_all = "snake_case")]
pub struct MigrateMsg {}
