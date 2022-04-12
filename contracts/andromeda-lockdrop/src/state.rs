use cosmwasm_std::{Addr, Decimal, Uint128};
use cw_storage_plus::{Item, Map};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use andromeda_protocol::lockdrop::LockupDurationParams;

pub const CONFIG_KEY: &str = "config";
pub const CONFIG: Item<Config> = Item::new(CONFIG_KEY);

pub const STATE_KEY: &str = "state";
pub const STATE: Item<State> = Item::new(STATE_KEY);

pub const USER_INFO: Map<&Addr, UserInfo> = Map::new("users");
pub const LOCKUP_INFO: Map<&[u8], LockupInfo> = Map::new("lockup_position");

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    /// Account which can update config
    pub owner: Addr,
    /// Contract used to query addresses related to red-bank (MARS Token)
    pub address_provider: Option<Addr>,
    ///  maUST token address - Minted upon UST deposits into red bank
    pub ma_ust_token: Option<Addr>,
    /// Auction Contract address to which MARS tokens can be deposited for bootstrapping MARS-UST Pool
    pub auction_contract_address: Option<Addr>,
    /// Timestamp when Contract will start accepting deposits
    pub init_timestamp: u64,
    /// Deposit Window Length
    pub deposit_window: u64,
    /// Withdrawal Window Length
    pub withdrawal_window: u64,
    ///  Durations and boosties params
    pub lockup_durations: Vec<LockupDurationParams>,
    /// Number of seconds per week
    pub seconds_per_duration_unit: u64,
    /// Total MARS lockdrop incentives to be distributed among the users
    pub lockdrop_incentives: Uint128,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct State {
    /// Total UST deposited at the end of Lockdrop window. This value remains unchanged post the lockdrop window
    pub final_ust_locked: Uint128,
    /// maUST minted at the end of Lockdrop window upon UST deposit in red bank. This value remains unchanged post the lockdrop window
    pub final_maust_locked: Uint128,
    /// UST deposited in the contract. This value is updated real-time upon each UST deposit / unlock
    pub total_ust_locked: Uint128,
    /// maUST held by the contract. This value is updated real-time upon each maUST withdrawal from red bank
    pub total_maust_locked: Uint128,
    /// MARS Tokens deposited into the bootstrap auction contract
    pub total_mars_delegated: Uint128,
    /// Boolean value indicating if the user can withdraw thier MARS rewards or not
    pub are_claims_allowed: bool,
    /// Total weighted deposits
    pub total_deposits_weight: Uint128,
    /// Ratio of MARS rewards accured to total_maust_locked. Used to calculate MARS incentives accured by each user
    pub xmars_rewards_index: Decimal,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct UserInfo {
    /// Total UST amount deposited by the user across all his lockup positions
    pub total_ust_locked: Uint128,
    /// User's maUST share against his total locked UST amount
    pub total_maust_share: Uint128,
    /// Contains lockup Ids of the User's lockup positions with different durations / deposit amounts
    pub lockup_positions: Vec<String>,
    /// MARS incentives allocated to the user for his weighted lockup positions
    pub total_mars_incentives: Uint128,
    /// MARS incentives deposited to the auction contract for MARS-UST Bootstrapping auction
    pub delegated_mars_incentives: Uint128,
    /// Boolean value indicating if the lockdrop_rewards for the lockup positions have been claimed or not
    pub lockdrop_claimed: bool,
    /// Ratio used to calculate deposit_rewards (XMARS) accured by the user
    pub reward_index: Decimal,
    /// Pending rewards to be claimed by the user
    pub total_xmars_claimed: Uint128,
}

impl Default for UserInfo {
    fn default() -> Self {
        UserInfo {
            total_ust_locked: Uint128::zero(),
            total_maust_share: Uint128::zero(),
            lockup_positions: vec![],
            total_mars_incentives: Uint128::zero(),
            delegated_mars_incentives: Uint128::zero(),
            lockdrop_claimed: false,
            reward_index: Decimal::zero(),
            total_xmars_claimed: Uint128::zero(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct LockupInfo {
    /// Lockup Duration
    pub duration: u64,
    /// UST locked as part of this lockup position
    pub ust_locked: Uint128,
    /// Lockdrop incentive allocated for this position
    pub lockdrop_reward: Uint128,
    /// Timestamp beyond which this position can be unlocked
    pub unlock_timestamp: u64,
    /// Boolean value indicating if the user's has withdrawn funds post the only 1 withdrawal limit cutoff
    pub withdrawal_flag: bool,
}

impl Default for LockupInfo {
    fn default() -> Self {
        LockupInfo {
            duration: 0_u64,
            ust_locked: Uint128::zero(),
            lockdrop_reward: Uint128::zero(),
            unlock_timestamp: 0_u64,
            withdrawal_flag: false,
        }
    }
}
