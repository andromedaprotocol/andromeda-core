use common::{
    ado_base::{recipient::Recipient, AndromedaMsg, AndromedaQuery},
    withdraw::WithdrawalType,
};
use cosmwasm_std::Uint128;
use cw0::Duration;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
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

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    AndrReceive(AndromedaMsg),
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
        start_after: Option<u64>,
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
        /// Whether or not the funds should be staked.
        stake: bool,
    },
    /// Stakes the given amount of tokens, or all if not specified.
    Stake {
        amount: Option<Uint128>,
        validator: String,
    },
    /// Unstakes the given amount of tokens, or all if not specified.
    Unstake {
        amount: Option<Uint128>,
        validator: String,
    },
    /// Claims all undelegated tokens that are ready to be claimed.
    ClaimUndelegatedTokens {},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    AndrQuery(AndromedaQuery),
    /// Queries the config.
    Config {},
    /// Queries the batch with the given id.
    Batch {
        id: u64,
    },
    /// Queries the batches with pagination.
    Batches {
        start_after: Option<u64>,
        limit: Option<u32>,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
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
