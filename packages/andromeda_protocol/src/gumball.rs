use common::{
    ado_base::{modules::Module, recipient::Recipient, AndromedaMsg, AndromedaQuery},
    mission::AndrAddress,
};
use cosmwasm_std::{attr, Addr, BankMsg, Binary, Coin, Event, Uint128};
use cw721_base::MintMsg;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::cw721::TokenExtension;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    pub andromeda_cw721_contract: AndrAddress,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    AndrReceive(AndromedaMsg),
    Mint(Box<MintMsg<TokenExtension>>),
    Buy {},
    // Sets price, max amount per wallet, and recipient
    SaleDetails {
        /// The price per token.
        price: Coin,
        /// The amount of tokens a wallet can purchase, default is 1.
        max_amount_per_wallet: Option<Uint128>,
        /// The recipient of the funds if the sale met the minimum sold.
        recipient: Recipient,
    },
    // Automatically switches to opposite status.
    // True means buying is allowed and minting is halted. False means the opposite.
    SwitchStatus {},
}
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    AndrQuery(AndromedaQuery),
    NumberOfNFTs {},
    SaleDetails {},
    Status {},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum RandQueryMsg {
    LatestDrand {},
    GetRandomness { round: u64 },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct NumberOfNFTsResponse {
    pub number: usize,
}
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct State {
    /// The price of each token.
    pub price: Coin,
    /// The max number of tokens allowed per wallet.
    pub max_amount_per_wallet: Uint128,
    /// The recipient of the funds upon a sale.
    /// Most likely the contract, but could also be the splitter contract for example.
    pub recipient: Recipient,
}
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct StateResponse {
    pub state: State,
}
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct StatusResponse {
    pub status: bool,
}
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct RandomResult {
    pub randomness: Binary,
    pub worker: Addr,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct LatestRandomResponse {
    pub round: u64,
    pub randomness: String,
    pub worker: Addr,
}
