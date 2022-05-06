use common::{ado_base::recipient::Recipient, withdraw::WithdrawalType};
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
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
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
    Stake { amount: Option<Uint128> },
    /// Unstakes the given amount of tokens, or all if not specified.
    Unstake { amount: Option<Uint128> },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    /// Queries the batch with the given id.
    Batch { id: String },
    /// Queries the batches with pagination.
    Batches {
        start_after: Option<String>,
        limit: Option<u32>,
    },
    /// Gets the claimable amount for a batch.
    ClaimableAmount { batch_id: String },
}
