use cosmwasm_std::{to_binary, Addr, CosmosMsg, Decimal, StdResult, Uint128, WasmMsg};
use cw20::Cw20ReceiveMsg;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    /// Timestamp till when deposits can be made
    pub init_timestamp: u64,
    /// Number of seconds for which lockup deposits will be accepted
    pub deposit_window: u64,
    /// Number of seconds for which lockup withdrawals will be allowed
    pub withdrawal_window: u64,
    /// Number of seconds per week
    pub seconds_per_duration_unit: u64,
    /// The token being given as incentive.
    pub incentive_token: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct UpdateConfigMsg {
    /// Bootstrap Auction contract address
    pub auction_contract_address: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    Receive(Cw20ReceiveMsg),

    UpdateConfig {
        new_config: UpdateConfigMsg,
    },
    /// Function to deposit UST in the contract locked for `duration` number of weeks, starting once the deposits/withdrawals are disabled
    DepositUst {},
    /// Function to withdraw UST from the lockup position which is locked for `duration` number of weeks
    WithdrawUst {
        amount: Uint128,
    },
    /*/// Deposit MARS to auction contract
    DepositMarsToAuction {
        amount: Uint128,
    },*/
    /// Facilitates MARS reward claim and optionally unlocking any lockup position once the lockup duration is over
    ClaimRewards {},
    /// Called by the bootstrap auction contract when liquidity is added to the MARS-UST Pool to enable MARS withdrawals by users
    EnableClaims {},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Cw20HookMsg {
    IncreaseIncentives {},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    Config {},
    State {},
    UserInfo { address: String },
    WithdrawalPercentAllowed { timestamp: Option<u64> },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ConfigResponse {
    /// Auction Contract address to which MARS tokens can be delegated to for bootstrapping MARS-UST Pool
    pub auction_contract_address: Option<Addr>,
    /// Timestamp till when deposits can be made
    pub init_timestamp: u64,
    /// Number of seconds for which lockup deposits will be accepted
    pub deposit_window: u64,
    /// Number of seconds for which lockup withdrawals will be allowed
    pub withdrawal_window: u64,
    /// Number of seconds per week
    pub seconds_per_duration_unit: u64,
    /// Total MARS lockdrop incentives to be distributed among the users
    pub lockdrop_incentives: Uint128,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct StateResponse {
    /// Total UST deposited at the end of Lockdrop window. This value remains unchanged post the lockdrop window
    pub final_ust_locked: Uint128,
    /// UST deposited in the contract. This value is updated real-time upon each UST deposit / unlock
    pub total_ust_locked: Uint128,
    /// MARS Tokens deposited into the bootstrap auction contract
    pub total_mars_delegated: Uint128,
    /// Boolean value indicating if the user can withdraw thier MARS rewards or not
    pub are_claims_allowed: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct UserInfoResponse {
    pub total_ust_locked: Uint128,
    pub total_mars_incentives: Uint128,
    pub delegated_mars_incentives: Uint128,
    pub is_lockdrop_claimed: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct LockupInfoResponse {
    /// returns lockup data if a match is found on a query, None otherwise
    pub lockup_info: Option<LockupInfoQueryData>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct LockupInfoQueryData {
    /// Lockup Duration
    pub duration: u64,
    /// UST locked as part of this lockup position
    pub ust_locked: Uint128,
    /// MA-UST share
    pub maust_balance: Uint128,
    /// Lockdrop incentive distributed to this position
    pub lockdrop_reward: Uint128,
    /// Timestamp beyond which this position can be unlocked
    pub unlock_timestamp: u64,
    /// Boolean value indicating if the user's has withdrawn funds post the only 1 withdrawal limit cutoff
    pub withdrawal_flag: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct MigrateMsg {}
