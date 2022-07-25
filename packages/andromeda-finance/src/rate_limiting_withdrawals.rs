use common::ado_base::{modules::Module, AndromedaMsg, AndromedaQuery};
use cosmwasm_std::{Timestamp, Uint128};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
/// Keeps track of the account's balance and time of latest withdrawal
pub struct AccountDetails {
    pub balance: Uint128,
    pub latest_withdrawal: Option<Timestamp>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct CoinAllowance {
    pub coin: String,
    pub limit: Uint128,
    pub minimal_withdrawal_frequency: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    pub allowed_coin: CoinAllowance,
    pub modules: Option<Vec<Module>>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    Deposit { recipient: Option<String> },
    Withdraw { amount: Uint128 },
    AndrReceive(AndromedaMsg),
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
