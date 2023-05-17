use andromeda_std::amp::addresses::AndrAddr;
use andromeda_std::common::expiration::MILLISECONDS_TO_NANOSECONDS_RATIO;
use andromeda_std::error::ContractError;
use andromeda_std::{andr_exec, andr_instantiate, andr_query};
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{ensure, Api, BlockInfo, Decimal, Decimal256, Uint128};
use cw20::Cw20ReceiveMsg;
use cw_asset::{AssetInfo, AssetInfoUnchecked};
use std::fmt;

#[andr_instantiate]
#[cw_serde]
pub struct InstantiateMsg {
    /// The cw20 token that can be staked.
    pub staking_token: AndrAddr,
    /// Any rewards in addition to the staking token. This list cannot include the staking token.
    pub additional_rewards: Option<Vec<RewardTokenUnchecked>>,
}

#[andr_exec]
#[cw_serde]
pub enum ExecuteMsg {
    Receive(Cw20ReceiveMsg),
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
    /// None. Funds may be sent along with this.
    UpdateGlobalIndexes {
        asset_infos: Option<Vec<AssetInfoUnchecked>>,
    },
}

#[cw_serde]
pub enum Cw20HookMsg {
    /// Stake the sent tokens. Address must match the `staking_token` given on instantiation. The user's pending
    /// rewards and indexes are updated for each additional reward token.
    StakeTokens {},
    /// Updates the global reward index on deposit of a valid cw20 token.
    UpdateGlobalIndex {},
}

#[andr_query]
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    /// Gets the config of the contract.
    #[returns(Config)]
    Config {},
    /// Gets the state of the contract.
    #[returns(State)]
    State {},
    /// Returns a `StakerResponse` for the given staker. The pending rewards are updated to the
    /// present index.
    #[returns(StakerResponse)]
    Staker { address: String },
    /// Returns a `Vec<StakerResponse>` for range of stakers. The pending rewards are updated to the
    /// present index for each staker.
    #[returns(Vec<StakerResponse>)]
    Stakers {
        start_after: Option<String>,
        limit: Option<u32>,
    },
    /// Queries the current timestamp.
    #[returns(u64)]
    Timestamp {},
}

#[cw_serde]
pub struct Config {
    /// The token accepted for staking.
    pub staking_token: AndrAddr,
    /// The current number of reward tokens, cannot exceed `MAX_REWARD_TOKENS`.
    pub number_of_reward_tokens: u32,
}

#[cw_serde]
pub struct State {
    /// The total share of the staking token in the contract.
    pub total_share: Uint128,
}

#[cw_serde]
pub struct RewardTokenUnchecked {
    pub asset_info: AssetInfoUnchecked,
    pub allocation_config: Option<AllocationConfig>,
}

impl RewardTokenUnchecked {
    /// Verifies that the specified asset_info is valid and returns a `RewardToken` with the
    /// correct `RewardType`.
    pub fn check(
        self,
        block_info: &BlockInfo,
        api: &dyn Api,
    ) -> Result<RewardToken, ContractError> {
        //TODO replace unwrap() with ? once cw-asset is integrated in error.rs
        let checked_asset_info = self.asset_info.check(api, None).unwrap();
        let reward_type = match self.allocation_config {
            None => RewardType::NonAllocated {
                previous_reward_balance: Uint128::zero(),
            },
            Some(allocation_config) => {
                let init_timestamp = allocation_config.init_timestamp;
                let till_timestamp = allocation_config.till_timestamp;
                let cycle_duration = allocation_config.cycle_duration;
                let cycle_rewards = allocation_config.cycle_rewards;
                let reward_increase = allocation_config.reward_increase;

                ensure!(
                    init_timestamp >= block_info.time.seconds(),
                    ContractError::StartTimeInThePast {
                        current_block: block_info.height,
                        current_time: block_info.time.nanos() / MILLISECONDS_TO_NANOSECONDS_RATIO,
                    }
                );

                ensure!(
                    init_timestamp < till_timestamp,
                    ContractError::StartTimeAfterEndTime {}
                );

                ensure!(cycle_duration > 0, ContractError::InvalidCycleDuration {});

                if let Some(reward_increase) = reward_increase {
                    ensure!(
                        reward_increase < Decimal::one(),
                        ContractError::InvalidRewardIncrease {}
                    );
                }

                RewardType::Allocated {
                    allocation_config,
                    allocation_state: AllocationState {
                        current_cycle: 0,
                        current_cycle_rewards: cycle_rewards,
                        last_distributed: init_timestamp,
                    },
                }
            }
        };

        Ok(RewardToken {
            asset_info: checked_asset_info,
            reward_type,
            index: Decimal256::zero(),
        })
    }
}

#[cw_serde]
pub enum RewardType {
    Allocated {
        allocation_config: AllocationConfig,
        allocation_state: AllocationState,
    },
    NonAllocated {
        previous_reward_balance: Uint128,
    },
}

#[cw_serde]
pub struct RewardToken {
    pub asset_info: AssetInfo,
    pub index: Decimal256,
    pub reward_type: RewardType,
}

impl fmt::Display for RewardToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.asset_info.fmt(f)
    }
}

#[cw_serde]
pub struct AllocationInfo {
    /// The allocation config, this is immutable.
    pub config: AllocationConfig,
    /// The allocation state, this is mutable and changes as time goes on.
    pub state: AllocationState,
}

#[cw_serde]
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

#[cw_serde]
pub struct AllocationState {
    /// Keeps track of the distribution cycle
    pub current_cycle: u64,
    /// Number of tokens to be distributed during the current cycle
    pub current_cycle_rewards: Uint128,
    /// Timestamp at which the global_reward_index was last updated
    pub last_distributed: u64,
}

#[cw_serde]
pub struct StakerResponse {
    /// Address of the staker.
    pub address: String,
    /// The staker's share of the tokens.
    pub share: Uint128,
    /// The staker's balance of tokens.
    pub balance: Uint128,
    /// The staker's pending rewards represented as [(token_1, amount_1), ..., (token_n, amount_n)]
    pub pending_rewards: Vec<(String, Uint128)>,
}

#[cw_serde]
pub enum MigrateMsg {}
