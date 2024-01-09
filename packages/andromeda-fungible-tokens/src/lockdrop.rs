use andromeda_std::andr_instantiate_modules;
use andromeda_std::{andr_exec, andr_instantiate, andr_query};
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Uint128;
use cw20::Cw20ReceiveMsg;

#[andr_instantiate]
#[andr_instantiate_modules]
#[cw_serde]
pub struct InstantiateMsg {
    /// The bootsrap contract to be used in the second phase.
    // pub bootstrap_contract: Option<AndrAddress>,
    /// Timestamp till when deposits can be made
    pub init_timestamp: u64,
    /// Number of seconds for which lockup deposits will be accepted
    pub deposit_window: u64,
    /// Number of seconds for which lockup withdrawals will be allowed
    pub withdrawal_window: u64,
    /// The token being given as incentive.
    pub incentive_token: String,
    /// The native token being deposited.
    pub native_denom: String,
}

#[andr_exec]
#[cw_serde]
pub enum ExecuteMsg {
    Receive(Cw20ReceiveMsg),
    /// Function to deposit native fund in the contract in exchange for recieving a proportion of the
    /// TOKEN.
    DepositNative {},
    /// Function to withdraw native fund from the lockup position.
    WithdrawNative {
        amount: Option<Uint128>,
    },
    /// Facilitates reward claim after claims are enabled.
    ClaimRewards {},
    /// Called by the bootstrap contract when liquidity is added to the TOKEN-NATIVE Pool to enable TOKEN withdrawals by users.
    EnableClaims {},
    // Called by the owner after the phase is over to withdraw all of the NATIVE token to the
    // given recipient, or themselves if not specified.
    // WithdrawProceeds {
    //     recipient: Option<String>,
    // },
}

#[cw_serde]
pub enum Cw20HookMsg {
    /// Increase the incentives for the deposited token. Sender must be the incentive token.
    IncreaseIncentives {},
}

#[andr_query]
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    /// Gets the config information.
    #[returns(ConfigResponse)]
    Config {},
    /// Gets the state information.
    #[returns(StateResponse)]
    State {},
    /// Gets information for the user with `address`.
    #[returns(UserInfoResponse)]
    UserInfo { address: String },
    /// Gets the withdrawal percent allowed given the timestamp, or the current time if not
    /// specified. Timestamp is in seconds.
    #[returns(::cosmwasm_std::Decimal)]
    WithdrawalPercentAllowed { timestamp: Option<u64> },
}

#[cw_serde]
pub struct ConfigResponse {
    /// Bootstrap Contract address to which tokens can be delegated to for bootstrapping TOKEN-NATIVE Pool.
    // pub bootstrap_contract_address: Option<String>,
    /// Timestamp till when deposits can be made.
    pub init_timestamp: u64,
    /// Number of seconds for which lockup deposits will be accepted.
    pub deposit_window: u64,
    /// Number of seconds for which lockup withdrawals will be allowed.
    pub withdrawal_window: u64,
    /// Total token lockdrop incentives to be distributed among the users.
    pub lockdrop_incentives: Uint128,
    /// The token being given as incentive.
    pub incentive_token: String,
    /// The native token being deposited.
    pub native_denom: String,
}

#[cw_serde]
pub struct StateResponse {
    /// Total NATIVE deposited at the end of Lockdrop window. This value remains unchanged post the lockdrop window.
    pub total_native_locked: Uint128,
    /// Boolean value indicating if the user can withdraw their token rewards or not.
    pub are_claims_allowed: bool,
}

#[cw_serde]
pub struct UserInfoResponse {
    pub total_native_locked: Uint128,
    pub total_incentives: Uint128,
    pub is_lockdrop_claimed: bool,
    pub withdrawal_flag: bool,
}

#[cw_serde]
pub struct MigrateMsg {}
