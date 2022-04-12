use cosmwasm_std::{Addr, Decimal, Uint128};
use cw_storage_plus::{Item, Map};

use common::mission::AndrAddress;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub const CONFIG_KEY: &str = "config";
pub const CONFIG: Item<Config> = Item::new(CONFIG_KEY);

pub const STATE_KEY: &str = "state";
pub const STATE: Item<State> = Item::new(STATE_KEY);

pub const USER_INFO: Map<&Addr, UserInfo> = Map::new("users");

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    /// Auction Contract address to which MARS tokens can be deposited for bootstrapping MARS-UST Pool
    pub auction_contract_address: Option<AndrAddress>,
    /// Timestamp when Contract will start accepting deposits
    pub init_timestamp: u64,
    /// Deposit Window Length
    pub deposit_window: u64,
    /// Withdrawal Window Length
    pub withdrawal_window: u64,
    /// Total MARS lockdrop incentives to be distributed among the users
    pub lockdrop_incentives: Uint128,
    pub incentive_token: String,
}

#[derive(Default, Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct State {
    /// Total UST deposited at the end of Lockdrop window. This value remains unchanged post the lockdrop window
    pub final_ust_locked: Uint128,
    /// maUST minted at the end of Lockdrop window upon UST deposit in red bank. This value remains unchanged post the lockdrop window
    pub final_maust_locked: Uint128,
    /// UST deposited in the contract. This value is updated real-time upon each UST deposit / unlock
    pub total_ust_locked: Uint128,
    /// MARS Tokens deposited into the bootstrap auction contract
    pub total_mars_delegated: Uint128,
    /// Boolean value indicating if the user can withdraw thier MARS rewards or not
    pub are_claims_allowed: bool,
}

#[derive(Default, Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct UserInfo {
    /// Total UST amount deposited by the user across all his lockup positions
    pub total_ust_locked: Uint128,
    /// MARS incentives allocated to the user for his weighted lockup positions
    pub total_incentives: Uint128,
    /// MARS incentives deposited to the auction contract for MARS-UST Bootstrapping auction
    pub delegated_mars_incentives: Uint128,
    /// Boolean value indicating if the lockdrop_rewards for the lockup positions have been claimed or not
    pub lockdrop_claimed: bool,
    /// Ratio used to calculate deposit_rewards (XMARS) accured by the user
    pub reward_index: Decimal,
    pub withdrawal_flag: bool,
}
