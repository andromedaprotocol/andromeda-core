use common::ado_base::{modules::Module, AndromedaMsg, AndromedaQuery};
use cosmwasm_std::{Timestamp, Uint128};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
/// Keeps track of the account's balance and time of latest withdrawal
pub struct AccountDetails {
    /// Account balance, no need for denom since only one is allowed
    pub balance: Uint128,
    /// Timestamp of latest withdrawal
    pub latest_withdrawal: Option<Timestamp>,
}
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct CoinAndLimit {
    /// Sets the accepted coin denom
    pub coin: String,
    /// Sets the withdrawal limit in terms of amount
    pub limit: Uint128,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct CoinAllowance {
    /// Sets the accepted coin denom
    pub coin: String,
    /// Sets the withdrawal limit in terms of amount
    pub limit: Uint128,
    /// Sets the minimum amount of time required between withdrawals in seconds
    pub minimal_withdrawal_frequency: Uint128,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ContractAndKey {
    pub contract_address: String,
    pub key: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    pub allowed_coin: CoinAndLimit,
    pub minimal_withdrawal_frequency: Option<Uint128>,
    pub contract_key: Option<ContractAndKey>,
    pub modules: Option<Vec<Module>>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    Deposit {
        recipient: Option<String>,
    },
    Withdraw {
        amount: Uint128,
    },
    AndrReceive(AndromedaMsg),
    UpdateAllowedCoin {
        allowed_coin: CoinAndLimit,
        minimal_withdrawal_frequency: Option<Uint128>,
        contract_key: Option<ContractAndKey>,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct MigrateMsg {}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    AndrQuery(AndromedaQuery),
    /// Provides the allowed coin and limits for withdrawal size and frequency
    CoinAllowanceDetails {},
    /// Shows the balance and latest withdrawal time
    AccountDetails {
        account: String,
    },
}
