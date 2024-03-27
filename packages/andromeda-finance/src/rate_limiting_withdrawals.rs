use andromeda_std::{andr_exec, andr_instantiate, andr_instantiate_modules, andr_query};
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Timestamp, Uint128};

#[cw_serde]
/// Keeps track of the account's balance and time of latest withdrawal
pub struct AccountDetails {
    /// Account balance, no need for denom since only one is allowed
    pub balance: Uint128,
    /// Timestamp of latest withdrawal
    pub latest_withdrawal: Option<Timestamp>,
}
#[cw_serde]
pub struct CoinAndLimit {
    /// Sets the accepted coin denom
    pub coin: String,
    /// Sets the withdrawal limit in terms of amount
    pub limit: Uint128,
}

#[cw_serde]
pub struct CoinAllowance {
    /// Sets the accepted coin denom
    pub coin: String,
    /// Sets the withdrawal limit in terms of amount
    pub limit: Uint128,
    /// Sets the minimum amount of time required between withdrawals in seconds
    pub minimal_withdrawal_frequency: Uint128,
}

#[cw_serde]
pub struct ContractAndKey {
    pub contract_address: String,
    pub key: Option<String>,
}

#[andr_instantiate]
#[andr_instantiate_modules]
#[cw_serde]
pub struct InstantiateMsg {
    pub allowed_coin: CoinAndLimit,
    pub minimal_withdrawal_frequency: MinimumFrequency,
}

#[cw_serde]
pub enum MinimumFrequency {
    Time { time: Uint128 },
    AddressAndKey { address_and_key: ContractAndKey },
}

#[andr_exec]
#[cw_serde]
//NOTE can't name Deposit and Withdraw while implementing andr_exec
pub enum ExecuteMsg {
    Deposits { recipient: Option<String> },
    Withdraws { amount: Uint128 },
}

#[andr_query]
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    /// Provides the allowed coin and limits for withdrawal size and frequency
    #[returns(CoinAllowance)]
    CoinAllowanceDetails {},
    /// Shows the balance and latest withdrawal time
    #[returns(AccountDetails)]
    AccountDetails { account: String },
}
