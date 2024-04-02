use andromeda_std::common::Milliseconds;
use cosmwasm_std::{Addr, Uint128};
use cw_storage_plus::{Item, Map};

// use common::app::AndrAddress;
use cosmwasm_schema::cw_serde;

pub const CONFIG_KEY: &str = "config";
pub const CONFIG: Item<Config> = Item::new(CONFIG_KEY);

pub const STATE_KEY: &str = "state";
pub const STATE: Item<State> = Item::new(STATE_KEY);

pub const USER_INFO: Map<&Addr, UserInfo> = Map::new("users");

#[cw_serde]
pub struct Config {
    /// Bootstrap Contract address to which incentive tokens can be deposited for bootstrapping TOKEN-NATIVE Pool
    // pub bootstrap_contract_address: Option<AndrAddress>,
    /// Timestamp when Contract will start accepting deposits
    pub init_timestamp: Milliseconds,
    /// Deposit Window Length
    pub deposit_window: Milliseconds,
    /// Withdrawal Window Length
    pub withdrawal_window: Milliseconds,
    /// Total Token lockdrop incentives to be distributed among the users
    pub lockdrop_incentives: Uint128,
    /// The token being given as incentive.
    pub incentive_token: Addr,
    /// The native token being deposited.
    pub native_denom: String,
}

#[cw_serde]
#[derive(Default)]
pub struct State {
    /// Total NATIVE deposited at the end of Lockdrop window. This value remains unchanged post the lockdrop window
    pub total_native_locked: Uint128,
    /// Number of Tokens deposited into the bootstrap contract
    pub total_delegated: Uint128,
    /// Boolean value indicating if the user can withdraw their token rewards or not
    pub are_claims_allowed: bool,
}

#[cw_serde]
#[derive(Default)]

pub struct UserInfo {
    /// Total UST amount deposited by the user across all his lockup positions
    pub total_native_locked: Uint128,
    /// TOKEN incentives deposited to the bootstrap contract for TOKEN-UST Bootstrapping
    pub delegated_incentives: Uint128,
    /// Boolean value indicating if the lockdrop_rewards for the lockup positions have been claimed or not
    pub lockdrop_claimed: bool,
    /// Whether or not the user has withdrawn during the withdrawal phase.
    pub withdrawal_flag: bool,
}
