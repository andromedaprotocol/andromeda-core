use common::ado_base::{recipient::Recipient, AndromedaMsg, AndromedaQuery};
use cosmwasm_bignumber::{Decimal256, Uint256};
use cosmwasm_std::Uint128;
use cw20::Cw20ReceiveMsg;
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
    Receive(Cw20ReceiveMsg),
    AndrReceive(AndromedaMsg),
    /// Deposit LUNA as collateral which will be converted to bLUNA.
    DepositCollateral {},
    /// Withdraw specified collateral. If unbond is true and collateral is bLuna, the unbonding
    /// process will begin, otherwise the collateral will be sent to the given recipient.
    WithdrawCollateral {
        collateral_addr: String,
        amount: Option<Uint256>,
        unbond: Option<bool>,
        recipient: Option<Recipient>,
    },
    /// Borrows funds to reach the desired loan-to-value ratio and sends the borrowed funds to the
    /// given recipient.
    Borrow {
        desired_ltv_ratio: Decimal256,
        recipient: Option<Recipient>,
    },
    /// Repays any existing loan with sent stable coins.
    RepayLoan {},

    /// INTERNAL
    DepositCollateralToAnchor {
        collateral_addr: String,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Cw20HookMsg {
    /// Deposit Cw20 assets as collateral.
    DepositCollateral {},
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
    pub anchor_bluna_hub: String,
    pub anchor_bluna_custody: String,
    pub anchor_overseer: String,
    pub bluna_token: String,
    pub anchor_oracle: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct PositionResponse {
    pub recipient: Recipient,
    pub aust_amount: Uint128,
}

/* Begin BLunaHub enums and structs */

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum BLunaHubExecuteMsg {
    Bond {},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum BLunaHubCw20HookMsg {
    Unbond {},
}
