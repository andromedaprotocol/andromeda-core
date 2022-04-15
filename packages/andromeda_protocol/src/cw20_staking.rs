use common::{
    ado_base::{AndromedaMsg, AndromedaQuery},
    error::ContractError,
    mission::AndrAddress,
};
use cosmwasm_std::{Api, Decimal, Uint128};
use cw20::Cw20ReceiveMsg;
use cw_asset::{AssetInfo, AssetInfoUnchecked};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, PartialEq)]
pub struct InstantiateMsg {
    /// The cw20 token that can be staked.
    pub staking_token: AndrAddress,
    /// Any rewards in addition to the staking token. This list cannot include the staking token.
    pub additional_rewards: Option<Vec<RewardTokenUnchecked>>,
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    Receive(Cw20ReceiveMsg),
    AndrReceive(AndromedaMsg),
    /// Add `reward_token` as another reward token. Owner only.
    AddRewardToken {
        reward_token: RewardTokenUnchecked,
    },
    /// Unstakes the specified amount of assets, or all if not specified. The user's pending
    /// rewards and indexes are updated for each additional reward token.
    UnstakeTokens {
        amount: Option<Uint128>,
    },
    /// Claims any outstanding rewards from the addtional reward tokens.
    ClaimRewards {},
    /// Updates the global reward index for the specified reward tokens or all of the specified ones if
    /// None. Funds may be sent along with this. Can only be done for non-allocated reward tokens.
    UpdateGlobalIndexes {
        asset_infos: Option<Vec<AssetInfoUnchecked>>,
    },
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Cw20HookMsg {
    /// Stake the sent tokens. Address must match the `staking_token` given on instantiation. The user's pending
    /// rewards and indexes are updated for each additional reward token.
    StakeTokens {},
    /// Updates the global reward index on deposit of a valid cw20 token.
    UpdateGlobalIndex {},
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    AndrQuery(AndromedaQuery),
    /// Gets the config of the contract.
    Config {},
    /// Gets the state of the contract.
    State {},
    /// Returns a `StakerResponse` for the given staker. The pending rewards are updated to the
    /// present index.
    Staker {
        address: String,
    },
    /// Returns a `Vec<StakerResponse>` for range of stakers. The pending rewards are updated to the
    /// present index for each staker.
    Stakers {
        start_after: Option<String>,
        limit: Option<u32>,
    },
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, PartialEq)]
pub struct RewardTokenUnchecked {
    pub asset_info: AssetInfoUnchecked,
    pub allocation_info: Option<AllocationInfo>,
}

impl RewardTokenUnchecked {
    pub fn check(self, api: &dyn Api) -> Result<RewardToken, ContractError> {
        let checked_asset_info = self.asset_info.check(api, None)?;
        Ok(RewardToken {
            asset_info: checked_asset_info,
            allocation_info: self.allocation_info,
        })
    }
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, PartialEq)]
pub struct RewardToken {
    pub asset_info: AssetInfo,
    pub allocation_info: Option<AllocationInfo>,
}

impl fmt::Display for RewardToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.asset_info.fmt(f)
    }
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, PartialEq)]
pub struct AllocationInfo {
    pub config: AllocationConfig,
    pub state: AllocationState,
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, PartialEq)]
pub struct AllocationConfig {
    /// Timestamp from which Rewards will start getting accrued against the staked LP tokens
    pub init_timestamp: u64,
    /// Timestamp till which Rewards will be accrued. No staking rewards are accrued beyond this timestamp
    pub till_timestamp: u64,
    /// Rewards distributed during the 1st cycle.
    pub cycle_rewards: Uint128,
    /// Cycle duration in timestamps
    pub cycle_duration: u64,
    /// Percent increase in Rewards per cycle
    pub reward_increase: Option<Decimal>,
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, PartialEq)]
pub struct AllocationState {
    /// Keeps track of the distribution cycle
    pub current_cycle: u64,
    /// Number of tokens to be distributed during the current cycle
    pub current_cycle_rewards: Uint128,
    /// Timestamp at which the global_reward_index was last updated
    pub last_distributed: u64,
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, PartialEq)]
pub struct StakerResponse {
    pub address: String,
    pub share: Uint128,
    pub pending_rewards: Vec<(String, Uint128)>,
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum MigrateMsg {}
