use cosmwasm_std::{to_binary, Addr, CosmosMsg, Decimal, StdResult, Uint128, WasmMsg};
use cw20::Cw20ReceiveMsg;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    /// Account who can update config
    pub owner: String,
    /// Contract used to query addresses related to red-bank (MARS Token)
    pub address_provider: Option<String>,
    ///  maUST token address - Minted upon UST deposits into red bank
    pub ma_ust_token: Option<String>,
    /// Timestamp till when deposits can be made
    pub init_timestamp: u64,
    /// Number of seconds for which lockup deposits will be accepted
    pub deposit_window: u64,
    /// Number of seconds for which lockup withdrawals will be allowed
    pub withdrawal_window: u64,
    /// Durations and boosties params
    pub lockup_durations: Vec<LockupDurationParams>,
    /// Number of seconds per week
    pub seconds_per_duration_unit: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct UpdateConfigMsg {
    /// Account who can update config
    pub owner: Option<String>,
    /// Contract used to query addresses related to red-bank (MARS Token)
    pub address_provider: Option<String>,
    ///  maUST token address - Minted upon UST deposits into red bank
    pub ma_ust_token: Option<String>,
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
    DepositUst {
        duration: u64,
    },
    /// Function to withdraw UST from the lockup position which is locked for `duration` number of weeks
    WithdrawUst {
        duration: u64,
        amount: Uint128,
    },
    /// ADMIN Function :: Deposits all UST into the Red Bank
    DepositUstInRedBank {},
    /// Deposit MARS to auction contract
    DepositMarsToAuction {
        amount: Uint128,
    },
    /// Facilitates MARS reward claim and optionally unlocking any lockup position once the lockup duration is over
    ClaimRewardsAndUnlock {
        lockup_to_unlock_duration: Option<u64>,
    },
    /// Called by the bootstrap auction contract when liquidity is added to the MARS-UST Pool to enable MARS withdrawals by users
    EnableClaims {},
    /// Callbacks; only callable by the contract itself.
    Callback(CallbackMsg),
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum CallbackMsg {
    UpdateStateOnRedBankDeposit {
        prev_ma_ust_balance: Uint128,
    },
    UpdateStateOnClaim {
        user: Addr,
        prev_xmars_balance: Uint128,
    },
    DissolvePosition {
        user: Addr,
        duration: u64,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Cw20HookMsg {
    IncreaseMarsIncentives {},
}

// Modified from
// https://github.com/CosmWasm/cosmwasm-plus/blob/v0.2.3/packages/cw20/src/receiver.rs#L15
impl CallbackMsg {
    pub fn to_cosmos_msg(&self, contract_addr: &Addr) -> StdResult<CosmosMsg> {
        Ok(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: String::from(contract_addr),
            msg: to_binary(&ExecuteMsg::Callback(self.clone()))?,
            funds: vec![],
        }))
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    Config {},
    State {},
    UserInfo { address: String },
    LockupInfo { address: String, duration: u64 },
    LockupInfoWithId { lockup_id: String },
    WithdrawalPercentAllowed { timestamp: Option<u64> },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ConfigResponse {
    /// Account who can update config
    pub owner: String,
    /// Contract used to query addresses related to red-bank (MARS Token)
    pub address_provider: Option<Addr>,
    ///  maUST token address - Minted upon UST deposits into red bank
    pub ma_ust_token: Option<Addr>,
    /// Auction Contract address to which MARS tokens can be delegated to for bootstrapping MARS-UST Pool
    pub auction_contract_address: Option<Addr>,
    /// Timestamp till when deposits can be made
    pub init_timestamp: u64,
    /// Number of seconds for which lockup deposits will be accepted
    pub deposit_window: u64,
    /// Number of seconds for which lockup withdrawals will be allowed
    pub withdrawal_window: u64,
    /// Durations and boosties params
    pub lockup_durations: Vec<LockupDurationParams>,
    /// Number of seconds per week
    pub seconds_per_duration_unit: u64,
    /// Total MARS lockdrop incentives to be distributed among the users
    pub lockdrop_incentives: Uint128,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct StateResponse {
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
pub struct UserInfoResponse {
    pub total_ust_locked: Uint128,
    pub total_maust_share: Uint128,
    pub lockup_position_ids: Vec<String>,
    pub total_mars_incentives: Uint128,
    pub delegated_mars_incentives: Uint128,
    pub is_lockdrop_claimed: bool,
    pub reward_index: Decimal,
    pub total_xmars_claimed: Uint128,
    pub pending_xmars_to_claim: Uint128,
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
pub struct LockupDurationParams {
    pub duration: u64,
    pub boost: Uint128,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct MigrateMsg {}
