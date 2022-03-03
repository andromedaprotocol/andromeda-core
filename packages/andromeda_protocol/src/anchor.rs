use crate::communication::{AndromedaMsg, AndromedaQuery, Recipient};
use astroport::asset::Asset;
use cosmwasm_std::{Decimal, Uint128};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum WithdrawalType {
    Amount(Uint128),
    Percentage(Uint128),
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    pub anchor_market: String,
    pub anchor_bluna_hub: String,
    pub anchor_bluna_custody: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    AndrReceive(AndromedaMsg),
    /// Deposit LUNA as collateral which will be converted to bLUNA.
    DepositCollateral {},
    /// Deposit LUNA as collateral which will be converted to bLUNA.
    WithdrawCollateral {
        collateral: Asset,
    },
    /// Borrows given percent of collateral worth and sends borrowed funds to recipient.
    Borrow {
        percent_of_collateral: Uint128,
        recipient: Recipient,
    },
    /// Repays any existing loan with sent stable coins.
    RepayLoan {},
}

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct MigrateMsg {}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    AndrQuery(AndromedaQuery),
    Config {},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ConfigResponse {
    pub anchor_market: String,
    pub aust_token: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct PositionResponse {
    pub recipient: Recipient,
    pub aust_amount: Uint128,
}

/* Begin BLunaHub enums and structs */

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub enum BLunaHubExecuteMsg {
    Bond {},
    UnBond {},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub enum BLunaHubQueryMsg {
    State {},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct BLunaHubStateResponse {
    pub bluna_exchange_rate: Decimal,
    pub stluna_exchange_rate: Decimal,
    pub total_bond_bluna_amount: Uint128,
    pub total_bond_stluna_amount: Uint128,
    pub last_index_modification: u64,
    pub prev_hub_balance: Uint128,
    pub last_unbonded_time: u64,
    pub last_processed_batch: u64,
}
