use common::ado_base::{modules::Module, recipient::Recipient, AndromedaMsg, AndromedaQuery};
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
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    pub allowed_coin: CoinAllowance,
    pub minimum_withdrawal_time: u64,
    pub modules: Option<Vec<Module>>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    Deposit { recipient: Option<Recipient> },
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
    MinimalWithdrawalFrequency {},
    CoinWithdrawalLimit {},
    /// Shows the balance and latest withdrawal time
    AccountDetails {
        account: String,
    },
}
